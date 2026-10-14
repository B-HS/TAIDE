use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::document::EditorError;
use crate::json_parser::Document;
use crate::json_schema_associations::{self, Association, PatternCompiler};
use crate::json_schema_refs::Resolved;
use crate::json_schema_uri;
use crate::json_value::{Arena, Member, Value};

const COMBINED_PREFIX: &str = "schemaservice://combinedSchema/";
const INTERNAL_SCHEME: &str = "schemaservice";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub associations: json_schema_associations::Limits,
    pub max_handles: usize,
    pub max_registered: usize,
    pub max_associations: usize,
    pub max_values: usize,
    pub max_text_units: usize,
    pub max_work: usize,
}

#[derive(Debug, Clone)]
pub struct Unresolved {
    pub schema: Arc<Arena>,
    pub errors: Vec<Vec<u16>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Load,
    Resolve,
}

#[derive(Debug, Clone)]
pub struct Ticket {
    owner: Arc<()>,
    handle: u64,
    generation: u64,
    phase: Phase,
    sequence: u64,
}

impl PartialEq for Ticket {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.handle == other.handle
            && self.generation == other.generation
            && self.phase == other.phase
            && self.sequence == other.sequence
    }
}

impl Eq for Ticket {}

#[derive(Debug, Clone)]
pub enum LoadState {
    Ready(Arc<Unresolved>),
    Start(Ticket),
    Pending(Ticket),
}

#[derive(Debug, Clone)]
pub enum ResolveState {
    Ready(Arc<Resolved>),
    Start { ticket: Ticket, source: LoadState },
    Pending(Ticket),
}

#[derive(Clone)]
struct Handle {
    id: Vec<u16>,
    generation: u64,
    resolve_sequence: u64,
    unresolved: Option<Arc<Unresolved>>,
    resolved: Option<Arc<Resolved>>,
    load_pending: bool,
    resolve_pending: bool,
    dependencies: Vec<Vec<u16>>,
}

struct AssociationEntry {
    entry: Arc<Association>,
    units: usize,
}

pub struct Service<'a> {
    compiler: &'a dyn PatternCompiler,
    limits: Limits,
    owner: Arc<()>,
    next_handle: u64,
    handles: BTreeMap<u64, Handle>,
    schemas: BTreeMap<Vec<u16>, u64>,
    contributions: Vec<(Vec<u16>, u64)>,
    registered: Vec<Vec<u16>>,
    associations: Vec<Arc<AssociationEntry>>,
    contribution_associations: Vec<Arc<AssociationEntry>>,
    cached_resource: Option<(Vec<u16>, Option<Vec<u16>>)>,
}

fn add(total: &mut usize, count: usize) -> Result<(), EditorError> {
    *total = total.checked_add(count).ok_or(EditorError::Capacity)?;
    Ok(())
}

fn is_schema(arena: &Arena) -> bool {
    arena
        .root()
        .and_then(|root| arena.get(root))
        .is_some_and(|root| matches!(root, Value::Object(_) | Value::Boolean(_)))
}

fn arena_cost(arena: &Arena, limits: Limits) -> Result<(usize, usize), EditorError> {
    if arena.values().len() > limits.max_values || arena.values().len() > limits.max_work {
        return Err(EditorError::Capacity);
    }
    let mut units = 0;
    for value in arena.values() {
        match value {
            Value::String(value) => add(&mut units, value.len())?,
            Value::Object(members) => {
                for member in members {
                    add(&mut units, member.key.len())?;
                }
            }
            _ => {}
        }
    }
    Ok((arena.values().len(), units))
}

fn handle_cost(handle: &Handle, limits: Limits) -> Result<(usize, usize), EditorError> {
    let mut values = 0;
    let mut units = handle.id.len();
    if let Some(unresolved) = &handle.unresolved {
        let cost = arena_cost(&unresolved.schema, limits)?;
        add(&mut values, cost.0)?;
        add(&mut units, cost.1)?;
        for error in &unresolved.errors {
            add(&mut units, error.len())?;
        }
    }
    if let Some(resolved) = &handle.resolved {
        let cost = arena_cost(&resolved.schema, limits)?;
        add(&mut values, cost.0)?;
        add(&mut units, cost.1)?;
        for error in &resolved.errors {
            add(&mut units, error.len())?;
        }
        if let Some(draft) = &resolved.schema_draft {
            add(&mut units, draft.len())?;
        }
    }
    for dependency in &handle.dependencies {
        add(&mut units, dependency.len())?;
    }
    Ok((values, units))
}

impl<'a> Service<'a> {
    pub fn new(compiler: &'a dyn PatternCompiler, limits: Limits) -> Self {
        Self {
            compiler,
            limits,
            owner: Arc::new(()),
            next_handle: 0,
            handles: BTreeMap::new(),
            schemas: BTreeMap::new(),
            contributions: Vec::new(),
            registered: Vec::new(),
            associations: Vec::new(),
            contribution_associations: Vec::new(),
            cached_resource: None,
        }
    }

    fn kept(&self, replacement: Option<(&[u16], bool)>) -> BTreeSet<u64> {
        let mut kept = BTreeSet::new();
        for (id, &handle) in &self.schemas {
            if replacement.is_none_or(|(target, _)| id != target) {
                kept.insert(handle);
            }
        }
        for (id, handle) in &self.contributions {
            if replacement.is_none_or(|(target, is_contribution)| !is_contribution || id != target)
            {
                kept.insert(*handle);
            }
        }
        kept
    }

    fn check(
        &self,
        kept: &BTreeSet<u64>,
        new_handle: Option<&Handle>,
        new_association: Option<&AssociationEntry>,
        registered_count: usize,
        cache: Option<(&[u16], Option<&[u16]>)>,
        new_registered: Option<&[u16]>,
    ) -> Result<(), EditorError> {
        let handles = new_handle.into_iter().collect::<Vec<_>>();
        self.check_handles(
            kept,
            &handles,
            new_association,
            registered_count,
            cache,
            new_registered,
        )
    }

    fn check_handles(
        &self,
        kept: &BTreeSet<u64>,
        new_handles: &[&Handle],
        new_association: Option<&AssociationEntry>,
        registered_count: usize,
        cache: Option<(&[u16], Option<&[u16]>)>,
        new_registered: Option<&[u16]>,
    ) -> Result<(), EditorError> {
        let count = kept
            .len()
            .checked_add(new_handles.len())
            .ok_or(EditorError::Capacity)?;
        let association_count = self
            .associations
            .len()
            .checked_add(usize::from(new_association.is_some()))
            .ok_or(EditorError::Capacity)?;
        if count > self.limits.max_handles
            || association_count > self.limits.max_associations
            || registered_count > self.limits.max_registered
        {
            return Err(EditorError::Capacity);
        }
        let mut values = 0;
        let mut units = 0;
        for handle in kept
            .iter()
            .filter_map(|key| self.handles.get(key))
            .chain(new_handles.iter().copied())
        {
            let cost = handle_cost(handle, self.limits)?;
            add(&mut values, cost.0)?;
            add(&mut units, cost.1)?;
        }
        for entry in self
            .associations
            .iter()
            .map(Arc::as_ref)
            .chain(new_association)
        {
            add(&mut units, entry.units)?;
        }
        for id in &self.registered {
            add(&mut units, id.len())?;
        }
        for (id, _) in &self.contributions {
            if !self.registered.contains(id) {
                add(&mut units, id.len())?;
            }
        }
        if let Some(id) = new_registered {
            add(&mut units, id.len())?;
        }
        if let Some((resource, selection)) = cache {
            add(&mut units, resource.len())?;
            if let Some(selection) = selection {
                add(&mut units, selection.len())?;
            }
        }
        if values > self.limits.max_values || units > self.limits.max_text_units {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn cache(&self) -> Option<(&[u16], Option<&[u16]>)> {
        self.cached_resource
            .as_ref()
            .map(|(resource, selection)| (resource.as_slice(), selection.as_deref()))
    }

    fn insert_handle(
        &mut self,
        id: Vec<u16>,
        handle: Handle,
        kept: BTreeSet<u64>,
    ) -> Result<u64, EditorError> {
        let key = self.next_handle;
        let next = key.checked_add(1).ok_or(EditorError::RevisionOverflow)?;
        self.handles.retain(|key, _| kept.contains(key));
        self.handles.insert(key, handle);
        self.schemas.insert(id, key);
        self.next_handle = next;
        Ok(key)
    }

    fn get_or_add(&mut self, id: Vec<u16>) -> Result<u64, EditorError> {
        if let Some(&key) = self.schemas.get(&id) {
            return Ok(key);
        }
        let handle = Handle {
            id: id.clone(),
            generation: 0,
            resolve_sequence: 0,
            unresolved: None,
            resolved: None,
            load_pending: false,
            resolve_pending: false,
            dependencies: Vec::new(),
        };
        let kept = self.kept(None);
        self.check(
            &kept,
            Some(&handle),
            None,
            self.registered.len(),
            self.cache(),
            None,
        )?;
        self.insert_handle(id, handle, kept)
    }

    pub fn register(
        &mut self,
        id: &[u16],
        schema: Option<Arc<Arena>>,
        patterns: &[Vec<u16>],
        folder: Option<&[u16]>,
        is_contribution: bool,
    ) -> Result<(), EditorError> {
        let id = json_schema_uri::normalize_id(id, self.limits.associations.uri)?;
        if let Some(schema) = &schema {
            if !is_schema(schema) {
                return Err(EditorError::InvalidBoundary);
            }
        }
        let mut units = id.len();
        for pattern in patterns {
            add(&mut units, pattern.len())?;
        }
        if let Some(folder) = folder {
            add(&mut units, folder.len())?;
        }
        let association = if patterns.is_empty() {
            None
        } else {
            Some(AssociationEntry {
                entry: Arc::new(Association::new(
                    patterns,
                    folder,
                    &[id.clone()],
                    self.compiler,
                    self.limits.associations,
                )?),
                units,
            })
        };
        let registered_count = self
            .registered
            .len()
            .checked_add(usize::from(
                !is_contribution && !self.registered.contains(&id),
            ))
            .ok_or(EditorError::Capacity)?;
        let has_contribution = self.contributions.iter().any(|(uri, _)| uri == &id);
        let restored_count = self
            .contributions
            .len()
            .checked_add(usize::from(is_contribution && !has_contribution))
            .ok_or(EditorError::Capacity)?;
        let should_replace = schema.is_some() || is_contribution;
        let handle = Handle {
            id: id.clone(),
            generation: 0,
            resolve_sequence: 0,
            unresolved: schema.map(|schema| {
                Arc::new(Unresolved {
                    schema,
                    errors: Vec::new(),
                })
            }),
            resolved: None,
            load_pending: false,
            resolve_pending: false,
            dependencies: Vec::new(),
        };
        let kept = self.kept(if should_replace {
            Some((&id, is_contribution))
        } else {
            None
        });
        let has_handle = self.schemas.contains_key(&id);
        self.check(
            &kept,
            if should_replace || !has_handle {
                Some(&handle)
            } else {
                None
            },
            association.as_ref(),
            registered_count.max(restored_count),
            None,
            if !self.registered.contains(&id) && !has_contribution {
                Some(&id)
            } else {
                None
            },
        )?;
        let key = if should_replace || !has_handle {
            self.insert_handle(id.clone(), handle, kept)?
        } else {
            *self.schemas.get(&id).ok_or(EditorError::NotFound)?
        };
        if is_contribution {
            if let Some(entry) = self.contributions.iter_mut().find(|(uri, _)| uri == &id) {
                entry.1 = key;
            } else {
                self.contributions.push((id.clone(), key));
            }
        }
        if !is_contribution && !self.registered.contains(&id) {
            self.registered.push(id);
        }
        if let Some(association) = association {
            let association = Arc::new(association);
            if is_contribution {
                self.contribution_associations
                    .push(Arc::clone(&association));
            }
            self.associations.push(association);
        }
        self.cached_resource = None;
        Ok(())
    }

    pub fn clear_external(&mut self) {
        self.schemas = self.contributions.iter().cloned().collect();
        self.registered = self
            .contributions
            .iter()
            .map(|(id, _)| id.clone())
            .collect();
        self.associations = self.contribution_associations.clone();
        self.cached_resource = None;
        let kept = self.kept(None);
        self.handles.retain(|key, _| kept.contains(key));
    }

    pub fn registered_ids(
        &self,
        include: impl Fn(&[u16]) -> bool,
    ) -> Result<Vec<Vec<u16>>, EditorError> {
        let mut result = Vec::new();
        let mut work = 0;
        for id in &self.registered {
            add(&mut work, id.len())?;
            if work > self.limits.max_work {
                return Err(EditorError::Capacity);
            }
            let scheme = json_schema_uri::scheme(id, self.limits.associations.uri)?;
            if scheme.iter().copied().eq(INTERNAL_SCHEME.encode_utf16()) || !include(&scheme) {
                continue;
            }
            result.push(id.clone());
        }
        Ok(result)
    }

    pub fn schema_uris(
        &self,
        resource: &[u16],
        document: Option<&Document>,
    ) -> Result<Vec<Vec<u16>>, EditorError> {
        if let Some(document) = document {
            if let Some(id) =
                json_schema_uri::schema_property(document, resource, self.limits.associations.uri)?
                    .filter(|id| !id.is_empty())
            {
                return Ok(vec![id]);
            }
        }
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        let mut work = 0;
        for association in &self.associations {
            add(
                &mut work,
                resource.len().checked_add(1).ok_or(EditorError::Capacity)?,
            )?;
            if work > self.limits.max_work {
                return Err(EditorError::Capacity);
            }
            if !association.entry.matches_resource(resource)? {
                continue;
            }
            for id in association.entry.uris() {
                if seen.insert(id) {
                    result.push(id.clone());
                }
            }
        }
        Ok(result)
    }

    pub fn select(
        &mut self,
        resource: &[u16],
        document: Option<&Document>,
    ) -> Result<Option<Vec<u16>>, EditorError> {
        if let Some(document) = document {
            if let Some(id) =
                json_schema_uri::schema_property(document, resource, self.limits.associations.uri)?
                    .filter(|id| !id.is_empty())
            {
                let id = json_schema_uri::normalize_id(&id, self.limits.associations.uri)?;
                self.get_or_add(id.clone())?;
                return Ok(Some(id));
            }
        }
        if let Some((cached, selection)) = &self.cached_resource {
            if cached == resource {
                return Ok(selection.clone());
            }
        }
        if resource.len() > self.limits.associations.uri.max_input_units {
            return Err(EditorError::Capacity);
        }
        let schemas = self.schema_uris(resource, None)?;
        let mut prepared = None;
        let selection = if schemas.len() == 1 {
            let id = schemas[0].clone();
            if !self.schemas.contains_key(&id) {
                prepared = Some(Handle {
                    id: id.clone(),
                    generation: 0,
                    resolve_sequence: 0,
                    unresolved: None,
                    resolved: None,
                    load_pending: false,
                    resolve_pending: false,
                    dependencies: Vec::new(),
                });
            }
            Some(id)
        } else if schemas.is_empty() {
            None
        } else {
            let encoded =
                json_schema_uri::encode_component(resource, self.limits.associations.uri)?;
            let id = COMBINED_PREFIX
                .encode_utf16()
                .chain(encoded)
                .collect::<Vec<_>>();
            if id.len() > self.limits.associations.uri.max_output_units {
                return Err(EditorError::Capacity);
            }
            let values = schemas
                .len()
                .checked_mul(2)
                .and_then(|count| count.checked_add(2))
                .ok_or(EditorError::Capacity)?;
            if values > self.limits.max_values || values > self.limits.max_work {
                return Err(EditorError::Capacity);
            }
            let mut nodes = vec![
                Value::Object(vec![Member {
                    key: "allOf".encode_utf16().collect(),
                    value: 1,
                }]),
                Value::Array(Vec::new()),
            ];
            let mut items = Vec::new();
            for uri in schemas {
                let index = nodes.len();
                items.push(index);
                nodes.push(Value::Object(vec![Member {
                    key: "$ref".encode_utf16().collect(),
                    value: index + 1,
                }]));
                nodes.push(Value::String(uri));
            }
            nodes[1] = Value::Array(items);
            let schema = Arc::new(Arena {
                root: Some(0),
                nodes,
            });
            let handle = Handle {
                id: id.clone(),
                generation: 0,
                resolve_sequence: 0,
                unresolved: Some(Arc::new(Unresolved {
                    schema,
                    errors: Vec::new(),
                })),
                resolved: None,
                load_pending: false,
                resolve_pending: false,
                dependencies: Vec::new(),
            };
            prepared = Some(handle);
            Some(id)
        };
        let kept = self.kept(
            prepared
                .as_ref()
                .map(|handle| (handle.id.as_slice(), false)),
        );
        self.check(
            &kept,
            prepared.as_ref(),
            None,
            self.registered.len(),
            Some((resource, selection.as_deref())),
            None,
        )?;
        if let Some(handle) = prepared {
            self.insert_handle(handle.id.clone(), handle, kept)?;
        }
        self.cached_resource = Some((resource.to_vec(), selection.clone()));
        Ok(selection)
    }

    fn ticket(&self, key: u64, phase: Phase) -> Result<Ticket, EditorError> {
        let handle = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        Ok(Ticket {
            owner: Arc::clone(&self.owner),
            handle: key,
            generation: handle.generation,
            phase,
            sequence: if phase == Phase::Resolve {
                handle.resolve_sequence
            } else {
                0
            },
        })
    }

    pub(crate) fn lookup_id(&self, id: &[u16]) -> Result<Vec<u16>, EditorError> {
        if self.schemas.contains_key(id) {
            return Ok(id.to_vec());
        }
        json_schema_uri::normalize_id(id, self.limits.associations.uri)
    }

    fn verify_owner(&self, ticket: &Ticket, phase: Phase) -> Result<u64, EditorError> {
        if !Arc::ptr_eq(&self.owner, &ticket.owner) || ticket.phase != phase {
            return Err(EditorError::StaleRevision);
        }
        let handle = self
            .handles
            .get(&ticket.handle)
            .ok_or(EditorError::StaleRevision)?;
        if handle.generation != ticket.generation
            || phase == Phase::Resolve && ticket.sequence != handle.resolve_sequence
        {
            return Err(EditorError::StaleRevision);
        }
        Ok(ticket.handle)
    }

    fn verify_identity(&self, ticket: &Ticket, phase: Phase) -> Result<u64, EditorError> {
        let key = self.verify_owner(ticket, phase)?;
        let handle = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        if self.schemas.get(&handle.id) != Some(&key) {
            return Err(EditorError::StaleRevision);
        }
        Ok(key)
    }

    fn verify(&self, ticket: &Ticket, phase: Phase) -> Result<u64, EditorError> {
        let key = self.verify_identity(ticket, phase)?;
        let handle = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        let pending = if phase == Phase::Load {
            handle.load_pending
        } else {
            handle.resolve_pending
        };
        if !pending {
            return Err(EditorError::StaleRevision);
        }
        Ok(key)
    }

    pub fn request_uri(&self, ticket: &Ticket) -> Result<&[u16], EditorError> {
        let key = self.verify(ticket, ticket.phase)?;
        Ok(&self.handles.get(&key).ok_or(EditorError::NotFound)?.id)
    }

    pub fn watch_load(&self, id: &[u16]) -> Result<Ticket, EditorError> {
        let id = self.lookup_id(id)?;
        let key = *self.schemas.get(&id).ok_or(EditorError::NotFound)?;
        self.ticket(key, Phase::Load)
    }

    pub fn read_unresolved(&self, ticket: &Ticket) -> Result<Option<Arc<Unresolved>>, EditorError> {
        let key = self.verify_identity(ticket, Phase::Load)?;
        Ok(self
            .handles
            .get(&key)
            .ok_or(EditorError::NotFound)?
            .unresolved
            .clone())
    }

    pub fn read_resolved(&self, ticket: &Ticket) -> Result<Option<Arc<Resolved>>, EditorError> {
        let key = self.verify_identity(ticket, Phase::Resolve)?;
        Ok(self
            .handles
            .get(&key)
            .ok_or(EditorError::NotFound)?
            .resolved
            .clone())
    }

    pub fn cancel_resolve(&mut self, ticket: &Ticket) -> Result<(), EditorError> {
        let key = self.verify_owner(ticket, Phase::Resolve)?;
        let handle = self.handles.get_mut(&key).ok_or(EditorError::NotFound)?;
        if !handle.resolve_pending {
            return Err(EditorError::StaleRevision);
        }
        let sequence = handle
            .resolve_sequence
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        handle.resolve_sequence = sequence;
        handle.resolve_pending = false;
        Ok(())
    }

    pub fn cancel_load(&mut self, ticket: &Ticket) -> Result<(), EditorError> {
        let key = self.verify_owner(ticket, Phase::Load)?;
        let handle = self.handles.get_mut(&key).ok_or(EditorError::NotFound)?;
        if !handle.load_pending {
            return Err(EditorError::StaleRevision);
        }
        let generation = handle
            .generation
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        handle.generation = generation;
        handle.load_pending = false;
        handle.resolve_pending = false;
        handle.unresolved = None;
        handle.resolved = None;
        handle.dependencies.clear();
        Ok(())
    }

    pub fn begin_load(&mut self, id: &[u16]) -> Result<LoadState, EditorError> {
        let id = self.lookup_id(id)?;
        let key = self.get_or_add(id)?;
        let handle = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        if let Some(unresolved) = &handle.unresolved {
            return Ok(LoadState::Ready(Arc::clone(unresolved)));
        }
        let ticket = self.ticket(key, Phase::Load)?;
        if handle.load_pending {
            return Ok(LoadState::Pending(ticket));
        }
        self.handles
            .get_mut(&key)
            .ok_or(EditorError::NotFound)?
            .load_pending = true;
        Ok(LoadState::Start(ticket))
    }

    pub fn begin_resolve(&mut self, id: &[u16]) -> Result<ResolveState, EditorError> {
        let id = self.lookup_id(id)?;
        let key = self.get_or_add(id.clone())?;
        let handle = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        if let Some(resolved) = &handle.resolved {
            return Ok(ResolveState::Ready(Arc::clone(resolved)));
        }
        if handle.resolve_pending {
            return Ok(ResolveState::Pending(self.ticket(key, Phase::Resolve)?));
        }
        let sequence = handle
            .resolve_sequence
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let source = self.begin_load(&id)?;
        let handle = self.handles.get_mut(&key).ok_or(EditorError::NotFound)?;
        handle.resolve_sequence = sequence;
        handle.resolve_pending = true;
        let ticket = self.ticket(key, Phase::Resolve)?;
        Ok(ResolveState::Start { ticket, source })
    }

    pub fn publish_load(
        &mut self,
        ticket: &Ticket,
        unresolved: Unresolved,
    ) -> Result<(), EditorError> {
        let key = self.verify(ticket, Phase::Load)?;
        if !is_schema(&unresolved.schema) {
            return Err(EditorError::InvalidBoundary);
        }
        let current = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        let candidate = Handle {
            id: current.id.clone(),
            generation: current.generation,
            resolve_sequence: current.resolve_sequence,
            unresolved: Some(Arc::new(unresolved)),
            resolved: current.resolved.clone(),
            load_pending: false,
            resolve_pending: current.resolve_pending,
            dependencies: current.dependencies.clone(),
        };
        let mut kept = self.kept(None);
        kept.remove(&key);
        self.check(
            &kept,
            Some(&candidate),
            None,
            self.registered.len(),
            self.cache(),
            None,
        )?;
        self.handles.insert(key, candidate);
        Ok(())
    }

    pub fn publish_resolved(
        &mut self,
        ticket: &Ticket,
        resolved: Resolved,
        dependencies: &[Vec<u16>],
    ) -> Result<(), EditorError> {
        let key = self.verify(ticket, Phase::Resolve)?;
        if !resolved.external.is_empty() {
            return Err(EditorError::InvalidBoundary);
        }
        if dependencies.len() > self.limits.max_handles {
            return Err(EditorError::Capacity);
        }
        let mut normalized = Vec::new();
        for dependency in dependencies {
            let dependency =
                json_schema_uri::normalize_id(dependency, self.limits.associations.uri)?;
            if !normalized.contains(&dependency) {
                normalized.push(dependency);
            }
        }
        let current = self.handles.get(&key).ok_or(EditorError::NotFound)?;
        if current.unresolved.is_none() || current.load_pending {
            return Err(EditorError::InvalidBoundary);
        }
        let candidate = Handle {
            id: current.id.clone(),
            generation: current.generation,
            resolve_sequence: current.resolve_sequence,
            unresolved: current.unresolved.clone(),
            resolved: Some(Arc::new(resolved)),
            load_pending: current.load_pending,
            resolve_pending: false,
            dependencies: normalized,
        };
        let mut kept = self.kept(None);
        kept.remove(&key);
        self.check(
            &kept,
            Some(&candidate),
            None,
            self.registered.len(),
            self.cache(),
            None,
        )?;
        self.handles.insert(key, candidate);
        Ok(())
    }

    pub fn publish_graph(
        &mut self,
        ticket: &Ticket,
        resolved: Resolved,
        dependencies: &[(Ticket, Vec<Vec<u16>>)],
    ) -> Result<(), EditorError> {
        let root = self.verify(ticket, Phase::Resolve)?;
        if !resolved.external.is_empty() {
            return Err(EditorError::InvalidBoundary);
        }
        if dependencies.len() > self.limits.max_handles {
            return Err(EditorError::Capacity);
        }
        let mut candidates = BTreeMap::<u64, Handle>::new();
        for (watch, uris) in dependencies {
            let key = self.verify_identity(watch, Phase::Load)?;
            if uris.len() > self.limits.max_handles {
                return Err(EditorError::Capacity);
            }
            let current = self.handles.get(&key).ok_or(EditorError::NotFound)?;
            if current.unresolved.is_none() || current.load_pending {
                return Err(EditorError::InvalidBoundary);
            }
            let candidate = candidates.entry(key).or_insert_with(|| current.clone());
            for uri in uris {
                let uri = json_schema_uri::normalize_id(uri, self.limits.associations.uri)?;
                if !candidate.dependencies.contains(&uri) {
                    candidate.dependencies.push(uri);
                }
            }
            if candidate.dependencies.len() > self.limits.max_handles {
                return Err(EditorError::Capacity);
            }
        }
        let current = self.handles.get(&root).ok_or(EditorError::NotFound)?;
        if current.unresolved.is_none() || current.load_pending {
            return Err(EditorError::InvalidBoundary);
        }
        let candidate = candidates.entry(root).or_insert_with(|| current.clone());
        candidate.resolved = Some(Arc::new(resolved));
        candidate.resolve_pending = false;
        let mut kept = self.kept(None);
        for key in candidates.keys() {
            kept.remove(key);
        }
        let handles = candidates.values().collect::<Vec<_>>();
        self.check_handles(
            &kept,
            &handles,
            None,
            self.registered.len(),
            self.cache(),
            None,
        )?;
        for (key, candidate) in candidates {
            self.handles.insert(key, candidate);
        }
        Ok(())
    }

    pub fn on_resource_change(&mut self, id: &[u16]) -> Result<bool, EditorError> {
        let id = json_schema_uri::normalize_id(id, self.limits.associations.uri)?;
        let mut pending = vec![id];
        let mut changed = BTreeSet::new();
        let mut work = 0;
        while let Some(id) = pending.pop() {
            for &key in self.schemas.values() {
                add(&mut work, 1)?;
                if work > self.limits.max_work {
                    return Err(EditorError::Capacity);
                }
                if changed.contains(&key) {
                    continue;
                }
                let handle = self.handles.get(&key).ok_or(EditorError::NotFound)?;
                if handle.id == id || handle.dependencies.contains(&id) {
                    handle
                        .generation
                        .checked_add(1)
                        .ok_or(EditorError::RevisionOverflow)?;
                    changed.insert(key);
                    if handle.id != id {
                        pending.push(handle.id.clone());
                    }
                }
            }
        }
        let mut has_changes = false;
        self.cached_resource = None;
        for key in changed {
            let handle = self.handles.get_mut(&key).ok_or(EditorError::NotFound)?;
            has_changes |= handle.unresolved.is_some() || handle.load_pending;
            handle.generation += 1;
            handle.unresolved = None;
            handle.resolved = None;
            handle.load_pending = false;
            handle.resolve_pending = false;
            handle.dependencies.clear();
        }
        Ok(has_changes)
    }
}

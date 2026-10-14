use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use crate::document::EditorError;
use crate::json_schema_refs::{self, External, Resolved, Resolver};
use crate::json_schema_service::{LoadState, ResolveState, Service, Ticket, Unresolved};
use crate::json_schema_uri;
use crate::json_value::{Arena, Member, Value};

const COLON: u16 = 0x3a;
const SLASH: u16 = 0x2f;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub refs: json_schema_refs::Limits,
    pub uri: json_schema_uri::Limits,
    pub max_documents: usize,
    pub max_requests: usize,
    pub max_uri_units: usize,
}

#[derive(Debug, Clone)]
pub struct Fetch {
    pub uri: Vec<u16>,
    pub referrer: Option<Vec<u16>>,
    pub ticket: Ticket,
}

pub enum Start {
    Ready(Arc<Resolved>),
    Pending(Ticket),
    Job(Job),
}

pub enum Progress {
    Ready(Arc<Resolved>),
    Waiting { job: Job, requests: Vec<Fetch> },
}

struct Document {
    uri: Vec<u16>,
    watch: Ticket,
    source: Option<Arc<Unresolved>>,
    root: Option<usize>,
    anchors: Option<BTreeMap<Vec<u16>, usize>>,
    dependencies: Vec<Vec<u16>>,
}

struct Link {
    reference: External,
    parent: usize,
    uri: Option<Vec<u16>>,
}

pub struct Job {
    ticket: Ticket,
    documents: Vec<Document>,
    by_uri: BTreeMap<Vec<u16>, usize>,
    engine: Option<Resolver>,
    pending: VecDeque<Link>,
    outgoing: Vec<Fetch>,
    visited: BTreeSet<(usize, Vec<u16>, Vec<u16>)>,
    value_units: usize,
    uri_units: usize,
    requests: usize,
    pre_work: usize,
    workspace_context: bool,
    limits: Limits,
}

fn source_units(schema: &Arena, limits: json_schema_refs::Limits) -> Result<usize, EditorError> {
    if schema.values().len() > limits.values.max_values {
        return Err(EditorError::Capacity);
    }
    let mut units = 0usize;
    for value in schema.values() {
        let size = match value {
            Value::String(value) => value.len(),
            Value::Object(members) => members.iter().try_fold(0usize, |count, member| {
                count
                    .checked_add(member.key.len())
                    .ok_or(EditorError::Capacity)
            })?,
            _ => 0,
        };
        units = units.checked_add(size).ok_or(EditorError::Capacity)?;
        if units > limits.values.max_text_units {
            return Err(EditorError::Capacity);
        }
    }
    Ok(units)
}

fn network_uri(uri: &[u16]) -> bool {
    if !uri
        .first()
        .is_some_and(|unit| matches!(unit,0x41..=0x5a|0x61..=0x7a))
    {
        return false;
    }
    let Some(end) = uri
        .iter()
        .position(|unit| !matches!(unit,0x41..=0x5a|0x61..=0x7a|0x30..=0x39|0x2b|0x2d|0x2e))
    else {
        return false;
    };
    uri.get(end..end + 3) == Some(&[COLON, SLASH, SLASH])
}

impl Job {
    pub fn start(
        service: &mut Service<'_>,
        uri: &[u16],
        workspace_context: bool,
        limits: Limits,
    ) -> Result<Start, EditorError> {
        if uri.len() > limits.uri.max_input_units || limits.max_documents == 0 {
            return Err(EditorError::Capacity);
        }
        let uri = service.lookup_id(uri)?;
        if uri.len() > limits.max_uri_units || uri.len() > limits.uri.max_input_units {
            return Err(EditorError::Capacity);
        }
        let state = service.begin_resolve(&uri)?;
        let ResolveState::Start { ticket, source } = state else {
            return Ok(match state {
                ResolveState::Ready(ready) => Start::Ready(ready),
                ResolveState::Pending(ticket) => Start::Pending(ticket),
                ResolveState::Start { .. } => return Err(EditorError::InvalidBoundary),
            });
        };
        let watch = service.watch_load(&uri)?;
        let outgoing = match source {
            LoadState::Start(ticket) => vec![Fetch {
                uri: uri.clone(),
                referrer: None,
                ticket,
            }],
            LoadState::Ready(_) | LoadState::Pending(_) => Vec::new(),
        };
        if outgoing.len() > limits.max_requests {
            for fetch in &outgoing {
                let _ = service.cancel_load(&fetch.ticket);
            }
            let _ = service.cancel_resolve(&ticket);
            return Err(EditorError::Capacity);
        }
        let requests = outgoing.len();
        let uri_units = uri.len();
        let by_uri = BTreeMap::from([(uri.clone(), 0)]);
        Ok(Start::Job(Self {
            ticket,
            documents: vec![Document {
                uri,
                watch,
                source: None,
                root: None,
                anchors: None,
                dependencies: Vec::new(),
            }],
            by_uri,
            engine: None,
            pending: VecDeque::new(),
            outgoing,
            visited: BTreeSet::new(),
            value_units: 0,
            uri_units,
            requests,
            pre_work: 0,
            workspace_context,
            limits,
        }))
    }

    fn charge(&mut self, count: usize) -> Result<(), EditorError> {
        if let Some(engine) = &mut self.engine {
            return engine.charge(count);
        }
        self.pre_work = self
            .pre_work
            .checked_add(count)
            .ok_or(EditorError::Capacity)?;
        if self.pre_work > self.limits.refs.max_work {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn fresh(&mut self, service: &Service<'_>) -> Result<(), EditorError> {
        service.request_uri(&self.ticket)?;
        self.charge(self.documents.len())?;
        for document in &mut self.documents {
            let current = service.read_unresolved(&document.watch)?;
            if let Some(previous) = &document.source {
                if !current
                    .as_ref()
                    .is_some_and(|current| Arc::ptr_eq(previous, current))
                {
                    return Err(EditorError::StaleRevision);
                }
            } else {
                document.source = current;
            }
        }
        Ok(())
    }

    fn collect(&mut self, parent: usize) -> Result<(), EditorError> {
        let engine = self.engine.as_mut().ok_or(EditorError::InvalidBoundary)?;
        for reference in std::mem::take(&mut engine.result.external) {
            self.requests = self.requests.checked_add(1).ok_or(EditorError::Capacity)?;
            if self.requests > self.limits.max_requests {
                return Err(EditorError::Capacity);
            }
            self.pending.push_back(Link {
                reference,
                parent,
                uri: None,
            });
        }
        Ok(())
    }

    fn traverse(&mut self, node: usize, parent: usize) -> Result<(), EditorError> {
        let root = self.documents[parent]
            .root
            .ok_or(EditorError::InvalidBoundary)?;
        let engine = self.engine.as_mut().ok_or(EditorError::InvalidBoundary)?;
        engine.anchors = self.documents[parent].anchors.take();
        engine.traverse(node, root, &self.documents[parent].uri)?;
        self.documents[parent].anchors = engine.anchors.take();
        self.collect(parent)
    }

    fn initialize(&mut self) -> Result<bool, EditorError> {
        let Some(source) = &self.documents[0].source else {
            return Ok(false);
        };
        self.value_units = source_units(&source.schema, self.limits.refs)?;
        let (mut engine, is_old) =
            json_schema_refs::create_resolver((*source.schema).clone(), self.limits.refs)?;
        engine.charge(self.pre_work)?;
        if !is_old {
            for error in &source.errors {
                engine.message(&[error])?;
            }
        }
        let root = engine
            .result
            .schema
            .root()
            .ok_or(EditorError::InvalidBoundary)?;
        self.documents[0].root = Some(root);
        self.engine = Some(engine);
        if !is_old {
            self.traverse(root, 0)?;
        }
        Ok(true)
    }

    fn document(
        &mut self,
        service: &mut Service<'_>,
        uri: &[u16],
        parent: usize,
    ) -> Result<usize, EditorError> {
        if let Some(&index) = self.by_uri.get(uri) {
            return Ok(index);
        }
        let total = self
            .uri_units
            .checked_add(uri.len())
            .and_then(|total| total.checked_add(self.documents[parent].uri.len()))
            .ok_or(EditorError::Capacity)?;
        if self.documents.len() >= self.limits.max_documents || total > self.limits.max_uri_units {
            return Err(EditorError::Capacity);
        }
        let state = service.begin_load(uri)?;
        let watch = service.watch_load(uri)?;
        let source = match state {
            LoadState::Ready(source) => Some(source),
            LoadState::Start(ticket) => {
                self.outgoing.push(Fetch {
                    uri: uri.to_vec(),
                    referrer: Some(self.documents[parent].uri.clone()),
                    ticket,
                });
                None
            }
            LoadState::Pending(_) => None,
        };
        let index = self.documents.len();
        self.documents.push(Document {
            uri: uri.to_vec(),
            watch,
            source,
            root: None,
            anchors: None,
            dependencies: Vec::new(),
        });
        self.by_uri.insert(uri.to_vec(), index);
        self.uri_units = total;
        Ok(index)
    }

    fn import(&mut self, index: usize) -> Result<(), EditorError> {
        if self.documents[index].root.is_some() {
            return Ok(());
        }
        let source = self.documents[index]
            .source
            .as_ref()
            .ok_or(EditorError::InvalidBoundary)?;
        let units = source_units(&source.schema, self.limits.refs)?;
        let total = self
            .value_units
            .checked_add(units)
            .ok_or(EditorError::Capacity)?;
        if total > self.limits.refs.values.max_text_units {
            return Err(EditorError::Capacity);
        }
        let engine = self.engine.as_mut().ok_or(EditorError::InvalidBoundary)?;
        let offset = engine.result.schema.nodes.len();
        let count = offset
            .checked_add(source.schema.values().len())
            .ok_or(EditorError::Capacity)?;
        if count > self.limits.refs.values.max_values {
            return Err(EditorError::Capacity);
        }
        engine.charge(source.schema.values().len())?;
        let remap = |id: usize| -> Result<usize, EditorError> {
            if id >= source.schema.values().len() {
                return Err(EditorError::InvalidBoundary);
            }
            id.checked_add(offset).ok_or(EditorError::Capacity)
        };
        let root = remap(source.schema.root().ok_or(EditorError::InvalidBoundary)?)?;
        for value in source.schema.values() {
            let value = match value {
                Value::Array(items) => Value::Array(
                    items
                        .iter()
                        .map(|&id| remap(id))
                        .collect::<Result<_, _>>()?,
                ),
                Value::Object(members) => Value::Object(
                    members
                        .iter()
                        .map(|member| {
                            Ok(Member {
                                key: member.key.clone(),
                                value: remap(member.value)?,
                            })
                        })
                        .collect::<Result<_, EditorError>>()?,
                ),
                _ => value.clone(),
            };
            engine.result.schema.nodes.push(value);
        }
        self.documents[index].root = Some(root);
        self.value_units = total;
        Ok(())
    }

    fn section(
        &mut self,
        document: usize,
        fragment: Option<&[u16]>,
    ) -> Result<Option<usize>, EditorError> {
        let fragment = fragment.unwrap_or(&[]);
        let value = match json_schema_refs::decode(fragment) {
            Ok(value) => value,
            Err(EditorError::InvalidBoundary) => return Ok(None),
            Err(error) => return Err(error),
        };
        let root = self.documents[document]
            .root
            .ok_or(EditorError::InvalidBoundary)?;
        if value.is_empty() {
            return Ok(Some(root));
        }
        let engine = self.engine.as_mut().ok_or(EditorError::InvalidBoundary)?;
        if value[0] == SLASH {
            let mut remaining = engine.limits.max_work - engine.work;
            let result =
                json_schema_refs::pointer(&engine.result.schema, root, &value, &mut remaining);
            engine.work = engine.limits.max_work - remaining;
            return match result {
                Err(EditorError::InvalidBoundary) => Ok(None),
                result => result,
            };
        }
        if self.documents[document].anchors.is_none() {
            engine.build_anchors(root)?;
            self.documents[document].anchors = engine.anchors.take();
        }
        Ok(self.documents[document]
            .anchors
            .as_ref()
            .and_then(|anchors| anchors.get(&value))
            .copied())
    }

    fn link(
        &mut self,
        service: &mut Service<'_>,
        mut link: Link,
        access: &mut impl FnMut(&[u16], &[u16]) -> bool,
    ) -> Result<Option<Link>, EditorError> {
        self.charge(1)?;
        let uri = match link.uri.take() {
            Some(uri) => uri,
            None => {
                let uri = if self.workspace_context && !network_uri(&link.reference.uri) {
                    json_schema_uri::resolve_relative_path(
                        &link.reference.uri,
                        &self.documents[link.parent].uri,
                        self.limits.uri,
                    )?
                } else {
                    link.reference.uri.clone()
                };
                let normalized = json_schema_uri::normalize_id(&uri, self.limits.uri)?;
                self.charge(normalized.len())?;
                normalized
            }
        };
        let access_units = uri
            .len()
            .checked_add(self.documents[link.parent].uri.len())
            .ok_or(EditorError::Capacity)?;
        self.charge(access_units)?;
        if !access(&self.documents[link.parent].uri, &uri) {
            self.engine
                .as_mut()
                .ok_or(EditorError::InvalidBoundary)?
                .message(&[
                    &"Schema reference access denied: '"
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                    &uri,
                    &"'.".encode_utf16().collect::<Vec<_>>(),
                ])?;
            return Ok(None);
        }
        let document = self.document(service, &uri, link.parent)?;
        if self.documents[document].source.is_none() {
            link.uri = Some(uri);
            return Ok(Some(link));
        }
        if !self.documents[link.parent].dependencies.contains(&uri) {
            self.documents[link.parent].dependencies.push(uri.clone());
        }
        let key = (
            link.reference.target,
            uri.clone(),
            link.reference.fragment.clone().unwrap_or_default(),
        );
        if !self.visited.insert(key) {
            return Ok(None);
        }
        self.import(document)?;
        let error = self.documents[document]
            .source
            .as_ref()
            .and_then(|source| source.errors.first());
        if let Some(error) = error {
            let engine = self.engine.as_mut().ok_or(EditorError::InvalidBoundary)?;
            let fragment = link.reference.fragment.as_deref().unwrap_or(&[]);
            if fragment.is_empty() {
                engine.message(&[
                    &"Problems loading reference '"
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                    &uri,
                    &"': ".encode_utf16().collect::<Vec<_>>(),
                    error,
                ])?;
            } else {
                engine.message(&[
                    &"Problems loading reference '"
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                    &uri,
                    &[0x23],
                    fragment,
                    &"': ".encode_utf16().collect::<Vec<_>>(),
                    error,
                ])?;
            }
        }
        if let Some(section) = self.section(document, link.reference.fragment.as_deref())? {
            self.engine
                .as_mut()
                .ok_or(EditorError::InvalidBoundary)?
                .merge(link.reference.target, section)?;
        } else {
            self.engine
                .as_mut()
                .ok_or(EditorError::InvalidBoundary)?
                .unresolved(link.reference.fragment.as_deref().unwrap_or(&[]), &uri)?;
        }
        self.traverse(link.reference.target, document)?;
        Ok(None)
    }

    fn drive(
        &mut self,
        service: &mut Service<'_>,
        access: &mut impl FnMut(&[u16], &[u16]) -> bool,
    ) -> Result<bool, EditorError> {
        self.fresh(service)?;
        if self.engine.is_none() && !self.initialize()? {
            return Ok(false);
        }
        loop {
            let count = self.pending.len();
            if count == 0 {
                return Ok(true);
            }
            let mut progressed = false;
            for _ in 0..count {
                let link = self
                    .pending
                    .pop_front()
                    .ok_or(EditorError::InvalidBoundary)?;
                if let Some(link) = self.link(service, link, access)? {
                    self.pending.push_back(link);
                } else {
                    progressed = true;
                }
            }
            if !progressed {
                return Ok(false);
            }
        }
    }

    fn cancel(&self, service: &mut Service<'_>) {
        for fetch in &self.outgoing {
            let _ = service.cancel_load(&fetch.ticket);
        }
        let _ = service.cancel_resolve(&self.ticket);
    }

    pub fn abort(self, service: &mut Service<'_>) {
        self.cancel(service)
    }

    pub fn advance(self, service: &mut Service<'_>) -> Result<Progress, EditorError> {
        self.advance_with_access(service, |_, _| true)
    }

    pub fn advance_with_access(
        mut self,
        service: &mut Service<'_>,
        mut access: impl FnMut(&[u16], &[u16]) -> bool,
    ) -> Result<Progress, EditorError> {
        match self.drive(service, &mut access) {
            Ok(false) => {
                let requests = std::mem::take(&mut self.outgoing);
                Ok(Progress::Waiting {
                    job: self,
                    requests,
                })
            }
            Err(error) => {
                self.cancel(service);
                Err(error)
            }
            Ok(true) => {
                if let Err(error) = self.fresh(service) {
                    self.cancel(service);
                    return Err(error);
                }
                let dependencies = self
                    .documents
                    .iter()
                    .map(|document| (document.watch.clone(), document.dependencies.clone()))
                    .collect::<Vec<_>>();
                let Some(engine) = self.engine.take() else {
                    self.cancel(service);
                    return Err(EditorError::InvalidBoundary);
                };
                if let Err(error) =
                    service.publish_graph(&self.ticket, engine.result, &dependencies)
                {
                    self.cancel(service);
                    return Err(error);
                }
                let ready = service
                    .read_resolved(&self.ticket)?
                    .ok_or(EditorError::InvalidBoundary)?;
                Ok(Progress::Ready(ready))
            }
        }
    }
}

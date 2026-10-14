use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use taide_lsp::native::Failure;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::document_symbols::DocumentSymbols;
use taide_native_editor::lsp::byte_to_position;
use taide_native_editor::sticky_model::StickyModel;
use taide_native_ui::command_palette::SymbolIndex;
use tokio::sync::watch;

const REFRESH_DEBOUNCE: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProviderIdentity {
    pub owner: crate::diagnostics::Owner,
    pub generation: u64,
    pub capability_revision: u64,
}

pub struct Group {
    pub provider: ProviderIdentity,
    pub model: Arc<DocumentSymbols>,
}

pub struct Response {
    pub palette_provider: Option<ProviderIdentity>,
    pub palette: Arc<DocumentSymbols>,
    pub groups: Vec<Group>,
    pub error: Option<Failure>,
}

#[derive(Clone)]
pub struct Request {
    pub project: ProjectId,
    pub snapshot: DocumentSnapshot,
    pub generation: u64,
    pub cancelled: watch::Receiver<bool>,
}

impl Request {
    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }
}

struct Entry {
    project: ProjectId,
    snapshot: DocumentSnapshot,
    generation: u64,
    providers: HashSet<ProviderIdentity>,
    model: Option<Arc<DocumentSymbols>>,
    sticky: Option<Arc<StickyModel>>,
    sticky_score: usize,
    preferred: Option<crate::diagnostics::Owner>,
    cancel: Option<watch::Sender<bool>>,
    complete: bool,
    due: Instant,
}

struct Builtin {
    generation: u64,
    model: Arc<DocumentSymbols>,
    sticky_score: usize,
}

impl Entry {
    fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.send_replace(true);
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<(ProjectId, DocumentId), Entry>,
    builtin: HashMap<DocumentId, Builtin>,
    builtin_failures: HashMap<DocumentId, DocumentSnapshot>,
    generation: u64,
}

impl State {
    pub(crate) fn install_builtin(
        &mut self,
        current: &DocumentSnapshot,
        model: Arc<DocumentSymbols>,
    ) -> Result<(), taide_native_editor::document::EditorError> {
        if !model.describes(current) {
            return Err(taide_native_editor::document::EditorError::StaleRevision);
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(taide_native_editor::document::EditorError::RevisionOverflow)?;
        self.builtin_failures.remove(&current.id);
        self.builtin.insert(
            current.id,
            Builtin {
                generation: self.generation,
                sticky_score: span_score(current, &model),
                model,
            },
        );
        Ok(())
    }

    pub(crate) fn retain_builtin(&mut self, documents: &HashSet<DocumentId>) {
        self.builtin.retain(|id, _| documents.contains(id));
        self.builtin_failures.retain(|id, _| documents.contains(id));
    }

    pub(crate) fn fail_builtin(&mut self, current: &DocumentSnapshot) {
        self.builtin.remove(&current.id);
        self.builtin_failures.insert(current.id, current.clone());
    }

    pub(crate) fn builtin_model(
        &self,
        current: &DocumentSnapshot,
    ) -> Option<&Arc<DocumentSymbols>> {
        self.builtin
            .get(&current.id)
            .map(|entry| &entry.model)
            .filter(|model| model.describes(current))
    }

    pub(crate) fn builtin_sticky(&self, current: &DocumentSnapshot) -> Option<Arc<StickyModel>> {
        self.builtin_model(current)?.sticky_model().cloned()
    }
    pub(crate) fn observe(
        &mut self,
        project: &ProjectId,
        snapshot: DocumentSnapshot,
        providers: HashSet<ProviderIdentity>,
        now: Instant,
    ) -> Option<Request> {
        let entry = self
            .entries
            .entry((project.clone(), snapshot.id))
            .or_insert_with(|| Entry {
                project: project.clone(),
                snapshot: snapshot.clone(),
                generation: 0,
                providers: providers.clone(),
                model: None,
                sticky: None,
                sticky_score: 0,
                preferred: None,
                cancel: None,
                complete: false,
                due: now,
            });
        let identity_changed = entry.project != *project
            || entry.snapshot.key != snapshot.key
            || entry.snapshot.metadata.language_id != snapshot.metadata.language_id;
        let revision_changed = entry.snapshot.revision != snapshot.revision;
        let provider_changed = entry.providers != providers && entry.cancel.is_none();
        if identity_changed || revision_changed || provider_changed {
            entry.cancel();
            entry.project = project.clone();
            entry.snapshot = snapshot.clone();
            entry.model = None;
            entry.sticky = None;
            entry.sticky_score = 0;
            entry.complete = false;
            entry.due = if revision_changed && !identity_changed {
                now + REFRESH_DEBOUNCE
            } else {
                now
            };
        }
        entry.providers = providers;
        if entry.complete || entry.cancel.is_some() || now < entry.due {
            return None;
        }
        self.generation = self.generation.checked_add(1)?;
        entry.generation = self.generation;
        let (cancel, cancelled) = watch::channel(false);
        entry.cancel = Some(cancel);
        Some(Request {
            project: project.clone(),
            snapshot,
            generation: self.generation,
            cancelled,
        })
    }

    pub(crate) fn accept(
        &mut self,
        request: &Request,
        current: &DocumentSnapshot,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> Result<bool, Failure> {
        let Some(entry) = self
            .entries
            .get_mut(&(request.project.clone(), current.id))
            .filter(|entry| {
                entry.project == request.project
                    && entry.generation == request.generation
                    && !request.is_cancelled()
                    && request.snapshot.id == current.id
                    && request.snapshot.key == current.key
                    && request.snapshot.revision == current.revision
                    && request.snapshot.metadata.language_id == current.metadata.language_id
            })
        else {
            return Ok(false);
        };
        entry.cancel.take();
        entry.providers = providers;
        match result {
            Ok(response)
                if response.palette.describes(current)
                    && response
                        .palette_provider
                        .is_none_or(|provider| entry.providers.contains(&provider))
                    && response.groups.iter().all(|group| {
                        group.model.describes(current) && entry.providers.contains(&group.provider)
                    }) =>
            {
                let groups = response
                    .groups
                    .iter()
                    .filter(|group| !group.model.symbols().is_empty())
                    .collect::<Vec<_>>();
                let preferred = groups
                    .iter()
                    .copied()
                    .find(|group| Some(group.provider.owner) == entry.preferred);
                let chosen = preferred.or_else(|| {
                    groups
                        .iter()
                        .copied()
                        .fold(None, |best: Option<&Group>, group| {
                            if best.is_none_or(|best| {
                                span_score(current, &group.model) > span_score(current, &best.model)
                            }) {
                                Some(group)
                            } else {
                                best
                            }
                        })
                });
                entry.sticky = chosen.and_then(|group| group.model.sticky_model().cloned());
                entry.sticky_score = chosen.map_or(0, |group| span_score(current, &group.model));
                entry.preferred = chosen.map(|group| group.provider.owner);
                entry.model = Some(response.palette);
                entry.complete = true;
                match response.error {
                    Some(error) => Err(error),
                    None => Ok(true),
                }
            }
            Ok(_) => {
                entry.complete = false;
                entry.due = Instant::now();
                Ok(false)
            }
            Err(error) => {
                entry.complete = !matches!(
                    error,
                    Failure::StaleGeneration
                        | Failure::StaleRevision
                        | Failure::Restarted
                        | Failure::Cancelled
                );
                entry.due = Instant::now();
                Err(error)
            }
        }
    }

    pub(crate) fn failed(&mut self, request: &Request) {
        if let Some(entry) = self
            .entries
            .get_mut(&(request.project.clone(), request.snapshot.id))
            .filter(|entry| entry.generation == request.generation)
        {
            entry.cancel();
            entry.complete = true;
        }
    }

    pub(crate) fn retain(&mut self, documents: &HashSet<(ProjectId, DocumentId)>) {
        self.entries.retain(|document, entry| {
            if documents.contains(document) {
                return true;
            }
            entry.cancel();
            false
        });
    }

    pub(crate) fn next_refresh(&self, now: Instant) -> Option<Duration> {
        self.entries
            .values()
            .filter(|entry| !entry.complete && entry.cancel.is_none())
            .map(|entry| entry.due.saturating_duration_since(now))
            .min()
    }

    pub(crate) fn model(
        &self,
        project: &ProjectId,
        current: &DocumentSnapshot,
    ) -> Option<&Arc<DocumentSymbols>> {
        let external = self
            .entries
            .get(&(project.clone(), current.id))
            .and_then(|entry| entry.model.as_ref())
            .filter(|model| model.describes(current));
        external
            .filter(|model| !model.symbols().is_empty())
            .or_else(|| self.builtin_model(current))
            .or(external)
    }

    pub(crate) fn sticky(
        &self,
        project: &ProjectId,
        current: &DocumentSnapshot,
    ) -> Option<Arc<StickyModel>> {
        if let Some(native) = self.builtin.get(&current.id)
            && native.model.describes(current)
            && native.model.sticky_model().is_some()
            && self
                .entries
                .get(&(project.clone(), current.id))
                .is_none_or(|entry| {
                    !entry.snapshot.key.eq(&current.key)
                        || entry.snapshot.revision != current.revision
                        || native.sticky_score > entry.sticky_score
                })
        {
            return native.model.sticky_model().cloned();
        }
        let model = self.model(project, current)?;
        if self
            .builtin_model(current)
            .is_some_and(|builtin| Arc::ptr_eq(builtin, model))
        {
            return model.sticky_model().cloned();
        }
        self.entries
            .get(&(project.clone(), current.id))?
            .sticky
            .clone()
    }

    pub(crate) fn palette<'a>(
        &'a self,
        project: Option<&ProjectId>,
        current: Option<&DocumentSnapshot>,
    ) -> SymbolIndex<'a> {
        let Some(current) = current else {
            return SymbolIndex::default();
        };
        if current.metadata.language_id == "json"
            && current.metadata.tier != taide_model::file::FileSizeTier::Normal
        {
            return SymbolIndex::default();
        }
        let external = project
            .and_then(|project| self.entries.get(&(project.clone(), current.id)))
            .filter(|entry| {
                entry
                    .model
                    .as_ref()
                    .is_some_and(|model| model.describes(current) && !model.symbols().is_empty())
            });
        if external.is_none()
            && let Some(entry) = self.builtin.get(&current.id)
            && entry.model.describes(current)
        {
            return SymbolIndex {
                entries: Some(entry.model.symbols()),
                generation: entry.generation,
                is_pending: false,
            };
        }
        if external.is_none()
            && self
                .builtin_failures
                .get(&current.id)
                .is_some_and(|failed| {
                    failed.key == current.key
                        && failed.revision == current.revision
                        && failed.metadata.language_id == current.metadata.language_id
                })
            && project
                .and_then(|project| self.entries.get(&(project.clone(), current.id)))
                .is_none_or(|entry| entry.complete && entry.cancel.is_none())
        {
            return SymbolIndex::default();
        }
        let Some(project) = project else {
            return SymbolIndex::default();
        };
        let Some(entry) = self.entries.get(&(project.clone(), current.id)) else {
            return SymbolIndex {
                is_pending: true,
                ..Default::default()
            };
        };
        SymbolIndex {
            entries: self.model(project, current).map(|model| model.symbols()),
            generation: entry.generation,
            is_pending: !entry.complete || entry.cancel.is_some(),
        }
    }

    pub(crate) fn position(
        &self,
        project: &ProjectId,
        current: &DocumentSnapshot,
        generation: u64,
        index: usize,
    ) -> Option<crate::editor_reveal::Position> {
        if self.palette(Some(project), Some(current)).generation != generation {
            return None;
        }
        if let Some(entry) = self.builtin.get(&current.id)
            && entry.generation == generation
            && entry.model.describes(current)
        {
            let position =
                byte_to_position(current, entry.model.symbols().get(index)?.selection.start)
                    .ok()?;
            return Some(crate::editor_reveal::Position {
                line: f64::from(position.line) + 1.0,
                column: f64::from(position.character) + 1.0,
            });
        }
        let entry = self
            .entries
            .get(&(project.clone(), current.id))
            .filter(|entry| entry.project == *project && entry.generation == generation)?;
        let model = entry
            .model
            .as_ref()
            .filter(|model| model.describes(current))?;
        let position =
            byte_to_position(current, model.symbols().get(index)?.selection.start).ok()?;
        Some(crate::editor_reveal::Position {
            line: f64::from(position.line) + 1.0,
            column: f64::from(position.character) + 1.0,
        })
    }
}

fn span_score(current: &DocumentSnapshot, model: &DocumentSymbols) -> usize {
    model
        .symbols()
        .iter()
        .map(|symbol| {
            current
                .rope
                .byte_to_line(symbol.bytes.end)
                .saturating_sub(current.rope.byte_to_line(symbol.selection.start))
        })
        .sum()
}

#[cfg(test)]
#[path = "editor-symbols-tests.rs"]
mod tests;

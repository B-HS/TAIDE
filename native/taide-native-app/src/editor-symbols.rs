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
    preferred: Option<crate::diagnostics::Owner>,
    cancel: Option<watch::Sender<bool>>,
    complete: bool,
    due: Instant,
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
    generation: u64,
}

impl State {
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
                            let score = |group: &Group| {
                                group
                                    .model
                                    .symbols()
                                    .iter()
                                    .map(|symbol| {
                                        current.rope.byte_to_line(symbol.bytes.end).saturating_sub(
                                            current.rope.byte_to_line(symbol.selection.start),
                                        )
                                    })
                                    .sum::<usize>()
                            };
                            if best.is_none_or(|best| score(group) > score(best)) {
                                Some(group)
                            } else {
                                best
                            }
                        })
                });
                entry.sticky = chosen.and_then(|group| group.model.sticky_model().cloned());
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
        self.entries
            .get(&(project.clone(), current.id))?
            .model
            .as_ref()
            .filter(|model| model.describes(current))
    }

    pub(crate) fn sticky(
        &self,
        project: &ProjectId,
        current: &DocumentSnapshot,
    ) -> Option<Arc<StickyModel>> {
        self.model(project, current)?;
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

#[cfg(test)]
#[path = "editor-symbols-tests.rs"]
mod tests;

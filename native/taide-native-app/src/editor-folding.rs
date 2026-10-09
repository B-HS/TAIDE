use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use taide_lsp::native::Failure;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::syntax_folding::{SyntaxFoldRange, SyntaxFolds};
use tokio::sync::watch;

use crate::editor_symbols::ProviderIdentity;

const REFRESH_DEBOUNCE: Duration = Duration::from_millis(200);

pub struct Group {
    pub provider: ProviderIdentity,
    pub ranges: Option<Vec<SyntaxFoldRange>>,
}

pub struct Response {
    pub groups: Vec<Group>,
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
    snapshot: DocumentSnapshot,
    generation: u64,
    providers: HashSet<ProviderIdentity>,
    model: Option<Arc<SyntaxFolds>>,
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
                snapshot: snapshot.clone(),
                generation: 0,
                providers: providers.clone(),
                model: None,
                cancel: None,
                complete: false,
                due: now,
            });
        let identity_changed = entry.snapshot.key != snapshot.key
            || entry.snapshot.metadata.language_id != snapshot.metadata.language_id;
        let revision_changed = entry.snapshot.revision != snapshot.revision;
        if identity_changed || revision_changed || entry.providers != providers {
            entry.cancel();
            entry.snapshot = snapshot.clone();
            entry.providers = providers;
            entry.model = None;
            entry.complete = false;
            entry.due = if revision_changed && !identity_changed {
                now + REFRESH_DEBOUNCE
            } else {
                now
            };
        }
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
    ) -> bool {
        let Some(entry) = self
            .entries
            .get_mut(&(request.project.clone(), current.id))
            .filter(|entry| {
                !request.is_cancelled()
                    && entry.generation == request.generation
                    && entry.snapshot.key == current.key
                    && entry.snapshot.revision == current.revision
                    && entry.snapshot.metadata.language_id == current.metadata.language_id
                    && entry.providers == providers
            })
        else {
            return false;
        };
        entry.cancel.take();
        match result {
            Ok(response)
                if response
                    .groups
                    .iter()
                    .all(|group| providers.contains(&group.provider)) =>
            {
                let ranges = response
                    .groups
                    .into_iter()
                    .filter_map(|group| group.ranges)
                    .collect::<Vec<_>>();
                entry.model =
                    (!ranges.is_empty()).then(|| Arc::new(SyntaxFolds::new(current, ranges)));
                entry.complete = true;
                true
            }
            Ok(_)
            | Err(
                Failure::StaleGeneration
                | Failure::StaleRevision
                | Failure::Restarted
                | Failure::Cancelled,
            ) => {
                entry.complete = false;
                entry.due = Instant::now();
                false
            }
            Err(_) => {
                entry.complete = true;
                false
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
        self.entries.retain(|key, entry| {
            if documents.contains(key) {
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
    ) -> Option<Arc<SyntaxFolds>> {
        self.entries
            .get(&(project.clone(), current.id))?
            .model
            .as_ref()
            .filter(|model| model.describes(current))
            .cloned()
    }
}

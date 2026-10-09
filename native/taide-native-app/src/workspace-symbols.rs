use std::collections::HashSet;
use std::time::{Duration, Instant};

use taide_lsp::native::{Failure, protocol::lsp_types};
use taide_model::ids::ProjectId;
use taide_native_ui::command_palette::{
    WorkspaceSymbol, WorkspaceSymbolIndex, trim_workspace_query,
};
use tokio::sync::watch;

use crate::editor_symbols::ProviderIdentity;

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub struct Request {
    pub project: ProjectId,
    pub query: String,
    pub term: String,
    pub generation: u64,
    pub cancelled: watch::Receiver<bool>,
}

impl Request {
    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }
}

pub struct Group {
    pub provider: ProviderIdentity,
    pub symbols: Vec<WorkspaceSymbol>,
}

#[derive(Default)]
pub struct Response {
    pub groups: Vec<Group>,
}

struct Entry {
    project: ProjectId,
    query: String,
    providers: HashSet<ProviderIdentity>,
    generation: u64,
    due: Instant,
    cancel: Option<watch::Sender<bool>>,
    complete: bool,
    symbols: Vec<WorkspaceSymbol>,
}

impl Drop for Entry {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.send_replace(true);
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    entry: Option<Entry>,
    generation: u64,
}

impl State {
    pub(crate) fn observe(
        &mut self,
        project: Option<&ProjectId>,
        query: Option<&str>,
        providers: HashSet<ProviderIdentity>,
        now: Instant,
    ) -> Option<Request> {
        let Some((project, query)) = project.zip(query) else {
            self.entry = None;
            return None;
        };
        let term = trim_workspace_query(query);
        let changed = self.entry.as_ref().is_none_or(|entry| {
            entry.project != *project || entry.query != query || entry.providers != providers
        });
        if changed {
            self.generation = self.generation.checked_add(1)?;
            self.entry = Some(Entry {
                project: project.clone(),
                query: query.into(),
                providers,
                generation: self.generation,
                due: now + SEARCH_DEBOUNCE,
                cancel: None,
                complete: term.is_empty(),
                symbols: Vec::new(),
            });
        }
        let entry = self.entry.as_mut()?;
        if entry.complete || entry.cancel.is_some() || now < entry.due {
            return None;
        }
        let (cancel, cancelled) = watch::channel(false);
        entry.cancel = Some(cancel);
        Some(Request {
            project: project.clone(),
            query: query.into(),
            term: term.into(),
            generation: entry.generation,
            cancelled,
        })
    }

    pub(crate) fn accept(
        &mut self,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> bool {
        let Some(entry) = self.entry.as_mut().filter(|entry| {
            entry.project == request.project
                && entry.query == request.query
                && entry.generation == request.generation
                && entry.providers == providers
                && !request.is_cancelled()
        }) else {
            return false;
        };
        entry.cancel.take();
        entry.complete = true;
        entry.symbols = result
            .map(|response| {
                response
                    .groups
                    .into_iter()
                    .filter(|group| providers.contains(&group.provider))
                    .flat_map(|group| group.symbols)
                    .collect()
            })
            .unwrap_or_default();
        true
    }

    pub(crate) fn index(&self, project: Option<&ProjectId>) -> WorkspaceSymbolIndex<'_> {
        let Some(entry) = self
            .entry
            .as_ref()
            .filter(|entry| Some(&entry.project) == project)
        else {
            return WorkspaceSymbolIndex::default();
        };
        WorkspaceSymbolIndex {
            entries: entry.complete.then_some(&entry.symbols),
            query: Some(&entry.query),
            generation: entry.generation,
            is_pending: !entry.complete,
        }
    }

    pub(crate) fn selected(
        &self,
        project: &ProjectId,
        generation: u64,
        index: usize,
    ) -> Option<&WorkspaceSymbol> {
        self.entry
            .as_ref()
            .filter(|entry| {
                entry.project == *project && entry.generation == generation && entry.complete
            })
            .and_then(|entry| entry.symbols.get(index))
    }

    pub(crate) fn delay(&self, now: Instant) -> Option<Duration> {
        self.entry
            .as_ref()
            .filter(|entry| !entry.complete && entry.cancel.is_none())
            .map(|entry| entry.due.saturating_duration_since(now))
    }
}

pub(crate) fn normalize(
    response: Option<lsp_types::WorkspaceSymbolResponse>,
) -> Vec<WorkspaceSymbol> {
    match response {
        Some(lsp_types::WorkspaceSymbolResponse::Flat(symbols)) => symbols
            .into_iter()
            .filter_map(|symbol| {
                normalize_location(
                    symbol.name,
                    symbol.kind,
                    symbol.container_name,
                    symbol.location,
                )
            })
            .collect(),
        Some(lsp_types::WorkspaceSymbolResponse::Nested(symbols)) => symbols
            .into_iter()
            .filter_map(|symbol| match symbol.location {
                lsp_types::OneOf::Left(location) => {
                    normalize_location(symbol.name, symbol.kind, symbol.container_name, location)
                }
                lsp_types::OneOf::Right(_) => None,
            })
            .collect(),
        None => Vec::new(),
    }
}

fn normalize_location(
    name: String,
    kind: lsp_types::SymbolKind,
    container_name: Option<String>,
    location: lsp_types::Location,
) -> Option<WorkspaceSymbol> {
    if location.range.start > location.range.end {
        return None;
    }
    let uri = url::Url::parse(location.uri.as_str()).ok()?;
    if uri.scheme() != "file" || uri.query().is_some() || uri.fragment().is_some() {
        return None;
    }
    let path = uri.to_file_path().ok()?;
    let path = path.to_str()?;
    if path.contains('\0') {
        return None;
    }
    Some(WorkspaceSymbol {
        name,
        kind,
        container_name: container_name.unwrap_or_default(),
        path: path.into(),
        line: location.range.start.line.checked_add(1)?,
        column: location.range.start.character.checked_add(1)?,
    })
}

#[cfg(test)]
#[path = "workspace-symbols-tests.rs"]
mod tests;

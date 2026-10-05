use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use taide_model::ids::TabId;

use crate::preview::{Failure, MAX_TEXTURE_BYTES, invalid};
use crate::preview_presentation::{MAX_DECODED_BYTES, MAX_SLIDES, Outline, Request};

struct Entry {
    ready: Option<Arc<Outline>>,
    bytes: usize,
    failed: Option<Failure>,
    loading: Option<u64>,
    has_source: bool,
    needs_read: bool,
    generation: u64,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            ready: None,
            bytes: 0,
            failed: None,
            loading: None,
            has_source: false,
            needs_read: true,
            generation: 0,
        }
    }
}

impl Entry {
    fn invalidate(&mut self) {
        self.loading = None;
        self.has_source = false;
        self.needs_read = true;
    }
}

struct View {
    path: String,
    selected: usize,
    scroll_generation: u64,
}

#[derive(Default)]
pub struct Cache {
    entries: HashMap<String, Entry>,
    views: HashMap<TabId, View>,
    active: Option<Request>,
    token: u64,
}

impl Cache {
    pub(crate) fn ensure(&mut self, tab: &TabId, path: &str) {
        let generation = self.entries.entry(path.into()).or_default().generation;
        if self.views.get(tab).is_none_or(|view| view.path != path) {
            self.views.insert(
                tab.clone(),
                View {
                    path: path.into(),
                    selected: 0,
                    scroll_generation: generation,
                },
            );
        }
    }

    pub fn reconcile(&mut self, paths: &HashMap<TabId, String>) {
        self.views.retain(|tab, view| {
            let Some(path) = paths.get(tab) else {
                return false;
            };
            if path != &view.path {
                view.path = path.clone();
                view.selected = 0;
                view.scroll_generation = 0;
            }
            true
        });
        let keep = paths.values().collect::<HashSet<_>>();
        self.entries.retain(|path, _| keep.contains(path));
    }

    pub fn invalidate(&mut self, path: &str) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.invalidate();
        }
    }

    pub fn invalidate_all(&mut self) {
        for entry in self.entries.values_mut() {
            entry.invalidate();
        }
    }

    pub fn invalidate_root(&mut self, root: &str) {
        for (_, entry) in self
            .entries
            .iter_mut()
            .filter(|(path, _)| Path::new(path).starts_with(root))
        {
            entry.invalidate();
        }
    }

    pub fn begin(&mut self, tab: &TabId, path: &str) -> Option<Request> {
        self.ensure(tab, path);
        if self.active.is_some() {
            return None;
        }
        let entry = self.entries.get_mut(path)?;
        if !entry.needs_read || entry.loading.is_some() {
            return None;
        }
        self.token = self.token.checked_add(1)?;
        let request = Request {
            path: path.into(),
            token: self.token,
        };
        entry.loading = Some(request.token);
        entry.has_source = false;
        entry.needs_read = false;
        self.active = Some(request.clone());
        Some(request)
    }

    pub fn source_ready(&mut self, request: &Request) {
        if self.active.as_ref() != Some(request) {
            return;
        }
        if let Some(entry) = self.entries.get_mut(&request.path)
            && entry.loading == Some(request.token)
        {
            entry.has_source = true;
            if matches!(entry.failed, Some(Failure::Read(_))) {
                entry.failed = None;
            }
        }
    }

    pub fn cancelled(&mut self, request: &Request) {
        if self.active.as_ref() != Some(request) {
            return;
        }
        self.active = None;
        if let Some(entry) = self.entries.get_mut(&request.path)
            && entry.loading == Some(request.token)
        {
            entry.invalidate();
        }
    }

    pub fn reset_pending(&mut self) {
        if let Some(request) = self.active.clone() {
            self.cancelled(&request);
        }
    }

    pub fn retained_bytes(&self) -> usize {
        self.entries.values().map(|entry| entry.bytes).sum()
    }

    pub fn accept(
        &mut self,
        request: Request,
        result: Result<Outline, Failure>,
        other_bytes: usize,
    ) {
        if self.active.as_ref() != Some(&request) {
            return;
        }
        self.active = None;
        let used = self.retained_bytes();
        let Some(entry) = self.entries.get_mut(&request.path) else {
            return;
        };
        if entry.loading != Some(request.token) {
            return;
        }
        entry.loading = None;
        let result = result.and_then(|outline| {
            if outline.slides.is_empty()
                || outline.slides.len() > MAX_SLIDES
                || outline
                    .slides
                    .iter()
                    .enumerate()
                    .any(|(position, slide)| slide.index != position + 1)
            {
                return Err(Failure::Decode(invalid(
                    "PPTX reply has invalid slide metadata",
                )));
            }
            let bytes = outline.retained_bytes();
            if bytes > MAX_DECODED_BYTES
                || used
                    .checked_sub(entry.bytes)
                    .and_then(|used| used.checked_add(other_bytes))
                    .and_then(|used| used.checked_add(bytes))
                    .is_none_or(|bytes| bytes > MAX_TEXTURE_BYTES)
            {
                return Err(Failure::Decode(invalid(
                    "native PPTX preview cache is full",
                )));
            }
            Ok((outline, bytes))
        });
        match result {
            Ok((outline, bytes)) => {
                let was_ready = entry.ready.is_some() && entry.failed.is_none();
                if !was_ready {
                    entry.generation = request.token;
                }
                entry.ready = Some(Arc::new(outline));
                entry.bytes = bytes;
                entry.failed = None;
                entry.has_source = true;
                for view in self
                    .views
                    .values_mut()
                    .filter(|view| view.path == request.path)
                {
                    view.selected = 0;
                    if !was_ready {
                        view.scroll_generation = entry.generation;
                    }
                }
            }
            Err(error) => {
                entry.ready = None;
                entry.bytes = 0;
                entry.failed = Some(error);
            }
        }
    }

    pub fn outline(&self, tab: &TabId) -> Option<&Arc<Outline>> {
        self.entries.get(&self.views.get(tab)?.path)?.ready.as_ref()
    }

    pub fn selected(&self, tab: &TabId) -> Option<usize> {
        let outline = self.outline(tab)?;
        let selected = self.views.get(tab)?.selected;
        Some(selected.min(outline.slides.len().checked_sub(1)?))
    }

    pub fn select(&mut self, tab: &TabId, index: usize) -> bool {
        if self
            .outline(tab)
            .is_none_or(|outline| index >= outline.slides.len())
        {
            return false;
        }
        let Some(view) = self.views.get_mut(tab) else {
            return false;
        };
        if view.selected == index {
            return false;
        }
        view.selected = index;
        true
    }

    pub(crate) fn scroll_generation(&self, tab: &TabId) -> u64 {
        self.views.get(tab).map_or(0, |view| view.scroll_generation)
    }

    pub fn error(&self, tab: &TabId) -> Option<&Failure> {
        self.entries
            .get(&self.views.get(tab)?.path)?
            .failed
            .as_ref()
    }

    pub fn has_source(&self, tab: &TabId) -> bool {
        self.views
            .get(tab)
            .and_then(|view| self.entries.get(&view.path))
            .is_some_and(|entry| entry.has_source)
    }
}

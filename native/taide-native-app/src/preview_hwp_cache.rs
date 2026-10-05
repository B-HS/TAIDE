use std::collections::{HashMap, HashSet};
use std::path::Path;

use eframe::egui::{self, ColorImage, TextureHandle};
use taide_model::ids::TabId;

use crate::preview::{Failure, invalid};
use crate::preview_hwp::{MAX_PAGES, Page, Request, Snapshot};

struct Ready {
    page: usize,
    texture: Option<TextureHandle>,
    bytes: usize,
}

struct View {
    path: String,
    page: usize,
    total_pages: Option<usize>,
    snapshot: Option<Snapshot>,
    ready: Option<Ready>,
    loading: Option<u64>,
    has_source: bool,
    failed: Option<Failure>,
    scroll_generation: u64,
}

impl View {
    fn new(path: String) -> Self {
        Self {
            path,
            page: 0,
            total_pages: None,
            snapshot: None,
            ready: None,
            loading: None,
            has_source: false,
            failed: None,
            scroll_generation: 0,
        }
    }
}

#[derive(Default)]
pub struct Cache {
    views: HashMap<TabId, View>,
    active: Option<Request>,
    token: u64,
}

impl Cache {
    pub(crate) fn ensure(&mut self, tab: &TabId, path: &str) {
        if self.views.get(tab).is_none_or(|view| view.path != path) {
            self.views.insert(tab.clone(), View::new(path.into()));
        }
    }

    pub fn reconcile(&mut self, paths: &HashMap<TabId, String>) {
        self.views.retain(|tab, view| {
            let Some(path) = paths.get(tab) else {
                return false;
            };
            if &view.path != path {
                *view = View::new(path.clone());
            }
            true
        });
    }

    pub fn invalidate(&mut self, path: &str) {
        for view in self.views.values_mut().filter(|view| view.path == path) {
            *view = View::new(view.path.clone());
        }
    }

    pub fn invalidate_root(&mut self, root: &str) {
        for view in self
            .views
            .values_mut()
            .filter(|view| Path::new(&view.path).starts_with(root))
        {
            *view = View::new(view.path.clone());
        }
    }

    pub fn invalidate_all(&mut self) {
        for view in self.views.values_mut() {
            *view = View::new(view.path.clone());
        }
    }

    pub fn selection(&self, tab: &TabId) -> Option<usize> {
        self.views.get(tab).map(|view| view.page)
    }

    pub fn total_pages(&self, tab: &TabId) -> Option<usize> {
        self.views.get(tab).and_then(|view| view.total_pages)
    }

    pub fn has_source(&self, tab: &TabId) -> bool {
        self.views.get(tab).is_some_and(|view| view.has_source)
    }

    pub(crate) fn scroll_generation(&self, tab: &TabId) -> u64 {
        self.views.get(tab).map_or(0, |view| view.scroll_generation)
    }

    pub fn source_ready(&mut self, request: &Request) {
        if self.active.as_ref() != Some(request) {
            return;
        }
        if let Some(view) = self.views.get_mut(&request.tab)
            && view.path == request.path
            && view.loading == Some(request.token)
        {
            view.has_source = true;
        }
    }

    pub fn change(&mut self, tab: &TabId, page: usize) -> bool {
        let Some(view) = self.views.get_mut(tab) else {
            return false;
        };
        if view.total_pages.is_none_or(|total| page >= total) || page == view.page {
            return false;
        }
        view.page = page;
        view.failed = None;
        true
    }

    pub fn begin(&mut self, tab: &TabId, path: &str, max_side: usize) -> Option<Request> {
        self.ensure(tab, path);
        if self.active.is_some() {
            return None;
        }
        let snapshot = self
            .views
            .values()
            .find(|view| view.path == path && view.snapshot.is_some())
            .and_then(|view| view.snapshot.clone());
        let view = self.views.get_mut(tab)?;
        if view.loading.is_some()
            || view.failed.is_some()
            || view
                .ready
                .as_ref()
                .is_some_and(|ready| ready.page == view.page)
        {
            return None;
        }
        self.token = self.token.checked_add(1)?;
        view.has_source |= snapshot.is_some();
        let request = Request {
            tab: tab.clone(),
            path: path.into(),
            token: self.token,
            page: view.page,
            max_side,
            snapshot,
        };
        view.loading = Some(request.token);
        self.active = Some(request.clone());
        Some(request)
    }

    pub fn cancelled(&mut self, request: &Request) {
        if self.active.as_ref() != Some(request) {
            return;
        }
        self.active = None;
        if let Some(view) = self.views.get_mut(&request.tab)
            && view.loading == Some(request.token)
        {
            view.loading = None;
        }
    }

    pub fn reset_pending(&mut self) {
        if let Some(request) = self.active.clone() {
            self.cancelled(&request);
        }
    }

    pub fn retained_bytes(&self) -> usize {
        let mut snapshots = HashSet::new();
        self.views
            .values()
            .map(|view| {
                let pixels = view.ready.as_ref().map_or(0, |ready| ready.bytes);
                let source = view
                    .snapshot
                    .as_ref()
                    .filter(|snapshot| snapshots.insert(snapshot.identity()))
                    .map_or(0, Snapshot::retained_bytes);
                pixels.saturating_add(source)
            })
            .fold(0, usize::saturating_add)
    }

    pub fn accept(
        &mut self,
        context: &egui::Context,
        request: Request,
        result: Result<Page, Failure>,
        other_bytes: usize,
    ) {
        if self.active.as_ref() != Some(&request) {
            return;
        }
        self.active = None;
        let used = self.retained_bytes();
        let Some(view) = self.views.get_mut(&request.tab) else {
            return;
        };
        if view.path != request.path || view.loading != Some(request.token) {
            return;
        }
        view.loading = None;
        if view.page != request.page {
            return;
        }
        let released = view.ready.take().map_or(0, |ready| ready.bytes);
        let old_snapshot = view.snapshot.take();
        let shared = self
            .views
            .values()
            .any(|view| view.snapshot == old_snapshot);
        let source_released = old_snapshot
            .as_ref()
            .filter(|_| !shared)
            .map_or(0, Snapshot::retained_bytes);
        let result = result.and_then(|page| {
            if page.total_pages > MAX_PAGES
                || (page.total_pages == 0 && (request.page != 0 || page.raster.is_some()))
                || (page.total_pages > 0 && request.page >= page.total_pages)
            {
                return Err(Failure::Decode(invalid(
                    "HWP reply has invalid page metadata",
                )));
            }
            let bytes = match page.raster.as_ref() {
                Some(raster) => {
                    if raster.animation.is_some() {
                        return Err(Failure::Decode(invalid(
                            "HWP page cannot contain raster animation",
                        )));
                    }
                    let renderer_side = context.input(|input| input.max_texture_side);
                    let bytes = crate::preview::rgba_bytes(
                        raster.size,
                        renderer_side.min(request.max_side),
                    )
                    .map_err(Failure::Decode)?;
                    if raster.rgba.len() != bytes {
                        return Err(Failure::Decode(invalid(
                            "HWP reply has an invalid pixel length",
                        )));
                    }
                    bytes
                }
                None => 0,
            };
            let source_bytes = page
                .snapshot
                .as_ref()
                .filter(|snapshot| {
                    !self
                        .views
                        .values()
                        .any(|view| view.snapshot.as_ref() == Some(snapshot))
                })
                .map_or(0, Snapshot::retained_bytes);
            if used
                .checked_sub(released + source_released)
                .and_then(|used| used.checked_add(other_bytes))
                .and_then(|used| used.checked_add(bytes))
                .and_then(|used| used.checked_add(source_bytes))
                .is_none_or(|total| total > crate::preview::MAX_TEXTURE_BYTES)
            {
                if let Some(snapshot) = &page.snapshot {
                    snapshot.discard();
                }
                return Err(Failure::Decode(invalid("native HWP preview cache is full")));
            }
            let texture = page.raster.map(|raster| {
                context.load_texture(
                    format!("native-hwp-{}", request.token),
                    ColorImage::from_rgba_unmultiplied(raster.size, &raster.rgba),
                    egui::TextureOptions::LINEAR,
                )
            });
            Ok((
                Ready {
                    page: request.page,
                    texture,
                    bytes,
                },
                page.total_pages,
                page.snapshot,
            ))
        });
        let view = self
            .views
            .get_mut(&request.tab)
            .expect("live HWP view remains after validation");
        match result {
            Ok((ready, pages, snapshot)) => {
                if view.total_pages.is_none() {
                    view.scroll_generation = request.token;
                }
                view.ready = Some(ready);
                view.total_pages = Some(pages);
                view.snapshot = snapshot;
                view.failed = None;
            }
            Err(error) => view.failed = Some(error),
        }
    }

    pub fn texture(&self, tab: &TabId) -> Option<&TextureHandle> {
        self.views
            .get(tab)
            .and_then(|view| view.ready.as_ref())
            .and_then(|ready| ready.texture.as_ref())
    }

    pub fn error(&self, tab: &TabId) -> Option<&Failure> {
        self.views.get(tab).and_then(|view| view.failed.as_ref())
    }
}

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{self, ColorImage, TextureHandle};

use taide_model::error::AppError;
use taide_model::error::AppResult;
use taide_model::ids::TabId;
use taide_runtime::AppServices;

pub use crate::preview::Failure;
use crate::preview::{Raster, invalid};

pub const MIN_ZOOM: u8 = 2;
pub const MAX_ZOOM: u8 = 12;
pub const INITIAL_ZOOM: u8 = 4;
pub const ZOOM_DIVISOR: u8 = 4;

struct Source {
    canonical: PathBuf,
    bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct Snapshot(Arc<Source>);

impl fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PdfSnapshot")
            .field("byte_len", &self.0.bytes.len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Snapshot {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub tab: TabId,
    pub path: String,
    pub token: u64,
    pub page: usize,
    pub zoom: u8,
    pub max_side: usize,
    pub snapshot: Option<Snapshot>,
}

#[derive(Debug)]
pub struct Page {
    pub total_pages: usize,
    pub raster: Raster,
    snapshot: Option<Snapshot>,
}

impl Page {
    pub fn new(total_pages: usize, raster: Raster) -> Self {
        Self {
            total_pages,
            raster,
            snapshot: None,
        }
    }
}

pub fn decode(bytes: &[u8], page: usize, zoom: u8, max_side: usize) -> AppResult<Page> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded PDF exceeds the file preview budget"));
    }
    if page == 0 || !(MIN_ZOOM..=MAX_ZOOM).contains(&zoom) {
        return Err(invalid("invalid PDF page or zoom"));
    }
    #[cfg(target_os = "macos")]
    return crate::preview_pdf_macos::decode(bytes, page, zoom, max_side);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = max_side;
        Err(invalid(
            "native PDF decoding is not connected on this platform",
        ))
    }
}

pub async fn read(
    services: &AppServices,
    request: &Request,
    on_source_ready: impl FnOnce() + Send + 'static,
) -> Result<Page, Failure> {
    let request = request.clone();
    let result = crate::preview::read_approved(
        services,
        request.path,
        "native-pdf-preview",
        move |source| {
            let snapshot = match request.snapshot {
                Some(snapshot) if snapshot.0.canonical == source => snapshot,
                Some(_) => {
                    return Err(AppError::Forbidden("PDF source identity changed".into()));
                }
                None => Snapshot(Arc::new(Source {
                    canonical: source.to_owned(),
                    bytes: taide_file::service::read_raw(source)?,
                })),
            };
            on_source_ready();
            let result = decode(
                &snapshot.0.bytes,
                request.page,
                request.zoom,
                request.max_side,
            )
            .map(|mut page| {
                page.snapshot = Some(snapshot);
                page
            });
            Ok(result)
        },
    )
    .await;
    match result {
        Ok(result) => result.map_err(Failure::Decode),
        Err(error) => Err(Failure::Read(error)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub page: usize,
    pub zoom: u8,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            page: 1,
            zoom: INITIAL_ZOOM,
        }
    }
}

struct Ready {
    selection: Selection,
    texture: TextureHandle,
    bytes: usize,
}

struct View {
    path: String,
    selection: Selection,
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
            selection: Selection::default(),
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

    pub fn selection(&self, tab: &TabId) -> Option<Selection> {
        self.views.get(tab).map(|view| view.selection)
    }

    pub fn total_pages(&self, tab: &TabId) -> Option<usize> {
        self.views.get(tab).and_then(|view| view.total_pages)
    }

    pub fn has_source(&self, tab: &TabId) -> bool {
        self.views.get(tab).is_some_and(|view| view.has_source)
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

    pub(crate) fn scroll_generation(&self, tab: &TabId) -> u64 {
        self.views.get(tab).map_or(0, |view| view.scroll_generation)
    }

    pub fn change(&mut self, tab: &TabId, selection: Selection) -> bool {
        let Some(view) = self.views.get_mut(tab) else {
            return false;
        };
        if view
            .total_pages
            .is_none_or(|total| selection.page == 0 || selection.page > total)
            || !(MIN_ZOOM..=MAX_ZOOM).contains(&selection.zoom)
            || view.selection == selection
        {
            return false;
        }
        view.selection = selection;
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
                .is_some_and(|ready| ready.selection == view.selection)
        {
            return None;
        }
        self.token = self.token.checked_add(1)?;
        view.has_source |= snapshot.is_some();
        let request = Request {
            tab: tab.clone(),
            path: path.into(),
            token: self.token,
            page: view.selection.page,
            zoom: view.selection.zoom,
            max_side,
            snapshot,
        };
        view.loading = Some(request.token);
        self.active = Some(request.clone());
        Some(request)
    }

    pub fn cancelled(&mut self, request: &Request) {
        if self.active.as_ref() == Some(request) {
            self.active = None;
            if let Some(view) = self.views.get_mut(&request.tab)
                && view.loading == Some(request.token)
            {
                view.loading = None;
            }
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
                    .filter(|snapshot| snapshots.insert(Arc::as_ptr(&snapshot.0)))
                    .map_or(0, |snapshot| snapshot.0.bytes.len());
                pixels + source
            })
            .sum()
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
        let selection = Selection {
            page: request.page,
            zoom: request.zoom,
        };
        if selection != view.selection {
            return;
        }
        let released = view.ready.take().map_or(0, |ready| ready.bytes);
        let old_snapshot = view.snapshot.clone();
        let shared = self
            .views
            .iter()
            .any(|(tab, view)| tab != &request.tab && view.snapshot == old_snapshot);
        let source_released = old_snapshot
            .as_ref()
            .filter(|_| !shared)
            .map_or(0, |snapshot| snapshot.0.bytes.len());
        let view = self
            .views
            .get_mut(&request.tab)
            .expect("live PDF view remains after accounting");
        view.snapshot = None;
        let result = result.and_then(|page| {
            if page.total_pages == 0
                || request.page > page.total_pages
                || page.raster.animation.is_some()
            {
                return Err(Failure::Decode(invalid(
                    "PDF reply has invalid page metadata",
                )));
            }
            let renderer_side = context.input(|input| input.max_texture_side);
            let bytes =
                crate::preview::rgba_bytes(page.raster.size, renderer_side.min(request.max_side))
                    .map_err(Failure::Decode)?;
            if page.raster.rgba.len() != bytes {
                return Err(Failure::Decode(invalid(
                    "PDF reply has an invalid pixel length",
                )));
            }
            let source_bytes = page
                .snapshot
                .as_ref()
                .filter(|snapshot| {
                    !self
                        .views
                        .values()
                        .any(|view| view.snapshot.as_ref() == Some(snapshot))
                })
                .map_or(0, |snapshot| snapshot.0.bytes.len());
            if used
                .checked_sub(released + source_released)
                .and_then(|used| used.checked_add(other_bytes))
                .and_then(|used| used.checked_add(bytes))
                .and_then(|used| used.checked_add(source_bytes))
                .is_none_or(|total| total > crate::preview::MAX_TEXTURE_BYTES)
            {
                return Err(Failure::Decode(invalid("native PDF preview cache is full")));
            }
            let texture = context.load_texture(
                format!("native-pdf-{}", request.token),
                ColorImage::from_rgba_unmultiplied(page.raster.size, &page.raster.rgba),
                egui::TextureOptions::LINEAR,
            );
            Ok((
                Ready {
                    selection,
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
            .expect("live PDF view remains after validation");
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
            Err(error) => {
                view.failed = Some(error);
            }
        }
    }

    pub fn texture(&self, tab: &TabId) -> Option<&TextureHandle> {
        self.views
            .get(tab)
            .and_then(|view| view.ready.as_ref())
            .map(|ready| &ready.texture)
    }

    pub fn error(&self, tab: &TabId) -> Option<&Failure> {
        self.views.get(tab).and_then(|view| view.failed.as_ref())
    }
}

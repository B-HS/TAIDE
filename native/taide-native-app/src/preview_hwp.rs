use std::{
    fmt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use rhwp::DocumentCore;
use taide_model::{
    error::{AppError, AppResult},
    ids::TabId,
};
use taide_native_retained::{Limits, RetainedBytes, measure};
use taide_runtime::AppServices;

use crate::preview::{Raster, invalid};

pub use crate::preview::Failure;
pub use crate::preview_hwp_cache::Cache;

pub const MAX_PAGES: usize = 4096;
pub const MAX_SVG_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RETAINED_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RETAINED_VISITS: usize = 262_144;

#[derive(RetainedBytes)]
pub struct Document {
    core: DocumentCore,
    pages: usize,
}

impl fmt::Debug for Document {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HwpDocument")
            .field("pages", &self.pages)
            .finish_non_exhaustive()
    }
}

impl Document {
    pub fn load(bytes: &[u8]) -> AppResult<Self> {
        let core = crate::preview_hwp_preflight::load_core(bytes)?;
        let pages = core.page_count() as usize;
        if pages > MAX_PAGES {
            return Err(invalid("HWP page count exceeds the preview budget"));
        }
        let document = Self { core, pages };
        document.retained_bytes()?;
        Ok(document)
    }

    pub fn page_count(&self) -> usize {
        self.pages
    }

    fn retained_bytes(&self) -> AppResult<usize> {
        measure(self, retained_limits())
            .map(|report| report.bytes)
            .map_err(|error| invalid(error.to_string()))
    }

    pub fn render(&self, page: usize, max_side: usize) -> AppResult<Option<Raster>> {
        if page >= self.pages {
            return Err(invalid("HWP page selection is outside the document"));
        }
        let Ok(svg) = self.core.render_page_svg_legacy_native(page as u32) else {
            return Ok(None);
        };
        if svg.len() > MAX_SVG_BYTES {
            return Ok(None);
        }
        Ok(crate::preview_svg::decode(svg.as_bytes(), max_side).ok())
    }
}

fn retained_limits() -> Limits {
    Limits {
        bytes: MAX_RETAINED_BYTES,
        visits: MAX_RETAINED_VISITS,
    }
}

#[derive(RetainedBytes)]
struct Source {
    canonical: PathBuf,
    document: Mutex<Option<Document>>,
    retained: AtomicUsize,
    base: usize,
}

impl Source {
    fn load(canonical: PathBuf, bytes: &[u8]) -> AppResult<Arc<Self>> {
        let document = Document::load(bytes)?;
        let mut source = Arc::new(Self {
            canonical,
            document: Mutex::new(None),
            retained: AtomicUsize::new(0),
            base: 0,
        });
        let base = measure(&source, retained_limits())
            .map_err(|error| invalid(error.to_string()))?
            .bytes;
        let cost = Self::cost(base, &document)?;
        let inner = Arc::get_mut(&mut source)
            .ok_or_else(|| invalid("HWP source is shared before initialization"))?;
        inner.base = base;
        inner.retained.store(cost, Ordering::Release);
        *inner
            .document
            .get_mut()
            .map_err(|_| invalid("HWP source initialization is poisoned"))? = Some(document);
        Ok(source)
    }

    fn cost(base: usize, document: &Document) -> AppResult<usize> {
        document
            .retained_bytes()?
            .checked_sub(size_of::<Document>())
            .and_then(|children| base.checked_add(children))
            .filter(|cost| *cost <= MAX_RETAINED_BYTES)
            .ok_or_else(|| invalid("HWP retained source exceeds the preview budget"))
    }

    fn discard(&self) {
        let mut guard = match self.document.lock() {
            Ok(guard) => guard,
            Err(error) => error.into_inner(),
        };
        *guard = None;
        self.retained.store(self.base, Ordering::Release);
    }

    fn render(&self, page: usize, max_side: usize) -> AppResult<(usize, Option<Raster>)> {
        let mut guard = match self.document.lock() {
            Ok(guard) => guard,
            Err(error) => {
                *error.into_inner() = None;
                self.retained.store(self.base, Ordering::Release);
                return Err(invalid("HWP source is poisoned"));
            }
        };
        let document = guard
            .as_ref()
            .ok_or_else(|| invalid("HWP source was discarded after a budget failure"))?;
        let total_pages = document.page_count();
        let result = if total_pages == 0 && page == 0 {
            Ok(None)
        } else {
            document.render(page, max_side)
        };
        match Self::cost(self.base, document) {
            Ok(cost) => self.retained.store(cost, Ordering::Release),
            Err(error) => {
                *guard = None;
                self.retained.store(self.base, Ordering::Release);
                return Err(error);
            }
        }
        result.map(|raster| (total_pages, raster))
    }
}

#[derive(Clone)]
pub struct Snapshot(Arc<Source>);

impl fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HwpSnapshot")
            .field("retained_bytes", &self.retained_bytes())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Snapshot {}

impl Snapshot {
    pub(crate) fn identity(&self) -> *const () {
        Arc::as_ptr(&self.0).cast()
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.0.retained.load(Ordering::Acquire)
    }

    pub(crate) fn discard(&self) {
        self.0.discard();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub tab: TabId,
    pub path: String,
    pub token: u64,
    pub page: usize,
    pub max_side: usize,
    pub snapshot: Option<Snapshot>,
}

#[derive(Debug)]
pub struct Page {
    pub total_pages: usize,
    pub raster: Option<Raster>,
    pub(crate) snapshot: Option<Snapshot>,
}

impl Page {
    pub fn new(total_pages: usize, raster: Option<Raster>) -> Self {
        Self {
            total_pages,
            raster,
            snapshot: None,
        }
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
        "native-hwp-preview",
        move |source| {
            let snapshot = match request.snapshot {
                Some(snapshot) if snapshot.0.canonical == source => {
                    on_source_ready();
                    Ok(snapshot)
                }
                Some(_) => return Err(AppError::Forbidden("HWP source identity changed".into())),
                None => {
                    let bytes = taide_file::service::read_raw(source)?;
                    on_source_ready();
                    Source::load(source.to_owned(), &bytes).map(Snapshot)
                }
            };
            let result = snapshot.and_then(|snapshot| {
                let (total_pages, raster) = snapshot.0.render(request.page, request.max_side)?;
                Ok(Page {
                    total_pages,
                    raster,
                    snapshot: Some(snapshot),
                })
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

#[cfg(test)]
mod tests {
    use eframe::egui::Context;
    use rhwp::model::bin_data::{BinDataBytes, BinDataContent};

    use super::*;

    const BLANK: &[u8] = include_bytes!("../vendor/rhwp/saved/blank2010.hwp");
    const MAX_SIDE: usize = 4096;

    #[test]
    fn hwp_persistent_core는_실제_비용_재사용_budget_폐기와_마지막_owner를_검사한다() {
        let source = Source::load(PathBuf::from("synthetic.hwp"), BLANK).unwrap();
        assert_eq!(
            source.retained.load(Ordering::Acquire),
            measure(&source, retained_limits()).unwrap().bytes
        );
        let before = source.retained.load(Ordering::Acquire);
        let (pages, raster) = source.render(0, MAX_SIDE).unwrap();
        assert_eq!(pages, 1);
        assert!(raster.is_some());
        let after = source.retained.load(Ordering::Acquire);
        assert!(after >= before);
        assert_eq!(after, measure(&source, retained_limits()).unwrap().bytes);
        source.render(0, MAX_SIDE).unwrap();
        assert_eq!(source.retained.load(Ordering::Acquire), after);
        let snapshot = Snapshot(source.clone());
        let weak = Arc::downgrade(&source);
        drop(source);
        let context = Context::default();
        let mut cache = Cache::default();
        let first = TabId::new();
        let second = TabId::new();
        let request = cache.begin(&first, "synthetic.hwp", MAX_SIDE).unwrap();
        cache.accept(
            &context,
            request,
            Ok(Page {
                total_pages: pages,
                raster,
                snapshot: Some(snapshot.clone()),
            }),
            0,
        );
        let request = cache.begin(&second, "synthetic.hwp", MAX_SIDE).unwrap();
        assert_eq!(request.snapshot, Some(snapshot.clone()));
        let source_cost = snapshot.retained_bytes();
        cache.accept(
            &context,
            request,
            Ok(Page {
                total_pages: pages,
                raster: None,
                snapshot: Some(snapshot.clone()),
            }),
            0,
        );
        assert_eq!(snapshot.retained_bytes(), source_cost);
        cache.reconcile(&std::collections::HashMap::from([(
            second.clone(),
            "synthetic.hwp".to_owned(),
        )]));
        assert!(weak.upgrade().is_some());
        cache.reconcile(&std::collections::HashMap::new());
        assert_eq!(cache.retained_bytes(), 0);
        assert!(weak.upgrade().is_some());
        drop(snapshot);
        assert!(weak.upgrade().is_none());

        let source = Source::load(PathBuf::from("synthetic-budget.hwp"), BLANK).unwrap();
        source
            .document
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .core
            .document_mut()
            .bin_data_content
            .push(BinDataContent {
                id: 1,
                data: BinDataBytes::Loaded(Vec::with_capacity(MAX_RETAINED_BYTES)),
                extension: "dat".to_owned(),
            });
        assert!(source.render(0, MAX_SIDE).is_err());
        assert!(source.document.lock().unwrap().is_none());
        assert_eq!(
            source.retained.load(Ordering::Acquire),
            measure(&source, retained_limits()).unwrap().bytes
        );
        assert!(source.render(0, MAX_SIDE).is_err());

        let source = Source::load(PathBuf::from("synthetic-global-budget.hwp"), BLANK).unwrap();
        let snapshot = Snapshot(source.clone());
        let mut cache = Cache::default();
        let request = cache
            .begin(&first, "synthetic-global-budget.hwp", MAX_SIDE)
            .unwrap();
        cache.accept(
            &context,
            request,
            Ok(Page {
                total_pages: 1,
                raster: None,
                snapshot: Some(snapshot.clone()),
            }),
            0,
        );
        let request = cache
            .begin(&second, "synthetic-global-budget.hwp", MAX_SIDE)
            .unwrap();
        cache.accept(
            &context,
            request,
            Ok(Page {
                total_pages: 1,
                raster: None,
                snapshot: Some(snapshot),
            }),
            crate::preview::MAX_TEXTURE_BYTES,
        );
        assert!(cache.error(&second).is_some());
        assert!(source.document.lock().unwrap().is_none());
        assert_eq!(cache.retained_bytes(), source.base);

        let mut path = PathBuf::from("synthetic-overhead.hwp");
        path.reserve(MAX_RETAINED_BYTES);
        assert!(Source::load(path, BLANK).is_err());
    }
}

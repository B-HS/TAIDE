use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use taide_model::ids::TabId;

use crate::preview::{Failure, MAX_TEXTURE_BYTES, invalid};
use crate::preview_spreadsheet::{MAX_DECODED_BYTES, MAX_SHEETS, Request, Workbook};

struct Entry {
    ready: Option<Arc<Workbook>>,
    bytes: usize,
    columns: Vec<Option<Arc<Vec<f32>>>>,
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
            columns: Vec::new(),
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
        result: Result<Workbook, Failure>,
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
        let result = result.and_then(|workbook| {
            if workbook.sheets.len() > MAX_SHEETS
                || workbook.sheets.iter().any(|sheet| {
                    sheet.total_row_count > crate::preview_spreadsheet::MAX_EXCEL_ROWS as usize
                        || sheet.rows.first().is_some_and(|row| {
                            row.len() > crate::preview_spreadsheet::MAX_EXCEL_COLUMNS as usize
                        })
                        || sheet.rows.len()
                            != sheet
                                .total_row_count
                                .min(crate::preview_spreadsheet::MAX_PREVIEW_ROWS)
                        || sheet.truncated
                            != (sheet.total_row_count
                                > crate::preview_spreadsheet::MAX_PREVIEW_ROWS)
                        || sheet.rows.first().is_some_and(|first| {
                            sheet.rows.iter().any(|row| row.len() != first.len())
                        })
                })
            {
                return Err(Failure::Decode(invalid(
                    "spreadsheet reply has invalid sheet metadata",
                )));
            }
            let layout_bytes = workbook.sheets.iter().fold(
                workbook
                    .sheets
                    .len()
                    .saturating_mul(size_of::<Option<Arc<Vec<f32>>>>()),
                |bytes, sheet| {
                    bytes.saturating_add(sheet.rows.first().map_or(0, |row| {
                        row.len()
                            .saturating_mul(size_of::<f32>())
                            .saturating_add(size_of::<Vec<f32>>() + size_of::<usize>() * 2)
                    }))
                },
            );
            let bytes = workbook.retained_bytes().saturating_add(layout_bytes);
            if bytes > MAX_DECODED_BYTES
                || used
                    .checked_sub(entry.bytes)
                    .and_then(|used| used.checked_add(other_bytes))
                    .and_then(|used| used.checked_add(bytes))
                    .is_none_or(|bytes| bytes > MAX_TEXTURE_BYTES)
            {
                return Err(Failure::Decode(invalid(
                    "native spreadsheet preview cache is full",
                )));
            }
            Ok((workbook, bytes))
        });
        match result {
            Ok((workbook, bytes)) => {
                let was_ready = entry.ready.is_some() && entry.failed.is_none();
                if !was_ready {
                    entry.generation = request.token;
                }
                entry.columns = vec![None; workbook.sheets.len()];
                entry.ready = Some(Arc::new(workbook));
                entry.bytes = bytes;
                entry.failed = None;
                entry.has_source = true;
                for view in self
                    .views
                    .values_mut()
                    .filter(|view| view.path == request.path)
                {
                    if !was_ready {
                        view.scroll_generation = entry.generation;
                    }
                }
            }
            Err(error) => {
                entry.ready = None;
                entry.columns = Vec::new();
                entry.bytes = 0;
                entry.failed = Some(error);
            }
        }
    }

    pub fn workbook(&self, tab: &TabId) -> Option<&Arc<Workbook>> {
        self.entries.get(&self.views.get(tab)?.path)?.ready.as_ref()
    }

    pub(crate) fn column_widths(&self, tab: &TabId, selected: usize) -> Option<Arc<Vec<f32>>> {
        self.entries
            .get(&self.views.get(tab)?.path)?
            .columns
            .get(selected)?
            .clone()
    }

    pub(crate) fn set_column_widths(
        &mut self,
        tab: &TabId,
        selected: usize,
        widths: Arc<Vec<f32>>,
    ) {
        let Some(view) = self.views.get(tab) else {
            return;
        };
        let Some(entry) = self.entries.get_mut(&view.path) else {
            return;
        };
        let Some(slot) = entry.columns.get_mut(selected) else {
            return;
        };
        *slot = Some(widths);
    }

    pub fn selected(&self, tab: &TabId) -> Option<usize> {
        let workbook = self.workbook(tab)?;
        let selected = self.views.get(tab)?.selected;
        Some(selected.min(workbook.sheets.len().checked_sub(1)?))
    }

    pub fn select(&mut self, tab: &TabId, index: usize) -> bool {
        if self
            .workbook(tab)
            .is_none_or(|workbook| index >= workbook.sheets.len())
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

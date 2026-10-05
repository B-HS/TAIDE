use std::collections::VecDeque;

use taide_model::error::{AppError, AppResult};
use taide_native_terminal::{Point, TerminalCore};

use crate::terminal_links::{
    CellRange, ExternalLink, FileLink, Row, external_at, file_links, read_row,
};

pub const CACHE_ROWS: usize = 256;
pub const MAX_CANDIDATES: usize = taide_terminal::service::MAX_LINK_CANDIDATES_PER_ROW;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    cwd: String,
    row: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    id: u64,
    key: Key,
    candidates: Vec<String>,
}

impl Request {
    pub fn cwd(&self) -> &str {
        &self.key.cwd
    }

    pub fn candidates(&self) -> &[String] {
        &self.candidates
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFileLink {
    pub matched: FileLink,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Link {
    External(ExternalLink),
    File(ResolvedFileLink),
}

impl Link {
    pub fn range(&self) -> CellRange {
        match self {
            Self::External(link) => link.range,
            Self::File(link) => link.matched.range,
        }
    }
}

struct Entry {
    key: Key,
    paths: Vec<Option<String>>,
}

#[derive(Default)]
pub struct Cache {
    entries: VecDeque<Entry>,
    pending: Option<Request>,
    sequence: u64,
}

impl Cache {
    pub fn request(&mut self, cwd: &str, row: &Row) -> AppResult<Option<Request>> {
        if cwd.is_empty() || self.pending.is_some() || self.cached(cwd, row).is_some() {
            return Ok(None);
        }
        let candidates = file_links(row)?
            .into_iter()
            .map(|link| link.path)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(None);
        }
        self.sequence = self.sequence.checked_add(1).ok_or_else(|| {
            AppError::Internal("native terminal file link sequence exhausted".into())
        })?;
        let request = Request {
            id: self.sequence,
            key: Key {
                cwd: cwd.into(),
                row: row.text.clone(),
            },
            candidates,
        };
        self.pending = Some(request.clone());
        Ok(Some(request))
    }

    pub fn complete(&mut self, request: &Request, result: AppResult<Vec<Option<String>>>) -> bool {
        if self.pending.as_ref() != Some(request) {
            return false;
        }
        self.pending = None;
        let Ok(paths) = result else { return true };
        if paths.len() != request.candidates.len() {
            return true;
        }
        self.entries.push_back(Entry {
            key: request.key.clone(),
            paths,
        });
        if self.entries.len() > CACHE_ROWS {
            self.entries.pop_front();
        }
        true
    }

    pub fn cancel(&mut self, request: &Request) {
        if self.pending.as_ref() == Some(request) {
            self.pending = None;
        }
    }

    fn cached(&self, cwd: &str, row: &Row) -> Option<&[Option<String>]> {
        self.entries
            .iter()
            .find(|entry| entry.key.cwd == cwd && entry.key.row == row.text)
            .map(|entry| entry.paths.as_slice())
    }

    pub fn at(&self, core: &TerminalCore, cwd: &str, point: Point) -> AppResult<Option<Link>> {
        if let Some(link) = external_at(core, point)? {
            return Ok(Some(Link::External(link)));
        }
        let row = read_row(core, point.line)?;
        let Some(paths) = self.cached(cwd, &row) else {
            return Ok(None);
        };
        Ok(file_links(&row)?
            .into_iter()
            .zip(paths)
            .find_map(|(matched, path)| {
                if !matched.range.contains(point) {
                    return None;
                }
                path.as_ref().filter(|path| !path.is_empty()).map(|path| {
                    Link::File(ResolvedFileLink {
                        matched,
                        path: path.clone(),
                    })
                })
            }))
    }
}

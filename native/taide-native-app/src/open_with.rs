use std::collections::{HashMap, HashSet, VecDeque};

use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{ProjectLayout, TabKind};

pub const MAX_OVERRIDES: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewKind {
    Image,
    Video,
    Audio,
    Pdf,
    Html,
    Spreadsheet,
    Presentation,
    Hwp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Editor,
    Preview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Editor,
    Preview(PreviewKind),
}

pub fn preview_kind(file_name: &str) -> Option<PreviewKind> {
    let dot = file_name.rfind('.')?;
    if dot == 0 {
        return None;
    }
    match file_name[dot + 1..].to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "avif" => {
            Some(PreviewKind::Image)
        }
        "mp4" | "webm" | "mov" | "m4v" => Some(PreviewKind::Video),
        "mp3" | "wav" | "flac" | "m4a" | "ogg" => Some(PreviewKind::Audio),
        "pdf" => Some(PreviewKind::Pdf),
        "html" | "htm" => Some(PreviewKind::Html),
        "xlsx" | "xls" | "csv" => Some(PreviewKind::Spreadsheet),
        "pptx" => Some(PreviewKind::Presentation),
        "hwp" | "hwpx" => Some(PreviewKind::Hwp),
        _ => None,
    }
}

#[derive(Default)]
pub struct Registry {
    paths: VecDeque<String>,
    observed: HashMap<ProjectId, (u32, HashMap<TabId, String>)>,
}

impl Registry {
    pub fn set(&mut self, path: String, mode: Mode) {
        self.paths.retain(|existing| existing != &path);
        if mode == Mode::Preview {
            return;
        }
        self.paths.push_back(path);
        if self.paths.len() > MAX_OVERRIDES {
            self.paths.pop_front();
        }
    }

    pub fn surface(&self, path: &str) -> Surface {
        if self.paths.iter().any(|existing| existing == path) {
            return Surface::Editor;
        }
        preview_kind(path.rsplit('/').next().unwrap_or_default())
            .map_or(Surface::Editor, Surface::Preview)
    }

    pub fn prune(&mut self, keep: &HashSet<String>) {
        self.paths.retain(|path| keep.contains(path));
    }

    pub fn close_path(&mut self, path: &str, layout: &ProjectLayout) {
        let remains = taide_layout::service::all_roots(layout)
            .flat_map(crate::tabs::tabs_in)
            .any(|tab| matches!(&tab.kind, TabKind::File { path: file } if file == path));
        if !remains {
            self.set(path.into(), Mode::Preview);
        }
    }

    pub fn reconcile(&mut self, layouts: &HashMap<ProjectId, ProjectLayout>) {
        let project_closed = self
            .observed
            .keys()
            .any(|project| !layouts.contains_key(project));
        self.observed
            .retain(|project, _| layouts.contains_key(project));
        for (project, next) in layouts {
            if self
                .observed
                .get(project)
                .is_some_and(|(revision, _)| *revision >= next.revision)
            {
                continue;
            }
            let tabs = taide_layout::service::all_roots(next)
                .flat_map(crate::tabs::tabs_in)
                .filter_map(|tab| match &tab.kind {
                    TabKind::File { path } => Some((tab.id.clone(), path.clone())),
                    _ => None,
                })
                .collect::<HashMap<_, _>>();
            if let Some((_, previous)) = self.observed.get(project) {
                let moves = tabs
                    .iter()
                    .filter_map(|(id, to)| {
                        let from = previous.get(id)?;
                        (from != to && self.paths.contains(from))
                            .then(|| (from.clone(), to.clone()))
                    })
                    .collect::<Vec<_>>();
                for (from, to) in moves {
                    self.set(from, Mode::Preview);
                    self.set(to, Mode::Editor);
                }
            }
            if let Some((_, previous)) = self.observed.get(project) {
                let closed = previous
                    .iter()
                    .filter(|(id, path)| {
                        !tabs.contains_key(*id) && !tabs.values().any(|current| current == *path)
                    })
                    .map(|(_, path)| path.clone())
                    .collect::<HashSet<_>>();
                for path in closed {
                    self.set(path, Mode::Preview);
                }
            }
            self.observed.insert(project.clone(), (next.revision, tabs));
        }
        if project_closed {
            let keep = self
                .observed
                .values()
                .flat_map(|(_, tabs)| tabs.values().cloned())
                .collect();
            self.prune(&keep);
        }
    }
}

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui::ViewportId;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, ProjectLayout, TabKind};

use crate::terminal_tabs::OpenedFileLink;

pub const REVEAL_TTL: Duration = Duration::from_secs(5);

pub struct Position {
    pub line: f64,
    pub column: f64,
}

struct Pending {
    project: ProjectId,
    viewport: ViewportId,
    path: String,
    position: Position,
    deadline: Instant,
}

#[derive(Default)]
pub struct Reveals {
    pending: HashMap<TabId, Pending>,
}

impl Reveals {
    pub fn queue(
        &mut self,
        opened: &OpenedFileLink,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        now: Instant,
    ) -> bool {
        self.reconcile(layouts, now);
        if !is_live(layouts, &opened.project, &opened.tab, &opened.path) {
            return false;
        }
        self.pending.insert(
            opened.tab.clone(),
            Pending {
                project: opened.project.clone(),
                viewport: opened.viewport,
                path: opened.path.clone(),
                position: Position {
                    line: opened.line,
                    column: opened.column,
                },
                deadline: now + REVEAL_TTL,
            },
        );
        true
    }

    pub fn consume(
        &mut self,
        tab: &TabId,
        path: &str,
        viewport: ViewportId,
        now: Instant,
    ) -> Option<Position> {
        let pending = self.pending.get(tab)?;
        if pending.deadline <= now {
            self.pending.remove(tab);
            return None;
        }
        if pending.path != path || pending.viewport != viewport {
            return None;
        }
        self.pending.remove(tab).map(|pending| pending.position)
    }

    pub fn reconcile(&mut self, layouts: &HashMap<ProjectId, ProjectLayout>, now: Instant) {
        self.pending.retain(|tab, pending| {
            pending.deadline > now && is_live(layouts, &pending.project, tab, &pending.path)
        });
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

fn is_live(
    layouts: &HashMap<ProjectId, ProjectLayout>,
    project: &ProjectId,
    tab: &TabId,
    path: &str,
) -> bool {
    let Some(layout) = layouts.get(project) else {
        return false;
    };
    std::iter::once(&layout.root)
        .chain(layout.auxiliary_windows.iter().map(|window| &window.root))
        .any(|root| contains(root, tab, path))
}

fn contains(root: &PaneNode, tab: &TabId, path: &str) -> bool {
    match root {
        PaneNode::Leaf { tabs, .. } => tabs.iter().any(|candidate| &candidate.id == tab && matches!(&candidate.kind, TabKind::File { path: candidate_path } if candidate_path == path)),
        PaneNode::Split { children, .. } => children.iter().any(|child| contains(child, tab, path)),
    }
}

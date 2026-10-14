use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui::ViewportId;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, ProjectLayout, TabKind};

use crate::terminal_tabs::OpenedFileLink;

pub(crate) fn reveal_document(
    store: &mut taide_native_editor::store::EditorStore,
    key: &taide_native_editor::view::ViewKey,
    requested: &taide_native_editor::document::DocumentSnapshot,
    selection: std::ops::Range<usize>,
) -> Result<Option<taide_native_editor::view::ViewId>, taide_native_editor::document::EditorError> {
    let Some(view) = store
        .views()
        .find(key)
        .and_then(|id| store.views().get(id))
        .cloned()
    else {
        return Ok(None);
    };
    if view.composition.is_some() {
        return Ok(None);
    }
    let current = store.documents().snapshot(view.document)?;
    if current.id != requested.id
        || current.key != requested.key
        || current.revision != requested.revision
        || current.metadata.language_id != requested.metadata.language_id
    {
        return Ok(None);
    }
    if selection.start > selection.end {
        return Err(taide_native_editor::document::EditorError::InvalidBoundary);
    }
    taide_native_editor::lsp::byte_to_position(&current, selection.end)?;
    store.set_view_state(
        view.id,
        taide_native_editor::view::SelectionSet {
            primary: 0,
            selections: vec![taide_native_editor::view::Selection {
                anchor: selection.start,
                head: selection.start,
            }],
        },
        view.scroll,
        view.folds,
    )?;
    taide_native_editor::folding::reveal_carets(store, view.id)?;
    store.request_selection_reveal(view.id, selection.start..selection.start, true)?;
    Ok(Some(view.id))
}

pub const REVEAL_TTL: Duration = Duration::from_secs(5);

pub struct Position {
    pub line: f64,
    pub column: f64,
}

pub struct Target<'a> {
    pub project: &'a ProjectId,
    pub tab: &'a TabId,
    pub path: &'a str,
    pub viewport: ViewportId,
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
        self.queue_position(
            Target {
                project: &opened.project,
                tab: &opened.tab,
                path: &opened.path,
                viewport: opened.viewport,
            },
            Position {
                line: opened.line,
                column: opened.column,
            },
            layouts,
            now,
        )
    }

    pub fn queue_position(
        &mut self,
        target: Target<'_>,
        position: Position,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        now: Instant,
    ) -> bool {
        self.reconcile(layouts, now);
        if !is_live(layouts, target.project, target.tab, target.path) {
            return false;
        }
        self.pending.insert(
            target.tab.clone(),
            Pending {
                project: target.project.clone(),
                viewport: target.viewport,
                path: target.path.to_owned(),
                position,
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

#[cfg(test)]
mod tests {
    use super::*;
    use taide_model::file::{FileSizeTier, OpenedFile};
    use taide_model::ids::PaneId;
    use taide_native_editor::document::{Edit, EditorError, UndoGroup};
    use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
    use taide_native_editor::view::ViewKey;

    const TEXT: &str = "a\u{1f600}\nb";
    const BYTE_LIMIT: usize = 1024;

    #[test]
    fn 문서_reveal은_실제_뷰와_byte를_검사하고_mirror_원문_readonly_및_닫힘을_보존한다() {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 2,
            max_views: 3,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let tab = TabId::new();
        let id = store
            .open_untitled(tab.clone(), TEXT, "json".into())
            .unwrap();
        let key = ViewKey {
            window: "reveal-test".into(),
            pane: PaneId::new(),
            tab: tab.clone(),
        };
        let view = store.attach_view(key.clone(), id).unwrap();
        let mirror = store
            .attach_view(
                ViewKey {
                    pane: PaneId::new(),
                    ..key.clone()
                },
                id,
            )
            .unwrap();
        let snapshot = store.documents().snapshot(id).unwrap();
        let byte = TEXT.find('b').unwrap();
        assert_eq!(
            reveal_document(&mut store, &key, &snapshot, byte..byte + 1),
            Ok(Some(view))
        );
        assert_eq!(
            store.views().get(view).unwrap().selection.selections[0].head,
            byte
        );
        assert_eq!(
            store.views().get(mirror).unwrap().selection.selections[0].head,
            0
        );
        assert_eq!(
            store.documents().snapshot(id).unwrap().revision,
            snapshot.revision
        );
        assert_eq!(
            reveal_document(&mut store, &key, &snapshot, 2..TEXT.len()),
            Err(EditorError::InvalidBoundary)
        );
        store
            .apply(
                id,
                Transaction {
                    revision: 0,
                    edits: vec![Edit {
                        bytes: 0..0,
                        text: "x".into(),
                    }],
                    group: UndoGroup(1),
                    origin: None,
                    selection_after: None,
                },
            )
            .unwrap();
        assert_eq!(
            reveal_document(&mut store, &key, &snapshot, byte..byte + 1),
            Ok(None)
        );
        let current = store.documents().snapshot(id).unwrap();
        store.detach_view(view).unwrap();
        assert_eq!(
            reveal_document(&mut store, &key, &current, byte..byte + 1),
            Ok(None)
        );
        let path = std::path::PathBuf::from("/synthetic/readonly-reveal.json");
        let readonly = store
            .open_file(
                path.clone(),
                OpenedFile {
                    path: path.to_string_lossy().into_owned(),
                    content: TEXT.into(),
                    language_id: "json".into(),
                    byte_size: TEXT.len().try_into().unwrap(),
                    line_count: TEXT.lines().count().try_into().unwrap(),
                    tier: FileSizeTier::Normal,
                    read_only: true,
                    encoding_lossy: false,
                    modified_ms: 0.0,
                    editor_config: Default::default(),
                },
            )
            .unwrap();
        let readonly_key = ViewKey {
            tab: TabId::new(),
            ..key
        };
        let view = store.attach_view(readonly_key.clone(), readonly).unwrap();
        let snapshot = store.documents().snapshot(readonly).unwrap();
        assert_eq!(
            reveal_document(&mut store, &readonly_key, &snapshot, byte..byte + 1),
            Ok(Some(view))
        );
        assert_eq!(
            store
                .documents()
                .snapshot(readonly)
                .unwrap()
                .rope
                .to_string(),
            TEXT
        );
        assert_eq!(
            store.documents().snapshot(readonly).unwrap().revision,
            snapshot.revision
        );
    }
}

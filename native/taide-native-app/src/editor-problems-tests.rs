use super::*;

use std::collections::HashSet;
use taide_lsp::native::protocol::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_ui::editor_problems::Provider as ProblemProvider;

const DOCUMENT_LIMIT: usize = 3;
const VIEW_LIMIT: usize = 6;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;

struct Fixture {
    store: EditorStore,
    source: ViewId,
    target: ViewId,
    state: State,
    diagnostics: crate::diagnostics::Store,
    commands: Vec<crate::host::HostCommand>,
    project: ProjectId,
}

impl Fixture {
    fn new(read_only: bool) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let mut views = Vec::new();
        for (path, text) in [
            ("/synthetic/a.rs", "abc def ghi"),
            ("/synthetic/b.rs", "α😀 def"),
        ] {
            let document = store
                .open_file(
                    path.into(),
                    OpenedFile {
                        path: path.into(),
                        content: text.into(),
                        language_id: "rust".into(),
                        byte_size: text.len().try_into().unwrap(),
                        line_count: 1,
                        tier: FileSizeTier::Normal,
                        read_only,
                        encoding_lossy: false,
                        modified_ms: 0.0,
                        editor_config: EditorConfigOptions::default(),
                    },
                )
                .unwrap();
            views.push(
                store
                    .attach_view(
                        ViewKey {
                            window: "problem-test".into(),
                            pane: PaneId::new(),
                            tab: TabId::new(),
                        },
                        document,
                    )
                    .unwrap(),
            );
        }
        let source = views[0];
        let target = views[1];
        let owner = crate::diagnostics::Owner::new();
        let documents = HashSet::from([
            store.views().get(source).unwrap().document,
            store.views().get(target).unwrap().document,
        ]);
        let mut diagnostics = crate::diagnostics::Store::default();
        diagnostics.retain_documents(documents.clone());
        diagnostics.reconcile(HashMap::from([(owner, documents)]));
        let diagnostic = |start, end, severity| Diagnostic {
            range: Range::new(Position::new(0, start), Position::new(0, end)),
            severity: Some(severity),
            message: format!("problem {start}"),
            ..Default::default()
        };
        diagnostics.publish(
            owner,
            &store
                .documents()
                .snapshot(store.views().get(source).unwrap().document)
                .unwrap(),
            vec![
                diagnostic(0, 3, DiagnosticSeverity::WARNING),
                diagnostic(4, 7, DiagnosticSeverity::ERROR),
                diagnostic(8, 11, DiagnosticSeverity::HINT),
            ],
        );
        diagnostics.publish(
            owner,
            &store
                .documents()
                .snapshot(store.views().get(target).unwrap().document)
                .unwrap(),
            vec![diagnostic(4, 7, DiagnosticSeverity::ERROR)],
        );
        Self {
            store,
            source,
            target,
            state: State::default(),
            diagnostics,
            commands: Vec::new(),
            project: ProjectId::new(),
        }
    }

    fn execute(&mut self, view: ViewId, command: Command) {
        let mut provider = Provider {
            state: &mut self.state,
            diagnostics: &self.diagnostics,
            project: Some(self.project.clone()),
            commands: &mut self.commands,
            viewport: eframe::egui::ViewportId::ROOT,
        };
        assert!(provider.execute(&mut self.store, view, command).unwrap());
    }

    fn current(&mut self, view: ViewId) -> Option<Widget> {
        let mut provider = Provider {
            state: &mut self.state,
            diagnostics: &self.diagnostics,
            project: Some(self.project.clone()),
            commands: &mut self.commands,
            viewport: eframe::egui::ViewportId::ROOT,
        };
        provider.current(&self.store, view)
    }

    fn request(&mut self) -> Request {
        match self.commands.pop().unwrap() {
            crate::host::HostCommand::OpenMarker(request) => request,
            _ => panic!("unexpected host command"),
        }
    }

    fn opened(&self, request: &Request) -> crate::terminal_tabs::OpenedFileLink {
        let pane = PaneId::new();
        let tab = TabId::new();
        let mut layout = taide_layout::service::default_layout();
        layout.focused_pane = pane.clone();
        layout.root = taide_model::layout::PaneNode::Leaf {
            id: pane.clone(),
            active: Some(tab.clone()),
            tabs: vec![taide_model::layout::Tab {
                id: tab.clone(),
                kind: taide_model::layout::TabKind::File {
                    path: request.path.clone(),
                },
                title: "b.rs".into(),
                pinned: false,
                preview: true,
                dirty: false,
                view_state: None,
            }],
        };
        crate::terminal_tabs::OpenedFileLink {
            project: self.project.clone(),
            pane,
            tab,
            path: request.path.clone(),
            line: request.line as f64,
            column: request.column as f64,
            viewport: request.viewport,
            layout,
        }
    }
}

#[test]
fn 문제_현재파일_이동은_읽기전용_접기_다중선택과_독립뷰_수명을_보존한다() {
    let mut fixture = Fixture::new(true);
    let source = fixture.source;
    let before = fixture.store.views().get(source).unwrap().clone();
    fixture
        .store
        .set_view_state(
            source,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 1, head: 2 },
                    Selection { anchor: 0, head: 0 },
                ],
            },
            before.scroll,
            vec![0..11],
        )
        .unwrap();
    fixture.execute(source, Command::Next);
    let shown = fixture.current(source).unwrap();
    assert_eq!(
        (
            shown.coordinate.index,
            shown.coordinate.total,
            shown.position
        ),
        (1, 2, 4)
    );
    let state = fixture.store.views().get(source).unwrap();
    assert_eq!(
        state.selection.selections,
        vec![Selection { anchor: 4, head: 4 }]
    );
    assert!(state.folds.is_empty());
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(state.document)
            .unwrap()
            .revision,
        0
    );
    let other = fixture
        .store
        .attach_view(
            ViewKey {
                window: "second".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            state.document,
        )
        .unwrap();
    assert!(fixture.current(other).is_none());
    fixture.execute(source, Command::Next);
    assert_eq!(
        fixture
            .current(source)
            .unwrap()
            .coordinate
            .problem
            .marker
            .message
            .severity,
        taide_native_editor::diagnostics::Severity::Warning
    );
    fixture.execute(source, Command::Close);
    assert!(fixture.current(source).is_none());
    assert!(fixture.commands.is_empty());
}

#[test]
fn 파일간_문제_이동은_utf16_좌표와_편집_anchor를_보존하고_대상뷰에서_이어간다() {
    let mut fixture = Fixture::new(false);
    let source = fixture.source;
    fixture.execute(source, Command::NextInFiles);
    fixture.execute(source, Command::NextInFiles);
    fixture.execute(source, Command::NextInFiles);
    let request = fixture.request();
    assert_eq!((request.line, request.column), (1, 5));
    assert_eq!(request.path, "/synthetic/b.rs");
    assert!(fixture.state.accepts(&request, &fixture.store));
    let document = fixture
        .store
        .documents()
        .snapshot(fixture.store.views().get(fixture.target).unwrap().document)
        .unwrap();
    fixture
        .store
        .apply(
            document.id,
            Transaction {
                revision: document.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "X".into(),
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    let opened = fixture.opened(&request);
    assert!(
        fixture
            .state
            .opened(&request, &opened, &mut fixture.store, &fixture.diagnostics)
            .unwrap()
    );
    assert!(!fixture.state.accepts(&request, &fixture.store));
    let view = fixture
        .store
        .views()
        .find(&ViewKey {
            window: "problem-test".into(),
            pane: opened.pane,
            tab: opened.tab,
        })
        .unwrap();
    let widget = fixture.current(view).unwrap();
    assert_eq!(
        (
            widget.coordinate.index,
            widget.coordinate.total,
            widget.position
        ),
        (3, 3, 8)
    );
    assert_eq!(
        fixture
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        8
    );
    fixture.execute(view, Command::PreviousInFiles);
    assert_eq!(fixture.request().path, "/synthetic/a.rs");
}

#[test]
fn 늦은_문제_응답은_닫힘_요청교체_owner폐기와_뷰폐기_뒤_적용하지_않는다() {
    let mut fixture = Fixture::new(false);
    let source = fixture.source;
    for _ in 0..3 {
        fixture.execute(source, Command::NextInFiles);
    }
    let old = fixture.request();
    fixture.execute(source, Command::Close);
    let opened = fixture.opened(&old);
    assert!(
        !fixture
            .state
            .opened(&old, &opened, &mut fixture.store, &fixture.diagnostics)
            .unwrap()
    );
    for _ in 0..3 {
        fixture.execute(source, Command::NextInFiles);
    }
    let request = fixture.request();
    assert_ne!(old.token, request.token);
    assert!(!fixture.state.accepts(&old, &fixture.store));
    fixture.state.failed(&old);
    assert!(fixture.state.accepts(&request, &fixture.store));
    fixture.diagnostics.reconcile(HashMap::new());
    let opened = fixture.opened(&request);
    assert!(
        !fixture
            .state
            .opened(&request, &opened, &mut fixture.store, &fixture.diagnostics)
            .unwrap()
    );
    assert!(fixture.current(source).is_none());
    fixture.store.detach_view(source).unwrap();
    fixture.state.retain(&fixture.store);
    assert!(!fixture.state.accepts(&request, &fixture.store));
}

#[test]
fn 문제_응답의_대상탭이_이미_닫힌_layout이면_뷰를_열지_않는다() {
    let mut fixture = Fixture::new(false);
    let source = fixture.source;
    for _ in 0..3 {
        fixture.execute(source, Command::NextInFiles);
    }
    let request = fixture.request();
    let mut opened = fixture.opened(&request);
    if let taide_model::layout::PaneNode::Leaf { active, .. } = &mut opened.layout.root {
        *active = None;
    }
    assert!(
        !fixture
            .state
            .opened(&request, &opened, &mut fixture.store, &fixture.diagnostics)
            .unwrap()
    );
}

#[test]
fn 문제_응답은_현재_layout과_같을_때만_적용하고_보조창의_입력원을_확인한다() {
    use taide_model::layout::{AuxWindowLayout, PaneNode};
    let mut fixture = Fixture::new(false);
    let source = fixture.source;
    for _ in 0..3 {
        fixture.execute(source, Command::NextInFiles);
    }
    let request = fixture.request();
    let opened = fixture.opened(&request);
    let mut layouts = HashMap::from([(opened.project.clone(), opened.layout.clone())]);
    assert!(reply_is_current(&opened, &layouts));
    layouts.get_mut(&opened.project).unwrap().revision += 1;
    assert!(!reply_is_current(&opened, &layouts));
    layouts.clear();
    assert!(!reply_is_current(&opened, &layouts));
    let mut layout = opened.layout.clone();
    layout.root = PaneNode::Leaf {
        id: request.source_key.pane.clone(),
        active: Some(request.source_key.tab.clone()),
        tabs: vec![taide_model::layout::Tab {
            id: request.source_key.tab.clone(),
            kind: TabKind::File {
                path: "/synthetic/a.rs".into(),
            },
            title: "a.rs".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
    };
    layout.focused_pane = request.source_key.pane.clone();
    assert!(request.source_is_active(&layout));
    let root = layout.root.clone();
    layout.root = taide_layout::service::default_layout().root;
    layout.focused_pane = PaneId::new();
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: 1,
        root,
        focused_pane: request.source_key.pane.clone(),
    });
    assert!(request.source_is_active(&layout));
    layout.auxiliary_windows[0].focused_pane = PaneId::new();
    assert!(!request.source_is_active(&layout));
}

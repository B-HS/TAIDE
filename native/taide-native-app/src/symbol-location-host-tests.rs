use super::*;
use std::collections::HashSet;
use std::sync::Arc;

use taide_model::{
    app_event::AppEvent,
    file::{EditorConfigOptions, FileSizeTier, OpenedFile},
    ids::{PaneId, ProjectId, ShellSlotId, TabId},
    layout::{AuxWindowLayout, PaneNode},
    paths::AppPaths,
    project::{Project, ShellSlotTree},
};
use taide_native_editor::{
    lsp::{LspRange, Position},
    store::{EditorLimits, EditorStore},
    symbol_locations::{Kind, Mode},
    view::ViewKey,
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const BYTES: usize = 4096;
const AUX_SLOT: u32 = 4;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const HORIZONTAL_PADDING: f32 = 8.0;

struct Events;
impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: std::path::PathBuf,
    services: Arc<AppServices>,
    request: Request,
    store: EditorStore,
    locations: crate::editor_locations::State,
}

impl Fixture {
    fn new(auxiliary: bool) -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-symbol-location-host-{}", ProjectId::new()));
        let root = directory.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.rs");
        let target = root.join("target.rs");
        std::fs::write(&source, "source").unwrap();
        std::fs::write(&target, "class\n  \u{1f600}method\nend").unwrap();
        let source = source.canonicalize().unwrap();
        let target = target.canonicalize().unwrap();
        let project = ProjectId::new();
        let pane = PaneId::new();
        let tab = TabId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.canonicalize().unwrap().to_str().unwrap().into(),
                name: "synthetic".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state.session.write().shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: ShellSlotId::new(),
            project_id: project.clone(),
        });
        let leaf = PaneNode::Leaf {
            id: pane.clone(),
            tabs: vec![Tab {
                id: tab.clone(),
                kind: TabKind::File {
                    path: source.to_str().unwrap().into(),
                },
                title: "source.rs".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: Some(tab.clone()),
        };
        let mut layout = taide_layout::service::default_layout();
        let scope = if auxiliary {
            layout.auxiliary_windows.push(AuxWindowLayout {
                slot: AUX_SLOT,
                root: leaf,
                focused_pane: pane.clone(),
            });
            WindowScope::Auxiliary {
                project: project.clone(),
                slot: AUX_SLOT,
            }
        } else {
            layout.root = leaf;
            layout.focused_pane = pane.clone();
            WindowScope::Main
        };
        let revision = layout.revision;
        state.layouts.write().insert(project.clone(), layout);
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 2,
            max_views: 2,
            max_undo_groups: 2,
            max_document_bytes: BYTES,
        })
        .unwrap();
        let document = store
            .open_file(
                source.clone(),
                OpenedFile {
                    path: source.to_str().unwrap().into(),
                    content: "source".into(),
                    language_id: "rust".into(),
                    byte_size: 6,
                    line_count: 1,
                    tier: FileSizeTier::Normal,
                    read_only: false,
                    encoding_lossy: false,
                    modified_ms: 0.0,
                    editor_config: EditorConfigOptions::default(),
                },
            )
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "synthetic".into(),
                    pane,
                    tab,
                },
                document,
            )
            .unwrap();
        let mut locations = crate::editor_locations::State::default();
        locations
            .begin(
                project,
                &store,
                view,
                Kind::Definition,
                Mode::GoTo,
                HashSet::new(),
                None,
            )
            .unwrap();
        let (source, token, cancelled) = locations.begin_open(&store, view).unwrap();
        let range = LspRange::new(Position::new(1, 4), Position::new(1, 10));
        let request = Request {
            source,
            token,
            cancelled,
            target: Target {
                uri: taide_lsp::service::workspace_folder_uri(target.to_str().unwrap())
                    .parse()
                    .unwrap(),
                range,
                selection: range,
                origin: None,
            },
            side: false,
            keep_peek: false,
            revision,
            scope,
            viewport: eframe::egui::ViewportId::ROOT,
        };
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Events),
        );
        Self {
            directory,
            services,
            request,
            store,
            locations,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn 위치_host는_현재_주창과_보조창의_pane에서_파일을_열고_utf16_좌표를_보존한다() {
    for auxiliary in [false, true] {
        let f = Fixture::new(auxiliary);
        let before = f.services.state.layouts.read()[&f.request.source.project].clone();
        let opened = open(&f.services, &f.request).await.unwrap();
        assert_eq!(opened.pane, f.request.source.source_key.pane);
        assert_eq!((opened.line, opened.column), (2.0, 5.0));
        let (root, focused) =
            crate::symbol_sidebar::window_tree(&opened.project, &opened.layout, &f.request.scope)
                .unwrap();
        assert_eq!(focused, &opened.pane);
        let tab = taide_native_ui::snapshot::active_tab(root, focused).unwrap();
        assert_eq!(tab.id, opened.tab);
        assert!(tab.preview);
        assert!(f.request.source.describes(&f.store, true));
        if auxiliary {
            assert_eq!(opened.layout.root, before.root);
        }
        f.services.tasks.shutdown().await;
    }
}

#[tokio::test]
async fn 위치_host의_옆_열기는_현재창_오른쪽에_새_그룹을_열고_원본_탭을_보존한다() {
    for auxiliary in [false, true] {
        let mut f = Fixture::new(auxiliary);
        f.request.side = true;
        let opened = open(&f.services, &f.request).await.unwrap();
        assert_ne!(opened.pane, f.request.source.source_key.pane);
        let (root, focused) =
            crate::symbol_sidebar::window_tree(&opened.project, &opened.layout, &f.request.scope)
                .unwrap();
        assert_eq!(taide_layout::service::collect_leaves(root).len(), 2);
        assert_eq!(focused, &opened.pane);
        assert_eq!(
            taide_native_ui::snapshot::active_tab(root, &f.request.source.source_key.pane)
                .unwrap()
                .id,
            f.request.source.source_key.tab
        );
        f.services.tasks.shutdown().await;
    }
}

#[tokio::test]
async fn 위치_host는_취소_프로젝트_닫힘_탭_교체_레이아웃_변경과_루트_밖_대상을_거절한다() {
    for case in 0..5 {
        let mut f = Fixture::new(false);
        match case {
            0 => f.locations.close(f.request.source.source),
            1 => {
                f.services.state.projects.write().clear();
            }
            2 => {
                let mut layouts = f.services.state.layouts.write();
                let layout = layouts.get_mut(&f.request.source.project).unwrap();
                if let PaneNode::Leaf { active, .. } = &mut layout.root {
                    *active = None;
                }
            }
            3 => {
                f.services
                    .state
                    .layouts
                    .write()
                    .get_mut(&f.request.source.project)
                    .unwrap()
                    .revision += 1
            }
            _ => {
                let outside = f.directory.join("outside.rs");
                std::fs::write(&outside, "outside").unwrap();
                f.request.target.uri =
                    taide_lsp::service::workspace_folder_uri(outside.to_str().unwrap())
                        .parse()
                        .unwrap();
            }
        }
        let before = f.services.state.layouts.read().clone();
        assert!(open(&f.services, &f.request).await.is_err(), "case {case}");
        assert_eq!(*f.services.state.layouts.read(), before);
        f.services.tasks.shutdown().await;
    }
}

#[tokio::test]
async fn 연속_참조_이동은_같은_파일로_돌아와도_앞선_파일_열기를_취소한다() {
    let mut f = Fixture::new(false);
    let layout = f.services.state.layouts.read()[&f.request.source.project].clone();
    let editor = taide_native_ui::editor_surface::NativeEditor {
        appearance: taide_native_ui::editor_surface::EditorAppearance {
            font: eframe::egui::FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: HORIZONTAL_PADDING,
            background: eframe::egui::Color32::BLACK,
            foreground: eframe::egui::Color32::WHITE,
            muted: eframe::egui::Color32::GRAY,
            selection: eframe::egui::Color32::BLUE,
            cursor: eframe::egui::Color32::WHITE,
            current_line: eframe::egui::Color32::DARK_GRAY,
            line_numbers: true,
            indent: "    ".into(),
        },
    };
    let models = crate::peek_models::Models::default();
    let presentation = taide_native_ui::editor_surface::EditorPresentation::default();
    let mut commands = Vec::new();
    let mut changed = Default::default();
    let mut focus_targets = Vec::new();
    let mut shown_lines = Vec::new();
    let source = f.request.source.clone();
    let target = f.request.target.clone();
    let taide_native_editor::document::DocumentKey::File(path) = &source.snapshot.key else {
        panic!("expected file");
    };
    let same = Target::from_location(taide_lsp::native::protocol::lsp_types::Location::new(
        taide_lsp::service::workspace_folder_uri(path.to_str().unwrap())
            .parse()
            .unwrap(),
        LspRange::new(Position::new(0, 0), Position::new(0, 3)),
    ));
    let mut provider = crate::editor_locations::Provider {
        state: &mut f.locations,
        models: &models,
        project: Some(source.project.clone()),
        lsp: None,
        layout: Some(&layout),
        scope: &f.request.scope,
        commands: &mut commands,
        viewport: eframe::egui::ViewportId::ROOT,
        editor: &editor,
        presentation: &presentation,
        tokens: None,
        hover_tokens: None,
        shown_lines: &mut shown_lines,
        changed: &mut changed,
        overrides: None,
        focus_targets: &mut focus_targets,
        find_history: None,
        find_appearance: None,
    };
    assert!(
        provider
            .open(&mut f.store, source.source, target, false, true)
            .unwrap()
    );
    assert!(
        provider
            .open(&mut f.store, source.source, same, false, true)
            .unwrap()
    );
    assert!(f.locations.current(source.source).is_some());
    let crate::host::HostCommand::OpenSymbolLocation(request) = commands.remove(0) else {
        panic!("expected location open");
    };
    let before = f.services.state.layouts.read().clone();
    assert!(open(&f.services, &request).await.is_err());
    assert_eq!(*f.services.state.layouts.read(), before);
    f.services.tasks.shutdown().await;
}

use super::*;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::Selection;

const BYTE_LIMIT: usize = 4096;
const PATH: &str = "/synthetic/source.rs";

fn file(path: &str) -> OpenedFile {
    OpenedFile {
        path: path.into(),
        content: "class\n  \u{1f600}method\nend".into(),
        language_id: "rust".into(),
        byte_size: 24,
        line_count: 3,
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 0.0,
        editor_config: EditorConfigOptions::default(),
    }
}

fn fixture() -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 3,
        max_views: 3,
        max_undo_groups: 2,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store.open_file(PATH.into(), file(PATH)).unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, document, view)
}

#[test]
fn 열린_peek의_anchor는_본문_편집을_추적하고_preview_view는_닫을때_회수한다() {
    let (mut store, document, view) = fixture();
    let project = ProjectId::new();
    let provider = provider(1);
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::Definition,
            Mode::Peek,
            providers.clone(),
            Some(Position::new(1, 4)),
        )
        .unwrap();
    state
        .accept(&request, &store, providers.clone(), Ok(response(provider)))
        .unwrap()
        .unwrap();
    state.show(view);
    assert_eq!(state.widget(&store, view).unwrap().position, 12);
    assert_eq!(
        state.prepare_preview(&mut store, view).unwrap(),
        Some(document)
    );
    let preview = state.current(view).unwrap().preview.unwrap();
    assert_ne!(preview, view);
    assert_eq!(
        state.preview_bindings(&store),
        vec![(project.clone(), document)]
    );
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "new ".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    state.reconcile(&store, &HashSet::from([project]), |_, _, _| {
        providers.clone()
    });
    assert!(!request.is_cancelled());
    assert_eq!(state.widget(&store, view).unwrap().position, 16);
    assert!(state.preview_views().contains(&preview));
    state.close(view);
    state.detach_retired(&mut store);
    assert!(state.preview_bindings(&store).is_empty());
    assert!(request.is_cancelled());
    assert!(store.views().get(preview).is_none());
    assert!(store.views().get(view).is_some());
    assert!(store.documents().snapshot(document).unwrap().dirty);
}

fn response(provider: ProviderIdentity) -> Response {
    Response {
        groups: vec![Group {
            provider,
            targets: vec![Target::from_location(lsp_types::Location::new(
                taide_lsp::service::workspace_folder_uri(PATH)
                    .parse()
                    .unwrap(),
                lsp_types::Range::new(Position::new(1, 4), Position::new(1, 10)),
            ))],
        }],
    }
}

fn provider(generation: u64) -> ProviderIdentity {
    ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation,
        capability_revision: 0,
    }
}

#[test]
fn 위치_응답은_요청_선택_provider와_문서_버전을_확인한다() {
    let (mut store, document, view) = fixture();
    let mut current = store.views().get(view).unwrap().clone();
    current.selection = SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: 12,
            head: 12,
        }],
    };
    store
        .set_view_state(view, current.selection, current.scroll, current.folds)
        .unwrap();
    let project = ProjectId::new();
    let provider = provider(1);
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::Definition,
            Mode::GoTo,
            providers.clone(),
            None,
        )
        .unwrap();
    assert_eq!(request.position, Position::new(1, 4));
    let model = state
        .accept(&request, &store, providers.clone(), Ok(response(provider)))
        .unwrap()
        .unwrap();
    assert_eq!(model.targets().len(), 1);
    assert_eq!(state.current(view).unwrap().selected, Some(0));
    let old = request;
    let request = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::References,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    assert!(old.is_cancelled());
    assert!(
        state
            .accept(&old, &store, providers.clone(), Ok(response(provider)))
            .unwrap()
            .is_none()
    );
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "new ".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    state.reconcile(&store, &HashSet::from([project]), |_, _, _| {
        providers.clone()
    });
    assert!(request.is_cancelled());
    assert!(state.current(view).is_none());
    assert!(
        state
            .accept(&request, &store, providers, Ok(response(provider)))
            .unwrap()
            .is_none()
    );
}

#[test]
fn 이동선택_뷰닫힘_프로젝트종료_재시작은_독립적인_보류요청을_회수한다() {
    for invalidation in ["selection", "view", "project", "provider"] {
        let (mut store, _, view) = fixture();
        let project = ProjectId::new();
        let provider = provider(1);
        let providers = HashSet::from([provider]);
        let mut state = State::default();
        let request = state
            .begin(
                project.clone(),
                &store,
                view,
                Kind::Definition,
                Mode::Peek,
                providers.clone(),
                None,
            )
            .unwrap();
        let mut projects = HashSet::from([project]);
        let mut current_providers = providers;
        match invalidation {
            "selection" => {
                let mut current = store.views().get(view).unwrap().clone();
                current.selection = SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor: 1, head: 1 }],
                };
                store
                    .set_view_state(view, current.selection, current.scroll, current.folds)
                    .unwrap();
            }
            "view" => {
                store.detach_view(view).unwrap();
            }
            "project" => projects.clear(),
            "provider" => current_providers.clear(),
            _ => unreachable!(),
        }
        state.reconcile(&store, &projects, |_, _, _| current_providers.clone());
        assert!(request.is_cancelled(), "{invalidation}");
        assert!(state.current(view).is_none());
    }
}

#[test]
fn 실패한_provider와_늦은_세대는_현재_정상결과를_덮어쓰지않는다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider(1);
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            project,
            &store,
            view,
            Kind::Definition,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    assert!(
        state
            .accept(
                &request,
                &store,
                providers.clone(),
                Ok(response(ProviderIdentity {
                    generation: 2,
                    ..provider
                }))
            )
            .unwrap()
            .is_none()
    );
    assert!(
        state
            .accept(
                &request,
                &store,
                providers.clone(),
                Err(Failure::TransportClosed)
            )
            .is_err()
    );
    assert!(
        state
            .accept(&request, &store, providers, Ok(response(provider)))
            .unwrap()
            .is_some()
    );
    state.close(view);
    assert!(request.is_cancelled());
}

#[test]
fn 파일_uri는_퍼센트경로만_해석하고_원격_쿼리_조각_nul을_거절한다() {
    assert_eq!(
        file_path(&"file:///synthetic/a%20b%23.rs".parse().unwrap()),
        Some("/synthetic/a b#.rs".into())
    );
    for uri in [
        "https://example.test/a.rs",
        "file://remote.test/a.rs",
        "file:///a.rs?x",
        "file:///a.rs#x",
        "file:///a%00.rs",
    ] {
        assert!(file_path(&uri.parse().unwrap()).is_none(), "{uri}");
    }
}

#[test]
fn hover의_교체와_늦은_닫힘은_열린_peek와_새_token을_취소하지_않는다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let peek = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::References,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    state
        .accept(&peek, &store, providers.clone(), Ok(response(identity)))
        .unwrap();
    state.show(view);
    let old = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::Definition,
            Mode::Hover,
            providers.clone(),
            None,
        )
        .unwrap();
    let hover = state
        .begin(
            project,
            &store,
            view,
            Kind::Definition,
            Mode::Hover,
            providers,
            None,
        )
        .unwrap();
    assert!(old.is_cancelled());
    state.close_request(&old);
    assert!(state.is_current(&hover));
    assert!(state.is_current(&peek));
    state.close_request(&hover);
    assert!(hover.is_cancelled());
    assert!(state.current(view).unwrap().shown);
    assert!(state.is_current(&peek));
}

#[test]
fn 실제_preview의_직렬_본문_명령과_저장은_대상_문서와_포커스를_사용한다() {
    use eframe::egui::{
        Color32, Context, Event, FontId, Modifiers, RawInput, Rect, UiBuilder, vec2,
    };
    use taide_native_ui::editor_locations::Provider as _;
    use taide_native_ui::editor_surface::{EditorAppearance, EditorPresentation, NativeEditor};
    const FONT_SIZE: f32 = 14.0;
    const LINE_HEIGHT: f32 = 20.0;
    const SCREEN_WIDTH: f32 = 640.0;
    const SCREEN_HEIGHT: f32 = 480.0;
    let (mut store, source_document, view) = fixture();
    let path = "/synthetic/target.rs";
    let target_document = store.open_file(path.into(), file(path)).unwrap();
    let project = ProjectId::new();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let request = state
        .begin(
            project,
            &store,
            view,
            Kind::Definition,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    state
        .accept(
            &request,
            &store,
            providers,
            Ok(Response {
                groups: vec![Group {
                    provider: identity,
                    targets: vec![Target::from_location(lsp_types::Location::new(
                        taide_lsp::service::workspace_folder_uri(path)
                            .parse()
                            .unwrap(),
                        lsp_types::Range::new(Position::new(1, 4), Position::new(1, 10)),
                    ))],
                }],
            }),
        )
        .unwrap();
    state.show(view);
    let editor = NativeEditor {
        appearance: EditorAppearance {
            font: FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: 8.0,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            selection: Color32::BLUE,
            cursor: Color32::WHITE,
            current_line: Color32::DARK_GRAY,
            line_numbers: true,
            indent: "    ".into(),
        },
    };
    let mut presentation = EditorPresentation::default();
    presentation.options.folding = true;
    let models = crate::peek_models::Models::default();
    let scope = taide_native_ui::shell::WindowScope::Main;
    let mut commands = Vec::new();
    let mut changed = HashMap::new();
    let mut targets = Vec::new();
    let mut shown_lines = Vec::new();
    let context = Context::default();
    context.set_os(eframe::egui::os::OperatingSystem::Mac);
    let overrides = r#"[{"actionId":"monaco.editor.action.transformToUppercase","key":"u","mods":["mod"]},{"actionId":"monaco.editor.foldAll","key":"j","mods":["mod"]}]"#;
    let mut frame = |events,
                     state: &mut State,
                     store: &mut EditorStore,
                     commands: &mut Vec<crate::host::HostCommand>,
                     changed: &mut HashMap<DocumentId, ViewId>,
                     targets: &mut Vec<(eframe::egui::Id, ViewId)>| {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    eframe::egui::Pos2::ZERO,
                    vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let rect = ui.available_rect_before_wrap();
                let mut ui =
                    ui.new_child(UiBuilder::new().id_salt("actual-preview").max_rect(rect));
                let mut provider = Provider {
                    state,
                    models: &models,
                    project: None,
                    lsp: None,
                    layout: None,
                    scope: &scope,
                    commands,
                    viewport: context.viewport_id(),
                    editor: &editor,
                    presentation: &presentation,
                    tokens: None,
                    hover_tokens: None,
                    shown_lines: &mut shown_lines,
                    changed,
                    overrides: Some(overrides),
                    focus_targets: targets,
                    find_history: None,
                    find_appearance: None,
                    documentation: None,
                    completion: None,
                    highlights: None,
                };
                provider
                    .render_preview(&mut ui, store, view, rect, false)
                    .unwrap();
            },
        );
        output.textures_delta.clear();
    };
    frame(
        Vec::new(),
        &mut state,
        &mut store,
        &mut commands,
        &mut changed,
        &mut targets,
    );
    let preview = state.current(view).unwrap().preview.unwrap();
    let focus = targets
        .iter()
        .find(|(_, owner)| *owner == preview)
        .unwrap()
        .0;
    context.memory_mut(|memory| memory.request_focus(focus));
    let current = store.views().get(preview).unwrap().clone();
    store
        .set_view_state(
            preview,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: 12,
                    head: 18,
                }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    let key = |key| Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::MAC_CMD | Modifiers::COMMAND,
    };
    frame(
        vec![key(eframe::egui::Key::U), key(eframe::egui::Key::S)],
        &mut state,
        &mut store,
        &mut commands,
        &mut changed,
        &mut targets,
    );
    assert_eq!(
        store
            .documents()
            .snapshot(target_document)
            .unwrap()
            .rope
            .to_string(),
        "class\n  \u{1f600}METHOD\nend"
    );
    assert_eq!(
        store
            .documents()
            .snapshot(source_document)
            .unwrap()
            .revision,
        0
    );
    assert_eq!(changed.get(&target_document), Some(&preview));
    assert_eq!(commands.len(), 1);
    let crate::host::HostCommand::Save {
        path: saved_path,
        snapshot,
    } = &commands[0]
    else {
        panic!("unexpected preview command");
    };
    assert_eq!(saved_path, path);
    assert_eq!(snapshot.document(), target_document);
    assert_eq!(snapshot.revision(), 1);
    assert_eq!(snapshot.rope().to_string(), "class\n  \u{1f600}METHOD\nend");
    assert!(state.local_key(preview, &key(eframe::egui::Key::S)));
    assert!(state.local_key(view, &key(eframe::egui::Key::F12)));
    frame(
        vec![key(eframe::egui::Key::J)],
        &mut state,
        &mut store,
        &mut commands,
        &mut changed,
        &mut targets,
    );
    frame(
        Vec::new(),
        &mut state,
        &mut store,
        &mut commands,
        &mut changed,
        &mut targets,
    );
    assert!(!store.views().get(preview).unwrap().folds.is_empty());
    assert!(store.views().get(view).unwrap().folds.is_empty());
    assert_eq!(
        store
            .documents()
            .snapshot(target_document)
            .unwrap()
            .revision,
        1
    );
}

#[test]
fn 추적_이력을_잃은_peek는_이전_참조로_이동하지_않고_미리보기만_회수한다() {
    let (mut store, document, view) = fixture();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let request = state
        .begin(
            ProjectId::new(),
            &store,
            view,
            Kind::References,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    state
        .accept(&request, &store, providers, Ok(response(identity)))
        .unwrap();
    state.show(view);
    state.prepare_preview(&mut store, view).unwrap();
    let preview = state.current(view).unwrap().preview.unwrap();
    for _ in 0..=taide_native_editor::change_journal::MAX_JOURNAL_ENTRIES {
        let snapshot = store.documents().snapshot(document).unwrap();
        store
            .apply(
                document,
                Transaction {
                    revision: snapshot.revision,
                    edits: vec![Edit {
                        bytes: 0..0,
                        text: "new\n".into(),
                    }],
                    group: UndoGroup(snapshot.revision),
                    origin: Some(view),
                    selection_after: None,
                },
            )
            .unwrap();
    }
    assert!(state.widget(&store, view).is_none());
    assert!(state.current(view).is_none());
    state.detach_retired(&mut store);
    assert!(store.views().get(preview).is_none());
    assert!(store.views().get(view).is_some());
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert!(request.is_cancelled());
}

#[test]
fn 참조_범위와_이동_강조는_줄_삽입을_추적하고_정해진_시간에_회수한다() {
    let (mut store, document, view) = fixture();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let request = state
        .begin(
            ProjectId::new(),
            &store,
            view,
            Kind::References,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    state
        .accept(&request, &store, providers, Ok(response(identity)))
        .unwrap();
    state.show(view);
    state.prepare_preview(&mut store, view).unwrap();
    let target = state
        .current(view)
        .unwrap()
        .model
        .as_ref()
        .unwrap()
        .targets()[0]
        .clone();
    let now = Instant::now();
    state.navigate(&mut store, view, &target, now).unwrap();
    let reveal = store.take_selection_reveal(view).unwrap().unwrap();
    assert_eq!(reveal.bytes, 12..12);
    assert!(reveal.near_top_if_outside);
    assert!(!reveal.center_if_outside);
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "new\n".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let widget = state.widget(&store, view).unwrap();
    assert_eq!(
        widget.model.targets()[0].selection,
        lsp_types::Range::new(Position::new(2, 4), Position::new(2, 10))
    );
    let (highlight, remaining) = state.highlight(&store, view, now, [1, 2, 3, 4]).unwrap();
    assert_eq!(highlight.items()[0].bytes, 16..22);
    assert_eq!(remaining, NAVIGATION_HIGHLIGHT_TTL);
    assert!(
        state
            .highlight(&store, view, now + NAVIGATION_HIGHLIGHT_TTL, [1, 2, 3, 4])
            .is_none()
    );
    assert!(state.current(view).is_some());
}

#[test]
fn 파일_열기_취소는_해결된_peek를_보존하고_최신_원본문_편집을_확인한다() {
    let (mut store, document, view) = fixture();
    let project = ProjectId::new();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let request = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::References,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    state
        .accept(&request, &store, providers.clone(), Ok(response(identity)))
        .unwrap();
    state.show(view);
    let (_, before, cancelled) = state.begin_open(&store, view).unwrap();
    let (_, latest, latest_cancelled) = state.begin_open(&store, view).unwrap();
    assert_ne!(before, latest);
    assert!(*cancelled.borrow());
    assert!(!*latest_cancelled.borrow());
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "new\n".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    state.reconcile(&store, &HashSet::from([project]), |_, _, _| {
        providers.clone()
    });
    assert!(*latest_cancelled.borrow());
    assert!(state.is_current(&request));
    assert!(state.current(view).unwrap().shown);
}

#[test]
fn 파일_열기의_레이아웃_이벤트는_아직_도착하지_않은_응답을_취소하지_않는다() {
    use taide_model::layout::{PaneNode, Tab, TabKind};
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let request = state
        .begin(
            project.clone(),
            &store,
            view,
            Kind::Definition,
            Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    state
        .accept(&request, &store, providers, Ok(response(identity)))
        .unwrap();
    state.show(view);
    let (_, _, cancelled) = state.begin_open(&store, view).unwrap();
    let source = store.views().get(view).unwrap();
    let mut layout = taide_layout::service::default_layout();
    let destination = TabId::new();
    layout.root = PaneNode::Leaf {
        id: source.key.pane.clone(),
        tabs: vec![Tab {
            id: destination.clone(),
            kind: TabKind::File {
                path: "/synthetic/target.rs".into(),
            },
            title: "target.rs".into(),
            pinned: false,
            preview: true,
            dirty: false,
            view_state: None,
        }],
        active: Some(destination),
    };
    layout.focused_pane = source.key.pane.clone();
    state.retain_active(
        &HashMap::from([(project, layout)]),
        &taide_native_ui::shell::WindowScope::Main,
    );
    assert!(!*cancelled.borrow());
    assert!(state.is_current(&request));
}

#[test]
fn peek_인계는_대상_뷰와_미리보기를_보존하고_프로젝트_종료_탭_변경_만료에_회수한다() {
    use taide_model::layout::{PaneNode, Tab, TabKind};
    for case in ["adopt", "project", "tab", "expiry"] {
        let (mut store, _, source) = fixture();
        let project = ProjectId::new();
        let identity = provider(1);
        let providers = HashSet::from([identity]);
        let mut state = State::default();
        let request = state
            .begin(
                project.clone(),
                &store,
                source,
                Kind::References,
                Mode::Peek,
                providers.clone(),
                None,
            )
            .unwrap();
        state
            .accept(&request, &store, providers.clone(), Ok(response(identity)))
            .unwrap();
        state.show(source);
        state.prepare_preview(&mut store, source).unwrap();
        let preview = state.current(source).unwrap().preview.unwrap();
        state.sessions.get_mut(&source).unwrap().focus =
            Some((0, taide_native_ui::editor_locations::Focus::Preview));
        let path = "/synthetic/target.rs";
        let target = store.open_file(path.into(), file(path)).unwrap();
        let key = ViewKey {
            window: "synthetic".into(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: key.pane.clone(),
            tabs: vec![Tab {
                id: key.tab.clone(),
                kind: TabKind::File { path: path.into() },
                title: "target.rs".into(),
                pinned: false,
                preview: true,
                dirty: false,
                view_state: None,
            }],
            active: Some(key.tab.clone()),
        };
        layout.focused_pane = key.pane.clone();
        let opened = crate::terminal_tabs::OpenedFileLink {
            project: project.clone(),
            pane: key.pane.clone(),
            tab: key.tab.clone(),
            path: path.into(),
            line: 1.0,
            column: 1.0,
            viewport: eframe::egui::ViewportId::ROOT,
            layout,
        };
        let now = Instant::now();
        state.rehome(source, &opened, now);
        assert!(state.current(source).is_none());
        assert!(state.preview_views().contains(&preview));
        let destination = store.attach_view(key, target).unwrap();
        if case == "adopt" {
            assert!(
                !state
                    .adopt_pending(&store, destination, &project, PATH, opened.viewport, now)
                    .unwrap()
            );
            assert!(
                state
                    .adopt_pending(&store, destination, &project, path, opened.viewport, now)
                    .unwrap()
            );
            let widget = state.widget(&store, destination).unwrap();
            assert_eq!(
                widget.focus,
                Some((0, taide_native_ui::editor_locations::Focus::Preview))
            );
            assert_eq!(state.current(destination).unwrap().preview, Some(preview));
            state.close(destination);
        } else if case == "project" {
            state.reconcile(&store, &HashSet::new(), |_, _, _| providers.clone());
        } else {
            let mut layout = opened.layout;
            if case == "tab" {
                if let PaneNode::Leaf { active, .. } = &mut layout.root {
                    *active = None;
                }
            }
            if case == "expiry" {
                state.pending_peeks[0].deadline = now;
            }
            state.retain_active(
                &HashMap::from([(project, layout)]),
                &taide_native_ui::shell::WindowScope::Main,
            );
        }
        state.detach_retired(&mut store);
        assert!(request.is_cancelled(), "{case}");
        assert!(state.pending_peeks.is_empty(), "{case}");
        assert!(store.views().get(preview).is_none(), "{case}");
        assert!(store.views().get(destination).is_some());
    }
}

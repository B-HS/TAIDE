use super::*;

const SCREEN: egui::Vec2 = egui::vec2(800.0, 600.0);

fn frame(context: &egui::Context, state: &mut State, pane: &PaneId, events: Vec<egui::Event>) {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            events,
            ..Default::default()
        },
        |ui| {
            state.toolbar(ui, pane, "markdown", "Preview");
            state.regions(ui, pane, "markdown", 1.0, egui::Color32::GRAY);
        },
    );
    output.textures_delta.clear();
}

#[test]
fn 실제_버튼과_divider_드래그_키보드는_양쪽_최소폭을_보존한다() {
    let context = egui::Context::default();
    let pane = PaneId::new();
    let key = (context.viewport_id(), pane.clone());
    let mut state = State::default();
    frame(&context, &mut state, &pane, Vec::new());
    let position = state.entries[&key].toggle.center();
    for pressed in [true, false] {
        frame(
            &context,
            &mut state,
            &pane,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(state.entries[&key].enabled);
    let left = state.entries[&key].editor;
    let right = state.entries[&key].preview.unwrap();
    let divider = egui::pos2(
        (left.right() + right.left()) * INITIAL_FRACTION,
        left.center().y,
    );
    frame(
        &context,
        &mut state,
        &pane,
        vec![
            egui::Event::PointerMoved(divider),
            egui::Event::PointerButton {
                pos: divider,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let moved = divider - egui::vec2(SCREEN.x, 0.0);
    frame(
        &context,
        &mut state,
        &pane,
        vec![egui::Event::PointerMoved(moved)],
    );
    frame(
        &context,
        &mut state,
        &pane,
        vec![egui::Event::PointerButton {
            pos: moved,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(state.entries[&key].fraction, MIN_FRACTION);
    context.memory_mut(|memory| memory.request_focus(egui::Id::new(("markdown-divider", &key))));
    frame(
        &context,
        &mut state,
        &pane,
        vec![egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: Some(egui::Key::ArrowRight),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(state.entries[&key].fraction > MIN_FRACTION);
    assert!(state.entries[&key].fraction <= MAX_FRACTION);
}

#[test]
fn 분할_토글은_언어별로_표시하고_기존_폭과_비활성_상태를_보존한다() {
    let context = egui::Context::default();
    let pane = PaneId::new();
    let key = (context.viewport_id(), pane.clone());
    let mut state = State::default();
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |ui| {
            state.toolbar(ui, &pane, "markdown", "Preview");
            let whole = state.regions(ui, &pane, "markdown", 1.0, egui::Color32::GRAY);
            assert!(whole.1.is_none());
            state.entries.get_mut(&key).unwrap().enabled = true;
            let (editor, preview) = state.regions(ui, &pane, "markdown", 1.0, egui::Color32::GRAY);
            let preview = preview.unwrap();
            assert!((editor.width() - preview.width()).abs() < 1.0);
            assert_eq!(editor.right() + 1.0, preview.left());
            state.entries.get_mut(&key).unwrap().fraction = MIN_FRACTION;
            state.toolbar(ui, &pane, "rust", "Preview");
            assert!(
                state
                    .regions(ui, &pane, "rust", 1.0, egui::Color32::GRAY)
                    .1
                    .is_none()
            );
            state.toolbar(ui, &pane, "markdown", "Preview");
            assert!(
                state
                    .regions(ui, &pane, "markdown", 1.0, egui::Color32::GRAY)
                    .1
                    .is_some()
            );
            assert_eq!(state.entries[&key].fraction, MIN_FRACTION);
        },
    );
    output.textures_delta.clear();
    let store = EditorStore::new(taide_native_editor::store::EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1,
    })
    .unwrap();
    state.reconcile(&context, &HashMap::new(), &store);
    assert!(state.entries.is_empty());
}

#[test]
fn 사라진_viewport의_분할_상태는_같은_pane이_남아도_회수한다() {
    let context = egui::Context::default();
    let mut state = State::default();
    let project = ProjectId::new();
    let layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let layouts = HashMap::from([(project, layout)]);
    let store = EditorStore::new(taide_native_editor::store::EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: 1,
    })
    .unwrap();
    let root = egui::ViewportId::ROOT;
    let child = egui::ViewportId::from_hash_of("markdown child");
    for viewport in [root, child] {
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: viewport,
                viewports: [root, child]
                    .into_iter()
                    .map(|id| {
                        (
                            id,
                            egui::ViewportInfo {
                                parent: (id != root).then_some(root),
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                state.toolbar(ui, &pane, "markdown", "Preview");
                state
                    .entries
                    .get_mut(&(viewport, pane.clone()))
                    .unwrap()
                    .enabled = viewport == child;
            },
        );
        output.textures_delta.clear();
    }
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |_| state.reconcile(&context, &layouts, &store),
    );
    output.textures_delta.clear();
    assert!(!state.entries.contains_key(&(child, pane.clone())));
    assert!(!state.entries[&(root, pane.clone())].enabled);
    assert_eq!(state.entries.len(), 1);
}

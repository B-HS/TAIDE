use eframe::egui;
use egui::{Color32, Context, Event, FontId, Modifiers, RawInput, Rect, pos2, vec2};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::locale::ResolvedLocale;
use taide_model::tree::{TreeEntryKind, TreeRow, TreeRowPage};
use taide_native_app::explorer::{Action, Explorer, Output};
use taide_native_editor::editing::select_all;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key as WinitKey, KeyCode, PhysicalKey};
use winit::window::WindowId;

#[path = "../../../experiments/native-shell-spike/vendor/eframe/src/native/paste_shortcuts.rs"]
mod paste_shortcuts;

const SCREEN: [f32; 2] = [400.0, 300.0];
const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 1024;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 8.0;

fn frame(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    events: Vec<Event>,
) -> Output {
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: [("explorer.pasteConflictSuffix".into(), "copy".into())].into(),
    };
    let mut result = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            result = Some(explorer.show(ui, project, page, &locale));
        },
    );
    output.textures_delta.clear();
    result.unwrap()
}

#[test]
fn 빈_paste의_실제adapter출력은_파일작업_한번만_만들고_editor선택을_보존한다() {
    let command = if cfg!(target_os = "macos") {
        Modifiers::COMMAND | Modifiers::MAC_CMD
    } else {
        Modifiers::COMMAND | Modifiers::CTRL
    };
    let window = WindowId::dummy();
    let mut adapter = paste_shortcuts::PasteShortcuts::default();
    let mut raw = RawInput::default();
    raw.events.push(Event::ModifiersChanged(command));
    adapter.on_window_event(window, &WindowEvent::Focused(true), 0, &mut raw);
    let first_event = raw.events.len();
    assert!(adapter.preserve_key(
        window,
        &WinitKey::Character("v".into()),
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        first_event,
        &mut raw
    ));
    assert!(
        !raw.events
            .iter()
            .any(|event| matches!(event, Event::Paste(_) | Event::Text(_)))
    );
    let context = Context::default();
    let project = ProjectId::new();
    let page = TreeRowPage {
        total: 1,
        rows: vec![TreeRow {
            name: "source.txt".into(),
            path: "/synthetic/source.txt".into(),
            depth: 0,
            kind: TreeEntryKind::File,
            expanded: false,
            has_children: false,
        }],
    };
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    explorer.selected = Some(page.rows[0].path.clone());
    frame(&context, &mut explorer, &project, &page, Vec::new());
    context
        .memory_mut(|memory| memory.request_focus(egui::Id::new(("native-tree-focus", &project))));
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![Event::ModifiersChanged(command), Event::Copy]
        )
        .actions
        .is_empty()
    );
    let output = frame(&context, &mut explorer, &project, &page, raw.events.clone());
    let [Action::Paste(request)] = output.actions.as_slice() else {
        panic!("expected one payload-independent paste");
    };
    assert_eq!(request.entry.path, page.rows[0].path);
    assert_eq!(request.target, "/synthetic");
    assert!(request.sibling_names.contains("source.txt"));
    assert!(!context.input(|input| input.key_down(egui::Key::Paste)));
    assert!(
        frame(&context, &mut explorer, &project, &page, Vec::new())
            .actions
            .is_empty()
    );
    context.memory_mut(|memory| {
        memory.surrender_focus(egui::Id::new(("native-tree-focus", &project)))
    });
    assert!(
        frame(&context, &mut explorer, &project, &page, raw.events.clone())
            .actions
            .is_empty()
    );
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_untitled(tab.clone(), "selection must survive", "plaintext".into())
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab,
            },
            document,
        )
        .unwrap();
    select_all(&mut store, view).unwrap();
    let before = store.documents().snapshot(document).unwrap();
    let selection = store.views().get(view).unwrap().selection.clone();
    let editor = NativeEditor {
        appearance: EditorAppearance {
            font: FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: PADDING,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            selection: Color32::BLUE,
            cursor: Color32::WHITE,
            current_line: Color32::DARK_GRAY,
            line_numbers: false,
            indent: "    ".into(),
        },
    };
    let editor_context = Context::default();
    for events in [Vec::new(), raw.events] {
        let mut output = editor_context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                let output = editor.show(ui, &mut store, view, true).unwrap();
                assert!(!output.changed);
                assert!(!output.save_requested);
                assert!(output.errors.is_empty());
            },
        );
        output.textures_delta.clear();
    }
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope, before.rope);
    assert_eq!(store.views().get(view).unwrap().selection, selection);
}

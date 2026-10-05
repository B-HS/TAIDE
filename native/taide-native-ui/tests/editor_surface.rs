use egui::{Color32, Context, Event, FontId, ImeEvent, Key, Modifiers, RawInput, Rect, pos2, vec2};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, EditorError, UndoGroup};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 16;
const BYTE_LIMIT: usize = 2 * 1024 * 1024;
const ROWS: usize = 50_000;
const SCROLL_LINE: usize = 40_000;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 8.0;
const SCREEN: [f32; 2] = [800.0, 200.0];
const REVEAL_PREFIX: usize = 1000;
const CENTER_DIVISOR: f32 = 2.0;
const FILE_INDENT_SIZE: u32 = 2;

#[test]
fn 파일별_들여쓰기는_실제_tab_입력에_적용되고_다른_editor의_기본값을_바꾸지_않는다() {
    let base = editor();
    for (spaces, expected) in [(true, "  draft"), (false, "\tdraft")] {
        let (mut store, view) = fixture("draft", false);
        let context = Context::default();
        let configured = base.with_indent(IndentOptions {
            tab_size: FILE_INDENT_SIZE,
            insert_spaces: spaces,
        });
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events: vec![key(Key::Tab, false)],
                ..Default::default()
            },
            |ui| {
                let shown = configured.show(ui, &mut store, view, true).unwrap();
                assert!(shown.changed);
                assert!(shown.errors.is_empty());
            },
        );
        output.textures_delta.clear();
        assert_eq!(text(&store, view), expected);
    }
    assert_eq!(base.appearance.indent, "    ");
    assert_eq!(base.appearance.font.size, FONT_SIZE);
}

#[test]
fn external_keymap은_재지정한_저장의_예전_cmd_s를_실행하지_않는다() {
    let (mut store, view) = fixture("draft", false);
    let context = Context::default();
    let editor = editor();
    let mut save_requested = false;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![key(Key::S, true)],
            ..Default::default()
        },
        |ui| {
            save_requested = editor
                .show_with_keymap(ui, &mut store, view, true, |_, _, _| false)
                .unwrap()
                .save_requested;
        },
    );
    output.textures_delta.clear();
    assert!(!save_requested);
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![key(Key::S, true)],
            ..Default::default()
        },
        |ui| {
            save_requested = editor
                .show(ui, &mut store, view, true)
                .unwrap()
                .save_requested;
        },
    );
    output.textures_delta.clear();
    assert!(save_requested);
    assert_eq!(text(&store, view), "draft");
}

#[test]
fn reveal은_단일_readonly_editor를_중앙표시하고_수평_cursor와_focus를_보존한다() {
    let target = format!("{}漢𐐀e\u{301}", "x".repeat(REVEAL_PREFIX));
    let text = (0..ROWS)
        .map(|line| {
            if line == SCROLL_LINE {
                format!("{target}\n")
            } else {
                format!("line {line}\n")
            }
        })
        .collect::<String>();
    let (mut store, view) = fixture(&text, true);
    let document = store.views().get(view).unwrap().document;
    let other = store
        .attach_view(
            ViewKey {
                window: "auxiliary".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let initial = store.documents().snapshot(document).unwrap();
    let context = Context::default();
    let mut viewport = Rect::NOTHING;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let editor = editor();
            editor
                .reveal(
                    ui,
                    &mut store,
                    view,
                    (SCROLL_LINE + 1) as f64,
                    (target.encode_utf16().count() + 1) as f64,
                )
                .unwrap();
            let shown = editor.show(ui, &mut store, view, true).unwrap();
            assert!(shown.rendered_lines.contains(&SCROLL_LINE));
            assert!(shown.response.has_focus());
            assert!(!shown.changed);
            assert!(shown.errors.is_empty());
            let scroll = &store.views().get(view).unwrap().scroll;
            assert!(scroll.x > 0.0);
            let expected = ((SCROLL_LINE as f32 + 1.0 / CENTER_DIVISOR) * LINE_HEIGHT
                - viewport.height() / CENTER_DIVISOR)
                .max(0.0);
            assert_eq!(scroll.y, expected);
        },
    );
    output.textures_delta.clear();
    assert!(output.platform_output.ime.is_none());
    let cursor = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::LineSegment { points, stroke }
                if stroke.color == Color32::WHITE =>
            {
                Some(Rect::from_two_pos(points[0], points[1]))
            }
            _ => None,
        })
        .expect("focused readonly editor paints its caret");
    assert!(cursor.left() >= viewport.left());
    assert!(cursor.right() <= viewport.right());
    assert!((cursor.center().y - viewport.center().y).abs() <= LINE_HEIGHT);
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, initial.revision);
    assert_eq!(after.dirty, initial.dirty);
    assert_eq!(after.rope, initial.rope);
    assert_eq!(
        store.views().get(other).unwrap().selection,
        SelectionSet::default()
    );
    assert_eq!(
        store.views().get(other).unwrap().scroll,
        ScrollPosition::default()
    );
}

fn fixture(text: &str, read_only: bool) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_file(
            "/synthetic/editor.rs".into(),
            OpenedFile {
                path: "/synthetic/editor.rs".into(),
                content: text.into(),
                language_id: "rust".into(),
                byte_size: u32::try_from(text.len()).unwrap(),
                line_count: 1,
                tier: FileSizeTier::Normal,
                read_only,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
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
    (store, view)
}

fn editor() -> NativeEditor {
    NativeEditor {
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
            line_numbers: true,
            indent: "    ".into(),
        },
    }
}

fn frame(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    events: Vec<Event>,
    focus: bool,
) -> (std::ops::Range<usize>, Vec<EditorError>, bool) {
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
            let output = editor().show(ui, store, view, focus).unwrap();
            result = Some((output.rendered_lines, output.errors, output.save_requested));
        },
    );
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
    result.unwrap()
}

fn key(key: Key, command: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            command,
            ..Modifiers::default()
        },
    }
}

#[test]
fn 비활성_editor는_확인창_뒤의_text_ime와_focus를_소비하지_않는다() {
    let (mut store, view) = fixture("abc", false);
    let document = store.views().get(view).unwrap().document;
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    let before = store.documents().snapshot(document).unwrap();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![
                Event::Text("must not enter the disabled document".into()),
                Event::Ime(egui::ImeEvent::Commit("確認".into())),
            ],
            ..Default::default()
        },
        |ui| {
            ui.add_enabled_ui(false, |ui| {
                let output = editor().show(ui, &mut store, view, true).unwrap();
                assert!(!output.changed);
            });
            assert_eq!(ui.input(|input| input.events.len()), 2);
        },
    );
    output.textures_delta.clear();
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope, before.rope);
}

fn text(store: &EditorStore, view: ViewId) -> String {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
        .rope
        .to_string()
}

#[test]
fn 실제_editor_surface는_입력_선택_ime_commit과_stale_거절을_연결한다() {
    let (mut store, view) = fixture("abc", false);
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    let (_, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![key(Key::End, false), Event::Text("한".into())],
        false,
    );
    assert!(errors.is_empty());
    assert_eq!(text(&store, view), "abc한");
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "글".into(),
            active_range_chars: None,
        })],
        false,
    );
    assert_eq!(text(&store, view), "abc한");
    assert!(store.views().get(view).unwrap().composition.is_some());
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Commit("글".into()))],
        false,
    );
    assert_eq!(text(&store, view), "abc한글");
    frame(&context, &mut store, view, vec![key(Key::Z, true)], false);
    assert_eq!(text(&store, view), "abc한");
    let (_, errors, save) = frame(&context, &mut store, view, vec![key(Key::S, true)], false);
    assert!(errors.is_empty());
    assert!(save);
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "旧".into(),
            active_range_chars: None,
        })],
        false,
    );
    let document = store.views().get(view).unwrap().document;
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "external".into(),
                }],
                group: UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    let before = text(&store, view);
    let (_, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Commit("旧".into()))],
        false,
    );
    assert_eq!(errors, vec![EditorError::StaleRevision]);
    assert_eq!(text(&store, view), before);
}

#[test]
fn 큰_document의_화면은_보이는_줄만_렌더하고_readonly_입력을_거절한다() {
    let text = "let x = 1;\n".repeat(ROWS);
    let (mut store, view) = fixture(&text, true);
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 0, head: 0 }],
            },
            ScrollPosition {
                x: 0.0,
                y: SCROLL_LINE as f32 * LINE_HEIGHT,
            },
            current.folds,
        )
        .unwrap();
    let context = Context::default();
    let (lines, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![Event::Paste("must-not-write".into())],
        true,
    );
    assert_eq!(lines.start, SCROLL_LINE);
    assert!(lines.len() <= (SCREEN[1] / LINE_HEIGHT).ceil() as usize + 1);
    assert_eq!(errors, vec![EditorError::ReadOnly]);
    assert_eq!(
        store
            .documents()
            .snapshot(current.document)
            .unwrap()
            .revision,
        0
    );
    let second = store
        .attach_view(
            ViewKey {
                window: "aux".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            current.document,
        )
        .unwrap();
    let auxiliary = Context::default();
    let (lines, _, _) = frame(&auxiliary, &mut store, second, Vec::new(), true);
    assert_eq!(lines.start, 0);
    assert_eq!(
        store.views().get(view).unwrap().scroll.y,
        SCROLL_LINE as f32 * LINE_HEIGHT
    );
}

#[test]
fn 커서_이동과_큰_선택_삭제는_스크롤을_현재_문서로_복원한다() {
    let (mut store, view) = fixture(&"row\n".repeat(ROWS), false);
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    let (lines, errors, _) = frame(&context, &mut store, view, vec![key(Key::End, true)], false);
    assert!(errors.is_empty());
    assert!(lines.start > SCROLL_LINE);
    frame(&context, &mut store, view, vec![key(Key::A, true)], false);
    let (lines, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![Event::Text("small".into())],
        false,
    );
    assert!(errors.is_empty());
    assert_eq!(lines, 0..1);
    assert_eq!(store.views().get(view).unwrap().scroll.y, 0.0);
    assert_eq!(text(&store, view), "small");
}

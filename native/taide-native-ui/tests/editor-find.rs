#![cfg(feature = "native-host")]

use std::cell::Cell;

use egui::{
    Color32, Event, FontData, FontDefinitions, FontFamily, FontId, Key, Modifiers, RawInput, Vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::editing::type_text;
use taide_native_editor::find::{
    FindCaptures, FindOptions, FindPattern, FindPatternCompiler, FindPatternError,
    FindPatternOptions,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_find::{EditorFind, FindCommand, FindError};
use taide_native_ui::editor_find_widget::{FindAppearance, FindHistory, ICON_FAMILY, show};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorPresentation, EditorRequest, NativeEditor,
};

const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024 * 1024;
const VIEW_WIDTH: f32 = 800.0;
const VIEW_HEIGHT: f32 = 400.0;
const LINE_HEIGHT: f32 = 21.0;
const FONT_SIZE: f32 = 14.0;
const PADDING: f32 = 8.0;

#[derive(Default)]
struct RejectCompiler(Cell<usize>);

impl FindPatternCompiler for RejectCompiler {
    fn compile(
        &self,
        _: &str,
        _: FindPatternOptions,
    ) -> Result<Box<dyn FindPattern>, FindPatternError> {
        self.0.set(self.0.get() + 1);
        Err(FindPatternError("Invalid pattern".into()))
    }
}

fn fixture(text: &str, read_only: bool) -> (EditorStore, ViewId, EditorFind) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 2,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/find-widget.txt".into(),
            OpenedFile {
                path: "/synthetic/find-widget.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
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
                window: "find-test".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let mut find = EditorFind::default();
    find.options = FindOptions {
        match_case: true,
        ..Default::default()
    };
    (store, view, find)
}

fn text(store: &EditorStore, view: ViewId) -> String {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
        .rope
        .to_string()
}

fn select(store: &mut EditorStore, view: ViewId, anchor: usize, head: usize) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor, head }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn selected(store: &EditorStore, view: ViewId) -> Selection {
    store.views().get(view).unwrap().selection.selections[0]
}

#[test]
fn 찾기는_입력과_문서_revision이_변할_때만_다시_평가하고_오류를_보존한다() {
    let (mut store, view, mut find) = fixture("cat", false);
    let compiler = RejectCompiler::default();
    find.search = "[".into();
    find.options.is_regex = true;
    assert!(find.refresh(&store, view, &compiler).unwrap());
    assert!(!find.refresh(&store, view, &compiler).unwrap());
    assert_eq!(compiler.0.get(), 1);
    type_text(&mut store, view, "dog").unwrap();
    assert!(find.refresh(&store, view, &compiler).unwrap());
    assert_eq!(compiler.0.get(), 1);
    assert_eq!(find.error.as_deref(), Some("Invalid pattern"));
    find.options.is_regex = false;
    find.search = "cat".into();
    assert!(find.refresh(&store, view, &compiler).unwrap());
    assert!(find.error.is_none());
    assert_eq!(find.results().matches.len(), 1);
}

#[test]
fn 탐색은_순환하고_치환은_먼저_일치를_선택한_뒤_한_단계로_적용한다() {
    let (mut store, view, mut find) = fixture("cat dog cat", false);
    let compiler = RejectCompiler::default();
    find.search = "cat".into();
    find.replacement = "fish".into();
    find.refresh(&store, view, &compiler).unwrap();
    assert!(
        !find
            .execute(FindCommand::ReplaceOne, &mut store, view, &compiler, None)
            .unwrap()
    );
    assert_eq!(selected(&store, view), Selection { anchor: 0, head: 3 });
    assert_eq!(text(&store, view), "cat dog cat");
    assert!(
        find.execute(FindCommand::ReplaceOne, &mut store, view, &compiler, None)
            .unwrap()
    );
    assert_eq!(text(&store, view), "fish dog cat");
    assert_eq!(
        selected(&store, view),
        Selection {
            anchor: 9,
            head: 12
        }
    );
    find.execute(FindCommand::Next, &mut store, view, &compiler, None)
        .unwrap();
    assert_eq!(
        selected(&store, view),
        Selection {
            anchor: 9,
            head: 12
        }
    );
    let document = store.views().get(view).unwrap().document;
    assert!(store.undo(document).unwrap());
    assert_eq!(text(&store, view), "cat dog cat");
}

#[test]
fn 선택_범위는_여러_줄에서_줄_경계를_맞추고_치환과_undo를_따라간다() {
    let (mut store, view, mut find) = fixture("cat one\ncat two\ncat three", false);
    let compiler = RejectCompiler::default();
    find.search = "cat".into();
    find.replacement = "fish".into();
    select(&mut store, view, 2, 16);
    find.execute(FindCommand::ToggleScope, &mut store, view, &compiler, None)
        .unwrap();
    assert_eq!(find.scopes(), [0..15]);
    assert_eq!(find.results().matches.len(), 2);
    find.execute(FindCommand::ReplaceAll, &mut store, view, &compiler, None)
        .unwrap();
    assert_eq!(text(&store, view), "fish one\nfish two\ncat three");
    assert_eq!(find.scopes(), [0..17]);
    let document = store.views().get(view).unwrap().document;
    store.undo(document).unwrap();
    find.refresh(&store, view, &compiler).unwrap();
    assert_eq!(find.scopes(), [0..15]);
    assert_eq!(find.results().matches.len(), 2);
    find.execute(FindCommand::Close, &mut store, view, &compiler, None)
        .unwrap();
    assert!(find.scopes().is_empty());
    assert!(!find.visible);
}

#[test]
fn 수집_상한_뒤의_일치도_탐색하고_전체_치환은_옵션을_유지한다() {
    const MATCHES: usize = 20_000;
    let before = "cat cats\n".repeat(MATCHES);
    let (mut store, view, mut find) = fixture(&before, false);
    let compiler = RejectCompiler::default();
    find.search = "cat".into();
    find.replacement = "dog".into();
    find.options.whole_word = true;
    find.refresh(&store, view, &compiler).unwrap();
    assert!(find.results().limit_reached);
    let last = before.len() - "cat cats\n".len();
    select(&mut store, view, last, last);
    find.navigate(&mut store, view, true, true).unwrap();
    assert_eq!(
        selected(&store, view),
        Selection {
            anchor: last,
            head: last + 3
        }
    );
    find.execute(FindCommand::ReplaceAll, &mut store, view, &compiler, None)
        .unwrap();
    assert_eq!(text(&store, view), "dog cats\n".repeat(MATCHES));
}

#[test]
fn 읽기_전용_문서는_탐색할_수_있고_치환은_문서를_수정하지_않는다() {
    let (mut store, view, mut find) = fixture("cat cat", true);
    let compiler = RejectCompiler::default();
    find.execute(FindCommand::Open, &mut store, view, &compiler, None)
        .unwrap();
    assert_eq!(find.search, "cat");
    find.execute(FindCommand::Next, &mut store, view, &compiler, None)
        .unwrap();
    for command in [
        FindCommand::OpenReplace,
        FindCommand::ReplaceOne,
        FindCommand::ReplaceAll,
    ] {
        assert!(matches!(
            find.execute(command, &mut store, view, &compiler, None),
            Err(FindError::Editor(
                taide_native_editor::document::EditorError::ReadOnly
            ))
        ));
    }
    assert_eq!(text(&store, view), "cat cat");
}

fn appearance() -> FindAppearance {
    FindAppearance {
        background: Color32::BLACK,
        border: Color32::GRAY,
        foreground: Color32::WHITE,
        input_background: Color32::BLACK,
        input_foreground: Color32::WHITE,
        input_border: Color32::GRAY,
        focus: Color32::BLUE,
        active_background: Color32::BLUE,
        active_foreground: Color32::WHITE,
        active_border: Color32::BLUE,
        hover: Color32::GRAY,
        error: Color32::RED,
        highlight: Color32::YELLOW,
        current_match: Color32::YELLOW,
        scope: Color32::TRANSPARENT,
        shadow: Color32::BLACK,
        quick_input_background: Color32::BLACK,
        quick_input_foreground: Color32::WHITE,
        placeholder: Color32::GRAY,
        error_background: Color32::BLACK,
        error_foreground: Color32::WHITE,
    }
}

fn context() -> egui::Context {
    let context = egui::Context::default();
    let mut definitions = FontDefinitions::default();
    definitions.font_data.insert(
        ICON_FAMILY.into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../taide-native-app/assets/codicons/codicon.ttf"
        ))),
    );
    definitions.families.insert(
        FontFamily::Name(ICON_FAMILY.into()),
        vec![ICON_FAMILY.into()],
    );
    context.set_fonts(definitions);
    context
}

fn frame(
    context: &egui::Context,
    events: Vec<Event>,
    find: &mut EditorFind,
    history: &mut FindHistory,
    store: &mut EditorStore,
    view: ViewId,
) -> bool {
    frame_with_shapes(context, events, find, history, store, view).0
}

fn frame_with_shapes(
    context: &egui::Context,
    events: Vec<Event>,
    find: &mut EditorFind,
    history: &mut FindHistory,
    store: &mut EditorStore,
    view: ViewId,
) -> (bool, Vec<egui::epaint::ClippedShape>) {
    let mut editor_focused = false;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(VIEW_WIDTH, VIEW_HEIGHT),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            let widget = show(
                ui,
                find,
                history,
                store,
                view,
                &RejectCompiler::default(),
                None,
                &appearance(),
                |_, _, _| false,
            )
            .unwrap();
            ui.allocate_space(Vec2::new(ui.available_width(), widget.reserved_height));
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
                    current_line: Color32::TRANSPARENT,
                    line_numbers: true,
                    indent: "    ".into(),
                },
            };
            let layers = find.decorations(
                &store.views().get(view).unwrap().selection,
                Color32::YELLOW.to_array(),
                Color32::YELLOW.to_array(),
                Color32::TRANSPARENT.to_array(),
            );
            let layers = layers.iter().collect::<Vec<_>>();
            let output = editor
                .show_request(
                    ui,
                    store,
                    view,
                    EditorRequest {
                        request_focus: widget.request_editor_focus,
                        keymap: |_: &egui::Ui, _: &Event, _: bool| false,
                        route: |response: &egui::Response| {
                            response.ctx.keyboard_input_route(response.id)
                        },
                        presentation: &EditorPresentation::default(),
                        tokens: |_: &EditorStore| None,
                        language: None,
                        decorations: &layers,
                        fold_commands: &[],
                        fold_controls: None,
                    },
                )
                .unwrap();
            editor_focused = output.response.has_focus();
        },
    );
    output.textures_delta.clear();
    (editor_focused, output.shapes)
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn 찾기_입력과_탐색은_문서_타이핑으로_새지_않고_닫으면_문서로_포커스를_돌린다() {
    let (mut store, view, mut find) = fixture("cat dog cat", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    find.execute(FindCommand::Open, &mut store, view, &compiler, None)
        .unwrap();
    assert!(!frame(
        &context,
        vec![],
        &mut find,
        &mut history,
        &mut store,
        view
    ));
    frame(
        &context,
        vec![Event::Text("dog".into())],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(find.search, "dog");
    assert_eq!(text(&store, view), "cat dog cat");
    assert_eq!(selected(&store, view), Selection { anchor: 4, head: 7 });
    frame(
        &context,
        vec![key(Key::Enter, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(text(&store, view), "cat dog cat");
    assert!(frame(
        &context,
        vec![key(Key::Escape, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view
    ));
    assert!(!find.visible);
    frame(
        &context,
        vec![Event::Text("fish".into())],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(text(&store, view), "cat fish cat");
}

#[test]
fn 치환_입력의_enter는_문서에_줄바꿈을_넣지_않고_치환한다() {
    let (mut store, view, mut find) = fixture("cat cat", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    select(&mut store, view, 0, 3);
    find.execute(FindCommand::OpenReplace, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    frame(
        &context,
        vec![Event::Text("dog".into())],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(find.replacement, "dog");
    frame(
        &context,
        vec![key(Key::Enter, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(text(&store, view), "dog cat");
    assert_eq!(selected(&store, view), Selection { anchor: 4, head: 7 });
    assert_eq!(history.replacement(), ["dog"]);
}

#[test]
fn 찾기_입력의_ctrl_enter는_검색어에만_줄바꿈을_넣고_tab_enter는_옵션을_전환한다() {
    let (mut store, view, mut find) = fixture("cat\ncat", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    find.execute(FindCommand::Open, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    let input_focus = context.memory(|memory| memory.focused()).unwrap();
    frame(
        &context,
        vec![key(Key::ArrowRight, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    frame(
        &context,
        vec![key(Key::Enter, Modifiers::CTRL)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(context.memory(|memory| memory.focused()), Some(input_focus));
    assert_eq!(find.search, "cat\n");
    assert_eq!(text(&store, view), "cat\ncat");
    frame(
        &context,
        vec![key(Key::Tab, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    frame(
        &context,
        vec![key(Key::Enter, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert!(!find.options.match_case);
    assert_eq!(find.search, "cat\n");
    assert_eq!(text(&store, view), "cat\ncat");
}

#[test]
fn 일치_번호_입력은_음수_순번을_미리_선택하고_enter로_확정한다() {
    let (mut store, view, mut find) = fixture("cat cat cat", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    find.execute(FindCommand::Open, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    find.execute(FindCommand::GoToMatch, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    frame(
        &context,
        vec![Event::Text("-1".into())],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(find.match_number.as_deref(), Some("-1"));
    assert_eq!(
        selected(&store, view),
        Selection {
            anchor: 8,
            head: 11
        }
    );
    frame(
        &context,
        vec![key(Key::Enter, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert!(find.match_number.is_none());
    assert_eq!(text(&store, view), "cat cat cat");
}

#[test]
fn 찾기_입력에서_치환을_열면_검색어를_유지하고_치환_입력에_포커스를_준다() {
    let (mut store, view, mut find) = fixture("cat dog", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    find.execute(FindCommand::Open, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    select(&mut store, view, 4, 7);
    find.execute(FindCommand::OpenReplace, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    frame(
        &context,
        vec![Event::Text("fish".into())],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(find.search, "cat");
    assert_eq!(find.replacement, "fish");
    assert_eq!(text(&store, view), "cat dog");
}

#[test]
fn 비활성_찾기창은_이전_포커스가_남아도_키를_라우팅하거나_문서를_치환하지_않는다() {
    let (mut store, view, mut find) = fixture("cat cat", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    select(&mut store, view, 0, 3);
    find.replacement = "dog".into();
    find.execute(FindCommand::OpenReplace, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    let mut routed = 0;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(VIEW_WIDTH, VIEW_HEIGHT),
            )),
            events: vec![key(Key::Enter, Modifiers::NONE)],
            ..Default::default()
        },
        |ui| {
            ui.disable();
            show(
                ui,
                &mut find,
                &mut history,
                &mut store,
                view,
                &compiler,
                None,
                &appearance(),
                |_, _, _| {
                    routed += 1;
                    false
                },
            )
            .unwrap();
        },
    );
    output.textures_delta.clear();
    assert_eq!(routed, 0);
    assert_eq!(text(&store, view), "cat cat");
}

#[test]
fn 범위_추적이_유실되면_전체_문서로_넓혀_치환하지_않고_범위를_다시_선택하게_한다() {
    let (mut store, view, mut find) = fixture("cat\ncat", false);
    let compiler = RejectCompiler::default();
    find.search = "cat".into();
    find.replacement = "dog".into();
    select(&mut store, view, 0, 3);
    find.execute(FindCommand::ToggleScope, &mut store, view, &compiler, None)
        .unwrap();
    for _ in 0..=taide_native_editor::change_journal::MAX_JOURNAL_ENTRIES {
        type_text(&mut store, view, "x").unwrap();
    }
    let before = text(&store, view);
    assert!(
        find.execute(FindCommand::ReplaceAll, &mut store, view, &compiler, None)
            .is_err()
    );
    assert_eq!(text(&store, view), before);
    assert!(!find.scopes().is_empty());
    assert!(
        find.error
            .as_deref()
            .unwrap()
            .contains("Select the range again")
    );
    find.execute(FindCommand::ToggleScope, &mut store, view, &compiler, None)
        .unwrap();
    assert!(find.error.is_none());
    assert!(find.scopes().is_empty());
    assert_eq!(find.results().matches.len(), 1);
}

#[test]
fn ime_조합_중인_입력은_다음_프레임의_enter를_탐색_명령으로_소비하지_않는다() {
    let (mut store, view, mut find) = fixture("cat cat", false);
    let compiler = RejectCompiler::default();
    let context = context();
    let mut history = FindHistory::default();
    find.execute(FindCommand::Open, &mut store, view, &compiler, None)
        .unwrap();
    frame(&context, vec![], &mut find, &mut history, &mut store, view);
    frame(
        &context,
        vec![Event::Ime(egui::ImeEvent::Preedit {
            text: "cat".into(),
            active_range_chars: None,
        })],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    let before = selected(&store, view);
    frame(
        &context,
        vec![key(Key::Enter, Modifiers::NONE)],
        &mut find,
        &mut history,
        &mut store,
        view,
    );
    assert_eq!(selected(&store, view), before);
    assert_eq!(text(&store, view), "cat cat");
}

struct LineStartCompiler;
struct LineStart;

impl FindPatternCompiler for LineStartCompiler {
    fn compile(
        &self,
        _: &str,
        _: FindPatternOptions,
    ) -> Result<Box<dyn FindPattern>, FindPatternError> {
        Ok(Box::new(LineStart))
    }
}
impl FindPattern for LineStart {
    fn captures_at(&self, _: &str, start: usize) -> Result<Option<FindCaptures>, FindPatternError> {
        Ok((start == 0).then_some(FindCaptures {
            groups: vec![Some(0..0)],
        }))
    }
}

#[test]
fn 길이가_없는_정규식_일치도_입력창에_포커스가_있을_때_문서에_보인다() {
    let (mut store, view, mut find) = fixture("cat", false);
    find.search = "^".into();
    find.options.is_regex = true;
    find.visible = true;
    find.refresh(&store, view, &LineStartCompiler).unwrap();
    let context = context();
    let mut history = FindHistory::default();
    let (_, shapes) =
        frame_with_shapes(&context, vec![], &mut find, &mut history, &mut store, view);
    assert!(shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == Color32::YELLOW && rect.rect.width() > 0.0)));
}

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{
    self, Context, Event, Key, Modifiers, PointerButton, RawInput, Rect, pos2, vec2,
};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::locale::ResolvedLocale;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::tree::{TreeEntryKind, TreeRow, TreeRowPage};
use taide_native_app::bootstrap::services;
use taide_native_app::explorer::{Action, Explorer, Output};
use taide_native_app::lsp::{LspBridge, Reply};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_runtime::{AppState, EventSink, TaskSupervisor, tree_actions};
use tokio::sync::Notify;

const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 200.0;
const TIMEOUT: Duration = Duration::from_secs(5);
const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 1024;
const NARROW_WIDTH: f32 = 180.0;
const VIRTUAL_ROWS: usize = 40;
const SOURCE_ROW_HEIGHT: f32 = 22.0;
const SOURCE_HEADER_HEIGHT: f32 = 32.0;
const SOURCE_TOOLBAR_BUTTON_SIZE: f32 = 24.0;
const ROW_EDGE_OFFSET: f32 = 2.0;
const CLICK_SEQUENCE_RESET_SECONDS: f64 = 1.0;
const SEARCH_START_SECONDS: f64 = 10.0;
const MENU_TEST_HEIGHT: f32 = 480.0;
const SCROLL_OVERFLOW_ROW: usize = 20;
const SCROLL_START_ROW: usize = 2;
const SCROLL_EPSILON: f32 = egui::emath::GUI_ROUNDING;

fn menu_frame(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    events: Vec<Event>,
) -> Output {
    let mut result = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(WIDTH, MENU_TEST_HEIGHT),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            result = Some(explorer.show(ui, project, page, &locale()));
        },
    );
    output.textures_delta.clear();
    result.unwrap()
}

fn menu_click(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    position: egui::Pos2,
    button: PointerButton,
) -> Output {
    let mut output = None;
    for pressed in [true, false] {
        output = Some(menu_frame(
            context,
            explorer,
            project,
            page,
            vec![
                Event::ModifiersChanged(Modifiers::NONE),
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
        ));
    }
    output.unwrap()
}

#[test]
fn 옆으로열기는_파일메뉴에서만_선택창의_빈pane_분할요청을_만든다() {
    use taide_native_ui::shell::WindowScope;
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = selection_page();
    page.rows[1].kind = TreeEntryKind::Directory;
    let mut explorer = Explorer::default();
    let first = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = first.rows[&page.rows[0].path].rect.center();
    menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        position,
        PointerButton::Secondary,
    );
    let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = menu.menu["explorer.openToTheSide"].rect.center();
    let output = menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        position,
        PointerButton::Primary,
    );
    assert_eq!(
        output.actions,
        vec![Action::OpenToSide(page.rows[0].clone())]
    );
    let mut layout = taide_layout::service::default_layout();
    layout.root = taide_model::layout::PaneNode::Leaf {
        id: layout.focused_pane.clone(),
        tabs: Vec::new(),
        active: None,
    };
    let request = taide_native_app::explorer::open_to_side_request(
        &project,
        &page.rows[0],
        Some(&layout),
        &WindowScope::Main,
    )
    .unwrap();
    assert_eq!(request.project_id, project);
    assert_eq!(request.target_pane, layout.focused_pane);
    assert_eq!(request.edge, taide_model::layout::DropEdge::Right);
    assert_eq!(
        request.kind,
        taide_model::layout::TabKind::File {
            path: page.rows[0].path.clone()
        }
    );
    assert_eq!(request.title, page.rows[0].name);
    assert!(!request.preview);
    assert!(
        taide_native_app::explorer::open_to_side_request(
            &project,
            &page.rows[0],
            None,
            &WindowScope::Main
        )
        .is_none()
    );
    assert!(
        taide_native_app::explorer::open_to_side_request(
            &project,
            &page.rows[1],
            Some(&layout),
            &WindowScope::Main
        )
        .is_none()
    );
    assert!(
        taide_native_app::explorer::open_to_side_request(
            &project,
            &page.rows[0],
            Some(&layout),
            &WindowScope::Auxiliary {
                project: project.clone(),
                slot: 1
            }
        )
        .is_none()
    );
    let auxiliary_pane = taide_model::ids::PaneId::new();
    layout
        .auxiliary_windows
        .push(taide_model::layout::AuxWindowLayout {
            slot: 1,
            focused_pane: auxiliary_pane.clone(),
            root: taide_model::layout::PaneNode::Leaf {
                id: auxiliary_pane.clone(),
                tabs: Vec::new(),
                active: None,
            },
        });
    let auxiliary = taide_native_app::explorer::open_to_side_request(
        &project,
        &page.rows[0],
        Some(&layout),
        &WindowScope::Auxiliary {
            project: project.clone(),
            slot: 1,
        },
    )
    .unwrap();
    assert_eq!(auxiliary.target_pane, auxiliary_pane);
    assert_ne!(auxiliary.target_pane, layout.focused_pane);
    assert!(
        taide_native_app::explorer::open_to_side_request(
            &project,
            &page.rows[0],
            Some(&layout),
            &WindowScope::Auxiliary {
                project: ProjectId::new(),
                slot: 1
            },
        )
        .is_none()
    );
    let frame = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = frame.rows[&page.rows[1].path].rect.center();
    menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        position,
        PointerButton::Secondary,
    );
    let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(!menu.menu.contains_key("explorer.openToTheSide"));
}

#[test]
fn open_with_하위메뉴는_preview_파일에서만_editor_preview_선택을_반환한다() {
    use taide_native_app::open_with::Mode;
    let project = ProjectId::new();
    let mut page = selection_page();
    page.rows[0].name = "합성.PNG".into();
    page.rows[0].path = "/root/합성.PNG".into();
    for (key, mode) in [
        ("explorer.openWithEditor", Mode::Editor),
        ("explorer.openWithPreview", Mode::Preview),
    ] {
        let context = Context::default();
        let mut explorer = Explorer::default();
        let frame = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            frame.rows[&page.rows[0].path].rect.center(),
            PointerButton::Secondary,
        );
        let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            menu.menu["explorer.openWith"].rect.center(),
            PointerButton::Primary,
        );
        let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        let output = menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            menu.menu[key].rect.center(),
            PointerButton::Primary,
        );
        assert_eq!(
            output.actions,
            vec![Action::OpenWith {
                row: page.rows[0].clone(),
                mode
            }]
        );
    }
    for (name, kind) in [
        ("note.md", TreeEntryKind::File),
        (".png", TreeEntryKind::File),
        ("folder.png", TreeEntryKind::Directory),
    ] {
        page.rows[0].name = name.into();
        page.rows[0].kind = kind;
        let context = Context::default();
        let mut explorer = Explorer::default();
        let frame = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            frame.rows[&page.rows[0].path].rect.center(),
            PointerButton::Secondary,
        );
        let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        assert!(!menu.menu.contains_key("explorer.openWith"));
    }
}

#[test]
fn 내부클립보드는_실제event_메뉴_single_primary와_동일위치cut을_보존한다() {
    use taide_native_app::explorer_clipboard::Mode;
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = selection_page();
    page.rows[3].kind = TreeEntryKind::Directory;
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE);
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        1,
        Modifiers::COMMAND,
    );
    let command = if cfg!(target_os = "macos") {
        Modifiers::COMMAND | Modifiers::MAC_CMD
    } else {
        Modifiers::COMMAND | Modifiers::CTRL
    };
    let output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::ModifiersChanged(command), Event::Copy],
    );
    assert!(output.actions.is_empty());
    assert_eq!(explorer.clipboard().unwrap().path, page.rows[1].path);
    assert_eq!(explorer.clipboard().unwrap().mode, Mode::Copy);
    explorer.selected = Some(page.rows[3].path.clone());
    let output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::Paste(
            "unrelated OS text must not become a path".into(),
        )],
    );
    let [Action::Paste(request)] = output.actions.as_slice() else {
        panic!("expected internal paste");
    };
    assert_eq!(request.entry.path, page.rows[1].path);
    assert_eq!(request.target, page.rows[3].path);
    assert!(request.sibling_names.is_empty());
    assert_eq!(request.conflict_suffix, "copy");
    explorer.selected = Some(page.rows[0].path.clone());
    let output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::Cut, Event::Paste("ignored".into())],
    );
    assert!(output.actions.is_empty());
    assert!(explorer.clipboard().is_none());
    let first = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = first.rows[&page.rows[1].path].rect.center();
    menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        position,
        PointerButton::Secondary,
    );
    let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(!menu.menu["explorer.paste"].enabled());
    let copy = menu.menu["explorer.copy"].rect.center();
    assert!(
        menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            copy,
            PointerButton::Primary
        )
        .actions
        .is_empty()
    );
    assert_eq!(explorer.clipboard().unwrap().path, page.rows[1].path);
    menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    let blank = menu_frame(&context, &mut explorer, &project, &page, Vec::new())
        .blank
        .unwrap()
        .rect
        .center();
    menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        blank,
        PointerButton::Secondary,
    );
    let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(!menu.menu.contains_key("explorer.copy"));
    assert!(menu.menu["explorer.paste"].enabled());
    let paste = menu.menu["explorer.paste"].rect.center();
    let output = menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        paste,
        PointerButton::Primary,
    );
    let [Action::Paste(request)] = output.actions.as_slice() else {
        panic!("expected root paste");
    };
    assert_eq!(request.target, "/synthetic");
    assert!(request.sibling_names.contains("button.tsx"));
    assert_eq!(explorer.clipboard().unwrap().mode, Mode::Copy);
    explorer.paste_finished(false, Some("/synthetic/button copy.tsx".into()));
    assert_eq!(
        explorer.selected.as_deref(),
        Some("/synthetic/button copy.tsx")
    );
    assert!(explorer.clipboard().is_some());
    explorer.paste_finished(true, None);
    assert!(explorer.clipboard().is_none());
}

#[test]
fn 복사단축키_실제adapter_event는_alt_modifier와_순서를_보존한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = selection_page();
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE);
    let command = if cfg!(target_os = "macos") {
        Modifiers::MAC_CMD | Modifiers::COMMAND
    } else {
        Modifiers::COMMAND | Modifiers::CTRL
    };
    let output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::ModifiersChanged(command | Modifiers::ALT),
            Event::Copy,
            Event::ModifiersChanged(command | Modifiers::ALT | Modifiers::SHIFT),
            Event::Copy,
            Event::ModifiersChanged(Modifiers::NONE),
        ],
    );
    assert_eq!(
        output.actions,
        vec![
            Action::CopyText(page.rows[0].path.clone()),
            Action::CopyText("App.tsx".into())
        ]
    );
    let mut rejected_modifiers = vec![command];
    if cfg!(target_os = "macos") {
        rejected_modifiers.push(command | Modifiers::CTRL | Modifiers::ALT);
    }
    for modifiers in rejected_modifiers {
        let output = frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![Event::ModifiersChanged(modifiers), Event::Copy],
        );
        assert!(
            !output
                .actions
                .iter()
                .any(|action| matches!(action, Action::CopyText(_)))
        );
    }
    #[cfg(target_os = "macos")]
    {
        let output = frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![modified_key(Key::ArrowDown, command | Modifiers::CTRL)],
        );
        assert!(output.actions.is_empty());
        assert_selected(&explorer, &page, &[1]);
    }
    context.memory_mut(|memory| {
        memory.surrender_focus(egui::Id::new(("native-tree-focus", &project)))
    });
    let output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::ModifiersChanged(command | Modifiers::ALT),
            Event::Copy,
        ],
    );
    assert!(output.actions.is_empty());
}

#[test]
fn 탐색기_경로메뉴는_복사_relative_exact_shortcut과_html_표시를_구분한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = selection_page();
    page.rows[0].name = "page.HTML".into();
    page.rows[0].path = "/synthetic/nested/文 page.HTML".into();
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic/".into());
    let first = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = first.rows[&page.rows[0].path].rect.center();
    for (key_value, expected) in [
        (
            "explorer.copyPath",
            Action::CopyText(page.rows[0].path.clone()),
        ),
        (
            "explorer.copyRelativePath",
            Action::CopyText("nested/文 page.HTML".into()),
        ),
        (
            "explorer.revealInFinder",
            Action::RevealPath(page.rows[0].path.clone()),
        ),
        (
            "explorer.openInBrowser",
            Action::OpenInBrowser(page.rows[0].path.clone()),
        ),
    ] {
        menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            position,
            PointerButton::Secondary,
        );
        let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        assert!(
            menu.menu.contains_key("explorer.openInBrowser"),
            "menu for {key_value}: {:?}",
            menu.menu.keys().collect::<Vec<_>>()
        );
        let action = menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            menu.menu[key_value].rect.center(),
            PointerButton::Primary,
        );
        assert_eq!(action.actions, vec![expected]);
        menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    }
    for (key_value, modifiers, expected) in [
        (
            Key::C,
            Modifiers::COMMAND | Modifiers::ALT,
            Action::CopyText(page.rows[0].path.clone()),
        ),
        (
            Key::C,
            Modifiers::COMMAND | Modifiers::ALT | Modifiers::SHIFT,
            Action::CopyText("nested/文 page.HTML".into()),
        ),
        (
            Key::R,
            Modifiers::COMMAND | Modifiers::ALT,
            Action::RevealPath(page.rows[0].path.clone()),
        ),
    ] {
        context.memory_mut(|memory| {
            memory.request_focus(egui::Id::new(("native-tree-focus", &project)))
        });
        let output = menu_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![modified_key(key_value, modifiers)],
        );
        assert_eq!(output.actions, vec![expected]);
    }
    assert!(
        menu_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![modified_key(
                Key::R,
                Modifiers::COMMAND | Modifiers::ALT | Modifiers::SHIFT
            )]
        )
        .actions
        .is_empty()
    );
    explorer.selected = Some("/synthetic-other/file.rs".into());
    page.rows[1].path = explorer.selected.clone().unwrap();
    let output = menu_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![modified_key(
            Key::C,
            Modifiers::COMMAND | Modifiers::ALT | Modifiers::SHIFT,
        )],
    );
    assert_eq!(
        output.actions,
        vec![Action::CopyText(page.rows[1].path.clone())]
    );
    explorer.selected = None;
    assert!(
        menu_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![modified_key(Key::C, Modifiers::COMMAND | Modifiers::ALT)]
        )
        .actions
        .is_empty()
    );
    for (name, kind, expected) in [
        ("normal.htm", TreeEntryKind::File, true),
        (".html", TreeEntryKind::File, false),
        ("plain.txt", TreeEntryKind::File, false),
        ("folder.html", TreeEntryKind::Directory, false),
    ] {
        page.rows[0].name = name.into();
        page.rows[0].kind = kind;
        menu_click(
            &context,
            &mut explorer,
            &project,
            &page,
            position,
            PointerButton::Secondary,
        );
        let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
        assert_eq!(menu.menu.contains_key("explorer.openInBrowser"), expected);
        menu_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::Escape)],
        );
        menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    }
    let blank = menu_frame(&context, &mut explorer, &project, &page, Vec::new())
        .blank
        .unwrap();
    menu_click(
        &context,
        &mut explorer,
        &project,
        &page,
        blank.rect.center(),
        PointerButton::Secondary,
    );
    let menu = menu_frame(&context, &mut explorer, &project, &page, Vec::new());
    for key_value in [
        "explorer.copyPath",
        "explorer.copyRelativePath",
        "explorer.revealInFinder",
        "explorer.openInBrowser",
    ] {
        assert!(!menu.menu.contains_key(key_value));
    }
}

fn selection_page() -> TreeRowPage {
    let rows = ["App.tsx", "button.tsx", "card.tsx", "Cn.ts"]
        .map(|name| TreeRow {
            path: format!("/synthetic/{name}"),
            name: name.into(),
            kind: TreeEntryKind::File,
            depth: 0,
            expanded: false,
            has_children: false,
        })
        .to_vec();
    TreeRowPage {
        total: u32::try_from(rows.len()).unwrap(),
        rows,
    }
}

fn selection_click(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    index: usize,
    modifiers: Modifiers,
) -> Output {
    let first = frame(context, explorer, project, page, Vec::new());
    let position = first.rows[&page.rows[index].path].rect.center();
    let mut result = None;
    for pressed in [true, false] {
        result = Some(frame(
            context,
            explorer,
            project,
            page,
            vec![
                Event::ModifiersChanged(modifiers),
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers,
                },
            ],
        ));
    }
    result.unwrap()
}

#[track_caller]
fn assert_selected(explorer: &Explorer, page: &TreeRowPage, indices: &[usize]) {
    assert_eq!(
        explorer.selected_paths(),
        &indices
            .iter()
            .map(|index| page.rows[*index].path.clone())
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn 탐색기_선택검색_포인터는_anchor_범위_추가_해제와_숨긴행을_보존한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = selection_page();
    let mut explorer = Explorer::default();
    assert_eq!(
        selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE).actions,
        vec![Action::Open {
            path: page.rows[0].path.clone(),
            preview: false,
        }]
    );
    for (index, modifiers, expected, primary) in [
        (2, Modifiers::COMMAND, vec![0, 2], Some(2)),
        (3, Modifiers::CTRL, vec![0, 2, 3], Some(3)),
        (3, Modifiers::COMMAND, vec![0, 2], Some(2)),
        (1, Modifiers::SHIFT, vec![1, 2, 3], Some(1)),
        (
            0,
            Modifiers::COMMAND | Modifiers::SHIFT,
            vec![0, 1, 2, 3],
            Some(0),
        ),
        (2, Modifiers::SHIFT, vec![2, 3], Some(2)),
        (2, Modifiers::CTRL, vec![3], Some(3)),
        (3, Modifiers::CTRL, vec![], None),
        (1, Modifiers::SHIFT, vec![1, 2, 3], Some(1)),
    ] {
        assert!(
            selection_click(&context, &mut explorer, &project, &page, index, modifiers)
                .actions
                .is_empty()
        );
        assert_selected(&explorer, &page, &expected);
        assert_eq!(
            explorer.selected,
            primary.map(|index| page.rows[index].path.clone())
        );
    }
    page.rows
        .retain(|row| row.name == "App.tsx" || row.name == "card.tsx");
    page.total = u32::try_from(page.rows.len()).unwrap();
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        0,
        Modifiers::SHIFT,
    );
    assert_selected(&explorer, &page, &[0]);
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        1,
        Modifiers::COMMAND,
    );
    let output = frame(&context, &mut explorer, &project, &page, Vec::new());
    pointer_click(
        &context,
        &mut explorer,
        &project,
        &page,
        output.rows[&page.rows[1].path].rect.center(),
        PointerButton::Secondary,
    );
    assert_selected(&explorer, &page, &[1]);
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Escape)],
    );
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        0,
        Modifiers::COMMAND,
    );
    let output = frame(&context, &mut explorer, &project, &page, Vec::new());
    pointer_click(
        &context,
        &mut explorer,
        &project,
        &page,
        output.blank.unwrap().rect.center(),
        PointerButton::Secondary,
    );
    assert_selected(&explorer, &page, &[]);
    assert_eq!(explorer.selected, None);
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Escape)],
    );
    selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE);
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        1,
        Modifiers::COMMAND,
    );
    explorer.moved(
        std::path::Path::new("/synthetic"),
        std::path::Path::new("/retarget"),
    );
    for row in &mut page.rows {
        row.path = format!("/retarget/{}", row.name);
    }
    assert_selected(&explorer, &page, &[0, 1]);
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        0,
        Modifiers::SHIFT,
    );
    assert_selected(&explorer, &page, &[0, 1]);
    explorer.start_rename(&page.rows[0]);
    assert_selected(&explorer, &page, &[0]);
    explorer.cancel_rename();
    explorer.selected = None;
    frame(&context, &mut explorer, &project, &page, Vec::new());
    assert_selected(&explorer, &page, &[]);
}

fn modified_key(key_value: Key, modifiers_value: Modifiers) -> Event {
    Event::Key {
        key: key_value,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: modifiers_value,
    }
}

#[test]
fn 탐색기_선택검색_방향키는_수정키_단일선택과_목록경계를_구분한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = selection_page();
    let mut explorer = Explorer::default();
    selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE);
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        3,
        Modifiers::COMMAND,
    );
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::ArrowDown)]
        )
        .actions
        .is_empty()
    );
    assert_selected(&explorer, &page, &[0, 3]);
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![modified_key(Key::ArrowUp, Modifiers::SHIFT)],
    );
    assert_selected(&explorer, &page, &[2]);
    selection_click(
        &context,
        &mut explorer,
        &project,
        &page,
        0,
        Modifiers::COMMAND,
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::ArrowUp)],
    );
    assert_selected(&explorer, &page, &[0, 2]);
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![modified_key(Key::ArrowDown, Modifiers::CTRL)],
    );
    assert_selected(&explorer, &page, &[1]);
    assert_eq!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![modified_key(Key::ArrowDown, Modifiers::COMMAND)]
        )
        .actions,
        vec![Action::Open {
            path: page.rows[1].path.clone(),
            preview: false
        }]
    );
    assert_selected(&explorer, &page, &[1]);
    explorer.selected = None;
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::ArrowUp)],
    );
    assert_selected(&explorer, &page, &[3]);
    page.rows[0].kind = TreeEntryKind::Directory;
    page.rows[0].expanded = true;
    page.rows[0].has_children = true;
    page.rows[1].depth = 1;
    explorer.selected = Some(page.rows[0].path.clone());
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![modified_key(Key::ArrowRight, Modifiers::ALT)],
    );
    assert_selected(&explorer, &page, &[1]);
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![modified_key(Key::ArrowLeft, Modifiers::SHIFT)],
    );
    assert_selected(&explorer, &page, &[0]);
    assert_eq!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::ArrowLeft)]
        )
        .actions,
        vec![Action::Toggle(page.rows[0].path.clone())]
    );
    page.rows[0].expanded = false;
    assert_eq!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::ArrowRight)]
        )
        .actions,
        vec![Action::Toggle(page.rows[0].path.clone())]
    );
}

fn timed_frame(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    offset: f64,
    events: Vec<Event>,
) -> Output {
    let mut result = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, HEIGHT))),
            time: Some(SEARCH_START_SECONDS + offset),
            events,
            ..Default::default()
        },
        |ui| {
            result = Some(explorer.show(ui, project, page, &locale()));
        },
    );
    output.textures_delta.clear();
    result.unwrap()
}

#[test]
fn 탐색기_동일frame_키입력은_반복_문자와_shortcut의_순서를_보존한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = selection_page();
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        0.0,
        vec![key(Key::ArrowDown), key(Key::ArrowDown)],
    );
    assert_selected(&explorer, &page, &[2]);
    assert_eq!(
        timed_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            1.0,
            vec![
                modified_key(Key::ArrowDown, Modifiers::CTRL),
                modified_key(Key::ArrowDown, Modifiers::COMMAND)
            ],
        )
        .actions,
        vec![Action::Open {
            path: page.rows[3].path.clone(),
            preview: false
        }]
    );
    assert_selected(&explorer, &page, &[3]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        2.0,
        vec![
            key(Key::B),
            Event::Text("B".into()),
            key(Key::U),
            Event::Text("u".into()),
        ],
    );
    assert_selected(&explorer, &page, &[1]);
    explorer.selected = Some(page.rows[0].path.clone());
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        3.0,
        vec![
            key(Key::C),
            Event::Text("c".into()),
            modified_key(Key::ArrowDown, Modifiers::CTRL),
        ],
    );
    assert_selected(&explorer, &page, &[3]);
    assert_eq!(
        timed_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            4.0,
            vec![
                key(Key::C),
                Event::Text("c".into()),
                key(Key::Space),
                Event::Text(" ".into()),
                key(Key::A),
                Event::Text("a".into())
            ],
        )
        .actions,
        vec![Action::Open {
            path: page.rows[3].path.clone(),
            preview: false
        }]
    );
    assert_selected(&explorer, &page, &[2]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        5.0,
        vec![key(Key::ArrowUp), key(Key::Enter), key(Key::ArrowDown)],
    );
    assert_selected(&explorer, &page, &[1]);
    assert_eq!(
        explorer.rename.as_ref().unwrap().row.path,
        page.rows[1].path
    );
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        6.0,
        vec![key(Key::Escape)],
    );
    assert!(explorer.rename.is_none());
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        7.0,
        vec![
            key(Key::ArrowDown),
            modified_key(Key::N, Modifiers::COMMAND),
            key(Key::ArrowUp),
        ],
    );
    assert_selected(&explorer, &page, &[2]);
    assert_eq!(explorer.create.as_ref().unwrap().parent, "/synthetic");
}

#[test]
fn 탐색기_선택검색_타이핑은_대소문자_순환_시간과_ime_focus를_구분한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = selection_page();
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    selection_click(&context, &mut explorer, &project, &page, 1, Modifiers::NONE);
    for (offset, text, expected) in [
        (0.0, "C", 2),
        (0.1, "N", 3),
        (1.0, "a", 0),
        (1.1, "z", 0),
        (1.2, "c", 0),
        (2.0, "c", 2),
        (2.1, "a", 2),
        (3.0, "c", 2),
    ] {
        timed_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            offset,
            vec![
                Event::ModifiersChanged(Modifiers::SHIFT),
                Event::Text(text.into()),
            ],
        );
        assert_selected(&explorer, &page, &[expected]);
    }
    for (offset, modifiers) in [
        (4.0, Modifiers::ALT),
        (4.1, Modifiers::CTRL),
        (4.2, Modifiers::COMMAND),
    ] {
        timed_frame(
            &context,
            &mut explorer,
            &project,
            &page,
            offset,
            vec![Event::ModifiersChanged(modifiers), Event::Text("b".into())],
        );
        assert_selected(&explorer, &page, &[2]);
    }
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        4.3,
        vec![
            Event::ModifiersChanged(Modifiers::NONE),
            key(Key::B),
            Event::Text("b".into()),
        ],
    );
    assert_selected(&explorer, &page, &[1]);
    context.memory_mut(|memory| {
        memory.surrender_focus(egui::Id::new(("native-tree-focus", &project)))
    });
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        5.0,
        vec![Event::Text("a".into())],
    );
    assert_selected(&explorer, &page, &[1]);
    context
        .memory_mut(|memory| memory.request_focus(egui::Id::new(("native-tree-focus", &project))));
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        5.1,
        vec![Event::Text("c".into())],
    );
    assert_selected(&explorer, &page, &[2]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        6.0,
        vec![
            Event::Paste("App".into()),
            Event::Text("\u{1f600}".into()),
            Event::Text("App".into()),
        ],
    );
    assert_selected(&explorer, &page, &[2]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        6.1,
        vec![Event::Text("b".into())],
    );
    assert_selected(&explorer, &page, &[1]);
    let blocked = timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        7.0,
        vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: None,
            }),
            key(Key::ArrowDown),
            modified_key(Key::N, Modifiers::COMMAND),
            Event::Text("a".into()),
        ],
    );
    assert!(blocked.actions.is_empty());
    assert!(explorer.create.is_none());
    assert_selected(&explorer, &page, &[1]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        7.1,
        vec![key(Key::ArrowDown), Event::Text("a".into())],
    );
    assert_selected(&explorer, &page, &[1]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        7.2,
        vec![
            Event::Ime(egui::ImeEvent::Commit("한".into())),
            Event::Text("a".into()),
        ],
    );
    assert_selected(&explorer, &page, &[1]);
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        7.3,
        vec![Event::Text("a".into())],
    );
    assert_selected(&explorer, &page, &[0]);
    explorer.selected = None;
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        8.0,
        vec![Event::Text("b".into())],
    );
    assert_selected(&explorer, &page, &[]);
    explorer.selected = Some(page.rows[0].path.clone());
    timed_frame(
        &context,
        &mut explorer,
        &project,
        &page,
        8.1,
        vec![Event::Text("c".into())],
    );
    assert_selected(&explorer, &page, &[2]);
}

#[test]
fn 탐색기_키보드_선택은_가시범위를_벗어날_때만_최소로_스크롤한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = TreeRowPage {
        total: u32::try_from(VIRTUAL_ROWS).unwrap(),
        rows: (0..VIRTUAL_ROWS)
            .map(|index| TreeRow {
                path: format!("/synthetic/file-{index}.txt"),
                name: format!("file-{index}.txt"),
                kind: TreeEntryKind::File,
                depth: 0,
                expanded: false,
                has_children: false,
            })
            .collect(),
    };
    let mut explorer = Explorer::default();
    selection_click(&context, &mut explorer, &project, &page, 0, Modifiers::NONE);
    let row_top = |output: &Output, index: usize| {
        output
            .rows
            .get(&page.rows[index].path)
            .map(|response| response.rect.top())
    };
    let is_near = |actual: Option<f32>, expected: f32| {
        actual.is_some_and(|actual| (actual - expected).abs() < SCROLL_EPSILON)
    };
    let list_top = row_top(
        &frame(&context, &mut explorer, &project, &page, Vec::new()),
        0,
    )
    .unwrap();
    assert!(
        is_near(Some(list_top), SOURCE_HEADER_HEIGHT),
        "list_top={list_top}"
    );
    let mut output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::ArrowDown)],
    );
    assert_selected(&explorer, &page, &[1]);
    assert!(is_near(row_top(&output, 0), list_top));
    assert!(is_near(row_top(&output, 1), list_top + SOURCE_ROW_HEIGHT));
    for _ in 1..SCROLL_OVERFLOW_ROW {
        output = frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::ArrowDown)],
        );
    }
    assert_selected(&explorer, &page, &[SCROLL_OVERFLOW_ROW]);
    let end_aligned = row_top(&output, SCROLL_OVERFLOW_ROW).unwrap();
    assert!(end_aligned > list_top + SOURCE_ROW_HEIGHT);
    assert!(
        is_near(Some(end_aligned + SOURCE_ROW_HEIGHT), HEIGHT),
        "list_top={list_top}, end_aligned={end_aligned}"
    );
    output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::ArrowDown)],
    );
    assert!(is_near(
        row_top(&output, SCROLL_OVERFLOW_ROW + 1),
        end_aligned
    ));
    output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::ArrowUp)],
    );
    assert_selected(&explorer, &page, &[SCROLL_OVERFLOW_ROW]);
    assert!(is_near(
        row_top(&output, SCROLL_OVERFLOW_ROW + 1),
        end_aligned
    ));
    assert!(is_near(
        row_top(&output, SCROLL_OVERFLOW_ROW),
        end_aligned - SOURCE_ROW_HEIGHT
    ));
    for _ in SCROLL_START_ROW..SCROLL_OVERFLOW_ROW {
        output = frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::ArrowUp)],
        );
    }
    assert_selected(&explorer, &page, &[SCROLL_START_ROW]);
    assert!(is_near(row_top(&output, SCROLL_START_ROW), list_top));
}

#[test]
fn 탐색기_alt_click은_파일을_열고_selection_modifier는_열지않는다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = page();
    let mut explorer = Explorer::default();
    let first = frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = first.rows[&page.rows[0].path].rect.center();
    for modifiers in [
        Modifiers::ALT,
        Modifiers::SHIFT,
        Modifiers::CTRL,
        Modifiers::COMMAND,
    ] {
        let mut result = None;
        for pressed in [true, false] {
            let mut output = context.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, HEIGHT))),
                    events: vec![
                        Event::ModifiersChanged(modifiers),
                        Event::PointerMoved(position),
                        Event::PointerButton {
                            pos: position,
                            button: PointerButton::Primary,
                            pressed,
                            modifiers,
                        },
                    ],
                    ..Default::default()
                },
                |ui| {
                    result = Some(explorer.show(ui, &project, &page, &locale()));
                },
            );
            output.textures_delta.clear();
        }
        let expected = if modifiers == Modifiers::ALT {
            vec![Action::Open {
                path: page.rows[0].path.clone(),
                preview: false,
            }]
        } else {
            Vec::new()
        };
        assert_eq!(result.unwrap().actions, expected);
    }
}

fn pointer_click(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    position: egui::Pos2,
    button: PointerButton,
) -> Output {
    frame(
        context,
        explorer,
        project,
        page,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        context,
        explorer,
        project,
        page,
        vec![Event::PointerButton {
            pos: position,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    )
}

#[test]
fn 탐색기_전체행과_빈영역은_선택_루트메뉴와_double_click을_구분한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = page();
    page.rows[0].depth = 2;
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    let first = frame(&context, &mut explorer, &project, &page, Vec::new());
    let row = &first.rows[&page.rows[0].path];
    assert_eq!(row.rect.height(), SOURCE_ROW_HEIGHT);
    assert_eq!(
        first.rows[&page.rows[1].path].rect.top() - row.rect.top(),
        SOURCE_ROW_HEIGHT
    );
    assert!(row.rect.width() > WIDTH - SOURCE_TOOLBAR_BUTTON_SIZE);
    let left = pos2(row.rect.left() + ROW_EDGE_OFFSET, row.rect.center().y);
    assert_eq!(
        pointer_click(
            &context,
            &mut explorer,
            &project,
            &page,
            left,
            PointerButton::Primary
        )
        .actions,
        vec![Action::Open {
            path: page.rows[0].path.clone(),
            preview: false
        }]
    );
    let right = pos2(row.rect.right() - ROW_EDGE_OFFSET, row.rect.center().y);
    assert_eq!(
        pointer_click(
            &context,
            &mut explorer,
            &project,
            &page,
            right,
            PointerButton::Primary
        )
        .actions,
        vec![Action::Open {
            path: page.rows[0].path.clone(),
            preview: false
        }]
    );
    let blank = first.blank.unwrap();
    assert!(blank.rect.top() >= first.rows[&page.rows[1].path].rect.bottom());
    pointer_click(
        &context,
        &mut explorer,
        &project,
        &page,
        blank.rect.center(),
        PointerButton::Secondary,
    );
    assert_eq!(explorer.selected, None);
    let menu = frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(!menu.menu.contains_key("explorer.delete"));
    let position = menu.menu["explorer.newFolder"].rect.center();
    pointer_click(
        &context,
        &mut explorer,
        &project,
        &page,
        position,
        PointerButton::Primary,
    );
    assert_eq!(explorer.create.as_ref().unwrap().parent, "/synthetic");
    assert_eq!(
        explorer.create.as_ref().unwrap().input.row.kind,
        TreeEntryKind::Directory
    );
    frame(&context, &mut explorer, &project, &page, Vec::new());
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Escape)],
    );
    assert!(explorer.create.is_none());
    let mut reset = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, HEIGHT))),
            time: Some(context.input(|input| input.time) + CLICK_SEQUENCE_RESET_SECONDS),
            ..Default::default()
        },
        |ui| {
            explorer.show(ui, &project, &page, &locale());
        },
    );
    reset.textures_delta.clear();
    let blank = frame(&context, &mut explorer, &project, &page, Vec::new())
        .blank
        .unwrap();
    pointer_click(
        &context,
        &mut explorer,
        &project,
        &page,
        blank.rect.center(),
        PointerButton::Primary,
    );
    assert!(explorer.create.is_none());
    pointer_click(
        &context,
        &mut explorer,
        &project,
        &page,
        blank.rect.center(),
        PointerButton::Primary,
    );
    assert_eq!(explorer.create.as_ref().unwrap().parent, "/synthetic");
    assert_eq!(
        explorer.create.as_ref().unwrap().input.row.kind,
        TreeEntryKind::File
    );
}

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: [
            ("explorer.entryNamePlaceholder", "Entry name"),
            ("explorer.rename", "Rename"),
            ("explorer.entryNameReserved", "Reserved"),
            ("explorer.entryNameInvalidChar", "Invalid"),
            ("explorer.entryNameDuplicate", "Duplicate"),
            ("explorer.title", "Explorer"),
            ("explorer.newFile", "New File"),
            ("explorer.newFolder", "New Folder"),
            ("explorer.refresh", "Refresh"),
            ("explorer.collapseAll", "Collapse All"),
            ("explorer.pasteConflictSuffix", "copy"),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect(),
    }
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn frame(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    events: Vec<Event>,
) -> Output {
    frame_with_width(context, explorer, project, page, events, WIDTH)
}

fn frame_with_width(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    events: Vec<Event>,
    width: f32,
) -> Output {
    let mut result = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, HEIGHT))),
            events,
            ..Default::default()
        },
        |ui| {
            result = Some(explorer.show(ui, project, page, &locale()));
        },
    );
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
    result.unwrap()
}

fn page() -> TreeRowPage {
    TreeRowPage {
        total: 2,
        rows: ["old.txt", "duplicate.txt"]
            .map(|name| TreeRow {
                path: format!("/synthetic/{name}"),
                name: name.into(),
                kind: TreeEntryKind::File,
                depth: 0,
                expanded: false,
                has_children: false,
            })
            .to_vec(),
    }
}

#[test]
fn 탐색기_이름입력은_선택_키보드_검증_중복확정_취소와_ime를_보존한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = page();
    let mut explorer = Explorer::default();
    let initial = frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = initial.rows[&page.rows[0].path].rect.center();
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    let clicked = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(
        clicked.actions,
        vec![Action::Open {
            path: page.rows[0].path.clone(),
            preview: false
        }]
    );
    assert_eq!(explorer.selected.as_ref(), Some(&page.rows[0].path));
    let editing = frame(&context, &mut explorer, &project, &page, vec![key(Key::F2)]);
    let input = editing.input.unwrap();
    assert!(input.has_focus());
    let cursor = egui::TextEdit::load_state(&context, input.id)
        .unwrap()
        .cursor
        .char_range()
        .unwrap();
    assert_eq!(cursor.sorted_cursors()[0].index, egui::text::CharIndex(0));
    assert_eq!(
        cursor.sorted_cursors()[1].index,
        egui::text::CharIndex("old.txt".chars().count())
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::Text("duplicate.txt".into())],
    );
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::Enter)]
        )
        .actions
        .is_empty()
    );
    assert_eq!(
        explorer.rename.as_ref().unwrap().error.as_deref(),
        Some("Duplicate")
    );
    for (name, error) in [
        (".", "Reserved"),
        ("foo.", "Invalid"),
        ("foo:bar", "Invalid"),
    ] {
        explorer.rename.as_mut().unwrap().name = name.into();
        assert!(explorer.commit_rename(&page.rows, &locale()).is_none());
        assert_eq!(
            explorer.rename.as_ref().unwrap().error.as_deref(),
            Some(error)
        );
    }
    explorer.rename.as_mut().unwrap().name = "  nested/文.txt  ".into();
    let request = explorer.commit_rename(&page.rows, &locale()).unwrap();
    assert_eq!(request.to, "/synthetic/nested/文.txt");
    assert!(explorer.commit_rename(&page.rows, &locale()).is_none());
    explorer.rename_finished(&request, Err("disk collision".into()));
    assert_eq!(
        explorer.rename.as_ref().unwrap().error.as_deref(),
        Some("disk collision")
    );
    frame(&context, &mut explorer, &project, &page, Vec::new());
    let composing = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: None,
            }),
            key(Key::Enter),
        ],
    );
    assert!(composing.actions.is_empty());
    assert!(explorer.rename.is_some());
    let committed = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::Ime(egui::ImeEvent::Commit("한".into())),
            key(Key::Enter),
        ],
    );
    assert!(committed.actions.is_empty());
    assert!(explorer.rename.is_some());
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Escape)],
    );
    assert!(explorer.rename.is_none());
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Enter)],
    );
    assert!(explorer.rename.is_some());
    frame(&context, &mut explorer, &project, &page, Vec::new());
    context.memory_mut(|memory| memory.request_focus(egui::Id::new("outside-explorer")));
    frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(explorer.rename.is_none());
    explorer.start_rename(&page.rows[0]);
    explorer.rename.as_mut().unwrap().name = " ".into();
    assert!(explorer.commit_rename(&page.rows, &locale()).is_none());
    assert!(explorer.rename.is_none());
}

#[test]
fn 긴_header는_toolbar를_밀지_않고_root_입력은_스크롤_아래에서도_보인다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    explorer.title = Some("very long synthetic project title exceeding the sidebar width".into());
    let page = page();
    let output = frame_with_width(
        &context,
        &mut explorer,
        &project,
        &page,
        Vec::new(),
        NARROW_WIDTH,
    );
    let baseline = output.toolbar["explorer.newFile"].rect;
    for button in output.toolbar.values() {
        assert_eq!(
            button.rect.size(),
            vec2(SOURCE_TOOLBAR_BUTTON_SIZE, SOURCE_TOOLBAR_BUTTON_SIZE)
        );
        assert_eq!(button.rect.min.y, baseline.min.y);
        assert!(button.rect.max.x <= NARROW_WIDTH);
    }
    assert_eq!(
        output.rows["/synthetic/duplicate.txt"].rect.center().y
            - output.rows["/synthetic/old.txt"].rect.center().y,
        SOURCE_ROW_HEIGHT
    );
    let large = TreeRowPage {
        total: u32::try_from(VIRTUAL_ROWS).unwrap(),
        rows: (0..VIRTUAL_ROWS)
            .map(|index| TreeRow {
                path: format!("/synthetic/file-{index}.txt"),
                name: format!("file-{index}.txt"),
                kind: TreeEntryKind::File,
                depth: 0,
                expanded: false,
                has_children: false,
            })
            .collect(),
    };
    explorer.start_rename(large.rows.last().unwrap());
    assert!(
        frame(&context, &mut explorer, &project, &large, Vec::new())
            .input
            .is_some()
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &large,
        vec![key(Key::Escape)],
    );
    assert!(explorer.rename.is_none());
    explorer.start_create(TreeEntryKind::File, &large);
    assert!(
        frame(&context, &mut explorer, &project, &large, Vec::new())
            .input
            .unwrap()
            .has_focus()
    );
    explorer.create = None;
    context
        .memory_mut(|memory| memory.request_focus(egui::Id::new(("native-tree-focus", &project))));
    let mut modified_new = key(Key::N);
    if let Event::Key { modifiers, .. } = &mut modified_new {
        *modifiers = Modifiers::COMMAND | Modifiers::ALT;
    }
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &large,
            vec![modified_new]
        )
        .actions
        .is_empty()
    );
    assert!(explorer.create.is_none());
}

#[test]
fn 탐색기_열기는_원본_container의_클릭_space_cmd_down_설정을_따른다() {
    let context = Context::default();
    let project = ProjectId::new();
    let page = page();
    let mut explorer = Explorer::default();
    let output = frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = output.rows[&page.rows[0].path].rect.center();
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    let clicked = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let expected = vec![Action::Open {
        path: page.rows[0].path.clone(),
        preview: false,
    }];
    assert_eq!(clicked.actions, expected);
    assert_eq!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![key(Key::Space)]
        )
        .actions,
        expected
    );
    let mut command_down = key(Key::ArrowDown);
    if let Event::Key { modifiers, .. } = &mut command_down {
        *modifiers = Modifiers::COMMAND;
    }
    assert_eq!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![command_down.clone()]
        )
        .actions,
        expected
    );
    let mut folders = page.clone();
    folders.rows[0].kind = TreeEntryKind::Directory;
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &folders,
            vec![key(Key::Space)]
        )
        .actions
        .is_empty()
    );
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &folders,
            vec![command_down]
        )
        .actions
        .is_empty()
    );
}

#[test]
fn 우클릭_생성은_해당_폴더를_선택하고_toolbar도_새_id로_작동한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = page();
    page.rows[0] = TreeRow {
        path: "/synthetic/target".into(),
        name: "target".into(),
        kind: TreeEntryKind::Directory,
        depth: 0,
        expanded: true,
        has_children: false,
    };
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    explorer.selected = Some(page.rows[1].path.clone());
    let initial = frame(&context, &mut explorer, &project, &page, Vec::new());
    let position = initial.rows[&page.rows[0].path].rect.center();
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Secondary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Secondary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let menu = frame(&context, &mut explorer, &project, &page, Vec::new());
    assert_eq!(explorer.selected.as_ref(), Some(&page.rows[0].path));
    let position = menu.menu["explorer.newFile"].rect.center();
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    );
    let input = frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(input.input.unwrap().has_focus());
    assert_eq!(
        explorer.create.as_ref().unwrap().parent,
        "/synthetic/target"
    );
    assert_eq!(
        explorer.create.as_ref().unwrap().input.row.kind,
        TreeEntryKind::File
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Escape)],
    );
    assert!(explorer.create.is_none());
    let created = click_toolbar(
        &context,
        &mut explorer,
        &project,
        &page,
        "explorer.newFolder",
    );
    assert!(created.input.unwrap().has_focus());
    assert_eq!(
        explorer.create.as_ref().unwrap().input.row.kind,
        TreeEntryKind::Directory
    );
}

struct Sink;

fn click_toolbar(
    context: &Context,
    explorer: &mut Explorer,
    project: &ProjectId,
    page: &TreeRowPage,
    key: &str,
) -> Output {
    let initial = frame(context, explorer, project, page, Vec::new());
    let position = initial.toolbar[key].rect.center();
    frame(
        context,
        explorer,
        project,
        page,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        context,
        explorer,
        project,
        page,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    )
}

#[test]
fn 탐색기_생성은_toolbar_접힌폴더_검증_ime와_키보드_확정을_보존한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let mut page = page();
    let mut explorer = Explorer::default();
    explorer.root = Some("/synthetic".into());
    let output = click_toolbar(&context, &mut explorer, &project, &page, "explorer.newFile");
    assert!(output.input.unwrap().has_focus());
    assert_eq!(explorer.create.as_ref().unwrap().parent, "/synthetic");
    assert_eq!(
        explorer.create.as_ref().unwrap().input.row.kind,
        TreeEntryKind::File
    );
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![Event::Text("duplicate.txt".into()), key(Key::Enter)],
    );
    assert_eq!(
        explorer.create.as_ref().unwrap().input.error.as_deref(),
        Some("Duplicate")
    );
    for (name, error) in [
        (".", "Reserved"),
        ("foo.", "Invalid"),
        ("foo:bar", "Invalid"),
        ("/", "Invalid"),
    ] {
        explorer.create.as_mut().unwrap().input.name = name.into();
        assert!(explorer.commit_create(&page.rows, &locale()).is_none());
        assert_eq!(
            explorer.create.as_ref().unwrap().input.error.as_deref(),
            Some(error)
        );
    }
    explorer.create.as_mut().unwrap().input.name = String::new();
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "文".into(),
                active_range_chars: None,
            }),
            key(Key::Enter),
        ],
    );
    assert!(explorer.create.is_some());
    assert!(
        frame(
            &context,
            &mut explorer,
            &project,
            &page,
            vec![
                Event::Ime(egui::ImeEvent::Commit("文".into())),
                key(Key::Enter)
            ]
        )
        .actions
        .is_empty()
    );
    assert!(explorer.create.is_some());
    explorer.create.as_mut().unwrap().input.name = "  sub/文.txt  ".into();
    let mut modified_enter = key(Key::Enter);
    if let Event::Key { modifiers, .. } = &mut modified_enter {
        *modifiers = Modifiers::COMMAND;
    }
    let output = frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![modified_enter],
    );
    let Action::Create(request) = &output.actions[0] else {
        panic!("expected creation");
    };
    assert_eq!(request.path, "/synthetic/sub/文.txt");
    assert!(explorer.commit_create(&page.rows, &locale()).is_none());
    explorer.create_finished(request, Err("disk collision".into()));
    assert_eq!(
        explorer.create.as_ref().unwrap().input.error.as_deref(),
        Some("disk collision")
    );
    assert_eq!(
        explorer.create.as_ref().unwrap().input.name,
        "  sub/文.txt  "
    );
    let next = explorer.commit_create(&page.rows, &locale()).unwrap();
    assert_ne!(next.token, request.token);
    explorer.create_finished(&next, Ok(()));
    assert_eq!(explorer.selected.as_deref(), Some(next.path.as_str()));
    assert!(explorer.create.is_none());

    page.rows.insert(
        0,
        TreeRow {
            path: "/synthetic/folder".into(),
            name: "folder".into(),
            kind: TreeEntryKind::Directory,
            depth: 0,
            expanded: false,
            has_children: true,
        },
    );
    explorer.selected = Some(page.rows[0].path.clone());
    assert_eq!(
        explorer.start_create(TreeEntryKind::Directory, &page),
        Some(Action::Toggle(page.rows[0].path.clone()))
    );
    assert!(explorer.create.is_none());
    page.rows[0].expanded = true;
    explorer.toggle_finished("/synthetic/folder", Some(&page));
    assert_eq!(
        explorer.create.as_ref().unwrap().parent,
        "/synthetic/folder"
    );
    assert_eq!(explorer.create.as_ref().unwrap().input.row.depth, 1);
    frame(&context, &mut explorer, &project, &page, Vec::new());
    frame(
        &context,
        &mut explorer,
        &project,
        &page,
        vec![key(Key::Escape)],
    );
    assert!(explorer.create.is_none());
    explorer.selected = Some("/synthetic/deleted".into());
    context
        .memory_mut(|memory| memory.request_focus(egui::Id::new(("native-tree-focus", &project))));
    let mut new_folder = key(Key::N);
    if let Event::Key { modifiers, .. } = &mut new_folder {
        *modifiers = Modifiers::COMMAND | Modifiers::SHIFT;
    }
    frame(&context, &mut explorer, &project, &page, vec![new_folder]);
    assert_eq!(explorer.create.as_ref().unwrap().parent, "/synthetic");
    assert_eq!(
        explorer.create.as_ref().unwrap().input.row.kind,
        TreeEntryKind::Directory
    );
    explorer.create.as_mut().unwrap().input.name = " ".into();
    assert!(explorer.commit_create(&page.rows, &locale()).is_none());
    assert!(explorer.create.is_none());
    explorer.start_create(TreeEntryKind::File, &page);
    frame(&context, &mut explorer, &project, &page, Vec::new());
    context.memory_mut(|memory| memory.request_focus(egui::Id::new("outside-explorer")));
    frame(&context, &mut explorer, &project, &page, Vec::new());
    assert!(explorer.create.is_none());
    assert_eq!(
        click_toolbar(&context, &mut explorer, &project, &page, "explorer.refresh").actions,
        vec![Action::Refresh]
    );
    assert_eq!(
        click_toolbar(
            &context,
            &mut explorer,
            &project,
            &page,
            "explorer.collapseAll"
        )
        .actions,
        vec![Action::Collapse]
    );
}

#[test]
fn 실제_탐색기_생성은_파일의_고정탭_폴더선택_충돌과_root경계를_보존한다() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-explorer-create-{}", ProjectId::new())));
        let root = directory.0.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        state.projects.write().insert(project.clone(), Project {
            id: project.clone(), root: root.to_str().unwrap().into(), name: "synthetic explorer".into(),
            capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
        });
        state.layouts.write().insert(project.clone(), taide_layout::service::default_layout());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
        let mut page = tree_actions::tree_rows(&state, &services.tree, &tasks, project.clone(), 0, None).await.unwrap();
        let repaint = Arc::new(Notify::new());
        let notify = repaint.clone();
        let mut bridge = LspBridge::connect(services.clone(), std::ffi::OsString::new(), Arc::new(move || notify.notify_one())).unwrap();
        let mut explorer = Explorer::default();
        explorer.root = Some(root.to_str().unwrap().into());
        let context = Context::default();
        for (kind, name) in [(TreeEntryKind::File, "nested/文.rs"), (TreeEntryKind::Directory, "assets/sub")] {
            explorer.selected = None;
            explorer.start_create(kind, &page);
            frame(&context, &mut explorer, &project, &page, Vec::new());
            frame(&context, &mut explorer, &project, &page, vec![Event::Text(name.into())]);
            let output = frame(&context, &mut explorer, &project, &page, vec![key(Key::Enter)]);
            let Action::Create(request) = &output.actions[0] else {panic!("expected creation");};
            bridge.create_entry(project.clone(), request.clone()).unwrap();
            let event = tokio::time::timeout(TIMEOUT, async {
                loop {
                    if let Some(reply) = bridge.poll() {
                        let Reply::ExplorerCreated(event) = reply else {panic!("unexpected creation reply");};
                        break event;
                    }
                    repaint.notified().await;
                }
            }).await.unwrap();
            assert_eq!(event.request, *request);
            assert_eq!(event.project, project);
            let created = event.result.unwrap();
            if kind == TreeEntryKind::File {
                assert_eq!(std::fs::read_to_string(&request.path).unwrap(), "");
                let layout = created.layout.unwrap();
                let taide_model::layout::PaneNode::Leaf { tabs, .. } = &layout.root else {panic!("expected initial pane");};
                let tab = tabs.iter().find(|tab|
                    matches!(&tab.kind, taide_model::layout::TabKind::File {path} if path == &request.path)).unwrap();
                assert!(!tab.preview);
                assert_eq!(tab.title, "文.rs");
            } else {
                assert!(std::path::Path::new(&request.path).is_dir());
                assert!(created.layout.is_none());
            }
            page = created.page;
            assert!(page.rows.iter().any(|row| row.path == request.path));
            explorer.create_finished(request, Ok(()));
            assert_eq!(explorer.selected.as_ref(), Some(&request.path));
            assert!(explorer.create.is_none());
        }
        let mut request = taide_native_app::explorer::CreateRequest {
            token: 1, path: root.join("nested/文.rs").to_str().unwrap().into(),
            parent: root.to_str().unwrap().into(), kind: TreeEntryKind::File,
        };
        std::fs::write(&request.path, "preserved").unwrap();
        assert!(taide_native_app::explorer::create_entry(&services, &project, &request).await.is_err());
        assert_eq!(std::fs::read_to_string(&request.path).unwrap(), "preserved");
        request.path = directory.0.join("outside.txt").to_str().unwrap().into();
        assert!(taide_native_app::explorer::create_entry(&services, &project, &request).await.is_err());
        assert!(!std::path::Path::new(&request.path).exists());
        let host_notify = Arc::new(Notify::new());
        let notify = host_notify.clone();
        let mut host = taide_native_app::host::HostBridge::connect(services.clone(), Arc::new(move || notify.notify_one())).unwrap();
        host.submit(taide_native_app::host::HostCommand::TreeCollapse(project.clone())).unwrap();
        let collapsed = tokio::time::timeout(TIMEOUT, async {
            loop {
                if let Some(reply) = host.poll() {
                    let taide_native_app::host::HostReply::Tree {result, ..} = reply else {panic!("unexpected collapse reply");};
                    break result.unwrap();
                }
                host_notify.notified().await;
            }
        }).await.unwrap();
        assert!(collapsed.rows.iter().all(|row| !row.expanded && row.depth == 0));
        state.projects.write().remove(&project);
        request.path = root.join("closed.txt").to_str().unwrap().into();
        assert!(taide_native_app::explorer::create_entry(&services, &project, &request).await.is_err());
        assert!(!std::path::Path::new(&request.path).exists());
        tokio::time::timeout(TIMEOUT, bridge.disconnect()).await.unwrap().unwrap();
        tokio::time::timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}
#[test]
fn 접힌_생성_parent는_실제_host의_일치하는_펼치기_완료만_기다린다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir()
                    .join(format!("taide-native-explorer-toggle-{}", ProjectId::new())),
            );
            let root = directory.0.join("root");
            std::fs::create_dir_all(root.join("parent")).unwrap();
            let root = std::fs::canonicalize(root).unwrap();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project.clone(),
                    root: root.to_str().unwrap().into(),
                    name: "synthetic toggle".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state, tasks.clone(), Arc::new(Sink));
            let page = tree_actions::tree_rows(
                &services.state,
                &services.tree,
                &tasks,
                project.clone(),
                0,
                None,
            )
            .await
            .unwrap();
            let path = root.join("parent").to_str().unwrap().to_owned();
            let mut explorer = Explorer::default();
            explorer.root = Some(root.to_str().unwrap().into());
            explorer.selected = Some(path.clone());
            assert_eq!(
                explorer.start_create(TreeEntryKind::File, &page),
                Some(Action::Toggle(path.clone()))
            );
            explorer.toggle_finished("wrong/path", None);
            assert!(explorer.create.is_none());
            let notify = Arc::new(Notify::new());
            let signal = notify.clone();
            let mut host = taide_native_app::host::HostBridge::connect(
                services,
                Arc::new(move || signal.notify_one()),
            )
            .unwrap();
            host.submit(taide_native_app::host::HostCommand::TreeToggle {
                project: project.clone(),
                path: path.clone(),
            })
            .unwrap();
            let (reply_path, page) = tokio::time::timeout(TIMEOUT, async {
                loop {
                    if let Some(reply) = host.poll() {
                        let taide_native_app::host::HostReply::TreeToggled {
                            project: reply_project,
                            path,
                            result,
                        } = reply
                        else {
                            panic!("expected correlated toggle reply");
                        };
                        assert_eq!(reply_project, project);
                        break (path, result.unwrap());
                    }
                    notify.notified().await;
                }
            })
            .await
            .unwrap();
            assert_eq!(reply_path, path);
            assert!(
                page.rows
                    .iter()
                    .find(|row| row.path == path)
                    .unwrap()
                    .expanded
            );
            explorer.toggle_finished(&reply_path, Some(&page));
            assert_eq!(explorer.create.as_ref().unwrap().parent, path);
            let context = Context::default();
            assert!(
                frame(&context, &mut explorer, &project, &page, Vec::new())
                    .input
                    .unwrap()
                    .has_focus()
            );
            frame(
                &context,
                &mut explorer,
                &project,
                &page,
                vec![key(Key::Escape)],
            );
            assert!(explorer.create.is_none());
            let missing_parent = TreeRowPage {
                total: 1,
                rows: vec![TreeRow {
                    path: root.join("deleted/file.txt").to_str().unwrap().into(),
                    name: "file.txt".into(),
                    kind: TreeEntryKind::File,
                    depth: 1,
                    expanded: false,
                    has_children: false,
                }],
            };
            explorer.selected = Some(missing_parent.rows[0].path.clone());
            explorer.start_create(TreeEntryKind::File, &missing_parent);
            assert!(explorer.create.is_none());
            explorer.selected = Some(path.clone());
            let mut collapsed = page.clone();
            collapsed
                .rows
                .iter_mut()
                .find(|row| row.path == path)
                .unwrap()
                .expanded = false;
            assert_eq!(
                explorer.start_create(TreeEntryKind::Directory, &collapsed),
                Some(Action::Toggle(path.clone()))
            );
            explorer.toggle_finished(&path, None);
            assert!(explorer.create.is_none());
            assert_eq!(
                explorer.start_create(TreeEntryKind::Directory, &collapsed),
                Some(Action::Toggle(path))
            );
            tokio::time::timeout(TIMEOUT, host.disconnect())
                .await
                .unwrap()
                .unwrap();
            tokio::time::timeout(TIMEOUT, tasks.shutdown())
                .await
                .unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}

impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Directory(std::path::PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn 실제_탐색기_이름변경은_입력에서_공유_worker와_새_tree_선택까지_이어진다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir()
                    .join(format!("taide-native-explorer-rename-{}", ProjectId::new())),
            );
            let root = directory.0.join("root");
            std::fs::create_dir_all(&root).unwrap();
            let root = std::fs::canonicalize(root).unwrap();
            let path = root.join("old.txt");
            std::fs::write(&path, "disk body").unwrap();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project.clone(),
                    root: root.to_str().unwrap().into(),
                    name: "synthetic explorer".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            state
                .layouts
                .write()
                .insert(project.clone(), taide_layout::service::default_layout());
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            let page =
                tree_actions::tree_rows(&state, &services.tree, &tasks, project.clone(), 0, None)
                    .await
                    .unwrap();
            let row = page
                .rows
                .iter()
                .find(|row| row.path == path.to_str().unwrap())
                .unwrap();
            let mut explorer = Explorer::default();
            explorer.start_rename(row);
            let context = Context::default();
            frame(&context, &mut explorer, &project, &page, Vec::new());
            frame(
                &context,
                &mut explorer,
                &project,
                &page,
                vec![Event::Text("sub/new 文.rs".into())],
            );
            let mut actions = frame(
                &context,
                &mut explorer,
                &project,
                &page,
                vec![key(Key::Enter)],
            )
            .actions;
            let Action::Rename(request) = actions.pop().unwrap() else {
                panic!("expected rename request")
            };
            assert!(actions.is_empty());
            let ready = Arc::new(Notify::new());
            let repaint = ready.clone();
            let mut bridge = LspBridge::connect(
                services.clone(),
                Default::default(),
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let document = store
                .open_file(
                    path.clone(),
                    taide_file::service::open_file(&path, &[], false).unwrap(),
                )
                .unwrap();
            bridge
                .rename_entry(project.clone(), request.clone())
                .unwrap();
            let page = tokio::time::timeout(TIMEOUT, async {
                loop {
                    let Some(reply) = bridge.poll() else {
                        ready.notified().await;
                        continue;
                    };
                    match reply {
                        Reply::RenamePrepare(event) => {
                            let result = taide_native_app::workspace_rename::prepare_documents(
                                &store,
                                &event.from,
                                &HashMap::new(),
                            );
                            let _result = event.completion.send(result);
                        }
                        Reply::Renamed(event) => {
                            assert!(event.warnings.is_empty());
                            explorer.moved(&event.from, &event.to);
                            let result = taide_native_app::workspace_rename::commit_documents(
                                &mut store, &event,
                            );
                            let _result = event.completion.send(result);
                        }
                        Reply::ExplorerRenamed(event) => {
                            assert_eq!(event.project, project);
                            assert_eq!(event.request, request);
                            let page = event.result.unwrap();
                            explorer.rename_finished(&event.request, Ok(()));
                            break page;
                        }
                        _ => panic!("unexpected explorer rename reply"),
                    }
                }
            })
            .await
            .unwrap();
            assert!(!path.exists());
            assert_eq!(std::fs::read_to_string(&request.to).unwrap(), "disk body");
            assert_eq!(
                store.documents().snapshot(document).unwrap().key,
                taide_native_editor::document::DocumentKey::File(request.to.clone().into())
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .metadata
                    .language_id,
                "rust"
            );
            assert_eq!(explorer.selected.as_deref(), Some(request.to.as_str()));
            assert!(explorer.rename.is_none());
            assert!(page.rows.iter().any(|row| row.path == request.to));
            assert!(!page.rows.iter().any(|row| row.path == request.from));
            let next = frame(&context, &mut explorer, &project, &page, Vec::new());
            assert!(next.rows.contains_key(&request.to));
            tokio::time::timeout(TIMEOUT, bridge.disconnect())
                .await
                .unwrap()
                .unwrap();
            tokio::time::timeout(TIMEOUT, tasks.shutdown())
                .await
                .unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}

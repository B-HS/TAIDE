use std::cell::RefCell;
use std::rc::Rc;

use super::inlay_hints_tests::key;
use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};
use super::*;
use taide_model::layout::{AuxWindowLayout, PaneNode};
use taide_model::project::ShellSlotTree;

const DEADLINE: Duration = Duration::from_secs(10);
const FIRST_SLOT: u32 = 1;
const SECOND_SLOT: u32 = 2;
const AUXILIARY_SCREEN: [f32; 2] = [1000.0, 700.0];
const OTHER_FILE: &str = "other.txt";
const TAB_BAR_BOTTOM: f32 =
    taide_native_ui::shell::TITLE_HEIGHT + taide_native_ui::shell::TAB_HEIGHT;
const WIDE_SCREEN: [f32; 2] = [1280.0, 800.0];
const FIRST_BRANCH: &str = "synthetic-title";
const NEXT_BRANCH: &str = "synthetic-next";
const LSP_MODE: &str = "--native-inlay-hints";
const LSP_FILE: &str = "current.rs";
const LSP_TEXT: &str = "class\n    method\nend";
const LSP_CANDIDATE: &str = "method";
const DETACHED_HOST_FRAMES: usize = 4;
const FILLER_FILE_COUNT: usize = 60;
const FIRST_FILE: &str = "a.txt";
const LAST_FILE: &str = "z.txt";
const RENAMED_FILE: &str = "renamed.txt";

#[derive(Default)]
struct Viewports {
    events: HashMap<egui::ViewportId, Vec<egui::Event>>,
    close: HashSet<egui::ViewportId>,
    focus: HashMap<egui::ViewportId, egui::Id>,
    outputs: HashMap<egui::ViewportId, egui::FullOutput>,
}

impl Viewports {
    fn texts(&self, viewport: egui::ViewportId) -> Vec<String> {
        self.outputs
            .get(&viewport)
            .map(|output| {
                output
                    .shapes
                    .iter()
                    .filter_map(|clipped| match &clipped.shape {
                        egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn shows(&self, viewport: egui::ViewportId, text: &str) -> bool {
        self.texts(viewport).iter().any(|shown| shown == text)
    }
}

fn install_viewports(context: &egui::Context) -> Rc<RefCell<Viewports>> {
    let viewports = Rc::new(RefCell::new(Viewports::default()));
    let shared = viewports.clone();
    context.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
        let id = viewport.ids.this;
        let (events, close, focus) = {
            let mut state = shared.borrow_mut();
            (
                state.events.remove(&id).unwrap_or_default(),
                state.close.remove(&id),
                state.focus.remove(&id),
            )
        };
        let info = egui::ViewportInfo {
            events: if close {
                vec![egui::ViewportEvent::Close]
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: id,
                viewports: std::iter::once((id, info)).collect(),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(AUXILIARY_SCREEN[0], AUXILIARY_SCREEN[1]),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                if let Some(focus) = focus {
                    ui.memory_mut(|memory| memory.request_focus(focus));
                }
                (viewport.viewport_ui_cb)(ui)
            },
        );
        output.textures_delta.clear();
        shared.borrow_mut().outputs.insert(id, output);
    });
    viewports
}

fn step_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = step(application, context, Vec::new());
        if ready(application, &output) {
            return output;
        }
        assert!(Instant::now() < deadline, "phase={phase}");
        std::thread::yield_now();
    }
}

fn file_tab(path: &std::path::Path) -> Tab {
    Tab {
        id: TabId::new(),
        kind: TabKind::File {
            path: path.to_str().unwrap().into(),
        },
        title: path.file_name().unwrap().to_str().unwrap().into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn auxiliary_window(slot: u32, tab: &Tab) -> AuxWindowLayout {
    let pane = PaneId::new();
    AuxWindowLayout {
        slot,
        root: PaneNode::Leaf {
            id: pane.clone(),
            tabs: vec![tab.clone()],
            active: Some(tab.id.clone()),
        },
        focused_pane: pane,
    }
}

fn viewport_of(
    application: &NativeApplication,
    project: &ProjectId,
    slot: u32,
) -> egui::ViewportId {
    application
        .auxiliary_windows
        .find(project, slot)
        .unwrap_or_else(|| panic!("auxiliary window {slot} is open"))
        .viewport
}

fn shortcut(key_code: egui::Key) -> Vec<egui::Event> {
    key(
        key_code,
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
    )
}

#[test]
fn 실제_app_보조_창은_viewport로_복원되어_자기_셸과_키맵과_닫힘을_따른다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-auxiliary-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    let missing = fixture.directory.join("missing");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    for name in ["notes.txt", "second.txt", "other.txt"] {
        std::fs::write(root.join(name), "synthetic\n").unwrap();
    }
    let is_other =
        |tab: &&Tab| matches!(&tab.kind, TabKind::File { path } if path.ends_with(OTHER_FILE));
    let notes = file_tab(&root.join("notes.txt"));
    let second = file_tab(&root.join("second.txt"));
    let alpha = ProjectId::new();
    let gone = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    insert_project(&state, &gone, &missing, "gone", true);
    let main_pane = PaneId::new();
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&alpha).unwrap();
        layout.root = PaneNode::Leaf {
            id: main_pane.clone(),
            tabs: Vec::new(),
            active: None,
        };
        layout.focused_pane = main_pane.clone();
        layout.shell_view.sidebar_collapsed = true;
        layout.auxiliary_windows = vec![
            auxiliary_window(FIRST_SLOT, &notes),
            auxiliary_window(SECOND_SLOT, &second),
        ];
        layouts.get_mut(&gone).unwrap().auxiliary_windows = vec![auxiliary_window(
            FIRST_SLOT,
            &file_tab(&missing.join("gone.txt")),
        )];
    }
    {
        let mut session = state.session.write();
        let mut gone_ref = project_ref(&gone, &missing, "gone");
        gone_ref.root_missing = true;
        session.projects = vec![project_ref(&alpha, &root, "alpha"), gone_ref];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();

    assert_eq!(
        services.windows.label_for(&alpha, FIRST_SLOT).as_deref(),
        Some("editor-1")
    );
    assert_eq!(
        services.windows.label_for(&alpha, SECOND_SLOT).as_deref(),
        Some("editor-2")
    );
    assert!(services.windows.label_for(&gone, FIRST_SLOT).is_none());
    let first = viewport_of(application, &alpha, FIRST_SLOT);
    let second_viewport = viewport_of(application, &alpha, SECOND_SLOT);

    let root_output = step_until(application, &context, "auxiliary explorers", |_, _| {
        let viewports = viewports.borrow();
        [first, second_viewport]
            .iter()
            .all(|viewport| viewports.shows(*viewport, "other.txt"))
    });
    for viewport in [first, second_viewport] {
        let output = root_output
            .viewport_output
            .get(&viewport)
            .expect("the root pass requests each auxiliary viewport");
        assert_eq!(output.parent, egui::ViewportId::ROOT);
        assert!(output.viewport_ui_cb.is_none());
        assert_eq!(output.builder.title.as_deref(), Some("alpha — TAIDE"));
        assert_eq!(
            output.builder.inner_size,
            Some(egui::vec2(
                crate::auxiliary_windows::DEFAULT_SIZE[0],
                crate::auxiliary_windows::DEFAULT_SIZE[1]
            ))
        );
        assert_eq!(
            output.builder.min_inner_size,
            Some(egui::vec2(
                crate::auxiliary_windows::MINIMUM_SIZE[0],
                crate::auxiliary_windows::MINIMUM_SIZE[1]
            ))
        );
        assert_eq!(output.builder.titlebar_shown, Some(false));
    }
    let root_texts: Vec<String> = root_output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    assert!(!root_texts.iter().any(|text| text == "other.txt"));
    if application.shell.has_title_bar {
        let separator = presentation::message(&application.locale, "window.titleSeparator", &[]);
        let title = format!("notes.txt {separator} alpha");
        assert!(
            viewports.borrow().shows(first, &title),
            "{:?}",
            viewports.borrow().texts(first)
        );
    }

    viewports
        .borrow_mut()
        .events
        .insert(first, shortcut(egui::Key::B));
    step_until(
        application,
        &context,
        "auxiliary explorer collapse",
        |application, _| {
            !application
                .auxiliary_windows
                .find(&alpha, FIRST_SLOT)
                .unwrap()
                .is_explorer_open
        },
    );
    step(application, &context, Vec::new());
    assert!(!viewports.borrow().shows(first, "other.txt"));
    assert!(viewports.borrow().shows(second_viewport, "other.txt"));
    assert!(
        services.state.layouts.read()[&alpha]
            .shell_view
            .sidebar_collapsed
    );

    let row = {
        let viewports = viewports.borrow();
        node_rect(
            &viewports.outputs[&second_viewport],
            "other.txt",
            egui::Rect::EVERYTHING,
        )
        .expect("the auxiliary explorer row is accessible")
    };
    viewports.borrow_mut().events.insert(
        second_viewport,
        pointer(row.center(), egui::PointerButton::Primary),
    );
    step_until(
        application,
        &context,
        "auxiliary explorer opens in its own window",
        |_, _| {
            let layouts = services.state.layouts.read();
            layouts[&alpha]
                .auxiliary_windows
                .iter()
                .find(|window| window.slot == SECOND_SLOT)
                .is_some_and(|window| crate::tabs::tabs_in(&window.root).iter().any(is_other))
        },
    );
    assert!(crate::tabs::tabs_in(&services.state.layouts.read()[&alpha].root).is_empty());

    viewports.borrow_mut().close.insert(second_viewport);
    step_until(
        application,
        &context,
        "close request returns tabs",
        |_, _| {
            let layouts = services.state.layouts.read();
            let layout = &layouts[&alpha];
            layout.auxiliary_windows.len() == 1
                && crate::tabs::tabs_in(&layout.root)
                    .iter()
                    .any(|tab| tab.id == second.id)
        },
    );
    let returned = step(application, &context, Vec::new());
    assert!(
        application
            .auxiliary_windows
            .find(&alpha, SECOND_SLOT)
            .is_none()
    );
    assert!(services.windows.label_for(&alpha, SECOND_SLOT).is_none());
    assert!(!returned.viewport_output.contains_key(&second_viewport));

    viewports
        .borrow_mut()
        .events
        .insert(first, shortcut(egui::Key::W));
    step_until(
        application,
        &context,
        "last tab close closes window",
        |_, _| {
            services.state.layouts.read()[&alpha]
                .auxiliary_windows
                .is_empty()
        },
    );
    let closed = step(application, &context, Vec::new());
    assert!(application.auxiliary_windows.windows().is_empty());
    assert!(services.windows.label_for(&alpha, FIRST_SLOT).is_none());
    assert!(!closed.viewport_output.contains_key(&first));
    let layouts = services.state.layouts.read();
    let main_tabs = crate::tabs::tabs_in(&layouts[&alpha].root);
    assert!(main_tabs.iter().any(|tab| tab.id == second.id));
    assert!(main_tabs.iter().any(is_other));
    assert!(!main_tabs.iter().any(|tab| tab.id == notes.id));
}

fn is_window_open(application: &NativeApplication, project: &ProjectId, slot: u32) -> bool {
    application.auxiliary_windows.find(project, slot).is_some()
}

fn is_explorer_open(application: &NativeApplication, project: &ProjectId, slot: u32) -> bool {
    application
        .auxiliary_windows
        .find(project, slot)
        .is_some_and(|window| window.is_explorer_open)
}

fn tab_bar() -> egui::Rect {
    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(f32::INFINITY, TAB_BAR_BOTTOM))
}

fn labels(output: &egui::FullOutput) -> Vec<String> {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .map(|update| {
            update
                .nodes
                .iter()
                .filter_map(|(_, node)| node.label().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn choose_tab_menu(
    application: &mut NativeApplication,
    context: &egui::Context,
    viewports: &Rc<RefCell<Viewports>>,
    viewport: Option<egui::ViewportId>,
    title: &str,
    item: &str,
) -> Vec<String> {
    let send = |application: &mut NativeApplication, events: Vec<egui::Event>| match viewport {
        Some(id) => {
            viewports.borrow_mut().events.insert(id, events);
            step(application, context, Vec::new());
            viewports
                .borrow()
                .outputs
                .get(&id)
                .cloned()
                .unwrap_or_default()
        }
        None => step(application, context, events),
    };
    let deadline = Instant::now() + DEADLINE;
    let tab = loop {
        if let Some(tab) = node_rect(&send(application, Vec::new()), title, tab_bar()) {
            break tab;
        }
        assert!(Instant::now() < deadline, "tab {title}");
        std::thread::yield_now();
    };
    send(
        application,
        pointer(tab.center(), egui::PointerButton::Secondary),
    );
    let deadline = Instant::now() + DEADLINE;
    let (menu, target) = loop {
        let output = send(application, Vec::new());
        if let Some(target) = node_rect(&output, item, egui::Rect::EVERYTHING) {
            break (labels(&output), target);
        }
        assert!(
            Instant::now() < deadline,
            "menu item {item}: {:?}",
            labels(&output)
        );
        std::thread::yield_now();
    };
    send(
        application,
        pointer(target.center(), egui::PointerButton::Primary),
    );
    menu
}

#[test]
fn 실제_app_탭_메뉴는_탭을_새창_기존창_메인창으로_옮기고_보조창_탐색기로_reveal_rename한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-auxiliary-move-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let [a, b, c] = ["a.txt", "b.txt", "c.txt"].map(|name| {
        std::fs::write(root.join(name), "synthetic\n").unwrap();
        file_tab(&root.join(name))
    });
    let a_path = root.join("a.txt").to_str().unwrap().to_owned();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&alpha).unwrap();
        let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
            panic!("expected leaf")
        };
        *tabs = vec![a.clone(), b.clone(), c.clone()];
        *active = Some(a.id.clone());
        layout.shell_view.sidebar_collapsed = true;
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&alpha, &root, "alpha")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let window_tabs = |slot: u32| -> Option<Vec<TabId>> {
        services.state.layouts.read()[&alpha]
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == slot)
            .map(|window| {
                crate::tabs::tabs_in(&window.root)
                    .into_iter()
                    .map(|tab| tab.id.clone())
                    .collect()
            })
    };
    let has_tab = |slot: u32, tab: &TabId| window_tabs(slot).is_some_and(|tabs| tabs.contains(tab));
    let main_tabs = || -> Vec<TabId> {
        crate::tabs::tabs_in(&services.state.layouts.read()[&alpha].root)
            .into_iter()
            .map(|tab| tab.id.clone())
            .collect()
    };
    let message = |key: &str, slot: Option<u32>| {
        let slot = slot.map(|slot| slot.to_string());
        let arguments: Vec<(&str, &str)> =
            slot.iter().map(|slot| ("slot", slot.as_str())).collect();
        presentation::message(&application.locale, key, &arguments)
    };
    let new_window = message("tab.moveToNewWindow", None);
    let main_window = message("tab.moveToMainWindow", None);
    let first_window = message("tab.moveToWindowNumbered", Some(FIRST_SLOT));
    let second_window = message("tab.moveToWindowNumbered", Some(SECOND_SLOT));
    let reveal = message("tab.revealInExplorerView", None);
    let rename = message("tab.rename", None);

    let menu = choose_tab_menu(
        application,
        &context,
        &viewports,
        None,
        "a.txt",
        &new_window,
    );
    assert!(!menu.contains(&main_window), "{menu:?}");
    assert!(!menu.contains(&first_window), "{menu:?}");
    step_until(application, &context, "a new window", |application, _| {
        window_tabs(FIRST_SLOT) == Some(vec![a.id.clone()])
            && is_window_open(application, &alpha, FIRST_SLOT)
    });
    assert_eq!(
        application
            .auxiliary_windows
            .find(&alpha, FIRST_SLOT)
            .unwrap()
            .label,
        "editor-1"
    );
    assert_eq!(
        services.windows.label_for(&alpha, FIRST_SLOT).as_deref(),
        Some("editor-1")
    );
    let first = viewport_of(application, &alpha, FIRST_SLOT);
    let output = step_until(application, &context, "new window viewport", |_, _| {
        viewports
            .borrow()
            .outputs
            .get(&first)
            .is_some_and(|output| node_rect(output, "a.txt", tab_bar()).is_some())
    });
    assert_eq!(
        output
            .viewport_output
            .get(&first)
            .and_then(|viewport| viewport.builder.title.as_deref()),
        Some("alpha — TAIDE")
    );

    let menu = choose_tab_menu(
        application,
        &context,
        &viewports,
        None,
        "c.txt",
        &first_window,
    );
    assert!(menu.contains(&new_window), "{menu:?}");
    assert!(!menu.contains(&main_window), "{menu:?}");
    step_until(application, &context, "c to window 1", |_, _| {
        has_tab(FIRST_SLOT, &c.id) && !main_tabs().contains(&c.id)
    });

    choose_tab_menu(
        application,
        &context,
        &viewports,
        None,
        "b.txt",
        &new_window,
    );
    step_until(application, &context, "b new window", |application, _| {
        window_tabs(SECOND_SLOT) == Some(vec![b.id.clone()])
            && is_window_open(application, &alpha, SECOND_SLOT)
    });
    assert_eq!(
        services.windows.label_for(&alpha, SECOND_SLOT).as_deref(),
        Some("editor-2")
    );
    assert!(main_tabs().is_empty());
    let second = viewport_of(application, &alpha, SECOND_SLOT);

    let menu = choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(first),
        "c.txt",
        &main_window,
    );
    assert!(
        menu.contains(&new_window) && menu.contains(&second_window),
        "{menu:?}"
    );
    assert!(!menu.contains(&first_window), "{menu:?}");
    step_until(application, &context, "c to main", |_, _| {
        main_tabs() == vec![c.id.clone()] && window_tabs(FIRST_SLOT) == Some(vec![a.id.clone()])
    });

    choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(first),
        "a.txt",
        &second_window,
    );
    step_until(application, &context, "emptied closes", |application, _| {
        window_tabs(FIRST_SLOT).is_none() && !is_window_open(application, &alpha, FIRST_SLOT)
    });
    assert!(services.windows.label_for(&alpha, FIRST_SLOT).is_none());
    assert!(has_tab(SECOND_SLOT, &a.id) && has_tab(SECOND_SLOT, &b.id));
    let closed = step(application, &context, Vec::new());
    assert!(!closed.viewport_output.contains_key(&first));
    assert_eq!(main_tabs(), vec![c.id.clone()]);

    viewports
        .borrow_mut()
        .events
        .insert(second, shortcut(egui::Key::B));
    step_until(
        application,
        &context,
        "explorer collapses",
        |application, _| !is_explorer_open(application, &alpha, SECOND_SLOT),
    );
    let mount = application
        .auxiliary_windows
        .find(&alpha, SECOND_SLOT)
        .unwrap()
        .explorer
        .clone();
    application.symbol_sidebars.entry(&alpha, &mount).view = crate::symbol_sidebar::View::Outline;
    choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(second),
        "a.txt",
        &reveal,
    );
    step_until(
        application,
        &context,
        "auxiliary reveal",
        |application, _| {
            let is_selected = application.explorers.get(&alpha).is_some_and(|explorer| {
                explorer.selected.as_deref() == Some(a_path.as_str()) && explorer.rename.is_none()
            });
            let is_loaded = application
                .trees
                .get(&alpha)
                .is_some_and(|page| page.rows.iter().any(|row| row.path == a_path));
            is_explorer_open(application, &alpha, SECOND_SLOT) && is_selected && is_loaded
        },
    );
    assert_eq!(
        application.symbol_sidebars.entry(&alpha, &mount).view,
        crate::symbol_sidebar::View::Files
    );
    assert!(
        services.state.layouts.read()[&alpha]
            .shell_view
            .sidebar_collapsed
    );

    choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(second),
        "a.txt",
        &rename,
    );
    step_until(
        application,
        &context,
        "auxiliary rename",
        |application, _| {
            application.explorers.get(&alpha).is_some_and(|explorer| {
                explorer
                    .rename
                    .as_ref()
                    .is_some_and(|draft| draft.row.path == a_path && draft.name == "a.txt")
            })
        },
    );
}

fn auxiliary_explorer_column() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(0.0, TAB_BAR_BOTTOM),
        egui::pos2(
            taide_native_ui::shell::EXPLORER_DEFAULT_WIDTH,
            AUXILIARY_SCREEN[1],
        ),
    )
}

fn focused_text_input(output: &egui::FullOutput) -> Option<(String, [usize; 2], egui::Rect)> {
    let update = output.platform_output.accesskit_update.as_ref()?;
    let (_, node) = update.nodes.iter().find(|(id, _)| *id == update.focus)?;
    if node.role() != egui::accesskit::Role::TextInput {
        return None;
    }
    let selection = node.text_selection()?;
    let anchor = selection.anchor.character_index;
    let focus = selection.focus.character_index;
    let bounds = node.bounds()?;
    Some((
        node.value()?.to_owned(),
        [anchor.min(focus), anchor.max(focus)],
        egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        ),
    ))
}

#[test]
fn 실제_app_보조_창_탐색기_reveal_rename은_메인_사이드바가_열려_있어도_그_창에서만_스크롤과_포커스를_받는다()
 {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-auxiliary-reveal-{}",
            ProjectId::new()
        )),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    for index in 0..FILLER_FILE_COUNT {
        std::fs::write(root.join(format!("m{index:02}.txt")), "synthetic\n").unwrap();
    }
    let [first, last] = [FIRST_FILE, LAST_FILE].map(|name| {
        std::fs::write(root.join(name), "synthetic\n").unwrap();
        file_tab(&root.join(name))
    });
    let [first_path, last_path] =
        [FIRST_FILE, LAST_FILE].map(|name| root.join(name).to_str().unwrap().to_owned());
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&alpha).unwrap();
        let main_pane = PaneId::new();
        layout.root = PaneNode::Leaf {
            id: main_pane.clone(),
            tabs: Vec::new(),
            active: None,
        };
        layout.focused_pane = main_pane;
        layout.shell_view.sidebar_collapsed = false;
        let pane = PaneId::new();
        layout.auxiliary_windows = vec![AuxWindowLayout {
            slot: FIRST_SLOT,
            root: PaneNode::Leaf {
                id: pane.clone(),
                tabs: vec![first.clone(), last.clone()],
                active: Some(last.id.clone()),
            },
            focused_pane: pane,
        }];
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&alpha, &root, "alpha")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let auxiliary = viewport_of(application, &alpha, FIRST_SLOT);
    let reveal = presentation::message(&application.locale, "tab.revealInExplorerView", &[]);
    let rename = presentation::message(&application.locale, "tab.rename", &[]);
    let auxiliary_row = |name: &str| {
        viewports
            .borrow()
            .outputs
            .get(&auxiliary)
            .is_some_and(|output| node_rect(output, name, auxiliary_explorer_column()).is_some())
    };
    let is_selected = |application: &NativeApplication, path: &str| {
        application
            .explorers
            .get(&alpha)
            .is_some_and(|explorer| explorer.selected.as_deref() == Some(path))
    };

    let main_output = step_until(application, &context, "both explorers", |_, output| {
        node_rect(output, "m00.txt", egui::Rect::EVERYTHING).is_some() && auxiliary_row("m00.txt")
    });
    assert!(node_rect(&main_output, LAST_FILE, egui::Rect::EVERYTHING).is_none());
    assert!(!auxiliary_row(LAST_FILE));

    choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(auxiliary),
        LAST_FILE,
        &reveal,
    );
    let main_output = step_until(
        application,
        &context,
        "auxiliary reveal scrolls the auxiliary explorer",
        |application, _| is_selected(application, &last_path) && auxiliary_row(LAST_FILE),
    );
    assert!(node_rect(&main_output, LAST_FILE, egui::Rect::EVERYTHING).is_none());
    assert!(!auxiliary_row(FIRST_FILE));

    choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(auxiliary),
        FIRST_FILE,
        &reveal,
    );
    step_until(
        application,
        &context,
        "auxiliary reveal measures the auxiliary scroll offset",
        |application, _| is_selected(application, &first_path) && auxiliary_row(FIRST_FILE),
    );

    choose_tab_menu(
        application,
        &context,
        &viewports,
        Some(auxiliary),
        FIRST_FILE,
        &rename,
    );
    let whole_name = [0, FIRST_FILE.chars().count()];
    let main_output = step_until(
        application,
        &context,
        "auxiliary rename focuses the auxiliary input",
        |_, _| {
            viewports
                .borrow()
                .outputs
                .get(&auxiliary)
                .and_then(focused_text_input)
                .is_some_and(|(value, selection, rect)| {
                    value == FIRST_FILE
                        && selection == whole_name
                        && auxiliary_explorer_column().contains_rect(rect)
                })
        },
    );
    assert!(focused_text_input(&main_output).is_none());
    assert!(
        node_rect(&main_output, "m00.txt", egui::Rect::EVERYTHING).is_some(),
        "the main explorer keeps showing the same rows"
    );

    viewports
        .borrow_mut()
        .events
        .insert(auxiliary, vec![egui::Event::Text(RENAMED_FILE.into())]);
    step_until(
        application,
        &context,
        "typing goes to the auxiliary rename input",
        |application, _| {
            application.explorers.get(&alpha).is_some_and(|explorer| {
                explorer
                    .rename
                    .as_ref()
                    .is_some_and(|draft| draft.row.path == first_path && draft.name == RENAMED_FILE)
            })
        },
    );
}

fn wide_step(application: &mut NativeApplication, context: &egui::Context) -> egui::FullOutput {
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(WIDE_SCREEN[0], WIDE_SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
    output
}

fn title_shows(output: &egui::FullOutput, text: &str) -> bool {
    output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(shape)
            if shape.galley.text() == text
                && shape.visual_bounding_rect().bottom() <= taide_native_ui::shell::TITLE_HEIGHT)
    })
}

#[cfg(target_os = "macos")]
#[test]
fn 실제_app_제목줄은_프로젝트_git_branch를_읽고_git_변경_뒤_다시_읽는다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-title-branch-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    let git = root.join(".git");
    for directory in [&data, &git.join("objects"), &git.join("refs").join("heads")] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let head = |branch: &str| {
        std::fs::write(git.join("HEAD"), format!("ref: refs/heads/{branch}\n")).unwrap()
    };
    head(FIRST_BRANCH);
    std::fs::write(root.join("notes.txt"), "synthetic\n").unwrap();
    let notes = file_tab(&root.join("notes.txt"));
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let PaneNode::Leaf { tabs, active, .. } = &mut layouts.get_mut(&alpha).unwrap().root else {
            panic!("expected leaf")
        };
        *tabs = vec![notes.clone()];
        *active = Some(notes.id.clone());
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&alpha, &root, "alpha")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let wait_branch = |application: &mut NativeApplication, branch: &str| {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let output = wide_step(application, &context);
            if application.title_branches.branch(&alpha) == Some(branch)
                && title_shows(&output, branch)
            {
                return;
            }
            assert!(Instant::now() < deadline, "branch {branch}");
            std::thread::yield_now();
        }
    };

    wait_branch(application, FIRST_BRANCH);
    let narrow = step(application, &context, Vec::new());
    assert!(!title_shows(&narrow, FIRST_BRANCH));

    head(NEXT_BRANCH);
    application
        .services
        .events
        .publish(AppEvent::GitStatusChanged {
            project_id: alpha.clone(),
        });
    wait_branch(application, NEXT_BRANCH);
}

#[cfg(target_os = "macos")]
#[test]
fn 실제_app_보조_창_제목줄_branch는_메인에_보이지_않는_프로젝트도_읽는다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-auxiliary-branch-{}",
            ProjectId::new()
        )),
        application: None,
    };
    let data = fixture.directory.join("data");
    let main_root = fixture.directory.join("alpha");
    let root = fixture.directory.join("beta");
    let git = root.join(".git");
    for directory in [
        &data,
        &main_root,
        &git.join("objects"),
        &git.join("refs").join("heads"),
    ] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(
        git.join("HEAD"),
        format!("ref: refs/heads/{FIRST_BRANCH}\n"),
    )
    .unwrap();
    std::fs::write(root.join("notes.txt"), "synthetic\n").unwrap();
    let notes = file_tab(&root.join("notes.txt"));
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &main_root, "alpha", false);
    insert_project(&state, &beta, &root, "beta", false);
    state
        .layouts
        .write()
        .get_mut(&beta)
        .unwrap()
        .auxiliary_windows = vec![auxiliary_window(FIRST_SLOT, &notes)];
    {
        let mut session = state.session.write();
        session.projects = vec![
            project_ref(&alpha, &main_root, "alpha"),
            project_ref(&beta, &root, "beta"),
        ];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    assert!(application.shell.has_title_bar);
    let viewport = viewport_of(application, &beta, FIRST_SLOT);

    step_until(
        application,
        &context,
        "auxiliary-only project branch",
        |application, _| {
            application.title_branches.branch(&beta) == Some(FIRST_BRANCH)
                && viewports.borrow().shows(viewport, "notes.txt")
        },
    );
}

#[test]
fn 실제_app_보조_창에서만_열린_파일도_lsp_동기화와_응답_수락을_받는다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-auxiliary-lsp-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let bin = fixture.directory.join("bin");
    let source = fixture.directory.join("alpha");
    for directory in [&data, &bin, &source] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(source.join(LSP_FILE), LSP_TEXT).unwrap();
    let root = source.canonicalize().unwrap();
    let file = root.join(LSP_FILE);
    let path = file.to_str().unwrap().to_owned();
    let tab = file_tab(&file);
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        let pane = PaneId::new();
        layout.root = PaneNode::Leaf {
            id: pane.clone(),
            tabs: Vec::new(),
            active: None,
        };
        layout.focused_pane = pane;
        layout.auxiliary_windows = vec![auxiliary_window(FIRST_SLOT, &tab)];
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&project, &root, "alpha")];
        session.active_project = Some(project.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let viewport = viewport_of(application, &project, FIRST_SLOT);
    step_until(
        application,
        &context,
        "auxiliary document",
        |application, _| application.files.contains_key(&path),
    );
    let document = application.files[&path].id;
    super::inlay_hints_tests::connect_lsp_mock(application, &context, &bin, LSP_MODE);

    super::inlay_hints_tests::wait(
        application,
        &context,
        "auxiliary-only document binding",
        |application| {
            let snapshot = application.store.documents().snapshot(document).unwrap();
            application.lsp.as_ref().is_some_and(|lsp| {
                !lsp.completion_providers(&project, &snapshot).is_empty()
                    && !lsp.inlay_hint_providers(&project, &snapshot).is_empty()
            })
        },
    );
    let view = super::inlay_hints_tests::body_view(application, &path, &tab.id);
    super::inlay_hints_tests::wait(
        application,
        &context,
        "auxiliary inlay hints accepted",
        |application| {
            application
                .editor_inlay_hints
                .display(
                    &application.store,
                    viewport,
                    view,
                    application.editor_inlay_hint_colors,
                )
                .is_some_and(|hints| !hints.items.is_empty())
        },
    );

    let caret = LSP_TEXT.find("\nend").unwrap();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: caret,
                    head: caret,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    let editor = application
        .editor_keymap_targets
        .iter()
        .find_map(|((target_viewport, id), (target, _))| {
            (*target_viewport == viewport && *target == view).then_some(*id)
        })
        .expect("the auxiliary editor registers its keymap target");
    viewports.borrow_mut().focus.insert(viewport, editor);
    application.document_edits.push((
        tab.id.clone(),
        DocumentEdit::Completion(taide_native_editor::completion::Command::Trigger),
    ));
    super::inlay_hints_tests::wait(
        application,
        &context,
        "auxiliary completion accepted",
        |application| {
            application
                .editor_completion
                .candidates(&application.store, view)
                .is_some_and(|items| {
                    items
                        .iter()
                        .any(|candidate| candidate.item.label == LSP_CANDIDATE)
                })
        },
    );
    assert_eq!(
        application
            .editor_completion
            .request(&application.store, view)
            .map(|request| request.viewport),
        Some(viewport)
    );
}

struct SingleWindow {
    fixture: Fixture,
    context: egui::Context,
    viewports: Rc<RefCell<Viewports>>,
    project: ProjectId,
    main: Tab,
    notes: Tab,
}

fn single_window(name: &str) -> SingleWindow {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-auxiliary-{name}-{}",
            ProjectId::new()
        )),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    for name in ["main.txt", "notes.txt"] {
        std::fs::write(root.join(name), "synthetic\n").unwrap();
    }
    let main = file_tab(&root.join("main.txt"));
    let notes = file_tab(&root.join("notes.txt"));
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        let pane = PaneId::new();
        layout.root = PaneNode::Leaf {
            id: pane.clone(),
            tabs: vec![main.clone()],
            active: Some(main.id.clone()),
        };
        layout.focused_pane = pane;
        layout.auxiliary_windows = vec![auxiliary_window(FIRST_SLOT, &notes)];
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&project, &root, "alpha")];
        session.active_project = Some(project.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let viewport = viewport_of(application, &project, FIRST_SLOT);
    step_until(application, &context, "auxiliary window shown", |_, _| {
        viewports.borrow().shows(viewport, "notes.txt")
    });
    SingleWindow {
        fixture,
        context,
        viewports,
        project,
        main,
        notes,
    }
}

#[test]
fn 실제_app_메인_패스가_dirty_반영에_실패해도_보조_창_viewport를_유지한다() {
    let mut window = single_window("flush");
    let application = window.fixture.application.as_mut().unwrap();
    let viewport = viewport_of(application, &window.project, FIRST_SLOT);
    let bridge = application.bridge.take();
    application
        .pending_dirty
        .insert(window.main.id.clone(), false);
    window.viewports.borrow_mut().outputs.clear();
    let failed = step(application, &window.context, Vec::new());
    let is_flush_refused = application.pending_dirty.contains_key(&window.main.id);
    application.bridge = bridge;

    assert!(is_flush_refused);
    assert!(failed.viewport_output.contains_key(&viewport));
    assert!(window.viewports.borrow().shows(viewport, "notes.txt"));
    assert!(is_window_open(application, &window.project, FIRST_SLOT));
    step_until(
        application,
        &window.context,
        "dirty flush resumes",
        |application, output| {
            !application.pending_dirty.contains_key(&window.main.id)
                && output.viewport_output.contains_key(&viewport)
        },
    );
}

#[test]
fn 실제_app_보조_창_닫기의_탭_복귀는_host_제출이_실패하면_다시_보낸다() {
    let mut window = single_window("return");
    let application = window.fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let viewport = viewport_of(application, &window.project, FIRST_SLOT);
    let bridge = application.bridge.take();
    window.viewports.borrow_mut().close.insert(viewport);
    let mut detached = step(application, &window.context, Vec::new());
    for _ in 0..DETACHED_HOST_FRAMES {
        detached = step(application, &window.context, Vec::new());
    }
    let is_return_pending = services.state.layouts.read()[&window.project]
        .auxiliary_windows
        .iter()
        .any(|auxiliary| auxiliary.slot == FIRST_SLOT);
    application.bridge = bridge;

    assert!(!is_window_open(application, &window.project, FIRST_SLOT));
    assert!(!detached.viewport_output.contains_key(&viewport));
    assert!(
        services
            .windows
            .label_for(&window.project, FIRST_SLOT)
            .is_none()
    );
    assert!(is_return_pending);
    step_until(
        application,
        &window.context,
        "tab return resubmitted",
        |_, _| {
            let layouts = services.state.layouts.read();
            let layout = &layouts[&window.project];
            layout.auxiliary_windows.is_empty()
                && crate::tabs::tabs_in(&layout.root)
                    .iter()
                    .any(|tab| tab.id == window.notes.id)
        },
    );
    assert!(!is_window_open(application, &window.project, FIRST_SLOT));
}

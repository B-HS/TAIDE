use super::auxiliary_window_tests::{
    auxiliary_window, file_tab, focused_text_input, install_viewports, step_until, viewport_of,
};
use super::inlay_hints_tests::key;
use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};
use super::*;
use taide_model::project::ShellSlotTree;

const NEEDLE: &str = "needle";
const SCOPE_DIR: &str = "sub";
const MANY_LINES: usize = 2100;
const VISIBLE_ROW_LIMIT: usize = 100;
const BEFORE_DEBOUNCE_FRAMES: usize = 10;
const AUX_SLOT: u32 = 1;
const OPENED_LINE_OFFSET: usize = 9;
const TOOLTIP_CLOSE_FRAMES: usize = 40;
const MAIN_FILES: [(&str, &str); 3] = [
    ("a.txt", "needle one\nother\n"),
    ("b.txt", "x needle\nneedle y\n"),
    ("sub/c.txt", "deep needle\n"),
];

pub(super) struct Launched {
    pub(super) fixture: Fixture,
    pub(super) context: egui::Context,
    pub(super) project: ProjectId,
    pub(super) slot: ShellSlotId,
    pub(super) root: std::path::PathBuf,
}

pub(super) fn launch_project(name: &str, files: &[(&str, &str)], with_auxiliary: bool) -> Launched {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-{name}-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&data).unwrap();
    for (relative, text) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        layout.shell_view.sidebar_collapsed = false;
        if with_auxiliary {
            layout.auxiliary_windows = vec![auxiliary_window(
                AUX_SLOT,
                &file_tab(&root.join(files[0].0)),
            )];
        }
    }
    {
        let mut session = state.session.write();
        session.active_project = Some(project.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
        session.projects = vec![project_ref(&project, &root, "alpha")];
        session.focused_shell_slot = Some(slot.clone());
    }
    let context = launch(&mut fixture, state);
    Launched {
        fixture,
        context,
        project,
        slot,
        root,
    }
}

pub(super) fn texts(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

pub(super) fn role_rect(
    output: &egui::FullOutput,
    role: egui::accesskit::Role,
    label: &str,
) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == role && node.label() == Some(label))
        .find_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
}

pub(super) fn message(
    application: &NativeApplication,
    key: &str,
    arguments: &[(&str, &str)],
) -> String {
    presentation::message(&application.locale, key, arguments)
}

pub(super) fn click(
    application: &mut NativeApplication,
    context: &egui::Context,
    rect: egui::Rect,
) {
    step(
        application,
        context,
        vec![egui::Event::PointerMoved(rect.center())],
    );
    step(
        application,
        context,
        pointer(rect.center(), egui::PointerButton::Primary),
    );
}

fn open_search_view(application: &mut NativeApplication, context: &egui::Context) {
    let label = message(application, "search.title", &[]);
    let output = step(application, context, Vec::new());
    let tab = role_rect(&output, egui::accesskit::Role::Tab, &label).expect("search tab");
    click(application, context, tab);
}

fn search(application: &mut NativeApplication, context: &egui::Context, text: &str) {
    step(application, context, vec![egui::Event::Text(text.into())]);
    step(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
}

fn panel_generation(application: &NativeApplication, slot: &ShellSlotId) -> Option<u64> {
    application
        .explorer_search_state
        .symbol_sidebars
        .search(slot)
        .map(crate::search_panel::Panel::generation)
}

#[test]
fn 실제_app_검색_보기는_enter로_파일별_결과를_보이고_접기와_일치_열기와_이력과_범위를_처리한다() {
    let Launched {
        mut fixture,
        context,
        project,
        slot,
        root,
    } = launch_project("search-panel-main", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    open_search_view(application, &context);
    let hint = message(application, "search.pressEnterHint", &[]);
    step_until(application, &context, "hint", |_, output| {
        texts(output).contains(&hint)
    });

    search(application, &context, NEEDLE);
    let summary = message(
        application,
        "search.matchCount",
        &[("count", "4"), ("files", "3")],
    );
    let output = step_until(application, &context, "results", |_, output| {
        texts(output).contains(&summary)
    });
    for name in ["a.txt", "b.txt", "c.txt"] {
        assert!(texts(&output).contains(&name.to_owned()), "{name}");
    }
    step_until(application, &context, "history", |application, _| {
        application.services.state.settings.read().recent_searches == [NEEDLE.to_owned()]
    });

    let b_path = root.join("b.txt").to_str().unwrap().to_owned();
    let header = format!("b.txt {b_path} 2");
    let header_rect = node_rect(&output, &header, egui::Rect::EVERYTHING).expect("b.txt header");
    assert!(node_rect(&output, "1 x needle", egui::Rect::EVERYTHING).is_some());
    click(application, &context, header_rect);
    step_until(application, &context, "collapsed", |_, output| {
        node_rect(output, "1 x needle", egui::Rect::EVERYTHING).is_none()
            && node_rect(output, "1 needle one", egui::Rect::EVERYTHING).is_some()
    });
    click(application, &context, header_rect);
    let output = step_until(application, &context, "expanded", |_, output| {
        node_rect(output, "2 needle y", egui::Rect::EVERYTHING).is_some()
    });
    let row = node_rect(&output, "2 needle y", egui::Rect::EVERYTHING).unwrap();
    click(application, &context, row);
    step_until(application, &context, "preview tab", |application, _| {
        let layouts = application.services.state.layouts.read();
        let layout = &layouts[&project];
        taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane).is_some_and(
            |tab| tab.preview && matches!(&tab.kind, TabKind::File { path } if *path == b_path),
        )
    });
    let tab = {
        let layouts = application.services.state.layouts.read();
        let layout = &layouts[&project];
        taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane)
            .unwrap()
            .id
            .clone()
    };
    step_until(application, &context, "reveal", |application, _| {
        let Some(document) = application.files.get(&b_path) else {
            return false;
        };
        application
            .store
            .views()
            .for_document(document.id)
            .find(|view| view.key.tab == tab)
            .and_then(|view| application.store.views().get(view.id))
            .and_then(|state| {
                state
                    .selection
                    .selections
                    .get(state.selection.primary)
                    .cloned()
            })
            .is_some_and(|selection| selection.head == OPENED_LINE_OFFSET)
    });

    {
        let sidebar = application
            .explorer_search_state
            .symbol_sidebars
            .get_mut(&slot)
            .unwrap();
        sidebar.search_request = Some(crate::search_panel::Request {
            scope_dir: Some(SCOPE_DIR.into()),
            ..Default::default()
        });
        sidebar.open_nonce += 1;
    }
    let chip = message(
        application,
        "explorer.searchScopeLabel",
        &[("path", SCOPE_DIR)],
    );
    step_until(application, &context, "chip", |_, output| {
        texts(output).contains(&chip)
    });
    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let scoped = message(
        application,
        "search.matchCount",
        &[("count", "1"), ("files", "1")],
    );
    let output = step_until(application, &context, "scoped results", |_, output| {
        texts(output).contains(&scoped)
    });
    assert!(!texts(&output).contains(&"a.txt".to_owned()));
    let clear = message(application, "search.clearScope", &[]);
    let clear_rect = node_rect(&output, &clear, egui::Rect::EVERYTHING).expect("clear scope");
    click(application, &context, clear_rect);
    step_until(application, &context, "chip cleared", |_, output| {
        !texts(output).contains(&chip)
    });
    assert!(
        application
            .explorer_search_state
            .symbol_sidebars
            .get_mut(&slot)
            .and_then(|sidebar| sidebar.search_request.clone())
            .is_some_and(|request| request.scope_dir.is_none() && request.include_glob.is_none())
    );
}

#[test]
fn 실제_app_잘못된_정규식_enter는_토스트와_실패_본문을_보이고_보기를_벗어나면_실행을_끊고_빈_패널로_돌아온다()
 {
    let Launched {
        mut fixture,
        context,
        slot,
        ..
    } = launch_project("search-panel-failure", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    open_search_view(application, &context);
    let regex = message(application, "search.regex", &[]);
    let output = step_until(application, &context, "toggles", |_, output| {
        node_rect(output, &regex, egui::Rect::EVERYTHING).is_some()
    });
    click(
        application,
        &context,
        node_rect(&output, &regex, egui::Rect::EVERYTHING).unwrap(),
    );
    let placeholder = message(application, "search.placeholder", &[]);
    let output = step(application, &context, Vec::new());
    let input = role_rect(&output, egui::accesskit::Role::TextInput, &placeholder).unwrap();
    for _ in 0..TOOLTIP_CLOSE_FRAMES {
        step(
            application,
            &context,
            vec![egui::Event::PointerMoved(input.center())],
        );
    }
    click(application, &context, input);
    let errors = application.toasts.inspection().len();
    search(application, &context, "(foo");
    let failed = message(application, "search.failed", &[]);
    step_until(application, &context, "failure", |application, output| {
        texts(output).contains(&failed) && application.toasts.inspection().len() > errors
    });

    let streams = application.explorer_search_state.search_streams.clone();
    let run = application
        .explorer_search_state
        .symbol_sidebars
        .get_mut(&slot)
        .and_then(|sidebar| sidebar.search.as_mut())
        .unwrap()
        .begin_unsubmitted_run(&streams);
    let files = message(application, "explorer.title", &[]);
    let output = step(application, &context, Vec::new());
    let tab = role_rect(&output, egui::accesskit::Role::Tab, &files).unwrap();
    click(application, &context, tab);
    step_until(application, &context, "files view", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    assert!(panel_generation(application, &slot).is_none());
    assert!(!streams.push(
        &run.session,
        run.generation,
        taide_model::search::SearchFileMatches {
            path: "late.txt".into(),
            matches: Vec::new(),
        },
    ));

    open_search_view(application, &context);
    let hint = message(application, "search.pressEnterHint", &[]);
    step_until(
        application,
        &context,
        "empty panel",
        |application, output| {
            texts(output).contains(&hint)
                && !texts(output).contains(&failed)
                && panel_generation(application, &slot) == Some(0)
        },
    );
}

#[test]
fn 실제_app_입력_검색은_디바운스_뒤에_한_번_돌고_이력을_남기지_않는다() {
    let Launched {
        mut fixture,
        context,
        slot,
        ..
    } = launch_project("search-panel-live", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    open_search_view(application, &context);
    let hint = message(application, "search.liveSearchHint", &[]);
    step_until(application, &context, "live hint", |_, output| {
        texts(output).contains(&hint)
    });
    step(
        application,
        &context,
        vec![egui::Event::Text(NEEDLE.into())],
    );
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(panel_generation(application, &slot), Some(0));
    let summary = message(
        application,
        "search.matchCount",
        &[("count", "4"), ("files", "3")],
    );
    step_until(application, &context, "live results", |_, output| {
        texts(output).contains(&summary)
    });
    let generation = panel_generation(application, &slot);
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(panel_generation(application, &slot), generation);
    assert!(
        application
            .services
            .state
            .settings
            .read()
            .recent_searches
            .is_empty()
    );

    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    search(application, &context, " one");
    let expected = [format!("{NEEDLE} one"), NEEDLE.to_owned()];
    let settings_file = application.services.state.paths.settings_file();
    step_until(application, &context, "two recent searches", |_, _| {
        std::fs::read(&settings_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|settings| {
                serde_json::from_value::<Vec<String>>(settings["recentSearches"].clone()).ok()
            })
            .is_some_and(|recent| recent == expected)
    });
}

#[test]
fn 실제_app_2000행이_넘는_결과는_보이는_행과_여유분만_그린다() {
    let many = format!("{NEEDLE}\n").repeat(MANY_LINES);
    let Launched {
        mut fixture,
        context,
        ..
    } = launch_project("search-panel-many", &[("many.txt", &many)], false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "many.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    open_search_view(application, &context);
    search(application, &context, NEEDLE);
    let count = MANY_LINES.to_string();
    let summary = message(
        application,
        "search.matchCount",
        &[("count", &count), ("files", "1")],
    );
    let output = step_until(application, &context, "many results", |_, output| {
        texts(output).contains(&summary)
    });
    let rows = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter(|(_, node)| {
            node.label()
                .is_some_and(|label| label.ends_with(&format!(" {NEEDLE}")))
        })
        .count();
    assert!(rows > 0 && rows < VISIBLE_ROW_LIMIT, "{rows}");
}

#[test]
fn 실제_app_보조_창_검색_보기의_일치는_그_창의_pane에_preview로_열린다() {
    let Launched {
        mut fixture,
        context,
        project,
        root,
        ..
    } = launch_project("search-panel-auxiliary", &MAIN_FILES, true);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().search_on_type = false;
    let auxiliary = viewport_of(application, &project, AUX_SLOT);
    let aux_output = || viewports.borrow().outputs.get(&auxiliary).cloned();
    let title = message(application, "search.title", &[]);
    step_until(application, &context, "auxiliary tabs", |_, _| {
        aux_output()
            .is_some_and(|output| role_rect(&output, egui::accesskit::Role::Tab, &title).is_some())
    });
    let tab = role_rect(&aux_output().unwrap(), egui::accesskit::Role::Tab, &title).unwrap();
    viewports.borrow_mut().events.insert(
        auxiliary,
        pointer(tab.center(), egui::PointerButton::Primary),
    );
    let hint = message(application, "search.pressEnterHint", &[]);
    step_until(application, &context, "auxiliary panel", |_, _| {
        aux_output().is_some_and(|output| texts(&output).contains(&hint))
    });
    let placeholder = message(application, "search.placeholder", &[]);
    let input = role_rect(
        &aux_output().unwrap(),
        egui::accesskit::Role::TextInput,
        &placeholder,
    )
    .unwrap();
    viewports.borrow_mut().events.insert(
        auxiliary,
        pointer(input.center(), egui::PointerButton::Primary),
    );
    step(application, &context, Vec::new());
    viewports
        .borrow_mut()
        .events
        .insert(auxiliary, vec![egui::Event::Text(NEEDLE.into())]);
    step(application, &context, Vec::new());
    viewports
        .borrow_mut()
        .events
        .insert(auxiliary, key(egui::Key::Enter, egui::Modifiers::NONE));
    let summary = message(
        application,
        "search.matchCount",
        &[("count", "4"), ("files", "3")],
    );
    step_until(application, &context, "auxiliary results", |_, _| {
        aux_output().is_some_and(|output| texts(&output).contains(&summary))
    });
    let row = node_rect(&aux_output().unwrap(), "2 needle y", egui::Rect::EVERYTHING).unwrap();
    viewports.borrow_mut().events.insert(
        auxiliary,
        pointer(row.center(), egui::PointerButton::Primary),
    );
    let b_path = root.join("b.txt").to_str().unwrap().to_owned();
    let is_b =
        |tab: &Tab| tab.preview && matches!(&tab.kind, TabKind::File { path } if *path == b_path);
    step_until(application, &context, "auxiliary open", |application, _| {
        let layouts = application.services.state.layouts.read();
        layouts[&project]
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == AUX_SLOT)
            .and_then(|window| {
                taide_native_ui::snapshot::active_tab(&window.root, &window.focused_pane)
            })
            .is_some_and(is_b)
    });
    let layouts = application.services.state.layouts.read();
    let layout = &layouts[&project];
    assert!(
        !taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane)
            .is_some_and(is_b)
    );
}

const REPLACEMENT: &str = "pin";
const NOT_UTF8_FILE: &str = "latin.txt";
const NOT_UTF8_BYTES: &[u8] = b"needle \xff\n";
const CONTEXT_LINES: &str = "2";
const SEED_LINE: &str = "needle one";
const COMMAND_SHIFT: egui::Modifiers = egui::Modifiers {
    alt: false,
    ctrl: false,
    shift: true,
    mac_cmd: true,
    command: true,
};
const COMMAND: egui::Modifiers = egui::Modifiers {
    shift: false,
    ..COMMAND_SHIFT
};

fn active_tab(application: &NativeApplication, project: &ProjectId) -> Option<Tab> {
    let layouts = application.services.state.layouts.read();
    let layout = layouts.get(project)?;
    taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane).cloned()
}

fn sidebar_request(
    application: &NativeApplication,
    slot: &ShellSlotId,
) -> Option<crate::search_panel::Request> {
    application
        .explorer_search_state
        .symbol_sidebars
        .search_request(slot)
        .cloned()
}

fn editor_generation(application: &NativeApplication, tab: &TabId) -> Option<u64> {
    application
        .explorer_search_state
        .search_editors
        .get(tab)
        .map(crate::search_panel::Panel::generation)
}

fn rightmost_input(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| {
            node.role() == egui::accesskit::Role::TextInput && node.label() == Some(label)
        })
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .max_by(|left, right| left.left().total_cmp(&right.left()))
}

pub(super) fn settle_pointer(
    application: &mut NativeApplication,
    context: &egui::Context,
    at: egui::Pos2,
) {
    for _ in 0..TOOLTIP_CLOSE_FRAMES {
        step(application, context, vec![egui::Event::PointerMoved(at)]);
    }
}

#[test]
fn 실제_app_모두_바꾸기는_확인한_파일만_바꾸고_요약과_건너뜀_toast를_띄운_뒤_같은_질의로_다시_검색한다()
 {
    let Launched {
        mut fixture,
        context,
        root,
        ..
    } = launch_project("search-replace", &MAIN_FILES[..2], false);
    std::fs::write(root.join(NOT_UTF8_FILE), NOT_UTF8_BYTES).unwrap();
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    open_search_view(application, &context);
    search(application, &context, NEEDLE);
    let summary = message(
        application,
        "search.matchCount",
        &[("count", "4"), ("files", "3")],
    );
    let output = step_until(application, &context, "results", |_, output| {
        texts(output).contains(&summary)
    });

    let toggle = message(application, "search.replaceToggle", &[]);
    let toggle_rect = node_rect(&output, &toggle, egui::Rect::EVERYTHING).unwrap();
    settle_pointer(application, &context, toggle_rect.center());
    click(application, &context, toggle_rect);
    let placeholder = message(application, "search.replacePlaceholder", &[]);
    let output = step_until(application, &context, "replace input", |_, output| {
        role_rect(output, egui::accesskit::Role::TextInput, &placeholder).is_some()
    });
    let input = role_rect(&output, egui::accesskit::Role::TextInput, &placeholder).unwrap();
    settle_pointer(application, &context, input.center());
    click(application, &context, input);
    step(
        application,
        &context,
        vec![egui::Event::Text(REPLACEMENT.into())],
    );

    let a_path = root.join("a.txt");
    let b_path = root.join("b.txt");
    let select_b = message(
        application,
        "search.selectFile",
        &[("path", b_path.to_str().unwrap())],
    );
    let output = step(application, &context, Vec::new());
    click(
        application,
        &context,
        node_rect(&output, &select_b, egui::Rect::EVERYTHING).unwrap(),
    );
    let replace_all = message(application, "search.replaceAll", &[]);
    let description = message(
        application,
        "search.replaceConfirmDescription",
        &[("files", "2"), ("matches", "2")],
    );
    let output = step(application, &context, Vec::new());
    let replace_rect = node_rect(&output, &replace_all, egui::Rect::EVERYTHING).unwrap();
    click(application, &context, replace_rect);
    step_until(application, &context, "confirm", |_, output| {
        texts(output).contains(&description)
    });
    step(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    step_until(application, &context, "escape", |_, output| {
        !texts(output).contains(&description)
    });
    assert_eq!(std::fs::read_to_string(&a_path).unwrap(), MAIN_FILES[0].1);

    let toasts = application.toasts.inspection().len();
    click(application, &context, replace_rect);
    let confirm = message(application, "common.confirm", &[]);
    let output = step_until(application, &context, "confirm again", |_, output| {
        role_rect(output, egui::accesskit::Role::Button, &confirm).is_some()
    });
    click(
        application,
        &context,
        role_rect(&output, egui::accesskit::Role::Button, &confirm).unwrap(),
    );
    let done = message(
        application,
        "search.replaceDone",
        &[("files", "1"), ("matches", "1")],
    );
    let skipped = message(application, "search.replaceSkipped", &[("count", "1")]);
    let reason = message(application, "search.replaceSkipReason.notUtf8", &[]);
    step_until(application, &context, "replace toasts", |application, _| {
        let inspection = application.toasts.inspection();
        inspection.len() >= toasts + 2
            && inspection.iter().any(|toast| {
                toast.kind == taide_native_ui::toast::Kind::Success && toast.title == done
            })
            && inspection.iter().any(|toast| {
                toast.kind == taide_native_ui::toast::Kind::Warning
                    && toast.title == skipped
                    && toast.description.as_ref().is_some_and(|description| {
                        description.contains(NOT_UTF8_FILE) && description.ends_with(&reason)
                    })
            })
    });
    assert_eq!(
        std::fs::read_to_string(&a_path).unwrap(),
        MAIN_FILES[0].1.replacen(NEEDLE, REPLACEMENT, 1)
    );
    assert_eq!(std::fs::read_to_string(&b_path).unwrap(), MAIN_FILES[1].1);
    assert_eq!(
        std::fs::read(root.join(NOT_UTF8_FILE)).unwrap(),
        NOT_UTF8_BYTES
    );
    let rerun = message(
        application,
        "search.matchCount",
        &[("count", "3"), ("files", "2")],
    );
    step_until(application, &context, "re-run", |_, output| {
        texts(output).contains(&rerun)
    });
}

#[test]
fn 실제_app_검색_편집기_탭은_패널_입력으로_열려_한_번_실행하고_떠났다_돌아와도_입력과_결과를_유지한다()
 {
    let Launched {
        mut fixture,
        context,
        project,
        slot,
        root,
    } = launch_project("search-editor", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    open_search_view(application, &context);
    step(
        application,
        &context,
        vec![egui::Event::Text(NEEDLE.into())],
    );
    let open_editor = message(application, "searchEditor.title", &[]);
    let output = step(application, &context, Vec::new());
    let open_editor_rect = node_rect(&output, &open_editor, egui::Rect::EVERYTHING).unwrap();
    settle_pointer(application, &context, open_editor_rect.center());
    click(application, &context, open_editor_rect);
    step_until(application, &context, "editor tab", |application, _| {
        active_tab(application, &project).is_some_and(|tab| {
            !tab.preview
                && tab.title == NEEDLE
                && matches!(&tab.kind, TabKind::SearchEditor { query } if query.text == NEEDLE && query.context_lines == 0)
        })
    });
    let editor_tab = active_tab(application, &project).unwrap().id;
    let summary = message(
        application,
        "search.matchCount",
        &[("count", "4"), ("files", "3")],
    );
    step_until(application, &context, "auto run", |_, output| {
        texts(output).contains(&summary)
    });
    assert_eq!(
        panel_generation(application, &slot),
        Some(0),
        "the sidebar panel itself did not run"
    );
    assert!(
        application
            .services
            .state
            .settings
            .read()
            .recent_searches
            .is_empty()
    );

    let context_label = message(application, "searchEditor.contextLinesLabel", &[]);
    let output = step(application, &context, Vec::new());
    let context_input =
        role_rect(&output, egui::accesskit::Role::TextInput, &context_label).unwrap();
    settle_pointer(application, &context, context_input.center());
    click(application, &context, context_input);
    step(
        application,
        &context,
        vec![egui::Event::Text(CONTEXT_LINES.into())],
    );
    let placeholder = message(application, "search.placeholder", &[]);
    let output = step(application, &context, Vec::new());
    click(
        application,
        &context,
        rightmost_input(&output, &placeholder).unwrap(),
    );
    let before = editor_generation(application, &editor_tab);
    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    step_until(
        application,
        &context,
        "context lines",
        |application, output| {
            editor_generation(application, &editor_tab) != before
                && texts(output).contains(&summary)
                && texts(output).contains(&"other".to_owned())
        },
    );
    step_until(application, &context, "editor history", |application, _| {
        application.services.state.settings.read().recent_searches == [NEEDLE.to_owned()]
    });
    let generation = editor_generation(application, &editor_tab);

    let b_path = root.join("b.txt").to_str().unwrap().to_owned();
    let output = step(application, &context, Vec::new());
    click(
        application,
        &context,
        node_rect(&output, "2 needle y", egui::Rect::EVERYTHING).unwrap(),
    );
    step_until(application, &context, "match opened", |application, _| {
        active_tab(application, &project).is_some_and(|tab| {
            tab.preview && matches!(&tab.kind, TabKind::File { path } if *path == b_path)
        })
    });
    application
        .controller
        .submit(ShellMutation::ActivateTab(editor_tab.clone()))
        .unwrap();
    step_until(application, &context, "editor back", |_, output| {
        texts(output).contains(&summary) && texts(output).contains(&"other".to_owned())
    });
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(editor_generation(application, &editor_tab), generation);
    assert_eq!(
        application
            .explorer_search_state
            .search_editors
            .get_mut(&editor_tab)
            .map(|panel| panel.context_lines()),
        Some(2)
    );

    let files = message(application, "explorer.title", &[]);
    let output = step(application, &context, Vec::new());
    click(
        application,
        &context,
        role_rect(&output, egui::accesskit::Role::Tab, &files).unwrap(),
    );
    open_search_view(application, &context);
    let output = step(application, &context, Vec::new());
    let open_editor_rect = node_rect(&output, &open_editor, egui::Rect::EVERYTHING).unwrap();
    settle_pointer(application, &context, open_editor_rect.center());
    click(application, &context, open_editor_rect);
    step_until(
        application,
        &context,
        "empty editor tab",
        |application, _| {
            active_tab(application, &project).is_some_and(|tab| {
                tab.title == open_editor
                    && matches!(&tab.kind, TabKind::SearchEditor { query } if query.text.is_empty())
            })
        },
    );
    let empty_tab = active_tab(application, &project).unwrap().id;
    let hint = message(application, "search.pressEnterHint", &[]);
    step_until(
        application,
        &context,
        "empty editor body",
        |application, output| {
            editor_generation(application, &empty_tab).is_some()
                && texts(output).iter().filter(|text| **text == hint).count() >= 1
        },
    );
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(editor_generation(application, &empty_tab), Some(0));
}

#[test]
fn 실제_app_명령_shift_f는_편집기의_한_줄_선택을_검색어로_넣고_전체_선택하며_바꾸기와_접힘과_범위와_명령_f를_처리한다()
 {
    let Launched {
        mut fixture,
        context,
        project,
        slot,
        root,
    } = launch_project("search-entry", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().search_on_type = false;
    let output = step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    click(
        application,
        &context,
        node_rect(&output, "a.txt", egui::Rect::EVERYTHING).unwrap(),
    );
    let a_path = root.join("a.txt").to_str().unwrap().to_owned();
    step_until(application, &context, "a.txt view", |application, _| {
        active_tab(application, &project)
            .is_some_and(|tab| matches!(&tab.kind, TabKind::File { path } if *path == a_path))
            && application.files.get(&a_path).is_some_and(|document| {
                application
                    .store
                    .views()
                    .for_document(document.id)
                    .next()
                    .is_some()
            })
    });
    let view = {
        let document = application.files.get(&a_path).unwrap().id;
        application
            .store
            .views()
            .for_document(document)
            .next()
            .unwrap()
            .id
    };
    let focus_editor = |application: &mut NativeApplication| {
        let body = crate::editor_context_menu::body(&context, view).unwrap();
        context.memory_mut(|memory| memory.request_focus(body));
        step(application, &context, Vec::new());
    };
    let select = |application: &mut NativeApplication, key_name: egui::Key| {
        step(application, &context, key(key_name, egui::Modifiers::SHIFT));
        step(application, &context, Vec::new());
    };
    focus_editor(application);
    select(application, egui::Key::End);
    step(application, &context, key(egui::Key::F, COMMAND_SHIFT));
    let output = step_until(application, &context, "seeded", |_, output| {
        focused_text_input(output)
            .is_some_and(|(value, range, _)| value == SEED_LINE && range == [0, SEED_LINE.len()])
    });
    assert_eq!(
        application
            .explorer_search_state
            .symbol_sidebars
            .view(&project, &slot),
        crate::symbol_sidebar::View::Search
    );
    let toggle = message(application, "search.replaceToggle", &[]);
    assert!(
        !output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some(toggle.as_str())
                && node.toggled() == Some(egui::accesskit::Toggled::True))
    );

    focus_editor(application);
    select(application, egui::Key::ArrowDown);
    step(application, &context, key(egui::Key::F, COMMAND_SHIFT));
    step_until(application, &context, "multi-line", |application, _| {
        sidebar_request(application, &slot)
            .is_some_and(|request| request.seed_text.is_none() && !request.open_replace)
    });

    step(application, &context, key(egui::Key::H, COMMAND_SHIFT));
    step_until(application, &context, "replace", |application, output| {
        sidebar_request(application, &slot).is_some_and(|request| request.open_replace)
            && output
                .platform_output
                .accesskit_update
                .as_ref()
                .is_some_and(|update| {
                    update.nodes.iter().any(|(_, node)| {
                        node.label() == Some(toggle.as_str())
                            && node.toggled() == Some(egui::accesskit::Toggled::True)
                    })
                })
    });

    application
        .explorer_search_state
        .symbol_sidebars
        .entry(&project, &slot)
        .request_search(crate::search_panel::Request {
            scope_dir: Some(SCOPE_DIR.into()),
            ..Default::default()
        });
    let chip = message(
        application,
        "explorer.searchScopeLabel",
        &[("path", SCOPE_DIR)],
    );
    step_until(application, &context, "scoped", |_, output| {
        texts(output).contains(&chip)
    });
    application
        .controller
        .submit(ShellMutation::SetSidebarCollapsed {
            project: project.clone(),
            collapsed: true,
        })
        .unwrap();
    step_until(application, &context, "collapsed", |application, _| {
        application.services.state.layouts.read()[&project]
            .shell_view
            .sidebar_collapsed
    });
    step(application, &context, key(egui::Key::F, COMMAND_SHIFT));
    step_until(
        application,
        &context,
        "expanded and unscoped",
        |application, output| {
            !application.services.state.layouts.read()[&project]
                .shell_view
                .sidebar_collapsed
                && !texts(output).contains(&chip)
                && sidebar_request(application, &slot)
                    .is_some_and(|request| request.scope_dir.is_none())
        },
    );

    let files = message(application, "explorer.title", &[]);
    let show_files = |application: &mut NativeApplication| {
        let output = step(application, &context, Vec::new());
        let tab = role_rect(&output, egui::accesskit::Role::Tab, &files).unwrap();
        settle_pointer(application, &context, tab.center());
        click(application, &context, tab);
        step_until(application, &context, "files view", |application, _| {
            application
                .explorer_search_state
                .symbol_sidebars
                .view(&project, &slot)
                == crate::symbol_sidebar::View::Files
        });
    };
    show_files(application);
    step(application, &context, key(egui::Key::F, COMMAND));
    step_until(
        application,
        &context,
        "find opens search",
        |application, _| {
            application
                .explorer_search_state
                .symbol_sidebars
                .view(&project, &slot)
                == crate::symbol_sidebar::View::Search
        },
    );
    show_files(application);
    focus_editor(application);
    step(application, &context, key(egui::Key::F, COMMAND));
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(
        application
            .explorer_search_state
            .symbol_sidebars
            .view(&project, &slot),
        crate::symbol_sidebar::View::Files
    );

    let body = crate::editor_context_menu::body(&context, view).unwrap();
    let is_find_input_focused = |application: &NativeApplication, output: &egui::FullOutput| {
        application
            .editor_find
            .get(&view)
            .is_some_and(|find| find.visible)
            && focused_text_input(output).is_some()
            && context.memory(|memory| memory.focused()) != Some(body)
    };
    step_until(
        application,
        &context,
        "editor find input",
        &is_find_input_focused,
    );
    step(application, &context, key(egui::Key::F, COMMAND));
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    let output = step(application, &context, Vec::new());
    assert_eq!(
        application
            .explorer_search_state
            .symbol_sidebars
            .view(&project, &slot),
        crate::symbol_sidebar::View::Files
    );
    assert!(is_find_input_focused(application, &output));
}

#[test]
fn 실제_app_ai_편집_입력에_focus가_있을_때_명령_f는_편집기_찾기를_연다() {
    let mut fixture = super::inlay_hints_tests::fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        SEED_LINE,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = super::inlay_hints_tests::body_view(application, &fixture.path, &fixture.tab);
    {
        let mut settings = application.services.state.settings.write();
        settings.ai_provider = Some(taide_model::ai::AiProviderId::Omlx);
        settings.ai_model = Some("fixture".into());
    }
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    super::inlay_hints_tests::paint(application, context, key(egui::Key::I, COMMAND));
    super::inlay_hints_tests::wait(application, context, "AI edit control focus", |_| {
        context.memory(|memory| {
            memory.focused().is_some_and(|focus| {
                taide_native_ui::editor_ai_edit::is_control(context, body, focus)
            })
        })
    });
    super::inlay_hints_tests::paint(application, context, key(egui::Key::F, COMMAND));
    super::inlay_hints_tests::wait(application, context, "editor find", |application| {
        application
            .editor_find
            .get(&view)
            .is_some_and(|find| find.visible)
    });
}

#[test]
fn 실제_app_바꾸기를_시작하지_않은_새_패널은_늦게_온_바꾸기_응답을_무시한다() {
    let Launched {
        mut fixture,
        context,
        project,
        slot,
        ..
    } = launch_project("search-replace-stale", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    open_search_view(application, &context);
    let hint = message(application, "search.pressEnterHint", &[]);
    step_until(application, &context, "hint", |_, output| {
        texts(output).contains(&hint)
    });
    let generation = panel_generation(application, &slot);
    application.apply_explorer_search_reply(
        &context,
        crate::domains::explorer_search::Reply::SearchReplaced {
            mount: slot.clone(),
            project,
            query: serde_json::from_value(serde_json::json!({ "text": NEEDLE })).unwrap(),
            result: Ok(taide_model::search::SearchReplaceResult {
                changed_files: 1,
                replaced_matches: 1,
                skipped: Vec::new(),
                skipped_count: 0,
            }),
        },
    );
    for _ in 0..BEFORE_DEBOUNCE_FRAMES {
        step(application, &context, Vec::new());
    }
    let summary = message(
        application,
        "search.replaceDone",
        &[("files", "1"), ("matches", "1")],
    );
    assert!(
        !application
            .toasts
            .is_showing(taide_native_ui::toast::Kind::Success, &summary)
    );
    assert_eq!(panel_generation(application, &slot), generation);
}

#[test]
fn 실제_app_검색_이력_저장이_실패하면_설정_저장_실패_toast를_띄우고_대기_목록을_비운다() {
    let Launched {
        mut fixture,
        context,
        ..
    } = launch_project("search-history-failure", &MAIN_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, "a.txt", egui::Rect::EVERYTHING).is_some()
    });
    application.services.state.settings.write().search_on_type = false;
    let settings_file = application.services.state.paths.settings_file();
    if settings_file.exists() {
        std::fs::rename(&settings_file, settings_file.with_extension("previous")).unwrap();
    }
    std::fs::create_dir(&settings_file).unwrap();
    open_search_view(application, &context);
    search(application, &context, NEEDLE);
    let failed = message(application, "settings.saveFailed", &[]);
    step_until(
        application,
        &context,
        "settings failed",
        |application, _| {
            application
                .toasts
                .is_showing(taide_native_ui::toast::Kind::Error, &failed)
                && application
                    .explorer_search_state
                    .search_history
                    .list(&[])
                    .is_empty()
        },
    );
    assert!(
        application
            .services
            .state
            .settings
            .read()
            .recent_searches
            .is_empty()
    );
}

#[test]
fn 실제_app_보조_창의_명령_shift_f는_그_창_탐색기의_검색_보기만_연다() {
    let Launched {
        mut fixture,
        context,
        project,
        slot,
        ..
    } = launch_project("search-entry-auxiliary", &MAIN_FILES, true);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().search_on_type = false;
    let auxiliary = viewport_of(application, &project, AUX_SLOT);
    let title = message(application, "search.title", &[]);
    step_until(application, &context, "auxiliary tabs", |_, _| {
        viewports
            .borrow()
            .outputs
            .get(&auxiliary)
            .is_some_and(|output| role_rect(output, egui::accesskit::Role::Tab, &title).is_some())
    });
    let explorer = application
        .shell_state
        .auxiliary_windows
        .find(&project, AUX_SLOT)
        .unwrap()
        .explorer
        .clone();
    viewports
        .borrow_mut()
        .events
        .insert(auxiliary, key(egui::Key::F, COMMAND_SHIFT));
    let hint = message(application, "search.pressEnterHint", &[]);
    step_until(
        application,
        &context,
        "auxiliary search",
        |application, _| {
            application
                .explorer_search_state
                .symbol_sidebars
                .view(&project, &explorer)
                == crate::symbol_sidebar::View::Search
                && viewports
                    .borrow()
                    .outputs
                    .get(&auxiliary)
                    .is_some_and(|output| texts(output).contains(&hint))
        },
    );
    assert_ne!(
        application
            .explorer_search_state
            .symbol_sidebars
            .view(&project, &slot),
        crate::symbol_sidebar::View::Search
    );
}

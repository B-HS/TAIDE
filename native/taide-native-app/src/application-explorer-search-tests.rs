use super::auxiliary_window_tests::{
    auxiliary_window, file_tab, install_viewports, step_until, viewport_of,
};
use super::inlay_hints_tests::key;
use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};
use super::*;
use taide_model::layout::PaneNode;
use taide_model::project::{ProjectRef, ShellSlotTree};

const AUX_SLOT: u32 = 1;
const MAIN_FILE: &str = "a.txt";
const AUX_FILE: &str = "b.txt";
const CREATED_FILE: &str = "c.txt";
const SETTLE_FRAMES: usize = 8;

fn directories(name: &str) -> (Fixture, std::path::PathBuf, std::path::PathBuf) {
    let fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-{name}-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&data).unwrap();
    (fixture, data, root)
}

fn open_in_slot(state: &AppState, projects: Vec<ProjectRef>, slot: &ShellSlotId) {
    let mut session = state.session.write();
    session.active_project = projects.first().map(|project| project.id.clone());
    session.shell_slots = projects.first().map(|project| ShellSlotTree::Leaf {
        slot_id: slot.clone(),
        project_id: project.id.clone(),
    });
    session.projects = projects;
    session.focused_shell_slot = Some(slot.clone());
}

fn texts(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn settle(application: &mut NativeApplication, context: &egui::Context) -> egui::FullOutput {
    (1..SETTLE_FRAMES).for_each(|_| {
        step(application, context, Vec::new());
    });
    step(application, context, Vec::new())
}

#[test]
fn 실제_app_보조_창은_problems를_그리지_않고_탐색기_선택과_새_파일은_그_mount에만_남는다() {
    let (mut fixture, data, root) = directories("explorer-mounts");
    std::fs::create_dir_all(&root).unwrap();
    for name in [MAIN_FILE, AUX_FILE] {
        std::fs::write(root.join(name), "synthetic\n").unwrap();
    }
    let main_path = root.join(MAIN_FILE).to_str().unwrap().to_owned();
    let aux_path = root.join(AUX_FILE).to_str().unwrap().to_owned();
    let created_path = root.join(CREATED_FILE).to_str().unwrap().to_owned();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&alpha).unwrap();
        layout.shell_view.sidebar_collapsed = false;
        layout.auxiliary_windows =
            vec![auxiliary_window(AUX_SLOT, &file_tab(&root.join(AUX_FILE)))];
    }
    open_in_slot(&state, vec![project_ref(&alpha, &root, "alpha")], &slot);
    let context = launch(&mut fixture, state);
    let viewports = install_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let auxiliary = viewport_of(application, &alpha, AUX_SLOT);
    let aux_mount = application
        .shell_state
        .auxiliary_windows
        .find(&alpha, AUX_SLOT)
        .unwrap()
        .explorer
        .clone();
    let selected = |application: &NativeApplication, mount: &ShellSlotId| {
        application
            .explorer_search_state
            .explorers
            .get(mount)
            .and_then(|explorer| explorer.selected.clone())
    };
    step_until(application, &context, "both explorers", |_, output| {
        node_rect(output, MAIN_FILE, egui::Rect::EVERYTHING).is_some()
            && viewports
                .borrow()
                .outputs
                .get(&auxiliary)
                .is_some_and(|output| {
                    node_rect(output, MAIN_FILE, egui::Rect::EVERYTHING).is_some()
                })
    });

    application.problems.toggle(&slot);
    let title = presentation::message(&application.locale, "problems.title", &[]);
    let toggle = presentation::message(&application.locale, "problems.toggleAriaLabel", &[]);
    let main_output = step_until(application, &context, "main problems", |_, output| {
        texts(output).contains(&title)
    });
    assert!(node_rect(&main_output, &toggle, egui::Rect::EVERYTHING).is_some());
    let auxiliary_texts = viewports.borrow().texts(auxiliary);
    assert!(!auxiliary_texts.contains(&title), "{auxiliary_texts:?}");
    assert!(
        viewports
            .borrow()
            .outputs
            .get(&auxiliary)
            .is_some_and(|output| node_rect(output, &toggle, egui::Rect::EVERYTHING).is_none())
    );

    let row = node_rect(&main_output, MAIN_FILE, egui::Rect::EVERYTHING).unwrap();
    step(
        application,
        &context,
        pointer(row.center(), egui::PointerButton::Primary),
    );
    step_until(application, &context, "main selection", |application, _| {
        selected(application, &slot).as_deref() == Some(main_path.as_str())
            && selected(application, &aux_mount).as_deref() == Some(aux_path.as_str())
    });
    application
        .services
        .state
        .settings
        .write()
        .explorer_auto_reveal = false;

    let new_file = presentation::message(&application.locale, "explorer.newFile", &[]);
    let button = viewports
        .borrow()
        .outputs
        .get(&auxiliary)
        .and_then(|output| node_rect(output, &new_file, egui::Rect::EVERYTHING))
        .unwrap();
    viewports.borrow_mut().events.insert(
        auxiliary,
        pointer(button.center(), egui::PointerButton::Primary),
    );
    step_until(
        application,
        &context,
        "auxiliary draft",
        |application, _| {
            application
                .explorer_search_state
                .explorers
                .get(&aux_mount)
                .is_some_and(|explorer| explorer.create.is_some())
        },
    );
    assert!(
        application
            .explorer_search_state
            .explorers
            .get(&slot)
            .is_some_and(|explorer| explorer.create.is_none())
    );
    viewports
        .borrow_mut()
        .events
        .insert(auxiliary, vec![egui::Event::Text(CREATED_FILE.into())]);
    step(application, &context, Vec::new());
    viewports
        .borrow_mut()
        .events
        .insert(auxiliary, key(egui::Key::Enter, egui::Modifiers::NONE));
    step_until(
        application,
        &context,
        "auxiliary create",
        |application, _| {
            selected(application, &aux_mount).as_deref() == Some(created_path.as_str())
        },
    );
    assert!(std::path::Path::new(&created_path).is_file());
    assert_eq!(selected(application, &slot), Some(main_path));

    application.forget_explorer_search_project(&alpha);
    assert_eq!(
        application
            .explorer_search_state
            .explorers
            .of_project(&alpha)
            .count(),
        0
    );
    assert!(!application.explorer_search_state.trees.contains_key(&alpha));
}

#[test]
fn 실제_app_루트가_없는_프로젝트는_누락_안내와_다시_열기를_그리고_트리를_반복_요청하지_않는다() {
    let (mut fixture, data, root) = directories("explorer-root-missing");
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", true);
    state
        .layouts
        .write()
        .get_mut(&alpha)
        .unwrap()
        .shell_view
        .sidebar_collapsed = false;
    let mut missing = project_ref(&alpha, &root, "alpha");
    missing.root_missing = true;
    open_in_slot(&state, vec![missing], &slot);
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let message = presentation::message(&application.locale, "explorer.projectRootMissing", &[]);
    let reopen = presentation::message(&application.locale, "explorer.reopenProject", &[]);
    let root_text = root.to_str().unwrap().to_owned();
    step_until(application, &context, "missing screen", |_, output| {
        texts(output).contains(&message)
    });
    let output = settle(application, &context);
    let shown = texts(&output);
    assert!(
        shown.contains(&message) && shown.contains(&root_text),
        "{shown:?}"
    );
    assert!(
        !application
            .explorer_search_state
            .tree_loading
            .contains(&alpha)
    );
    assert!(!application.explorer_search_state.trees.contains_key(&alpha));
    let button = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(reopen.as_str()))
        .map(|(_, node)| node.clone())
        .unwrap();
    assert_eq!(button.role(), egui::accesskit::Role::Button);
    let errors = application.toasts.inspection().len();
    let bounds = button.bounds().unwrap();
    step(
        application,
        &context,
        pointer(
            egui::pos2(
                ((bounds.x0 + bounds.x1) / 2.0) as f32,
                ((bounds.y0 + bounds.y1) / 2.0) as f32,
            ),
            egui::PointerButton::Primary,
        ),
    );
    step_until(
        application,
        &context,
        "closed and reopen failed",
        |application, _| {
            !application
                .services
                .state
                .projects
                .read()
                .contains_key(&alpha)
                && application.toasts.inspection().len() > errors
        },
    );
}

#[test]
fn 실제_app_루트_플래그만_켜진_프로젝트는_다시_열기로_새_id의_트리를_보인다() {
    let (mut fixture, data, root) = directories("explorer-reopen");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(MAIN_FILE), "synthetic\n").unwrap();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", true);
    let mut flagged = project_ref(&alpha, &root, "alpha");
    flagged.root_missing = true;
    open_in_slot(&state, vec![flagged], &slot);
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let reopen = presentation::message(&application.locale, "explorer.reopenProject", &[]);
    let output = step_until(application, &context, "reopen button", |_, output| {
        node_rect(output, &reopen, egui::Rect::EVERYTHING).is_some()
    });
    let button = node_rect(&output, &reopen, egui::Rect::EVERYTHING).unwrap();
    step(
        application,
        &context,
        pointer(button.center(), egui::PointerButton::Primary),
    );
    let root_text = std::fs::canonicalize(&root)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    step_until(application, &context, "closed", |application, _| {
        !application
            .services
            .state
            .projects
            .read()
            .contains_key(&alpha)
    });
    step_until(application, &context, "reopened", |application, _| {
        application
            .services
            .state
            .projects
            .read()
            .values()
            .any(|project| project.root == root_text)
    });
    step_until(application, &context, "reopened tree", |_, output| {
        node_rect(output, MAIN_FILE, egui::Rect::EVERYTHING).is_some()
    });
}

#[test]
fn 실제_app_트리_조회_실패는_누락_화면으로_멈추고_watcher_갱신_성공에_복귀하며_툴바_실패는_표시하지_않는다()
 {
    let (mut fixture, data, root) = directories("explorer-tree-failure");
    std::fs::create_dir_all(root.parent().unwrap()).unwrap();
    std::fs::write(&root, "not a directory\n").unwrap();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    state
        .layouts
        .write()
        .get_mut(&alpha)
        .unwrap()
        .shell_view
        .sidebar_collapsed = false;
    open_in_slot(&state, vec![project_ref(&alpha, &root, "alpha")], &slot);
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let message = presentation::message(&application.locale, "explorer.projectRootMissing", &[]);
    step_until(
        application,
        &context,
        "tree failure",
        |application, output| {
            application
                .explorer_search_state
                .tree_failures
                .contains(&alpha)
                && texts(output).contains(&message)
        },
    );
    let output = settle(application, &context);
    assert!(
        !application
            .explorer_search_state
            .tree_loading
            .contains(&alpha)
    );
    let errors = application.toasts.inspection().len();
    application.status = None;
    let refresh = presentation::message(&application.locale, "explorer.refresh", &[]);
    let button = node_rect(&output, &refresh, egui::Rect::EVERYTHING).unwrap();
    step(
        application,
        &context,
        pointer(button.center(), egui::PointerButton::Primary),
    );
    settle(application, &context);
    assert!(application.status.is_none());
    assert_eq!(application.toasts.inspection().len(), errors);
    assert!(
        application
            .explorer_search_state
            .tree_failures
            .contains(&alpha)
    );

    std::fs::remove_file(&root).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(MAIN_FILE), "synthetic\n").unwrap();
    application
        .explorer_search_state
        .tree_changes
        .merge(alpha.clone(), None);
    step_until(
        application,
        &context,
        "tree restored",
        |application, output| {
            !application
                .explorer_search_state
                .tree_failures
                .contains(&alpha)
                && node_rect(output, MAIN_FILE, egui::Rect::EVERYTHING).is_some()
        },
    );
}

#[test]
fn 실제_app_활성_파일_자동_reveal은_설정과_사이드바_보임을_따르고_조용히_실패한다() {
    let (mut fixture, data, root) = directories("explorer-auto-reveal");
    let deep = root.join("src/deep");
    std::fs::create_dir_all(&deep).unwrap();
    for path in [deep.join("a.txt"), root.join("b.txt"), root.join("c.txt")] {
        std::fs::write(path, "synthetic\n").unwrap();
    }
    let path_of = |path: std::path::PathBuf| path.to_str().unwrap().to_owned();
    let [deep_path, b_path, c_path] =
        [deep.join("a.txt"), root.join("b.txt"), root.join("c.txt")].map(path_of);
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&alpha).unwrap();
        let pane = PaneId::new();
        let tab = file_tab(&deep.join("a.txt"));
        layout.root = PaneNode::Leaf {
            id: pane.clone(),
            tabs: vec![tab.clone()],
            active: Some(tab.id),
        };
        layout.focused_pane = pane;
        layout.shell_view.sidebar_collapsed = false;
    }
    open_in_slot(&state, vec![project_ref(&alpha, &root, "alpha")], &slot);
    let context = launch(&mut fixture, state);
    let services = fixture.application.as_ref().unwrap().services.clone();
    let application = fixture.application.as_mut().unwrap();
    let selected = |application: &NativeApplication| {
        application
            .explorer_search_state
            .explorers
            .get(&slot)
            .and_then(|explorer| explorer.selected.clone())
    };
    let tree = egui::Id::new(("native-tree-focus", &alpha));
    step_until(
        application,
        &context,
        "reveal active file",
        |application, _| {
            selected(application).as_deref() == Some(deep_path.as_str())
                && application
                    .explorer_search_state
                    .trees
                    .get(&alpha)
                    .is_some_and(|page| page.rows.iter().any(|row| row.path == deep_path))
        },
    );
    assert!(!context.memory(|memory| memory.has_focus(tree)));

    let open = |application: &mut NativeApplication, path: &str| {
        assert!(
            application.submit(HostCommand::Editor(domains::editor::Command::OpenFileTab {
                project: alpha.clone(),
                pane: None,
                path: path.to_owned(),
                preview: false,
            }))
        );
    };
    let is_active = |path: &str| {
        let layouts = services.state.layouts.read();
        let layout = &layouts[&alpha];
        taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane).is_some_and(
            |tab| matches!(&tab.kind, TabKind::File { path: active } if active == path),
        )
    };
    services.state.settings.write().explorer_auto_reveal = false;
    open(application, &b_path);
    step_until(application, &context, "b active", |_, _| is_active(&b_path));
    settle(application, &context);
    assert_eq!(selected(application), Some(deep_path.clone()));
    services.state.settings.write().explorer_auto_reveal = true;
    step_until(application, &context, "select only b", |application, _| {
        selected(application).as_deref() == Some(b_path.as_str())
    });

    application
        .controller
        .submit(ShellMutation::SetSidebarCollapsed {
            project: alpha.clone(),
            collapsed: true,
        })
        .unwrap();
    settle(application, &context);
    open(application, &c_path);
    step_until(application, &context, "c active", |_, _| is_active(&c_path));
    settle(application, &context);
    assert_eq!(selected(application), Some(b_path.clone()));
    application
        .controller
        .submit(ShellMutation::SetSidebarCollapsed {
            project: alpha.clone(),
            collapsed: false,
        })
        .unwrap();
    step_until(
        application,
        &context,
        "reveal after expand",
        |application, _| selected(application).as_deref() == Some(c_path.as_str()),
    );

    let missing = root
        .join("missing-dir/missing.txt")
        .to_str()
        .unwrap()
        .to_owned();
    let errors = application.toasts.inspection().len();
    application.status = None;
    let reveal = |application: &mut NativeApplication, quiet: bool| {
        application.submit(HostCommand::ExplorerSearch(
            domains::explorer_search::Command::RevealExplorerPath {
                project: alpha.clone(),
                mount: slot.clone(),
                path: missing.clone(),
                rename: false,
                quiet,
            },
        ));
    };
    reveal(application, true);
    settle(application, &context);
    assert!(application.status.is_none(), "{:?}", application.status);
    assert_eq!(application.toasts.inspection().len(), errors);
    reveal(application, false);
    step_until(
        application,
        &context,
        "loud reveal failure",
        |application, _| application.status.is_some(),
    );
}

const MENU_FILES: [(&str, &str); 3] = [
    ("note.txt", "needle top\n"),
    ("page.html", "<p></p>\n"),
    ("node_modules/pkg/inner.txt", "needle inner\n"),
];
const IGNORED_FOLDER: &str = "node_modules";

fn terminal_cwds(application: &NativeApplication, project: &ProjectId) -> Vec<String> {
    let layouts = application.services.state.layouts.read();
    crate::tabs::tabs_in(&layouts[project].root)
        .into_iter()
        .filter_map(|tab| match &tab.kind {
            TabKind::Terminal { cwd, .. } if !tab.preview => cwd.clone(),
            _ => None,
        })
        .collect()
}

#[test]
fn 실제_app_터미널에서_열기는_그_폴더를_cwd로_한_고정_터미널_탭을_열고_폴더에서_찾기_요청은_기본_제외_폴더_안만_검색한다()
 {
    use super::search_panel_tests::{
        Launched, click, launch_project, message, role_rect, settle_pointer,
    };
    let Launched {
        mut fixture,
        context,
        project,
        slot,
        root,
    } = launch_project("explorer-menu", &MENU_FILES, false);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().search_on_type = false;
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, IGNORED_FOLDER, egui::Rect::EVERYTHING).is_some()
    });
    let folder_text = root.join(IGNORED_FOLDER).to_str().unwrap().to_owned();
    let title = message(application, "terminal.title", &[]);
    let before = terminal_cwds(application, &project).len();
    application.submit(HostCommand::ExplorerSearch(
        domains::explorer_search::Command::OpenTerminalAt {
            project: project.clone(),
            pane: None,
            cwd: folder_text.clone(),
            title: title.clone(),
        },
    ));
    step_until(
        application,
        &context,
        "folder terminal",
        |application, _| {
            let count = terminal_cwds(application, &project).len();
            let layouts = application.services.state.layouts.read();
            let layout = &layouts[&project];
            count == before + 1
            && taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane)
                .is_some_and(|tab| {
                    !tab.preview
                        && tab.title == title
                        && matches!(&tab.kind, TabKind::Terminal { session_id, cwd } if session_id.is_empty() && cwd.as_deref() == Some(folder_text.as_str()))
                })
        },
    );

    application
        .explorer_search_state
        .symbol_sidebars
        .entry(&project, &slot)
        .request_search(crate::search_panel::Request {
            scope_dir: Some(IGNORED_FOLDER.into()),
            ..Default::default()
        });
    let chip = message(
        application,
        "explorer.searchScopeLabel",
        &[("path", IGNORED_FOLDER)],
    );
    step_until(
        application,
        &context,
        "scoped search",
        |application, output| {
            application
                .explorer_search_state
                .symbol_sidebars
                .view(&project, &slot)
                == crate::symbol_sidebar::View::Search
                && texts(output).contains(&chip)
        },
    );
    step(
        application,
        &context,
        vec![egui::Event::Text("needle".into())],
    );
    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let one = message(
        application,
        "search.matchCount",
        &[("count", "1"), ("files", "1")],
    );
    let output = step_until(application, &context, "scoped results", |_, output| {
        texts(output).contains(&one) && texts(output).contains(&"inner.txt".to_owned())
    });
    assert!(!texts(&output).contains(&"note.txt".to_owned()));
    let clear = message(application, "search.clearScope", &[]);
    click(
        application,
        &context,
        node_rect(&output, &clear, egui::Rect::EVERYTHING).unwrap(),
    );
    let placeholder = message(application, "search.placeholder", &[]);
    let output = step_until(application, &context, "unscoped", |_, output| {
        !texts(output).contains(&chip)
    });
    let input = role_rect(&output, egui::accesskit::Role::TextInput, &placeholder).unwrap();
    settle_pointer(application, &context, input.center());
    click(application, &context, input);
    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    step_until(application, &context, "unscoped results", |_, output| {
        texts(output).contains(&one)
            && texts(output).contains(&"note.txt".to_owned())
            && !texts(output).contains(&"inner.txt".to_owned())
    });
}

#[test]
fn 실제_app_삭제_뒤_부모_갱신이_실패하면_오류_toast로_알린다() {
    use super::search_panel_tests::{Launched, launch_project};
    let Launched {
        mut fixture,
        context,
        project,
        root,
        ..
    } = launch_project("explorer-delete-refresh", &[(MAIN_FILE, "a\n")], false);
    let application = fixture.application.as_mut().unwrap();
    step_until(application, &context, "tree", |_, output| {
        node_rect(output, MAIN_FILE, egui::Rect::EVERYTHING).is_some()
    });
    std::fs::remove_dir_all(&root).unwrap();
    settle(application, &context);
    let errors = |application: &NativeApplication| {
        application
            .toasts
            .inspection()
            .iter()
            .filter(|toast| toast.kind == taide_native_ui::toast::Kind::Error)
            .count()
    };
    let before = errors(application);
    application.apply_explorer_search_lsp_reply(crate::lsp::Reply::ExplorerDeleteFinished {
        request: crate::explorer_delete::Request {
            project,
            path: root.join(MAIN_FILE).to_str().unwrap().to_owned(),
            name: MAIN_FILE.into(),
        },
        result: Ok(Vec::new()),
    });
    step_until(
        application,
        &context,
        "refresh failure toast",
        |application, _| errors(application) > before,
    );
}

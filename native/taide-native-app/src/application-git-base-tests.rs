use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::git::GitChangeKind;
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::layout::{PaneNode, SplitDir};
use taide_model::paths::AppPaths;
use taide_model::project::ShellSlotTree;
use taide_runtime::AppState;

use super::fixture;
use crate::application::NativeApplication;
use crate::application::auxiliary_window_tests::{
    auxiliary_window, file_tab, install_viewports, viewport_of,
};
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, pointer, project_ref,
};
use crate::presentation;

const DEADLINE: Duration = Duration::from_secs(20);
const SCREEN: egui::Vec2 = egui::vec2(1280.0, 800.0);
const SETTLE_FRAMES: usize = 6;
const HALF: f32 = 50.0;
const AUXILIARY_SLOT: u32 = 1;
const ORIGINAL: &str = "one\n";
const STAGED: &str = "staged\n";
const WORKTREE: &str = "staged and edited\n";
const MAIN_BRANCH: &str = "main";
const NESTED_DIRECTORY: &str = "docs";
const NESTED_NAME: &str = "d.txt";
const NESTED: &str = "docs/d.txt";

pub(super) struct Workspace {
    pub(super) fixture: Fixture,
    pub(super) data: PathBuf,
    pub(super) empty: PathBuf,
    pub(super) roots: Vec<PathBuf>,
    pub(super) projects: Vec<ProjectId>,
    pub(super) slots: Vec<ShellSlotId>,
}

impl Workspace {
    pub(super) fn new(label: &str, names: &[&str]) -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-git-{label}-{}", ProjectId::new()));
        let data = directory.join("data");
        let empty = directory.join("empty");
        let roots: Vec<_> = names.iter().map(|name| directory.join(name)).collect();
        for path in roots.iter().chain([&data, &empty]) {
            std::fs::create_dir_all(path).unwrap();
        }
        Self {
            fixture: Fixture {
                directory,
                application: None,
            },
            data,
            empty,
            projects: names.iter().map(|_| ProjectId::new()).collect(),
            slots: names.iter().map(|_| ShellSlotId::new()).collect(),
            roots,
        }
    }

    pub(super) fn state(&self, state: AppState) -> AppState {
        let names: Vec<_> = self
            .roots
            .iter()
            .map(|root| root.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();
        for ((project, root), name) in self.projects.iter().zip(&self.roots).zip(&names) {
            insert_project(&state, project, root, name, false);
        }
        {
            let mut session = state.session.write();
            session.projects = self
                .projects
                .iter()
                .zip(&self.roots)
                .zip(&names)
                .map(|((project, root), name)| project_ref(project, root, name))
                .collect();
            session.active_project = self.projects.last().cloned();
            let leaves: Vec<_> = self
                .projects
                .iter()
                .zip(&self.slots)
                .map(|(project, slot)| ShellSlotTree::Leaf {
                    slot_id: slot.clone(),
                    project_id: project.clone(),
                })
                .collect();
            session.shell_slots = Some(match leaves.len() {
                1 => leaves.into_iter().next().unwrap(),
                _ => ShellSlotTree::Split {
                    dir: SplitDir::Horizontal,
                    sizes: vec![HALF; leaves.len()],
                    children: leaves,
                },
            });
            session.focused_shell_slot = self.slots.last().cloned();
        }
        state
    }

    pub(super) fn git(&self, index: usize, args: &[&str]) -> String {
        fixture::git(&self.roots[index], &self.empty, args)
    }
}

pub(super) fn frame(
    application: &mut NativeApplication,
    context: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
            events,
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
    output
}

pub(super) fn until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = frame(application, context, Vec::new());
        if ready(application, &output) {
            return output;
        }
        assert!(Instant::now() < deadline, "phase={phase}");
        std::thread::yield_now();
    }
}

pub(super) fn settle(
    application: &mut NativeApplication,
    context: &egui::Context,
) -> egui::FullOutput {
    let mut output = frame(application, context, Vec::new());
    for _ in 1..SETTLE_FRAMES {
        output = frame(application, context, Vec::new());
    }
    output
}

pub(super) fn text(application: &NativeApplication, key: &str) -> String {
    presentation::message(&application.locale, key, &[])
}

pub(super) fn text_shapes<'a>(
    output: &'a egui::FullOutput,
    text: &'a str,
) -> impl Iterator<Item = &'a egui::epaint::TextShape> + 'a {
    output
        .shapes
        .iter()
        .filter_map(move |clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some(shape),
            _ => None,
        })
}

pub(super) fn has_text(output: &egui::FullOutput, text: &str) -> bool {
    text_shapes(output, text).next().is_some()
}

pub(super) fn nodes<'a>(
    output: &'a egui::FullOutput,
    label: &'a str,
) -> impl Iterator<Item = (egui::Rect, &'a egui::accesskit::Node)> + 'a {
    output
        .platform_output
        .accesskit_update
        .iter()
        .flat_map(|update| update.nodes.iter())
        .filter(move |(_, node)| node.label() == Some(label))
        .filter_map(|(_, node)| {
            node.bounds().map(|bounds| {
                (
                    egui::Rect::from_min_max(
                        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                    ),
                    node,
                )
            })
        })
}

pub(super) fn node(
    output: &egui::FullOutput,
    label: &str,
) -> Option<(egui::Rect, egui::accesskit::Node)> {
    nodes(output, label)
        .next()
        .map(|(rect, node)| (rect, node.clone()))
}

fn sidebar_view(
    application: &mut NativeApplication,
    project: &ProjectId,
    slot: &ShellSlotId,
) -> crate::symbol_sidebar::View {
    application
        .explorer_search_state
        .symbol_sidebars
        .entry(project, slot)
        .view
}

pub(super) fn show_git(application: &mut NativeApplication, context: &egui::Context) {
    frame(
        application,
        context,
        key(egui::Key::G, egui::Modifiers::CTRL | egui::Modifiers::SHIFT),
    );
}

fn status_color(application: &NativeApplication, kind: GitChangeKind) -> egui::Color32 {
    presentation::color(
        &application.settings_state.resolved_theme,
        crate::git_scm_model::status_color_key(kind),
    )
    .unwrap()
}

fn letter_colors(output: &egui::FullOutput, letter: &str) -> Vec<egui::Color32> {
    text_shapes(output, letter)
        .map(|shape| shape.galley.job.sections[0].format.color)
        .collect()
}

fn title_shows(output: &egui::FullOutput, text: &str) -> bool {
    text_shapes(output, text)
        .any(|shape| shape.visual_bounding_rect().bottom() <= taide_native_ui::shell::TITLE_HEIGHT)
}

#[test]
fn 실제_app_git_키와_view_git_명령은_포커스_slot만_git_보기로_바꾸고_접힌_사이드바를_펼친다() {
    let mut workspace = Workspace::new("view", &["alpha", "beta"]);
    let state = workspace.state(isolated_state(workspace.data.clone()));
    for project in &workspace.projects {
        state
            .layouts
            .write()
            .get_mut(project)
            .unwrap()
            .shell_view
            .sidebar_collapsed = true;
    }
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let (projects, slots) = (&workspace.projects, &workspace.slots);
    settle(application, &context);
    for (project, slot) in projects.iter().zip(slots) {
        application
            .explorer_search_state
            .symbol_sidebars
            .entry(project, slot)
            .view = crate::symbol_sidebar::View::Outline;
    }
    let collapsed = |index: usize| {
        services.state.layouts.read()[&projects[index]]
            .shell_view
            .sidebar_collapsed
    };

    show_git(application, &context);
    let notice = text(application, "git.notARepository");
    let output = until(
        application,
        &context,
        "focused slot git view",
        |_, output| !collapsed(1) && has_text(output, &notice),
    );
    assert_eq!(
        sidebar_view(application, &projects[1], &slots[1]),
        crate::symbol_sidebar::View::Git
    );
    assert_eq!(text_shapes(&output, &notice).count(), 1);
    settle(application, &context);
    assert!(collapsed(0));
    assert_eq!(
        sidebar_view(application, &projects[0], &slots[0]),
        crate::symbol_sidebar::View::Outline
    );

    application
        .explorer_search_state
        .symbol_sidebars
        .entry(&projects[1], &slots[1])
        .view = crate::symbol_sidebar::View::Files;
    application
        .commands_state
        .palette_commands
        .push("view.git".into());
    let explorer = until(application, &context, "view.git command", |_, output| {
        has_text(output, &notice)
    });
    assert_eq!(
        sidebar_view(application, &projects[1], &slots[1]),
        crate::symbol_sidebar::View::Git
    );
    assert_eq!(text_shapes(&explorer, &notice).count(), 1);
}

#[test]
fn 실제_app_보조_창의_git_키는_그_창의_탐색기를_열고_git_보기로_바꾼다() {
    let mut workspace = Workspace::new("auxiliary", &["alpha"]);
    let notes = workspace.roots[0].join("notes.txt");
    std::fs::write(&notes, ORIGINAL).unwrap();
    let state = workspace.state(isolated_state(workspace.data.clone()));
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&workspace.projects[0]).unwrap();
        let main = taide_model::ids::PaneId::new();
        layout.root = PaneNode::Leaf {
            id: main.clone(),
            tabs: Vec::new(),
            active: None,
        };
        layout.focused_pane = main;
        layout.auxiliary_windows = vec![auxiliary_window(AUXILIARY_SLOT, &file_tab(&notes))];
    }
    let context = launch(&mut workspace.fixture, state);
    let viewports = install_viewports(&context);
    let application = workspace.fixture.application.as_mut().unwrap();
    let alpha = workspace.projects[0].clone();
    let viewport = viewport_of(application, &alpha, AUXILIARY_SLOT);
    let window = |application: &NativeApplication| {
        application
            .shell_state
            .auxiliary_windows
            .find(&alpha, AUXILIARY_SLOT)
            .map(|window| (window.is_explorer_open, window.explorer.clone()))
            .unwrap()
    };
    settle(application, &context);
    viewports.borrow_mut().events.insert(
        viewport,
        key(
            egui::Key::B,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    until(
        application,
        &context,
        "auxiliary explorer closes",
        |application, _| !window(application).0,
    );

    viewports.borrow_mut().events.insert(
        viewport,
        key(egui::Key::G, egui::Modifiers::CTRL | egui::Modifiers::SHIFT),
    );
    until(
        application,
        &context,
        "auxiliary git view",
        |application, _| window(application).0,
    );
    let explorer = window(application).1;
    assert_eq!(
        sidebar_view(application, &alpha, &explorer),
        crate::symbol_sidebar::View::Git
    );
    let notice = text(application, "git.notARepository");
    until(application, &context, "auxiliary git body", |_, _| {
        viewports
            .borrow()
            .outputs
            .get(&viewport)
            .is_some_and(|output| has_text(output, &notice))
    });
    let main_view = sidebar_view(application, &alpha, &workspace.slots[0]);
    assert_eq!(main_view, crate::symbol_sidebar::View::Files);
}

#[test]
fn 실제_app_비저장소_git_보기는_안내와_초기화_버튼을_보이고_init_결과를_toast로_알린다() {
    let mut workspace = Workspace::new("init", &["alpha"]);
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    let alpha = workspace.projects[0].clone();
    settle(application, &context);
    show_git(application, &context);
    let notice = text(application, "git.notARepository");
    let button = text(application, "git.initRepository");
    let output = until(application, &context, "not a repository", |_, output| {
        has_text(output, &notice) && node(output, &button).is_some()
    });
    let (target, init) = node(&output, &button).unwrap();
    assert!(!init.is_disabled());
    let output = settle(application, &context);
    assert_eq!(application.git_state.title_branch(&alpha), None);
    assert!(!title_shows(&output, MAIN_BRANCH));

    application.git_state.init = Arc::new(|_, _| {
        Box::pin(async { Err(AppError::Internal("synthetic init failure".into())) })
    });
    frame(
        application,
        &context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
    let failed = text(application, "git.initFailed");
    let output = until(application, &context, "init failure toast", |_, output| {
        has_text(output, &failed)
    });
    assert!(has_text(&output, &notice));
    assert!(!has_text(&output, &text(application, "git.initSuccess")));

    let gate = Arc::new(tokio::sync::Notify::new());
    let release = gate.clone();
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    application.git_state.init = Arc::new(move |services, project| {
        let (gate, root, empty) = (gate.clone(), root.clone(), empty.clone());
        Box::pin(async move {
            gate.notified().await;
            fixture::init(&root, &empty);
            services.events.publish(AppEvent::GitStatusChanged {
                project_id: project.clone(),
            });
            services.events.publish(AppEvent::GitRefsChanged {
                project_id: project,
            });
            Ok(())
        })
    });
    frame(
        application,
        &context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
    until(
        application,
        &context,
        "init pending disables",
        |_, output| node(output, &button).is_some_and(|(_, node)| node.is_disabled()),
    );
    frame(
        application,
        &context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
    release.notify_one();
    let succeeded = text(application, "git.initSuccess");
    let clean = text(application, "git.noChanges");
    let output = until(application, &context, "initialized panel", |_, output| {
        has_text(output, &succeeded) && has_text(output, &clean) && !has_text(output, &notice)
    });
    assert!(node(&output, &button).is_none());
    if application.shell.has_title_bar {
        until(
            application,
            &context,
            "title branch after init",
            |_, output| title_shows(output, MAIN_BRANCH),
        );
    }
}

#[test]
fn 실제_app_변경_목록은_그룹과_건수와_행_표시를_원본대로_그리고_git과_fs_변경_뒤_다시_읽는다() {
    let mut workspace = Workspace::new("changes", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    std::fs::create_dir_all(root.join(NESTED_DIRECTORY)).unwrap();
    fixture::repository(
        &root,
        &empty,
        &[("a.txt", ORIGINAL), ("b.txt", ORIGINAL), (NESTED, ORIGINAL)],
    );
    std::fs::write(root.join("a.txt"), STAGED).unwrap();
    workspace.git(0, &["add", "--", "a.txt"]);
    std::fs::write(root.join("a.txt"), WORKTREE).unwrap();
    workspace.git(0, &["mv", "b.txt", "c.txt"]);
    std::fs::write(root.join(NESTED), WORKTREE).unwrap();
    std::fs::write(root.join("new.txt"), ORIGINAL).unwrap();
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    let alpha = workspace.projects[0].clone();
    settle(application, &context);
    show_git(application, &context);
    let staged = text(application, "git.stagedChanges");
    let changes = text(application, "git.changes");
    let output = until(application, &context, "change groups", |_, output| {
        node(output, &staged).is_some()
            && node(output, &changes).is_some()
            && node(output, "new.txt").is_some()
    });
    assert_eq!(
        nodes(&output, "a.txt").count(),
        2,
        "staged and unstaged rows"
    );
    assert_eq!(
        nodes(&output, "b.txt").count(),
        1,
        "git2 names an index rename by its old path"
    );
    assert!(node(&output, "c.txt").is_none());
    for title in [&staged, &changes] {
        assert!(has_text(&output, &title.to_uppercase()), "{title}");
        assert_eq!(
            node(&output, title).unwrap().1.is_expanded(),
            Some(true),
            "{title}"
        );
    }
    let (staged_header, _) = node(&output, &staged).unwrap();
    let (changes_header, _) = node(&output, &changes).unwrap();
    let staged_rows = nodes(&output, "a.txt")
        .chain(nodes(&output, "b.txt"))
        .filter(|(rect, _)| rect.top() > staged_header.top() && rect.top() < changes_header.top())
        .count();
    assert_eq!(staged_rows, 2);
    let badges = |count: &str| {
        text_shapes(&output, count)
            .filter(|shape| {
                [staged_header, changes_header]
                    .iter()
                    .any(|header| header.contains(shape.visual_bounding_rect().center()))
            })
            .count()
    };
    assert_eq!((badges("2"), badges("3")), (1, 1));
    let modified = status_color(application, GitChangeKind::Modified);
    assert_eq!(letter_colors(&output, "M"), [modified; 3]);
    assert_eq!(
        letter_colors(&output, "R"),
        [status_color(application, GitChangeKind::Renamed)]
    );
    assert_eq!(
        letter_colors(&output, "U"),
        [status_color(application, GitChangeKind::Untracked)]
    );
    if application.shell.has_title_bar {
        until(application, &context, "title branch", |_, output| {
            title_shows(output, MAIN_BRANCH)
        });
    }

    let nested = format!("{NESTED_NAME} {NESTED_DIRECTORY}");
    let (row, _) = node(&output, &nested).unwrap();
    assert!(has_text(&output, NESTED_NAME) && has_text(&output, NESTED_DIRECTORY));
    let stage = text(application, "git.stageChanges");
    let discard = text(application, "git.discard");
    let open = text(application, "git.openFile");
    frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(row.center())],
    );
    let hovered = until(application, &context, "nested row hover", |_, output| {
        has_text(output, NESTED)
    });
    for action in [&stage, &discard, &open] {
        assert_eq!(
            nodes(&hovered, action)
                .filter(|(rect, _)| rect.center().y >= row.top() && rect.center().y <= row.bottom())
                .count(),
            1,
            "{action}"
        );
    }
    assert_eq!(letter_colors(&hovered, "M").len(), 2);
    frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(egui::pos2(SCREEN.x, SCREEN.y))],
    );
    let away = settle(application, &context);
    assert!(nodes(&away, &stage).next().is_none());
    assert_eq!(letter_colors(&away, "M").len(), 3);

    fixture::git(&root, &empty, &["add", "--", "a.txt", "new.txt", NESTED]);
    application
        .services
        .events
        .publish(AppEvent::GitStatusChanged {
            project_id: alpha.clone(),
        });
    let output = until(
        application,
        &context,
        "staged after git add",
        |_, output| node(output, &changes).is_none() && nodes(output, "new.txt").count() == 1,
    );
    assert_eq!(nodes(&output, "a.txt").count(), 1);
    assert_eq!(
        letter_colors(&output, "A"),
        [status_color(application, GitChangeKind::Added)]
    );

    let later = root.join("later.txt");
    std::fs::write(&later, ORIGINAL).unwrap();
    application.services.events.publish(AppEvent::FsChanged {
        project_id: alpha,
        change: FsChange {
            kind: FsChangeKind::Created,
            paths: vec![later.to_str().unwrap().into()],
            from_app: false,
        },
    });
    let output = until(
        application,
        &context,
        "worktree file after fs event",
        |_, output| node(output, "later.txt").is_some(),
    );
    assert!(node(&output, &changes).is_some());
}

#[test]
fn 실제_app_탐색기_git_장식은_git_cache_status로_파일_배지와_상위_폴더를_그리고_git_변경_뒤_갱신한다()
 {
    let mut workspace = Workspace::new("explorer-decorations", &["alpha"]);
    workspace.roots[0] = workspace.roots[0].canonicalize().unwrap();
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    std::fs::create_dir_all(root.join(NESTED_DIRECTORY)).unwrap();
    fixture::repository(&root, &empty, &[("a.txt", ORIGINAL), (NESTED, ORIGINAL)]);
    std::fs::write(root.join("a.txt"), WORKTREE).unwrap();
    std::fs::write(root.join(NESTED_DIRECTORY).join("new.txt"), ORIGINAL).unwrap();
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    let alpha = workspace.projects[0].clone();
    let described = |output: &egui::FullOutput, label: &str| {
        nodes(output, label).find_map(|(_, node)| node.description().map(str::to_owned))
    };
    let output = until(
        application,
        &context,
        "explorer decorations",
        |_, output| {
            described(output, "a.txt").as_deref() == Some("modified")
                && described(output, NESTED_DIRECTORY).as_deref() == Some("untracked")
        },
    );
    assert_eq!(
        crate::symbol_sidebar::View::Files,
        sidebar_view(application, &alpha, &workspace.slots[0])
    );
    assert!(
        application
            .git_state
            .cache
            .status_result(&alpha)
            .is_some_and(|status| status.is_ok())
    );
    let explorer_modified = presentation::color(
        &application.settings_state.resolved_theme,
        "explorer.gitModified",
    )
    .unwrap();
    assert_eq!(letter_colors(&output, "M"), [explorer_modified]);

    workspace.git(0, &["commit", "-am", "test(fixture): settle a"]);
    let application = workspace.fixture.application.as_mut().unwrap();
    application
        .services
        .events
        .publish(AppEvent::GitStatusChanged {
            project_id: alpha.clone(),
        });
    until(
        application,
        &context,
        "decoration after commit",
        |_, output| {
            nodes(output, "a.txt").next().is_some()
                && described(output, "a.txt").is_none()
                && described(output, NESTED_DIRECTORY).as_deref() == Some("untracked")
        },
    );
}

#[test]
fn 실제_app_섹션_접힘은_토글마다_settings에_저장되고_새_app에서_복원된다() {
    let mut workspace = Workspace::new("collapse", &["alpha"]);
    let root = workspace.roots[0].clone();
    fixture::repository(
        &root,
        &workspace.empty,
        &[("a.txt", ORIGINAL), ("b.txt", ORIGINAL)],
    );
    std::fs::write(root.join("a.txt"), STAGED).unwrap();
    workspace.git(0, &["add", "--", "a.txt"]);
    std::fs::write(root.join("b.txt"), WORKTREE).unwrap();
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let mut application = workspace.fixture.application.take().unwrap();
    let services = application.services.clone();
    settle(&mut application, &context);
    show_git(&mut application, &context);
    let staged = text(&application, "git.stagedChanges");
    let changes = text(&application, "git.changes");
    let output = until(&mut application, &context, "groups", |_, output| {
        node(output, "a.txt").is_some() && node(output, "b.txt").is_some()
    });
    let stored = || {
        let file: serde_json::Value =
            serde_json::from_slice(&std::fs::read(services.state.paths.settings_file()).unwrap())
                .unwrap();
        file["gitSectionsCollapsed"].clone()
    };
    assert_eq!(stored(), serde_json::json!(["stashes"]));

    let (changes_header, _) = node(&output, &changes).unwrap();
    frame(
        &mut application,
        &context,
        pointer(changes_header.center(), egui::PointerButton::Primary),
    );
    let output = until(
        &mut application,
        &context,
        "changes collapsed",
        |_, output| node(output, "b.txt").is_none(),
    );
    assert_eq!(
        node(&output, &changes).unwrap().1.is_expanded(),
        Some(false)
    );
    let deadline = Instant::now() + DEADLINE;
    while stored() != serde_json::json!(["changes", "stashes"]) {
        assert!(Instant::now() < deadline, "first toggle is saved");
        frame(&mut application, &context, Vec::new());
    }

    let (staged_header, _) = node(&output, &staged).unwrap();
    frame(
        &mut application,
        &context,
        pointer(staged_header.center(), egui::PointerButton::Primary),
    );
    let (changes_header, _) =
        node(&frame(&mut application, &context, Vec::new()), &changes).unwrap();
    frame(
        &mut application,
        &context,
        pointer(changes_header.center(), egui::PointerButton::Primary),
    );
    let deadline = Instant::now() + DEADLINE;
    while stored() != serde_json::json!(["staged", "stashes"]) {
        assert!(Instant::now() < deadline, "both quick toggles are saved");
        frame(&mut application, &context, Vec::new());
    }
    let output = until(
        &mut application,
        &context,
        "staged collapsed",
        |_, output| node(output, "a.txt").is_none() && node(output, "b.txt").is_some(),
    );
    assert_eq!(node(&output, &staged).unwrap().1.is_expanded(), Some(false));
    eframe::App::on_exit(&mut application);
    drop(application);

    let restored = AppState::new(AppPaths::new(workspace.data.clone()));
    *restored.settings.write() = taide_settings::service::load_settings(&restored.paths);
    let restored = workspace.state(restored);
    let mut second = Fixture {
        directory: workspace.fixture.directory.join("second"),
        application: None,
    };
    let context = launch(&mut second, restored);
    let application = second.application.as_mut().unwrap();
    settle(application, &context);
    show_git(application, &context);
    let output = until(application, &context, "restored collapse", |_, output| {
        node(output, "b.txt").is_some()
    });
    assert!(node(&output, "a.txt").is_none());
    assert_eq!(node(&output, &staged).unwrap().1.is_expanded(), Some(false));
    assert_eq!(node(&output, &changes).unwrap().1.is_expanded(), Some(true));
}

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::git::GitChangeKind;
use taide_model::ids::ProjectId;
use taide_model::layout::{PaneNode, TabKind};
use taide_native_ui::commands::ShellIntent;

use super::base_tests::{Workspace, frame, has_text, node, nodes, settle, show_git, text, until};
use super::fixture;
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};
use crate::git_scm_model::Collapsed;

const SCREEN: egui::Vec2 = egui::vec2(1280.0, 800.0);
const ORIGINAL: &str = "one\n";
const STAGED: &str = "staged\n";
const WORKTREE: &str = "edited\n";
const MANY: usize = 500;
const STAGED_COUNT: usize = 3;
const WHEEL: f32 = 4000.0;
const ROW: f32 = 24.0;
const ARROW_STEPS: usize = 40;
const VISIBLE_ROW_LIMIT: usize = 60;
const MENU_ITEM_MIN_WIDTH: f32 = 2.0 * ROW;

fn away() -> Vec<egui::Event> {
    vec![egui::Event::PointerMoved(egui::pos2(SCREEN.x, SCREEN.y))]
}

fn frame_with(
    application: &mut NativeApplication,
    context: &egui::Context,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) {
    frame(
        application,
        context,
        std::iter::once(egui::Event::ModifiersChanged(modifiers))
            .chain(events)
            .collect(),
    );
    frame(
        application,
        context,
        vec![egui::Event::ModifiersChanged(egui::Modifiers::NONE)],
    );
}

fn focused_label(output: &egui::FullOutput) -> Option<String> {
    let update = output.platform_output.accesskit_update.as_ref()?;
    update
        .nodes
        .iter()
        .find(|(id, _)| *id == update.focus)
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

fn is_selected(node: &egui::accesskit::Node) -> bool {
    node.toggled() == Some(egui::accesskit::Toggled::True)
}

fn kinds(
    application: &NativeApplication,
    project: &ProjectId,
) -> (Vec<(TabKind, String)>, Option<TabKind>) {
    let layouts = application.services.state.layouts.read();
    let layout = &layouts[project];
    let mut stack = vec![&layout.root];
    let (mut tabs, mut active) = (Vec::new(), None);
    while let Some(pane) = stack.pop() {
        match pane {
            PaneNode::Split { children, .. } => stack.extend(children),
            PaneNode::Leaf {
                id,
                tabs: leaf,
                active: current,
            } => {
                tabs.extend(leaf.iter().map(|tab| (tab.kind.clone(), tab.title.clone())));
                if *id == layout.focused_pane {
                    active = leaf
                        .iter()
                        .find(|tab| Some(&tab.id) == current.as_ref())
                        .map(|tab| tab.kind.clone());
                }
            }
        }
    }
    (tabs, active)
}

/// The absolute path the repository reports for `path`: git resolves the temporary directory's
/// symbolic link (`/var` → `/private/var` on macOS).
fn real(path: &std::path::Path) -> String {
    path.parent()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join(path.file_name().unwrap())
        .to_str()
        .unwrap()
        .into()
}

fn diff(path: &std::path::Path, staged: bool) -> TabKind {
    TabKind::Diff {
        path: real(path),
        staged,
        compare_with: None,
        rev: None,
        parent_rev: None,
        before_path: None,
    }
}

fn status(
    application: &NativeApplication,
    project: &ProjectId,
    path: &str,
) -> Option<(Option<GitChangeKind>, Option<GitChangeKind>)> {
    let status = application.git_state.cache.status_result(project)?.ok()?;
    status
        .rows
        .iter()
        .find(|row| row.path == path)
        .map(|row| (row.staged, row.unstaged))
}

/// The change rows named `label`, top first: row buttons carry a pressed state, which tells them
/// apart from a tooltip or breadcrumb with the same text.
fn rows(output: &egui::FullOutput, label: &str) -> Vec<(egui::Rect, egui::accesskit::Node)> {
    let mut rows: Vec<_> = nodes(output, label)
        .filter(|(_, node)| node.toggled().is_some())
        .map(|(rect, node)| (rect, node.clone()))
        .collect();
    rows.sort_by(|left, right| left.0.top().total_cmp(&right.0.top()));
    rows
}

/// A dialog or menu button named `label`, not the 16px row button that shares its text.
fn wide(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    nodes(output, label)
        .map(|(rect, _)| rect)
        .find(|rect| rect.width() > MENU_ITEM_MIN_WIDTH)
}

fn hover(
    application: &mut NativeApplication,
    context: &egui::Context,
    label: &str,
    nth: usize,
) -> egui::FullOutput {
    let output = settle(application, context);
    let (row, _) = rows(&output, label).swap_remove(nth);
    frame(
        application,
        context,
        vec![egui::Event::PointerMoved(row.center())],
    );
    settle(application, context)
}

fn click_in_row(
    application: &mut NativeApplication,
    context: &egui::Context,
    row: &str,
    nth: usize,
    action: &str,
) {
    let output = hover(application, context, row, nth);
    let (row, _) = rows(&output, row).swap_remove(nth);
    let (target, _) = nodes(&output, action)
        .find(|(rect, _)| rect.center().y >= row.top() && rect.center().y <= row.bottom())
        .unwrap_or_else(|| panic!("{action}"));
    frame(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
}

fn click(application: &mut NativeApplication, context: &egui::Context, target: egui::Rect) {
    frame(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
}

/// Leaves the list and comes back onto the row, as a pointer does: the hoverable tooltip of the
/// row clicked before is drawn below that row and would otherwise take the right click.
fn right_click(application: &mut NativeApplication, context: &egui::Context, target: egui::Rect) {
    frame(application, context, away());
    settle(application, context);
    frame(
        application,
        context,
        vec![egui::Event::PointerMoved(target.center())],
    );
    settle(application, context);
    frame(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Secondary),
    );
}

fn launch_changes(workspace: &mut Workspace) -> egui::Context {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    settle(application, &context);
    show_git(application, &context);
    context
}

#[test]
fn 실제_app_행_버튼과_그룹_버튼은_stage_unstage를_실행하고_discard는_확인_뒤에만_되돌린다() {
    let mut workspace = Workspace::new("actions", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(
        &root,
        &empty,
        &[
            ("a.txt", ORIGINAL),
            ("b.txt", ORIGINAL),
            ("c.txt", ORIGINAL),
        ],
    );
    std::fs::write(root.join("b.txt"), STAGED).unwrap();
    workspace.git(0, &["add", "--", "b.txt"]);
    std::fs::write(root.join("a.txt"), WORKTREE).unwrap();
    std::fs::write(root.join("c.txt"), WORKTREE).unwrap();
    let context = launch_changes(&mut workspace);
    let (project, slot) = (workspace.projects[0].clone(), workspace.slots[0].clone());
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "rows", |_, output| {
        node(output, "a.txt").is_some() && node(output, "b.txt").is_some()
    });
    let modified = Some(GitChangeKind::Modified);

    let stage = text(application, "git.stageChanges");
    click_in_row(application, &context, "a.txt", 0, &stage);
    until(application, &context, "a staged", |application, _| {
        status(application, &project, "a.txt") == Some((modified, None))
    });
    let unstage = text(application, "git.unstageChanges");
    click_in_row(application, &context, "a.txt", 0, &unstage);
    until(application, &context, "a unstaged", |application, _| {
        status(application, &project, "a.txt") == Some((None, modified))
    });

    let changes = text(application, "git.changes");
    let stage_all = text(application, "git.stageAll");
    let output = until(application, &context, "changes header", |_, output| {
        node(output, &changes).is_some() && node(output, "c.txt").is_some()
    });
    let (header, _) = node(&output, &changes).unwrap();
    assert!(node(&output, &stage_all).is_none(), "group action hidden");
    frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(header.center())],
    );
    let output = settle(application, &context);
    let (action, _) = node(&output, &stage_all).expect("group action on hover");
    click(application, &context, action);
    let output = until(application, &context, "stage all", |application, output| {
        node(output, &changes).is_none()
            && ["a.txt", "b.txt", "c.txt"]
                .iter()
                .all(|path| status(application, &project, path) == Some((modified, None)))
    });
    assert_eq!(
        application.git_state.panels.collapsed(&project, &slot),
        Some(Collapsed::from_ids(&["stashes".to_owned()])),
        "the action click does not toggle its header"
    );

    let staged = text(application, "git.stagedChanges");
    let unstage_all = text(application, "git.unstageAll");
    click(application, &context, rows(&output, "a.txt")[0].0);
    frame(application, &context, away());
    frame(
        application,
        &context,
        key(egui::Key::ArrowUp, egui::Modifiers::NONE),
    );
    let output = until(application, &context, "header focus", |_, output| {
        focused_label(output).as_deref() == Some(staged.as_str())
    });
    let (action, _) = node(&output, &unstage_all).expect("group action on keyboard focus");
    click(application, &context, action);
    until(
        application,
        &context,
        "unstage all",
        |application, output| {
            node(output, &staged).is_none()
                && ["a.txt", "b.txt", "c.txt"]
                    .iter()
                    .all(|path| status(application, &project, path) == Some((None, modified)))
        },
    );

    let discard = text(application, "git.discard");
    let title = text(application, "git.discardTitle");
    let cancel = text(application, "common.cancel");
    let confirm = text(application, "git.discardConfirm");
    let description =
        crate::git_scm_model::discard_description(&application.locale, &["c.txt".to_owned()]);
    click_in_row(application, &context, "c.txt", 0, &discard);
    let output = until(application, &context, "discard dialog", |_, output| {
        has_text(output, &title) && has_text(output, &description)
    });
    let output = if focused_label(&output).as_deref() == Some(cancel.as_str()) {
        output
    } else {
        until(application, &context, "cancel focus", |_, output| {
            focused_label(output).as_deref() == Some(cancel.as_str())
        })
    };
    click(application, &context, wide(&output, &cancel).unwrap());
    settle(application, &context);
    let output = settle(application, &context);
    assert!(!has_text(&output, &title));
    assert_eq!(
        std::fs::read_to_string(root.join("c.txt")).unwrap(),
        WORKTREE
    );

    click_in_row(application, &context, "a.txt", 0, &discard);
    until(application, &context, "second dialog", |_, output| {
        has_text(output, &title)
    });
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    let output = settle(application, &context);
    assert!(!has_text(&output, &title), "Escape cancels");
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        WORKTREE
    );

    click_in_row(application, &context, "c.txt", 0, &discard);
    let output = until(application, &context, "third dialog", |_, output| {
        wide(output, &confirm).is_some()
    });
    click(application, &context, wide(&output, &confirm).unwrap());
    until(application, &context, "discarded", |application, output| {
        status(application, &project, "c.txt").is_none() && node(output, "c.txt").is_none()
    });
    assert_eq!(
        std::fs::read_to_string(root.join("c.txt")).unwrap(),
        ORIGINAL
    );
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        WORKTREE
    );

    let open = text(application, "git.openFile");
    click_in_row(application, &context, "b.txt", 0, &open);
    let expected = TabKind::File {
        path: real(&root.join("b.txt")),
    };
    until(application, &context, "file tab", |application, _| {
        kinds(application, &project).1.as_ref() == Some(&expected)
    });
}

#[test]
fn 실제_app_행_클릭은_diff_탭을_열고_수식키는_선택만_바꾸며_우클릭_메뉴는_그룹별_항목을_보인다() {
    let mut workspace = Workspace::new("rows", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(
        &root,
        &empty,
        &[
            ("a.txt", ORIGINAL),
            ("b.txt", ORIGINAL),
            ("c.txt", ORIGINAL),
        ],
    );
    std::fs::write(root.join("a.txt"), STAGED).unwrap();
    workspace.git(0, &["add", "--", "a.txt"]);
    for path in ["a.txt", "b.txt", "c.txt"] {
        std::fs::write(root.join(path), WORKTREE).unwrap();
    }
    let context = launch_changes(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let row =
        |output: &egui::FullOutput, label: &str, nth: usize| rows(output, label).swap_remove(nth);
    let output = until(application, &context, "rows", |_, output| {
        nodes(output, "a.txt").count() == 2 && node(output, "c.txt").is_some()
    });

    click(application, &context, row(&output, "b.txt", 0).0);
    let b_diff = diff(&root.join("b.txt"), false);
    until(application, &context, "b diff", |application, _| {
        kinds(application, &project).1.as_ref() == Some(&b_diff)
    });
    let (tabs, _) = kinds(application, &project);
    assert!(tabs.contains(&(b_diff.clone(), "b.txt (diff)".to_owned())));
    let output = settle(application, &context);
    assert!(is_selected(&row(&output, "b.txt", 0).1));
    click(application, &context, row(&output, "b.txt", 0).0);
    settle(application, &context);
    let output = settle(application, &context);
    assert_eq!(
        kinds(application, &project)
            .0
            .iter()
            .filter(|(kind, _)| *kind == b_diff)
            .count(),
        1,
        "the same row reopens its tab"
    );

    click(application, &context, row(&output, "a.txt", 0).0);
    let a_staged = diff(&root.join("a.txt"), true);
    until(application, &context, "staged diff", |application, _| {
        kinds(application, &project).1.as_ref() == Some(&a_staged)
    });
    let output = settle(application, &context);
    let tab_count = kinds(application, &project).0.len();
    frame_with(
        application,
        &context,
        pointer(
            row(&output, "c.txt", 0).0.center(),
            egui::PointerButton::Primary,
        ),
        egui::Modifiers::SHIFT,
    );
    let output = settle(application, &context);
    for (label, nth) in [("a.txt", 0), ("a.txt", 1), ("b.txt", 0), ("c.txt", 0)] {
        assert!(is_selected(&row(&output, label, nth).1), "{label} {nth}");
    }
    frame_with(
        application,
        &context,
        pointer(
            row(&output, "b.txt", 0).0.center(),
            egui::PointerButton::Primary,
        ),
        egui::Modifiers::COMMAND,
    );
    let output = settle(application, &context);
    assert!(!is_selected(&row(&output, "b.txt", 0).1));
    assert!(is_selected(&row(&output, "c.txt", 0).1));
    assert_eq!(
        kinds(application, &project).0.len(),
        tab_count,
        "modifier clicks open nothing"
    );
    assert_eq!(kinds(application, &project).1.as_ref(), Some(&a_staged));

    let labels = |application: &NativeApplication, keys: &[&str]| {
        keys.iter()
            .map(|key| text(application, key))
            .collect::<Vec<_>>()
    };
    let changes_menu = labels(
        application,
        &[
            "git.openFile",
            "git.openChanges",
            "git.stageChanges",
            "git.discard",
            "explorer.copyPath",
            "explorer.reveal",
        ],
    );
    let staged_menu = labels(
        application,
        &[
            "git.openFile",
            "git.openChanges",
            "git.unstageChanges",
            "explorer.copyPath",
            "explorer.reveal",
        ],
    );
    let all_menu_labels = labels(
        application,
        &[
            "git.openFile",
            "git.openChanges",
            "git.stageChanges",
            "git.unstageChanges",
            "git.discard",
            "explorer.copyPath",
            "explorer.reveal",
        ],
    );
    let menu = |output: &egui::FullOutput| {
        let mut items: Vec<_> = all_menu_labels
            .iter()
            .flat_map(|label| {
                nodes(output, label)
                    .filter(|(rect, _)| rect.width() > MENU_ITEM_MIN_WIDTH)
                    .map(move |(rect, _)| (rect.top(), label.clone()))
            })
            .collect();
        items.sort_by(|left, right| left.0.total_cmp(&right.0));
        items
            .into_iter()
            .map(|(_, label)| label)
            .collect::<Vec<_>>()
    };

    right_click(application, &context, row(&output, "c.txt", 0).0);
    frame(application, &context, away());
    let output = settle(application, &context);
    assert_eq!(menu(&output), changes_menu);
    assert!(is_selected(&row(&output, "c.txt", 0).1));
    assert!(
        !is_selected(&row(&output, "a.txt", 0).1),
        "right click selects only its row"
    );
    let (open_changes, _) = node(&output, &text(application, "git.openChanges")).unwrap();
    click(application, &context, open_changes);
    let c_diff = diff(&root.join("c.txt"), false);
    until(
        application,
        &context,
        "menu open changes",
        |application, output| {
            kinds(application, &project).1.as_ref() == Some(&c_diff) && menu(output).is_empty()
        },
    );

    let output = settle(application, &context);
    right_click(application, &context, row(&output, "a.txt", 0).0);
    frame(application, &context, away());
    let output = settle(application, &context);
    assert_eq!(menu(&output), staged_menu);
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    let output = settle(application, &context);
    assert!(menu(&output).is_empty(), "Escape closes the menu");

    let (header, _) = node(&output, &text(application, "git.changes")).unwrap();
    right_click(application, &context, header);
    frame(application, &context, away());
    let output = settle(application, &context);
    assert!(menu(&output).is_empty(), "a header has no menu");

    right_click(application, &context, row(&output, "c.txt", 0).0);
    frame(application, &context, away());
    let output = settle(application, &context);
    assert_eq!(menu(&output), changes_menu);
    fixture::git(&root, &empty, &["add", "--", "c.txt"]);
    application
        .services
        .events
        .publish(AppEvent::GitStatusChanged {
            project_id: project.clone(),
        });
    until(
        application,
        &context,
        "menu target left",
        |application, output| {
            status(application, &project, "c.txt") == Some((Some(GitChangeKind::Modified), None))
                && menu(output).is_empty()
        },
    );
}

#[test]
fn 실제_app_행_메뉴의_경로_복사와_reveal은_절대_경로_intent를_낸다() {
    let mut workspace = Workspace::new("menu-intents", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    std::fs::create_dir_all(root.join("docs")).unwrap();
    fixture::repository(&root, &empty, &[("docs/d.txt", ORIGINAL)]);
    std::fs::write(root.join("docs/d.txt"), WORKTREE).unwrap();
    let context = launch_changes(&mut workspace);
    let (project, slot) = (workspace.projects[0].clone(), workspace.slots[0].clone());
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "row", |_, output| {
        node(output, "d.txt docs").is_some()
    });
    let absolute = real(&root.join("docs/d.txt"));
    for (key, expected) in [
        ("explorer.copyPath", ShellIntent::CopyText(absolute.clone())),
        ("explorer.reveal", ShellIntent::RevealPath(absolute.clone())),
    ] {
        let surface = egui::Context::default();
        surface.enable_accesskit();
        let label = text(application, key);
        let run = |application: &NativeApplication, events: Vec<egui::Event>| {
            let view = crate::git_scm::View {
                project: &project,
                slot: &slot,
                status: application.git_state.cache.status_result(&project),
                collapsed: Collapsed::from_ids(&["stashes".to_owned()]),
                panel: application.git_state.panels.panel(&project, &slot),
                is_initializing: false,
                remotes: application.git_state.cache.remotes(&project),
                branches: application.git_state.cache.branches(&project),
                stashes: application.git_state.cache.stashes(&project),
                is_stashing: false,
                is_syncing: false,
                commit: Default::default(),
                graph: Default::default(),
                appearance: application.git_state.appearance.as_ref().unwrap(),
                locale: &application.locale,
                tooltips: &application.tooltips,
                tooltip_appearance: &application.tooltip_appearance,
            };
            let mut intents = Vec::new();
            let mut output = surface.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    crate::git_scm::show(
                        ui,
                        &view,
                        &mut crate::navigation_icons::Deferred::default(),
                        &mut intents,
                    )
                    .unwrap();
                },
            );
            output.textures_delta.clear();
            (output, intents)
        };
        let (output, _) = run(application, Vec::new());
        let (row, _) = node(&output, "d.txt docs").unwrap();
        let (_, intents) = run(
            application,
            pointer(row.center(), egui::PointerButton::Secondary),
        );
        let target = intents
            .iter()
            .find_map(|intent| match intent {
                ShellIntent::Git(taide_native_ui::intents::git::Intent::RowMenu {
                    target, ..
                }) => Some(target.clone()),
                _ => None,
            })
            .expect("row menu intent");
        assert_eq!(target.path, "docs/d.txt");
        application
            .git_state
            .panels
            .open_menu(&project, &slot, target);
        let (output, _) = run(application, Vec::new());
        let (item, _) = node(&output, &label).unwrap_or_else(|| panic!("{key}"));
        let (_, intents) = run(
            application,
            pointer(item.center(), egui::PointerButton::Primary),
        );
        assert!(
            intents.iter().any(|intent| match (intent, &expected) {
                (ShellIntent::CopyText(left), ShellIntent::CopyText(right))
                | (ShellIntent::RevealPath(left), ShellIntent::RevealPath(right)) => left == right,
                _ => false,
            }),
            "{key}"
        );
    }
}

#[test]
fn 실제_app_긴_목록은_보이는_행만_그리고_sticky_머리말과_화살표_roving을_원본대로_한다() {
    let mut workspace = Workspace::new("virtual", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("seed.txt", ORIGINAL)]);
    for index in 0..STAGED_COUNT {
        let name = format!("s{index}.txt");
        std::fs::write(root.join(&name), ORIGINAL).unwrap();
        workspace.git(0, &["add", "--", &name]);
    }
    for index in 0..MANY {
        std::fs::write(root.join(format!("f{index:03}.txt")), ORIGINAL).unwrap();
    }
    let context = launch_changes(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let staged = text(application, "git.stagedChanges");
    let changes = text(application, "git.changes");
    let many_rows = |output: &egui::FullOutput| {
        (0..MANY)
            .filter_map(|index| node(output, &format!("f{index:03}.txt")))
            .map(|(rect, _)| rect)
            .collect::<Vec<_>>()
    };
    let output = until(application, &context, "long list", |_, output| {
        node(output, "f000.txt").is_some()
    });
    let drawn = many_rows(&output).len();
    assert!(drawn > 0 && drawn < VISIBLE_ROW_LIMIT, "drawn={drawn}");

    let (top_row, _) = node(&output, "s0.txt").unwrap();
    frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(top_row.center())],
    );
    frame(
        application,
        &context,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -WHEEL),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    frame(application, &context, away());
    let output = until(application, &context, "scrolled", |_, output| {
        node(output, "f000.txt").is_none() && node(output, &staged).is_none()
    });
    let rows = many_rows(&output);
    let (header, _) = node(&output, &changes).expect("sticky header");
    let first = rows
        .iter()
        .map(|rect| rect.top())
        .fold(f32::INFINITY, f32::min);
    assert!(
        header.top() >= first - 1.0 && header.top() <= first + ROW + 1.0,
        "header={header:?} first={first}"
    );
    assert!(many_rows(&output).len() < VISIBLE_ROW_LIMIT);

    frame(
        application,
        &context,
        vec![
            egui::Event::PointerMoved(header.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, WHEEL * 2.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let output = until(application, &context, "back to top", |_, output| {
        node(output, "s0.txt").is_some() && node(output, &staged).is_some()
    });
    let (s0, _) = node(&output, "s0.txt").unwrap();
    click(application, &context, s0);
    frame(application, &context, away());
    frame(
        application,
        &context,
        key(egui::Key::ArrowUp, egui::Modifiers::NONE),
    );
    until(application, &context, "header focus", |_, output| {
        focused_label(output).as_deref() == Some(staged.as_str())
    });
    frame(
        application,
        &context,
        key(egui::Key::ArrowLeft, egui::Modifiers::NONE),
    );
    until(application, &context, "collapsed", |_, output| {
        node(output, &staged).is_some_and(|(_, node)| node.is_expanded() == Some(false))
            && node(output, "s0.txt").is_none()
    });
    frame(
        application,
        &context,
        key(egui::Key::ArrowRight, egui::Modifiers::NONE),
    );
    until(application, &context, "expanded", |_, output| {
        node(output, &staged).is_some_and(|(_, node)| node.is_expanded() == Some(true))
            && node(output, "s0.txt").is_some()
    });
    let sequence: Vec<String> = std::iter::once(staged.clone())
        .chain((0..STAGED_COUNT).map(|index| format!("s{index}.txt")))
        .chain(std::iter::once(changes.clone()))
        .chain((0..MANY).map(|index| format!("f{index:03}.txt")))
        .collect();
    for step in 1..=ARROW_STEPS {
        frame(
            application,
            &context,
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
        );
        until(application, &context, &sequence[step], |_, output| {
            focused_label(output).as_deref() == Some(sequence[step].as_str())
        });
    }
    let target = &sequence[ARROW_STEPS];
    let output = settle(application, &context);
    let (focused, _) = node(&output, target).expect("focused row is drawn");
    let (header, _) = node(&output, &changes).unwrap();
    assert!(
        focused.top() >= header.bottom() - 1.0,
        "below the sticky header"
    );
    assert!(focused.bottom() <= SCREEN.y);

    frame(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let expected = diff(&root.join(target), false);
    until(application, &context, "keyboard diff", |application, _| {
        kinds(application, &project).1.as_ref() == Some(&expected)
    });
    let output = settle(application, &context);
    assert!(is_selected(&node(&output, target).unwrap().1));
}

#[test]
fn 실제_app_화살표는_양끝에서_멈춘다() {
    let mut workspace = Workspace::new("clamp", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ORIGINAL)]);
    std::fs::write(root.join("a.txt"), WORKTREE).unwrap();
    let context = launch_changes(&mut workspace);
    let application = workspace.fixture.application.as_mut().unwrap();
    let changes = text(application, "git.changes");
    let graph = text(application, "git.graph");
    let output = until(application, &context, "row", |_, output| {
        node(output, "a.txt").is_some() && node(output, &graph).is_some()
    });
    let (row, _) = node(&output, "a.txt").unwrap();
    click(application, &context, row);
    frame(application, &context, away());
    for _ in 0..2 {
        frame(
            application,
            &context,
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
        );
        let output = settle(application, &context);
        assert_eq!(focused_label(&output).as_deref(), Some(graph.as_str()));
    }
    let expanded = |output: &egui::FullOutput| node(output, &graph).unwrap().1.is_expanded();
    for (pressed, is_expanded) in [
        (egui::Key::ArrowLeft, Some(false)),
        (egui::Key::ArrowRight, Some(true)),
    ] {
        frame(application, &context, key(pressed, egui::Modifiers::NONE));
        let output = until(application, &context, "graph toggle", |_, output| {
            expanded(output) == is_expanded
        });
        assert_eq!(focused_label(&output).as_deref(), Some(graph.as_str()));
    }
    for target in ["a.txt", changes.as_str(), changes.as_str()] {
        frame(
            application,
            &context,
            key(egui::Key::ArrowUp, egui::Modifiers::NONE),
        );
        let output = settle(application, &context);
        assert_eq!(focused_label(&output).as_deref(), Some(target));
    }
}

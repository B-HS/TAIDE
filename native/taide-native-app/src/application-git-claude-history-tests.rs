use std::sync::Arc;

use eframe::egui;
use serde_json::{Value, json};
use taide_ide::protocol::{diff_outcome_text, text_content};
use taide_model::ide::IdeDiffOutcome;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, Tab, TabKind};
use taide_native_editor::document::{DocumentKey, VirtualKey, VirtualSide};
use taide_native_ui::diff_view::Hunk;
use taide_native_ui::shell::{TAB_HEIGHT, TITLE_HEIGHT};
use taide_native_ui::toast::Kind;
use taide_runtime::layout_actions;

use super::base_tests::{
    Workspace, frame, has_text, node, nodes, settle, text, text_shapes, until,
};
use super::{Command, fixture};
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};
use crate::host::HostCommand;

const COMMITTED: &str = "alpha\nbeta\n";
const EDITED: &str = "alpha\nBETA\n";
const THIRD: &str = "alpha\nBETA\ngamma\n";
const ADDED: &str = "added\n";
const GONE: &str = "removed\n";
const DISK: &str = "alpha\nbeta\ngamma\n";
const PROPOSED: &str = "alpha  \nbeta\nGAMMA\n";
const TITLE: &str = "synthetic claude diff";
const MISSING_REV: &str = "0123456789abcdef0123456789abcdef01234567";
const CARET_INSET: f32 = 1.0;

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

fn launch_git(workspace: &mut Workspace) -> egui::Context {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    settle(workspace.fixture.application.as_mut().unwrap(), &context);
    context
}

fn all_tabs(application: &NativeApplication, project: &ProjectId) -> Vec<Tab> {
    let layouts = application.services.state.layouts.read();
    let mut stack = vec![&layouts[project].root];
    let mut tabs = Vec::new();
    while let Some(pane) = stack.pop() {
        match pane {
            PaneNode::Split { children, .. } => stack.extend(children),
            PaneNode::Leaf { tabs: leaf, .. } => tabs.extend(leaf.iter().cloned()),
        }
    }
    tabs
}

fn active(application: &NativeApplication, project: &ProjectId) -> Option<Tab> {
    let layouts = application.services.state.layouts.read();
    taide_native_ui::snapshot::active_tab(&layouts[project].root, &layouts[project].focused_pane)
        .cloned()
}

fn virtual_text(
    application: &NativeApplication,
    owner: &TabId,
    side: VirtualSide,
) -> Option<String> {
    let documents = application.store.documents();
    let document = documents.find(&DocumentKey::Virtual(VirtualKey {
        owner: owner.clone(),
        side,
    }))?;
    Some(documents.snapshot(document).ok()?.rope.to_string())
}

fn open(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    kind: TabKind,
) -> Tab {
    assert!(application.submit(HostCommand::Git(Command::OpenTab {
        project: project.clone(),
        kind: kind.clone(),
        title: "commit diff".into(),
        target: None,
        preview: false,
    })));
    until(application, context, "tab active", |application, _| {
        active(application, project).is_some_and(|tab| tab.kind == kind)
    });
    active(application, project).unwrap()
}

fn commit_kind(path: &str, rev: &str, parent: Option<&str>) -> TabKind {
    TabKind::Diff {
        path: path.into(),
        staged: false,
        compare_with: None,
        rev: Some(rev.into()),
        parent_rev: parent.map(Into::into),
        before_path: Some(path.into()),
    }
}

/// Runs the IDE `openDiff` tool the way an agent does, answering with its outcome text.
fn open_diff(application: &NativeApplication, path: &str) -> tokio::task::JoinHandle<Value> {
    ide_tool(
        application,
        "openDiff",
        json!({"new_file_path": path, "new_file_contents": PROPOSED, "tab_name": TITLE}),
    )
}

fn ide_tool(
    application: &NativeApplication,
    name: &'static str,
    arguments: Value,
) -> tokio::task::JoinHandle<Value> {
    let services = application.services.clone();
    application.runtime.spawn(async move {
        let actions = crate::ide_tools::LayoutActions {
            view_states: Arc::default(),
            wait_markers: Arc::default(),
            open_file_tab: |services, project, path, title, preview| {
                Box::pin(async move {
                    layout_actions::layout_open_tab(
                        services.events.as_ref(),
                        &services.state,
                        project,
                        TabKind::File { path },
                        title,
                        None,
                        preview,
                    )
                    .await
                    .map(|_| ())
                })
            },
            close_tab: Arc::new(|services, tab| {
                Box::pin(async move {
                    layout_actions::close_tab_and_finish(
                        services.events.as_ref(),
                        &services.state,
                        &tab,
                        |tab| services.ide.reconcile_closed_tab(tab),
                    )
                    .await
                    .map(|(_, closed, _)| closed.tab)
                })
            }),
        };
        match crate::ide_tools::dispatch(services, &actions, name, &arguments).await {
            Ok(value) => value,
            Err(error) => panic!("{}", error.message),
        }
    })
}

fn answered(
    application: &mut NativeApplication,
    context: &egui::Context,
    pending: tokio::task::JoinHandle<Value>,
) -> Value {
    until(application, context, "ide answer", |_, _| {
        pending.is_finished()
    });
    application.runtime.block_on(pending).unwrap()
}

fn claude_tab(application: &NativeApplication, project: &ProjectId) -> Option<Tab> {
    active(application, project)
        .filter(|tab| matches!(tab.kind, TabKind::ClaudeDiff { .. }) && tab.title == TITLE)
}

fn rightmost(output: &egui::FullOutput, word: &str) -> egui::Pos2 {
    let shape = text_shapes(output, word)
        .max_by(|left, right| left.pos.x.total_cmp(&right.pos.x))
        .unwrap_or_else(|| panic!("{word}"));
    egui::pos2(
        shape.pos.x + CARET_INSET,
        shape.pos.y + shape.galley.size().y / 2.0,
    )
}

fn leftmost(output: &egui::FullOutput, word: &str) -> egui::Pos2 {
    let shape = text_shapes(output, word)
        .min_by(|left, right| left.pos.x.total_cmp(&right.pos.x))
        .unwrap_or_else(|| panic!("{word}"));
    egui::pos2(
        shape.pos.x + CARET_INSET,
        shape.pos.y + shape.galley.size().y / 2.0,
    )
}

fn click(application: &mut NativeApplication, context: &egui::Context, at: egui::Pos2) {
    frame(
        application,
        context,
        pointer(at, egui::PointerButton::Primary),
    );
}

fn type_text(application: &mut NativeApplication, context: &egui::Context, typed: &str) {
    frame(application, context, vec![egui::Event::Text(typed.into())]);
}

#[test]
fn 실제_app_커밋_diff_탭은_수정_추가_삭제_최초_커밋을_그리고_rev_오류는_실패_문구를_그린다() {
    let mut workspace = Workspace::new("commit-diff", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED), ("c.txt", GONE)]);
    let first = workspace.git(0, &["rev-parse", "HEAD"]);
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    std::fs::write(root.join("b.txt"), ADDED).unwrap();
    workspace.git(0, &["rm", "-q", "--", "c.txt"]);
    workspace.git(0, &["add", "--", "a.txt", "b.txt"]);
    workspace.git(0, &["commit", "-q", "-m", "test(fixture): second"]);
    let second = workspace.git(0, &["rev-parse", "HEAD"]);
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let [a, b, c] = ["a.txt", "b.txt", "c.txt"].map(|name| real(&root.join(name)));

    for (path, rev, parent, original, modified) in [
        (&a, &second, Some(first.as_str()), COMMITTED, EDITED),
        (&b, &second, Some(first.as_str()), "", ADDED),
        (&c, &second, Some(first.as_str()), GONE, ""),
        (&a, &first, None, "", COMMITTED),
    ] {
        let tab = open(
            application,
            &context,
            &project,
            commit_kind(path, rev, parent),
        );
        until(
            application,
            &context,
            "commit diff sides",
            |application, _| {
                virtual_text(application, &tab.id, VirtualSide::Original).as_deref()
                    == Some(original)
                    && virtual_text(application, &tab.id, VirtualSide::Modified).as_deref()
                        == Some(modified)
            },
        );
        let key = super::Key::CommitDiff {
            rev: rev.clone(),
            parent: parent.map(Into::into),
            path: path.clone(),
            before_path: path.clone(),
        };
        let fetched = application.git_state.cache.fetched_at(&project, &key);
        application
            .git_state
            .cache
            .invalidate(&project, &crate::git_cache::Invalidation::repository());
        settle(application, &context);
        assert_eq!(
            application.git_state.cache.fetched_at(&project, &key),
            fetched,
            "불변 범위는 저장소 변경에 다시 읽지 않음"
        );
    }

    let failed = text(application, "editor.diffLoadFailed");
    open(
        application,
        &context,
        &project,
        commit_kind(&a, MISSING_REV, Some(&first)),
    );
    until(application, &context, "missing rev", |_, output| {
        has_text(output, &failed)
    });
}

#[test]
fn 실제_app_ide_open_diff는_claude_diff_탭을_열고_수정_쪽_편집_뒤_accept로_저장한다() {
    let mut workspace = Workspace::new("claude-accept", &["alpha"]);
    let root = workspace.roots[0].clone();
    std::fs::write(root.join("a.txt"), DISK).unwrap();
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));

    let pending = open_diff(application, &path);
    let output = until(
        application,
        &context,
        "claude diff tab",
        |application, output| {
            claude_tab(application, &project).is_some()
                && has_text(output, "GAMMA")
                && has_text(output, "gamma")
        },
    );
    let tab = claude_tab(application, &project).unwrap();
    assert!(!tab.preview);
    let TabKind::ClaudeDiff { request_id, .. } = &tab.kind else {
        unreachable!();
    };
    assert_eq!(application.git_state.claude.contents(request_id), PROPOSED);
    until(
        application,
        &context,
        "whitespace ignored",
        |application, _| {
            application.git_state.claude.hunks(&tab.id)
                == [Hunk {
                    old_start: 3,
                    old_lines: 1,
                    new_start: 3,
                    new_lines: 1,
                }]
        },
    );

    click(application, &context, leftmost(&output, "beta"));
    type_text(application, &context, "Q");
    click(application, &context, rightmost(&output, "beta"));
    type_text(application, &context, "Z");
    let edited = "alpha  \nZbeta\nGAMMA\n";
    until(application, &context, "edited", |application, _| {
        virtual_text(application, &tab.id, VirtualSide::Modified).as_deref() == Some(edited)
    });
    assert_eq!(
        virtual_text(application, &tab.id, VirtualSide::Original).as_deref(),
        Some(DISK),
        "원본 쪽은 읽기 전용"
    );
    let output = until(application, &context, "recomputed", |application, _| {
        application.git_state.claude.hunks(&tab.id)
            == [Hunk {
                old_start: 2,
                old_lines: 2,
                new_start: 2,
                new_lines: 2,
            }]
    });

    let accept = text(application, "ide.acceptChanges");
    let (button, _) = node(&output, &accept).expect("accept 버튼");
    click(application, &context, button.center());
    assert_eq!(
        answered(application, &context, pending),
        text_content(diff_outcome_text(IdeDiffOutcome::Saved))
    );
    assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), edited);
    let accepted = text(application, "ide.diffAccepted");
    until(application, &context, "accepted", |application, _| {
        application.toasts.is_showing(Kind::Success, &accepted)
            && !all_tabs(application, &project)
                .iter()
                .any(|tab| matches!(tab.kind, TabKind::ClaudeDiff { .. }))
    });
    assert_eq!(application.git_state.claude.contents(request_id), "");
}

#[test]
fn 실제_app_claude_diff는_자동_열기_끄기_reject_탭_닫기_ide_close_tab을_원본대로_처리한다() {
    let mut workspace = Workspace::new("claude-reject", &["alpha"]);
    let root = workspace.roots[0].clone();
    std::fs::write(root.join("a.txt"), DISK).unwrap();
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));
    let rejected = text_content(diff_outcome_text(IdeDiffOutcome::Rejected));
    let has_claude_tab = |application: &NativeApplication| {
        all_tabs(application, &project)
            .iter()
            .any(|tab| matches!(tab.kind, TabKind::ClaudeDiff { .. }))
    };

    application
        .services
        .state
        .settings
        .write()
        .ide_auto_open_diff = false;
    let pending = open_diff(application, &path);
    assert_eq!(answered(application, &context, pending), rejected);
    settle(application, &context);
    assert!(
        !has_claude_tab(application),
        "자동 열기를 끄면 탭 없이 거절"
    );
    application
        .services
        .state
        .settings
        .write()
        .ide_auto_open_diff = true;

    let pending = open_diff(application, &path);
    let output = until(
        application,
        &context,
        "reject tab",
        |application, output| {
            claude_tab(application, &project).is_some() && has_text(output, "GAMMA")
        },
    );
    let reject = text(application, "ide.rejectChanges");
    let (button, _) = node(&output, &reject).expect("reject 버튼");
    click(application, &context, button.center());
    assert_eq!(answered(application, &context, pending), rejected);
    assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), DISK);
    let message = text(application, "ide.diffRejected");
    until(application, &context, "rejected", |application, _| {
        application.toasts.is_showing(Kind::Info, &message) && !has_claude_tab(application)
    });

    let pending = open_diff(application, &path);
    until(application, &context, "close tab", |application, output| {
        claude_tab(application, &project).is_some() && has_text(output, "GAMMA")
    });
    let tab = claude_tab(application, &project).unwrap();
    application.request_tab_close(tab.id.clone());
    assert_eq!(
        answered(application, &context, pending),
        text_content(diff_outcome_text(IdeDiffOutcome::TabClosed))
    );

    let pending = open_diff(application, &path);
    until(
        application,
        &context,
        "ide close tab",
        |application, output| {
            claude_tab(application, &project).is_some() && has_text(output, "GAMMA")
        },
    );
    let tab = claude_tab(application, &project).unwrap();
    let TabKind::ClaudeDiff { request_id, .. } = &tab.kind else {
        unreachable!();
    };
    assert_eq!(application.git_state.claude.contents(request_id), PROPOSED);
    let closed = ide_tool(application, "close_tab", json!({"tab_name": TITLE}));
    assert_eq!(
        answered(application, &context, closed),
        text_content("TAB_CLOSED")
    );
    assert_eq!(
        answered(application, &context, pending),
        text_content(diff_outcome_text(IdeDiffOutcome::TabClosed))
    );
    settle(application, &context);
    assert_eq!(application.git_state.claude.contents(request_id), "");
    assert!(!has_claude_tab(application));
}

#[test]
fn 실제_app_파일_이력은_명령_탭_메뉴로_열리고_항목을_inline_diff로_보여준다() {
    let mut workspace = Workspace::new("file-history", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED)]);
    for (text, message) in [
        (EDITED, "test(fixture): second"),
        (THIRD, "test(fixture): third"),
    ] {
        std::fs::write(root.join("a.txt"), text).unwrap();
        workspace.git(0, &["commit", "-q", "-am", message]);
    }
    std::fs::write(root.join("b.txt"), ADDED).unwrap();
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let [a, b] = ["a.txt", "b.txt"].map(|name| real(&root.join(name)));
    let window = egui::ViewportId::ROOT;

    application
        .commands_state
        .palette_commands
        .push("git.openFileHistory".into());
    settle(application, &context);
    assert!(
        application.git_state.histories.get(window).is_none(),
        "편집기가 없으면 무동작"
    );

    assert!(application.submit(HostCommand::Editor(
        crate::domains::editor::Command::OpenFileTab {
            project: project.clone(),
            pane: None,
            path: a.clone(),
            preview: false,
        }
    )));
    until(application, &context, "file tab", |_, output| {
        has_text(output, "gamma")
    });
    application
        .commands_state
        .palette_commands
        .push("git.openFileHistory".into());
    let title = text(application, "git.fileHistory");
    let output = until(application, &context, "history list", |_, output| {
        has_text(output, "test(fixture): third")
            && has_text(output, "test(fixture): second")
            && has_text(output, "test(fixture): initial")
    });
    assert!(has_text(&output, &title));
    assert_eq!(
        application
            .git_state
            .histories
            .get(window)
            .map(|panel| panel.path.clone()),
        Some(a.clone())
    );

    let entry = text_shapes(&output, "test(fixture): second")
        .next()
        .unwrap()
        .pos;
    click(
        application,
        &context,
        entry + egui::vec2(CARET_INSET, CARET_INSET),
    );
    let owner = application
        .git_state
        .histories
        .get(window)
        .unwrap()
        .owner
        .clone();
    let output = until(
        application,
        &context,
        "inline diff",
        |application, output| {
            virtual_text(application, &owner, VirtualSide::Modified).as_deref() == Some(EDITED)
                && virtual_text(application, &owner, VirtualSide::Original).as_deref()
                    == Some(COMMITTED)
                && has_text(output, "a.txt")
        },
    );
    assert!(has_text(&output, "BETA"));
    let (back, _) = node(&output, &title).expect("뒤로 버튼");
    click(application, &context, back.center());
    until(
        application,
        &context,
        "list again",
        |application, output| {
            application
                .git_state
                .histories
                .get(window)
                .is_some_and(|panel| panel.selected.is_none())
                && has_text(output, "test(fixture): third")
        },
    );
    settle(application, &context);
    assert_eq!(
        virtual_text(application, &owner, VirtualSide::Modified),
        None,
        "목록으로 돌아가면 문서 해제"
    );
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    until(application, &context, "escape closes", |application, _| {
        application.git_state.histories.get(window).is_none()
    });

    let tab_bar = |output: &egui::FullOutput| {
        nodes(output, "a.txt")
            .map(|(rect, _)| rect)
            .find(|rect| rect.bottom() <= TITLE_HEIGHT + TAB_HEIGHT)
    };
    let output = until(application, &context, "tab bar", |_, output| {
        tab_bar(output).is_some()
    });
    frame(
        application,
        &context,
        pointer(
            tab_bar(&output).unwrap().center(),
            egui::PointerButton::Secondary,
        ),
    );
    let output = until(application, &context, "tab menu", |_, output| {
        node(output, &title).is_some()
    });
    click(
        application,
        &context,
        node(&output, &title).unwrap().0.center(),
    );
    until(
        application,
        &context,
        "menu history",
        |application, output| {
            application
                .git_state
                .histories
                .get(window)
                .is_some_and(|panel| panel.path == a)
                && has_text(output, "test(fixture): third")
        },
    );
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    until(application, &context, "closed", |application, _| {
        application.git_state.histories.get(window).is_none()
    });

    let sidebar_row = |output: &egui::FullOutput| {
        nodes(output, "b.txt")
            .map(|(rect, _)| rect)
            .min_by(|left, right| left.left().total_cmp(&right.left()))
    };
    let output = until(application, &context, "explorer row", |_, output| {
        sidebar_row(output).is_some()
    });
    frame(
        application,
        &context,
        pointer(
            sidebar_row(&output).unwrap().center(),
            egui::PointerButton::Secondary,
        ),
    );
    let output = until(application, &context, "explorer menu", |_, output| {
        node(output, &title).is_some()
    });
    click(
        application,
        &context,
        node(&output, &title).unwrap().0.center(),
    );
    until(
        application,
        &context,
        "explorer history",
        |application, _| {
            application
                .git_state
                .histories
                .get(window)
                .is_some_and(|panel| {
                    std::fs::canonicalize(&panel.path).ok() == Some(std::path::PathBuf::from(&b))
                })
        },
    );
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );

    assert!(application.submit(HostCommand::Editor(
        crate::domains::editor::Command::OpenFileTab {
            project: project.clone(),
            pane: None,
            path: b.clone(),
            preview: false,
        }
    )));
    until(application, &context, "untracked tab", |_, output| {
        has_text(output, "added")
    });
    application
        .commands_state
        .palette_commands
        .push("git.openFileHistory".into());
    let empty_history = text(application, "git.noFileHistory");
    until(
        application,
        &context,
        "untracked file",
        |application, output| {
            application
                .git_state
                .histories
                .get(window)
                .is_some_and(|panel| panel.path == b)
                && has_text(output, &empty_history)
        },
    );
}

fn claude_tabs_in(application: &NativeApplication, project: &ProjectId) -> Vec<TabId> {
    let layouts = application.services.state.layouts.read();
    taide_layout::service::all_roots(&layouts[project])
        .flat_map(taide_layout::service::collect_claude_diff_tab_ids)
        .collect()
}

#[test]
fn 실제_app_claude_diff_탭을_보조_창으로_옮기면_응답_없이_유지되고_그_창에서_accept한다() {
    use crate::application::auxiliary_window_tests::{
        choose_tab_menu, install_viewports, viewport_of,
    };
    let mut workspace = Workspace::new("claude-move", &["alpha"]);
    let root = workspace.roots[0].clone();
    std::fs::write(root.join("a.txt"), DISK).unwrap();
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let viewports = install_viewports(&context);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    settle(application, &context);
    let path = real(&root.join("a.txt"));

    let pending = open_diff(application, &path);
    let output = until(
        application,
        &context,
        "claude diff tab",
        |application, output| {
            claude_tab(application, &project).is_some() && has_text(output, "GAMMA")
        },
    );
    let tab = claude_tab(application, &project).unwrap();
    let TabKind::ClaudeDiff { request_id, .. } = &tab.kind else {
        unreachable!();
    };
    let request_id = request_id.clone();
    click(application, &context, leftmost(&output, "beta"));
    type_text(application, &context, "Q");
    let edited = "alpha  \nQbeta\nGAMMA\n";
    until(application, &context, "edited", |application, _| {
        virtual_text(application, &tab.id, VirtualSide::Modified).as_deref() == Some(edited)
    });

    let move_item = text(application, "tab.moveToNewWindow");
    choose_tab_menu(application, &context, &viewports, None, TITLE, &move_item);
    until(application, &context, "moved", |application, _| {
        let layouts = application.services.state.layouts.read();
        let layout = &layouts[&project];
        let main = taide_layout::service::collect_claude_diff_tab_ids(&layout.root);
        let auxiliary = layout
            .auxiliary_windows
            .first()
            .map(|window| taide_layout::service::collect_claude_diff_tab_ids(&window.root))
            .unwrap_or_default();
        !main.contains(&tab.id) && auxiliary.contains(&tab.id)
    });
    settle(application, &context);
    assert!(!pending.is_finished(), "다른 창 이동은 응답을 보내지 않음");
    assert_eq!(application.git_state.claude.contents(&request_id), PROPOSED);
    assert_eq!(
        virtual_text(application, &tab.id, VirtualSide::Modified).as_deref(),
        Some(edited),
        "native는 창 이동에도 가상 문서를 유지해 편집이 남습니다(원본은 model 재생성으로 잃음)"
    );

    let viewport = viewport_of(application, &project, 1);
    let accept = text(application, "ide.acceptChanges");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let button = loop {
        frame(application, &context, Vec::new());
        let found = viewports
            .borrow()
            .outputs
            .get(&viewport)
            .filter(|output| has_text(output, "GAMMA"))
            .and_then(|output| node(output, &accept))
            .map(|(button, _)| button.center());
        if let Some(center) = found {
            break center;
        }
        assert!(std::time::Instant::now() < deadline, "보조 창 accept 버튼");
        std::thread::yield_now();
    };
    viewports
        .borrow_mut()
        .events
        .insert(viewport, pointer(button, egui::PointerButton::Primary));
    frame(application, &context, Vec::new());
    assert_eq!(
        answered(application, &context, pending),
        text_content(diff_outcome_text(IdeDiffOutcome::Saved))
    );
    assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), edited);
    settle(application, &context);
    assert!(claude_tabs_in(application, &project).is_empty());
}

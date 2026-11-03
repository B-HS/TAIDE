use std::path::PathBuf;

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_native_ui::toast::Kind;

use super::base_tests::{
    Workspace, frame, has_text, node, nodes, settle, show_git, text, text_shapes, until,
};
use super::fixture;
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};
use crate::presentation;

const ORIGINAL: &str = "one\n";
const FEATURE: &str = "feature\n";
const EDITED: &str = "edited\n";
const SECOND: &str = "second\n";
const REMOTE_NAME: &str = "origin";
const HEADER_HALF: f32 = 16.0;
const AWAY: egui::Pos2 = egui::pos2(1280.0, 800.0);

fn launch_git(workspace: &mut Workspace) -> egui::Context {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    settle(application, &context);
    show_git(application, &context);
    context
}

fn named(application: &NativeApplication, key: &str, name: &str) -> String {
    presentation::message(&application.locale, key, &[("name", name)])
}

fn click(application: &mut NativeApplication, context: &egui::Context, target: egui::Rect) {
    frame(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
}

fn away(application: &mut NativeApplication, context: &egui::Context) {
    frame(application, context, vec![egui::Event::PointerMoved(AWAY)]);
}

fn typed(application: &mut NativeApplication, context: &egui::Context, text: &str) {
    frame(application, context, vec![egui::Event::Text(text.into())]);
}

fn pressed(application: &mut NativeApplication, context: &egui::Context, pressed: egui::Key) {
    frame(application, context, key(pressed, egui::Modifiers::NONE));
}

/// A picker item named `label`: cmdk items carry a selected state, which tells them apart from
/// the trigger, tooltips and painted names.
fn item(output: &egui::FullOutput, label: &str) -> Option<(egui::Rect, egui::accesskit::Node)> {
    nodes(output, label)
        .find(|(_, node)| node.toggled().is_some())
        .map(|(rect, node)| (rect, node.clone()))
}

fn items(output: &egui::FullOutput, labels: &[&str]) -> Vec<String> {
    labels
        .iter()
        .filter(|label| item(output, label).is_some())
        .map(|label| (*label).to_owned())
        .collect()
}

fn trigger(output: &egui::FullOutput, label: &str) -> Option<(egui::Rect, egui::accesskit::Node)> {
    nodes(output, label)
        .find(|(_, node)| node.is_expanded().is_some())
        .map(|(rect, node)| (rect, node.clone()))
}

fn buttons<'a>(
    output: &'a egui::FullOutput,
    label: &'a str,
) -> impl Iterator<Item = (egui::Rect, &'a egui::accesskit::Node)> + 'a {
    nodes(output, label).filter(|(_, node)| node.role() != egui::accesskit::Role::Tooltip)
}

fn button(output: &egui::FullOutput, label: &str) -> Option<(egui::Rect, egui::accesskit::Node)> {
    buttons(output, label)
        .next()
        .map(|(rect, node)| (rect, node.clone()))
}

fn is_active(output: &egui::FullOutput, label: &str) -> bool {
    item(output, label)
        .is_some_and(|(_, node)| node.toggled() == Some(egui::accesskit::Toggled::True))
}

fn focused_role(output: &egui::FullOutput) -> Option<egui::accesskit::Role> {
    let update = output.platform_output.accesskit_update.as_ref()?;
    update
        .nodes
        .iter()
        .find(|(id, _)| *id == update.focus)
        .map(|(_, node)| node.role())
}

/// Text painted on the header row that holds the branch switcher.
fn header_texts(output: &egui::FullOutput, switcher: &str, wanted: &str) -> usize {
    let Some((switcher, _)) = trigger(output, switcher) else {
        return 0;
    };
    text_shapes(output, wanted)
        .filter(|shape| {
            (shape.visual_bounding_rect().center().y - switcher.center().y).abs() < HEADER_HALF
        })
        .count()
}

/// Text painted on the row `row` covers, such as a section header's badge.
fn row_texts(output: &egui::FullOutput, row: egui::Rect, wanted: &str) -> usize {
    text_shapes(output, wanted)
        .filter(|shape| {
            row.y_range()
                .contains(shape.visual_bounding_rect().center().y)
        })
        .count()
}

fn focused_label(output: &egui::FullOutput) -> Option<String> {
    let update = output.platform_output.accesskit_update.as_ref()?;
    update
        .nodes
        .iter()
        .find(|(id, _)| *id == update.focus)
        .and_then(|(_, node)| node.label().map(str::to_owned))
}

fn open_picker(application: &mut NativeApplication, context: &egui::Context, switch: &str) {
    away(application, context);
    let output = settle(application, context);
    let (switcher, _) = trigger(&output, switch).expect("switcher");
    click(application, context, switcher);
}

fn head(workspace: &Workspace) -> String {
    workspace.git(0, &["rev-parse", "--abbrev-ref", "HEAD"])
}

fn refresh(application: &NativeApplication, project: &ProjectId) {
    application
        .services
        .events
        .publish(AppEvent::GitStatusChanged {
            project_id: project.clone(),
        });
    application
        .services
        .events
        .publish(AppEvent::GitRefsChanged {
            project_id: project.clone(),
        });
}

fn shows_toast(application: &NativeApplication, kind: Kind, title: &str) -> bool {
    application.toasts.is_showing(kind, title)
}

fn errors(application: &NativeApplication) -> usize {
    application
        .toasts
        .inspection()
        .iter()
        .filter(|toast| toast.kind == Kind::Error)
        .count()
}

fn bare_remote(workspace: &Workspace) -> PathBuf {
    let directory = workspace.roots[0].parent().unwrap().to_path_buf();
    let remote = directory.join("remote.git");
    let empty = workspace.empty.to_str().unwrap();
    fixture::git(
        &directory,
        &workspace.empty,
        &[
            "init",
            "--bare",
            "--initial-branch=main",
            "--template",
            empty,
            remote.to_str().unwrap(),
        ],
    );
    fixture::git(
        &directory,
        &workspace.empty,
        &[
            "--git-dir",
            remote.to_str().unwrap(),
            "config",
            "--local",
            "core.hooksPath",
            empty,
        ],
    );
    workspace.git(0, &["remote", "add", REMOTE_NAME, remote.to_str().unwrap()]);
    remote
}

fn remote_head(workspace: &Workspace, remote: &std::path::Path) -> String {
    workspace.git(
        0,
        &[
            "--git-dir",
            remote.to_str().unwrap(),
            "rev-parse",
            "refs/heads/main",
        ],
    )
}

#[test]
fn 실제_app_브랜치_스위처는_필터와_그룹과_키보드로_체크아웃_원격_체크아웃_생성을_하고_결과를_toast로_알린다()
 {
    let mut workspace = Workspace::new("branches", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ORIGINAL)]);
    workspace.git(0, &["checkout", "-b", "feature"]);
    std::fs::write(root.join("a.txt"), FEATURE).unwrap();
    workspace.git(0, &["commit", "-am", "test(fixture): feature"]);
    workspace.git(0, &["checkout", "main"]);
    bare_remote(&workspace);
    workspace.git(0, &["push", REMOTE_NAME, "main:feature-remote"]);
    workspace.git(0, &["fetch", REMOTE_NAME]);
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let switch = text(application, "git.branchSwitch");
    let no_repository = text(application, "git.noRepositoryLabel");

    let output = until(application, &context, "switcher", |_, output| {
        trigger(output, &switch).is_some()
    });
    assert!(
        trigger(&output, &switch).unwrap().1.is_disabled(),
        "no branch before the first status"
    );
    assert_eq!(header_texts(&output, &switch, &no_repository), 1);

    let output = until(application, &context, "branch", |_, output| {
        header_texts(output, &switch, "main") == 1
            && trigger(output, &switch).is_some_and(|(_, node)| !node.is_disabled())
    });
    let (switcher, _) = trigger(&output, &switch).unwrap();
    frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(switcher.center())],
    );
    until(application, &context, "switcher tooltip", |_, output| {
        text_shapes(output, &switch)
            .any(|shape| shape.visual_bounding_rect().top() > switcher.bottom())
    });

    open_picker(application, &context, &switch);
    let all = ["main", "feature", "origin/feature-remote"];
    let local = text(application, "git.branchLocal");
    let remote_heading = text(application, "git.branchRemote");
    let output = until(application, &context, "picker", |_, output| {
        items(output, &all) == all && focused_role(output) == Some(egui::accesskit::Role::TextInput)
    });
    assert!(
        is_active(&output, "feature"),
        "the first item in the backend's sorted ref order starts highlighted"
    );
    assert!(has_text(&output, &local) && has_text(&output, &remote_heading));
    assert!(has_text(
        &output,
        &text(application, "git.checkoutRemoteBranch")
    ));
    assert_eq!(
        trigger(&output, &switch).unwrap().1.is_expanded(),
        Some(true)
    );

    typed(application, &context, "FEAT");
    let create = named(application, "git.branchCreateNamed", "FEAT");
    let output = until(application, &context, "filtered", |_, output| {
        items(output, &all) == ["feature", "origin/feature-remote"]
            && item(output, &create).is_some()
    });
    assert!(
        is_active(&output, &create),
        "the create item leads the list"
    );
    pressed(application, &context, egui::Key::ArrowDown);
    let output = settle(application, &context);
    assert!(is_active(&output, "feature"));
    pressed(application, &context, egui::Key::ArrowDown);
    pressed(application, &context, egui::Key::ArrowDown);
    let output = settle(application, &context);
    assert!(
        is_active(&output, "origin/feature-remote"),
        "no wrap at the end"
    );
    pressed(application, &context, egui::Key::Home);
    let output = settle(application, &context);
    assert!(is_active(&output, &create));
    pressed(application, &context, egui::Key::ArrowDown);
    pressed(application, &context, egui::Key::Enter);
    let switched = named(application, "git.branchSwitched", "feature");
    until(application, &context, "checkout", |application, output| {
        shows_toast(application, Kind::Success, &switched)
            && header_texts(output, &switch, "feature") == 1
            && item(output, "feature").is_none()
    });
    assert_eq!(head(&workspace), "feature");
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        FEATURE
    );

    let application = workspace.fixture.application.as_mut().unwrap();
    open_picker(application, &context, &switch);
    let output = until(application, &context, "filter cleared", |_, output| {
        items(output, &all) == all
    });
    assert!(is_active(&output, "feature"));
    typed(application, &context, " feature ");
    let output = until(application, &context, "exact name", |_, output| {
        items(output, &all) == ["feature", "origin/feature-remote"]
    });
    assert!(
        item(
            &output,
            &named(application, "git.branchCreateNamed", "feature")
        )
        .is_none(),
        "an exact name offers no create item"
    );
    assert!(!has_text(&output, &text(application, "git.branchNotFound")));
    pressed(application, &context, egui::Key::Escape);
    let output = until(application, &context, "escape closes", |_, output| {
        items(output, &all).is_empty()
    });
    assert_eq!(
        trigger(&output, &switch).unwrap().1.is_expanded(),
        Some(false)
    );
    assert_eq!(focused_label(&output).as_deref(), Some(switch.as_str()));
    open_picker(application, &context, &switch);
    let output = until(application, &context, "filter kept", |_, output| {
        items(output, &all) == ["feature", "origin/feature-remote"]
    });
    let (remote, _) = item(&output, "origin/feature-remote").unwrap();
    frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(remote.center())],
    );
    let output = settle(application, &context);
    assert!(
        is_active(&output, "origin/feature-remote"),
        "the pointer highlights"
    );
    click(application, &context, remote);
    let switched = named(application, "git.branchSwitched", "origin/feature-remote");
    until(
        application,
        &context,
        "remote checkout",
        |application, output| {
            shows_toast(application, Kind::Success, &switched)
                && header_texts(output, &switch, "feature-remote") == 1
        },
    );
    assert_eq!(head(&workspace), "feature-remote");
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        ORIGINAL
    );

    let application = workspace.fixture.application.as_mut().unwrap();
    open_picker(application, &context, &switch);
    until(application, &context, "reopen", |_, output| {
        item(output, "main").is_some()
    });
    typed(application, &context, "topic");
    let create = named(application, "git.branchCreateNamed", "topic");
    let output = until(application, &context, "create item", |_, output| {
        item(output, &create).is_some() && is_active(output, &create)
    });
    assert!(
        !has_text(&output, &local) && !has_text(&output, &remote_heading),
        "groups without a match are hidden"
    );
    pressed(application, &context, egui::Key::Enter);
    let switched = named(application, "git.branchSwitched", "topic");
    until(application, &context, "created", |application, output| {
        shows_toast(application, Kind::Success, &switched)
            && header_texts(output, &switch, "topic") == 1
    });
    assert_eq!(head(&workspace), "topic");

    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    let before = errors(application);
    open_picker(application, &context, &switch);
    let output = until(application, &context, "picker again", |_, output| {
        item(output, "feature").is_some()
    });
    click(application, &context, item(&output, "feature").unwrap().0);
    until(
        application,
        &context,
        "checkout conflict",
        |application, _| errors(application) > before,
    );
    assert_eq!(head(&workspace), "topic");
    assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), EDITED);

    workspace.git(0, &["checkout", "--detach"]);
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    until(application, &context, "detached", |_, output| {
        header_texts(output, &switch, "HEAD") == 1
    });
}

#[test]
fn 실제_app_브랜치가_하나도_없으면_목록은_찾을_수_없음을_보인다() {
    let mut workspace = Workspace::new("unborn", &["alpha"]);
    fixture::init(&workspace.roots[0], &workspace.empty);
    let context = launch_git(&mut workspace);
    let application = workspace.fixture.application.as_mut().unwrap();
    let switch = text(application, "git.branchSwitch");
    let output = until(application, &context, "unborn branch", |_, output| {
        header_texts(output, &switch, "main") == 1
    });
    click(application, &context, trigger(&output, &switch).unwrap().0);
    let empty = text(application, "git.branchNotFound");
    until(application, &context, "no branches", |_, output| {
        has_text(output, &empty)
    });
    typed(application, &context, "next");
    let create = named(application, "git.branchCreateNamed", "next");
    let output = until(application, &context, "create only", |_, output| {
        item(output, &create).is_some()
    });
    assert!(
        !has_text(&output, &empty),
        "a create item is not an empty list"
    );
}

#[test]
fn 실제_app_헤더는_원격이_있을_때만_ahead_behind와_sync와_원격_이름을_보이고_sync는_pull_뒤_push한다()
 {
    let mut workspace = Workspace::new("sync", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ORIGINAL)]);
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let switch = text(application, "git.branchSwitch");
    let sync = text(application, "git.sync");
    until(application, &context, "local only", |_, output| {
        header_texts(output, &switch, "main") == 1
    });
    let output = settle(application, &context);
    assert!(button(&output, &sync).is_none(), "no remote, no sync");
    assert_eq!(header_texts(&output, &switch, REMOTE_NAME), 0);

    let remote = bare_remote(&workspace);
    workspace.git(0, &["config", "--local", "branch.main.remote", REMOTE_NAME]);
    workspace.git(
        0,
        &["config", "--local", "branch.main.merge", "refs/heads/main"],
    );
    workspace.git(0, &["push"]);
    workspace.git(0, &["fetch", REMOTE_NAME]);
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    let output = until(application, &context, "remote", |_, output| {
        header_texts(output, &switch, REMOTE_NAME) == 1 && button(output, &sync).is_some()
    });
    assert_eq!(header_texts(&output, &switch, "1"), 0, "even with upstream");

    let local = workspace.git(0, &["rev-parse", "HEAD"]);
    std::fs::write(root.join("b.txt"), SECOND).unwrap();
    workspace.git(0, &["add", "--", "b.txt"]);
    workspace.git(0, &["commit", "-m", "test(fixture): pushed then dropped"]);
    workspace.git(0, &["push"]);
    workspace.git(0, &["reset", "--hard", &local]);
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    until(application, &context, "behind", |application, output| {
        header_texts(output, &switch, "1") == 1
            && application
                .git_state
                .cache
                .status_result(&project)
                .and_then(Result::ok)
                .is_some_and(|status| (status.ahead, status.behind) == (0, 1))
    });

    let state = application.services.state.clone();
    let guard = state.begin_mutation_blocking();
    let output = settle(application, &context);
    click(application, &context, button(&output, &sync).unwrap().0);
    until(application, &context, "syncing", |application, output| {
        application.git_state.syncing.contains(&project)
            && button(output, &sync).is_some_and(|(_, node)| node.is_disabled())
    });
    drop(guard);
    let pulled = workspace.git(0, &["rev-parse", "refs/remotes/origin/main"]);
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "pulled", |application, output| {
        !application.git_state.syncing.contains(&project)
            && header_texts(output, &switch, "1") == 0
            && button(output, &sync).is_some_and(|(_, node)| !node.is_disabled())
    });
    assert_eq!(workspace.git(0, &["rev-parse", "HEAD"]), pulled);

    std::fs::write(root.join("c.txt"), SECOND).unwrap();
    workspace.git(0, &["add", "--", "c.txt"]);
    workspace.git(0, &["commit", "-m", "test(fixture): ahead"]);
    let ahead = workspace.git(0, &["rev-parse", "HEAD"]);
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    let output = until(application, &context, "ahead", |application, output| {
        header_texts(output, &switch, "1") == 1
            && application
                .git_state
                .cache
                .status_result(&project)
                .and_then(Result::ok)
                .is_some_and(|status| (status.ahead, status.behind) == (1, 0))
    });
    let before = errors(application);
    click(application, &context, button(&output, &sync).unwrap().0);
    until(application, &context, "pushed", |application, output| {
        !application.git_state.syncing.contains(&project) && header_texts(output, &switch, "1") == 0
    });
    assert_eq!(errors(application), before);
    assert_eq!(remote_head(&workspace, &remote), ahead);

    std::fs::write(root.join("d.txt"), SECOND).unwrap();
    workspace.git(0, &["add", "--", "d.txt"]);
    workspace.git(0, &["commit", "-m", "test(fixture): not pushed"]);
    let moved = remote.with_extension("moved");
    std::fs::rename(&remote, &moved).unwrap();
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    let output = until(application, &context, "ahead again", |_, output| {
        header_texts(output, &switch, "1") == 1
    });
    click(application, &context, button(&output, &sync).unwrap().0);
    until(application, &context, "pull failure", |application, _| {
        !application.git_state.syncing.contains(&project) && errors(application) > before
    });
    std::fs::rename(&moved, &remote).unwrap();
    assert_eq!(remote_head(&workspace, &remote), ahead, "push never ran");
}

#[test]
fn 실제_app_stash는_변경이_있을_때만_넣고_섹션은_배지와_기본_접힘과_apply_drop을_원본대로_한다() {
    let mut workspace = Workspace::new("stash", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ORIGINAL), ("b.txt", ORIGINAL)]);
    let context = launch_git(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let switch = text(application, "git.branchSwitch");
    let stash_push = text(application, "git.stashPush");
    let stash = text(application, "git.stash");
    let clean = text(application, "git.noChanges");
    let output = until(application, &context, "clean", |_, output| {
        has_text(output, &clean) && header_texts(output, &switch, "main") == 1
    });
    assert!(button(&output, &stash_push).is_some_and(|(_, node)| node.is_disabled()));
    assert!(node(&output, &stash).is_none(), "no stash, no section");

    let pushed = text(application, "git.stashPushed");
    for name in ["a.txt", "b.txt"] {
        std::fs::write(root.join(name), EDITED).unwrap();
        let application = workspace.fixture.application.as_mut().unwrap();
        refresh(application, &project);
        let output = until(application, &context, "dirty", |_, output| {
            node(output, name).is_some()
                && button(output, &stash_push).is_some_and(|(_, node)| !node.is_disabled())
        });
        click(
            application,
            &context,
            button(&output, &stash_push).unwrap().0,
        );
        until(application, &context, "stashed", |application, output| {
            shows_toast(application, Kind::Success, &pushed)
                && node(output, name).is_none()
                && has_text(output, &clean)
        });
        assert_eq!(std::fs::read_to_string(root.join(name)).unwrap(), ORIGINAL);
    }
    let application = workspace.fixture.application.as_mut().unwrap();
    let output = until(application, &context, "two stashes", |_, output| {
        node(output, &stash).is_some_and(|(row, _)| row_texts(output, row, "2") == 1)
    });
    let (header, section) = node(&output, &stash).unwrap();
    assert_eq!(
        section.is_expanded(),
        Some(false),
        "stashes start collapsed"
    );
    let apply = text(application, "git.stashApply");
    let drop_label = text(application, "git.stashDrop");
    assert!(node(&output, &apply).is_none());

    click(application, &context, header);
    let output = until(application, &context, "expanded", |_, output| {
        buttons(output, &apply).count() == 2 && buttons(output, &drop_label).count() == 2
    });
    let state = application.services.state.clone();
    let guard = state.begin_mutation_blocking();
    let (first, _) = buttons(&output, &apply)
        .min_by(|left, right| left.0.top().total_cmp(&right.0.top()))
        .unwrap();
    click(application, &context, first);
    until(
        application,
        &context,
        "stash busy",
        |application, output| {
            application.git_state.stashing.contains_key(&project)
                && buttons(output, &apply).count() == 2
                && buttons(output, &apply).all(|(_, node)| node.is_disabled())
                && buttons(output, &drop_label).all(|(_, node)| node.is_disabled())
        },
    );
    drop(guard);
    let application = workspace.fixture.application.as_mut().unwrap();
    let output = until(application, &context, "applied", |_, output| {
        node(output, "b.txt").is_some()
            && buttons(output, &apply).count() == 2
            && buttons(output, &apply).all(|(_, node)| !node.is_disabled())
    });
    assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), EDITED);
    assert!(!shows_toast(application, Kind::Success, &apply));

    let (first, _) = buttons(&output, &drop_label)
        .min_by(|left, right| left.0.top().total_cmp(&right.0.top()))
        .unwrap();
    click(application, &context, first);
    until(application, &context, "dropped", |_, output| {
        buttons(output, &apply).count() == 1
    });
    assert_eq!(workspace.git(0, &["stash", "list"]).lines().count(), 1);

    let application = workspace.fixture.application.as_mut().unwrap();
    let output = settle(application, &context);
    let (row, _) = nodes(&output, "b.txt")
        .find(|(_, node)| node.toggled().is_some())
        .unwrap();
    click(application, &context, row);
    away(application, &context);
    pressed(application, &context, egui::Key::ArrowDown);
    let output = until(application, &context, "stash header focus", |_, output| {
        focused_label(output).as_deref() == Some(stash.as_str())
    });
    assert_eq!(node(&output, &stash).unwrap().1.is_expanded(), Some(true));
    pressed(application, &context, egui::Key::ArrowLeft);
    until(application, &context, "collapse by key", |_, output| {
        node(output, &stash).is_some_and(|(_, node)| node.is_expanded() == Some(false))
            && buttons(output, &apply).count() == 0
    });
    pressed(application, &context, egui::Key::ArrowUp);
    until(application, &context, "back to the row", |_, output| {
        focused_label(output).as_deref() == Some("b.txt")
    });
}

use std::path::Path;

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::git::HunkKind;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::PaneNode;
use taide_native_editor::document::{DocumentKey, Edit, UndoGroup};
use taide_native_editor::store::Transaction;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::intents::git::{ConflictAction, Intent};
use taide_native_ui::toast::Kind;

use super::Key;
use super::base_tests::{Workspace, frame, node, settle, text, text_shapes, until};
use super::fixture;
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};
use crate::host::HostCommand;

const COMMITTED: &str = "a1\na2\na3\na4\na5\na6\na7\n";
const EDITED: &str = "a1\nA2\na3\na5\na6\na7\na8\n";
const BLAME_SECOND: &str = "a1\na2\nb3\na4\na5\na6\na7\n";
const BLAME_EDITED: &str = "a1\nA2\nb3\na5\na6\na7\na8\n";
const BLAME_REWRITTEN: &str = "a1\nA2\nb3\na4\na5\na6\na7\n";
const SECOND_SUMMARY: &str = "test(fixture): second";
const OTHER: &str = "b1\nb2\n";
const STAGE_COMMITTED: &str = "c1\nc2\nc3\nc4\nc5\nc6\n";
const STAGE_EDITED: &str = "c1\nC2\nC3\nc4\nc5\nC6\n";
const BASE: &str = "head\nshared\nk1\nk2\nk3\nk4\nk5\nother\ntail\n";
const OURS: &str = "head\nours1\nk1\nk2\nk3\nk4\nk5\nours2\ntail\n";
const THEIRS: &str = "head\ntheirs1\nk1\nk2\nk3\nk4\nk5\ntheirs2\ntail\n";
const MERGED: &str = "head\n<<<<<<< HEAD\nours1\n=======\ntheirs1\n>>>>>>> feature\nk1\nk2\nk3\nk4\nk5\n<<<<<<< HEAD\nours2\n=======\ntheirs2\n>>>>>>> feature\ntail\n";
const FIRST_RESOLVED: &str = "head\nours1\nk1\nk2\nk3\nk4\nk5\n<<<<<<< HEAD\nours2\n=======\ntheirs2\n>>>>>>> feature\ntail\n";
const RESOLVED: &str = "head\nours1\nk1\nk2\nk3\nk4\nk5\ntheirs2\ntail\n";
const SHORTENED: &str = "head\ntail\n";
const SUMMARY: &str = "test(fixture): initial";
const OVERLAY_PREFIX: &str = "    ";
const LANE_INSET: f32 = 2.0;
const LANE_BAR_WIDTH: f32 = 3.0;
const SAME_ROW: f32 = 4.0;
const FEATURE: &str = "feature";

fn real(path: &Path) -> String {
    path.parent()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join(path.file_name().unwrap())
        .to_str()
        .unwrap()
        .into()
}

fn launch_editor(workspace: &mut Workspace) -> egui::Context {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    settle(workspace.fixture.application.as_mut().unwrap(), &context);
    context
}

fn open_file(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    path: &str,
    shown: &str,
) -> egui::FullOutput {
    assert!(application.submit(HostCommand::Editor(
        crate::domains::editor::Command::OpenFileTab {
            project: project.clone(),
            pane: None,
            path: path.into(),
            preview: false,
        }
    )));
    until(application, context, path, |_, output| {
        has(output, |text| text == shown)
    })
}

fn has(output: &egui::FullOutput, matches: impl Fn(&str) -> bool) -> bool {
    output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, egui::Shape::Text(shape) if matches(shape.galley.text()))
    })
}

fn footer_blame(text: &str) -> bool {
    text.starts_with("You, ") && text.ends_with(SUMMARY)
}

fn footer_second(text: &str) -> bool {
    text.starts_with("You, ") && text.ends_with(SECOND_SUMMARY)
}

fn overlay_blame(text: &str) -> bool {
    text.starts_with(OVERLAY_PREFIX) && text.ends_with(SUMMARY)
}

fn overlay_second(text: &str) -> bool {
    text.starts_with(OVERLAY_PREFIX) && text.ends_with(SECOND_SUMMARY)
}

fn active(application: &NativeApplication, project: &ProjectId) -> (PaneId, TabId) {
    let layouts = application.services.state.layouts.read();
    let layout = &layouts[project];
    let mut stack = vec![&layout.root];
    while let Some(node) = stack.pop() {
        match node {
            PaneNode::Split { children, .. } => stack.extend(children),
            PaneNode::Leaf { id, active, .. } if *id == layout.focused_pane => {
                return (id.clone(), active.clone().unwrap());
            }
            PaneNode::Leaf { .. } => {}
        }
    }
    panic!("focused pane")
}

fn view(application: &NativeApplication, project: &ProjectId) -> ViewId {
    let (pane, tab) = active(application, project);
    application
        .store
        .views()
        .find(&ViewKey {
            window: crate::application::WINDOW_LABEL.into(),
            pane,
            tab,
        })
        .unwrap()
}

fn document_text(application: &NativeApplication, view: ViewId) -> String {
    let document = application.store.views().get(view).unwrap().document;
    application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string()
}

fn select(application: &mut NativeApplication, view: ViewId, anchor: usize, head: usize) {
    let current = application.store.views().get(view).unwrap();
    let (scroll, folds) = (current.scroll.clone(), current.folds.clone());
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor, head }],
            },
            scroll,
            folds,
        )
        .unwrap();
}

/// The gutter lane on the row drawing `word` next to line number `line`.
fn lane_at(output: &egui::FullOutput, word: &str, line: usize) -> egui::Pos2 {
    let line = line.to_string();
    text_shapes(output, word)
        .find_map(|row| {
            text_shapes(output, &line)
                .filter(|number| {
                    (number.pos.y - row.pos.y).abs() < SAME_ROW && number.pos.x < row.pos.x
                })
                .max_by(|left, right| left.pos.x.total_cmp(&right.pos.x))
                .map(|number| {
                    egui::pos2(
                        number.pos.x + number.galley.size().x + LANE_INSET,
                        row.pos.y + row.galley.size().y / 2.0,
                    )
                })
        })
        .unwrap_or_else(|| panic!("{word} 를 {line} 행에 그리지 않았습니다"))
}

fn click_label(application: &mut NativeApplication, context: &egui::Context, label: &str) {
    let output = until(application, context, label, |_, output| {
        node(output, label).is_some()
    });
    let (rect, _) = node(&output, label).unwrap();
    frame(
        application,
        context,
        pointer(rect.center(), egui::PointerButton::Primary),
    );
}

fn run(application: &mut NativeApplication, context: &egui::Context, command: &str) {
    application
        .commands_state
        .palette_commands
        .push(command.into());
    frame(application, context, Vec::new());
}

fn menu_action(
    application: &mut NativeApplication,
    context: &egui::Context,
    view: ViewId,
    label: &str,
) {
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    frame(application, context, Vec::new());
    let opened = frame(
        application,
        context,
        key(egui::Key::F10, egui::Modifiers::SHIFT),
    );
    assert!(
        has(&opened, |text| text == label),
        "편집기 메뉴에 {label} 항목이 없습니다"
    );
    let mut events = vec![egui::Event::Text(label.into())];
    events.extend(key(egui::Key::Enter, egui::Modifiers::NONE));
    frame(application, context, events);
}

fn lane_marks(application: &NativeApplication, output: &egui::FullOutput, kind: HunkKind) -> usize {
    let appearance = application.git_state.editors.appearance.unwrap();
    let color = match kind {
        HunkKind::Added => appearance.added,
        HunkKind::Modified => appearance.modified,
        HunkKind::Deleted => appearance.deleted,
    };
    output
        .shapes
        .iter()
        .filter(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => rect.fill == color && rect.rect.width() == LANE_BAR_WIDTH,
            egui::Shape::Path(path) => kind == HunkKind::Deleted && path.fill == color,
            _ => false,
        })
        .count()
}

fn merge_conflict(workspace: &Workspace) {
    let root = &workspace.roots[0];
    fixture::repository(root, &workspace.empty, &[("m.txt", BASE)]);
    workspace.git(0, &["checkout", "-q", "-b", FEATURE]);
    std::fs::write(root.join("m.txt"), THEIRS).unwrap();
    workspace.git(0, &["commit", "-q", "-am", "test(fixture): theirs"]);
    workspace.git(0, &["checkout", "-q", "main"]);
    std::fs::write(root.join("m.txt"), OURS).unwrap();
    workspace.git(0, &["commit", "-q", "-am", "test(fixture): ours"]);
    let merged = std::process::Command::new("git")
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("-c")
        .arg(format!("core.hooksPath={}", workspace.empty.display()))
        .arg("-c")
        .arg("core.fsmonitor=false")
        .args(["merge", "--no-edit", FEATURE])
        .output()
        .unwrap();
    assert!(!merged.status.success(), "충돌 병합이어야 합니다");
    assert_eq!(std::fs::read_to_string(root.join("m.txt")).unwrap(), MERGED);
}

#[test]
fn 실제_app_파일_편집기는_gutter_막대와_blame_footer_overlay를_원본대로_그린다() {
    let mut workspace = Workspace::new("editor-blame", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED), ("b.txt", OTHER)]);
    std::fs::write(root.join("a.txt"), BLAME_SECOND).unwrap();
    workspace.git(0, &["commit", "-q", "-am", SECOND_SUMMARY]);
    std::fs::write(root.join("a.txt"), BLAME_EDITED).unwrap();
    let context = launch_editor(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let [a, b] = ["a.txt", "b.txt"].map(|name| real(&root.join(name)));

    open_file(application, &context, &project, &a, "A2");
    let output = until(application, &context, "gutter", |application, output| {
        application.git_state.cache.gutter(&project, &a).len() == 3
            && lane_marks(application, output, HunkKind::Deleted) == 1
    });
    assert_eq!(lane_marks(application, &output, HunkKind::Modified), 1);
    assert_eq!(lane_marks(application, &output, HunkKind::Added), 1);
    let modified = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect)
                if rect.fill == application.git_state.editors.appearance.unwrap().modified
                    && rect.rect.width() == LANE_BAR_WIDTH =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .unwrap();
    let row = text_shapes(&output, "A2").next().unwrap();
    assert!(
        (modified.center().y - (row.pos.y + row.galley.size().y / 2.0)).abs() < SAME_ROW,
        "수정 막대는 2행에 있습니다"
    );

    std::fs::write(root.join("a.txt"), BLAME_REWRITTEN).unwrap();
    application.services.events.publish(AppEvent::FsChanged {
        project_id: project.clone(),
        change: FsChange {
            kind: FsChangeKind::Modified,
            paths: vec![a.clone()],
            from_app: false,
        },
    });
    until(
        application,
        &context,
        "gutter refreshed",
        |application, _| application.git_state.cache.gutter(&project, &a).len() == 1,
    );

    let output = until(application, &context, "footer", |_, output| {
        has(output, footer_blame)
    });
    assert!(!has(&output, footer_second));
    let view = view(application, &project);
    let second = document_text(application, view).find("b3").unwrap();
    select(application, view, second, second);
    let output = frame(application, &context, Vec::new());
    assert!(
        has(&output, footer_blame),
        "새 줄을 읽는 동안 이전 문구를 유지합니다"
    );
    until(application, &context, "second footer", |_, output| {
        has(output, footer_second) && !has(output, footer_blame)
    });

    run(application, &context, "git.toggleBlame");
    let output = until(application, &context, "overlay", |_, output| {
        has(output, overlay_blame) && has(output, overlay_second)
    });
    let a1 = text_shapes(&output, "a1").next().unwrap().pos;
    let overlay = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if overlay_blame(shape.galley.text()) => Some(shape),
            _ => None,
        })
        .unwrap();
    assert!(overlay.pos.x > a1.x, "줄 끝 뒤에 그립니다");
    assert!(
        overlay.galley.job.sections[0].format.italics,
        "overlay는 기울임입니다"
    );
    assert_eq!(
        text_shapes(&output, "a1").next().unwrap().pos,
        a1,
        "overlay는 줄 글자 위치를 바꾸지 않습니다"
    );
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        second,
        "overlay는 커서를 옮기지 않습니다"
    );

    let label = text(application, "git.toggleBlame");
    menu_action(application, &context, view, &label);
    let output = settle(application, &context);
    assert!(!has(&output, overlay_blame), "메뉴로 다시 누르면 꺼집니다");

    run(application, &context, "git.toggleBlame");
    until(application, &context, "overlay again", |_, output| {
        has(output, overlay_blame)
    });
    assert!(application.submit(HostCommand::Editor(
        crate::domains::editor::Command::OpenFileTab {
            project: project.clone(),
            pane: None,
            path: b.clone(),
            preview: false,
        }
    )));
    let output = until(application, &context, "other file", |_, output| {
        has(output, |text| text == "b1")
    });
    assert!(
        !has(&output, |text| text.starts_with("You, ")),
        "파일을 바꾸면 footer를 즉시 비웁니다"
    );
    assert!(!has(&output, |text| text.starts_with(OVERLAY_PREFIX)));
    let output = until(application, &context, "other footer", |_, output| {
        has(output, footer_blame)
    });
    assert!(
        !has(&output, overlay_blame),
        "경로가 바뀌면 overlay가 꺼집니다"
    );
}

#[test]
fn 실제_app_gutter_누름은_hunk_discard_대화상자를_열고_편집기_메뉴는_선택_줄과_커서_hunk를_stage한다()
 {
    let mut workspace = Workspace::new("editor-gutter", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(
        &root,
        &empty,
        &[("a.txt", COMMITTED), ("c.txt", STAGE_COMMITTED)],
    );
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    std::fs::write(root.join("c.txt"), STAGE_EDITED).unwrap();
    let context = launch_editor(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let [a, c] = ["a.txt", "c.txt"].map(|name| real(&root.join(name)));

    open_file(application, &context, &project, &a, "A2");
    let output = until(application, &context, "gutter", |application, _| {
        application.git_state.cache.gutter(&project, &a).len() == 3
    });
    let (pane, _) = active(application, &project);
    frame(
        application,
        &context,
        pointer(lane_at(&output, "a1", 1), egui::PointerButton::Primary),
    );
    settle(application, &context);
    assert_eq!(
        application
            .git_state
            .editors
            .get(&pane)
            .unwrap()
            .pending_hunk,
        None,
        "hunk 밖 줄은 무시합니다"
    );

    frame(
        application,
        &context,
        pointer(lane_at(&output, "a8", 7), egui::PointerButton::Primary),
    );
    let title = text(application, "git.discardHunkTitle");
    until(application, &context, "discard dialog", |_, output| {
        has(output, |text| text == title)
    });
    click_label(application, &context, &text(application, "common.cancel"));
    settle(application, &context);
    assert_eq!(
        std::fs::read_to_string(root.join("a.txt")).unwrap(),
        EDITED,
        "취소하면 디스크는 그대로입니다"
    );

    let output = settle(application, &context);
    frame(
        application,
        &context,
        pointer(lane_at(&output, "A2", 2), egui::PointerButton::Primary),
    );
    let description = crate::presentation::message(
        &application.locale,
        "git.discardHunkDescription",
        &[("start", "2"), ("end", "2")],
    );
    until(application, &context, "discard range", |_, output| {
        has(output, |text| text == description)
    });
    click_label(
        application,
        &context,
        &text(application, "git.discardConfirm"),
    );
    until(application, &context, "hunk discarded", |_, _| {
        std::fs::read_to_string(root.join("a.txt")).unwrap() == "a1\na2\na3\na5\na6\na7\na8\n"
    });

    open_file(application, &context, &project, &c, "C2");
    until(application, &context, "stage gutter", |application, _| {
        application.git_state.cache.gutter(&project, &c).len() == 2
    });
    let view = view(application, &project);
    let text_c = document_text(application, view);
    let (second, third) = (text_c.find("C2").unwrap(), text_c.find("C3").unwrap());
    select(application, view, second, third);
    let stage = text(application, "git.stageChanges");
    menu_action(application, &context, view, &stage);
    until(application, &context, "line staged", |_, _| {
        let staged = workspace_diff(&root, &empty);
        staged.contains("+C2") && !staged.contains("+C3")
    });
    let sixth = text_c.find("C6").unwrap();
    select(application, view, sixth, sixth);
    run(application, &context, "monaco.taide.gitStageSelection");
    until(application, &context, "hunk staged", |_, _| {
        let staged = workspace_diff(&root, &empty);
        staged.contains("+C2") && staged.contains("+C6") && !staged.contains("+C3")
    });
}

fn workspace_diff(root: &Path, empty: &Path) -> String {
    fixture::git(root, empty, &["diff", "--cached", "--", "c.txt"])
}

#[test]
fn 실제_app_충돌_파일은_영역_배경과_lane을_그리고_해결_비교_대화상자로_정리한다() {
    let mut workspace = Workspace::new("editor-conflict", &["alpha"]);
    merge_conflict(&workspace);
    let root = workspace.roots[0].clone();
    let context = launch_editor(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("m.txt"));

    open_file(application, &context, &project, &path, "ours1");
    let (pane, _) = active(application, &project);
    let regions = |application: &NativeApplication| {
        application
            .git_state
            .editors
            .get(&pane)
            .map(|editor| {
                editor
                    .conflicts
                    .regions()
                    .iter()
                    .map(|region| region.start)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    until(application, &context, "regions", |application, _| {
        regions(application) == [2, 12]
    });
    let appearance = application.git_state.editors.appearance.unwrap();
    let output = until(application, &context, "region marks", |_, output| {
        fills(output, appearance.conflicted) == 2
    });
    let backgrounds = |color| {
        output
            .shapes
            .iter()
            .filter(|clipped| {
                matches!(&clipped.shape, egui::Shape::Rect(rect) if rect.fill == color && rect.rect.width() > LANE_BAR_WIDTH)
            })
            .count()
    };
    assert!(
        backgrounds(appearance.current) >= 2 && backgrounds(appearance.incoming) >= 2,
        "현재·수신 영역 줄 배경"
    );
    assert_eq!(
        lane_marks(application, &output, HunkKind::Modified),
        0,
        "충돌 파일은 hunk 막대 대신 영역을 그립니다"
    );

    let view = view(application, &project);
    frame(
        application,
        &context,
        pointer(
            lane_at(&output, "<<<<<<< HEAD", 2),
            egui::PointerButton::Primary,
        ),
    );
    let description = crate::presentation::message(
        &application.locale,
        "git.resolveConflictDescription",
        &[("start", "2"), ("end", "6")],
    );
    until(application, &context, "resolve dialog", |_, output| {
        has(output, |text| text == description)
    });
    click_label(
        application,
        &context,
        &text(application, "git.acceptCurrentChange"),
    );
    until(application, &context, "first resolved", |application, _| {
        document_text(application, view) == FIRST_RESOLVED
    });
    let document = application.store.views().get(view).unwrap().document;
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
    assert_eq!(
        std::fs::read_to_string(root.join("m.txt")).unwrap(),
        MERGED,
        "마커가 남으면 디스크에 쓰지 않습니다"
    );
    assert!(application.store.undo(document).unwrap());
    assert_eq!(document_text(application, view), MERGED, "undo 한 번");
    assert!(application.store.redo(document).unwrap());
    assert_eq!(document_text(application, view), FIRST_RESOLVED);
    until(application, &context, "reparsed", |application, _| {
        regions(application) == [8]
    });
    let output = until(application, &context, "one region mark", |_, output| {
        fills(output, appearance.conflicted) == 1
    });
    let second = lane_at(&output, "<<<<<<< HEAD", 8);
    frame(
        application,
        &context,
        pointer(second, egui::PointerButton::Primary),
    );
    click_label(
        application,
        &context,
        &text(application, "git.compareChanges"),
    );
    until(application, &context, "compare", |application, _| {
        let texts = virtual_texts(application);
        texts.contains(&OURS.to_owned()) && texts.contains(&THEIRS.to_owned())
    });
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    until(application, &context, "compare closed", |application, _| {
        virtual_texts(application).is_empty()
    });

    let output = settle(application, &context);
    frame(
        application,
        &context,
        pointer(
            lane_at(&output, "<<<<<<< HEAD", 8),
            egui::PointerButton::Primary,
        ),
    );
    click_label(
        application,
        &context,
        &text(application, "git.acceptIncomingChange"),
    );
    let resolved = text(application, "git.conflictResolved");
    until(application, &context, "resolved", |application, _| {
        application.toasts.is_showing(Kind::Success, &resolved)
    });
    assert_eq!(
        std::fs::read_to_string(root.join("m.txt")).unwrap(),
        RESOLVED
    );
    until(application, &context, "settled", |application, _| {
        !application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
            && application
                .git_state
                .cache
                .status_result(&project)
                .and_then(Result::ok)
                .is_some_and(|status| status.rows.iter().all(|row| !row.is_conflicted))
    });

    let errors = |application: &NativeApplication| {
        application
            .toasts
            .inspection()
            .iter()
            .filter(|toast| toast.kind == Kind::Error)
            .count()
    };
    let before = errors(application);
    application
        .git_state
        .editors
        .get_mut(&pane)
        .unwrap()
        .conflicts
        .pending = Some(crate::git_conflict::Region {
        start: 1,
        base: None,
        separator: 2,
        end: 3,
    });
    application.apply_git_editor_intent(
        Intent::ResolveConflict {
            pane: pane.clone(),
            action: ConflictAction::Compare,
        },
        &context,
    );
    until(application, &context, "compare failed", |application, _| {
        errors(application) == before + 1
    });
    until(
        application,
        &context,
        "실패 응답 뒤에는 비교 대화상자를 그리지 않습니다",
        |application, _| virtual_texts(application).is_empty(),
    );
    assert!(
        application
            .git_state
            .cache
            .get(&project, &Key::ConflictSides { path: path.clone() })
            .is_some_and(Result::is_err)
    );
}

#[test]
fn 실제_app_해결_대화상자가_열린_동안_문서가_짧아지면_수락은_패닉_없이_무동작으로_닫힌다() {
    let mut workspace = Workspace::new("editor-conflict-stale", &["alpha"]);
    merge_conflict(&workspace);
    let root = workspace.roots[0].clone();
    let context = launch_editor(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("m.txt"));

    open_file(application, &context, &project, &path, "ours1");
    let (pane, _) = active(application, &project);
    until(application, &context, "regions", |application, _| {
        application
            .git_state
            .editors
            .get(&pane)
            .is_some_and(|editor| editor.conflicts.regions().len() == 2)
    });
    let appearance = application.git_state.editors.appearance.unwrap();
    let output = until(application, &context, "region marks", |_, output| {
        fills(output, appearance.conflicted) == 2
    });
    frame(
        application,
        &context,
        pointer(
            lane_at(&output, "<<<<<<< HEAD", 2),
            egui::PointerButton::Primary,
        ),
    );
    let description = crate::presentation::message(
        &application.locale,
        "git.resolveConflictDescription",
        &[("start", "2"), ("end", "6")],
    );
    until(application, &context, "resolve dialog", |_, output| {
        has(output, |text| text == description)
    });

    let view = view(application, &project);
    let document = application.store.views().get(view).unwrap().document;
    let snapshot = application.store.documents().snapshot(document).unwrap();
    application
        .store
        .apply_separate(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text: SHORTENED.into(),
                }],
                group: UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    click_label(
        application,
        &context,
        &text(application, "git.acceptCurrentChange"),
    );
    let output = until(application, &context, "dialog closed", |_, output| {
        !has(output, |text| text == description)
    });
    assert!(
        application
            .git_state
            .editors
            .get(&pane)
            .is_some_and(|editor| editor.conflicts.pending.is_none())
    );
    assert_eq!(document_text(application, view), SHORTENED, "무동작");
    assert_eq!(
        std::fs::read_to_string(root.join("m.txt")).unwrap(),
        MERGED,
        "디스크에 쓰지 않습니다"
    );
    assert!(has(&output, |text| text == "tail"));
    assert!(
        application
            .toasts
            .inspection()
            .iter()
            .all(|toast| toast.kind != Kind::Error)
    );
}

fn fills(output: &egui::FullOutput, color: egui::Color32) -> usize {
    output
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(rect) if rect.fill == color && rect.rect.width() == LANE_BAR_WIDTH)
        })
        .count()
}

fn virtual_texts(application: &NativeApplication) -> Vec<String> {
    application
        .store
        .documents()
        .snapshots()
        .filter(|snapshot| matches!(snapshot.key, DocumentKey::Virtual(_)))
        .map(|snapshot| snapshot.rope.to_string())
        .collect()
}

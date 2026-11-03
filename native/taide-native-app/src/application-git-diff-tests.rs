use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::git::GitChangeKind;
use taide_model::ids::ProjectId;
use taide_model::layout::{PaneNode, Tab, TabKind};
use taide_native_editor::document::DocumentKey;
use taide_native_ui::shell::{TAB_HEIGHT, TITLE_HEIGHT};

use super::base_tests::{
    Workspace, frame, has_text, node, nodes, settle, text, text_shapes, until,
};
use super::fixture;
use super::{Command, Key};
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};
use crate::host::HostCommand;

const COMMITTED: &str = "alpha\nbeta\ngamma\ndelta\nepsilon\nzeta\neta\ntheta\n";
const EDITED: &str = "alpha\nBETA\ngamma\ndelta\nepsilon\nzeta\nETA\ntheta\n";
const REFRESHED: &str = "alpha\nBETA\ngamma\ndelta\nepsilon\nzeta\nETA\nTHETA\n";
const OTHER: &str = "first\nsecond\n";
const LANE_INSET: f32 = 2.0;
const SAME_ROW: f32 = 4.0;
const NUMBERED_LINES: usize = 40;
const NUMBERED_CHANGED_LINE: usize = 20;
const JSON_COMMITTED: &str = "{\n\"name\": \"beta\",\n\"version\": 1\n}\n";
const JSON_EDITED: &str = "{\n\"name\": \"BETA\",\n\"version\": 1\n}\n";
const JSON_UNCHANGED_LINE: &str = "\"version\": 1";
const JSON_EDITED_LINE: &str = "\"name\": \"BETA\",";
const JSON_SERVICE_WAIT: std::time::Duration = std::time::Duration::from_secs(1);

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

fn diff_kind(path: String, staged: bool, compare_with: Option<String>) -> TabKind {
    TabKind::Diff {
        path,
        staged,
        compare_with,
        rev: None,
        parent_rev: None,
        before_path: None,
    }
}

fn tabs(application: &NativeApplication, project: &ProjectId) -> (Vec<Tab>, Option<Tab>) {
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
                tabs.extend(leaf.iter().cloned());
                if *id == layout.focused_pane {
                    active = leaf
                        .iter()
                        .find(|tab| Some(&tab.id) == current.as_ref())
                        .cloned();
                }
            }
        }
    }
    (tabs, active)
}

fn open(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    kind: TabKind,
) -> egui::FullOutput {
    assert!(application.submit(HostCommand::Git(Command::OpenTab {
        project: project.clone(),
        kind: kind.clone(),
        title: "diff".into(),
        target: None,
        preview: false,
    })));
    until(application, context, "diff tab active", |application, _| {
        tabs(application, project).1.map(|tab| tab.kind) == Some(kind.clone())
    })
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

/// The gutter lane of the right-most editor at the row drawing `word`: line numbers are drawn
/// right-aligned against the lane column.
fn lane_at(output: &egui::FullOutput, word: &str, line: usize) -> egui::Pos2 {
    let row = text_shapes(output, word)
        .max_by(|left, right| left.pos.x.total_cmp(&right.pos.x))
        .unwrap_or_else(|| panic!("{word} 를 그리지 않았습니다"));
    let line = line.to_string();
    let number = text_shapes(output, &line)
        .filter(|shape| (shape.pos.y - row.pos.y).abs() < SAME_ROW && shape.pos.x < row.pos.x)
        .max_by(|left, right| left.pos.x.total_cmp(&right.pos.x))
        .expect("수정 쪽 줄 번호");
    egui::pos2(
        number.pos.x + number.galley.size().x + LANE_INSET,
        row.pos.y + row.galley.size().y / 2.0,
    )
}

fn stage_bars(application: &NativeApplication, output: &egui::FullOutput) -> usize {
    let color = application
        .git_state
        .diff_appearance
        .expect("diff 외형")
        .colors
        .stage;
    output
        .shapes
        .iter()
        .filter(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == color))
        .count()
}

fn virtual_documents(application: &NativeApplication) -> usize {
    application
        .store
        .documents()
        .snapshots()
        .filter(|snapshot| matches!(snapshot.key, DocumentKey::Virtual(_)))
        .count()
}

fn launch_diff(workspace: &mut Workspace) -> egui::Context {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    settle(workspace.fixture.application.as_mut().unwrap(), &context);
    context
}

#[test]
fn 실제_app_작업트리와_staged_diff_탭은_두_편집기로_그리고_lane_누름으로_hunk를_stage_unstage한다()
{
    let mut workspace = Workspace::new("diff-stage", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED)]);
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));

    open(
        application,
        &context,
        &project,
        diff_kind(path.clone(), false, None),
    );
    let output = until(application, &context, "workdir diff", |_, output| {
        has_text(output, "BETA") && has_text(output, "beta") && has_text(output, "ETA")
    });
    let key = Key::Diff {
        path: path.clone(),
        staged: false,
        before_path: None,
    };
    let hunks = application
        .git_state
        .cache
        .diff(&project, &key)
        .unwrap()
        .unwrap()
        .hunks
        .len();
    assert_eq!(hunks, 2);
    assert_eq!(virtual_documents(application), 2);
    assert_eq!(text_shapes(&output, "alpha").count(), 2, "좌우 두 편집기");
    assert_eq!(stage_bars(application, &output), 2);

    frame(
        application,
        &context,
        pointer(lane_at(&output, "gamma", 3), egui::PointerButton::Primary),
    );
    settle(application, &context);
    assert_eq!(
        status(application, &project, "a.txt"),
        Some((None, Some(GitChangeKind::Modified))),
        "hunk 밖 줄은 무시합니다"
    );

    frame(
        application,
        &context,
        pointer(lane_at(&output, "BETA", 2), egui::PointerButton::Primary),
    );
    until(
        application,
        &context,
        "first hunk staged",
        |application, _| {
            status(application, &project, "a.txt")
                == Some((Some(GitChangeKind::Modified), Some(GitChangeKind::Modified)))
        },
    );
    let output = until(
        application,
        &context,
        "workdir diff refreshed",
        |application, _| {
            application
                .git_state
                .cache
                .diff(&project, &key)
                .and_then(Result::ok)
                .is_some_and(|diff| diff.hunks.len() == 1)
        },
    );
    assert!(has_text(&output, "ETA"));

    open(
        application,
        &context,
        &project,
        diff_kind(path.clone(), true, None),
    );
    let output = until(application, &context, "staged diff", |_, output| {
        has_text(output, "BETA") && !has_text(output, "ETA")
    });
    assert_eq!(stage_bars(application, &output), 1);
    frame(
        application,
        &context,
        pointer(lane_at(&output, "BETA", 2), egui::PointerButton::Primary),
    );
    until(application, &context, "hunk unstaged", |application, _| {
        status(application, &project, "a.txt") == Some((None, Some(GitChangeKind::Modified)))
    });
    settle(application, &context);
    assert_eq!(
        virtual_documents(application),
        2,
        "보이지 않는 diff 탭의 문서는 해제됩니다"
    );
}

#[test]
fn 실제_app_diff_탭은_파일_저장_이벤트로_다시_읽고_alt_백슬래시로_inline을_왕복하며_다시_보면_좌우로_돌아온다()
 {
    let mut workspace = Workspace::new("diff-inline", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED), ("b.txt", OTHER)]);
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));

    let kind = diff_kind(path.clone(), false, None);
    open(application, &context, &project, kind.clone());
    until(application, &context, "diff", |_, output| {
        text_shapes(output, "alpha").count() == 2
    });

    frame(
        application,
        &context,
        key(egui::Key::Backslash, egui::Modifiers::ALT),
    );
    until(application, &context, "inline", |_, output| {
        text_shapes(output, "alpha").count() == 1 && has_text(output, "beta")
    });
    frame(
        application,
        &context,
        key(egui::Key::Backslash, egui::Modifiers::ALT),
    );
    until(application, &context, "side by side", |_, output| {
        text_shapes(output, "alpha").count() == 2
    });
    frame(
        application,
        &context,
        key(egui::Key::Backslash, egui::Modifiers::ALT),
    );
    until(application, &context, "inline again", |_, output| {
        text_shapes(output, "alpha").count() == 1
    });

    open(
        application,
        &context,
        &project,
        diff_kind(real(&root.join("b.txt")), false, None),
    );
    settle(application, &context);
    open(application, &context, &project, kind);
    until(
        application,
        &context,
        "remounted side by side",
        |_, output| text_shapes(output, "alpha").count() == 2,
    );

    std::fs::write(root.join("a.txt"), REFRESHED).unwrap();
    application.services.events.publish(AppEvent::FsChanged {
        project_id: project.clone(),
        change: FsChange {
            kind: FsChangeKind::Modified,
            paths: vec![path],
            from_app: false,
        },
    });
    until(application, &context, "refreshed", |_, output| {
        has_text(output, "THETA")
    });
}

#[test]
fn 실제_app_파일_비교_탭은_두_파일을_stage_막대_없이_보이고_읽기_실패는_오류_문구를_보인다() {
    let mut workspace = Workspace::new("diff-compare", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED), ("b.txt", OTHER)]);
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let (a, b) = (real(&root.join("a.txt")), real(&root.join("b.txt")));

    open(
        application,
        &context,
        &project,
        diff_kind(b.clone(), false, Some(a.clone())),
    );
    let output = until(application, &context, "compare", |_, output| {
        has_text(output, "alpha") && has_text(output, "second")
    });
    assert_eq!(
        stage_bars(application, &output),
        0,
        "파일 비교는 stage 막대가 없습니다"
    );
    assert!(!has_text(
        &output,
        &text(application, "editor.diffLoadFailed")
    ));

    let missing = real(&root.join("a.txt")).replace("a.txt", "missing.txt");
    open(
        application,
        &context,
        &project,
        diff_kind(b, false, Some(missing)),
    );
    until(application, &context, "failed", |application, output| {
        has_text(output, &text(application, "editor.diffLoadFailed"))
    });
}

#[test]
fn 실제_app_탐색기_비교_메뉴는_선택한_파일과_대상_파일의_preview_비교_탭을_연다() {
    let mut workspace = Workspace::new("diff-explorer", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED), ("b.txt", OTHER)]);
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let (select, compare) = (
        text(application, "explorer.selectForCompare"),
        text(application, "explorer.compareWithSelected"),
    );
    let sidebar_row = |output: &egui::FullOutput, row: &str| {
        nodes(output, row)
            .map(|(rect, _)| rect)
            .min_by(|left, right| left.left().total_cmp(&right.left()))
    };
    let choose = |application: &mut NativeApplication, row: &str, item: &str| {
        let output = until(application, &context, row, |_, output| {
            sidebar_row(output, row).is_some()
        });
        let rect = sidebar_row(&output, row).unwrap();
        frame(
            application,
            &context,
            pointer(rect.center(), egui::PointerButton::Secondary),
        );
        let output = until(application, &context, item, |_, output| {
            node(output, item).is_some()
        });
        let (rect, menu_item) = node(&output, item).unwrap();
        frame(
            application,
            &context,
            pointer(rect.center(), egui::PointerButton::Primary),
        );
        settle(application, &context);
        menu_item.is_disabled()
    };

    let has_diff_tab = |application: &NativeApplication| {
        tabs(application, &project)
            .0
            .iter()
            .any(|tab| matches!(tab.kind, TabKind::Diff { .. }))
    };
    assert!(
        choose(application, "b.txt", &compare),
        "선택 전에는 비활성입니다"
    );
    assert!(!has_diff_tab(application));
    assert!(!choose(application, "a.txt", &select));
    assert!(!choose(application, "b.txt", &compare));
    let output = until(application, &context, "compare tab", |_, output| {
        has_text(output, "alpha") && has_text(output, "second")
    });
    let active = tabs(application, &project).1.expect("비교 탭");
    assert_eq!(active.title, "a.txt vs b.txt");
    assert!(active.preview);
    let TabKind::Diff {
        path, compare_with, ..
    } = &active.kind
    else {
        panic!("{:?}", active.kind);
    };
    assert!(path.ends_with("b.txt"));
    assert!(
        compare_with
            .as_deref()
            .is_some_and(|source| source.ends_with("a.txt"))
    );
    assert_eq!(stage_bars(application, &output), 0);
    assert!(
        application
            .explorer_search_state
            .explorers
            .get(&project)
            .is_some_and(|explorer| explorer.compare_source.is_none()),
        "비교 뒤 선택은 지워집니다"
    );
}

#[test]
fn 실제_app_파일_탭_메뉴의_변경_열기는_focus_pane에_preview_diff_탭을_연다() {
    let mut workspace = Workspace::new("diff-menu", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED)]);
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));
    assert!(application.submit(HostCommand::Editor(
        crate::domains::editor::Command::OpenFileTab {
            project: project.clone(),
            pane: None,
            path: path.clone(),
            preview: false,
        }
    )));
    let tab_bar = |output: &egui::FullOutput| {
        nodes(output, "a.txt")
            .map(|(rect, _)| rect)
            .find(|rect| rect.bottom() <= TITLE_HEIGHT + TAB_HEIGHT)
    };
    let output = until(application, &context, "file tab", |_, output| {
        tab_bar(output).is_some() && has_text(output, "BETA")
    });
    let tab = tab_bar(&output).unwrap();
    frame(
        application,
        &context,
        pointer(tab.center(), egui::PointerButton::Secondary),
    );
    let label = text(application, "tab.openChanges");
    let output = until(application, &context, "menu", |_, output| {
        node(output, &label).is_some()
    });
    let (item, _) = node(&output, &label).unwrap();
    frame(
        application,
        &context,
        pointer(item.center(), egui::PointerButton::Primary),
    );
    let expected = diff_kind(path, false, None);
    until(application, &context, "diff tab", |application, _| {
        tabs(application, &project)
            .1
            .is_some_and(|tab| tab.kind == expected)
    });
    let (all, active) = tabs(application, &project);
    let active = active.unwrap();
    assert_eq!(active.title, "a.txt (diff)");
    assert!(active.preview);
    assert_eq!(all.iter().filter(|tab| tab.kind == expected).count(), 1);
}

#[test]
fn 실제_app_diff_탭은_응답_전_첫_그림에서_내용과_오류_없이_빈_배경만_그린다() {
    let mut workspace = Workspace::new("diff-loading", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", COMMITTED)]);
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let path = real(&root.join("a.txt"));
    let title = "loading diff";

    assert!(application.submit(HostCommand::Git(Command::OpenTab {
        project: project.clone(),
        kind: diff_kind(path.clone(), false, None),
        title: title.into(),
        target: None,
        preview: false,
    })));
    let first = until(application, &context, "first draw", |_, output| {
        has_text(output, title)
    });
    let key = Key::Diff {
        path,
        staged: false,
        before_path: None,
    };
    assert!(
        application.git_state.cache.diff(&project, &key).is_none(),
        "첫 그림은 응답 전입니다"
    );
    assert!(!has_text(&first, "alpha"), "응답 전에는 내용이 없습니다");
    assert!(!has_text(
        &first,
        &text(application, "editor.diffLoadFailed")
    ));
    assert_eq!(virtual_documents(application), 0);
    until(application, &context, "loaded", |_, output| {
        text_shapes(output, "alpha").count() == 2
    });
}

#[test]
fn 실제_app_json_파일_diff_탭은_구문_강조를_그리고_내장_json_서비스_오류_toast를_띄우지_않는다() {
    let mut workspace = Workspace::new("diff-json", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("package.json", JSON_COMMITTED)]);
    std::fs::write(root.join("package.json"), JSON_EDITED).unwrap();
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();

    open(
        application,
        &context,
        &project,
        diff_kind(real(&root.join("package.json")), false, None),
    );
    until(application, &context, "json diff colors", |_, output| {
        let colored = text_shapes(output, JSON_UNCHANGED_LINE)
            .filter(|shape| {
                shape
                    .galley
                    .job
                    .sections
                    .iter()
                    .map(|section| section.format.color)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    > 1
            })
            .count();
        colored == 2 && has_text(output, JSON_EDITED_LINE)
    });
    let virtual_ids = application
        .store
        .documents()
        .snapshots()
        .filter(|snapshot| matches!(snapshot.key, DocumentKey::Virtual(_)))
        .map(|snapshot| snapshot.id)
        .collect::<Vec<_>>();
    assert_eq!(virtual_ids.len(), 2);
    let deadline = std::time::Instant::now() + JSON_SERVICE_WAIT;
    until(application, &context, "json services idle", |_, _| {
        std::time::Instant::now() >= deadline
    });
    let errors = application
        .toasts
        .inspection()
        .into_iter()
        .filter(|toast| toast.kind == taide_native_ui::toast::Kind::Error)
        .map(|toast| toast.title)
        .collect::<Vec<_>>();
    assert!(errors.is_empty(), "오류 toast: {errors:?}");
    let (structure, validation) = (
        application.editor_json.documents(),
        application.editor_json_validation.documents(),
    );
    assert!(
        virtual_ids
            .iter()
            .all(|id| !structure.contains(id) && !validation.contains(id)),
        "가상 문서는 내장 json 서비스 대상이 아닙니다"
    );
}

fn numbered(changed: Option<&str>) -> String {
    (1..=NUMBERED_LINES)
        .map(|line| match changed {
            Some(text) if line == NUMBERED_CHANGED_LINE => format!("{text}\n"),
            _ => format!("line {line:02}\n"),
        })
        .collect()
}

fn hide_unchanged(application: &mut NativeApplication, enabled: bool) {
    assert!(application.submit(HostCommand::Git(Command::PatchSettings(
        taide_model::settings::SettingsPatch {
            editor_diff_hide_unchanged_regions: Some(enabled),
            ..Default::default()
        }
    ))));
}

#[test]
fn 실제_app_미변경_영역_숨김_설정은_열린_diff_탭에_바로_반영되고_끄면_모두_보인다() {
    let mut workspace = Workspace::new("diff-hide", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    let committed = numbered(None);
    fixture::repository(&root, &empty, &[("long.txt", committed.as_str())]);
    std::fs::write(root.join("long.txt"), numbered(Some("LINE 20"))).unwrap();
    let context = launch_diff(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();

    open(
        application,
        &context,
        &project,
        diff_kind(real(&root.join("long.txt")), false, None),
    );
    let output = until(application, &context, "diff", |_, output| {
        has_text(output, "LINE 20") && text_shapes(output, "line 05").count() == 2
    });
    assert!(!has_text(&output, "15 hidden lines"));

    hide_unchanged(application, true);
    until(application, &context, "collapsed", |_, output| {
        text_shapes(output, "15 hidden lines").count() == 2
            && text_shapes(output, "18 hidden lines").count() == 2
            && !has_text(output, "line 05")
            && text_shapes(output, "line 17").count() == 2
    });

    hide_unchanged(application, false);
    until(application, &context, "expanded", |_, output| {
        !has_text(output, "15 hidden lines") && text_shapes(output, "line 05").count() == 2
    });
}

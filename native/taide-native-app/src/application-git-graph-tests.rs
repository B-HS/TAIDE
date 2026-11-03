use eframe::egui;
use taide_model::ids::ProjectId;
use taide_model::layout::{PaneNode, Tab, TabKind};
use taide_native_ui::toast::Kind;

use super::base_tests::{
    Workspace, frame, has_text, nodes, settle, show_git, text, text_shapes, until,
};
use super::fixture;
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};

const FIRST: &str = "test(fixture): initial";
const SECOND: &str = "test(fixture): second";
const THIRD: &str = "test(fixture): third";
const ONE: &str = "one\n";
const TWO: &str = "two\n";
const THREE: &str = "three\n";
const RAISE: f32 = 100.0;
const COLLAPSE_LEFT: f32 = 30.0;
const SHORT: usize = 7;
const AWAY: egui::Pos2 = egui::pos2(1280.0, 800.0);

fn launch_graph(workspace: &mut Workspace) -> egui::Context {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    settle(application, &context);
    show_git(application, &context);
    context
}

fn commit_all(workspace: &Workspace, file: &str, text: &str, message: &str) {
    std::fs::write(workspace.roots[0].join(file), text).unwrap();
    workspace.git(0, &["add", "--", file]);
    workspace.git(0, &["commit", "-m", message]);
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

/// The commit row named `summary`: rows carry a selected state, unlike painted text.
fn commit_row(output: &egui::FullOutput, summary: &str) -> Option<(egui::Rect, bool)> {
    nodes(output, summary)
        .find(|(_, node)| node.toggled().is_some())
        .map(|(rect, node)| (rect, node.toggled() == Some(egui::accesskit::Toggled::True)))
}

fn button(output: &egui::FullOutput, label: &str) -> Option<(egui::Rect, egui::accesskit::Node)> {
    nodes(output, label)
        .find(|(_, node)| node.role() == egui::accesskit::Role::Button)
        .map(|(rect, node)| (rect, node.clone()))
}

fn texts_in(output: &egui::FullOutput, row: egui::Rect, wanted: &str) -> usize {
    text_shapes(output, wanted)
        .filter(|shape| {
            row.y_range()
                .contains(shape.visual_bounding_rect().center().y)
        })
        .count()
}

fn click(application: &mut NativeApplication, context: &egui::Context, target: egui::Rect) {
    frame(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
}

fn right_click(application: &mut NativeApplication, context: &egui::Context, summary: &str) {
    frame(application, context, vec![egui::Event::PointerMoved(AWAY)]);
    let output = settle(application, context);
    let (row, _) = commit_row(&output, summary).unwrap_or_else(|| panic!("{summary}"));
    frame(
        application,
        context,
        pointer(row.center(), egui::PointerButton::Secondary),
    );
}

fn primary(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

/// Presses the separator above the graph header, moves it by `delta` in two steps and releases.
fn drag_separator(
    application: &mut NativeApplication,
    context: &egui::Context,
    header: egui::Rect,
    delta: f32,
) {
    let start = egui::pos2(header.center().x, header.top() - 0.5);
    frame(application, context, primary(start, true));
    for step in [delta / 2.0, delta] {
        frame(
            application,
            context,
            vec![egui::Event::PointerMoved(start + egui::vec2(0.0, step))],
        );
    }
    frame(
        application,
        context,
        primary(start + egui::vec2(0.0, delta), false),
    );
}

fn tabs(application: &NativeApplication, project: &ProjectId) -> (Vec<Tab>, Option<Tab>) {
    let layouts = application.services.state.layouts.read();
    let layout = &layouts[project];
    let mut stack = vec![&layout.root];
    let (mut all, mut active) = (Vec::new(), None);
    while let Some(pane) = stack.pop() {
        match pane {
            PaneNode::Split { children, .. } => stack.extend(children),
            PaneNode::Leaf {
                id,
                tabs,
                active: current,
            } => {
                all.extend(tabs.iter().cloned());
                if *id == layout.focused_pane {
                    active = tabs
                        .iter()
                        .find(|tab| Some(&tab.id) == current.as_ref())
                        .cloned();
                }
            }
        }
    }
    (all, active)
}

fn head_summary(workspace: &Workspace) -> String {
    workspace.git(0, &["log", "-1", "--format=%s"])
}

fn shows(application: &NativeApplication, kind: Kind, key: &str) -> bool {
    application.toasts.is_showing(kind, &text(application, key))
}

fn errors(application: &NativeApplication) -> usize {
    application
        .toasts
        .inspection()
        .iter()
        .filter(|toast| toast.kind == Kind::Error)
        .count()
}

#[test]
fn 실제_app_그래프는_로그를_레인_refs_배지_해시로_그리고_선택하면_상세와_commit_diff_탭을_연다() {
    let mut workspace = Workspace::new("graph-log", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ONE)]);
    commit_all(&workspace, "a.txt", TWO, SECOND);
    commit_all(&workspace, "b.txt", ONE, THIRD);
    workspace.git(0, &["tag", "v1"]);
    let head = workspace.git(0, &["rev-parse", "HEAD"]);
    let second = workspace.git(0, &["rev-parse", "HEAD~1"]);
    let context = launch_graph(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let graph = text(application, "git.graph");
    let just_now = text(application, "git.timeJustNow");

    let output = until(application, &context, "three rows", |_, output| {
        [FIRST, SECOND, THIRD]
            .iter()
            .all(|summary| commit_row(output, summary).is_some())
    });
    let (third, is_selected) = commit_row(&output, THIRD).unwrap();
    assert!(!is_selected);
    for wanted in [
        "main",
        "v1",
        &head[..SHORT],
        just_now.as_str(),
        "Synthetic Test",
    ] {
        assert_eq!(texts_in(&output, third, wanted), 1, "{wanted}");
    }
    let (header, _) = button(&output, &graph).expect("graph header");
    assert_eq!(texts_in(&output, header, "3"), 1, "graph count badge");
    let (first, _) = commit_row(&output, FIRST).unwrap();
    assert!(first.top() > third.top(), "newest first");

    click(application, &context, third);
    let loading = frame(application, &context, Vec::new());
    assert!(has_text(&loading, &text(application, "common.loading")));
    assert!(commit_row(&loading, THIRD).unwrap().1);
    let changed = text(application, "git.changedFiles").to_uppercase();
    let output = until(application, &context, "commit files", |_, output| {
        button(output, "b.txt").is_some()
    });
    assert!(has_text(&output, &changed));
    assert!(!has_text(&output, &text(application, "common.loading")));

    let (file, _) = button(&output, "b.txt").unwrap();
    click(application, &context, file);
    let b = real(&root.join("b.txt"));
    until(
        application,
        &context,
        "commit diff tab",
        |application, _| {
            tabs(application, &project).1.is_some_and(|tab| {
                tab.kind
                    == TabKind::Diff {
                        path: b.clone(),
                        staged: false,
                        compare_with: None,
                        rev: Some(head.clone()),
                        parent_rev: Some(second.clone()),
                        before_path: Some(b.clone()),
                    }
                    && tab.title == format!("b.txt @ {}", &head[..SHORT])
                    && tab.preview
            })
        },
    );

    let output = settle(application, &context);
    let (row, _) = commit_row(&output, SECOND).unwrap();
    click(application, &context, row);
    let output = until(application, &context, "second files", |_, output| {
        button(output, "a.txt").is_some() && commit_row(output, SECOND).is_some_and(|row| row.1)
    });
    assert!(!commit_row(&output, THIRD).unwrap().1);
    let (file, _) = button(&output, "a.txt").unwrap();
    click(application, &context, file);
    until(
        application,
        &context,
        "preview replaced",
        |application, _| {
            let (all, active) = tabs(application, &project);
            active.is_some_and(|tab| tab.title == format!("a.txt @ {}", &second[..SHORT]))
                && all
                    .iter()
                    .filter(|tab| matches!(tab.kind, TabKind::Diff { .. }))
                    .count()
                    == 1
        },
    );

    let output = settle(application, &context);
    let (row, _) = commit_row(&output, SECOND).unwrap();
    click(application, &context, row);
    let output = until(application, &context, "toggled off", |_, output| {
        commit_row(output, SECOND).is_some_and(|row| !row.1)
    });
    assert!(!has_text(&output, &changed));

    click(application, &context, row);
    let output = until(application, &context, "selected again", |_, output| {
        button(output, "a.txt").is_some()
    });
    let close = text(application, "common.close");
    let (close, _) = nodes(&output, &close)
        .filter(|(rect, node)| {
            rect.top() > header.bottom() && node.role() == egui::accesskit::Role::Button
        })
        .map(|(rect, node)| (rect, node.clone()))
        .next()
        .expect("detail close");
    click(application, &context, close);
    let output = until(application, &context, "closed", |_, output| {
        commit_row(output, SECOND).is_some_and(|row| !row.1)
    });
    assert!(!has_text(&output, &changed));
}

#[test]
fn 실제_app_그래프_분리선은_끌면_120ms_뒤_높이를_저장하고_최소_미만이면_접힘만_기록하고_펼치면_저장_높이로_돌아온다()
 {
    let mut workspace = Workspace::new("graph-pane", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ONE)]);
    let context = launch_graph(&mut workspace);
    let (project, slot) = (workspace.projects[0].clone(), workspace.slots[0].clone());
    let application = workspace.fixture.application.as_mut().unwrap();
    let graph = text(application, "git.graph");
    let initial = application
        .services
        .state
        .settings
        .read()
        .git_graph_panel_size_px;

    let output = until(application, &context, "graph header", |_, output| {
        commit_row(output, FIRST).is_some() && button(output, &graph).is_some()
    });
    let (header, _) = button(&output, &graph).unwrap();
    drag_separator(application, &context, header, -RAISE);
    assert_eq!(
        application
            .services
            .state
            .settings
            .read()
            .git_graph_panel_size_px,
        initial,
        "not saved on release"
    );
    assert!(
        application
            .git_state
            .panels
            .next_resize(std::time::Instant::now())
            .is_some(),
        "save waits for the delay"
    );
    until(application, &context, "height saved", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .git_graph_panel_size_px
            > initial
    });
    let saved = application
        .services
        .state
        .settings
        .read()
        .git_graph_panel_size_px;
    let output = settle(application, &context);
    let (raised, _) = button(&output, &graph).unwrap();
    assert!((header.top() - raised.top() - (saved - initial) as f32).abs() <= 1.0);

    drag_separator(application, &context, raised, saved as f32 - COLLAPSE_LEFT);
    let output = until(application, &context, "collapsed", |application, output| {
        application
            .services
            .state
            .settings
            .read()
            .git_sections_collapsed
            .iter()
            .any(|id| id == "graph")
            && button(output, &graph).is_some_and(|(_, node)| node.is_expanded() == Some(false))
    });
    assert_eq!(
        application
            .services
            .state
            .settings
            .read()
            .git_graph_panel_size_px,
        saved
    );
    assert!(
        application
            .git_state
            .panels
            .next_resize(std::time::Instant::now())
            .is_none()
    );
    assert!(
        commit_row(&output, FIRST).is_none(),
        "collapsed graph hides rows"
    );
    assert!(
        application
            .git_state
            .panels
            .collapsed(&project, &slot)
            .unwrap()
            .is_collapsed(taide_native_ui::intents::git::Section::Graph)
    );

    let (collapsed, _) = button(&output, &graph).unwrap();
    click(application, &context, collapsed);
    let output = until(application, &context, "expanded", |_, output| {
        commit_row(output, FIRST).is_some()
    });
    let (expanded, _) = button(&output, &graph).unwrap();
    assert!(
        (expanded.top() - raised.top()).abs() <= 1.0,
        "stored height"
    );
}

#[test]
fn 실제_app_그래프_메뉴_revert는_성공_충돌_실패를_toast로_알리고_충돌이면_첫_파일을_연다() {
    let mut workspace = Workspace::new("graph-revert", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ONE)]);
    commit_all(&workspace, "b.txt", TWO, SECOND);
    let context = launch_graph(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let revert = text(application, "git.revert");
    until(application, &context, "rows", |_, output| {
        commit_row(output, SECOND).is_some()
    });

    right_click(application, &context, SECOND);
    let output = until(application, &context, "menu", |_, output| {
        button(output, &revert).is_some()
    });
    assert!(button(&output, &text(application, "git.createTag")).is_some());
    assert!(submenu(&output, &text(application, "git.deleteTag")).is_none());
    let (item, _) = button(&output, &revert).unwrap();
    click(application, &context, item);
    until(application, &context, "reverted", |application, _| {
        shows(application, Kind::Success, "git.revertSuccess")
    });
    assert!(head_summary(&workspace).starts_with("Revert"));
    assert!(!root.join("b.txt").exists());

    std::fs::write(root.join("a.txt"), TWO).unwrap();
    workspace.git(0, &["add", "--", "a.txt"]);
    let application = workspace.fixture.application.as_mut().unwrap();
    let before = errors(application);
    until(application, &context, "new head row", |_, output| {
        commit_row(output, "Revert \"test(fixture): second\"").is_some()
    });
    right_click(application, &context, FIRST);
    let output = until(application, &context, "menu again", |_, output| {
        button(output, &revert).is_some()
    });
    let (item, _) = button(&output, &revert).unwrap();
    click(application, &context, item);
    until(
        application,
        &context,
        "staged blocks revert",
        |application, _| errors(application) > before,
    );
    workspace.git(0, &["commit", "-m", THIRD]);
    std::fs::write(root.join("a.txt"), THREE).unwrap();
    workspace.git(0, &["commit", "-am", "test(fixture): fourth"]);

    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "fourth row", |_, output| {
        commit_row(output, "test(fixture): fourth").is_some()
    });
    right_click(application, &context, THIRD);
    let output = until(application, &context, "menu conflict", |_, output| {
        button(output, &revert).is_some()
    });
    let (item, _) = button(&output, &revert).unwrap();
    click(application, &context, item);
    let a = real(&root.join("a.txt"));
    until(application, &context, "conflict file", |application, _| {
        shows(application, Kind::Warning, "git.revertConflict")
            && tabs(application, &project)
                .1
                .is_some_and(|tab| tab.kind == TabKind::File { path: a.clone() })
    });
}

#[test]
fn 실제_app_상세_패널은_변경_파일_조회_전에는_로딩을_실패하면_오류색_문구를_보인다() {
    let mut workspace = Workspace::new("graph-detail", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ONE)]);
    let context = launch_graph(&mut workspace);
    let (project, slot) = (workspace.projects[0].clone(), workspace.slots[0].clone());
    let application = workspace.fixture.application.as_mut().unwrap();
    let output = until(application, &context, "row", |_, output| {
        commit_row(output, FIRST).is_some()
    });
    click(application, &context, commit_row(&output, FIRST).unwrap().0);
    until(application, &context, "files", |_, output| {
        button(output, "a.txt").is_some()
    });
    let failure = taide_model::error::AppError::Internal("synthetic failure".into());
    let appearance = application.git_state.appearance.as_ref().unwrap();
    for (files, key, color) in [
        (None, "common.loading", appearance.muted),
        (
            Some(Err(&failure)),
            "editor.diffLoadFailed",
            appearance.error,
        ),
    ] {
        let surface = egui::Context::default();
        surface.enable_accesskit();
        let view = crate::git_scm::View {
            project: &project,
            slot: &slot,
            status: application.git_state.cache.status_result(&project),
            collapsed: application
                .git_state
                .panels
                .collapsed(&project, &slot)
                .unwrap(),
            panel: application.git_state.panels.panel(&project, &slot),
            is_initializing: false,
            remotes: &[],
            branches: &[],
            stashes: &[],
            is_stashing: false,
            is_syncing: false,
            commit: Default::default(),
            graph: crate::git_scm::Graph {
                log: application.git_state.cache.log(&project),
                commit_files: files,
                size: taide_model::settings::DEFAULT_GIT_GRAPH_PANEL_SIZE_PX as f32,
                resizer_thickness: 1.0,
                ..Default::default()
            },
            appearance,
            locale: &application.locale,
            tooltips: &application.tooltips,
            tooltip_appearance: &application.tooltip_appearance,
        };
        let mut output = surface.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(AWAY.x, AWAY.y),
                )),
                ..Default::default()
            },
            |ui| {
                crate::git_scm::show(
                    ui,
                    &view,
                    &mut crate::navigation_icons::Deferred::default(),
                    &mut Vec::new(),
                )
                .unwrap();
            },
        );
        output.textures_delta.clear();
        let wanted = text(application, key);
        let colors: Vec<_> = text_shapes(&output, &wanted)
            .map(|shape| shape.galley.job.sections[0].format.color)
            .collect();
        assert_eq!(colors, [color], "{key}");
        assert!(button(&output, "a.txt").is_none(), "{key}: no file rows");
        assert!(has_text(
            &output,
            &text(application, "git.changedFiles").to_uppercase()
        ));
    }
}

fn typed(application: &mut NativeApplication, context: &egui::Context, text: &str) {
    frame(application, context, vec![egui::Event::Text(text.into())]);
}

/// Opens the create tag dialog from `summary`'s row menu and waits for its confirm button.
fn open_tag_dialog(
    application: &mut NativeApplication,
    context: &egui::Context,
    summary: &str,
) -> egui::accesskit::Node {
    let create = text(application, "git.createTag");
    let revert = text(application, "git.revert");
    right_click(application, context, summary);
    let output = until(application, context, "tag menu", |_, output| {
        button(output, &create).is_some()
    });
    let (item, _) = button(&output, &create).unwrap();
    click(application, context, item);
    let output = until(application, context, "tag dialog", |_, output| {
        button(output, &create).is_some() && button(output, &revert).is_none()
    });
    button(&output, &create).expect("confirm").1
}

/// The submenu trigger whose label starts with `label` (its arrow follows the text).
fn submenu(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .iter()
        .flat_map(|update| update.nodes.iter())
        .find(|(_, node)| {
            node.role() == egui::accesskit::Role::Button
                && node.label().is_some_and(|text| text.starts_with(label))
        })
        .and_then(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
}

fn tag_type(workspace: &Workspace, name: &str) -> String {
    workspace.git(0, &["cat-file", "-t", name])
}

/// Hovers the Delete Tag submenu of `summary`'s row menu and lists the tag items it shows.
fn delete_items(
    application: &mut NativeApplication,
    context: &egui::Context,
    summary: &str,
    names: &[&str],
) -> Vec<(String, egui::Rect)> {
    let delete = text(application, "git.deleteTag");
    right_click(application, context, summary);
    let output = until(application, context, "delete trigger", |_, output| {
        submenu(output, &delete).is_some()
    });
    let trigger = submenu(&output, &delete).unwrap();
    frame(
        application,
        context,
        vec![egui::Event::PointerMoved(trigger.center())],
    );
    let output = until(application, context, "delete items", |_, output| {
        names.iter().any(|name| button(output, name).is_some())
    });
    names
        .iter()
        .filter_map(|name| button(&output, name).map(|(rect, _)| ((*name).to_owned(), rect)))
        .collect()
}

#[test]
fn 실제_app_태그_대화상자는_열때마다_비고_빈이름을_막고_주석_경량_태그를_만들고_중복은_오류이며_삭제_서브메뉴는_그_커밋_태그만_보인다()
 {
    let mut workspace = Workspace::new("graph-tags", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ONE)]);
    commit_all(&workspace, "a.txt", TWO, SECOND);
    let second = workspace.git(0, &["rev-parse", "HEAD"]);
    let context = launch_graph(&mut workspace);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let create = text(application, "git.createTag");
    until(application, &context, "rows", |_, output| {
        commit_row(output, SECOND).is_some()
    });

    let confirm = open_tag_dialog(application, &context, SECOND);
    assert!(confirm.is_disabled(), "empty name");
    let output = settle(application, &context);
    assert!(has_text(&output, &second), "target shown");
    typed(application, &context, "v1");
    frame(
        application,
        &context,
        key(egui::Key::Tab, egui::Modifiers::NONE),
    );
    typed(application, &context, "release one");
    let output = settle(application, &context);
    let (confirm, node) = button(&output, &create).unwrap();
    assert!(!node.is_disabled());
    click(application, &context, confirm);
    until(application, &context, "annotated", |application, output| {
        shows(application, Kind::Success, "git.tagCreated") && button(output, &create).is_none()
    });
    assert_eq!(tag_type(&workspace, "v1"), "tag");

    let application = workspace.fixture.application.as_mut().unwrap();
    let confirm = open_tag_dialog(application, &context, SECOND);
    assert!(confirm.is_disabled(), "inputs reset on reopen");
    typed(application, &context, "  v2  ");
    let output = settle(application, &context);
    let (confirm, _) = button(&output, &create).unwrap();
    click(application, &context, confirm);
    until(application, &context, "lightweight", |_, output| {
        button(output, &create).is_none()
    });
    assert_eq!(tag_type(&workspace, "v2"), "commit");

    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "lightweight badge", |_, output| {
        commit_row(output, SECOND).is_some_and(|(row, _)| {
            texts_in(output, row, "v2") == 1 && texts_in(output, row, "v1") == 0
        })
    });
    let before = errors(application);
    open_tag_dialog(application, &context, SECOND);
    typed(application, &context, "v1");
    let output = settle(application, &context);
    let (confirm, _) = button(&output, &create).unwrap();
    click(application, &context, confirm);
    until(application, &context, "duplicate", |application, _| {
        errors(application) > before
    });
    let output = settle(application, &context);
    assert!(
        button(&output, &create).is_some_and(|(_, node)| !node.is_disabled()),
        "stays open"
    );
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    let output = settle(application, &context);
    assert!(button(&output, &create).is_none(), "escape closes");

    let items = delete_items(application, &context, SECOND, &["v1", "v2"]);
    assert_eq!(
        items
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["v1", "v2"]
    );
    click(application, &context, items[0].1);
    until(application, &context, "deleted", |application, _| {
        shows(application, Kind::Success, "git.tagDeleted")
    });
    assert_eq!(workspace.git(0, &["tag", "--list"]), "v2");
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "tags reread", |application, _| {
        application.git_state.cache.tags(&project).len() == 1
    });
    let items = delete_items(application, &context, SECOND, &["v1", "v2"]);
    assert_eq!(
        items
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["v2"]
    );
    frame(
        application,
        &context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    let revert = text(application, "git.revert");
    let delete = text(application, "git.deleteTag");
    right_click(application, &context, FIRST);
    let output = until(application, &context, "untagged menu", |_, output| {
        button(output, &revert).is_some()
    });
    assert!(submenu(&output, &delete).is_none(), "no tags, no submenu");
}

#[test]
fn 실제_app_팔레트_git_명령은_head를_revert하고_접힌_그래프에서도_포커스_slot에_head_태그_대화상자를_하나만_연다()
 {
    let mut workspace = Workspace::new("graph-commands", &["alpha", "beta"]);
    let (root, empty) = (workspace.roots[1].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ONE)]);
    std::fs::write(root.join("b.txt"), TWO).unwrap();
    workspace.git(1, &["add", "--", "b.txt"]);
    workspace.git(1, &["commit", "-m", SECOND]);
    let context = launch_graph(&mut workspace);
    let (projects, slots) = (workspace.projects.clone(), workspace.slots.clone());
    let application = workspace.fixture.application.as_mut().unwrap();
    let graph = text(application, "git.graph");
    let create = text(application, "git.createTag");
    let output = until(application, &context, "rows", |_, output| {
        commit_row(output, SECOND).is_some()
    });
    let (header, _) = button(&output, &graph).unwrap();
    click(application, &context, header);
    until(
        application,
        &context,
        "graph collapsed",
        |application, output| {
            commit_row(output, SECOND).is_none()
                && application
                    .services
                    .state
                    .settings
                    .read()
                    .git_sections_collapsed
                    .iter()
                    .any(|id| id == "graph")
        },
    );

    let before = errors(application);
    application
        .commands_state
        .palette_commands
        .push("git.revertHead".into());
    until(
        application,
        &context,
        "head revert answered",
        |application, _| {
            shows(application, Kind::Success, "git.revertSuccess") || errors(application) > before
        },
    );
    assert!(
        shows(application, Kind::Success, "git.revertSuccess"),
        "{:?}",
        application
            .toasts
            .inspection()
            .iter()
            .map(|toast| (toast.title.clone(), toast.description.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        workspace.git(1, &["log", "-1", "--format=%s"]),
        format!("Revert \"{SECOND}\"")
    );
    assert!(!root.join("b.txt").exists(), "file reverted");

    let application = workspace.fixture.application.as_mut().unwrap();
    application
        .explorer_search_state
        .symbol_sidebars
        .entry(&projects[1], &slots[1])
        .view = crate::symbol_sidebar::View::Files;
    settle(application, &context);

    application
        .commands_state
        .palette_commands
        .push("git.createTagOnHead".into());
    until(application, &context, "head tag dialog", |_, output| {
        button(output, &create).is_some()
    });
    let output = settle(application, &context);
    assert_eq!(
        nodes(&output, &create)
            .filter(|(_, node)| node.role() == egui::accesskit::Role::Button)
            .count(),
        1,
        "one dialog"
    );
    assert!(has_text(&output, "HEAD"));
    let mut view = |index: usize| {
        application
            .explorer_search_state
            .symbol_sidebars
            .entry(&projects[index], &slots[index])
            .view
    };
    assert_ne!(view(0), crate::symbol_sidebar::View::Git, "other slot");
    assert_eq!(view(1), crate::symbol_sidebar::View::Git, "focused slot");
    assert!(
        application
            .git_state
            .panels
            .collapsed(&projects[1], &slots[1])
            .unwrap()
            .is_collapsed(taide_native_ui::intents::git::Section::Graph),
        "graph stays collapsed"
    );
    typed(application, &context, "head-tag");
    let output = settle(application, &context);
    let (confirm, _) = button(&output, &create).unwrap();
    click(application, &context, confirm);
    until(application, &context, "head tag", |application, _| {
        shows(application, Kind::Success, "git.tagCreated")
    });
    assert_eq!(
        workspace.git(1, &["rev-parse", "head-tag^{commit}"]),
        workspace.git(1, &["rev-parse", "HEAD"])
    );
    assert_eq!(workspace.git(1, &["cat-file", "-t", "head-tag"]), "commit");
}

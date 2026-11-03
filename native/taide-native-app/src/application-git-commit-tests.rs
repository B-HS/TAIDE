use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui;
use taide_model::ai::{AiCommitMessageRequest, AiTextResponse};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::project::ShellSlotTree;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::toast::Kind;
use tokio::sync::oneshot;

use super::base_tests::{Workspace, frame, has_text, nodes, settle, show_git, text, until};
use super::fixture;
use crate::application::NativeApplication;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{isolated_state, launch, pointer};
use crate::presentation;

const ORIGINAL: &str = "one\n";
const EDITED: &str = "edited\n";
const OURS: &str = "ours\n";
const THEIRS: &str = "theirs\n";
const MESSAGE: &str = "feat: commit box";
const FAILING_MESSAGE: &str = "chore: nothing to commit";
const LOCK_FILE: &str = "Cargo.lock";
const SHORT_HASH: usize = 7;
const SAME_SPOT: f32 = 1.0;
const AWAY: egui::Pos2 = egui::pos2(1280.0, 800.0);

type Answer = AppResult<Option<String>>;

struct Call {
    request: AiCommitMessageRequest,
    answer: Option<oneshot::Sender<Answer>>,
    is_cancelled: Arc<AtomicBool>,
}

type Calls = Arc<Mutex<Vec<Call>>>;

/// An AI provider stand-in: it registers in the real request store like `ai_commit_message`,
/// notes whether a cancel reached that store, and answers when the test says so.
fn fake_ai(calls: Calls) -> crate::git_commit::AiPort {
    Arc::new(move |services, request| {
        let calls = calls.clone();
        Box::pin(async move {
            let (token, mut cancelled) = services
                .ai_requests
                .begin(&request.owner, &request.request_id)
                .expect("a fresh request id");
            let (answer, mut receive) = oneshot::channel();
            let is_cancelled = Arc::new(AtomicBool::new(false));
            calls.lock().unwrap().push(Call {
                request: request.clone(),
                answer: Some(answer),
                is_cancelled: is_cancelled.clone(),
            });
            let answer = tokio::select! {
                answer = &mut receive => answer,
                signal = &mut cancelled => {
                    is_cancelled.store(signal.is_ok(), Ordering::SeqCst);
                    receive.await
                }
            };
            services
                .ai_requests
                .finish(&request.owner, &request.request_id, &token);
            answer.unwrap_or(Ok(None)).map(|text| AiTextResponse {
                request_id: request.request_id,
                text,
            })
        })
    })
}

fn answer(calls: &Calls, index: usize, value: Answer) {
    let sender = calls.lock().unwrap()[index].answer.take().unwrap();
    sender.send(value).unwrap();
}

fn call_count(calls: &Calls) -> usize {
    calls.lock().unwrap().len()
}

fn is_cancelled(calls: &Calls, index: usize) -> bool {
    calls.lock().unwrap()[index]
        .is_cancelled
        .load(Ordering::SeqCst)
}

fn draft(application: &NativeApplication, project: &ProjectId) -> String {
    application
        .git_state
        .commit
        .draft(crate::application::WINDOW_LABEL, project)
        .to_owned()
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

fn is_enabled(output: &egui::FullOutput, label: &str) -> bool {
    button(output, label).is_some_and(|(_, node)| !node.is_disabled())
}

fn is_disabled(output: &egui::FullOutput, label: &str) -> bool {
    button(output, label).is_some_and(|(_, node)| node.is_disabled())
}

/// The commit message box: the multi-line text input with the commit placeholder.
fn input(application: &NativeApplication, output: &egui::FullOutput) -> Option<egui::Rect> {
    let placeholder = text(application, "git.commitMessagePlaceholder");
    output
        .platform_output
        .accesskit_update
        .iter()
        .flat_map(|update| update.nodes.iter())
        .filter(|(_, node)| {
            node.role() == egui::accesskit::Role::MultilineTextInput
                && node.placeholder() == Some(placeholder.as_str())
        })
        .find_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
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

/// Clicks into the message box and types. Waits for the panel to settle first: egui hit-tests a
/// click against the previous frame's widgets, and the panel's first frame draws before it mounts.
fn type_message(application: &mut NativeApplication, context: &egui::Context, message: &str) {
    until(
        application,
        context,
        "message box",
        |application, output| input(application, output).is_some(),
    );
    let output = settle(application, context);
    let target = input(application, &output).unwrap();
    click(application, context, target);
    typed(application, context, message);
}

/// Types into whatever has focus; a second click this soon would count as a double click.
fn typed(application: &mut NativeApplication, context: &egui::Context, message: &str) {
    frame(
        application,
        context,
        vec![egui::Event::Text(message.into())],
    );
}

fn mod_enter() -> Vec<egui::Event> {
    key(
        egui::Key::Enter,
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
    )
}

fn toasts(application: &NativeApplication, kind: Kind, title: &str) -> usize {
    application
        .toasts
        .inspection()
        .iter()
        .filter(|toast| toast.kind == kind && toast.title == title)
        .count()
}

fn description(application: &NativeApplication, kind: Kind, title: &str) -> Option<String> {
    application
        .toasts
        .inspection()
        .into_iter()
        .rev()
        .find(|toast| toast.kind == kind && toast.title == title)
        .and_then(|toast| toast.description)
}

fn refresh(application: &NativeApplication, project: &ProjectId) {
    for event in [
        AppEvent::GitStatusChanged {
            project_id: project.clone(),
        },
        AppEvent::GitRefsChanged {
            project_id: project.clone(),
        },
    ] {
        application.services.events.publish(event);
    }
}

fn launch_git(workspace: &mut Workspace, state: taide_runtime::AppState) -> egui::Context {
    let context = launch(&mut workspace.fixture, state);
    let application = workspace.fixture.application.as_mut().unwrap();
    settle(application, &context);
    show_git(application, &context);
    context
}

/// Every project open, but only the first one placed in the window's single slot, so activating
/// another project swaps what that slot shows.
fn single_slot(workspace: &Workspace) -> taide_runtime::AppState {
    let state = workspace.state(isolated_state(workspace.data.clone()));
    {
        let mut session = state.session.write();
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: workspace.slots[0].clone(),
            project_id: workspace.projects[0].clone(),
        });
        session.focused_shell_slot = Some(workspace.slots[0].clone());
        session.active_project = Some(workspace.projects[0].clone());
    }
    state
}

fn activate(application: &mut NativeApplication, context: &egui::Context, project: &ProjectId) {
    application
        .controller
        .submit(ShellMutation::ActivateProject(project.clone()))
        .unwrap();
    until(application, context, "activated", |application, _| {
        application.controller.snapshot().focused_project() == Some(project)
    });
    settle(application, context);
    show_git(application, context);
}

#[test]
fn 실제_app_커밋_박스는_빈_메시지와_진행_중을_막고_전체_스테이지를_확인한_뒤_커밋하며_mod_enter와_충돌을_원본대로_다룬다()
 {
    let mut workspace = Workspace::new("commit", &["alpha"]);
    let (root, empty) = (workspace.roots[0].clone(), workspace.empty.clone());
    fixture::repository(&root, &empty, &[("a.txt", ORIGINAL)]);
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    let initial = workspace.git(0, &["rev-parse", "HEAD"]);
    let state = workspace.state(isolated_state(workspace.data.clone()));
    let context = launch_git(&mut workspace, state);
    let project = workspace.projects[0].clone();
    let application = workspace.fixture.application.as_mut().unwrap();
    let commit = text(application, "git.commit");
    let committing = text(application, "git.committing");
    let placeholder = text(application, "git.commitMessagePlaceholder");
    let title = text(application, "git.stageAllTitle");
    let explanation = text(application, "git.stageAllDescription");
    let cancel = text(application, "common.cancel");
    let blocked = text(application, "git.commitBlockedByConflicts");

    let output = until(
        application,
        &context,
        "commit box",
        |application, output| {
            input(application, output).is_some()
                && has_text(output, &placeholder)
                && nodes(output, "a.txt").count() > 0
        },
    );
    assert!(
        is_disabled(&output, &commit),
        "an empty message cannot commit"
    );

    type_message(application, &context, "   ");
    let output = until(application, &context, "blank message", |application, _| {
        draft(application, &project) == "   "
    });
    assert!(
        is_disabled(&output, &commit),
        "a blank message cannot commit"
    );
    typed(application, &context, MESSAGE);
    let entered = format!("   {MESSAGE}");
    let output = settle(application, &context);
    assert_eq!(draft(application, &project), entered);
    assert!(is_enabled(&output, &commit));
    assert!(has_text(&output, &entered));
    assert!(!has_text(&output, &placeholder));

    frame(
        application,
        &context,
        [egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "\u{3131}".into(),
            active_range_chars: None,
        })]
        .into_iter()
        .chain(mod_enter())
        .collect(),
    );
    frame(
        application,
        &context,
        vec![egui::Event::Ime(egui::ImeEvent::Preedit {
            text: String::new(),
            active_range_chars: None,
        })],
    );
    let output = settle(application, &context);
    assert!(
        !has_text(&output, &title),
        "a composing mod+Enter is ignored"
    );
    assert_eq!(draft(application, &project), entered);

    frame(application, &context, mod_enter());
    let output = until(application, &context, "stage all dialog", |_, output| {
        has_text(output, &title) && has_text(output, &explanation)
    });
    assert_eq!(
        draft(application, &project),
        entered,
        "mod+Enter adds no line break"
    );
    click(application, &context, button(&output, &cancel).unwrap().0);
    until(application, &context, "cancelled", |_, output| {
        !has_text(output, &title)
    });
    assert_eq!(workspace.git(0, &["rev-parse", "HEAD"]), initial);

    let application = workspace.fixture.application.as_mut().unwrap();
    away(application, &context);
    let output = settle(application, &context);
    let (panel_button, _) = button(&output, &commit).unwrap();
    click(application, &context, panel_button);
    let output = until(application, &context, "dialog by button", |_, output| {
        has_text(output, &title) && buttons(output, &commit).count() == 2
    });
    let (confirm, _) = buttons(&output, &commit)
        .find(|(rect, _)| (rect.center() - panel_button.center()).length() > SAME_SPOT)
        .unwrap();
    let state = application.services.state.clone();
    let guard = state.begin_mutation_blocking();
    click(application, &context, confirm);
    until(
        application,
        &context,
        "committing",
        |application, output| {
            application.git_state.commit.committing.contains(&project)
                && !has_text(output, &title)
                && is_disabled(output, &committing)
        },
    );
    drop(guard);
    let application = workspace.fixture.application.as_mut().unwrap();
    let output = until(application, &context, "committed", |application, output| {
        draft(application, &project).is_empty()
            && has_text(output, &placeholder)
            && is_disabled(output, &commit)
            && nodes(output, "a.txt").count() == 0
    });
    assert!(!has_text(&output, &committing));
    assert_eq!(
        workspace.git(0, &["log", "-1", "--format=%s"]),
        entered.trim()
    );
    assert_eq!(workspace.git(0, &["status", "--porcelain"]), "");

    let application = workspace.fixture.application.as_mut().unwrap();
    let failures_before = application
        .toasts
        .inspection()
        .iter()
        .filter(|toast| toast.kind == Kind::Error)
        .count();
    type_message(application, &context, FAILING_MESSAGE);
    until(
        application,
        &context,
        "failing message",
        |application, output| {
            draft(application, &project) == FAILING_MESSAGE && is_enabled(output, &commit)
        },
    );
    frame(application, &context, mod_enter());
    until(application, &context, "commit failure", |application, _| {
        application
            .toasts
            .inspection()
            .iter()
            .filter(|toast| toast.kind == Kind::Error)
            .count()
            > failures_before
            && !application.git_state.commit.committing.contains(&project)
    });
    assert_eq!(draft(application, &project), FAILING_MESSAGE);
    assert_eq!(
        workspace.git(0, &["log", "-1", "--format=%s"]),
        entered.trim()
    );

    workspace.git(0, &["checkout", "-b", "other"]);
    std::fs::write(root.join("a.txt"), THEIRS).unwrap();
    workspace.git(0, &["commit", "-am", "test(fixture): theirs"]);
    workspace.git(0, &["checkout", "main"]);
    std::fs::write(root.join("a.txt"), OURS).unwrap();
    workspace.git(0, &["commit", "-am", "test(fixture): ours"]);
    let base = workspace.git(0, &["merge-base", "HEAD", "other"]);
    workspace.git(0, &["read-tree", "-m", &base, "HEAD", "other"]);
    let head = workspace.git(0, &["rev-parse", "HEAD"]);
    let application = workspace.fixture.application.as_mut().unwrap();
    refresh(application, &project);
    let output = until(application, &context, "conflict blocks", |_, output| {
        has_text(output, &blocked) && is_disabled(output, &commit)
    });
    assert_eq!(draft(application, &project), FAILING_MESSAGE);
    assert!(input(application, &output).is_some());
    frame(application, &context, mod_enter());
    let output = settle(application, &context);
    assert!(
        !has_text(&output, &title),
        "conflicts skip the stage-all dialog"
    );
    assert!(!application.git_state.commit.committing.contains(&project));
    assert_eq!(workspace.git(0, &["rev-parse", "HEAD"]), head);
    assert!(
        workspace
            .git(0, &["status", "--porcelain"])
            .starts_with("UU"),
        "the conflict stays unresolved"
    );
}

#[test]
fn 실제_app_커밋_초안은_프로젝트마다_기억되고_프로젝트_전환과_파일_보기를_다녀와도_남는다() {
    let mut workspace = Workspace::new("drafts", &["alpha", "beta"]);
    for root in &workspace.roots {
        fixture::repository(root, &workspace.empty, &[("a.txt", ORIGINAL)]);
    }
    let state = single_slot(&workspace);
    let context = launch_git(&mut workspace, state);
    let (alpha, beta) = (workspace.projects[0].clone(), workspace.projects[1].clone());
    let application = workspace.fixture.application.as_mut().unwrap();
    let placeholder = text(application, "git.commitMessagePlaceholder");

    type_message(application, &context, "alpha draft");
    let output = settle(application, &context);
    assert_eq!(draft(application, &alpha), "alpha draft");
    assert!(has_text(&output, "alpha draft"));

    activate(application, &context, &beta);
    let output = until(application, &context, "beta box", |application, output| {
        input(application, output).is_some() && has_text(output, &placeholder)
    });
    assert!(!has_text(&output, "alpha draft"));
    type_message(application, &context, "beta draft");
    until(
        application,
        &context,
        "beta typed",
        |application, output| {
            draft(application, &beta) == "beta draft" && has_text(output, "beta draft")
        },
    );

    activate(application, &context, &alpha);
    let output = until(application, &context, "alpha restored", |_, output| {
        has_text(output, "alpha draft")
    });
    assert!(!has_text(&output, "beta draft"));

    frame(
        application,
        &context,
        key(
            egui::Key::E,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        ),
    );
    until(
        application,
        &context,
        "files view",
        |application, output| input(application, output).is_none(),
    );
    show_git(application, &context);
    until(application, &context, "back to git", |_, output| {
        has_text(output, "alpha draft")
    });
    assert_eq!(draft(application, &beta), "beta draft");
}

#[test]
fn 실제_app_ai_커밋_메시지는_정리한_응답과_안내를_넣고_취소_늦은_응답_실패_프로젝트_전환을_원본대로_다룬다()
 {
    let mut workspace = Workspace::new("ai-message", &["alpha", "beta"]);
    for root in &workspace.roots {
        fixture::repository(root, &workspace.empty, &[("a.txt", ORIGINAL)]);
    }
    let root = workspace.roots[0].clone();
    std::fs::write(root.join("a.txt"), EDITED).unwrap();
    std::fs::write(root.join(LOCK_FILE), "synthetic lock\n").unwrap();
    let head = workspace.git(0, &["rev-parse", "HEAD"]);
    let initial = workspace.git(0, &["log", "-1", "--format=%s"]);
    let state = single_slot(&workspace);
    let context = launch(&mut workspace.fixture, state);
    let (alpha, beta) = (workspace.projects[0].clone(), workspace.projects[1].clone());
    let calls = Calls::default();
    let application = workspace.fixture.application.as_mut().unwrap();
    application.git_state.commit.ai = fake_ai(calls.clone());
    settle(application, &context);
    show_git(application, &context);
    let generate = text(application, "git.generateCommitMessage");
    let generating = text(application, "git.generatingCommitMessage");
    let nothing = text(application, "git.noChangesForCommitMessage");
    let generated = text(application, "git.commitMessageGenerated");
    let failed = text(application, "git.generateCommitMessageFailed");
    let empty_response = text(application, "git.commitMessageEmptyResponse");

    let output = until(
        application,
        &context,
        "generate enabled",
        |application, output| {
            is_enabled(output, &generate) && application.git_state.cache.log(&alpha).len() == 1
        },
    );
    click(application, &context, button(&output, &generate).unwrap().0);
    until(application, &context, "first request", |_, output| {
        call_count(&calls) == 1 && is_enabled(output, &generating)
    });
    {
        let calls = calls.lock().unwrap();
        let request = &calls[0].request;
        assert_eq!(request.owner, crate::application::WINDOW_LABEL);
        assert_eq!((request.provider, request.model.as_deref()), (None, None));
        assert_eq!(
            request.recent_commits,
            format!("{} {initial}", &head[..SHORT_HASH])
        );
        assert!(request.diff_text.contains("edited"));
        assert!(!request.diff_text.contains("synthetic lock"));
    }
    answer(&calls, 0, Ok(Some("```\n\"feat: generated\"\n```".into())));
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "generated", |application, output| {
        draft(application, &alpha) == "feat: generated"
            && has_text(output, "feat: generated")
            && toasts(application, Kind::Success, &generated) == 1
            && is_enabled(output, &generate)
    });
    let skipped = presentation::message(
        &application.locale,
        "git.commitMessageFilesSkipped",
        &[("count", "1")],
    );
    assert_eq!(
        description(application, Kind::Success, &generated),
        Some(format!(
            "{} · {skipped}",
            text(application, "git.commitMessageUsedUnstaged")
        ))
    );

    away(application, &context);
    let output = settle(application, &context);
    let (ai_button, _) = button(&output, &generate).unwrap();
    click(application, &context, ai_button);
    until(application, &context, "second request", |_, output| {
        call_count(&calls) == 2 && is_enabled(output, &generating)
    });
    click(application, &context, ai_button);
    until(application, &context, "cancelled", |_, output| {
        is_cancelled(&calls, 1) && is_enabled(output, &generate)
    });
    let successes = toasts(application, Kind::Success, &generated);
    answer(&calls, 1, Ok(Some("feat: late".into())));
    let output = settle(application, &context);
    assert_eq!(draft(application, &alpha), "feat: generated");
    assert!(!has_text(&output, "feat: late"));
    assert!(toasts(application, Kind::Success, &generated) <= successes);

    click(application, &context, ai_button);
    until(application, &context, "third request", |_, _| {
        call_count(&calls) == 3
    });
    answer(
        &calls,
        2,
        Err(AppError::Internal("synthetic ai failure".into())),
    );
    let application = workspace.fixture.application.as_mut().unwrap();
    until(
        application,
        &context,
        "failure toast",
        |application, output| {
            toasts(application, Kind::Error, &failed) == 1 && is_enabled(output, &generate)
        },
    );
    assert_eq!(
        description(application, Kind::Error, &failed),
        Some(crate::toast::describe_error(
            &application.locale,
            &AppError::Internal("synthetic ai failure".into())
        ))
    );

    click(application, &context, ai_button);
    until(application, &context, "fourth request", |_, _| {
        call_count(&calls) == 4
    });
    answer(&calls, 3, Ok(Some("```\n \n```".into())));
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "empty response", |application, _| {
        toasts(application, Kind::Error, &empty_response) == 1
    });
    assert_eq!(draft(application, &alpha), "feat: generated");

    click(application, &context, ai_button);
    until(application, &context, "fifth request", |_, _| {
        call_count(&calls) == 5
    });
    answer(&calls, 4, Ok(None));
    let application = workspace.fixture.application.as_mut().unwrap();
    until(application, &context, "no text", |_, output| {
        is_enabled(output, &generate)
    });
    assert_eq!(draft(application, &alpha), "feat: generated");

    click(application, &context, ai_button);
    until(application, &context, "sixth request", |_, output| {
        call_count(&calls) == 6 && is_enabled(output, &generating)
    });
    activate(application, &context, &beta);
    let output = until(application, &context, "beta panel", |_, output| {
        is_disabled(output, &nothing)
    });
    assert!(!has_text(&output, "feat: generated"));
    until(application, &context, "switch cancels", |_, _| {
        is_cancelled(&calls, 5)
    });
    answer(&calls, 5, Ok(Some("feat: wrong project".into())));
    let output = settle(application, &context);
    assert_eq!(draft(application, &beta), "");
    assert_eq!(draft(application, &alpha), "feat: generated");
    assert!(!has_text(&output, "feat: wrong project"));
}

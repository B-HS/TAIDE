use std::path::Path;

use futures_util::FutureExt;
use taide_ide::protocol::{at_mentioned_notification, selection_changed_notification};
use taide_ide::store::{IdeStore, PendingDiff, PendingSave};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ide::{IdeDiagnostic, IdeDiagnosticSeverity, IdeDiffOutcome, IdeSelectionInput, IdeStatus};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::remote::REMOTE_OWNER_LABEL;
use taide_runtime::{ide_actions, AppState, IdeSaveFile};
use tokio::sync::oneshot;
use uuid::Uuid;

const IDE_COMMANDS: &[&str] = &[
    "ide_get_status",
    "ide_set_selection",
    "ide_clear_selection",
    "ide_publish_diagnostics",
    "ide_resolve_diff",
    "ide_resolve_save",
    "ide_notify_at_mention",
];

fn state() -> AppState {
    AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-ide-actions-{}", Uuid::new_v4())),
    ))
}

fn save_success(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    assert_eq!(path, state.paths.data_dir.join("diff.txt"));
    assert_eq!(content, "replacement");
    assert!(
        state.begin_mutation().now_or_never().is_none(),
        "저장 port 호출 중 guard를 보유한다"
    );
    state.session.write().window_chrome.zen = true;
    Ok(())
}

fn save_forbidden(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    save_success(state, path, content)?;
    Err(AppError::Forbidden("fixture root no longer open".to_string()))
}

fn save_internal_error(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    save_success(state, path, content)?;
    Err(AppError::Internal("fixture save failed".to_string()))
}

fn unexpected_save(_: &AppState, _: &Path, _: &str) -> AppResult<()> {
    panic!("이 경로는 저장 port를 호출하지 않는다")
}

fn pending_diff(ide: &IdeStore, state: &AppState, request_id: &str) -> oneshot::Receiver<(IdeDiffOutcome, Option<String>)> {
    let (sender, receiver) = oneshot::channel();
    ide.insert_pending_diff(
        request_id.to_string(),
        PendingDiff {
            project_id: ProjectId::new(),
            new_path: state.paths.data_dir.join("diff.txt"),
            responder: sender,
        },
    );
    receiver
}

#[tokio::test]
async fn desktop_selection만_state를_갱신하고_notification을_발행하며_remote는_무시한다() {
    let ide = IdeStore::default();
    let mut receiver = ide.subscribe();
    let input = IdeSelectionInput {
        owner: "main".to_string(),
        project_id: ProjectId::new(),
        path: "fixture.txt".to_string(),
        text: "selection".to_string(),
        start_line: 0,
        start_character: 0,
        end_line: 1,
        end_character: 0,
        is_empty: false,
    };
    ide_actions::ide_set_selection(&ide, input.clone()).await.unwrap();
    let selection = ide.current_selection().unwrap();
    assert_eq!(selection.project_id, input.project_id);
    assert_eq!(selection.path, input.path);
    assert_eq!(selection.text, input.text);
    assert_eq!(selection.start_line, input.start_line);
    assert_eq!(selection.start_character, input.start_character);
    assert_eq!(selection.end_line, input.end_line);
    assert_eq!(selection.end_character, input.end_character);
    assert_eq!(selection.is_empty, input.is_empty);
    assert_eq!(receiver.try_recv().unwrap(), selection_changed_notification(&selection));
    ide_actions::ide_set_selection(
        &ide,
        IdeSelectionInput {
            owner: REMOTE_OWNER_LABEL.to_string(),
            text: "remote".to_string(),
            ..input
        },
    )
    .await
    .unwrap();
    ide_actions::ide_clear_selection(&ide, REMOTE_OWNER_LABEL.to_string())
        .await
        .unwrap();
    assert_eq!(ide.current_selection(), Some(selection.clone()));
    assert_eq!(ide.latest_selection(), Some(selection.clone()));
    assert!(receiver.try_recv().is_err());
    ide_actions::ide_clear_selection(&ide, "main".to_string()).await.unwrap();
    assert!(ide.current_selection().is_none());
    assert_eq!(ide.latest_selection(), Some(selection));
    assert!(receiver.try_recv().is_err(), "기존 clear는 notification을 추가하지 않는다");
}

#[tokio::test]
async fn status_diagnostics_at_mention은_기존_store와_protocol을_사용한다() {
    let ide = IdeStore::default();
    assert_eq!(ide_actions::ide_get_status(&ide).await.unwrap(), IdeStatus::default());
    assert!(ide.diagnostics(None).is_none());
    let diagnostic = IdeDiagnostic {
        path: "fixture.txt".to_string(),
        severity: IdeDiagnosticSeverity::Warning,
        start_line: 0,
        start_character: 0,
        end_line: 1,
        end_character: 0,
        message: "fixture".to_string(),
        source: None,
    };
    ide_actions::ide_publish_diagnostics(&ide, ProjectId::new(), vec![diagnostic.clone()])
        .await
        .unwrap();
    assert_eq!(ide.diagnostics(None), Some(vec![diagnostic]));
    let mut receiver = ide.subscribe();
    ide_actions::ide_notify_at_mention(&ide, "fixture.txt".to_string(), 0, 1)
        .await
        .unwrap();
    assert_eq!(receiver.try_recv().unwrap(), at_mentioned_notification("fixture.txt", 0, 1));
}

#[tokio::test]
async fn saved_diff는_guard를_보유해_port를_호출한_뒤_content를_응답한다() {
    let state = state();
    let ide = IdeStore::default();
    let receiver = pending_diff(&ide, &state, "saved");
    ide_actions::ide_resolve_diff(
        &state,
        &IdeSaveFile(save_success),
        &ide,
        "saved".to_string(),
        IdeDiffOutcome::Saved,
        Some("replacement".to_string()),
    )
    .await
    .unwrap();
    assert!(state.session.read().window_chrome.zen);
    assert_eq!(receiver.await.unwrap(), (IdeDiffOutcome::Saved, Some("replacement".to_string())));
    assert!(state.begin_mutation().now_or_never().is_some());
    assert!(ide.take_pending_diff("saved").is_none());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn forbidden_diff_저장_오류만_기존_saved_resolution으로_완료한다() {
    let state = state();
    let ide = IdeStore::default();
    let receiver = pending_diff(&ide, &state, "forbidden");
    ide_actions::ide_resolve_diff(
        &state,
        &IdeSaveFile(save_forbidden),
        &ide,
        "forbidden".to_string(),
        IdeDiffOutcome::Saved,
        Some("replacement".to_string()),
    )
    .await
    .unwrap();
    assert!(state.session.read().window_chrome.zen);
    assert_eq!(receiver.await.unwrap(), (IdeDiffOutcome::Saved, Some("replacement".to_string())));
    assert!(state.begin_mutation().now_or_never().is_some());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 다른_저장_오류는_전파하고_이미_소비한_pending의_응답을_보내지_않는다() {
    let state = state();
    let ide = IdeStore::default();
    let receiver = pending_diff(&ide, &state, "error");
    let error = ide_actions::ide_resolve_diff(
        &state,
        &IdeSaveFile(save_internal_error),
        &ide,
        "error".to_string(),
        IdeDiffOutcome::Saved,
        Some("replacement".to_string()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert!(state.session.read().window_chrome.zen);
    assert!(receiver.await.is_err());
    assert!(ide.take_pending_diff("error").is_none());
    assert!(state.begin_mutation().now_or_never().is_some());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn saved_content_누락은_pending을_먼저_소비한_뒤_저장_전에_거절한다() {
    let state = state();
    let ide = IdeStore::default();
    let receiver = pending_diff(&ide, &state, "missing-content");
    let error = ide_actions::ide_resolve_diff(
        &state,
        &IdeSaveFile(unexpected_save),
        &ide,
        "missing-content".to_string(),
        IdeDiffOutcome::Saved,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(receiver.await.is_err());
    assert!(ide.take_pending_diff("missing-content").is_none());
    assert!(state.begin_mutation().now_or_never().is_some());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn rejected와_tab_closed는_저장_없이_content_none을_응답한다() {
    let state = state();
    let ide = IdeStore::default();
    for outcome in [IdeDiffOutcome::Rejected, IdeDiffOutcome::TabClosed] {
        let receiver = pending_diff(&ide, &state, "not-saved");
        ide_actions::ide_resolve_diff(
            &state,
            &IdeSaveFile(unexpected_save),
            &ide,
            "not-saved".to_string(),
            outcome,
            Some("ignored".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(receiver.await.unwrap(), (outcome, None));
    }
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn pending_누락은_기존_not_found이고_save_응답은_bool을_그대로_전달한다() {
    let state = state();
    let ide = IdeStore::default();
    let diff_error = ide_actions::ide_resolve_diff(
        &state,
        &IdeSaveFile(unexpected_save),
        &ide,
        "missing".to_string(),
        IdeDiffOutcome::Saved,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(diff_error.kind(), AppErrorKind::NotFound);
    let save_error = ide_actions::ide_resolve_save(&ide, "missing".to_string(), true).await.unwrap_err();
    assert_eq!(save_error.kind(), AppErrorKind::NotFound);
    for saved in [false, true] {
        let (sender, receiver) = oneshot::channel();
        ide.insert_pending_save("save".to_string(), PendingSave { responder: sender });
        ide_actions::ide_resolve_save(&ide, "save".to_string(), saved).await.unwrap();
        assert_eq!(receiver.await.unwrap(), saved);
        assert!(ide.take_pending_save("save").is_none());
    }
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 이미_닫힌_responder의_send_실패는_기존처럼_성공으로_반환한다() {
    let state = state();
    let ide = IdeStore::default();
    drop(pending_diff(&ide, &state, "closed-diff"));
    ide_actions::ide_resolve_diff(
        &state,
        &IdeSaveFile(unexpected_save),
        &ide,
        "closed-diff".to_string(),
        IdeDiffOutcome::Rejected,
        None,
    )
    .await
    .unwrap();
    let (sender, receiver) = oneshot::channel();
    ide.insert_pending_save("closed-save".to_string(), PendingSave { responder: sender });
    drop(receiver);
    ide_actions::ide_resolve_save(&ide, "closed-save".to_string(), true).await.unwrap();
    assert!(!state.paths.data_dir.exists());
}

#[test]
fn 일곱_공개_command는_기존_tauri_경로에서_runtime으로_위임한다() {
    let source = include_str!("../src/domain/ide/commands.rs");
    for command in IDE_COMMANDS {
        let body = source
            .split_once(&format!("pub async fn {command}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains(&format!("ide_actions::{command}(")), "{command}");
    }
}

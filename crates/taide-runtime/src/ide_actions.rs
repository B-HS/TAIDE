use taide_ide::protocol::{at_mentioned_notification, selection_changed_notification};
use taide_ide::store::{IdeSelectionSnapshot, IdeStore};
use taide_model::error::{AppError, AppResult};
use taide_model::ide::{IdeDiagnostic, IdeDiffOutcome, IdeSelectionInput, IdeStatus};
use taide_model::ids::ProjectId;

use crate::{AppState, IdeSaveFile};

/// Applies the shared ide get status policy.
pub async fn ide_get_status(ide: &IdeStore) -> AppResult<IdeStatus> {
    Ok(ide.status())
}

/// Applies the shared ide set selection policy.
pub async fn ide_set_selection(ide: &IdeStore, input: IdeSelectionInput) -> AppResult<()> {
    if !IdeStore::is_desktop_owner(&input.owner) {
        return Ok(());
    }

    let selection = IdeSelectionSnapshot {
        project_id: input.project_id,
        path: input.path,
        text: input.text,
        start_line: input.start_line,
        start_character: input.start_character,
        end_line: input.end_line,
        end_character: input.end_character,
        is_empty: input.is_empty,
    };
    let notification = selection_changed_notification(&selection);
    ide.set_selection(selection);
    ide.broadcast(notification);
    Ok(())
}

/// Applies the shared ide clear selection policy.
pub async fn ide_clear_selection(ide: &IdeStore, owner: String) -> AppResult<()> {
    if !IdeStore::is_desktop_owner(&owner) {
        return Ok(());
    }
    ide.clear_selection();
    Ok(())
}

/// Applies the shared ide publish diagnostics policy.
pub async fn ide_publish_diagnostics(ide: &IdeStore, project_id: ProjectId, items: Vec<IdeDiagnostic>) -> AppResult<()> {
    ide.publish_diagnostics(project_id, items);
    Ok(())
}

/// Applies the shared ide resolve diff policy.
pub async fn ide_resolve_diff(
    state: &AppState,
    save_file: &IdeSaveFile,
    ide: &IdeStore,
    request_id: String,
    outcome: IdeDiffOutcome,
    content: Option<String>,
) -> AppResult<()> {
    let pending = ide
        .take_pending_diff(&request_id)
        .ok_or_else(|| AppError::NotFound(format!("pending diff not found: {request_id}")))?;

    let resolved_content = match outcome {
        IdeDiffOutcome::Saved => {
            let content = content.ok_or_else(|| AppError::InvalidArgument("saved outcome requires content".to_string()))?;
            let _guard = state.begin_mutation().await;
            match (save_file.0)(state, &pending.new_path, &content) {
                Ok(()) => {}
                Err(AppError::Forbidden(message)) => {
                    log::warn!("IDE diff 저장 대상이 더 이상 프로젝트 루트 안에 있지 않습니다: {message}");
                }
                Err(error) => return Err(error),
            }
            Some(content)
        }
        IdeDiffOutcome::Rejected | IdeDiffOutcome::TabClosed => None,
    };

    let _ = pending.responder.send((outcome, resolved_content));
    Ok(())
}

/// Applies the shared ide resolve save policy.
pub async fn ide_resolve_save(ide: &IdeStore, request_id: String, saved: bool) -> AppResult<()> {
    let pending = ide
        .take_pending_save(&request_id)
        .ok_or_else(|| AppError::NotFound(format!("pending save not found: {request_id}")))?;
    let _ = pending.responder.send(saved);
    Ok(())
}

/// Applies the shared ide notify at mention policy.
pub async fn ide_notify_at_mention(ide: &IdeStore, path: String, line_start: u32, line_end: u32) -> AppResult<()> {
    ide.broadcast(at_mentioned_notification(&path, line_start, line_end));
    Ok(())
}

use tauri::State;

pub use taide_runtime::AiRequestStore;

use crate::domain::ai::prompt;
use crate::domain::ai::service;
use crate::domain::ai::types::{
    AiCommitMessageRequest, AiInlineCompleteRequest, AiInlineEditRequest, AiModelInfo, AiProviderId, AiTextResponse, AiTokenStatus,
};
use crate::error::{AppError, AppResult};
use crate::infra::http::{outbound_http_client, HttpClientProfile};
use crate::infra::secret::SecretStoreState;
use crate::state::AppState;

/// Sanity upper bounds (byte length, not a token budget) on `ai_inline_complete`/`ai_inline_edit`/
/// `ai_commit_message` request fields. All three commands are remotely dispatchable
/// (`remote_gateway.rs`), and the normal UI flow naturally bounds these (editor
/// selection/context-window size, `git_diff_staged_text`'s own `STAGED_DIFF_TEXT_MAX_BYTES` cap) —
/// but a caller that skips that flow entirely (a raw IPC/remote call) has nothing else stopping it
/// from handing an arbitrarily large payload to the provider HTTP request. Rejected outright (not
/// truncated) so the "selection/instruction are never truncated" contract (`prompt.rs`'s
/// `INLINE_EDIT_CONTEXT_CHAR_LIMIT` doc comment) still holds for every request this store actually
/// processes.
const AI_INLINE_COMPLETE_PREFIX_MAX_BYTES: usize = 32 * 1024;
const AI_INLINE_COMPLETE_SUFFIX_MAX_BYTES: usize = 16 * 1024;
const AI_INLINE_EDIT_SELECTION_MAX_BYTES: usize = 100 * 1024;
const AI_INLINE_EDIT_INSTRUCTION_MAX_BYTES: usize = 4 * 1024;
const AI_COMMIT_MESSAGE_DIFF_MAX_BYTES: usize = 64 * 1024;
const AI_COMMIT_MESSAGE_RECENT_COMMITS_MAX_BYTES: usize = 8 * 1024;

fn ensure_within_byte_limit(field_name: &str, value: &str, max_bytes: usize) -> AppResult<()> {
    if value.len() > max_bytes {
        return Err(AppError::InvalidArgument(format!(
            "'{field_name}' is {} bytes, exceeding the {max_bytes}-byte limit",
            value.len()
        )));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ai_token_status(state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<AiTokenStatus> {
    let omlx_base_url = state.settings.read().ai_omlx_base_url.clone();
    service::token_status(secret.0.as_ref(), omlx_base_url.as_deref())
}

#[tauri::command]
#[specta::specta]
pub async fn ai_set_token(secret: State<'_, SecretStoreState>, provider: AiProviderId, token: String) -> AppResult<()> {
    let client = outbound_http_client(HttpClientProfile::Api);
    service::set_token(secret.0.as_ref(), &client, provider, token).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_clear_token(secret: State<'_, SecretStoreState>, provider: AiProviderId) -> AppResult<()> {
    service::clear_token(secret.0.as_ref(), provider)
}

#[tauri::command]
#[specta::specta]
pub async fn ai_list_models(
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
    provider: AiProviderId,
) -> AppResult<Vec<AiModelInfo>> {
    let omlx_base_url = state.settings.read().ai_omlx_base_url.clone();
    let client = outbound_http_client(HttpClientProfile::Api);
    service::list_models(secret.0.as_ref(), &client, provider, omlx_base_url).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_inline_complete(
    state: State<'_, AppState>,
    request_store: State<'_, AiRequestStore>,
    secret: State<'_, SecretStoreState>,
    request: AiInlineCompleteRequest,
) -> AppResult<AiTextResponse> {
    ensure_within_byte_limit("prefix", &request.prefix, AI_INLINE_COMPLETE_PREFIX_MAX_BYTES)?;
    ensure_within_byte_limit("suffix", &request.suffix, AI_INLINE_COMPLETE_SUFFIX_MAX_BYTES)?;

    let Some((request_token, cancel_rx)) = request_store.begin(&request.owner, &request.request_id) else {
        return Err(AppError::InvalidArgument(format!(
            "an inline completion request with id '{}' is already in flight",
            request.request_id
        )));
    };

    let template = prompt::load_prompt_template(&state.paths);
    let omlx_base_url = state.settings.read().ai_omlx_base_url.clone();
    let client = outbound_http_client(HttpClientProfile::Api);

    let text = tokio::select! {
        result = service::complete(secret.0.as_ref(), &client, &request, &template, omlx_base_url) => result,
        _ = cancel_rx => Ok(None),
    };

    request_store.finish(&request.owner, &request.request_id, &request_token);

    Ok(AiTextResponse {
        request_id: request.request_id,
        text: text?,
    })
}

/// `provider`/`model` are resolved before `request_store.begin()` — resolving after would leave a
/// `begin()`ed entry stranded with no matching `finish()` on a resolution failure. A later
/// request with the same owner and request ID would then be rejected as already in flight.
#[tauri::command]
#[specta::specta]
pub async fn ai_inline_edit(
    state: State<'_, AppState>,
    request_store: State<'_, AiRequestStore>,
    secret: State<'_, SecretStoreState>,
    request: AiInlineEditRequest,
) -> AppResult<AiTextResponse> {
    ensure_within_byte_limit("selection", &request.selection, AI_INLINE_EDIT_SELECTION_MAX_BYTES)?;
    ensure_within_byte_limit("instruction", &request.instruction, AI_INLINE_EDIT_INSTRUCTION_MAX_BYTES)?;

    let (provider, model, omlx_base_url) = {
        let settings = state.settings.read();
        let (provider, model) = service::resolve_provider_and_model(
            request.provider,
            request.model.clone(),
            settings.ai_provider,
            settings.ai_model.clone(),
        )?;
        (provider, model, settings.ai_omlx_base_url.clone())
    };

    let Some((request_token, cancel_rx)) = request_store.begin(&request.owner, &request.request_id) else {
        return Err(AppError::InvalidArgument(format!(
            "an AI request with id '{}' is already in flight",
            request.request_id
        )));
    };

    let template = prompt::load_inline_edit_prompt_template(&state.paths);
    let client = outbound_http_client(HttpClientProfile::Api);

    let text = tokio::select! {
        result = service::inline_edit(secret.0.as_ref(), &client, provider, &model, &request, &template, omlx_base_url) => result,
        _ = cancel_rx => Ok(None),
    };

    request_store.finish(&request.owner, &request.request_id, &request_token);

    Ok(AiTextResponse {
        request_id: request.request_id,
        text: text?,
    })
}

/// `provider`/`model` are resolved before `request_store.begin()` — see [`ai_inline_edit`]'s doc
/// comment for why.
#[tauri::command]
#[specta::specta]
pub async fn ai_commit_message(
    state: State<'_, AppState>,
    request_store: State<'_, AiRequestStore>,
    secret: State<'_, SecretStoreState>,
    request: AiCommitMessageRequest,
) -> AppResult<AiTextResponse> {
    ensure_within_byte_limit("diffText", &request.diff_text, AI_COMMIT_MESSAGE_DIFF_MAX_BYTES)?;
    ensure_within_byte_limit("recentCommits", &request.recent_commits, AI_COMMIT_MESSAGE_RECENT_COMMITS_MAX_BYTES)?;

    let (provider, model, omlx_base_url) = {
        let settings = state.settings.read();
        let (provider, model) = service::resolve_provider_and_model(
            request.provider,
            request.model.clone(),
            settings.ai_provider,
            settings.ai_model.clone(),
        )?;
        (provider, model, settings.ai_omlx_base_url.clone())
    };

    let Some((request_token, cancel_rx)) = request_store.begin(&request.owner, &request.request_id) else {
        return Err(AppError::InvalidArgument(format!(
            "an AI request with id '{}' is already in flight",
            request.request_id
        )));
    };

    let template = prompt::load_commit_message_prompt_template(&state.paths);
    let client = outbound_http_client(HttpClientProfile::Api);

    let text = tokio::select! {
        result = service::commit_message(secret.0.as_ref(), &client, provider, &model, &request, &template, omlx_base_url) => result,
        _ = cancel_rx => Ok(None),
    };

    request_store.finish(&request.owner, &request.request_id, &request_token);

    Ok(AiTextResponse {
        request_id: request.request_id,
        text: text?,
    })
}

/// `owner` (see [`AiInlineCompleteRequest::owner`](crate::domain::ai::types::AiInlineCompleteRequest)'s
/// doc comment) must match the `owner` the in-flight request itself was `begin()`ed with — otherwise
/// this could cancel a same-`requestId` request actually in flight in a different window (R6#20).
#[tauri::command]
#[specta::specta]
pub async fn ai_request_cancel(request_store: State<'_, AiRequestStore>, owner: String, request_id: String) -> AppResult<()> {
    request_store.cancel(&owner, &request_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_within_byte_limit은_상한_이내면_통과한다() {
        assert!(ensure_within_byte_limit("selection", "short", 10).is_ok());
    }

    #[test]
    fn ensure_within_byte_limit은_상한을_넘으면_에러를_반환한다() {
        let result = ensure_within_byte_limit("selection", "this is too long", 5);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn ensure_within_byte_limit은_정확히_상한과_같으면_통과한다() {
        assert!(ensure_within_byte_limit("selection", "12345", 5).is_ok());
    }

    #[test]
    fn ai_inline_complete의_prefix_상한을_넘으면_거부된다() {
        let oversized_prefix = "a".repeat(AI_INLINE_COMPLETE_PREFIX_MAX_BYTES + 1);
        let result = ensure_within_byte_limit("prefix", &oversized_prefix, AI_INLINE_COMPLETE_PREFIX_MAX_BYTES);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn ai_inline_complete의_suffix_상한을_넘으면_거부된다() {
        let oversized_suffix = "a".repeat(AI_INLINE_COMPLETE_SUFFIX_MAX_BYTES + 1);
        let result = ensure_within_byte_limit("suffix", &oversized_suffix, AI_INLINE_COMPLETE_SUFFIX_MAX_BYTES);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn ai_inline_complete의_prefix_suffix는_상한_이내면_통과한다() {
        assert!(ensure_within_byte_limit("prefix", "fn main() {}", AI_INLINE_COMPLETE_PREFIX_MAX_BYTES).is_ok());
        assert!(ensure_within_byte_limit("suffix", "", AI_INLINE_COMPLETE_SUFFIX_MAX_BYTES).is_ok());
    }
}

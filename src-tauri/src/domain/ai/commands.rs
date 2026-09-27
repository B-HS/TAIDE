use tauri::State;

use taide_runtime::ai_actions;
pub use taide_runtime::AiRequestStore;

use crate::domain::ai::types::{
    AiCommitMessageRequest, AiInlineCompleteRequest, AiInlineEditRequest, AiModelInfo, AiProviderId, AiTextResponse, AiTokenStatus,
};
use crate::error::AppResult;
use crate::infra::secret::SecretStoreState;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn ai_token_status(state: State<'_, AppState>, secret: State<'_, SecretStoreState>) -> AppResult<AiTokenStatus> {
    ai_actions::ai_token_status(&state, &secret).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_set_token(secret: State<'_, SecretStoreState>, provider: AiProviderId, token: String) -> AppResult<()> {
    ai_actions::ai_set_token(&secret, provider, token).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_clear_token(secret: State<'_, SecretStoreState>, provider: AiProviderId) -> AppResult<()> {
    ai_actions::ai_clear_token(&secret, provider).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_list_models(
    state: State<'_, AppState>,
    secret: State<'_, SecretStoreState>,
    provider: AiProviderId,
) -> AppResult<Vec<AiModelInfo>> {
    ai_actions::ai_list_models(&state, &secret, provider).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_inline_complete(
    state: State<'_, AppState>,
    request_store: State<'_, AiRequestStore>,
    secret: State<'_, SecretStoreState>,
    request: AiInlineCompleteRequest,
) -> AppResult<AiTextResponse> {
    ai_actions::ai_inline_complete(&state, &request_store, &secret, request).await
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
    ai_actions::ai_inline_edit(&state, &request_store, &secret, request).await
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
    ai_actions::ai_commit_message(&state, &request_store, &secret, request).await
}

/// `owner` (see [`AiInlineCompleteRequest::owner`](crate::domain::ai::types::AiInlineCompleteRequest)'s
/// doc comment) must match the `owner` the in-flight request itself was `begin()`ed with — otherwise
/// this could cancel a same-`requestId` request actually in flight in a different window (R6#20).
#[tauri::command]
#[specta::specta]
pub async fn ai_request_cancel(request_store: State<'_, AiRequestStore>, owner: String, request_id: String) -> AppResult<()> {
    ai_actions::ai_request_cancel(&request_store, owner, request_id).await
}

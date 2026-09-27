use crate::{AiRequestStore, AppState};

use taide_ai::prompt;
use taide_ai::service;
use taide_infra::http::{outbound_http_client, HttpClientProfile};
use taide_infra::secret::SecretStoreState;
use taide_model::ai::{
    AiCommitMessageRequest, AiInlineCompleteRequest, AiInlineEditRequest, AiModelInfo, AiProviderId, AiTextResponse, AiTokenStatus,
};
use taide_model::error::{AppError, AppResult};

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

/// Executes the shared AI action without toolkit state.
pub async fn ai_token_status(state: &AppState, secret: &SecretStoreState) -> AppResult<AiTokenStatus> {
    let omlx_base_url = state.settings.read().ai_omlx_base_url.clone();
    service::token_status(secret.0.as_ref(), omlx_base_url.as_deref())
}

/// Executes the shared AI action without toolkit state.
pub async fn ai_set_token(secret: &SecretStoreState, provider: AiProviderId, token: String) -> AppResult<()> {
    let client = outbound_http_client(HttpClientProfile::Api);
    service::set_token(secret.0.as_ref(), &client, provider, token).await
}

/// Executes the shared AI action without toolkit state.
pub async fn ai_clear_token(secret: &SecretStoreState, provider: AiProviderId) -> AppResult<()> {
    service::clear_token(secret.0.as_ref(), provider)
}

/// Executes the shared AI action without toolkit state.
pub async fn ai_list_models(state: &AppState, secret: &SecretStoreState, provider: AiProviderId) -> AppResult<Vec<AiModelInfo>> {
    let omlx_base_url = state.settings.read().ai_omlx_base_url.clone();
    let client = outbound_http_client(HttpClientProfile::Api);
    service::list_models(secret.0.as_ref(), &client, provider, omlx_base_url).await
}

/// Executes the shared AI action without toolkit state.
pub async fn ai_inline_complete(
    state: &AppState,
    request_store: &AiRequestStore,
    secret: &SecretStoreState,
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

/// Executes the shared AI action without toolkit state.
pub async fn ai_inline_edit(
    state: &AppState,
    request_store: &AiRequestStore,
    secret: &SecretStoreState,
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

/// Executes the shared AI action without toolkit state.
pub async fn ai_commit_message(
    state: &AppState,
    request_store: &AiRequestStore,
    secret: &SecretStoreState,
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

/// Executes the shared AI action without toolkit state.
pub async fn ai_request_cancel(request_store: &AiRequestStore, owner: String, request_id: String) -> AppResult<()> {
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

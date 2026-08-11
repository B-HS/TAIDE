use std::collections::HashMap;

use parking_lot::Mutex;
use tauri::State;
use tokio::sync::oneshot;

use crate::domain::ai::prompt;
use crate::domain::ai::service;
use crate::domain::ai::types::{AiInlineCompleteRequest, AiInlineCompleteResponse, AiModelInfo, AiProviderId, AiTokenStatus};
use crate::error::{AppError, AppResult};
use crate::infra::http::create_outbound_http_client;
use crate::infra::secret::SecretStoreState;
use crate::state::AppState;

#[derive(Default)]
pub struct AiInlineStore(Mutex<HashMap<String, oneshot::Sender<()>>>);

impl AiInlineStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rejects a re-entrant `begin` for a `requestId` already in flight — mirrors
    /// `LspInstallStore::begin` (`domain/lsp/commands.rs`): a silent overwrite would leak the
    /// first request's cancel sender and let `finish` race-remove the second request's still-live
    /// entry.
    fn begin(&self, request_id: &str) -> Option<oneshot::Receiver<()>> {
        let mut store = self.0.lock();
        if store.contains_key(request_id) {
            return None;
        }
        let (tx, rx) = oneshot::channel();
        store.insert(request_id.to_string(), tx);
        Some(rx)
    }

    fn finish(&self, request_id: &str) {
        self.0.lock().remove(request_id);
    }

    fn cancel(&self, request_id: &str) {
        if let Some(tx) = self.0.lock().remove(request_id) {
            let _ = tx.send(());
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn ai_token_status(secret: State<'_, SecretStoreState>) -> AppResult<AiTokenStatus> {
    service::token_status(secret.0.as_ref())
}

#[tauri::command]
#[specta::specta]
pub async fn ai_set_token(secret: State<'_, SecretStoreState>, provider: AiProviderId, token: String) -> AppResult<()> {
    let client = create_outbound_http_client();
    service::set_token(secret.0.as_ref(), &client, provider, token).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_clear_token(secret: State<'_, SecretStoreState>, provider: AiProviderId) -> AppResult<()> {
    service::clear_token(secret.0.as_ref(), provider)
}

#[tauri::command]
#[specta::specta]
pub async fn ai_list_models(secret: State<'_, SecretStoreState>, provider: AiProviderId) -> AppResult<Vec<AiModelInfo>> {
    let client = create_outbound_http_client();
    service::list_models(secret.0.as_ref(), &client, provider).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_inline_complete(
    state: State<'_, AppState>,
    inline_store: State<'_, AiInlineStore>,
    secret: State<'_, SecretStoreState>,
    request: AiInlineCompleteRequest,
) -> AppResult<AiInlineCompleteResponse> {
    let Some(cancel_rx) = inline_store.begin(&request.request_id) else {
        return Err(AppError::InvalidArgument(format!(
            "an inline completion request with id '{}' is already in flight",
            request.request_id
        )));
    };

    let template = prompt::load_prompt_template(&state.paths);
    let client = create_outbound_http_client();

    let text = tokio::select! {
        result = service::complete(secret.0.as_ref(), &client, &request, &template) => result,
        _ = cancel_rx => Ok(None),
    };

    inline_store.finish(&request.request_id);

    Ok(AiInlineCompleteResponse {
        request_id: request.request_id,
        text: text?,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn ai_inline_cancel(inline_store: State<'_, AiInlineStore>, request_id: String) -> AppResult<()> {
    inline_store.cancel(&request_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 같은_request_id로_두번_시작하면_두번째는_거부된다() {
        let store = AiInlineStore::new();
        let _first = store.begin("req-1").expect("first begin");
        assert!(store.begin("req-1").is_none());
    }

    #[test]
    fn finish_후에는_같은_request_id를_다시_시작할_수_있다() {
        let store = AiInlineStore::new();
        let _first = store.begin("req-1").expect("first begin");
        store.finish("req-1");
        assert!(store.begin("req-1").is_some());
    }

    #[test]
    fn cancel_은_대기중인_receiver를_깨운다() {
        let store = AiInlineStore::new();
        let rx = store.begin("req-1").expect("begin");
        store.cancel("req-1");
        assert!(tauri::async_runtime::block_on(rx).is_ok());
    }

    #[test]
    fn 모르는_request_id를_취소해도_안전하다() {
        let store = AiInlineStore::new();
        store.cancel("unknown");
    }
}

use serde::{Deserialize, Serialize};

use crate::domain::ai::providers::codex::CodexProvider;
use crate::domain::ai::providers::ollama::OllamaCloudProvider;
use crate::domain::ai::providers::{mask_provider_error, AiProviderClient};
use crate::domain::ai::types::{AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate, AiProviderId, AiTokenStatus};
use crate::error::{AppError, AppResult};
use crate::infra::secret::{SecretAccount, SecretStore};

const CODEX_WHOAMI_URL: &str = "https://auth.openai.com/api/accounts/v1/user-auth-credential/whoami";

/// Codex needs both the access token and the `chatgpt-account-id` for every API call, so the
/// credential JSON-encodes both fields into the single `ai-codex` keyring entry rather than
/// adding a fourth `SecretAccount` variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexCredential {
    access_token: String,
    account_id: String,
}

#[derive(Debug, Deserialize)]
struct CodexWhoamiResponse {
    chatgpt_account_id: String,
}

fn provider_account(provider: AiProviderId) -> SecretAccount {
    match provider {
        AiProviderId::OllamaCloud => SecretAccount::AiOllamaCloud,
        AiProviderId::Codex => SecretAccount::AiCodex,
    }
}

pub fn token_status(secret: &dyn SecretStore) -> AppResult<AiTokenStatus> {
    Ok(AiTokenStatus {
        ollama_cloud: secret.get(SecretAccount::AiOllamaCloud)?.is_some(),
        codex: secret.get(SecretAccount::AiCodex)?.is_some(),
    })
}

async fn fetch_codex_account_id(client: &reqwest::Client, access_token: &str) -> AppResult<String> {
    let res = client
        .get(CODEX_WHOAMI_URL)
        .header("authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

    if !res.status().is_success() {
        return Err(AppError::InvalidArgument("Codex access token was rejected".to_string()));
    }

    let parsed: CodexWhoamiResponse = res
        .json()
        .await
        .map_err(|_| AppError::InvalidArgument("Codex whoami response was malformed".to_string()))?;
    Ok(parsed.chatgpt_account_id)
}

/// Stores a provider token. Codex is verified against `whoami` first (and its resolved
/// `accountId` bundled into the stored credential) — on verification failure nothing is written,
/// so a bad paste never leaves a half-configured provider behind.
pub async fn set_token(secret: &dyn SecretStore, client: &reqwest::Client, provider: AiProviderId, token: String) -> AppResult<()> {
    if token.trim().is_empty() {
        return Err(AppError::InvalidArgument("token must not be empty".to_string()));
    }

    match provider {
        AiProviderId::OllamaCloud => secret.set(SecretAccount::AiOllamaCloud, &token),
        AiProviderId::Codex => {
            let account_id = fetch_codex_account_id(client, &token).await?;
            let encoded = serde_json::to_string(&CodexCredential {
                access_token: token,
                account_id,
            })?;
            secret.set(SecretAccount::AiCodex, &encoded)
        }
    }
}

pub fn clear_token(secret: &dyn SecretStore, provider: AiProviderId) -> AppResult<()> {
    secret.delete(provider_account(provider))
}

fn load_codex_credential(secret: &dyn SecretStore) -> AppResult<CodexCredential> {
    let raw = secret
        .get(SecretAccount::AiCodex)?
        .ok_or_else(|| AppError::InvalidArgument("Codex access token is not configured".to_string()))?;
    serde_json::from_str(&raw).map_err(|_| AppError::Internal("stored Codex credential is malformed".to_string()))
}

fn load_ollama_api_key(secret: &dyn SecretStore) -> AppResult<String> {
    secret
        .get(SecretAccount::AiOllamaCloud)?
        .ok_or_else(|| AppError::InvalidArgument("Ollama Cloud API key is not configured".to_string()))
}

fn codex_provider(credential: CodexCredential) -> CodexProvider {
    CodexProvider {
        access_token: credential.access_token,
        account_id: credential.account_id,
    }
}

pub async fn list_models(secret: &dyn SecretStore, client: &reqwest::Client, provider: AiProviderId) -> AppResult<Vec<AiModelInfo>> {
    match provider {
        AiProviderId::OllamaCloud => {
            OllamaCloudProvider {
                api_key: load_ollama_api_key(secret)?,
            }
            .list_models(client)
            .await
        }
        AiProviderId::Codex => codex_provider(load_codex_credential(secret)?).list_models(client).await,
    }
}

pub async fn complete(
    secret: &dyn SecretStore,
    client: &reqwest::Client,
    request: &AiInlineCompleteRequest,
    template: &AiPromptTemplate,
) -> AppResult<Option<String>> {
    match request.provider {
        AiProviderId::OllamaCloud => {
            OllamaCloudProvider {
                api_key: load_ollama_api_key(secret)?,
            }
            .complete(client, request, template)
            .await
        }
        AiProviderId::Codex => {
            codex_provider(load_codex_credential(secret)?)
                .complete(client, request, template)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::secret::test_support::InMemorySecretStore;

    #[test]
    fn 토큰이_없으면_상태는_전부_false다() {
        let store = InMemorySecretStore::default();
        let status = token_status(&store).unwrap();
        assert!(!status.ollama_cloud);
        assert!(!status.codex);
    }

    #[test]
    fn 저장된_토큰만_상태에_true로_반영된다() {
        let store = InMemorySecretStore::default();
        store.set(SecretAccount::AiOllamaCloud, "ollama-key").unwrap();

        let status = token_status(&store).unwrap();

        assert!(status.ollama_cloud);
        assert!(!status.codex);
    }

    #[test]
    fn 빈_토큰은_저장을_거부한다() {
        let store = InMemorySecretStore::default();
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(set_token(&store, &client, AiProviderId::OllamaCloud, "   ".to_string()));

        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
        assert_eq!(store.get(SecretAccount::AiOllamaCloud).unwrap(), None);
    }

    #[test]
    fn ollama_토큰은_원문_그대로_저장된다() {
        let store = InMemorySecretStore::default();
        let client = reqwest::Client::new();

        tauri::async_runtime::block_on(set_token(&store, &client, AiProviderId::OllamaCloud, "raw-key".to_string())).unwrap();

        assert_eq!(store.get(SecretAccount::AiOllamaCloud).unwrap(), Some("raw-key".to_string()));
    }

    #[test]
    fn clear_token은_해당_provider_항목만_지운다() {
        let store = InMemorySecretStore::default();
        store.set(SecretAccount::AiOllamaCloud, "ollama-key").unwrap();
        store.set(SecretAccount::AiCodex, "{}").unwrap();

        clear_token(&store, AiProviderId::OllamaCloud).unwrap();

        assert_eq!(store.get(SecretAccount::AiOllamaCloud).unwrap(), None);
        assert!(store.get(SecretAccount::AiCodex).unwrap().is_some());
    }

    #[test]
    fn ollama_키가_없으면_모델_조회는_설정_필요_에러를_반환한다() {
        let store = InMemorySecretStore::default();
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(list_models(&store, &client, AiProviderId::OllamaCloud));

        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn codex_자격증명이_손상되면_내부_에러를_반환한다() {
        let store = InMemorySecretStore::default();
        store.set(SecretAccount::AiCodex, "not json").unwrap();
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(list_models(&store, &client, AiProviderId::Codex));

        assert!(matches!(result, Err(AppError::Internal(_))));
    }
}

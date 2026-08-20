use serde::{Deserialize, Serialize};

use crate::domain::ai::prompt;
use crate::domain::ai::providers::codex::CodexProvider;
use crate::domain::ai::providers::ollama::OllamaCloudProvider;
use crate::domain::ai::providers::omlx::OmlxProvider;
use crate::domain::ai::providers::AiProviderClient;
use crate::domain::ai::types::{
    AiCommitMessagePromptTemplate, AiCommitMessagePromptVars, AiCommitMessageRequest, AiInlineCompleteRequest, AiInlineEditPromptTemplate,
    AiInlineEditPromptVars, AiInlineEditRequest, AiModelInfo, AiPromptTemplate, AiProviderId, AiTokenStatus,
};
use crate::error::{AppError, AppResult};
use crate::infra::redact::mask_provider_error;
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
        AiProviderId::Omlx => SecretAccount::AiOmlx,
    }
}

/// OMLX has no fixed base URL (unlike Ollama Cloud/Codex's hardcoded hosts), so "configured" for
/// it means "the user has pointed it at a server" rather than "a token is stored" — an API key is
/// optional for oMLX. `omlx_base_url` is `Settings.ai_omlx_base_url`, read by the caller (a
/// keyring-only `SecretStore` has no visibility into app settings).
pub fn token_status(secret: &dyn SecretStore, omlx_base_url: Option<&str>) -> AppResult<AiTokenStatus> {
    Ok(AiTokenStatus {
        ollama_cloud: secret.get(SecretAccount::AiOllamaCloud)?.is_some(),
        codex: secret.get(SecretAccount::AiCodex)?.is_some(),
        omlx: omlx_base_url.is_some(),
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
/// so a bad paste never leaves a half-configured provider behind. OMLX's API key is optional and
/// unverified (the local server may be down when the key is saved — that's an allowed state, per
/// the "configured = base URL set" contract in `token_status`), so an empty OMLX token is treated
/// as "clear the stored key" rather than rejected outright like the other providers' required tokens.
pub async fn set_token(secret: &dyn SecretStore, client: &reqwest::Client, provider: AiProviderId, token: String) -> AppResult<()> {
    if provider == AiProviderId::Omlx && token.trim().is_empty() {
        return clear_token(secret, provider);
    }
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
        AiProviderId::Omlx => secret.set(SecretAccount::AiOmlx, &token),
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

/// `omlx_base_url` is `Settings.ai_omlx_base_url`, unused for the other two providers (they use a
/// hardcoded host). A `None` base URL for an `Omlx` request/list is a caller error — surfaced as
/// `AppError::InvalidArgument` rather than silently no-op'd — since the settings UI always writes
/// this field before enabling OMLX as the auto-tab provider.
fn omlx_provider(secret: &dyn SecretStore, base_url: Option<String>) -> AppResult<OmlxProvider> {
    let base_url = base_url.ok_or_else(|| AppError::InvalidArgument("OMLX base URL is not configured".to_string()))?;
    Ok(OmlxProvider {
        base_url,
        api_key: secret.get(SecretAccount::AiOmlx)?,
    })
}

/// The three provider clients [`AiProviderClient`] can't be `dyn`-dispatched over (its methods
/// return `impl Future`, which is not object-safe), so every call site that needs "the concrete
/// client for this `AiProviderId`" resolved credentials via its own copy of this exact match
/// (R6#13) — [`resolve_provider`] now does that once, and this enum's own [`AiProviderClient`]
/// impl below fans a call out to whichever variant it holds.
enum ResolvedAiProvider {
    OllamaCloud(OllamaCloudProvider),
    Codex(CodexProvider),
    Omlx(OmlxProvider),
}

fn resolve_provider(secret: &dyn SecretStore, provider: AiProviderId, omlx_base_url: Option<String>) -> AppResult<ResolvedAiProvider> {
    match provider {
        AiProviderId::OllamaCloud => Ok(ResolvedAiProvider::OllamaCloud(OllamaCloudProvider {
            api_key: load_ollama_api_key(secret)?,
        })),
        AiProviderId::Codex => Ok(ResolvedAiProvider::Codex(codex_provider(load_codex_credential(secret)?))),
        AiProviderId::Omlx => Ok(ResolvedAiProvider::Omlx(omlx_provider(secret, omlx_base_url)?)),
    }
}

impl AiProviderClient for ResolvedAiProvider {
    async fn list_models(&self, client: &reqwest::Client) -> AppResult<Vec<AiModelInfo>> {
        match self {
            ResolvedAiProvider::OllamaCloud(provider) => provider.list_models(client).await,
            ResolvedAiProvider::Codex(provider) => provider.list_models(client).await,
            ResolvedAiProvider::Omlx(provider) => provider.list_models(client).await,
        }
    }

    async fn complete(
        &self,
        client: &reqwest::Client,
        request: &AiInlineCompleteRequest,
        template: &AiPromptTemplate,
    ) -> AppResult<Option<String>> {
        match self {
            ResolvedAiProvider::OllamaCloud(provider) => provider.complete(client, request, template).await,
            ResolvedAiProvider::Codex(provider) => provider.complete(client, request, template).await,
            ResolvedAiProvider::Omlx(provider) => provider.complete(client, request, template).await,
        }
    }

    async fn instruct(&self, client: &reqwest::Client, model: &str, system: &str, user: &str) -> AppResult<Option<String>> {
        match self {
            ResolvedAiProvider::OllamaCloud(provider) => provider.instruct(client, model, system, user).await,
            ResolvedAiProvider::Codex(provider) => provider.instruct(client, model, system, user).await,
            ResolvedAiProvider::Omlx(provider) => provider.instruct(client, model, system, user).await,
        }
    }
}

pub async fn list_models(
    secret: &dyn SecretStore,
    client: &reqwest::Client,
    provider: AiProviderId,
    omlx_base_url: Option<String>,
) -> AppResult<Vec<AiModelInfo>> {
    resolve_provider(secret, provider, omlx_base_url)?.list_models(client).await
}

pub async fn complete(
    secret: &dyn SecretStore,
    client: &reqwest::Client,
    request: &AiInlineCompleteRequest,
    template: &AiPromptTemplate,
    omlx_base_url: Option<String>,
) -> AppResult<Option<String>> {
    resolve_provider(secret, request.provider, omlx_base_url)?
        .complete(client, request, template)
        .await
}

/// Resolves the provider/model an `ai_inline_edit`/`ai_commit_message` request should run
/// against: the request's own `provider`/`model` win when the caller supplied them, otherwise the
/// app's configured `Settings.ai_provider`/`ai_model` (the same fields auto-tab reads) act as the
/// default — so most call sites don't need to resolve "the configured AI provider" themselves,
/// only override it for a one-off request. Errors when neither the request nor settings name a
/// usable provider/model, rather than silently guessing one. `default_provider` is
/// `Option<AiProviderId>` (not `Option<String>`) since `Settings.ai_provider` was narrowed to that
/// same enum (T1-B) — the string-parsing/"unknown AI provider" fallback this used to need is gone
/// with it, since an invalid value can no longer reach here as a typed `Settings`.
pub fn resolve_provider_and_model(
    request_provider: Option<AiProviderId>,
    request_model: Option<String>,
    default_provider: Option<AiProviderId>,
    default_model: Option<String>,
) -> AppResult<(AiProviderId, String)> {
    let provider = request_provider
        .or(default_provider)
        .ok_or_else(|| AppError::InvalidArgument("AI provider is not configured".to_string()))?;
    let model = request_model
        .or(default_model)
        .ok_or_else(|| AppError::InvalidArgument("AI model is not configured".to_string()))?;
    Ok((provider, model))
}

async fn instruct(
    secret: &dyn SecretStore,
    client: &reqwest::Client,
    provider: AiProviderId,
    model: &str,
    system: &str,
    user: &str,
    omlx_base_url: Option<String>,
) -> AppResult<Option<String>> {
    resolve_provider(secret, provider, omlx_base_url)?
        .instruct(client, model, system, user)
        .await
}

pub async fn inline_edit(
    secret: &dyn SecretStore,
    client: &reqwest::Client,
    provider: AiProviderId,
    model: &str,
    request: &AiInlineEditRequest,
    template: &AiInlineEditPromptTemplate,
    omlx_base_url: Option<String>,
) -> AppResult<Option<String>> {
    let vars = AiInlineEditPromptVars {
        selection: &request.selection,
        instruction: &request.instruction,
        language: &request.language,
        file_path: &request.file_path,
        prefix: &request.prefix,
        suffix: &request.suffix,
    };
    let system = prompt::render_inline_edit(&template.system, &vars);
    let user = prompt::render_inline_edit(&template.user, &vars);
    instruct(secret, client, provider, model, &system, &user, omlx_base_url).await
}

pub async fn commit_message(
    secret: &dyn SecretStore,
    client: &reqwest::Client,
    provider: AiProviderId,
    model: &str,
    request: &AiCommitMessageRequest,
    template: &AiCommitMessagePromptTemplate,
    omlx_base_url: Option<String>,
) -> AppResult<Option<String>> {
    let vars = AiCommitMessagePromptVars {
        diff: &request.diff_text,
        recent_commits: &request.recent_commits,
    };
    let system = prompt::render_commit_message(&template.system, &vars);
    let user = prompt::render_commit_message(&template.user, &vars);
    instruct(secret, client, provider, model, &system, &user, omlx_base_url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::secret::test_support::InMemorySecretStore;

    #[test]
    fn 토큰이_없으면_상태는_전부_false다() {
        let store = InMemorySecretStore::default();
        let status = token_status(&store, None).unwrap();
        assert!(!status.ollama_cloud);
        assert!(!status.codex);
        assert!(!status.omlx);
    }

    #[test]
    fn 저장된_토큰만_상태에_true로_반영된다() {
        let store = InMemorySecretStore::default();
        store.set(SecretAccount::AiOllamaCloud, "ollama-key").unwrap();

        let status = token_status(&store, None).unwrap();

        assert!(status.ollama_cloud);
        assert!(!status.codex);
        assert!(!status.omlx);
    }

    #[test]
    fn omlx는_api_키_없이도_base_url만_설정되면_상태가_true다() {
        let store = InMemorySecretStore::default();

        let status = token_status(&store, Some("http://localhost:8000")).unwrap();

        assert!(status.omlx);
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
    fn omlx_토큰은_검증없이_원문_그대로_저장된다() {
        let store = InMemorySecretStore::default();
        let client = reqwest::Client::new();

        tauri::async_runtime::block_on(set_token(&store, &client, AiProviderId::Omlx, "omlx-key".to_string())).unwrap();

        assert_eq!(store.get(SecretAccount::AiOmlx).unwrap(), Some("omlx-key".to_string()));
    }

    #[test]
    fn omlx는_빈_토큰_입력을_해제로_간주한다() {
        let store = InMemorySecretStore::default();
        store.set(SecretAccount::AiOmlx, "existing-key").unwrap();
        let client = reqwest::Client::new();

        tauri::async_runtime::block_on(set_token(&store, &client, AiProviderId::Omlx, "   ".to_string())).unwrap();

        assert_eq!(store.get(SecretAccount::AiOmlx).unwrap(), None);
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

        let result = tauri::async_runtime::block_on(list_models(&store, &client, AiProviderId::OllamaCloud, None));

        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn codex_자격증명이_손상되면_내부_에러를_반환한다() {
        let store = InMemorySecretStore::default();
        store.set(SecretAccount::AiCodex, "not json").unwrap();
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(list_models(&store, &client, AiProviderId::Codex, None));

        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    #[test]
    fn omlx는_base_url이_없으면_모델_조회가_설정_필요_에러를_반환한다() {
        let store = InMemorySecretStore::default();
        let client = reqwest::Client::new();

        let result = tauri::async_runtime::block_on(list_models(&store, &client, AiProviderId::Omlx, None));

        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn 요청에_provider_모델이_있으면_설정값보다_우선한다() {
        let (provider, model) = resolve_provider_and_model(
            Some(AiProviderId::Codex),
            Some("gpt-5.6-sol".to_string()),
            Some(AiProviderId::OllamaCloud),
            Some("qwen2.5-coder".to_string()),
        )
        .unwrap();

        assert_eq!(provider, AiProviderId::Codex);
        assert_eq!(model, "gpt-5.6-sol");
    }

    #[test]
    fn 요청에_provider_모델이_없으면_설정값으로_폴백한다() {
        let (provider, model) =
            resolve_provider_and_model(None, None, Some(AiProviderId::Omlx), Some("qwen2.5-coder".to_string())).unwrap();

        assert_eq!(provider, AiProviderId::Omlx);
        assert_eq!(model, "qwen2.5-coder");
    }

    #[test]
    fn provider가_요청과_설정_양쪽에_없으면_에러를_반환한다() {
        let result = resolve_provider_and_model(None, Some("qwen2.5-coder".to_string()), None, None);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }

    #[test]
    fn 모델이_요청과_설정_양쪽에_없으면_에러를_반환한다() {
        let result = resolve_provider_and_model(Some(AiProviderId::Codex), None, None, None);
        assert!(matches!(result, Err(AppError::InvalidArgument(_))));
    }
}

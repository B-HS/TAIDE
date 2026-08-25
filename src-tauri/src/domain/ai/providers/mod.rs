pub mod codex;
pub mod ollama;
pub mod omlx;

use crate::domain::ai::types::{AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::redact::mask_provider_error;

pub trait AiProviderClient {
    fn list_models(&self, client: &reqwest::Client) -> impl std::future::Future<Output = AppResult<Vec<AiModelInfo>>> + Send;
    fn complete(
        &self,
        client: &reqwest::Client,
        request: &AiInlineCompleteRequest,
        template: &AiPromptTemplate,
    ) -> impl std::future::Future<Output = AppResult<Option<String>>> + Send;
    /// General-purpose system+user chat completion, already rendered by the caller — the
    /// entry point Inline Edit and AI commit messages call, as opposed to [`Self::complete`]'s
    /// FIM-first/chat-fallback auto-tab contract. Every provider already had this exact chat
    /// mechanics as its `complete` fallback path (or, for Codex, as `complete` itself); this
    /// method reuses that same request-building/parsing code, just with pre-rendered strings
    /// instead of a template.
    fn instruct(
        &self,
        client: &reqwest::Client,
        model: &str,
        system: &str,
        user: &str,
    ) -> impl std::future::Future<Output = AppResult<Option<String>>> + Send;
}

/// Classifies a non-2xx provider response into a locale key, shared by all three providers'
/// `list_models`/`complete`/`instruct` paths (T2-J 축2 — see
/// `docs/acknowledge/2026-08-24-d34-apperror-campaign-contract.md` §3.2). `body` is masked once
/// here rather than by each call site.
pub(crate) fn provider_http_error(provider: &str, status: reqwest::StatusCode, body: &str) -> AppError {
    let detail = mask_provider_error(body);
    let status_code = status.as_u16();
    match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => AppError::localized(
            AppErrorKind::Internal,
            "error.ai.unauthorized",
            format!("{provider}: request was unauthorized ({status_code})"),
        )
        .with_arg("provider", provider)
        .with_arg("status", status_code)
        .with_arg("detail", &detail),
        reqwest::StatusCode::NOT_FOUND => AppError::localized(
            AppErrorKind::Internal,
            "error.ai.endpointNotFound",
            format!("{provider}: endpoint not found ({status_code})"),
        )
        .with_arg("provider", provider)
        .with_arg("status", status_code)
        .with_arg("detail", &detail),
        reqwest::StatusCode::REQUEST_TIMEOUT | reqwest::StatusCode::GATEWAY_TIMEOUT => {
            AppError::localized(AppErrorKind::Internal, "error.ai.timeout", format!("{provider}: request timed out"))
                .with_arg("provider", provider)
                .with_arg("detail", &detail)
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => AppError::localized(
            AppErrorKind::Internal,
            "error.ai.rateLimited",
            format!("{provider}: rate limit exceeded"),
        )
        .with_arg("provider", provider)
        .with_arg("detail", &detail),
        s if s.is_server_error() => AppError::localized(
            AppErrorKind::Internal,
            "error.ai.providerUnavailable",
            format!("{provider} is temporarily unavailable ({status_code})"),
        )
        .with_arg("provider", provider)
        .with_arg("status", status_code)
        .with_arg("detail", &detail),
        _ => AppError::localized(
            AppErrorKind::Internal,
            "error.ai.requestFailed",
            format!("{provider} request failed ({status_code}): {detail}"),
        )
        .with_arg("provider", provider)
        .with_arg("status", status_code)
        .with_arg("detail", &detail),
    }
}

/// Classifies a `send()`/`.json()` transport failure into a locale key — the counterpart to
/// [`provider_http_error`] for errors that never produced an HTTP response at all.
pub(crate) fn provider_transport_error(provider: &str, error: &reqwest::Error) -> AppError {
    let detail = mask_provider_error(&error.to_string());
    if error.is_timeout() {
        return AppError::localized(AppErrorKind::Internal, "error.ai.timeout", format!("{provider}: request timed out"))
            .with_arg("provider", provider)
            .with_arg("detail", &detail);
    }
    AppError::localized(
        AppErrorKind::Internal,
        "error.ai.networkFailed",
        format!("{provider}: network error: {detail}"),
    )
    .with_arg("provider", provider)
    .with_arg("detail", &detail)
}

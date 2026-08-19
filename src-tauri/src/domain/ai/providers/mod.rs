pub mod codex;
pub mod ollama;
pub mod omlx;

use crate::domain::ai::types::{AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate};
use crate::error::AppResult;

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

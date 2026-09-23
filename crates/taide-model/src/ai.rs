use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AiProviderId {
    OllamaCloud,
    Codex,
    Omlx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiTokenStatus {
    pub ollama_cloud: bool,
    pub codex: bool,
    pub omlx: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub model_id: String,
    pub display_name: Option<String>,
}

/// `owner` (`getCurrentWindow().label` on the frontend — `main`/`editor-<n>`, or the remote client's
/// fixed `domain::remote::types::REMOTE_OWNER_LABEL`) combines with `request_id` to key
/// `AiRequestStore` (R6#20) — a caller-supplied `request_id` alone is shared global state, so two
/// windows that happen to generate the same id (or a remote session replaying one) would otherwise
/// collide: the second `begin()` would be rejected as "already in flight", or a `ai_request_cancel`
/// from one window could cancel a same-id request actually in flight in another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiInlineCompleteRequest {
    pub request_id: String,
    pub owner: String,
    pub provider: AiProviderId,
    pub model: String,
    pub prefix: String,
    pub suffix: String,
    pub language: String,
    pub file_path: String,
}

/// Shared response shape for every AI text-generation command — auto-tab inline completion
/// (`ai_inline_complete`), Inline Edit (`ai_inline_edit`), and AI commit messages
/// (`ai_commit_message`) all resolve to the same `{requestId, text}` pair (the request that was
/// answered, and the generated text — `null` when nothing was produced: the request was
/// cancelled, or the provider returned no usable text (each provider normalizes an empty/
/// whitespace-only response to `None`, e.g. `providers::ollama::extract_chat_text`)). Replaces
/// three structurally identical structs (`AiInlineCompleteResponse`/`AiInlineEditResponse`/
/// `AiCommitMessageResponse`) that only differed by name — see
/// `docs/acknowledge/2026-08-25-d37-ai-batch-contract.md` §3 for why a single type here is safe
/// (the wire shape was already identical, so this only changes the TS type name, not any runtime
/// behavior).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiTextResponse {
    pub request_id: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiPromptTemplate {
    pub version: u32,
    pub fim: AiFimPromptTemplate,
    pub chat: AiChatPromptTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiFimPromptTemplate {
    pub prompt: String,
    pub suffix: String,
    #[serde(default)]
    pub stop: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiChatPromptTemplate {
    pub system: String,
    pub user: String,
}

pub struct AiPromptVars<'a> {
    pub prefix: &'a str,
    pub suffix: &'a str,
    pub language: &'a str,
    pub file_path: &'a str,
}

/// Dedicated prompt template for the Inline Edit feature (`ai_inline_edit`) — kept separate from
/// [`AiPromptTemplate`] (auto-tab) even though both currently carry a `{system, user}` chat shape,
/// so a user override at `{app_data}/prompts/inline-edit-default.json` can never collide with (or
/// be silently reset by) the auto-tab template file. See `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §2-3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiInlineEditPromptTemplate {
    pub version: u32,
    pub system: String,
    pub user: String,
}

pub struct AiInlineEditPromptVars<'a> {
    pub selection: &'a str,
    pub instruction: &'a str,
    pub language: &'a str,
    pub file_path: &'a str,
    pub prefix: &'a str,
    pub suffix: &'a str,
}

/// Dedicated prompt template for AI commit message generation (`ai_commit_message`) — see
/// [`AiInlineEditPromptTemplate`]'s doc comment for why this is a separate type/file rather than a
/// new section bolted onto an existing template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiCommitMessagePromptTemplate {
    pub version: u32,
    pub system: String,
    pub user: String,
}

pub struct AiCommitMessagePromptVars<'a> {
    pub diff: &'a str,
    pub recent_commits: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiInlineEditRequest {
    pub request_id: String,
    /// See [`AiInlineCompleteRequest::owner`]'s doc comment.
    pub owner: String,
    #[serde(default)]
    pub provider: Option<AiProviderId>,
    #[serde(default)]
    pub model: Option<String>,
    pub selection: String,
    pub instruction: String,
    pub language: String,
    pub file_path: String,
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiCommitMessageRequest {
    pub request_id: String,
    /// See [`AiInlineCompleteRequest::owner`]'s doc comment.
    pub owner: String,
    #[serde(default)]
    pub provider: Option<AiProviderId>,
    #[serde(default)]
    pub model: Option<String>,
    pub diff_text: String,
    pub recent_commits: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_아이디는_카멜케이스로_직렬화된다() {
        assert_eq!(serde_json::to_string(&AiProviderId::OllamaCloud).unwrap(), "\"ollamaCloud\"");
        assert_eq!(serde_json::to_string(&AiProviderId::Codex).unwrap(), "\"codex\"");
        assert_eq!(serde_json::to_string(&AiProviderId::Omlx).unwrap(), "\"omlx\"");
    }

    #[test]
    fn 토큰_상태는_provider_아이디와_같은_키로_직렬화된다() {
        let status = AiTokenStatus {
            ollama_cloud: true,
            codex: false,
            omlx: true,
        };
        assert_eq!(
            serde_json::to_string(&status).unwrap(),
            r#"{"ollamaCloud":true,"codex":false,"omlx":true}"#
        );
    }

    #[test]
    fn ai_텍스트_응답은_request_id와_text를_카멜케이스로_직렬화한다() {
        let response = AiTextResponse {
            request_id: "req-1".to_string(),
            text: Some("fn main() {}".to_string()),
        };
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"requestId":"req-1","text":"fn main() {}"}"#
        );

        let cancelled_or_empty = AiTextResponse {
            request_id: "req-1".to_string(),
            text: None,
        };
        assert_eq!(
            serde_json::to_string(&cancelled_or_empty).unwrap(),
            r#"{"requestId":"req-1","text":null}"#
        );
    }
}

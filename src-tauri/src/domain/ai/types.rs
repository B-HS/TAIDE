use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AiProviderId {
    OllamaCloud,
    Codex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiTokenStatus {
    pub ollama_cloud: bool,
    pub codex: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub model_id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiInlineCompleteRequest {
    pub request_id: String,
    pub provider: AiProviderId,
    pub model: String,
    pub prefix: String,
    pub suffix: String,
    pub language: String,
    pub file_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AiInlineCompleteResponse {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_아이디는_카멜케이스로_직렬화된다() {
        assert_eq!(serde_json::to_string(&AiProviderId::OllamaCloud).unwrap(), "\"ollamaCloud\"");
        assert_eq!(serde_json::to_string(&AiProviderId::Codex).unwrap(), "\"codex\"");
    }

    #[test]
    fn 토큰_상태는_provider_아이디와_같은_키로_직렬화된다() {
        let status = AiTokenStatus {
            ollama_cloud: true,
            codex: false,
        };
        assert_eq!(serde_json::to_string(&status).unwrap(), r#"{"ollamaCloud":true,"codex":false}"#);
    }
}

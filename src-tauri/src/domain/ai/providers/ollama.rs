use serde::Deserialize;

use crate::domain::ai::prompt;
use crate::domain::ai::providers::AiProviderClient;
use crate::domain::ai::types::{
    AiChatPromptTemplate, AiFimPromptTemplate, AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate, AiPromptVars,
};
use crate::error::{AppError, AppResult};
use crate::infra::redact::mask_provider_error;

const OLLAMA_BASE: &str = "https://ollama.com/api";
/// Output budget for the auto-tab ghost-text path (`complete_chat`) — a single-line/few-line
/// suggestion never needs more, and this value must not change without re-validating auto-tab
/// latency (contract §3.1 "auto-tab 회귀 0").
const OLLAMA_NUM_PREDICT: u32 = 256;
/// Output budget for [`AiProviderClient::instruct`] (Inline Edit selection replacement, AI commit
/// messages) — a whole-selection replacement is routinely tens of lines, far beyond
/// [`OLLAMA_NUM_PREDICT`]'s ghost-text budget. Kept separate so raising it can never regress
/// auto-tab latency/cost.
const OLLAMA_INSTRUCT_NUM_PREDICT: u32 = 4_096;

pub struct OllamaCloudProvider {
    pub api_key: String,
}

impl OllamaCloudProvider {
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    #[serde(default)]
    models: Vec<OllamaTagModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagModel {
    name: String,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OllamaGenerateResponse {
    #[serde(default)]
    response: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    #[serde(default)]
    message: Option<OllamaChatMessage>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    done_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatMessage {
    #[serde(default)]
    content: String,
}

fn build_generate_body(model: &str, prompt: &str, suffix: &str, stop: &[String]) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
        "suffix": suffix,
        "stream": false,
        "options": { "num_predict": OLLAMA_NUM_PREDICT, "stop": stop },
    })
}

fn build_chat_body(model: &str, system: &str, user: &str, num_predict: u32) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        "stream": false,
        "options": { "num_predict": num_predict },
    })
}

fn is_fim_response_usable(parsed: &OllamaGenerateResponse) -> bool {
    parsed.error.is_none() && !parsed.response.trim().is_empty()
}

impl AiProviderClient for OllamaCloudProvider {
    async fn instruct(&self, client: &reqwest::Client, model: &str, system: &str, user: &str) -> AppResult<Option<String>> {
        self.send_chat_request(client, model, system, user, OLLAMA_INSTRUCT_NUM_PREDICT, true)
            .await
    }

    async fn list_models(&self, client: &reqwest::Client) -> AppResult<Vec<AiModelInfo>> {
        let res = client
            .get(format!("{OLLAMA_BASE}/tags"))
            .header("authorization", self.auth_header())
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "ollama tags request failed ({status}): {}",
                mask_provider_error(&body)
            )));
        }

        let parsed: OllamaTagsResponse = res
            .json()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;
        Ok(parsed
            .models
            .into_iter()
            .map(|model| AiModelInfo {
                model_id: model.name,
                display_name: model.model,
            })
            .collect())
    }

    async fn complete(
        &self,
        client: &reqwest::Client,
        request: &AiInlineCompleteRequest,
        template: &AiPromptTemplate,
    ) -> AppResult<Option<String>> {
        let vars = AiPromptVars {
            prefix: &request.prefix,
            suffix: &request.suffix,
            language: &request.language,
            file_path: &request.file_path,
        };

        if let Some(text) = self.complete_fim(client, request, &template.fim, &vars).await {
            return Ok(Some(text));
        }

        self.complete_chat(client, request, &template.chat, &vars).await
    }
}

impl OllamaCloudProvider {
    /// Attempts the FIM (`/api/generate` + `suffix`) path first. Any failure — network error,
    /// non-2xx status, unparseable body, or a body that signals the server doesn't support the
    /// `suffix` field (empty `response`/`error` set) — is treated as "not usable" so the caller
    /// falls back to the chat completion path, per the provider's documented FIM-first/chat-fallback
    /// contract. Only the chat path's failures are surfaced to the user.
    async fn complete_fim(
        &self,
        client: &reqwest::Client,
        request: &AiInlineCompleteRequest,
        fim: &AiFimPromptTemplate,
        vars: &AiPromptVars<'_>,
    ) -> Option<String> {
        let rendered_prompt = prompt::render(&fim.prompt, vars);
        let rendered_suffix = prompt::render(&fim.suffix, vars);
        let body = build_generate_body(&request.model, &rendered_prompt, &rendered_suffix, &fim.stop);

        let res = client
            .post(format!("{OLLAMA_BASE}/generate"))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await
            .ok()?;

        if !res.status().is_success() {
            return None;
        }

        let parsed: OllamaGenerateResponse = res.json().await.ok()?;
        is_fim_response_usable(&parsed).then_some(parsed.response)
    }

    async fn complete_chat(
        &self,
        client: &reqwest::Client,
        request: &AiInlineCompleteRequest,
        chat: &AiChatPromptTemplate,
        vars: &AiPromptVars<'_>,
    ) -> AppResult<Option<String>> {
        let system = prompt::render(&chat.system, vars);
        let user = prompt::render(&chat.user, vars);
        self.send_chat_request(client, &request.model, &system, &user, OLLAMA_NUM_PREDICT, false)
            .await
    }

    /// The `/api/chat` request/response mechanics shared by the auto-tab chat-fallback path
    /// ([`Self::complete_chat`], which renders a template first, ghost-text budget, truncation
    /// tolerated) and [`AiProviderClient::instruct`] (already-rendered strings, larger
    /// selection-replacement budget). `fail_on_truncation` turns a `done_reason: "length"` response
    /// into an error instead of silently returning the cut-off text — appropriate for `instruct`
    /// (a truncated code replacement would otherwise look like a valid, complete suggestion) but
    /// not for auto-tab ghost text, where a short completion hitting the budget is an expected,
    /// harmless outcome.
    async fn send_chat_request(
        &self,
        client: &reqwest::Client,
        model: &str,
        system: &str,
        user: &str,
        num_predict: u32,
        fail_on_truncation: bool,
    ) -> AppResult<Option<String>> {
        let body = build_chat_body(model, system, user, num_predict);

        let res = client
            .post(format!("{OLLAMA_BASE}/chat"))
            .header("authorization", self.auth_header())
            .json(&body)
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "ollama chat request failed ({status}): {}",
                mask_provider_error(&text)
            )));
        }

        let parsed: OllamaChatResponse = res
            .json()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;
        extract_chat_text(parsed, fail_on_truncation)
    }
}

/// Turns a parsed `/api/chat` response into the caller's `Option<String>` result — pulled out of
/// [`OllamaCloudProvider::send_chat_request`] so the truncation/error handling can be unit tested
/// without a live HTTP round trip. A `done_reason: "length"` response only becomes an error when
/// `fail_on_truncation` is set (see [`OllamaCloudProvider::send_chat_request`]'s doc comment).
fn extract_chat_text(parsed: OllamaChatResponse, fail_on_truncation: bool) -> AppResult<Option<String>> {
    if let Some(error) = parsed.error {
        return Err(AppError::Internal(mask_provider_error(&error)));
    }
    if fail_on_truncation && parsed.done_reason.as_deref() == Some("length") {
        return Err(AppError::Internal(
            "ollama response was truncated at the output token limit".to_string(),
        ));
    }

    Ok(parsed
        .message
        .map(|message| message.content)
        .filter(|content| !content.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_요청_바디는_prompt_suffix_stop_옵션을_포함한다() {
        let body = build_generate_body("qwen2.5-coder", "fn main() {", "\n}", &["\n\n\n".to_string()]);

        assert_eq!(body["model"], "qwen2.5-coder");
        assert_eq!(body["prompt"], "fn main() {");
        assert_eq!(body["suffix"], "\n}");
        assert_eq!(body["stream"], false);
        assert_eq!(body["options"]["num_predict"], OLLAMA_NUM_PREDICT);
        assert_eq!(body["options"]["stop"][0], "\n\n\n");
    }

    #[test]
    fn chat_요청_바디는_system_user_메시지_순서를_지킨다() {
        let body = build_chat_body("qwen2.5-coder", "system prompt", "user prompt", OLLAMA_NUM_PREDICT);

        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "system prompt");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "user prompt");
        assert_eq!(body["options"]["num_predict"], OLLAMA_NUM_PREDICT);
    }

    #[test]
    fn instruct_요청_바디는_auto_tab보다_큰_토큰_예산을_사용한다() {
        let body = build_chat_body("qwen2.5-coder", "system prompt", "user prompt", OLLAMA_INSTRUCT_NUM_PREDICT);

        assert_eq!(body["options"]["num_predict"], OLLAMA_INSTRUCT_NUM_PREDICT);
    }

    #[test]
    fn error_필드가_있으면_fim_응답을_사용할_수_없다() {
        let parsed = OllamaGenerateResponse {
            response: "ignored".to_string(),
            error: Some("suffix not supported".to_string()),
        };
        assert!(!is_fim_response_usable(&parsed));
    }

    #[test]
    fn 응답이_비어있으면_fim_응답을_사용할_수_없다() {
        let parsed = OllamaGenerateResponse {
            response: "   ".to_string(),
            error: None,
        };
        assert!(!is_fim_response_usable(&parsed));
    }

    #[test]
    fn 정상_응답은_fim_응답으로_사용할_수_있다() {
        let parsed = OllamaGenerateResponse {
            response: "    return 1;".to_string(),
            error: None,
        };
        assert!(is_fim_response_usable(&parsed));
    }

    #[test]
    fn instruct_경로에서_길이_제한으로_잘린_응답은_에러로_처리된다() {
        let parsed = OllamaChatResponse {
            message: Some(OllamaChatMessage {
                content: "half a function".to_string(),
            }),
            error: None,
            done_reason: Some("length".to_string()),
        };

        let result = extract_chat_text(parsed, true);

        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    #[test]
    fn auto_tab_경로에서_길이_제한으로_잘린_응답은_그대로_반환된다() {
        let parsed = OllamaChatResponse {
            message: Some(OllamaChatMessage {
                content: "short suggestion".to_string(),
            }),
            error: None,
            done_reason: Some("length".to_string()),
        };

        let result = extract_chat_text(parsed, false).unwrap();

        assert_eq!(result, Some("short suggestion".to_string()));
    }

    #[test]
    fn done_reason이_stop이면_잘리지_않은_것으로_처리된다() {
        let parsed = OllamaChatResponse {
            message: Some(OllamaChatMessage {
                content: "complete response".to_string(),
            }),
            error: None,
            done_reason: Some("stop".to_string()),
        };

        let result = extract_chat_text(parsed, true).unwrap();

        assert_eq!(result, Some("complete response".to_string()));
    }
}

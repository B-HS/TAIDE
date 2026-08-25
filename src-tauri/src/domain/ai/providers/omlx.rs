use serde::Deserialize;

use crate::domain::ai::prompt;
use crate::domain::ai::providers::{provider_http_error, provider_transport_error, AiProviderClient};
use crate::domain::ai::types::{
    AiChatPromptTemplate, AiFimPromptTemplate, AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate, AiPromptVars,
};
use crate::error::{AppError, AppResult};

const OMLX_PROVIDER_NAME: &str = "omlx";

/// Output budget for the auto-tab ghost-text path (`complete_chat`) — see
/// `OLLAMA_NUM_PREDICT`'s doc comment in `providers/ollama.rs` (same reasoning applies here).
const OMLX_MAX_TOKENS: u32 = 256;
/// Output budget for [`AiProviderClient::instruct`] (Inline Edit selection replacement, AI commit
/// messages) — see `OLLAMA_INSTRUCT_NUM_PREDICT`'s doc comment in `providers/ollama.rs`.
const OMLX_INSTRUCT_MAX_TOKENS: u32 = 4_096;

/// Fill-in-the-middle sentinel families, keyed by a case-insensitive substring match against the
/// model id (oMLX exposes the model *directory name* as `id` — see `providers/mod.rs` doc on
/// `AiProviderClient::list_models`). Sentinel strings and stop-token sets were re-verified against
/// each family's official model card / tokenizer config and cross-checked with Continue.dev's
/// production autocomplete templates (`core/autocomplete/templating/AutocompleteTemplate.ts`):
///   - Qwen(-Coder): https://huggingface.co/Qwen/Qwen2.5-Coder-7B (FIM special tokens section)
///   - CodeGemma: https://huggingface.co/google/codegemma-7b (README "Formatting" section)
///   - DeepSeek-Coder: https://deepwiki.com/deepseek-ai/DeepSeek-Coder/3.2-code-insertion
///   - StarCoder2: https://huggingface.co/bigcode/starcoder2-15b/discussions/6
///   - CodeLlama: https://huggingface.co/docs/transformers/en/model_doc/code_llama (infilling)
///   - Codestral: https://huggingface.co/mistralai/Codestral-22B-v0.1/discussions/10
///   - Continue.dev (all six, verified verbatim):
///     https://github.com/continuedev/continue/blob/main/core/autocomplete/templating/AutocompleteTemplate.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FimFamily {
    Qwen,
    CodeGemma,
    DeepSeek,
    StarCoder,
    CodeLlama,
    Codestral,
}

const FIM_FAMILY_NEEDLES: &[(&str, FimFamily)] = &[
    ("codegemma", FimFamily::CodeGemma),
    ("qwen", FimFamily::Qwen),
    ("deepseek", FimFamily::DeepSeek),
    ("starcoder", FimFamily::StarCoder),
    ("codellama", FimFamily::CodeLlama),
    ("codestral", FimFamily::Codestral),
];

/// `None` means the model id matched no known FIM family — the caller skips the FIM attempt
/// entirely and goes straight to the chat-completion fallback, per the Ollama provider's
/// "unknown model = no FIM" precedent.
fn detect_fim_family(model_id: &str) -> Option<FimFamily> {
    let lower = model_id.to_lowercase();
    FIM_FAMILY_NEEDLES
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map(|(_, family)| *family)
}

/// Assembles the single `prompt` field sent to `/v1/completions` from the already-rendered
/// prefix/suffix text (rendered via `prompt::render` so user template overrides still apply).
/// Sentinel placement (including CodeLlama's asymmetric spacing) matches Continue.dev's
/// production templates verbatim — see the `FimFamily` doc comment for sources.
fn assemble_fim_prompt(family: FimFamily, prefix: &str, suffix: &str) -> String {
    match family {
        FimFamily::Qwen | FimFamily::CodeGemma => format!("<|fim_prefix|>{prefix}<|fim_suffix|>{suffix}<|fim_middle|>"),
        FimFamily::DeepSeek => format!("<｜fim▁begin｜>{prefix}<｜fim▁hole｜>{suffix}<｜fim▁end｜>"),
        FimFamily::StarCoder => format!("<fim_prefix>{prefix}<fim_suffix>{suffix}<fim_middle>"),
        FimFamily::CodeLlama => format!("<PRE> {prefix} <SUF>{suffix} <MID>"),
        FimFamily::Codestral => format!("[SUFFIX]{suffix}[PREFIX]{prefix}"),
    }
}

/// Per-family stop tokens, cross-checked against Continue.dev's production templates. DeepSeek's
/// upstream template also stops on a bare `//` (to cut runaway comment generation) — that is
/// deliberately omitted here because it would also truncate any legitimate completion containing
/// `//` (a URL literal, a Rust/JS/C line comment mid-snippet), which is a correctness regression
/// this provider is not willing to trade for DeepSeek's comment-runaway mitigation.
fn family_stop_tokens(family: FimFamily) -> Vec<String> {
    let tokens: &[&str] = match family {
        FimFamily::Qwen => &[
            "<|endoftext|>",
            "<|fim_prefix|>",
            "<|fim_middle|>",
            "<|fim_suffix|>",
            "<|fim_pad|>",
            "<|repo_name|>",
            "<|file_sep|>",
            "<|im_start|>",
            "<|im_end|>",
        ],
        FimFamily::CodeGemma => &[
            "<|fim_prefix|>",
            "<|fim_suffix|>",
            "<|fim_middle|>",
            "<|file_separator|>",
            "<end_of_turn>",
            "<eos>",
        ],
        FimFamily::DeepSeek => &["<｜fim▁begin｜>", "<｜fim▁hole｜>", "<｜fim▁end｜>", "<｜end▁of▁sentence｜>"],
        FimFamily::StarCoder => &["<fim_prefix>", "<fim_suffix>", "<fim_middle>", "<file_sep>", "<|endoftext|>"],
        FimFamily::CodeLlama => &["<PRE>", "<SUF>", "<MID>", "<EOT>"],
        FimFamily::Codestral => &["[PREFIX]", "[SUFFIX]"],
    };
    tokens.iter().map(|token| token.to_string()).collect()
}

pub struct OmlxProvider {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl OmlxProvider {
    /// API key is optional for oMLX (server may run with `--api-key` unset) — the header is only
    /// attached when a non-empty key is configured, matching the server's `HTTPBearer(auto_error=false)`
    /// contract (a missing/empty header is a valid, anonymous request).
    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.api_key.as_deref().map(str::trim) {
            Some(key) if !key.is_empty() => builder.header("authorization", format!("Bearer {key}")),
            _ => builder,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct OmlxModelsResponse {
    #[serde(default)]
    data: Vec<OmlxModel>,
}

#[derive(Debug, Deserialize)]
struct OmlxModel {
    id: String,
}

#[derive(Debug, Default, Deserialize)]
struct OmlxCompletionResponse {
    #[serde(default)]
    choices: Vec<OmlxCompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct OmlxCompletionChoice {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct OmlxChatResponse {
    #[serde(default)]
    choices: Vec<OmlxChatChoice>,
}

#[derive(Debug, Deserialize)]
struct OmlxChatChoice {
    #[serde(default)]
    message: Option<OmlxChatMessage>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OmlxChatMessage {
    #[serde(default)]
    content: String,
}

fn build_completions_body(model: &str, prompt: &str, stop: &[String]) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "max_tokens": OMLX_MAX_TOKENS,
        "stop": stop,
    })
}

fn build_chat_body(model: &str, system: &str, user: &str, max_tokens: u32) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
        "stream": false,
        "max_tokens": max_tokens,
    })
}

impl AiProviderClient for OmlxProvider {
    async fn instruct(&self, client: &reqwest::Client, model: &str, system: &str, user: &str) -> AppResult<Option<String>> {
        self.send_chat_request(client, model, system, user, OMLX_INSTRUCT_MAX_TOKENS, true)
            .await
    }

    async fn list_models(&self, client: &reqwest::Client) -> AppResult<Vec<AiModelInfo>> {
        let res = self
            .apply_auth(client.get(format!("{}/v1/models", self.base_url)))
            .send()
            .await
            .map_err(|error| provider_transport_error(OMLX_PROVIDER_NAME, &error))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(provider_http_error(OMLX_PROVIDER_NAME, status, &body));
        }

        let parsed: OmlxModelsResponse = res
            .json()
            .await
            .map_err(|error| provider_transport_error(OMLX_PROVIDER_NAME, &error))?;
        Ok(parsed
            .data
            .into_iter()
            .map(|model| AiModelInfo {
                model_id: model.id,
                display_name: None,
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

impl OmlxProvider {
    /// Attempts the FIM (`/v1/completions` with a model-family sentinel prompt) path first. `None`
    /// is returned — meaning "fall back to chat" — for an unrecognized model family (no sentinel
    /// table entry), a network error, non-2xx status, unparseable body, or an empty completion.
    /// Only the chat path's failures are surfaced to the caller, mirroring the Ollama Cloud
    /// provider's FIM-first/chat-fallback contract.
    async fn complete_fim(
        &self,
        client: &reqwest::Client,
        request: &AiInlineCompleteRequest,
        fim: &AiFimPromptTemplate,
        vars: &AiPromptVars<'_>,
    ) -> Option<String> {
        let family = detect_fim_family(&request.model)?;
        let rendered_prefix = prompt::render(&fim.prompt, vars);
        let rendered_suffix = prompt::render(&fim.suffix, vars);
        let sentinel_prompt = assemble_fim_prompt(family, &rendered_prefix, &rendered_suffix);

        let mut stop = family_stop_tokens(family);
        stop.extend(fim.stop.iter().cloned());

        let body = build_completions_body(&request.model, &sentinel_prompt, &stop);

        let res = self
            .apply_auth(client.post(format!("{}/v1/completions", self.base_url)))
            .json(&body)
            .send()
            .await
            .ok()?;

        if !res.status().is_success() {
            return None;
        }

        let parsed: OmlxCompletionResponse = res.json().await.ok()?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.text)
            .filter(|text| !text.trim().is_empty())
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
        self.send_chat_request(client, &request.model, &system, &user, OMLX_MAX_TOKENS, false)
            .await
    }

    /// The `/v1/chat/completions` request/response mechanics shared by the auto-tab
    /// chat-fallback path ([`Self::complete_chat`], ghost-text budget, truncation tolerated) and
    /// [`AiProviderClient::instruct`] (already-rendered strings, larger selection-replacement
    /// budget). `fail_on_truncation` mirrors the Ollama Cloud provider's `send_chat_request` — see
    /// its doc comment for why `instruct` treats a truncated response as an error.
    async fn send_chat_request(
        &self,
        client: &reqwest::Client,
        model: &str,
        system: &str,
        user: &str,
        max_tokens: u32,
        fail_on_truncation: bool,
    ) -> AppResult<Option<String>> {
        let body = build_chat_body(model, system, user, max_tokens);

        let res = self
            .apply_auth(client.post(format!("{}/v1/chat/completions", self.base_url)))
            .json(&body)
            .send()
            .await
            .map_err(|error| provider_transport_error(OMLX_PROVIDER_NAME, &error))?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(provider_http_error(OMLX_PROVIDER_NAME, status, &text));
        }

        let parsed: OmlxChatResponse = res
            .json()
            .await
            .map_err(|error| provider_transport_error(OMLX_PROVIDER_NAME, &error))?;
        extract_chat_text(parsed, fail_on_truncation)
    }
}

/// Turns a parsed `/v1/chat/completions` response into the caller's `Option<String>` result —
/// pulled out of [`OmlxProvider::send_chat_request`] so the truncation handling can be unit tested
/// without a live HTTP round trip. A `finish_reason: "length"` first choice only becomes an error
/// when `fail_on_truncation` is set (see [`OmlxProvider::send_chat_request`]'s doc comment).
fn extract_chat_text(parsed: OmlxChatResponse, fail_on_truncation: bool) -> AppResult<Option<String>> {
    let Some(choice) = parsed.choices.into_iter().next() else {
        return Ok(None);
    };
    if fail_on_truncation && choice.finish_reason.as_deref() == Some("length") {
        return Err(AppError::Internal(
            "omlx response was truncated at the output token limit".to_string(),
        ));
    }

    Ok(choice
        .message
        .map(|message| message.content)
        .filter(|content| !content.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 모델_아이디_소문자_부분문자열로_패밀리를_판별한다() {
        assert_eq!(detect_fim_family("Qwen2.5-Coder-7B"), Some(FimFamily::Qwen));
        assert_eq!(detect_fim_family("codegemma-7b-it"), Some(FimFamily::CodeGemma));
        assert_eq!(detect_fim_family("deepseek-coder-6.7b-base"), Some(FimFamily::DeepSeek));
        assert_eq!(detect_fim_family("starcoder2-15b"), Some(FimFamily::StarCoder));
        assert_eq!(detect_fim_family("codellama-13b"), Some(FimFamily::CodeLlama));
        assert_eq!(detect_fim_family("codestral-22b-v0.1"), Some(FimFamily::Codestral));
    }

    #[test]
    fn 알수없는_모델은_패밀리가_없다() {
        assert_eq!(detect_fim_family("llama-3.1-8b-instruct"), None);
    }

    #[test]
    fn codegemma는_qwen보다_먼저_매칭되어_gemma_패밀리로_판별된다() {
        assert_eq!(detect_fim_family("codegemma-2b"), Some(FimFamily::CodeGemma));
    }

    #[test]
    fn qwen_코드젬마_패밀리는_동일한_센티널_형식을_사용한다() {
        assert_eq!(
            assemble_fim_prompt(FimFamily::Qwen, "P", "S"),
            "<|fim_prefix|>P<|fim_suffix|>S<|fim_middle|>"
        );
        assert_eq!(
            assemble_fim_prompt(FimFamily::CodeGemma, "P", "S"),
            "<|fim_prefix|>P<|fim_suffix|>S<|fim_middle|>"
        );
    }

    #[test]
    fn deepseek_센티널은_전각_구분자를_사용한다() {
        assert_eq!(
            assemble_fim_prompt(FimFamily::DeepSeek, "P", "S"),
            "<｜fim▁begin｜>P<｜fim▁hole｜>S<｜fim▁end｜>"
        );
    }

    #[test]
    fn starcoder_센티널_형식() {
        assert_eq!(
            assemble_fim_prompt(FimFamily::StarCoder, "P", "S"),
            "<fim_prefix>P<fim_suffix>S<fim_middle>"
        );
    }

    #[test]
    fn codellama_센티널은_prefix_앞뒤와_suffix_뒤에_공백을_둔다() {
        assert_eq!(assemble_fim_prompt(FimFamily::CodeLlama, "P", "S"), "<PRE> P <SUF>S <MID>");
    }

    #[test]
    fn codestral_센티널은_suffix가_prefix보다_먼저_온다() {
        assert_eq!(assemble_fim_prompt(FimFamily::Codestral, "P", "S"), "[SUFFIX]S[PREFIX]P");
    }

    #[test]
    fn deepseek_stop_토큰에는_주석_구분자가_포함되지_않는다() {
        let stop = family_stop_tokens(FimFamily::DeepSeek);
        assert!(!stop.contains(&"//".to_string()));
        assert!(stop.contains(&"<｜fim▁end｜>".to_string()));
    }

    #[test]
    fn completions_요청_바디는_prompt_stream_stop_필드를_포함한다() {
        let body = build_completions_body(
            "qwen2.5-coder",
            "<|fim_prefix|>P<|fim_suffix|>S<|fim_middle|>",
            &["\n\n\n".to_string()],
        );

        assert_eq!(body["model"], "qwen2.5-coder");
        assert_eq!(body["prompt"], "<|fim_prefix|>P<|fim_suffix|>S<|fim_middle|>");
        assert_eq!(body["stream"], false);
        assert_eq!(body["max_tokens"], OMLX_MAX_TOKENS);
        assert_eq!(body["stop"][0], "\n\n\n");
    }

    #[test]
    fn chat_요청_바디는_system_user_메시지_순서를_지킨다() {
        let body = build_chat_body("qwen2.5-coder", "system prompt", "user prompt", OMLX_MAX_TOKENS);

        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "system prompt");
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "user prompt");
        assert_eq!(body["max_tokens"], OMLX_MAX_TOKENS);
    }

    #[test]
    fn instruct_요청_바디는_auto_tab보다_큰_토큰_예산을_사용한다() {
        let body = build_chat_body("qwen2.5-coder", "system prompt", "user prompt", OMLX_INSTRUCT_MAX_TOKENS);

        assert_eq!(body["max_tokens"], OMLX_INSTRUCT_MAX_TOKENS);
    }

    #[test]
    fn instruct_경로에서_길이_제한으로_잘린_응답은_에러로_처리된다() {
        let parsed = OmlxChatResponse {
            choices: vec![OmlxChatChoice {
                message: Some(OmlxChatMessage {
                    content: "half a function".to_string(),
                }),
                finish_reason: Some("length".to_string()),
            }],
        };

        let result = extract_chat_text(parsed, true);

        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    #[test]
    fn auto_tab_경로에서_길이_제한으로_잘린_응답은_그대로_반환된다() {
        let parsed = OmlxChatResponse {
            choices: vec![OmlxChatChoice {
                message: Some(OmlxChatMessage {
                    content: "short suggestion".to_string(),
                }),
                finish_reason: Some("length".to_string()),
            }],
        };

        let result = extract_chat_text(parsed, false).unwrap();

        assert_eq!(result, Some("short suggestion".to_string()));
    }

    #[test]
    fn finish_reason이_stop이면_잘리지_않은_것으로_처리된다() {
        let parsed = OmlxChatResponse {
            choices: vec![OmlxChatChoice {
                message: Some(OmlxChatMessage {
                    content: "complete response".to_string(),
                }),
                finish_reason: Some("stop".to_string()),
            }],
        };

        let result = extract_chat_text(parsed, true).unwrap();

        assert_eq!(result, Some("complete response".to_string()));
    }

    #[test]
    fn api_key가_없으면_authorization_헤더를_붙이지_않는다() {
        let provider = OmlxProvider {
            base_url: "http://localhost:8000".to_string(),
            api_key: None,
        };
        let client = reqwest::Client::new();
        let builder = provider.apply_auth(client.get("http://localhost:8000/v1/models"));
        let request = builder.build().expect("request builds");
        assert!(request.headers().get("authorization").is_none());
    }

    #[test]
    fn api_key가_있으면_bearer_헤더를_붙인다() {
        let provider = OmlxProvider {
            base_url: "http://localhost:8000".to_string(),
            api_key: Some("secret-key".to_string()),
        };
        let client = reqwest::Client::new();
        let builder = provider.apply_auth(client.get("http://localhost:8000/v1/models"));
        let request = builder.build().expect("request builds");
        assert_eq!(request.headers().get("authorization").unwrap(), "Bearer secret-key");
    }
}

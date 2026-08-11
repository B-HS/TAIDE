use futures_util::{Stream, StreamExt};
use serde::Deserialize;

use crate::domain::ai::prompt;
use crate::domain::ai::providers::{mask_provider_error, AiProviderClient};
use crate::domain::ai::types::{AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate, AiPromptVars};
use crate::error::{AppError, AppResult};

const CODEX_BASE: &str = "https://chatgpt.com/backend-api/codex";
const CODEX_CLIENT_VERSION: &str = "0.144.1";

/// Codex access-token credential, as returned by `whoami` verification
/// (see `domain::ai::service::verify_codex_token`). Kept here — not just the raw token — because
/// every Codex API call needs the `chatgpt-account-id` header alongside the bearer token.
pub struct CodexProvider {
    pub access_token: String,
    pub account_id: String,
}

impl CodexProvider {
    fn apply_auth_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header("authorization", format!("Bearer {}", self.access_token))
            .header("chatgpt-account-id", &self.account_id)
            .header("originator", "codex_cli_rs")
    }
}

/// Ported 1:1 from the reference implementation's `CODEX_FALLBACK_MODELS`
/// (`~/development/b-hub/service/domain/ai/providers/codex-provider.ts`), minus the `metadata`
/// field — `AiModelInfo` here carries only `modelId`/`displayName`.
fn codex_fallback_models() -> Vec<AiModelInfo> {
    [
        ("gpt-5.6-sol", "GPT-5.6 Sol"),
        ("gpt-5.6-terra", "GPT-5.6 Terra"),
        ("gpt-5.6-luna", "GPT-5.6 Luna"),
        ("gpt-5.5", "GPT-5.5"),
        ("gpt-5.4", "GPT-5.4"),
        ("gpt-5.4-mini", "GPT-5.4 Mini"),
        ("gpt-5.3-codex-spark", "GPT-5.3 Codex Spark"),
    ]
    .into_iter()
    .map(|(model_id, display_name)| AiModelInfo {
        model_id: model_id.to_string(),
        display_name: Some(display_name.to_string()),
    })
    .collect()
}

#[derive(Debug, Deserialize)]
struct CodexModelsResponse {
    #[serde(default)]
    models: Vec<CodexModel>,
}

#[derive(Debug, Deserialize)]
struct CodexModel {
    slug: String,
    #[serde(default)]
    display_name: Option<String>,
}

fn build_responses_body(model: &str, instructions: &str, user_text: &str) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "instructions": instructions,
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    { "type": "input_text", "text": user_text }
                ]
            }
        ],
        "store": false,
        "stream": true,
        "tool_choice": "auto",
        "include": [],
    })
}

/// SSE payload shape emitted by `POST {CODEX_BASE}/responses` — mirrors the reference
/// implementation's `parseCodexEvent`. Only the fields the auto-tab ghost-text path needs
/// (delta text, completion, failure) are modeled.
#[derive(Debug, Deserialize)]
struct CodexSseEnvelope {
    #[serde(rename = "type")]
    event_type: Option<String>,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    response: Option<CodexSseResponse>,
}

#[derive(Debug, Deserialize)]
struct CodexSseResponse {
    #[serde(default)]
    error: Option<CodexSseError>,
}

#[derive(Debug, Deserialize)]
struct CodexSseError {
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum CodexStreamStep {
    Delta(String),
    Completed,
    Failed(String),
    Continue,
}

/// Splits a `data: ...` SSE line into its JSON envelope. Non-`data:` lines (blank lines, `event:`,
/// comments), the `[DONE]` sentinel, and unparseable payloads all fall through to `None` — the
/// caller treats them the same way the reference `parseCodexEvent` treats a `null` return: ignore
/// and keep reading.
fn parse_codex_data_line(line: &str) -> Option<CodexSseEnvelope> {
    let trimmed = line.trim();
    let payload = trimmed.strip_prefix("data:")?.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }
    serde_json::from_str(payload).ok()
}

/// Classifies one already-parsed SSE envelope into the step the stream reader should take, per
/// the reference implementation's `relayCodexStream` switch on `evt.type`.
fn classify_codex_event(envelope: &CodexSseEnvelope) -> CodexStreamStep {
    match envelope.event_type.as_deref() {
        Some("response.output_text.delta") => match &envelope.delta {
            Some(delta) => CodexStreamStep::Delta(delta.clone()),
            None => CodexStreamStep::Continue,
        },
        Some("response.completed") => CodexStreamStep::Completed,
        Some(event_type @ ("response.failed" | "response.incomplete")) => {
            let message = envelope
                .response
                .as_ref()
                .and_then(|response| response.error.as_ref())
                .and_then(|error| error.message.clone())
                .unwrap_or_else(|| event_type.to_string());
            CodexStreamStep::Failed(message)
        }
        _ => CodexStreamStep::Continue,
    }
}

/// One line of the raw SSE body in, one stream-reader instruction out. `None` for lines that
/// carry no actionable event (see `parse_codex_data_line`).
fn process_codex_line(line: &str) -> Option<CodexStreamStep> {
    parse_codex_data_line(line).map(|envelope| classify_codex_event(&envelope))
}

/// Appends `chunk` (raw bytes, not yet decoded) to `buffer` and drains every complete
/// `\n`-terminated line out of it (stripping a trailing `\r` for CRLF streams), leaving any
/// trailing partial line buffered for the next chunk — mirrors the reference implementation's
/// `iterateStreamLines`. Buffering at the byte level (rather than decoding each chunk to `str`
/// independently) is required because `reqwest::bytes_stream` chunk boundaries follow TCP/HTTP
/// framing, not UTF-8 character boundaries — a multi-byte character split across two chunks would
/// otherwise decode to U+FFFD replacement characters in each half. Only a complete, buffered line
/// is ever decoded (via `String::from_utf8_lossy`), by which point all of its bytes have arrived.
fn feed_lines(buffer: &mut Vec<u8>, chunk: &[u8]) -> Vec<String> {
    buffer.extend_from_slice(chunk);
    let mut lines = Vec::new();
    while let Some(newline_index) = buffer.iter().position(|&byte| byte == b'\n') {
        let line_bytes: Vec<u8> = buffer.drain(..=newline_index).collect();
        let line = String::from_utf8_lossy(&line_bytes);
        let line = line.strip_suffix('\n').unwrap_or(&line);
        let line = line.strip_suffix('\r').unwrap_or(line);
        lines.push(line.to_string());
    }
    lines
}

/// Drives the `/responses` SSE body to completion, accumulating `response.output_text.delta`
/// text until `response.completed` (success) or `response.failed`/`response.incomplete` (masked
/// error). The stream is consumed for a single, non-streamed return value — auto-tab ghost text
/// only needs the finished completion, not incremental deltas relayed to the frontend.
async fn read_codex_completion<C, E>(mut stream: impl Stream<Item = Result<C, E>> + Unpin) -> AppResult<Option<String>>
where
    C: AsRef<[u8]>,
    E: std::fmt::Display,
{
    let mut line_buffer: Vec<u8> = Vec::new();
    let mut content = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;
        for line in feed_lines(&mut line_buffer, chunk.as_ref()) {
            match process_codex_line(&line) {
                Some(CodexStreamStep::Delta(delta)) => content.push_str(&delta),
                Some(CodexStreamStep::Completed) => return Ok((!content.trim().is_empty()).then_some(content)),
                Some(CodexStreamStep::Failed(message)) => return Err(AppError::Internal(mask_provider_error(&message))),
                Some(CodexStreamStep::Continue) | None => {}
            }
        }
    }

    Ok((!content.trim().is_empty()).then_some(content))
}

impl AiProviderClient for CodexProvider {
    async fn list_models(&self, client: &reqwest::Client) -> AppResult<Vec<AiModelInfo>> {
        let url = format!("{CODEX_BASE}/models?client_version={CODEX_CLIENT_VERSION}");
        let res = self
            .apply_auth_headers(client.get(url))
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "codex models request failed ({status}): {}",
                mask_provider_error(&body)
            )));
        }

        let parsed: CodexModelsResponse = res
            .json()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;
        if parsed.models.is_empty() {
            return Ok(codex_fallback_models());
        }

        Ok(parsed
            .models
            .into_iter()
            .map(|model| AiModelInfo {
                model_id: model.slug,
                display_name: model.display_name,
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
        let instructions = prompt::render(&template.chat.system, &vars);
        let user_text = prompt::render(&template.chat.user, &vars);
        let body = build_responses_body(&request.model, &instructions, &user_text);

        let res = self
            .apply_auth_headers(client.post(format!("{CODEX_BASE}/responses")))
            .header("accept", "text/event-stream")
            .header("session_id", uuid::Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|error| AppError::Internal(mask_provider_error(&error.to_string())))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "codex completion request failed ({status}): {}",
                mask_provider_error(&body)
            )));
        }

        read_codex_completion(res.bytes_stream()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 여러_청크로_쪼개진_라인도_완전한_라인으로_복원된다() {
        let mut buffer = Vec::new();

        let first = feed_lines(&mut buffer, b"data: {\"type\":\"resp");
        assert!(first.is_empty());

        let second = feed_lines(&mut buffer, b"onse.output_text.delta\"}\ndata: next-line\n");
        assert_eq!(second, vec!["data: {\"type\":\"response.output_text.delta\"}", "data: next-line"]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn crlf_개행도_라인_구분자로_처리된다() {
        let mut buffer = Vec::new();
        let lines = feed_lines(&mut buffer, b"data: a\r\ndata: b\r\n");
        assert_eq!(lines, vec!["data: a", "data: b"]);
    }

    #[test]
    fn 청크_경계에서_쪼개진_멀티바이트_문자도_손상없이_복원된다() {
        let mut buffer = Vec::new();
        let full_line_str = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"가\"}\n";
        let full_line = full_line_str.as_bytes().to_vec();
        let char_start = full_line_str.find('가').expect("multi-byte char present");
        let split_at = char_start + 1;
        let (first_chunk, second_chunk) = full_line.split_at(split_at);
        assert!(
            std::str::from_utf8(first_chunk).is_err(),
            "split must land inside the multi-byte character"
        );

        let first = feed_lines(&mut buffer, first_chunk);
        assert!(first.is_empty());

        let second = feed_lines(&mut buffer, second_chunk);
        assert_eq!(second, vec!["data: {\"type\":\"response.output_text.delta\",\"delta\":\"가\"}"]);
        assert!(!second[0].contains('\u{FFFD}'));
    }

    #[test]
    fn delta_이벤트는_델타_텍스트를_반환한다() {
        let line = r#"data: {"type":"response.output_text.delta","delta":"fn main"}"#;
        assert_eq!(process_codex_line(line), Some(CodexStreamStep::Delta("fn main".to_string())));
    }

    #[test]
    fn completed_이벤트는_스트림_종료를_알린다() {
        let line = r#"data: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":2}}}"#;
        assert_eq!(process_codex_line(line), Some(CodexStreamStep::Completed));
    }

    #[test]
    fn failed_이벤트는_에러_메시지를_담아_반환한다() {
        let line = r#"data: {"type":"response.failed","response":{"error":{"message":"rate limited"}}}"#;
        assert_eq!(process_codex_line(line), Some(CodexStreamStep::Failed("rate limited".to_string())));
    }

    #[test]
    fn incomplete_이벤트도_실패로_취급된다() {
        let line = r#"data: {"type":"response.incomplete","response":{"error":{"message":"max_output_tokens"}}}"#;
        assert_eq!(
            process_codex_line(line),
            Some(CodexStreamStep::Failed("max_output_tokens".to_string()))
        );
    }

    #[test]
    fn 에러_메시지가_없는_실패_이벤트는_이벤트_타입을_메시지로_사용한다() {
        let line = r#"data: {"type":"response.failed"}"#;
        assert_eq!(
            process_codex_line(line),
            Some(CodexStreamStep::Failed("response.failed".to_string()))
        );
    }

    #[test]
    fn done_센티널은_무시된다() {
        assert_eq!(process_codex_line("data: [DONE]"), None);
    }

    #[test]
    fn 빈_라인과_이벤트_필드_라인은_무시된다() {
        assert_eq!(process_codex_line(""), None);
        assert_eq!(process_codex_line("event: message"), None);
        assert_eq!(process_codex_line(": comment"), None);
    }

    #[test]
    fn 깨진_json_페이로드는_무시된다() {
        assert_eq!(process_codex_line("data: {not json"), None);
    }

    #[test]
    fn 알수없는_이벤트_타입은_계속_진행으로_처리된다() {
        let line = r#"data: {"type":"response.output_item.added"}"#;
        assert_eq!(process_codex_line(line), Some(CodexStreamStep::Continue));
    }

    #[test]
    fn 완성_바디는_모델_지시문_사용자_메시지_필드를_포함한다() {
        let body = build_responses_body("gpt-5.6-sol", "system instructions", "user text");

        assert_eq!(body["model"], "gpt-5.6-sol");
        assert_eq!(body["instructions"], "system instructions");
        assert_eq!(body["input"][0]["role"], "user");
        assert_eq!(body["input"][0]["content"][0]["text"], "user text");
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert_eq!(body["tool_choice"], "auto");
        assert_eq!(body["include"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn 폴백_모델_목록은_비어있지_않다() {
        let models = codex_fallback_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|model| model.model_id == "gpt-5.6-sol"));
    }

    /// End-to-end read of a fabricated `/responses` SSE body, shaped after the reference
    /// implementation's real event sequence: a handful of `response.output_text.delta` chunks
    /// (split across stream reads exactly as `reqwest::bytes_stream` would deliver them)
    /// followed by `response.completed`.
    #[test]
    fn sse_스트림을_읽어_완성된_텍스트를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> = vec![
            Ok(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"fn add(a: i32"),
            Ok(b", b: i32) -> i32 {\"}\n"),
            Ok(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"\\n    a + b\\n}\"}\n"),
            Ok(b"data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":12,\"output_tokens\":6}}}\n"),
        ];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream)).unwrap();

        assert_eq!(result, Some("fn add(a: i32, b: i32) -> i32 {\n    a + b\n}".to_string()));
    }

    #[test]
    fn 실패_이벤트를_만나면_마스킹된_에러를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> =
            vec![Ok(b"data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"Bearer at-thisisaverylongopaquetoken1234567890 rejected\"}}}\n")];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream));

        let Err(AppError::Internal(message)) = result else {
            panic!("expected AppError::Internal, got {result:?}");
        };
        assert!(!message.contains("at-thisisaverylongopaquetoken1234567890"));
    }

    #[test]
    fn 완성_이벤트_없이_스트림이_끝나도_누적된_텍스트를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> =
            vec![Ok(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n")];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream)).unwrap();

        assert_eq!(result, Some("partial".to_string()));
    }

    #[test]
    fn 빈_스트림은_none을_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> = vec![];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream)).unwrap();

        assert_eq!(result, None);
    }
}

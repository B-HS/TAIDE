use futures_util::{Stream, StreamExt};
use serde::Deserialize;

use crate::domain::ai::prompt;
use crate::domain::ai::providers::{provider_http_error, provider_transport_error, AiProviderClient};
use crate::domain::ai::types::{AiInlineCompleteRequest, AiModelInfo, AiPromptTemplate, AiPromptVars};
use crate::error::{AppError, AppResult};
use crate::infra::redact::mask_provider_error;

const CODEX_PROVIDER_NAME: &str = "codex";

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
    /// `response.incomplete` — Codex stopped generating before `response.completed`. This is a
    /// superset event, not truncation-only: the Responses API fires it both when the provider hit
    /// its own output-token budget (`incomplete_details.reason == "max_output_tokens"`) and when a
    /// content filter cut the response short (`"content_filter"`). `CodexSseEnvelope` does not
    /// parse `incomplete_details`, so the two cases aren't distinguished here — matching the
    /// reference implementation, which likewise doesn't model `incomplete_details`. Distinct from
    /// `Failed`'s `response.failed` (an actual provider-side error). Kept as its own step (rather
    /// than folded into `Failed`) so [`read_codex_completion`] can honor `fail_on_truncation` the
    /// same way Ollama Cloud/oMLX's `extract_chat_text` does for a `done_reason`/`finish_reason`
    /// of `"length"` — see `docs/acknowledge/2026-08-25-d37-ai-batch-contract.md` §3 for the full
    /// before/after.
    Incomplete(String),
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

/// Pulls the human-readable error message out of a `response.failed`/`response.incomplete`
/// envelope, falling back to the event type itself when the provider sent no `response.error`
/// (see `에러_메시지가_없는_실패_이벤트는_이벤트_타입을_메시지로_사용한다`).
fn codex_event_message(envelope: &CodexSseEnvelope, event_type: &str) -> String {
    envelope
        .response
        .as_ref()
        .and_then(|response| response.error.as_ref())
        .and_then(|error| error.message.clone())
        .unwrap_or_else(|| event_type.to_string())
}

/// Classifies one already-parsed SSE envelope into the step the stream reader should take, per
/// the reference implementation's `relayCodexStream` switch on `evt.type`. `response.failed` and
/// `response.incomplete` are distinct steps (not the same arm) so [`read_codex_completion`] can
/// treat a mere truncation more leniently than an actual provider failure — see
/// [`CodexStreamStep::Incomplete`]'s doc comment.
fn classify_codex_event(envelope: &CodexSseEnvelope) -> CodexStreamStep {
    match envelope.event_type.as_deref() {
        Some("response.output_text.delta") => match &envelope.delta {
            Some(delta) => CodexStreamStep::Delta(delta.clone()),
            None => CodexStreamStep::Continue,
        },
        Some("response.completed") => CodexStreamStep::Completed,
        Some(event_type @ "response.failed") => CodexStreamStep::Failed(codex_event_message(envelope, event_type)),
        Some(event_type @ "response.incomplete") => CodexStreamStep::Incomplete(codex_event_message(envelope, event_type)),
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
/// text until `response.completed` (success), `response.failed` (always a masked error — a
/// genuine provider-side failure), or `response.incomplete` (a truncation, not a failure —
/// resolved per `fail_on_truncation` exactly like Ollama Cloud/oMLX's `extract_chat_text`:
/// `false` — the auto-tab ghost-text path — returns whatever delta text had accumulated so far,
/// the same tolerant outcome this function already gives a stream that ends without any
/// `response.*` terminal event at all; `true` — [`AiProviderClient::instruct`]'s Inline
/// Edit/commit-message path — masks and errors, since a silently truncated selection replacement
/// would otherwise look like a complete, valid suggestion). The stream is consumed for a single,
/// non-streamed return value — auto-tab ghost text only needs the finished completion, not
/// incremental deltas relayed to the frontend.
async fn read_codex_completion<C, E>(
    mut stream: impl Stream<Item = Result<C, E>> + Unpin,
    fail_on_truncation: bool,
) -> AppResult<Option<String>>
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
                Some(CodexStreamStep::Incomplete(message)) => {
                    if fail_on_truncation {
                        return Err(AppError::Internal(mask_provider_error(&message)));
                    }
                    return Ok((!content.trim().is_empty()).then_some(content));
                }
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
            .map_err(|error| provider_transport_error(CODEX_PROVIDER_NAME, &error))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(provider_http_error(CODEX_PROVIDER_NAME, status, &body));
        }

        let parsed: CodexModelsResponse = res
            .json()
            .await
            .map_err(|error| provider_transport_error(CODEX_PROVIDER_NAME, &error))?;
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
        self.send_responses_request(client, &request.model, &instructions, &user_text, false)
            .await
    }

    async fn instruct(&self, client: &reqwest::Client, model: &str, system: &str, user: &str) -> AppResult<Option<String>> {
        self.send_responses_request(client, model, system, user, true).await
    }
}

impl CodexProvider {
    /// The `/responses` SSE request/response mechanics shared by the auto-tab completion path
    /// ([`AiProviderClient::complete`], which renders a template first, `fail_on_truncation:
    /// false`) and [`AiProviderClient::instruct`] (which is handed already-rendered strings,
    /// `fail_on_truncation: true`) — Codex has no separate FIM endpoint, so this *is* the
    /// provider's only completion mechanism, unlike Ollama Cloud/oMLX's FIM-first/chat-fallback
    /// split. `fail_on_truncation` is threaded straight through to [`read_codex_completion`] — see
    /// its doc comment for what each value does with a `response.incomplete` event.
    async fn send_responses_request(
        &self,
        client: &reqwest::Client,
        model: &str,
        instructions: &str,
        user_text: &str,
        fail_on_truncation: bool,
    ) -> AppResult<Option<String>> {
        let body = build_responses_body(model, instructions, user_text);

        let res = self
            .apply_auth_headers(client.post(format!("{CODEX_BASE}/responses")))
            .header("accept", "text/event-stream")
            .header("session_id", uuid::Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|error| provider_transport_error(CODEX_PROVIDER_NAME, &error))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(provider_http_error(CODEX_PROVIDER_NAME, status, &body));
        }

        read_codex_completion(res.bytes_stream(), fail_on_truncation).await
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
    fn incomplete_이벤트는_failed와_별개의_스텝으로_분류된다() {
        let line = r#"data: {"type":"response.incomplete","response":{"error":{"message":"max_output_tokens"}}}"#;
        assert_eq!(
            process_codex_line(line),
            Some(CodexStreamStep::Incomplete("max_output_tokens".to_string()))
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
    fn 에러_메시지가_없는_incomplete_이벤트도_이벤트_타입을_메시지로_사용한다() {
        let line = r#"data: {"type":"response.incomplete"}"#;
        assert_eq!(
            process_codex_line(line),
            Some(CodexStreamStep::Incomplete("response.incomplete".to_string()))
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

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, false)).unwrap();

        assert_eq!(result, Some("fn add(a: i32, b: i32) -> i32 {\n    a + b\n}".to_string()));
    }

    #[test]
    fn 실패_이벤트를_만나면_마스킹된_에러를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> =
            vec![Ok(b"data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"Bearer at-thisisaverylongopaquetoken1234567890 rejected\"}}}\n")];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, false));

        let Err(AppError::Internal(message)) = result else {
            panic!("expected AppError::Internal, got {result:?}");
        };
        assert!(!message.contains("at-thisisaverylongopaquetoken1234567890"));
    }

    /// `response.failed` errors unconditionally, regardless of `fail_on_truncation` — a genuine
    /// provider-side failure is never the tolerant "return what we have" outcome, unlike
    /// `response.incomplete` (see the two tests below).
    #[test]
    fn 실패_이벤트는_fail_on_truncation이_false여도_에러를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> = vec![Ok(
            b"data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"message\":\"rate limited\"}}}\n",
        )];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, false));

        assert!(matches!(result, Err(AppError::Internal(_))));
    }

    /// #18 회귀: auto-tab(`fail_on_truncation: false`)이 `response.incomplete`를 만나면 그때까지
    /// 누적된 델타 텍스트를 관용적으로 반환해야 한다 — ollama/omlx의 `done_reason`/
    /// `finish_reason: "length"` 처리와 동일한 의미.
    #[test]
    fn 관용_경로에서_incomplete_이벤트는_그때까지_누적된_텍스트를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> = vec![
            Ok(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"fn add(a\"}\n"),
            Ok(b"data: {\"type\":\"response.incomplete\",\"response\":{\"error\":{\"message\":\"max_output_tokens\"}}}\n"),
        ];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, false)).unwrap();

        assert_eq!(result, Some("fn add(a".to_string()));
    }

    /// #18 회귀: instruct(`fail_on_truncation: true`, Inline Edit/AI 커밋 메시지)는 잘린 응답을
    /// 완전한 제안처럼 보이게 두지 않고 에러로 처리해야 한다 — ollama/omlx의 instruct 경로와
    /// 동일한 의미.
    #[test]
    fn instruct_경로에서_incomplete_이벤트는_마스킹된_에러를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> = vec![
            Ok(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"fn add(a\"}\n"),
            Ok(b"data: {\"type\":\"response.incomplete\",\"response\":{\"error\":{\"message\":\"max_output_tokens\"}}}\n"),
        ];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, true));

        let Err(AppError::Internal(message)) = result else {
            panic!("expected AppError::Internal, got {result:?}");
        };
        assert!(message.contains("max_output_tokens"));
    }

    #[test]
    fn 완성_이벤트_없이_스트림이_끝나도_누적된_텍스트를_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> =
            vec![Ok(b"data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n")];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, false)).unwrap();

        assert_eq!(result, Some("partial".to_string()));
    }

    #[test]
    fn 빈_스트림은_none을_반환한다() {
        let chunks: Vec<Result<&'static [u8], std::io::Error>> = vec![];
        let stream = futures_util::stream::iter(chunks);

        let result = tauri::async_runtime::block_on(read_codex_completion(stream, false)).unwrap();

        assert_eq!(result, None);
    }
}

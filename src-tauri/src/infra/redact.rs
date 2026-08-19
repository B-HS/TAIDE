const MAX_PROVIDER_ERROR_MESSAGE_LEN: usize = 500;
const MIN_OPAQUE_TOKEN_LEN: usize = 20;
const BEARER_NEEDLE: &str = "bearer ";
const BEARER_REPLACEMENT: &str = "Bearer [redacted]";
const OPAQUE_TOKEN_REPLACEMENT: &str = "[redacted]";

/// A provider's response body may (rarely) echo request headers or other sensitive substrings
/// back in an error message. Bearer tokens and long opaque token-like strings are redacted
/// before the message is ever attached to an `AppError` that reaches IPC/logs. Lives in `infra`
/// because both the AI provider clients and `domain::sync`'s GitHub client need it — a pure
/// string utility kept out of the domain graph, the same descent `infra::archive::
/// extract_hardened_zip` made (T1-I §1.3).
pub fn mask_provider_error(message: &str) -> String {
    let bearer_masked = mask_bearer_values(message);
    let token_masked = mask_long_tokens(&bearer_masked);
    token_masked.chars().take(MAX_PROVIDER_ERROR_MESSAGE_LEN).collect()
}

/// ASCII case-insensitive substring search. `needle` is always ASCII-only in this module, so a
/// byte-for-byte match can never straddle a multi-byte UTF-8 sequence (its continuation bytes all
/// have the high bit set and can't equal an ASCII byte) — every match start is guaranteed to land
/// on a `str` char boundary, unlike comparing offsets found in a `to_lowercase()` copy against the
/// original string.
fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    if needle_bytes.is_empty() || haystack_bytes.len() < needle_bytes.len() {
        return None;
    }
    (0..=haystack_bytes.len() - needle_bytes.len())
        .find(|&start| haystack_bytes[start..start + needle_bytes.len()].eq_ignore_ascii_case(needle_bytes))
}

/// Redacts every `bearer <value>` occurrence (case-insensitive), not just the first one.
fn mask_bearer_values(message: &str) -> String {
    let mut result = String::with_capacity(message.len());
    let mut rest = message;

    while let Some(start) = find_ascii_case_insensitive(rest, BEARER_NEEDLE) {
        let value_start = start + BEARER_NEEDLE.len();
        let value_end = rest[value_start..]
            .find(char::is_whitespace)
            .map(|offset| value_start + offset)
            .unwrap_or(rest.len());

        result.push_str(&rest[..start]);
        result.push_str(BEARER_REPLACEMENT);
        rest = &rest[value_end..];
    }
    result.push_str(rest);
    result
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.'
}

/// Redacts every run of token-like characters (`[A-Za-z0-9._-]`) at least `MIN_OPAQUE_TOKEN_LEN`
/// long, wherever it appears — inside JSON string values, next to punctuation, or separated by
/// any whitespace (including newlines) — rather than only whole space-delimited words. Every run
/// boundary here is ASCII, so slicing at it never crosses a multi-byte char.
fn mask_long_tokens(message: &str) -> String {
    let mut result = String::with_capacity(message.len());
    let mut run_start: Option<usize> = None;

    for (index, ch) in message.char_indices() {
        match (run_start, is_token_char(ch)) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                push_run_or_literal(&mut result, message, start, index);
                result.push(ch);
                run_start = None;
            }
            (None, false) => result.push(ch),
            (Some(_), true) => {}
        }
    }
    if let Some(start) = run_start {
        push_run_or_literal(&mut result, message, start, message.len());
    }

    result
}

fn push_run_or_literal(result: &mut String, message: &str, start: usize, end: usize) {
    let run = &message[start..end];
    if run.len() >= MIN_OPAQUE_TOKEN_LEN {
        result.push_str(OPAQUE_TOKEN_REPLACEMENT);
    } else {
        result.push_str(run);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_토큰은_마스킹된다() {
        let masked = mask_provider_error("request failed: Authorization: Bearer sk-abcdef1234567890 rejected");
        assert!(!masked.contains("sk-abcdef1234567890"));
        assert!(masked.contains("Bearer [redacted]"));
    }

    #[test]
    fn 긴_불투명_토큰_문자열은_마스킹된다() {
        let masked = mask_provider_error("invalid token at-thisisaveryverylongopaquetokenvalue123");
        assert!(!masked.contains("at-thisisaveryverylongopaquetokenvalue123"));
        assert!(masked.contains("[redacted]"));
    }

    #[test]
    fn 짧은_단어는_마스킹되지_않는다() {
        let masked = mask_provider_error("model not found");
        assert_eq!(masked, "model not found");
    }

    #[test]
    fn 최대_길이를_넘는_메시지는_잘린다() {
        let long_message = "word ".repeat(200);
        let masked = mask_provider_error(&long_message);
        assert_eq!(masked.chars().count(), MAX_PROVIDER_ERROR_MESSAGE_LEN);
    }

    #[test]
    fn 멀티바이트_문자가_bearer_앞에_와도_패닉하지_않고_토큰을_마스킹한다() {
        let masked = mask_provider_error("ẞẞ Bearer at-thisisaverylongopaquetoken1234567890 rejected");
        assert!(!masked.contains("at-thisisaverylongopaquetoken1234567890"));
        assert!(masked.contains("Bearer [redacted]"));
        assert!(masked.starts_with("ẞẞ"));
    }

    #[test]
    fn bearer가_여러번_나오면_전부_마스킹된다() {
        let masked = mask_provider_error("Bearer at-thisisaverylongopaquetoken111 then again Bearer at-thisisaverylongopaquetoken222 done");
        assert!(!masked.contains("at-thisisaverylongopaquetoken111"));
        assert!(!masked.contains("at-thisisaverylongopaquetoken222"));
        assert_eq!(masked.matches("Bearer [redacted]").count(), 2);
    }

    #[test]
    fn json_바디에_담긴_토큰도_따옴표에_붙어있어도_마스킹된다() {
        let masked = mask_provider_error(r#"{"error":{"message":"invalid token: at-thisisaverylongopaquetoken1234567890"}}"#);
        assert!(!masked.contains("at-thisisaverylongopaquetoken1234567890"));
        assert!(masked.contains("[redacted]"));
        assert!(masked.starts_with(r#"{"error":{"message""#));
    }

    #[test]
    fn 개행으로_구분된_토큰도_마스킹된다() {
        let masked = mask_provider_error("token rejected:\nat-thisisaveryverylongopaquetokenvalue123\nplease retry");
        assert!(!masked.contains("at-thisisaveryverylongopaquetokenvalue123"));
        assert!(masked.contains("[redacted]"));
    }
}

use std::ops::Range;
use std::sync::OnceLock;

use regex::Regex;

const MAX_PROVIDER_ERROR_MESSAGE_LEN: usize = 500;
const MIN_OPAQUE_TOKEN_LEN: usize = 20;
const BEARER_NEEDLE: &str = "bearer ";
const BEARER_REPLACEMENT: &str = "Bearer [redacted]";
const OPAQUE_TOKEN_REPLACEMENT: &str = "[redacted]";

/// The credential shapes [`mask_known_secrets`] removes, as one alternation so a single pass over
/// the text finds all of them. Every branch carries exactly one **named** group, and it is that
/// group's span — not the whole match's — that gets redacted: the last three branches match
/// surrounding context (`://user:`, `Authorization: Bearer `, `token=`) purely to locate the value,
/// and keeping that context in the output is what makes a masked line still diagnosable.
///
/// Branch order matters where two prefixes nest (`sk-ant-` before `sk-`): the regex crate's
/// alternation is leftmost-**first**, so at a shared start offset the earlier branch wins and the
/// redaction is named after the more specific issuer.
///
/// Only issuer-shaped credentials are listed. IP addresses, MAC addresses, phone numbers and long
/// opaque identifiers are deliberately absent — see [`mask_known_secrets`].
const SECRET_PATTERN: &str = concat!(
    r"(?P<github>gh[pousr]_[A-Za-z0-9]{16,}|github_pat_[A-Za-z0-9_]{20,})",
    r"|(?P<gitlab>glpat-[A-Za-z0-9_-]{16,})",
    r"|(?P<anthropic>sk-ant-[A-Za-z0-9_-]{16,})",
    r"|(?P<stripe>[sr]k_live_[A-Za-z0-9]{16,})",
    r"|(?P<openai>sk-[A-Za-z0-9_-]{16,})",
    r"|(?P<aws>AKIA[0-9A-Z]{16})",
    r"|(?P<google>AIza[0-9A-Za-z_-]{35})",
    r"|(?P<slack>xox[baprs]-[A-Za-z0-9-]{10,})",
    r"|(?P<npm>npm_[A-Za-z0-9]{20,})",
    r"|(?P<jwt>eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]*)",
    r"|://[^\s:/@]+:(?P<url_password>[^\s@/]+)@",
    r"|(?i:authorization\s*:\s*bearer\s+)(?P<bearer>\S+)",
    r#"|(?i:(?:api[_-]?key|secret|token|password|passwd)\s*=\s*)(?P<key_value>[^\s"'&;]+)"#,
);

/// Every named group of [`SECRET_PATTERN`], which is also the label a redaction carries
/// (`[redacted:github]`).
const SECRET_GROUP_NAMES: &[&str] = &[
    "github",
    "gitlab",
    "anthropic",
    "stripe",
    "openai",
    "aws",
    "google",
    "slack",
    "npm",
    "jwt",
    "url_password",
    "bearer",
    "key_value",
];

fn secret_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(SECRET_PATTERN).expect("유효한 시크릿 정규식"))
}

/// Removes credential-shaped substrings from `text` and leaves everything else byte-for-byte
/// intact, so a masked subprocess `stderr` still reads as the diagnostic it is.
///
/// This is the mask for text whose *diagnostic value is the point* — git command failures, language
/// server installer output, notification bodies. [`mask_provider_error`] is the opposite trade: it
/// deletes every token-shaped run over [`MIN_OPAQUE_TOKEN_LEN`], which also deletes commit shas,
/// UUIDs, `refs/heads/...` and long paths — acceptable for an AI provider's response body, fatal
/// for "git push failed: <reason>". Contract `docs/acknowledge/2026-09-06-d57-infra-hardening-wave1-contract.md` §1.C.
pub fn mask_known_secrets(text: &str) -> String {
    let mut spans: Vec<(Range<usize>, &'static str)> = Vec::new();
    for captures in secret_pattern().captures_iter(text) {
        let Some((name, matched)) = SECRET_GROUP_NAMES
            .iter()
            .find_map(|name| captures.name(name).map(|matched| (*name, matched)))
        else {
            continue;
        };
        spans.push((matched.range(), name));
    }

    let mut masked = text.to_string();
    for (range, name) in spans.into_iter().rev() {
        masked.replace_range(range, &format!("[redacted:{name}]"));
    }
    masked
}

/// A provider's response body may (rarely) echo request headers or other sensitive substrings
/// back in an error message. Bearer tokens and long opaque token-like strings are redacted
/// before the message is ever attached to an `AppError` that reaches IPC/logs. Lives in `infra`
/// because both the AI provider clients and `domain::sync`'s GitHub client need it — a pure
/// string utility kept out of the domain graph, the same descent `infra::archive::
/// extract_hardened_zip` made (T1-I §1.3).
///
/// Runs [`mask_known_secrets`] first, so a recognized credential is named in the output instead of
/// vanishing unnamed into the length-based pass below.
pub fn mask_provider_error(message: &str) -> String {
    let known_masked = mask_known_secrets(message);
    let bearer_masked = mask_bearer_values(&known_masked);
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

    /// Issuer prefix, body, and the redaction label each pair must produce. Split into two halves so
    /// that no line of this file is itself a credential-shaped literal.
    const SECRET_FIXTURES: &[(&str, &str, &str)] = &[
        ("ghp_", "abcdefghijklmnopqrstuvwxyz0123456789", "github"),
        ("github_pat_", "11ABCDEFG0abcdefghijklmnopqrstuvwxyz", "github"),
        ("glpat-", "abcdefghijklmnopqrst", "gitlab"),
        ("sk-ant-", "api03-abcdefghijklmnopqrstuvwxyz", "anthropic"),
        ("sk-", "abcdefghijklmnopqrstuvwxyz012345", "openai"),
        ("sk_live_", "abcdefghijklmnopqrstuvwxyz", "stripe"),
        ("rk_live_", "abcdefghijklmnopqrstuvwxyz", "stripe"),
        ("AKIA", "IOSFODNN7EXAMPLE", "aws"),
        ("AIza", "SyA1234567890abcdefghijklmnopqrstuv", "google"),
        ("xoxb-", "1234567890-abcdefghijkl", "slack"),
        ("npm_", "abcdefghijklmnopqrstuvwxyz0123456789", "npm"),
        ("eyJhbGciOiJIUzI1NiJ9.", "eyJzdWIiOiIxIn0.dBjftJeZ4CVPmB92K27u", "jwt"),
    ];

    fn fixture(prefix: &str, body: &str) -> String {
        format!("{prefix}{body}")
    }

    #[test]
    fn 발급자별_자격증명은_이름과_함께_마스킹된다() {
        for (prefix, body, name) in SECRET_FIXTURES {
            let secret = fixture(prefix, body);
            let masked = mask_known_secrets(&format!("install failed with {secret} at the end"));

            assert!(!masked.contains(&secret), "{name} 자격증명이 그대로 남아 있습니다: {masked}");
            assert!(
                masked.contains(&format!("[redacted:{name}]")),
                "{name} 이름으로 치환되지 않았습니다: {masked}"
            );
        }
    }

    #[test]
    fn url_사용자정보의_비밀번호만_마스킹되고_나머지_주소는_남는다() {
        let secret = fixture("ghp_", "abcdefghijklmnopqrstuvwxyz012345");
        let masked = mask_known_secrets(&format!(
            "fatal: could not read from https://taide:{secret}@github.com/org/repo.git"
        ));

        assert!(!masked.contains(&secret));
        assert!(masked.contains("https://taide:[redacted:url_password]@github.com/org/repo.git"));
    }

    #[test]
    fn authorization_bearer_헤더의_값만_마스킹된다() {
        let masked = mask_known_secrets("Authorization: Bearer abcdefghijklmnop.qrstuvwxyz\nX-Request-Id: 42");

        assert!(!masked.contains("abcdefghijklmnop.qrstuvwxyz"));
        assert!(masked.contains("Authorization: Bearer [redacted:bearer]"));
        assert!(masked.contains("X-Request-Id: 42"), "나머지 진단 정보는 그대로 남아야 한다");
    }

    #[test]
    fn 키_이름이_붙은_값은_키를_남기고_값만_마스킹된다() {
        let masked = mask_known_secrets("npm ERR! //registry.npmjs.org/:_authToken=abcd-1234-efgh-5678 was rejected");

        assert!(!masked.contains("abcd-1234-efgh-5678"));
        assert!(masked.contains("_authToken=[redacted:key_value]"));
        assert!(masked.contains("//registry.npmjs.org/"), "어느 레지스트리였는지는 진단에 필요하다");
        assert!(mask_known_secrets("api_key = plaintextvalue").contains("[redacted:key_value]"));
        assert!(mask_known_secrets("PASSWORD=hunter2").contains("[redacted:key_value]"));
    }

    #[test]
    fn 한_문자열에_여러_시크릿이_있어도_전부_마스킹된다() {
        let github = fixture("ghp_", "abcdefghijklmnopqrstuvwxyz0123456789");
        let aws = fixture("AKIA", "IOSFODNN7EXAMPLE");
        let masked = mask_known_secrets(&format!("first {github} then {aws} done"));

        assert!(masked.contains("[redacted:github]"));
        assert!(masked.contains("[redacted:aws]"));
        assert!(masked.starts_with("first "));
        assert!(masked.ends_with(" done"));
    }

    /// The reason this mask exists next to [`mask_provider_error`] instead of replacing it: the
    /// identifiers a git or installer failure is *about* have to survive it unchanged.
    #[test]
    fn 진단에_필요한_긴_식별자는_마스킹되지_않는다() {
        let survivors = [
            "9f2c1a4d7b3e5f60819a2b3c4d5e6f708192a3b4",
            "550e8400-e29b-41d4-a716-446655440000",
            "refs/heads/feature/long-branch-name",
            "node_modules/@scope/some-really-long-package-name/dist/esm/index.js",
            "https://github.com/org/repo.git",
            "error: pathspec 'docs/acknowledge/2026-09-06-contract.md' did not match",
            "npm_config_registry",
            "compiled 1234 modules in 5678ms",
        ];

        for text in survivors {
            assert_eq!(mask_known_secrets(text), text, "진단에 필요한 문자열이 마스킹되었습니다: {text}");
        }
    }

    #[test]
    fn 시크릿이_없으면_원문이_그대로다() {
        let text = "fatal: not a git repository (or any of the parent directories): .git";
        assert_eq!(mask_known_secrets(text), text);
    }

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

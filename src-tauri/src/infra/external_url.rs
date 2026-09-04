use crate::error::{AppError, AppErrorKind, AppResult};

/// Schemes `system_open_external_url` accepts, checked ASCII-case-insensitively against the
/// (trimmed) start of the URL.
const EXTERNAL_URL_ALLOWED_SCHEMES: &[&str] = &["http://", "https://"];

/// Unicode "format" (Cf category) characters commonly abused to visually spoof a URL's displayed
/// host — bidi overrides/embeddings (e.g. U+202E RIGHT-TO-LEFT OVERRIDE can make `evil.com` render
/// reversed inside an otherwise-trustworthy-looking string), zero-width joiners/spaces that can
/// split a hostname into something a human skims past, and a stray BOM. `char::is_control()` only
/// covers the Cc category and lets every one of these through. This is a fixed denylist of the
/// characters actually used for this kind of spoofing rather than a full Cf-category check — no
/// unicode-category crate is pulled in for it, the same "no new dependency" call
/// `EXTERNAL_URL_ALLOWED_SCHEMES` already makes.
const UNICODE_SPOOFING_CONTROL_CHARS: &[char] = &[
    '\u{200B}', '\u{200C}', '\u{200D}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2060}',
    '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{FEFF}',
];

/// Whitelists `http(s)://` for `tauri_plugin_opener::open_url` — the callers are the app's own
/// "open this link outside" paths (`domain::system::commands::system_open_external_url`, reached
/// from a clicked terminal link or an external anchor, and
/// [`navigation_guard::open_new_window_externally`](super::navigation_guard::open_new_window_externally), which
/// re-routes a denied `window.open()` to the OS browser), all of which only ever mean an
/// `http(s)://` link, so a scheme prefix check alone is enough to keep this from
/// becoming a generic "open anything the OS shell understands" primitive (`file://`,
/// `javascript:`, a custom app-registered scheme, etc. are all rejected). No `url` crate is
/// pulled in for this — a prefix check plus the two checks below are the whole job. Leading/
/// trailing whitespace is trimmed first (a terminal selection commonly carries it); anything left
/// after that — a control character, interior whitespace, or a [`UNICODE_SPOOFING_CONTROL_CHARS`]
/// character — is rejected, since a crafted OSC 8 hyperlink or a language-server-generated string
/// routed here some other way could otherwise smuggle a shell-hostile or visually-spoofed value
/// through to the OS opener. A userinfo segment (`@`) before the host is rejected too — browsers
/// still navigate `https://trusted.example@evil.example/` to `evil.example`, so without this a
/// URL that *displays* a trusted-looking prefix can open a completely different site (the same
/// spoofing pattern `settings::service::is_valid_allowed_host` already rejects for allowed-host
/// entries).
pub fn validate_external_url(url: &str) -> AppResult<String> {
    let trimmed = url.trim();
    let matched_scheme_len = EXTERNAL_URL_ALLOWED_SCHEMES
        .iter()
        .find(|scheme| {
            trimmed
                .get(..scheme.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
        })
        .map(|scheme| scheme.len());
    let Some(scheme_len) = matched_scheme_len else {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.system.urlSchemeNotAllowed",
            "only URLs starting with http:// or https:// can be opened",
        ));
    };
    if trimmed
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || UNICODE_SPOOFING_CONTROL_CHARS.contains(&c))
    {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.system.urlHasControlChars",
            "the URL cannot contain control characters or whitespace",
        ));
    }
    let authority = &trimmed[scheme_len..];
    let authority_end = authority.find(['/', '?', '#']).unwrap_or(authority.len());
    if authority[..authority_end].contains('@') {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.system.urlHasUserInfo",
            "the URL cannot contain user info (@) before the host",
        ));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_와_https_스킴은_허용된다() {
        assert!(validate_external_url("http://example.com").is_ok());
        assert!(validate_external_url("https://example.com/path?query=1").is_ok());
    }

    #[test]
    fn 대소문자가_섞인_https_스킴도_허용된다() {
        assert!(validate_external_url("HTTPS://example.com").is_ok());
        assert!(validate_external_url("HtTp://example.com").is_ok());
    }

    #[test]
    fn file_스킴은_거부된다() {
        assert!(validate_external_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn 제어_문자가_섞이면_거부된다() {
        assert!(validate_external_url("http://example.com/\u{7}").is_err());
    }

    #[test]
    fn 빈_문자열은_거부된다() {
        assert!(validate_external_url("").is_err());
    }

    #[test]
    fn url_중간의_공백은_거부되지만_전후_공백은_트림된다() {
        assert!(
            validate_external_url("http://example .com").is_err(),
            "URL 중간 공백은 거부되어야 한다"
        );
        assert_eq!(
            validate_external_url("  http://example.com  ").expect("전후 공백은 트림되어 통과해야 한다"),
            "http://example.com"
        );
    }

    #[test]
    fn bidi_override_등_유니코드_시각_위장_문자가_섞이면_거부된다() {
        assert!(
            validate_external_url("https://exa\u{202E}mple.com").is_err(),
            "U+202E RIGHT-TO-LEFT OVERRIDE 는 거부되어야 한다"
        );
        assert!(
            validate_external_url("https://example.com/\u{200B}path").is_err(),
            "U+200B ZERO WIDTH SPACE 는 거부되어야 한다"
        );
        assert!(
            validate_external_url("\u{FEFF}https://example.com").is_err(),
            "U+FEFF BOM 은 거부되어야 한다"
        );
    }

    #[test]
    fn 호스트_앞_userinfo는_거부된다() {
        assert!(
            validate_external_url("https://github.com@evil.example/login").is_err(),
            "userinfo(@) 로 신뢰 도메인을 위장한 URL 은 거부되어야 한다"
        );
        assert!(
            validate_external_url("https://user:pass@evil.example").is_err(),
            "user:pass@ 형태의 userinfo 도 거부되어야 한다"
        );
    }

    #[test]
    fn 경로나_쿼리에_있는_at_기호는_허용된다() {
        assert!(validate_external_url("https://example.com/path@2x.png").is_ok());
        assert!(validate_external_url("https://example.com/search?q=a@b.com").is_ok());
    }
}

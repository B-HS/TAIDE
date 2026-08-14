use crate::domain::locale::service::{builtin_by_id, builtin_en};
use crate::domain::locale::types::LocalePack;

use super::types::REMOTE_LOGIN_PATH;

/// CSP for the login page response. Stricter than the SPA's `BROWSER_CSP`
/// (`serving.rs`) — this page ships zero script and no cross-origin assets,
/// so it can afford `script-src 'none'` and `connect-src 'none'` outright.
pub const LOGIN_PAGE_CSP: &str =
    "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'; script-src 'none'; connect-src 'none'";

const LOGIN_PAGE_STYLE: &str = r#"
:root {
    color-scheme: dark;
    --tp-bg: #16181d;
    --tp-card-bg: #1f2229;
    --tp-border: #33373f;
    --tp-text: #e7e9ee;
    --tp-text-muted: #9aa0ab;
    --tp-accent: #5b8def;
    --tp-error: #f2685c;
}
@media (prefers-color-scheme: light) {
    :root {
        color-scheme: light;
        --tp-bg: #f4f5f7;
        --tp-card-bg: #ffffff;
        --tp-border: #dde1e7;
        --tp-text: #1b1e24;
        --tp-text-muted: #5c6270;
        --tp-accent: #3868d6;
        --tp-error: #c53a2d;
    }
}
* { box-sizing: border-box; }
body {
    margin: 0;
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--tp-bg);
    color: var(--tp-text);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    padding: 24px;
}
.card {
    width: 100%;
    max-width: 340px;
    background: var(--tp-card-bg);
    border: 1px solid var(--tp-border);
    border-radius: 12px;
    padding: 32px 28px;
}
h1 {
    font-size: 17px;
    font-weight: 600;
    margin: 0 0 20px;
    text-align: center;
}
label {
    display: block;
    font-size: 13px;
    color: var(--tp-text-muted);
    margin-bottom: 6px;
}
input[type='password'] {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--tp-border);
    background: transparent;
    color: var(--tp-text);
    font-size: 14px;
    margin-bottom: 16px;
}
input[type='password']:disabled {
    opacity: 0.5;
}
button {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    border: none;
    background: var(--tp-accent);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
}
button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}
.status {
    font-size: 13px;
    margin: 0 0 16px;
    padding: 8px 10px;
    border-radius: 8px;
}
.status-error {
    color: var(--tp-error);
    background: color-mix(in srgb, var(--tp-error) 14%, transparent);
}
.status-locked {
    color: var(--tp-text-muted);
    background: color-mix(in srgb, var(--tp-text-muted) 14%, transparent);
}
.notice {
    margin: 16px 0 0;
    font-size: 12px;
    color: var(--tp-text-muted);
    text-align: center;
}
"#;

pub struct LoginPageParams<'a> {
    pub language: &'a str,
    pub failed: bool,
    pub locked_remaining_seconds: Option<u64>,
    pub insecure: bool,
}

fn resolve_pack(language: &str) -> LocalePack {
    builtin_by_id(language).unwrap_or_else(builtin_en)
}

fn message(pack: &LocalePack, key: &str) -> String {
    pack.messages.get(key).cloned().unwrap_or_default()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Renders the self-contained login page: no external assets, no inline or
/// external script (the CSP forbids both), a single `<form method="post">`.
/// Strings come from the app's own builtin locale packs (`en`/`ko`/`ja`) so
/// the login page never carries a second copy of these messages.
pub fn render(params: LoginPageParams) -> String {
    let pack = resolve_pack(params.language);
    let title = escape_html(&message(&pack, "remote.loginTitle"));
    let password_label = escape_html(&message(&pack, "remote.loginPasswordLabel"));
    let submit_label = escape_html(&message(&pack, "remote.loginSubmit"));

    let status_html = match (params.locked_remaining_seconds, params.failed) {
        (Some(remaining_seconds), _) => {
            let locked_text = escape_html(&message(&pack, "remote.loginLocked").replace("{{seconds}}", &remaining_seconds.to_string()));
            format!(r#"<p class="status status-locked">{locked_text}</p>"#)
        }
        (None, true) => {
            let failed_text = escape_html(&message(&pack, "remote.loginFailed"));
            format!(r#"<p class="status status-error">{failed_text}</p>"#)
        }
        (None, false) => String::new(),
    };

    let insecure_html = if params.insecure {
        let insecure_notice = escape_html(&message(&pack, "remote.loginInsecureNotice"));
        format!(r#"<p class="notice">{insecure_notice}</p>"#)
    } else {
        String::new()
    };

    let disabled_attr = if params.locked_remaining_seconds.is_some() {
        " disabled"
    } else {
        ""
    };

    format!(
        r#"<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>{LOGIN_PAGE_STYLE}</style>
</head>
<body>
<main class="card">
<h1>{title}</h1>
{status_html}
<form method="post" action="{REMOTE_LOGIN_PATH}">
<label for="password">{password_label}</label>
<input id="password" name="password" type="password" autocomplete="current-password" autofocus required{disabled_attr}>
<button type="submit"{disabled_attr}>{submit_label}</button>
</form>
{insecure_html}
</main>
</body>
</html>"#,
        lang = escape_html(&pack.id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 지원하지_않는_언어는_영어로_대체된다() {
        let html = render(LoginPageParams {
            language: "fr",
            failed: false,
            locked_remaining_seconds: None,
            insecure: false,
        });
        assert!(html.contains("TAIDE Remote Access"));
    }

    #[test]
    fn 한국어_로케일_문자열을_사용한다() {
        let html = render(LoginPageParams {
            language: "ko",
            failed: false,
            locked_remaining_seconds: None,
            insecure: false,
        });
        assert!(html.contains("TAIDE 원격 접속"));
    }

    #[test]
    fn 실패_상태에서_오류_문구를_보여준다() {
        let html = render(LoginPageParams {
            language: "en",
            failed: true,
            locked_remaining_seconds: None,
            insecure: false,
        });
        assert!(html.contains("Incorrect password"));
    }

    #[test]
    fn 잠금_상태에서_남은_시간을_보여주고_입력을_비활성화한다() {
        let html = render(LoginPageParams {
            language: "en",
            failed: false,
            locked_remaining_seconds: Some(42),
            insecure: false,
        });
        assert!(html.contains("42"));
        assert!(html.contains("disabled"));
    }

    #[test]
    fn 비암호화_고지는_insecure일_때만_보인다() {
        let secure_html = render(LoginPageParams {
            language: "en",
            failed: false,
            locked_remaining_seconds: None,
            insecure: false,
        });
        let insecure_html = render(LoginPageParams {
            language: "en",
            failed: false,
            locked_remaining_seconds: None,
            insecure: true,
        });
        assert!(!secure_html.contains("not encrypted"));
        assert!(insecure_html.contains("not encrypted"));
    }

    #[test]
    fn 응답에_인라인_스크립트가_없다() {
        let html = render(LoginPageParams {
            language: "en",
            failed: false,
            locked_remaining_seconds: None,
            insecure: false,
        });
        assert!(!html.contains("<script"));
    }
}

use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{Config, Manager, Runtime, Url, WebviewWindowBuilder};

use super::external_url::validate_external_url;

/// URL schemes the app's own documents are served under, plus the two schemes a same-origin
/// document legitimately navigates a frame to on its own:
///
/// - `tauri` — the production app origin on macOS/Linux (`tauri://localhost`).
/// - `asset` — the `protocol-asset` scheme `infra::asset_protocol` answers (root-scoped media).
/// - `about` — the URL a `srcdoc` iframe reports (`about:srcdoc`) and the blank document a frame
///   starts on (`about:blank`); denying it would break every iframe before its real load starts.
/// - `blob` — `features/preview/html-preview.tsx` loads its sandboxed preview from a
///   `URL.createObjectURL` blob, which the CSP already allows (`frame-src 'self' blob:`). A blob
///   URL can only be minted by same-origin script, so allowing the scheme grants no new reach.
const ALLOWED_NAVIGATION_SCHEMES: &[&str] = &["tauri", "asset", "about", "blob"];

/// Hosts that mean "this app" when the runtime serves it over `http(s)` instead of a custom
/// scheme — Windows' production origin (`http://tauri.localhost`), the asset protocol's Windows
/// form, and Tauri's IPC host. Matched only for `http`/`https`; the same names under any other
/// scheme are not the app.
const ALLOWED_NAVIGATION_HOSTS: &[&str] = &["tauri.localhost", "asset.localhost", "ipc.localhost"];

/// The single policy both app windows enforce on every navigation their webview attempts: only
/// the app's own origins (and, in a dev build, the Vite dev server the webview is actually
/// loaded from) may become a document. Everything else — an `http(s)` site, a `javascript:` URL,
/// a `file:` path — is denied here and, when it is a user-meant external link, re-routed to the
/// OS browser by [`open_new_window_externally`].
///
/// This is the Rust half of "external URLs always leave the app window"
/// (`docs/acknowledge/2026-09-04-usability-batch3-contract.md` §B.2); the frontend half
/// (`entities/system/external-url.ts` and the anchor/terminal call sites) is what actually opens
/// those links. This guard exists so that a JS path that *escapes* those call sites still cannot
/// turn an app window into a browser tab.
///
/// **The webview's navigation policy does not distinguish frames** — wry 0.55.1's
/// `wkwebview/navigation.rs` hands the delegate only the URL string, so an iframe's navigation is
/// judged by exactly this function too. That is why `about:` and `blob:` must be allowed: they are
/// the schemes the HTML preview iframe legitimately loads.
pub fn is_navigation_allowed(url: &Url, dev_url: Option<&Url>) -> bool {
    if ALLOWED_NAVIGATION_SCHEMES.contains(&url.scheme()) {
        return true;
    }
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some_and(|host| ALLOWED_NAVIGATION_HOSTS.contains(&host)) {
        return true;
    }
    dev_url.is_some_and(|dev| url.origin() == dev.origin())
}

/// Answers every `window.open()` (and every target-`_blank`-style new window request the webview
/// raises) by handing the URL to the OS browser and denying the window — the app never grows a
/// second, chrome-less webview it does not own.
///
/// The URL passes through [`validate_external_url`] first, so exactly the same whitelist the
/// `system_open_external_url` command enforces (http(s) only, no control/spoofing characters,
/// no userinfo) applies here; a URL that fails it is dropped with a warning rather
/// than handed to the OS opener. Either way the response is [`NewWindowResponse::Deny`].
pub fn open_new_window_externally<R: Runtime>(url: Url) -> NewWindowResponse<R> {
    match validate_external_url(url.as_str()) {
        Ok(validated) => {
            if let Err(error) = tauri_plugin_opener::open_url(&validated, None::<&str>) {
                log::warn!("새 창 요청을 외부 브라우저로 넘기지 못했습니다: url={validated} error={error}");
            }
        }
        Err(error) => log::warn!("새 창 요청 URL 이 외부 열기 조건을 만족하지 않습니다: url={url} error={error}"),
    }
    NewWindowResponse::Deny
}

/// The dev server origin [`is_navigation_allowed`] additionally trusts, or `None` in a release
/// build. `build.dev_url` stays embedded in the generated config even for release binaries, but
/// Tauri only ever navigates to it under `cfg(dev)` (`tauri::manager::AppManager::get_app_url`) —
/// so trusting it in a shipped app would widen the allowlist to a localhost port nothing loads.
fn dev_server_url(config: &Config) -> Option<Url> {
    if cfg!(dev) {
        config.build.dev_url.clone()
    } else {
        None
    }
}

/// Attaches the two guards above to a webview window builder. Every window the app creates —
/// `lib.rs`'s main window and `domain::window::commands::open_auxiliary_window`'s `editor-<n>`
/// windows — goes through this one function so the two can never drift apart.
pub fn apply_navigation_guard<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
    config: &Config,
) -> WebviewWindowBuilder<'a, R, M> {
    let dev_url = dev_server_url(config);
    builder
        .on_navigation(move |url| {
            if is_navigation_allowed(url, dev_url.as_ref()) {
                return true;
            }
            log::warn!("앱 오리진 밖 웹뷰 네비게이션을 차단했습니다: url={url}");
            false
        })
        .on_new_window(|url: Url, _features: NewWindowFeatures| open_new_window_externally(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(url: &str) -> Url {
        Url::parse(url).expect("유효한 URL")
    }

    #[test]
    fn 앱_오리진_네비게이션은_허용된다() {
        assert!(is_navigation_allowed(&parse("tauri://localhost/index.html"), None));
        assert!(is_navigation_allowed(&parse("http://tauri.localhost/"), None));
        assert!(is_navigation_allowed(&parse("asset://localhost/x"), None));
        assert!(is_navigation_allowed(&parse("http://asset.localhost/x"), None));
        assert!(is_navigation_allowed(&parse("http://ipc.localhost/"), None));
    }

    #[test]
    fn 프리뷰_iframe_이_쓰는_about_과_blob_은_허용된다() {
        assert!(is_navigation_allowed(&parse("about:srcdoc"), None));
        assert!(is_navigation_allowed(&parse("about:blank"), None));
        assert!(is_navigation_allowed(
            &parse("blob:tauri://localhost/1f0c4a2e-0000-4000-8000-000000000000"),
            None
        ));
    }

    #[test]
    fn dev_서버와_같은_오리진만_추가로_허용된다() {
        let dev_url = parse("http://localhost:5173");
        assert!(is_navigation_allowed(
            &parse("http://localhost:5173/index.html?projectId=prj-1"),
            Some(&dev_url)
        ));
        assert!(!is_navigation_allowed(&parse("http://localhost:9999"), Some(&dev_url)));
        assert!(!is_navigation_allowed(&parse("https://localhost:5173/"), Some(&dev_url)));
        assert!(!is_navigation_allowed(&parse("http://localhost:5173/"), None));
    }

    #[test]
    fn 외부_사이트와_위험_스킴은_거부된다() {
        assert!(!is_navigation_allowed(&parse("https://example.com"), None));
        assert!(!is_navigation_allowed(&parse("http://example.com/path"), None));
        assert!(!is_navigation_allowed(&parse("javascript:alert(1)"), None));
        assert!(!is_navigation_allowed(&parse("file:///etc/passwd"), None));
        assert!(!is_navigation_allowed(&parse("data:text/html,<script>alert(1)</script>"), None));
    }

    #[test]
    fn 허용_호스트_이름은_http_스킴에서만_유효하다() {
        assert!(!is_navigation_allowed(&parse("ftp://tauri.localhost/"), None));
    }
}

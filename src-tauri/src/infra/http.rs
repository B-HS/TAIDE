use std::sync::OnceLock;
use std::time::Duration;

const CONNECT_TIMEOUT_SECS: u64 = 10;
const API_REQUEST_TIMEOUT_SECS: u64 = 60;

/// Which outbound-traffic shape a cached [`reqwest::Client`] is tuned for. A plain Rust enum (not
/// the TS `as const`-union convention — this never crosses the IPC boundary, it is purely an
/// in-process cache key) so the match in [`build_client`]/[`outbound_http_client`] stays
/// exhaustive as profiles are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpClientProfile {
    /// AI provider calls (`domain::ai`) and GitHub sync (`domain::sync`) — bounded request/response
    /// bodies (a single completion/diff payload), so a hard end-to-end request timeout is safe and
    /// desirable (an unresponsive provider must not hang the calling command indefinitely).
    Api,
    /// LSP server binary downloads (`domain::lsp::commands::run_download_install`) — response
    /// bodies can be tens of megabytes over a slow connection, so only the initial connect is
    /// time-bounded; the download loop enforces its own responsiveness via the caller-supplied
    /// `AtomicBool` cancellation flag (`lsp_install::download_to_file`), not a wall-clock timeout
    /// that would otherwise abort a legitimately slow-but-progressing transfer.
    Download,
}

fn build_client(profile: HttpClientProfile) -> reqwest::Client {
    let builder = reqwest::Client::builder().connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS));
    let builder = match profile {
        HttpClientProfile::Api => builder.timeout(Duration::from_secs(API_REQUEST_TIMEOUT_SECS)),
        HttpClientProfile::Download => builder,
    };
    builder.build().unwrap_or_default()
}

static API_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static DOWNLOAD_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Returns the process-wide singleton [`reqwest::Client`] for the given profile, building it
/// lazily on first use and reusing it on every later call. Replaces the previous
/// `create_outbound_http_client`, which built (and immediately discarded) a brand-new client — and
/// therefore a brand-new connection pool plus a fresh TLS handshake — on every single AI inline
/// completion, inline edit, commit-message, and sync request; that cost was directly on the
/// keystroke-triggered inline-completion path (`ai_inline_complete`, called on every typed
/// character), where it was fully user-visible latency. `reqwest::Client` is `Arc`-backed, so
/// `.clone()` here is cheap and shares the pool with every other call site.
pub fn outbound_http_client(profile: HttpClientProfile) -> reqwest::Client {
    let cell = match profile {
        HttpClientProfile::Api => &API_CLIENT,
        HttpClientProfile::Download => &DOWNLOAD_CLIENT,
    };
    cell.get_or_init(|| build_client(profile)).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 같은_프로필을_반복_요청하면_한_번만_초기화되고_이후에는_캐시된_클라이언트를_반환한다() {
        let _first = outbound_http_client(HttpClientProfile::Api);
        assert!(API_CLIENT.get().is_some(), "첫 호출 후에는 OnceLock이 채워져 있어야 한다");

        let before = format!("{:?}", API_CLIENT.get().unwrap());
        let _second = outbound_http_client(HttpClientProfile::Api);
        let after = format!("{:?}", API_CLIENT.get().unwrap());
        assert_eq!(before, after, "두 번째 호출이 새 클라이언트로 셀을 다시 채우지 않아야 한다");
    }

    /// `reqwest::Client` doesn't expose its internal settings (timeouts included) via a public
    /// getter, so this asserts only that the two profiles' `Debug` output differs — the `Api`
    /// profile's extra whole-request timeout field is expected to change that output without this
    /// test depending on `reqwest`'s internal field names.
    #[test]
    fn api와_download_프로필은_서로_다르게_설정된_클라이언트를_만든다() {
        let api = build_client(HttpClientProfile::Api);
        let download = build_client(HttpClientProfile::Download);

        assert_ne!(
            format!("{api:?}"),
            format!("{download:?}"),
            "api 프로필의 전체 요청 타임아웃이 download 프로필과 다른 클라이언트 설정을 만들어야 한다"
        );
    }
}

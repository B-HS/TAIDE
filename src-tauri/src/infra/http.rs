use std::time::Duration;

const HTTP_CLIENT_CONNECT_TIMEOUT_SECS: u64 = 10;
const HTTP_CLIENT_REQUEST_TIMEOUT_SECS: u64 = 60;

/// Shared client factory for every outbound AI-provider / GitHub-sync HTTP call (`domain::ai`,
/// `domain::sync`) — the only egress paths in the app, since all external HTTP goes through Rust.
/// `reqwest::Client::new()` has no timeout at all, so an unresponsive provider/GitHub endpoint (or
/// a half-open TCP connection) would otherwise hang the calling command indefinitely; several of
/// those commands hold `AppState::begin_mutation`'s guard while awaiting the response, which would
/// also block every other mutation command for as long as the hang lasts.
pub fn create_outbound_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CLIENT_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(HTTP_CLIENT_REQUEST_TIMEOUT_SECS))
        .build()
        .unwrap_or_default()
}

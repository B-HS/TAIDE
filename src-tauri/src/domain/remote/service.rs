use crate::infra::crypto::constant_time_eq;
use sha2::{Digest, Sha256};

use super::types::{REMOTE_LINK_TOKEN_QUERY_KEY, REMOTE_LOOPBACK_HOSTNAMES, REMOTE_PASSWORD_MIN_LEN};

fn generate_opaque_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn generate_link_token() -> String {
    generate_opaque_token()
}

pub fn generate_session_token() -> String {
    generate_opaque_token()
}

pub fn generate_login_nonce() -> String {
    generate_opaque_token()
}

pub fn digest_bytes(token: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hasher.finalize().to_vec()
}

pub fn digest_hex(token: &str) -> String {
    digest_bytes(token).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Single-owner wildcard-prefix syntax for a `remote_allowed_hosts` entry (RFC 6125 single-label
/// wildcard, matched by [`host_matches_allowed_entry`]) — `settings::service::is_valid_allowed_host`
/// (sanitizing user input) and this module's own matcher/link-formatting callers all read the same
/// constant instead of each hardcoding `"*."`.
pub const ALLOWED_HOST_WILDCARD_PREFIX: &str = "*.";

/// Whether `entry` uses the wildcard-prefix syntax owned by [`ALLOWED_HOST_WILDCARD_PREFIX`].
pub fn is_wildcard_entry(entry: &str) -> bool {
    entry.starts_with(ALLOWED_HOST_WILDCARD_PREFIX)
}

pub fn is_allowed_origin(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    let Some(host) = host else { return false };
    origin_matches_host(origin, host)
}

fn origin_matches_host(origin: &str, host: &str) -> bool {
    origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .is_some_and(|rest| rest == host)
}

/// Splits an HTTP `Host` header into `(hostname, port)`, lower-casing the
/// hostname for case-insensitive comparison. Handles a bracketed IPv6
/// literal (`[::1]:51301`) as well as the ordinary `hostname:port` form. A
/// header with no port (`hostname` alone) yields `port: None` — this server
/// never binds a default HTTP/HTTPS port, so a portless header can never be
/// this server's own loopback traffic and is rejected by
/// [`is_allowed_host`]'s loopback branch by construction. A registered
/// `allowed_hosts` entry has no such requirement (see [`is_allowed_host`]),
/// so a portless header there is exactly the expected shape for a tunnel
/// terminating TLS on the default `443` port.
fn split_host_header(host: &str) -> (String, Option<u32>) {
    if let Some(rest) = host.strip_prefix('[') {
        let Some((address, remainder)) = rest.split_once(']') else {
            return (host.to_ascii_lowercase(), None);
        };
        let port = remainder.strip_prefix(':').and_then(|value| value.parse::<u32>().ok());
        return (address.to_ascii_lowercase(), port);
    }
    match host.rsplit_once(':') {
        Some((hostname, port_str)) => match port_str.parse::<u32>() {
            Ok(port) => (hostname.to_ascii_lowercase(), Some(port)),
            Err(_) => (host.to_ascii_lowercase(), None),
        },
        None => (host.to_ascii_lowercase(), None),
    }
}

/// Extracts just the hostname portion of a `Host` header (see
/// [`split_host_header`]) — used by the insecure-connection notice, which
/// cares whether the request targets a loopback vs. a registered tunnel
/// hostname but not the port.
pub fn host_header_hostname(host: &str) -> String {
    split_host_header(host).0
}

pub fn is_loopback_hostname(hostname: &str) -> bool {
    let lower = hostname.to_ascii_lowercase();
    REMOTE_LOOPBACK_HOSTNAMES.contains(&lower.as_str())
}

/// Matches a lower-cased request hostname against one `Settings::remote_allowed_hosts` entry. A
/// `*.` prefix on `entry` matches exactly one leading DNS label of `hostname` — RFC 6125 wildcard
/// semantics — via `split_once('.')`, which structurally can only ever peel off one label,
/// deliberately excluding the base domain itself (`*.example.com` matches `foo.example.com` but
/// not `example.com`, and not `a.b.example.com` either — that hostname's first label is `a`, whose
/// remainder `b.example.com` still isn't `example.com`).
///
/// This is structurally immune to the classic `ends_with`-style wildcard bug, where a naive
/// `hostname.ends_with(&format!(".{suffix}"))` check (or worse, `hostname.ends_with(suffix)`)
/// would wrongly accept a look-alike hostname that shares a byte suffix with the registered
/// domain but not an actual label boundary — e.g. `evil-trycloudflare.com` or
/// `foo.eviltrycloudflare.com` against a `*.trycloudflare.com` entry. Splitting on the *first* dot
/// instead compares `hostname`'s remainder-after-first-label (`com`, and
/// `eviltrycloudflare.com` respectively) against the entry's `trycloudflare.com` suffix as whole
/// strings, which never matches either impostor. A bare `entry` (no `*.` prefix) still requires an
/// exact, case-insensitive match, unchanged from before.
pub fn host_matches_allowed_entry(hostname: &str, entry: &str) -> bool {
    match entry.strip_prefix(ALLOWED_HOST_WILDCARD_PREFIX) {
        Some(suffix) => hostname
            .split_once('.')
            .is_some_and(|(label, rest)| !label.is_empty() && rest.eq_ignore_ascii_case(suffix)),
        None => entry.eq_ignore_ascii_case(hostname),
    }
}

/// DNS-rebinding defense: the remote server only ever binds `127.0.0.1` on a
/// random port, so any request whose `Host` header doesn't resolve to a
/// loopback alias or a hostname the user explicitly registered
/// (`Settings::remote_allowed_hosts` — tunnels/port-forwarding) is rejected,
/// regardless of what `Origin` says (`Origin` and `Host` are checked
/// independently — see [`is_allowed_origin`]).
///
/// The two branches enforce the port differently. A loopback alias must also
/// get the port right (the server's actual bind port) — a request that gets
/// the hostname right but the port wrong cannot be this server's own
/// loopback traffic. A registered `allowed_hosts` entry is matched by
/// hostname alone, with **no port requirement**: it fronts a tunnel or
/// reverse proxy the user explicitly opted into, which commonly terminates
/// TLS on the default `443` port and forwards either a portless `Host`
/// header or an arbitrary external port — the server has no way to know or
/// pin that external port, and requiring it to equal the internal loopback
/// bind port would reject every such tunnel outright (see
/// `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6). This is
/// safe because `allowed_hosts` is itself the trust boundary: only a
/// hostname the user explicitly registered ever reaches this branch.
pub fn is_allowed_host(host: Option<&str>, allowed_hosts: &[String], bind_port: u32) -> bool {
    let Some(host) = host else { return false };
    let (hostname, port) = split_host_header(host);
    if is_loopback_hostname(&hostname) {
        return port == Some(bind_port);
    }
    allowed_hosts.iter().any(|allowed| host_matches_allowed_entry(&hostname, allowed))
}

/// Formats the URL handed back to [`remote_issue_link`]'s caller. Prefers
/// the first non-wildcard `Settings::remote_allowed_hosts` entry (over
/// `https`) so a link meant for a device that can only reach this app
/// through a registered tunnel actually resolves — `http://127.0.0.1:{port}`
/// is meaningless off the machine the server is bound on, and a `*.`-prefixed
/// entry is a pattern, not a hostname a browser could actually navigate to.
/// Falls back to the loopback URL when no host is registered, or when every
/// registered host is a wildcard pattern (unchanged legacy behavior —
/// same-machine/LAN access). See
/// `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6.
///
/// [`remote_issue_link`]: super::commands::remote_issue_link
pub fn format_issue_link_url(allowed_hosts: &[String], port: u32, token: &str) -> String {
    match allowed_hosts.iter().find(|host| !is_wildcard_entry(host)) {
        Some(host) => format!("https://{host}/?{REMOTE_LINK_TOKEN_QUERY_KEY}={token}"),
        None => format!("http://127.0.0.1:{port}/?{REMOTE_LINK_TOKEN_QUERY_KEY}={token}"),
    }
}

/// Trims `password` and checks the result meets `REMOTE_PASSWORD_MIN_LEN`
/// (measured in `chars`, i.e. Unicode scalar values, not bytes). Returns the
/// trimmed password on success so [`remote_set_password`]'s caller hashes
/// exactly what was validated instead of re-trimming a second time. The
/// login form deliberately does *not* trim its candidate — the stored value
/// is already trimmed, so an untrimmed login attempt against a trimmed
/// stored password fails safely rather than silently normalizing user input
/// at verification time too.
///
/// [`remote_set_password`]: super::commands::remote_set_password
pub fn validate_and_trim_password(password: &str) -> Option<&str> {
    let trimmed = password.trim();
    (trimmed.chars().count() >= REMOTE_PASSWORD_MIN_LEN).then_some(trimmed)
}

/// Hashes `password` with a freshly generated salt and returns the storable
/// `salt$digest` string (hex-encoded SHA-256 of `salt || password`). The salt
/// makes every call produce a different string even for the same password.
pub fn hash_password(password: &str) -> String {
    let salt = generate_opaque_token();
    let digest = digest_hex(&format!("{salt}{password}"));
    format!("{salt}${digest}")
}

/// Verifies `candidate` against a `salt$digest` string produced by
/// [`hash_password`] using a constant-time comparison. Malformed `stored`
/// values (missing the `$` separator) are rejected rather than panicking.
pub fn verify_password(stored: &str, candidate: &str) -> bool {
    let Some((salt, expected_digest)) = stored.split_once('$') else {
        return false;
    };
    let candidate_digest = digest_hex(&format!("{salt}{candidate}"));
    constant_time_eq(candidate_digest.as_bytes(), expected_digest.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 발급된_토큰의_다이제스트는_원문이_같으면_일치한다() {
        let token = generate_link_token();
        assert_eq!(digest_bytes(&token), digest_bytes(&token));
    }

    #[test]
    fn 다른_토큰의_다이제스트는_불일치한다() {
        let a = generate_link_token();
        let b = generate_link_token();
        assert_ne!(digest_bytes(&a), digest_bytes(&b));
    }

    #[test]
    fn 다이제스트_hex는_64자_소문자다() {
        let hex = digest_hex("sample-token");
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn origin이_없으면_허용한다() {
        assert!(is_allowed_origin(None, Some("127.0.0.1:53211")));
    }

    #[test]
    fn origin이_host와_같으면_허용한다() {
        assert!(is_allowed_origin(Some("http://127.0.0.1:53211"), Some("127.0.0.1:53211")));
    }

    #[test]
    fn origin이_host와_다르면_거부한다() {
        assert!(!is_allowed_origin(Some("http://evil.example"), Some("127.0.0.1:53211")));
    }

    #[test]
    fn origin은_있는데_host_헤더가_없으면_거부한다() {
        assert!(!is_allowed_origin(Some("http://127.0.0.1:53211"), None));
    }

    #[test]
    fn host_헤더가_없으면_허용목록_검사를_거부한다() {
        assert!(!is_allowed_host(None, &[], 53_211));
    }

    #[test]
    fn 루프백_호스트는_바인드_포트와_일치하면_허용한다() {
        assert!(is_allowed_host(Some("127.0.0.1:53211"), &[], 53_211));
        assert!(is_allowed_host(Some("localhost:53211"), &[], 53_211));
        assert!(is_allowed_host(Some("[::1]:53211"), &[], 53_211));
    }

    #[test]
    fn 포트가_바인드_포트와_다르면_루프백_호스트여도_거부한다() {
        assert!(!is_allowed_host(Some("127.0.0.1:9999"), &[], 53_211));
    }

    #[test]
    fn 포트가_없는_루프백_host_헤더는_거부한다() {
        assert!(!is_allowed_host(Some("127.0.0.1"), &[], 53_211));
    }

    #[test]
    fn 루프백_호스트는_등록목록에_들어있어도_포트_불일치면_거부한다() {
        let allowed = vec!["127.0.0.1".to_string()];
        assert!(
            !is_allowed_host(Some("127.0.0.1:9999"), &allowed, 53_211),
            "allowed_hosts 등재로 루프백의 포트 방어를 우회할 수 없어야 한다"
        );
    }

    #[test]
    fn 등록되지_않은_호스트명은_거부한다() {
        assert!(!is_allowed_host(Some("attacker.example:53211"), &[], 53_211));
    }

    #[test]
    fn settings에_등록된_호스트명은_대소문자_무관하게_허용한다() {
        let allowed = vec!["Tunnel.Example.Com".to_string()];
        assert!(is_allowed_host(Some("tunnel.example.com:53211"), &allowed, 53_211));
    }

    #[test]
    fn 등록된_호스트는_443_포트로_와도_바인드_포트와_무관하게_허용한다() {
        let allowed = vec!["tunnel.example.com".to_string()];
        assert!(is_allowed_host(Some("tunnel.example.com:443"), &allowed, 53_211));
    }

    #[test]
    fn 등록된_호스트는_임의_포트여도_허용한다() {
        let allowed = vec!["tunnel.example.com".to_string()];
        assert!(is_allowed_host(Some("tunnel.example.com:9999"), &allowed, 53_211));
    }

    #[test]
    fn 등록된_호스트는_포트가_없어도_허용한다() {
        let allowed = vec!["tunnel.example.com".to_string()];
        assert!(
            is_allowed_host(Some("tunnel.example.com"), &allowed, 53_211),
            "443 종단 터널은 포트 없는 Host 헤더를 보낼 수 있다"
        );
    }

    #[test]
    fn dns_rebinding_시도는_등록되지_않은_한_거부한다() {
        assert!(!is_allowed_host(Some("rebind.example:53211"), &[], 53_211));
    }

    #[test]
    fn 와일드카드_등록은_선두_한_레이블만_허용한다() {
        let allowed = vec!["*.trycloudflare.com".to_string()];
        assert!(is_allowed_host(Some("foo.trycloudflare.com:53211"), &allowed, 53_211));
    }

    #[test]
    fn 와일드카드_등록은_레이블이_두_개_이상이면_거부한다() {
        let allowed = vec!["*.trycloudflare.com".to_string()];
        assert!(!is_allowed_host(Some("a.b.trycloudflare.com:53211"), &allowed, 53_211));
    }

    #[test]
    fn 와일드카드_등록은_베이스_도메인_자체를_거부한다() {
        let allowed = vec!["*.trycloudflare.com".to_string()];
        assert!(
            !is_allowed_host(Some("trycloudflare.com:53211"), &allowed, 53_211),
            "*.example.com 은 example.com 자체를 포함하지 않아야 한다"
        );
    }

    #[test]
    fn 와일드카드_매칭은_대소문자를_구분하지_않는다() {
        let allowed = vec!["*.TryCloudflare.Com".to_string()];
        assert!(is_allowed_host(Some("foo.trycloudflare.com:53211"), &allowed, 53_211));
    }

    #[test]
    fn 와일드카드_등록은_유사_도메인을_거부한다() {
        let allowed = vec!["*.trycloudflare.com".to_string()];
        assert!(
            !is_allowed_host(Some("evil-trycloudflare.com:53211"), &allowed, 53_211),
            "레이블 경계가 없는 유사 도메인은 거부해야 한다"
        );
        assert!(
            !is_allowed_host(Some("foo.eviltrycloudflare.com:53211"), &allowed, 53_211),
            "ends_with 함정 형태의 유사 도메인은 거부해야 한다"
        );
    }

    #[test]
    fn 루프백_호스트는_와일드카드_등록이_있어도_포트_불일치면_거부한다() {
        let allowed = vec!["*.trycloudflare.com".to_string()];
        assert!(
            !is_allowed_host(Some("127.0.0.1:9999"), &allowed, 53_211),
            "와일드카드 등재로 루프백의 포트 방어를 우회할 수 없어야 한다"
        );
    }

    #[test]
    fn 루프백_판정은_대소문자를_구분하지_않는다() {
        assert!(is_loopback_hostname("LOCALHOST"));
        assert!(!is_loopback_hostname("tunnel.example.com"));
    }

    #[test]
    fn host_헤더_호스트명만_추출하면_포트가_빠진다() {
        assert_eq!(host_header_hostname("127.0.0.1:53211"), "127.0.0.1");
        assert_eq!(host_header_hostname("[::1]:53211"), "::1");
    }

    #[test]
    fn 등록된_호스트가_없으면_루프백_http_링크를_발급한다() {
        assert_eq!(format_issue_link_url(&[], 53_211, "tok123"), "http://127.0.0.1:53211/?t=tok123");
    }

    #[test]
    fn 등록된_호스트가_있으면_그_호스트로_https_링크를_발급한다() {
        let allowed = vec!["tunnel.example.com".to_string()];
        assert_eq!(
            format_issue_link_url(&allowed, 53_211, "tok123"),
            "https://tunnel.example.com/?t=tok123"
        );
    }

    #[test]
    fn 등록된_호스트가_여러_개면_첫_번째_호스트를_사용한다() {
        let allowed = vec!["first.example.com".to_string(), "second.example.com".to_string()];
        assert_eq!(
            format_issue_link_url(&allowed, 53_211, "tok123"),
            "https://first.example.com/?t=tok123"
        );
    }

    #[test]
    fn 첫_호스트가_와일드카드면_건너뛰고_다음_비와일드카드_호스트를_사용한다() {
        let allowed = vec!["*.trycloudflare.com".to_string(), "second.example.com".to_string()];
        assert_eq!(
            format_issue_link_url(&allowed, 53_211, "tok123"),
            "https://second.example.com/?t=tok123",
            "와일드카드 패턴은 브라우저가 실제로 열 수 있는 호스트가 아니므로 링크에 쓰지 않아야 한다"
        );
    }

    #[test]
    fn 등록된_호스트가_전부_와일드카드면_루프백_링크로_폴백한다() {
        let allowed = vec!["*.trycloudflare.com".to_string(), "*.ngrok.io".to_string()];
        assert_eq!(
            format_issue_link_url(&allowed, 53_211, "tok123"),
            "http://127.0.0.1:53211/?t=tok123"
        );
    }

    #[test]
    fn 최소_길이_미만_비밀번호는_거부된다() {
        assert!(validate_and_trim_password("short7").is_none());
    }

    #[test]
    fn 최소_길이_이상_비밀번호는_트림되어_허용된다() {
        assert_eq!(validate_and_trim_password("  hunter22  "), Some("hunter22"));
    }

    #[test]
    fn 트림_전_길이만으로_최소_길이를_충족해도_트림_후_짧으면_거부된다() {
        assert!(validate_and_trim_password("   1234   ").is_none());
    }

    #[test]
    fn 공백만_있는_비밀번호는_거부된다() {
        assert!(validate_and_trim_password("                ").is_none());
    }

    #[test]
    fn 같은_비밀번호도_매번_다른_해시_문자열을_만든다() {
        let a = hash_password("hunter2");
        let b = hash_password("hunter2");
        assert_ne!(a, b);
    }

    #[test]
    fn 올바른_비밀번호는_검증에_성공한다() {
        let stored = hash_password("hunter2");
        assert!(verify_password(&stored, "hunter2"));
    }

    #[test]
    fn 틀린_비밀번호는_검증에_실패한다() {
        let stored = hash_password("hunter2");
        assert!(!verify_password(&stored, "wrong-password"));
    }

    #[test]
    fn 구분자가_없는_저장값은_검증에_실패한다() {
        assert!(!verify_password("not-a-valid-hash", "anything"));
    }
}

use serde::{Deserialize, Serialize};
use specta::Type;

pub const ALLOWED_HOST_WILDCARD_PREFIX: &str = "*.";

/// Minimum accepted length (in `chars`, after trimming) for a newly set
/// remote-access password. Only enforced on write (Tauri's `remote_set_password`) —
/// a password already stored below this length keeps working until the user
/// changes it.
pub const REMOTE_PASSWORD_MIN_LEN: usize = 8;

/// RFC 1035 §3.1 length limit of a whole hostname.
pub const DNS_HOSTNAME_MAX_LEN: usize = 253;

/// RFC 1035 §3.1 length limit of one hostname label.
pub const DNS_LABEL_MAX_LEN: usize = 63;

/// Accepts one hostname label: 1 to [`DNS_LABEL_MAX_LEN`] ASCII letters, digits or `-`,
/// never starting or ending with `-`.
pub fn is_valid_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= DNS_LABEL_MAX_LEN
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Accepts only a bare hostname or IPv4 literal for a `remote_allowed_hosts`
/// entry — no scheme, userinfo, path/query, whitespace, or port — with one
/// exception: a leading `*.` marks the entry as an RFC 6125 single-label
/// wildcard (`remote::service::host_matches_allowed_entry` is the matcher).
/// `remote::service::is_allowed_host` matches entries against the `Host`
/// header's hostname alone, and `remote::service::format_issue_link_url`
/// interpolates the first non-wildcard entry verbatim into `https://{host}/...`;
/// anything else either never matches (silently disabling the tunnel) or
/// corrupts the issued link (a scheme yields `https://https://...`, and
/// `real.example.com@attacker.example` would make a browser send the link's
/// one-time token to `attacker.example` as if `real.example.com` were
/// userinfo). See `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6.
///
/// The wildcard's remainder is validated the same way a bare hostname is,
/// plus one extra requirement: it must itself contain at least one more `.`
/// (i.e. two or more labels), so a bare `*` (no `*.` prefix, falls through to
/// the non-wildcard branch and fails there since `*` isn't a valid DNS label)
/// and `*.com` (single-label remainder) are both rejected — the latter would
/// otherwise register a wildcard broad enough to match almost any TLD-only
/// hostname. A wildcard elsewhere than the very front (`foo.*.com`) or fused
/// into a label (`*foo.com`) never matches the literal `*.` prefix check at
/// all, so both fall through to the non-wildcard branch and are rejected
/// there too (`*` isn't a valid DNS label).
pub fn is_valid_allowed_host(value: &str) -> bool {
    match value.strip_prefix(ALLOWED_HOST_WILDCARD_PREFIX) {
        Some(remainder) => {
            remainder.contains('.') && remainder.len() <= DNS_HOSTNAME_MAX_LEN && remainder.split('.').all(is_valid_dns_label)
        }
        None => !value.is_empty() && value.len() <= DNS_HOSTNAME_MAX_LEN && value.split('.').all(is_valid_dns_label),
    }
}

pub use taide_remote_wire::protocol::RemoteRequest;
pub use taide_remote_wire::REMOTE_OWNER_LABEL;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    pub running: bool,
    pub port: u32,
    pub client_count: u32,
    pub password_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLinkInfo {
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONGEST_LABEL: usize = 63;
    const LONGEST_HOSTNAME: usize = 253;

    #[test]
    fn 허용_호스트는_원본_ts_사례와_같게_판정한다() {
        assert!(is_valid_allowed_host("tunnel.example.com"));
        assert!(is_valid_allowed_host("*.trycloudflare.com"));
        assert!(!is_valid_allowed_host("*"));
        assert!(!is_valid_allowed_host("*.com"));
        assert!(!is_valid_allowed_host("foo.*.com"));
        assert!(!is_valid_allowed_host("*foo.com"));
    }

    #[test]
    fn 허용_호스트는_rfc_1035_길이와_하이픈과_문자_경계를_지킨다() {
        let label = "a".repeat(LONGEST_LABEL);
        assert!(is_valid_dns_label(&label));
        assert!(!is_valid_dns_label(&format!("{label}a")));
        assert!(!is_valid_dns_label(""));
        assert!(!is_valid_dns_label("-edge"));
        assert!(!is_valid_dns_label("edge-"));
        assert!(is_valid_dns_label("mid-dle"));
        let full_labels = ["a", "b", "c"].map(|letter| letter.repeat(LONGEST_LABEL));
        let used = full_labels.len() * (LONGEST_LABEL + 1);
        let hostname = format!("{}.{}", full_labels.join("."), "d".repeat(LONGEST_HOSTNAME - used));
        assert_eq!(hostname.len(), LONGEST_HOSTNAME);
        assert!(is_valid_allowed_host(&hostname));
        assert!(!is_valid_allowed_host(&format!("{hostname}d")));
        assert!(is_valid_allowed_host(&format!("*.{hostname}")));
        assert!(!is_valid_allowed_host(""));
        assert!(!is_valid_allowed_host("https://tunnel.example.com"));
        assert!(!is_valid_allowed_host("tunnel.example.com:8080"));
        assert!(!is_valid_allowed_host("real.example.com@attacker.example"));
        assert!(!is_valid_allowed_host("two words.example.com"));
        assert!(is_valid_allowed_host("192.168.0.10"));
    }

    #[test]
    fn 원격_비밀번호_최소_길이는_8자다() {
        assert_eq!(REMOTE_PASSWORD_MIN_LEN, 8);
    }
}

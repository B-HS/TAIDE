use crate::infra::crypto::constant_time_eq;
use sha2::{Digest, Sha256};

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

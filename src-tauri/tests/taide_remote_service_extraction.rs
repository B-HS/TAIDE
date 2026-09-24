use taide_lib::domain::remote::{service as legacy_service, types as legacy_types};

#[test]
fn remote_인증과_호스트_정책은_독립_crate와_기존_경로에서_같다() {
    let extracted_verify: fn(&str, &str) -> bool = taide_remote::service::verify_password;
    let legacy_verify: fn(&str, &str) -> bool = legacy_service::verify_password;
    assert!(std::ptr::fn_addr_eq(extracted_verify, legacy_verify));

    assert_eq!(taide_remote::types::REMOTE_PASSWORD_MIN_LEN, legacy_types::REMOTE_PASSWORD_MIN_LEN);
    assert_eq!(
        taide_remote::types::REMOTE_LOOPBACK_HOSTNAMES,
        legacy_types::REMOTE_LOOPBACK_HOSTNAMES
    );
    assert_eq!(
        taide_remote::types::REMOTE_LINK_TOKEN_QUERY_KEY,
        legacy_types::REMOTE_LINK_TOKEN_QUERY_KEY
    );

    let stored = taide_remote::service::hash_password("valid-password");
    assert!(legacy_service::verify_password(&stored, "valid-password"));
    assert!(!legacy_service::verify_password(&stored, "wrong-password"));
    assert!(!legacy_service::is_allowed_host(Some("rebind.example:53211"), &[], 53_211));
}

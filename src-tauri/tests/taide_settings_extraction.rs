use taide_lib::domain::remote::types::ALLOWED_HOST_WILDCARD_PREFIX as LEGACY_WILDCARD_PREFIX;
use taide_lib::domain::settings::service as legacy_service;
use taide_model::error::{AppErrorKind, AppResult};
use taide_model::settings::Settings;

#[test]
fn 설정_마이그레이션은_독립_crate와_기존_경로에서_같다() {
    let extracted: fn(&str) -> AppResult<Settings> = taide_settings::service::parse_settings_json;
    let legacy: fn(&str) -> AppResult<Settings> = legacy_service::parse_settings_json;
    assert!(std::ptr::fn_addr_eq(extracted, legacy));
    let default_json = serde_json::to_string(&Settings::default()).expect("기본 설정 직렬화");
    assert_eq!(
        extracted(&default_json).expect("기본 설정 파싱"),
        legacy(&default_json).expect("기존 경로 파싱")
    );
    assert_eq!(extracted("{").unwrap_err().kind(), AppErrorKind::InvalidArgument);
    assert_eq!(taide_model::remote::ALLOWED_HOST_WILDCARD_PREFIX, LEGACY_WILDCARD_PREFIX);
}

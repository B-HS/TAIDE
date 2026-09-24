use taide_lib::domain::locale::service as legacy_service;
use taide_model::locale::LocalePack;

#[test]
fn 번들_언어_카탈로그는_독립_crate와_기존_경로에서_같다() {
    let extracted: fn() -> LocalePack = taide_locale::service::builtin_en;
    let legacy: fn() -> LocalePack = legacy_service::builtin_en;
    assert!(std::ptr::fn_addr_eq(extracted, legacy));
    assert_eq!(extracted(), legacy());
    assert_eq!(taide_locale::service::builtin_ko(), legacy_service::builtin_ko());
    assert_eq!(taide_locale::service::builtin_ja(), legacy_service::builtin_ja());
    assert_eq!(
        taide_locale::service::required_message_keys(),
        legacy_service::required_message_keys()
    );
}

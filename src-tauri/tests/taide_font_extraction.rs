use taide_lib::domain::font::service as legacy_service;
use taide_model::font::FontFamily;

#[test]
fn 글꼴_서비스는_독립_crate와_기존_경로에서_같은_진입점을_쓴다() {
    let extracted: fn() -> Vec<FontFamily> = taide_font::service::list_families;
    let legacy: fn() -> Vec<FontFamily> = legacy_service::list_families;

    assert!(std::ptr::fn_addr_eq(extracted, legacy));
    assert_eq!(extracted(), legacy());
}

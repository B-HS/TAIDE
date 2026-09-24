use taide_lib::domain::theme::service as legacy_service;
use taide_model::theme::Theme;

#[test]
fn 번들_테마와_기존_공개_경로는_같은_서비스를_쓴다() {
    let extracted: fn() -> Vec<Theme> = taide_theme::service::bundled_themes;
    let legacy: fn() -> Vec<Theme> = legacy_service::bundled_themes;
    assert!(std::ptr::fn_addr_eq(extracted, legacy));
    assert_eq!(extracted(), legacy());
    assert_eq!(taide_theme::service::builtin_dark(), legacy_service::builtin_dark());
    assert_eq!(taide_theme::service::builtin_light(), legacy_service::builtin_light());
    assert_eq!(taide_theme::service::required_color_keys(), legacy_service::required_color_keys());
}

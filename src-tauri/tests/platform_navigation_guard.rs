use taide_lib::infra::navigation_guard as legacy_navigation_guard;
use taide_lib::platform::navigation_guard;
use tauri::Url;

#[test]
fn 웹뷰_네비게이션_정책은_platform_경계와_기존_infra_경계에서_동일하다() {
    let app_url = Url::parse("tauri://localhost/index.html").unwrap();
    let external_url = Url::parse("https://example.com").unwrap();

    assert!(navigation_guard::is_navigation_allowed(&app_url, None));
    assert!(!navigation_guard::is_navigation_allowed(&external_url, None));
    assert_eq!(
        navigation_guard::is_navigation_allowed(&app_url, None),
        legacy_navigation_guard::is_navigation_allowed(&app_url, None)
    );
    assert_eq!(
        navigation_guard::is_navigation_allowed(&external_url, None),
        legacy_navigation_guard::is_navigation_allowed(&external_url, None)
    );
}

#[test]
fn 메인과_보조_창은_같은_platform_네비게이션_가드를_부착한다() {
    let app_source = include_str!("../src/lib.rs");
    let window_source = include_str!("../src/domain/window/commands.rs");

    assert!(app_source.contains("platform::navigation_guard::apply_navigation_guard("));
    assert!(window_source.contains("use crate::platform::navigation_guard;"));
    assert!(window_source.contains("navigation_guard::apply_navigation_guard("));
}

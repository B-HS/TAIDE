use taide_lib::domain::remote::{login_page as legacy_login_page, types as legacy_types};

#[test]
fn remote_로그인_페이지는_독립_crate와_기존_경로에서_같다() {
    let extracted_html = taide_remote::login_page::render(taide_remote::login_page::LoginPageParams {
        language: "ko",
        failed: false,
        link_expired: false,
        locked_remaining_seconds: Some(42),
        insecure: true,
    });
    let legacy_html = legacy_login_page::render(legacy_login_page::LoginPageParams {
        language: "ko",
        failed: false,
        link_expired: false,
        locked_remaining_seconds: Some(42),
        insecure: true,
    });

    assert_eq!(extracted_html, legacy_html);
    assert_eq!(taide_remote::login_page::LOGIN_PAGE_CSP, legacy_login_page::LOGIN_PAGE_CSP);
    assert!(taide_remote::login_page::LOGIN_PAGE_CSP.contains("script-src 'none'"));
    assert!(taide_remote::login_page::LOGIN_PAGE_CSP.contains("connect-src 'none'"));
    assert_eq!(taide_remote::types::REMOTE_LOGIN_PATH, legacy_types::REMOTE_LOGIN_PATH);
    assert!(extracted_html.contains("action=\"/__taide/login\""));
}

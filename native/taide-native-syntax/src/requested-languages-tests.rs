use super::{CORE_LANGUAGE_IDS, RequestedLanguages, is_bundled_language};

#[test]
fn 처음에는_json_jsonc_markdown만_요청돼_있다() {
    let languages = RequestedLanguages::default();
    assert_eq!(languages.ids(), CORE_LANGUAGE_IDS);
    assert!(languages.contains("markdown"));
    assert!(!languages.contains("rust"));
}

#[test]
fn 번들_언어를_처음_요청할_때만_집합이_커지고_줄어들지_않는다() {
    let mut languages = RequestedLanguages::default();
    assert!(languages.request("rust"));
    assert!(!languages.request("rust"));
    assert!(!languages.request("json"));
    assert!(languages.request("typescriptreact"));
    assert_eq!(
        languages.ids(),
        ["json", "jsonc", "markdown", "rust", "typescriptreact"]
    );
}

#[test]
fn 번들에_없는_언어는_요청해도_집합에_들어가지_않는다() {
    let mut languages = RequestedLanguages::default();
    for language_id in ["plaintext", "", "csharp", "Rust", "tsx"] {
        assert!(!is_bundled_language(language_id), "{language_id}");
        assert!(!languages.request(language_id), "{language_id}");
    }
    assert_eq!(languages.ids(), CORE_LANGUAGE_IDS);
    assert!(is_bundled_language("heex"));
}

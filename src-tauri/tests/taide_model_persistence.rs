use std::collections::BTreeMap;

use taide_lib::domain::locale::types::{LocalePack, LocaleSummary, ResolvedLocale};
use taide_lib::domain::snippet::types::{SnippetEntry, SnippetFile, SnippetMap, SnippetStringOrList};
use taide_lib::domain::theme::types::{ResolvedTheme, SyntaxStyle, Theme, ThemeSummary, ThemeType, TokenColorRule, TokenColorSettings};

#[test]
fn 구버전_언어팩의_기본값과_공개_타입을_유지한다() {
    let legacy = serde_json::json!({ "version": 1, "id": "custom", "name": "Custom" });
    let pack: taide_model::locale::LocalePack = serde_json::from_value(legacy).expect("구버전 언어팩");
    let facade: LocalePack = pack.clone();
    assert_eq!(facade.extends, None);
    assert!(facade.messages.is_empty());
    assert_eq!(
        serde_json::to_value(&pack).expect("언어팩 직렬화")["messages"],
        serde_json::json!({})
    );

    let _: taide_model::locale::LocaleSummary = LocaleSummary {
        id: "custom".into(),
        name: "Custom".into(),
        builtin: false,
    };
    let resolved: taide_model::locale::ResolvedLocale = serde_json::from_value(serde_json::json!({
        "id": "custom", "name": "Custom", "messages": {}
    }))
    .expect("warnings 없는 기존 언어팩");
    let _: ResolvedLocale = resolved.clone();
    assert!(serde_json::to_value(resolved)
        .expect("해석된 언어팩 직렬화")
        .get("warnings")
        .is_none());
}

#[test]
fn 스니펫의_기존_문자열_배열과_선택적_필드를_유지한다() {
    let wire = serde_json::json!({ "prefix": "log", "body": ["console.log($1)"] });
    let entry: taide_model::snippet::SnippetEntry = serde_json::from_value(wire.clone()).expect("기존 스니펫");
    let facade: SnippetEntry = entry.clone();
    assert_eq!(facade.prefix, SnippetStringOrList::Single("log".into()));
    assert_eq!(facade.body, SnippetStringOrList::Multiple(vec!["console.log($1)".into()]));
    assert_eq!(serde_json::to_value(&entry).expect("스니펫 직렬화"), wire);

    let snippets: taide_model::snippet::SnippetMap = BTreeMap::from([("Log".into(), entry)]);
    let _: SnippetMap = snippets.clone();
    let file = SnippetFile {
        file_name: "javascript.json".into(),
        snippets,
    };
    let model_file: taide_model::snippet::SnippetFile = file;
    assert_eq!(
        serde_json::to_value(model_file).expect("스니펫 파일 직렬화")["fileName"],
        "javascript.json"
    );
}

#[test]
fn 구버전_테마의_기본값과_type_필드를_유지한다() {
    let legacy = serde_json::json!({ "version": 1, "id": "custom", "name": "Custom", "type": "dark" });
    let theme: taide_model::theme::Theme = serde_json::from_value(legacy).expect("구버전 테마");
    let facade: Theme = theme.clone();
    assert_eq!(facade.theme_type, ThemeType::Dark);
    assert!(facade.colors.is_empty());
    assert!(facade.syntax.is_empty());
    assert_eq!(facade.token_colors, None);
    let saved = serde_json::to_value(theme).expect("테마 직렬화");
    assert_eq!(saved["type"], "dark");
    assert!(saved.get("themeType").is_none());
    assert!(saved.get("tokenColors").is_none());

    let _: taide_model::theme::ThemeType = ThemeType::Light;
    let _: taide_model::theme::SyntaxStyle = SyntaxStyle {
        fg: "#fff".into(),
        bold: false,
        italic: false,
    };
    let _: taide_model::theme::TokenColorSettings = TokenColorSettings {
        foreground: None,
        background: None,
        font_style: None,
    };
    let _: taide_model::theme::TokenColorRule = TokenColorRule {
        scope: vec![],
        settings: TokenColorSettings {
            foreground: None,
            background: None,
            font_style: None,
        },
    };
    let _: taide_model::theme::ThemeSummary = ThemeSummary {
        id: "custom".into(),
        name: "Custom".into(),
        theme_type: ThemeType::Dark,
        builtin: false,
    };
    let _: taide_model::theme::ResolvedTheme = ResolvedTheme {
        id: "custom".into(),
        name: "Custom".into(),
        theme_type: ThemeType::Dark,
        colors: BTreeMap::new(),
        syntax: BTreeMap::new(),
        terminal: BTreeMap::new(),
        token_colors: None,
        syntax_overrides: vec![],
        warnings: vec![],
        author: None,
        license: None,
        source: None,
    };
}

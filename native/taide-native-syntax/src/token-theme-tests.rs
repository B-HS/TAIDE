use serde_json::{Value, json};
use taide_model::theme::ResolvedTheme;

use super::{TokenTheme, TokenThemeError};
use crate::theme_settings::{ThemeSetting, ThemeStyle};

fn resolved(overrides: Value) -> ResolvedTheme {
    let mut theme = json!({
        "id": "synthetic",
        "name": "Synthetic",
        "type": "dark",
        "colors": { "editor.foreground": "#d4d4d4", "editor.background": "#1e1e1e" },
        "syntax": {},
        "terminal": {},
    });
    theme
        .as_object_mut()
        .unwrap()
        .extend(overrides.as_object().unwrap().clone());
    serde_json::from_value(theme).unwrap()
}

fn rule(scopes: &[&str], foreground: &str, font_style: Option<&str>) -> ThemeSetting {
    ThemeSetting {
        scope: Some(scopes.iter().map(|scope| (*scope).to_owned()).collect()),
        settings: Some(ThemeStyle {
            foreground: Some(foreground.to_owned()),
            background: None,
            font_style: font_style.map(str::to_owned),
        }),
    }
}

fn global(foreground: &str, background: &str) -> ThemeSetting {
    ThemeSetting {
        scope: None,
        settings: Some(ThemeStyle {
            foreground: Some(foreground.to_owned()),
            background: Some(background.to_owned()),
            font_style: None,
        }),
    }
}

#[test]
fn token_colors가_없으면_먼저_나온_syntax_토큰이_겹치는_scope를_차지한다() {
    let theme = TokenTheme::from_resolved(&resolved(json!({
        "syntax": {
            "method": { "fg": "#222222", "italic": true },
            "function": { "fg": "#111111", "bold": true, "italic": true },
            "tag": { "fg": "#333333" },
        },
    })))
    .unwrap();
    assert_eq!(
        theme.settings(),
        &[
            global("#d4d4d4", "#1e1e1e"),
            rule(
                &["entity.name.function", "support.function"],
                "#111111",
                Some("bold italic")
            ),
            rule(
                &["entity.name.function.member", "meta.function-call"],
                "#222222",
                Some("italic")
            ),
            rule(&["entity.name.tag"], "#333333", None),
            rule(&["taideSemantic.function"], "#111111", Some("bold italic")),
            rule(&["taideSemantic.method"], "#222222", Some("italic")),
        ]
    );
}

#[test]
fn token_colors가_있으면_syntax_overrides에_든_토큰만_덧붙이고_빈_설정값은_뺀다() {
    let theme = TokenTheme::from_resolved(&resolved(json!({
        "syntax": {
            "comment": { "fg": "#444444" },
            "docComment": { "fg": "#555555" },
            "tag": { "fg": "not-a-color" },
        },
        "tokenColors": [
            { "scope": ["keyword"], "settings": { "foreground": "#666666", "fontStyle": "" } },
        ],
        "syntaxOverrides": ["docComment", "notASyntaxToken"],
    })))
    .unwrap();
    assert_eq!(
        theme.settings(),
        &[
            global("#d4d4d4", "#1e1e1e"),
            rule(&["keyword"], "#666666", None),
            rule(&["comment.block.documentation", "comment"], "#555555", None),
            rule(&["taideSemantic.comment"], "#444444", None),
        ]
    );
}

#[test]
fn 샵으로_시작하지_않는_색은_나온_순서대로_자리표시_색으로_바꾼다() {
    let theme = TokenTheme::from_resolved(&resolved(json!({
        "colors": { "editor.foreground": "white" },
        "tokenColors": [
            { "scope": ["a"], "settings": { "foreground": "red", "background": "white" } },
            { "scope": ["b"], "settings": { "foreground": "red" } },
        ],
    })))
    .unwrap();
    assert_eq!(
        theme.settings(),
        &[
            global("#00000001", "#1e1e1e"),
            ThemeSetting {
                scope: Some(vec!["a".to_owned()]),
                settings: Some(ThemeStyle {
                    foreground: Some("#00000002".to_owned()),
                    background: Some("#00000001".to_owned()),
                    font_style: None,
                }),
            },
            rule(&["b"], "#00000002", None),
        ]
    );
}

#[test]
fn 쓰이는_syntax_색이나_monaco가_읽지_못하는_색이_있으면_테마를_거절한다() {
    let invalid_syntax = TokenTheme::from_resolved(&resolved(json!({
        "syntax": { "keyword": { "fg": "#abc" } },
    })));
    assert_eq!(
        invalid_syntax,
        Err(TokenThemeError::InvalidSyntaxColor("#abc".to_owned()))
    );
    let illegal_rule = TokenTheme::from_resolved(&resolved(json!({
        "tokenColors": [
            { "scope": ["keyword"], "settings": { "foreground": "#0083080" } },
        ],
    })));
    assert_eq!(
        illegal_rule,
        Err(TokenThemeError::IllegalTokenColor("0083080".to_owned()))
    );
    let illegal_editor = TokenTheme::from_resolved(&resolved(json!({
        "colors": { "editor.foreground": "#12345", "editor.background": "#1e1e1e" },
        "tokenColors": [],
    })));
    assert_eq!(
        illegal_editor,
        Err(TokenThemeError::IllegalTokenColor("#12345".to_owned()))
    );
}

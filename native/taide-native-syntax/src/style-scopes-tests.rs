use taide_native_editor::syntax::TokenKind;

use super::{StyleScopes, standard_token_kind};
use crate::theme_settings::{ThemeSetting, ThemeStyle};
use crate::tokenizer::{
    FONT_STYLE_BOLD, FONT_STYLE_ITALIC, FONT_STYLE_STRIKETHROUGH, FONT_STYLE_UNDERLINE,
};

const NO_FONT_STYLE: u32 = 0;

fn rule(scopes: &[&str], foreground: Option<&str>, font_style: Option<&str>) -> ThemeSetting {
    ThemeSetting {
        scope: Some(scopes.iter().map(|scope| (*scope).to_owned()).collect()),
        settings: Some(ThemeStyle {
            foreground: foreground.map(str::to_owned),
            background: None,
            font_style: font_style.map(str::to_owned),
        }),
    }
}

#[test]
fn 색과_글꼴_스타일이_같은_첫_규칙의_첫_scope를_돌려준다() {
    let scopes = StyleScopes::from_theme(&[
        rule(&["comment", "punctuation.comment"], Some("#5C6370"), None),
        rule(&["string"], Some("#5c6370"), None),
        rule(&["keyword"], Some("#5C6370"), Some("italic")),
    ]);
    assert_eq!(scopes.scope("#5C6370", NO_FONT_STYLE), "comment");
    assert_eq!(scopes.scope("#5C6370", FONT_STYLE_ITALIC), "keyword");
    assert_eq!(scopes.scope("#5C6370", FONT_STYLE_BOLD), "");
    assert_eq!(scopes.scope("#000000", NO_FONT_STYLE), "");
    assert_eq!(scopes.scope("", NO_FONT_STYLE), "");
}

#[test]
fn 세_자리와_네_자리_색은_자리마다_두_번_써서_맞춘다() {
    let scopes = StyleScopes::from_theme(&[
        rule(&["short"], Some("#F0a"), None),
        rule(&["alpha"], Some("#F0a8"), None),
    ]);
    assert_eq!(scopes.scope("#FF00AA", NO_FONT_STYLE), "short");
    assert_eq!(scopes.scope("#FF00AA88", NO_FONT_STYLE), "alpha");
}

#[test]
fn 글꼴_스타일_문자열은_정해진_순서와_별칭으로_정규화한다() {
    let scopes = StyleScopes::from_theme(&[
        rule(&["emphasis"], Some("#111111"), Some("bold  ITALIC")),
        rule(
            &["deleted"],
            Some("#111111"),
            Some("line-through,underline"),
        ),
        rule(&["unknown"], Some("#222222"), Some("wavy")),
    ]);
    assert_eq!(
        scopes.scope("#111111", FONT_STYLE_ITALIC | FONT_STYLE_BOLD),
        "emphasis"
    );
    assert_eq!(
        scopes.scope("#111111", FONT_STYLE_UNDERLINE | FONT_STYLE_STRIKETHROUGH),
        "deleted"
    );
    assert_eq!(scopes.scope("#222222", NO_FONT_STYLE), "unknown");
}

#[test]
fn 전경색이_없는_규칙과_scope가_없는_설정은_역조회_표에_들어가지_않는다() {
    let scopes = StyleScopes::from_theme(&[
        ThemeSetting {
            scope: None,
            settings: Some(ThemeStyle {
                foreground: Some("#abb2bf".to_owned()),
                background: Some("#282c34".to_owned()),
                font_style: None,
            }),
        },
        rule(&["markup.bold"], None, Some("bold")),
        rule(&[], Some("#333333"), None),
    ]);
    assert_eq!(scopes.scope("#ABB2BF", NO_FONT_STYLE), "");
    assert_eq!(scopes.scope("#ABB2BF", FONT_STYLE_BOLD), "");
    assert_eq!(scopes.scope("#333333", NO_FONT_STYLE), "");
}

#[test]
fn 표준_토큰_종류는_scope_문자열에서_가장_먼저_나오는_단어로_정한다() {
    let cases = [
        ("", TokenKind::Other),
        ("keyword.control", TokenKind::Other),
        ("comment.line.double-slash", TokenKind::Comment),
        ("punctuation.definition.string.begin", TokenKind::String),
        ("string.regexp", TokenKind::String),
        ("keyword.operator.regexp", TokenKind::Regex),
        ("source.regex", TokenKind::Regex),
        ("meta.tag string.quoted", TokenKind::String),
        ("comment-string", TokenKind::Comment),
        ("my_string", TokenKind::Other),
        ("strings", TokenKind::Other),
        ("regexpx.string", TokenKind::String),
        ("한글.comment", TokenKind::Comment),
    ];
    for (scope, kind) in cases {
        assert_eq!(standard_token_kind(scope), kind, "{scope}");
    }
}

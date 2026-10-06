use super::{IllegalTokenColor, MatchedTokenStyle, MonacoTokenRule, MonacoTokenTheme};
use crate::tokenizer::{FONT_STYLE_BOLD, FONT_STYLE_ITALIC, FONT_STYLE_UNDERLINE};

const NO_FONT_STYLE: u32 = 0;
const BLACK: [u8; 3] = [0x00, 0x00, 0x00];
const EDITOR_FOREGROUND: [u8; 3] = [0xd4, 0xd4, 0xd4];
const COMMENT_FOREGROUND: [u8; 3] = [0x6a, 0x99, 0x55];
const DOCUMENTATION_FOREGROUND: [u8; 3] = [0x60, 0x8b, 0x4e];

fn rule(
    token: &str,
    foreground: Option<&str>,
    background: Option<&str>,
    font_style: Option<&str>,
) -> MonacoTokenRule {
    MonacoTokenRule {
        token: token.to_owned(),
        foreground: foreground.map(str::to_owned),
        background: background.map(str::to_owned),
        font_style: font_style.map(str::to_owned),
    }
}

fn style(font_style_bits: u32, foreground: [u8; 3]) -> MatchedTokenStyle {
    MatchedTokenStyle {
        font_style_bits,
        foreground,
    }
}

fn theme(rules: Vec<MonacoTokenRule>) -> MonacoTokenTheme {
    MonacoTokenTheme::new(rules).unwrap()
}

#[test]
fn 규칙이_없으면_검은_전경색과_글꼴_스타일_없음이_기본이다() {
    let theme = theme(Vec::new());
    assert_eq!(theme.matched(""), style(NO_FONT_STYLE, BLACK));
    assert_eq!(theme.matched("comment.line"), style(NO_FONT_STYLE, BLACK));
}

#[test]
fn 빈_token_규칙은_기본값이_되고_자식은_만들어질_때의_부모_값을_물려받는다() {
    let theme = theme(vec![
        rule("", Some("#d4d4d4"), Some("#1e1e1e"), None),
        rule("comment.block.documentation", None, None, Some("bold")),
        rule("comment", Some("6a9955"), None, Some("italic")),
        rule("comment.block", None, None, Some("")),
    ]);
    assert_eq!(theme.matched(""), style(NO_FONT_STYLE, EDITOR_FOREGROUND));
    assert_eq!(
        theme.matched("keyword.control"),
        style(NO_FONT_STYLE, EDITOR_FOREGROUND)
    );
    assert_eq!(
        theme.matched("comment"),
        style(FONT_STYLE_ITALIC, COMMENT_FOREGROUND)
    );
    assert_eq!(
        theme.matched("comment.line.double-slash"),
        style(FONT_STYLE_ITALIC, COMMENT_FOREGROUND)
    );
    assert_eq!(
        theme.matched("comment.block"),
        style(NO_FONT_STYLE, COMMENT_FOREGROUND)
    );
    assert_eq!(
        theme.matched("comment.block.documentation.js"),
        style(FONT_STYLE_BOLD, COMMENT_FOREGROUND)
    );
}

#[test]
fn 같은_token의_뒤_규칙이_앞_규칙을_덮되_없는_값은_남긴다() {
    let theme = theme(vec![
        rule("", Some("#d4d4d4"), Some("#1e1e1e"), None),
        rule("comment", Some("6a9955"), None, Some("italic underline")),
        rule("comment", None, None, Some("bold")),
        rule("comment.block", Some("608b4e"), None, None),
        rule("", None, None, Some("underline")),
    ]);
    assert_eq!(
        theme.matched("string"),
        style(FONT_STYLE_UNDERLINE, EDITOR_FOREGROUND)
    );
    assert_eq!(
        theme.matched("comment"),
        style(FONT_STYLE_BOLD, COMMENT_FOREGROUND)
    );
    assert_eq!(
        theme.matched("comment.block"),
        style(FONT_STYLE_BOLD, DOCUMENTATION_FOREGROUND)
    );
}

#[test]
fn 여덟_자리_색은_앞_여섯_자리만_쓰고_글꼴_스타일은_공백으로만_나눈다() {
    let theme = theme(vec![
        rule("alpha", Some("#6A995580"), None, Some("italic,bold")),
        rule("spaced", Some("6a9955"), None, Some("bold  wavy italic")),
    ]);
    assert_eq!(
        theme.matched("alpha"),
        style(NO_FONT_STYLE, COMMENT_FOREGROUND)
    );
    assert_eq!(
        theme.matched("spaced"),
        style(FONT_STYLE_BOLD | FONT_STYLE_ITALIC, COMMENT_FOREGROUND)
    );
}

#[test]
fn 여섯_자리나_여덟_자리_16진수가_아닌_색은_테마_전체를_거절한다() {
    let illegal_colors = ["", "12345", "#0083080", "abc", "zzzzzz", "가나다라마바"];
    for color in illegal_colors {
        let foreground = MonacoTokenTheme::new(vec![rule("keyword", Some(color), None, None)]);
        assert_eq!(
            foreground.err(),
            Some(IllegalTokenColor(color.to_owned())),
            "{color}"
        );
        let background = MonacoTokenTheme::new(vec![rule("keyword", None, Some(color), None)]);
        assert_eq!(
            background.err(),
            Some(IllegalTokenColor(color.to_owned())),
            "{color}"
        );
        let default = MonacoTokenTheme::new(vec![rule("", Some(color), None, None)]);
        assert_eq!(
            default.err(),
            Some(IllegalTokenColor(color.to_owned())),
            "{color}"
        );
    }
}

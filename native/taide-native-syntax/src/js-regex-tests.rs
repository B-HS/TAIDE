use super::{JsRegex, JsRegexError, oniguruma_source};

const NO_FLAGS: &str = "";
const NO_BREAK_SPACE: &str = "\u{A0}";
const BYTE_ORDER_MARK: &str = "\u{FEFF}";
const NEXT_LINE_CONTROL: &str = "\u{85}";

fn regex(source: &str, flags: &str) -> JsRegex {
    JsRegex::new(source, flags).unwrap_or_else(|error| panic!("{source}: {error:?}"))
}

#[test]
fn 단어_문자와_숫자_클래스는_ascii_만_맞춘다() {
    let word = regex(r"^\w+$", NO_FLAGS);
    assert!(word.is_match("snake_case9"));
    assert!(!word.is_match("한글"));
    assert!(!word.is_match("café"));
    let digit = regex(r"^\d$", NO_FLAGS);
    assert!(digit.is_match("7"));
    assert!(!digit.is_match("٣"));
    assert!(regex(r"^\W$", NO_FLAGS).is_match("한"));
    assert!(regex(r"^\D$", NO_FLAGS).is_match("٣"));
}

#[test]
fn 공백_클래스는_js_의_공백_목록을_따른다() {
    let space = regex(r"^\s$", NO_FLAGS);
    assert!(space.is_match(" "));
    assert!(space.is_match("\t"));
    assert!(space.is_match(NO_BREAK_SPACE));
    assert!(space.is_match(BYTE_ORDER_MARK));
    assert!(!space.is_match(NEXT_LINE_CONTROL));
    let not_space = regex(r"^\S$", NO_FLAGS);
    assert!(not_space.is_match(NEXT_LINE_CONTROL));
    assert!(!not_space.is_match(NO_BREAK_SPACE));
    assert!(regex(r"^[a\s]+$", NO_FLAGS).is_match("a \u{3000}a"));
    assert!(regex(r"^[\S]+$", NO_FLAGS).is_match("한글"));
}

#[test]
fn 단어_경계는_ascii_단어_문자만_본다() {
    let boundary = regex(r"\bend\b", NO_FLAGS);
    assert!(boundary.is_match("the end"));
    assert!(boundary.is_match("한end글"));
    assert!(!boundary.is_match("ending"));
    assert!(!boundary.is_match("end_"));
    let inside = regex(r"\Bend", NO_FLAGS);
    assert!(inside.is_match("bend"));
    assert!(!inside.is_match("한end"));
}

#[test]
fn 줄_앵커는_m_플래그가_없으면_입력_전체의_처음과_끝만_맞춘다() {
    let whole = regex(r"^a$", NO_FLAGS);
    assert!(whole.is_match("a"));
    assert!(!whole.is_match("b\na"));
    assert!(!whole.is_match("a\n"));
    let per_line = regex(r"^a$", "m");
    assert!(per_line.is_match("b\na\nc"));
}

#[test]
fn 점은_줄_끝_문자를_맞추지_않고_s_플래그에서는_모두_맞춘다() {
    assert!(!regex(r"^a.b$", NO_FLAGS).is_match("a\nb"));
    assert!(!regex(r"^a.b$", NO_FLAGS).is_match("a\u{2028}b"));
    assert!(regex(r"^a.b$", NO_FLAGS).is_match("a한b"));
    assert!(regex(r"^a.b$", "s").is_match("a\nb"));
}

#[test]
fn 클래스_안의_클래스_이스케이프_옆_하이픈과_여는_대괄호는_문자_그대로다() {
    let name = regex(r"^[_:\w][_:\w-.\d]*$", NO_FLAGS);
    assert!(name.is_match("my-tag.name2"));
    assert!(!name.is_match("my tag"));
    assert!(regex(r"^[\w-?]+$", NO_FLAGS).is_match("a-b?"));
    assert!(regex(r"^[[{]+$", NO_FLAGS).is_match("[{["));
    assert!(regex(r"^[a&&b]+$", NO_FLAGS).is_match("a&b"));
}

#[test]
fn 빈_클래스와_부정된_빈_클래스를_js_처럼_다룬다() {
    assert!(!regex(r"a[]", NO_FLAGS).is_match("a"));
    assert!(regex(r"^a[^]b$", NO_FLAGS).is_match("a\nb"));
}

#[test]
fn 의미_없는_문자_이스케이프와_짝_없는_중괄호는_문자_그대로다() {
    assert!(regex(r"^\h\e\z$", NO_FLAGS).is_match("hez"));
    assert!(regex(r"^a{b}$", NO_FLAGS).is_match("a{b}"));
    assert!(regex(r"^a{,2}$", NO_FLAGS).is_match("a{,2}"));
    assert!(regex(r"^a{2,3}$", NO_FLAGS).is_match("aaa"));
    assert!(regex(r"^\ \*\/$", NO_FLAGS).is_match(" */"));
}

#[test]
fn 역참조와_전방_탐색과_게으른_수량자를_그대로_쓴다() {
    let quoted = regex(r#"^("|')(.*?)\1$"#, NO_FLAGS);
    assert!(quoted.is_match("'text'"));
    assert!(!quoted.is_match("'text\""));
    assert!(regex(r"^\/\*\*(?!\/)", NO_FLAGS).is_match("/** doc"));
    assert!(!regex(r"^\/\*\*(?!\/)", NO_FLAGS).is_match("/**/"));
}

#[test]
fn i_플래그는_ascii_대소문자를_구분하지_않는다() {
    let tag = regex(r"^<(\w[\w\d]*)>$", "i");
    assert!(tag.is_match("<DIV>"));
    assert!(regex("^end$", "i").is_match("END"));
    assert!(!regex("^end$", NO_FLAGS).is_match("END"));
}

#[test]
fn 일치한_부분을_모두_지운_문자열을_돌려준다() {
    let brackets = regex(r"(\{)|(\})|(\bdo\b)", "gi");
    assert_eq!(brackets.without_matches("{ a } do DO done"), " a    done");
    assert_eq!(brackets.ranges("x{y}"), [1..2, 3..4]);
}

#[test]
fn 첫_묶음이_글자를_잡았는지로_시작과_끝_표식을_가른다() {
    let markers = regex(r"(^\s*#region\b)|(?:^\s*#endregion\b)", NO_FLAGS);
    assert_eq!(
        markers.captures_text_in_first_group("  #region a"),
        Some(true)
    );
    assert_eq!(
        markers.captures_text_in_first_group("#endregion"),
        Some(false)
    );
    assert_eq!(markers.captures_text_in_first_group("code"), None);
}

#[test]
fn 지원하지_않는_플래그와_문법은_오류로_알린다() {
    assert_eq!(
        oniguruma_source("a", "u"),
        Err(JsRegexError::UnsupportedFlag('u'))
    );
    assert_eq!(
        oniguruma_source("a", "y"),
        Err(JsRegexError::UnsupportedFlag('y'))
    );
    assert!(matches!(
        oniguruma_source(r"\012", NO_FLAGS),
        Err(JsRegexError::UnsupportedSyntax(_))
    ));
    assert!(matches!(
        oniguruma_source("(?i:a)", NO_FLAGS),
        Err(JsRegexError::UnsupportedSyntax(_))
    ));
    assert!(matches!(
        oniguruma_source("[a", NO_FLAGS),
        Err(JsRegexError::UnsupportedSyntax(_))
    ));
}

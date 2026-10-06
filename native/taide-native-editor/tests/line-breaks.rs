use taide_native_editor::line_breaks::{
    LineBreakData, WrapSettings, WrappingIndent, create_line_breaks, is_full_width_character,
};

const TAB_SIZE: u32 = 4;
const FULL_WIDTH_COLUMNS: f64 = 2.0;
const MEASURED_FULL_WIDTH_COLUMNS: f64 = 1.5;
const SUPPLEMENTARY: char = '\u{10400}';
const CONTROL: char = '\u{1}';

fn settings(wrap_column: u32) -> WrapSettings {
    WrapSettings {
        wrap_column,
        tab_size: TAB_SIZE,
        full_width_columns: FULL_WIDTH_COLUMNS,
        wrapping_indent: WrappingIndent::Same,
    }
}

fn indented(wrap_column: u32, wrapping_indent: WrappingIndent) -> WrapSettings {
    WrapSettings {
        wrapping_indent,
        ..settings(wrap_column)
    }
}

fn rows(text: &str, settings: &WrapSettings) -> Vec<String> {
    let Some(data) = create_line_breaks(settings, text) else {
        return vec![text.into()];
    };
    let mut start = 0;
    data.break_offsets
        .iter()
        .map(|end| {
            let row = text[start..*end].to_owned();
            start = *end;
            row
        })
        .collect()
}

fn indent(text: &str, settings: &WrapSettings) -> u32 {
    create_line_breaks(settings, text).map_or(0, |data| data.wrapped_text_indent_length)
}

#[test]
fn 줄이_wrap_열_안에_들면_줄바꿈_자료를_만들지_않는다() {
    for text in ["", "a", "aaa", "aaaaa", "\t", "aa\ta"] {
        assert_eq!(create_line_breaks(&settings(5), text), None, "{text:?}");
    }
}

#[test]
fn 줄바꿈_후보가_없으면_wrap_열을_넘는_문자_앞에서_강제로_나눈다() {
    assert_eq!(
        create_line_breaks(&settings(5), "aaaaaa"),
        Some(LineBreakData {
            break_offsets: vec![5, 6],
            break_offsets_visible_column: vec![5.0, 6.0],
            wrapped_text_indent_length: 0,
        })
    );
    assert_eq!(rows("aaaaaaaaaaaa", &settings(5)), ["aaaaa", "aaaaa", "aa"]);
}

#[test]
fn break_after_문자_뒤와_break_before_문자_앞에서_나누고_연속된_문자는_묶는다() {
    for (text, wrap_column, expected) in [
        ("aaaa.aaa", 5, vec!["aaaa.", "aaa"]),
        ("aaa.aaa", 5, vec!["aaa.", "aaa"]),
        ("aaa..aaa", 5, vec!["aaa..", "aaa"]),
        ("aaa(aaa", 5, vec!["aaa", "(aaa"]),
        ("aaa((aaa", 5, vec!["aaa", "((aaa"]),
        ("aaaa+bbbb", 6, vec!["aaaa", "+bbbb"]),
        ("path/to/file", 8, vec!["path/to/", "file"]),
    ] {
        assert_eq!(rows(text, &settings(wrap_column)), expected, "{text:?}");
    }
}

#[test]
fn 공백은_앞_표시_줄_끝에_남고_공백_앞은_후보가_아니어서_넘칠_때만_강제로_나뉜다() {
    for (text, wrap_column, expected) in [
        ("aaa aaa", 5, vec!["aaa ", "aaa"]),
        ("aa   bbbb", 5, vec!["aa   ", "bbbb"]),
        ("aaaaa a", 5, vec!["aaaaa", " a"]),
        ("      ", 4, vec!["    ", "  "]),
    ] {
        assert_eq!(rows(text, &settings(wrap_column)), expected, "{text:?}");
    }
}

#[test]
fn 탭은_다음_탭_정지까지의_열_폭으로_세고_탭_뒤가_줄바꿈_후보다() {
    for (text, expected) in [
        ("\taaaa", vec!["\t", "aaaa"]),
        ("aa\taa", vec!["aa\t", "aa"]),
        ("\ta\taa", vec!["\t", "a\t", "aa"]),
    ] {
        assert_eq!(rows(text, &settings(5)), expected, "{text:?}");
    }
    assert_eq!(
        create_line_breaks(&settings(5), "aa\taa")
            .unwrap()
            .break_offsets_visible_column,
        [4.0, 6.0]
    );
    assert_eq!(
        create_line_breaks(&settings(5), "\ta\taa")
            .unwrap()
            .break_offsets_visible_column,
        [4.0, 8.0, 10.0]
    );
}

#[test]
fn 전각_문자는_전각_열_폭으로_세고_한자와_가나만_문자_사이에서_나뉜다() {
    for (text, wrap_column, expected) in [
        ("漢字漢字", 5, vec!["漢字", "漢字"]),
        ("abc漢字", 4, vec!["abc", "漢字"]),
        ("한글한글한글", 5, vec!["한글", "한글", "한글"]),
    ] {
        assert_eq!(rows(text, &settings(wrap_column)), expected, "{text:?}");
    }
    let measured = WrapSettings {
        full_width_columns: MEASURED_FULL_WIDTH_COLUMNS,
        ..settings(5)
    };
    assert_eq!(
        create_line_breaks(&measured, "한글한글한글"),
        Some(LineBreakData {
            break_offsets: vec![9, 18],
            break_offsets_visible_column: vec![4.5, 9.0],
            wrapped_text_indent_length: 0,
        })
    );
    for (character, expected) in [
        ('\u{2E80}', true),
        ('한', true),
        ('\u{FF4D}', true),
        ('\u{FFE6}', true),
        ('a', false),
        ('\u{2E7F}', false),
        ('\u{FF61}', false),
        (SUPPLEMENTARY, false),
    ] {
        assert_eq!(
            is_full_width_character(character),
            expected,
            "{character:?}"
        );
    }
}

#[test]
fn 금칙은_닫는_문자_앞과_여는_문자_뒤에서_나누지_않고_가나_범위는_break_after_목록보다_앞선다() {
    for (text, wrap_column, expected) in [
        ("漢字。漢字", 5, vec!["漢", "字。", "漢字"]),
        ("漢字「漢字", 6, vec!["漢字", "「漢字"]),
        ("a aaーa", 5, vec!["a aa", "ーa"]),
    ] {
        assert_eq!(rows(text, &settings(wrap_column)), expected, "{text:?}");
    }
}

#[test]
fn 보조_평면_문자는_2열_한_단위이고_제어_문자는_전각_열_폭이다() {
    let astral_middle = format!("aaa{SUPPLEMENTARY}a");
    let astral_first = format!("{SUPPLEMENTARY}aaaa");
    let controls = CONTROL.to_string().repeat(3);
    assert_eq!(
        rows(&astral_middle, &settings(4)),
        ["aaa".to_owned(), format!("{SUPPLEMENTARY}a")]
    );
    assert_eq!(
        rows(&astral_first, &settings(4)),
        [format!("{SUPPLEMENTARY}aa"), "aa".to_owned()]
    );
    assert_eq!(
        rows(&controls, &settings(4)),
        [CONTROL.to_string().repeat(2), CONTROL.to_string()]
    );
}

#[test]
fn 이어지는_줄_들여쓰기는_wrapping_indent를_따르고_본문이_들어갈_폭이_없으면_0이_된다() {
    let text = "    aaaa aaaa";
    for (wrapping_indent, expected_rows, expected_indent) in [
        (WrappingIndent::None, vec!["    aaaa ", "aaaa"], 0),
        (WrappingIndent::Same, vec!["    aaaa ", "aaaa"], 4),
        (WrappingIndent::Indent, vec!["    aaaa ", "aa", "aa"], 8),
        (WrappingIndent::DeepIndent, vec!["    aaaa ", "aaaa"], 0),
    ] {
        let settings = indented(10, wrapping_indent);
        assert_eq!(rows(text, &settings), expected_rows, "{wrapping_indent:?}");
        assert_eq!(
            indent(text, &settings),
            expected_indent,
            "{wrapping_indent:?}"
        );
    }
    for (text, wrap_column, expected_rows, expected_indent) in [
        ("\t\taaaa aaaa", 14, vec!["\t\taaaa ", "aaaa"], 8),
        ("        aaaa", 9, vec!["        ", "aaaa"], 0),
        ("    a aaaaaaaaa", 10, vec!["    a ", "aaaaaa", "aaa"], 4),
        ("    aaaaaaaaaa", 10, vec!["    aaaaaa", "aaaa"], 4),
        ("      ", 4, vec!["    ", "  "], 0),
    ] {
        let settings = settings(wrap_column);
        assert_eq!(rows(text, &settings), expected_rows, "{text:?}");
        assert_eq!(indent(text, &settings), expected_indent, "{text:?}");
    }
}

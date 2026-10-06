use crate::command_registry::PaletteEntry;
use crate::keybinding_search::js_whitespace;

pub const COMMAND_MODE_PREFIX: &str = ">";
pub const SYMBOL_MODE_PREFIX: &str = "@";
pub const LINE_MODE_PREFIX: &str = ":";
pub const WORKSPACE_SYMBOL_MODE_PREFIX: &str = "#";
const LINE_COLUMN_SEPARATOR: char = ':';
const FIRST_POSITION: f64 = 1.0;
const FIRST_COLUMN: f64 = FIRST_POSITION;
const MODE_PREFIXES: [(&str, Mode); 4] = [
    (COMMAND_MODE_PREFIX, Mode::Commands),
    (SYMBOL_MODE_PREFIX, Mode::Symbol),
    (LINE_MODE_PREFIX, Mode::Line),
    (WORKSPACE_SYMBOL_MODE_PREFIX, Mode::WorkspaceSymbol),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Commands,
    Files,
    Symbol,
    Line,
    WorkspaceSymbol,
}

impl Mode {
    pub fn placeholder_key(self) -> &'static str {
        match self {
            Self::Files => "palette.filePlaceholder",
            Self::Commands => "palette.commandPlaceholder",
            Self::Symbol => "palette.symbolPlaceholder",
            Self::Line => "palette.linePlaceholder",
            Self::WorkspaceSymbol => "palette.workspaceSymbolPlaceholder",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Query<'a> {
    pub mode: Mode,
    pub search_term: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineTarget {
    pub line: f64,
    pub column: f64,
}

impl LineTarget {
    pub fn label(self) -> String {
        if self.column > FIRST_COLUMN {
            return format!("{}:{}", self.line, self.column);
        }
        self.line.to_string()
    }
}

pub fn parse(raw_query: &str) -> Query<'_> {
    MODE_PREFIXES
        .iter()
        .find_map(|(prefix, mode)| {
            Some(Query {
                mode: *mode,
                search_term: raw_query
                    .strip_prefix(prefix)?
                    .trim_start_matches(js_whitespace),
            })
        })
        .unwrap_or(Query {
            mode: Mode::Files,
            search_term: raw_query,
        })
}

pub fn command_mode_query(search_term: &str) -> String {
    format!("{COMMAND_MODE_PREFIX}{search_term}")
}

pub fn entry_query(entry: PaletteEntry) -> String {
    match entry {
        PaletteEntry::Files => String::new(),
        PaletteEntry::Commands => command_mode_query(""),
        PaletteEntry::WorkspaceSymbols => WORKSPACE_SYMBOL_MODE_PREFIX.into(),
    }
}

pub fn parse_line_target(search_term: &str) -> Option<LineTarget> {
    let reference = search_term.trim_matches(js_whitespace);
    let (line, column) = match reference.split_once(LINE_COLUMN_SEPARATOR) {
        Some((line, column)) => (line, Some(column)),
        None => (reference, None),
    };
    let line = position(line)?;
    let column = match column {
        Some(column) => position(column)?,
        None => FIRST_COLUMN,
    };
    Some(LineTarget { line, column })
}

fn position(digits: &str) -> Option<f64> {
    if digits.is_empty() || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    digits
        .parse::<f64>()
        .ok()
        .filter(|value| *value >= FIRST_POSITION)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(raw_query: &str) -> (Mode, &str) {
        let query = parse(raw_query);
        (query.mode, query.search_term)
    }

    fn line_target(search_term: &str) -> Option<(f64, f64)> {
        parse_line_target(search_term).map(|target| (target.line, target.column))
    }

    #[test]
    fn parse는_접두사로_모드를_정하고_나머지를_검색어로_본다() {
        assert_eq!(
            parsed("foo.ts"),
            (Mode::Files, "foo.ts"),
            "'>' 로 시작하지 않으면 파일 모드로 판단하고 입력 전체를 검색어로 본다"
        );
        assert_eq!(parsed(""), (Mode::Files, ""), "빈 입력은 파일 모드다");
        assert_eq!(
            parsed(">reload"),
            (Mode::Commands, "reload"),
            "'>' 로 시작하면 커맨드 모드로 판단하고 접두사를 제거한 나머지를 검색어로 본다"
        );
        assert_eq!(
            parsed(">"),
            (Mode::Commands, ""),
            "'>' 단독 입력은 검색어가 빈 커맨드 모드다"
        );
        assert_eq!(
            parsed(">   reload window"),
            (Mode::Commands, "reload window"),
            "'>' 뒤 공백은 검색어에서 제거된다"
        );
        assert_eq!(
            parsed("@handleSave"),
            (Mode::Symbol, "handleSave"),
            "'@' 로 시작하면 symbol 모드다"
        );
        assert_eq!(
            parsed("@"),
            (Mode::Symbol, ""),
            "'@' 단독 입력은 검색어가 빈 symbol 모드다"
        );
        assert_eq!(
            parsed(":123"),
            (Mode::Line, "123"),
            "':' 로 시작하면 line 모드다"
        );
        assert_eq!(
            parsed(":123:45"),
            (Mode::Line, "123:45"),
            "':' 뒤 '줄:열' 표기도 검색어로 그대로 보존한다"
        );
        assert_eq!(
            parsed("#handleSave"),
            (Mode::WorkspaceSymbol, "handleSave"),
            "'#' 로 시작하면 workspaceSymbol 모드다"
        );
    }

    #[test]
    fn command_mode_query는_검색어_앞에_접두사를_붙인다() {
        assert_eq!(command_mode_query(""), ">");
        assert_eq!(command_mode_query("reload"), ">reload");
        assert_eq!(entry_query(PaletteEntry::Files), "");
        assert_eq!(entry_query(PaletteEntry::Commands), ">");
        assert_eq!(entry_query(PaletteEntry::WorkspaceSymbols), "#");
        for mode in [
            Mode::Files,
            Mode::Commands,
            Mode::Symbol,
            Mode::Line,
            Mode::WorkspaceSymbol,
        ] {
            assert!(mode.placeholder_key().starts_with("palette."));
        }
    }

    #[test]
    fn parse_line_target은_줄과_줄_열_표기만_1부터_받아들인다() {
        assert_eq!(
            line_target("123"),
            Some((123.0, 1.0)),
            "숫자만 있으면 1열로 취급한다"
        );
        assert_eq!(
            line_target("123:45"),
            Some((123.0, 45.0)),
            "'줄:열' 표기를 파싱한다"
        );
        assert_eq!(
            line_target("  42  "),
            Some((42.0, 1.0)),
            "앞뒤 공백은 무시한다"
        );
        assert_eq!(line_target(""), None, "빈 문자열은 None 이다");
        assert_eq!(line_target("abc"), None, "숫자가 아닌 입력은 None 이다");
        assert_eq!(line_target("0"), None, "0 이하의 줄은 None 이다");
        assert_eq!(line_target("1:0"), None, "0 이하의 열은 None 이다");
        assert_eq!(
            line_target("1:2:3"),
            None,
            "콜론이 2개 이상이면 형식이 맞지 않아 None 이다"
        );
        for malformed in ["1:", ":1", "+1", "1.5", "１２", "1 2", "-1"] {
            assert_eq!(line_target(malformed), None, "{malformed}");
        }
        assert_eq!(line_target("007:010"), Some((7.0, 10.0)));
    }

    #[test]
    fn line_target_표시는_1열을_생략한다() {
        assert_eq!(parse_line_target("42").unwrap().label(), "42");
        assert_eq!(parse_line_target("42:1").unwrap().label(), "42");
        assert_eq!(parse_line_target("42:10").unwrap().label(), "42:10");
    }
}

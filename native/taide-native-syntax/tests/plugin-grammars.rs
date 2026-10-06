use taide_native_syntax::{
    PluginGrammar, SyntaxError, TextmateTokenizer, ThemeSetting, ThemeStyle, TokenizerLimits,
    bundled_grammar_set, style_id,
};

const FOREGROUND: &str = "#d4d4d4";
const COMMENT: &str = "#6a9955";
const STRING: &str = "#ce9178";
const KEYWORD: &str = "#569cd6";
const STORAGE: &str = "#c586c0";
const LANGUAGE_CONSTANT: &str = "#4fc1ff";
const CORE: [&str; 3] = ["json", "jsonc", "markdown"];
const CORE_AND_RUST: [&str; 4] = ["json", "jsonc", "markdown", "rust"];
const CORE_AND_SHELL: [&str; 4] = ["json", "jsonc", "markdown", "shellscript"];
const INI_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-ini",
    "patterns": [{ "include": "#comment" }, { "include": "#string" }],
    "repository": {
        "comment": { "match": ";.*$", "name": "comment.line.semicolon.taide-ini" },
        "string": { "begin": "\"", "end": "\"", "name": "string.quoted.double.taide-ini" }
    }
}"##;
const INI_LINE: &str = "key = \"v\" ; note";
const INI_STRING_START: u32 = 6;
const INI_GAP_START: u32 = 9;
const INI_COMMENT_START: u32 = 10;
const RUST_HOST_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-host",
    "patterns": [
        {
            "begin": "^<rust>$",
            "end": "^</rust>$",
            "name": "meta.embedded.block.rust",
            "patterns": [{ "include": "source.rust" }]
        },
        { "match": "\\bhost\\b", "name": "keyword.control.taide-host" }
    ]
}"##;
const RUST_LINE: &str = "let s = \"a\"; // n";
const RUST_HOST_LINES: [&str; 4] = ["host", "<rust>", RUST_LINE, "</rust>"];
const EMBEDDED_LINE: usize = 2;
const JSON_HOST_GRAMMAR: &str =
    r##"{ "scopeName": "source.taide-host", "patterns": [{ "include": "source.json" }] }"##;
const TRUE_AS_COMMENT_PATTERNS: &str =
    r##""patterns": [{ "match": "\\btrue\\b", "name": "comment.line.taide" }]"##;
const JSON_LINE: &str = "[true]";
const JSON_LITERAL_START: u32 = 1;
const JSON_FENCE_LINES: [&str; 3] = ["```json", JSON_LINE, "```"];
const FENCED_LINE: usize = 1;
const NESTED_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-nested",
    "patterns": [{ "include": "#groups" }],
    "repository": {
        "groups": { "patterns": [{ "include": "#group" }] },
        "group": {
            "begin": "\\(",
            "end": "\\)",
            "name": "string.group.taide-nested",
            "patterns": [{ "include": "$self" }, { "include": "$base" }]
        }
    }
}"##;
const NESTED_LINE: &str = "a (b (c)) d";
const NESTED_GROUP_START: u32 = 2;
const NESTED_GROUP_END: u32 = 9;
const BROKEN_REGEX_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-broken-regex",
    "patterns": [{ "match": "(", "name": "keyword.taide-broken-regex" }]
}"##;
const CYCLE_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-cycle",
    "patterns": [{ "include": "#a" }],
    "repository": {
        "a": { "patterns": [{ "include": "#b" }] },
        "b": { "patterns": [{ "include": "#a" }, { "match": "x", "name": "keyword.taide-cycle" }] }
    }
}"##;
const SELF_CYCLE_GRAMMAR: &str =
    r##"{ "scopeName": "source.taide-self-cycle", "patterns": [{ "include": "$self" }] }"##;
const FIRST_MUTUAL_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-mutual-first",
    "patterns": [{ "include": "source.taide-mutual-second" }]
}"##;
const SECOND_MUTUAL_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-mutual-second",
    "patterns": [{ "include": "source.taide-mutual-first#entry" }],
    "repository": { "unused": { "match": "x" } }
}"##;
const FIRST_MUTUAL_WITH_ENTRY_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-mutual-first",
    "patterns": [{ "include": "#entry" }],
    "repository": { "entry": { "include": "source.taide-mutual-second" } }
}"##;
const MISSHAPEN_GRAMMAR: &str = r##"{ "scopeName": "source.taide-misshapen", "patterns": [5] }"##;

fn setting(scope: Option<&str>, foreground: &str) -> ThemeSetting {
    ThemeSetting {
        scope: scope.map(|scope| vec![scope.to_owned()]),
        settings: Some(ThemeStyle {
            foreground: Some(foreground.to_owned()),
            background: None,
            font_style: None,
        }),
    }
}

fn theme() -> Vec<ThemeSetting> {
    vec![
        setting(None, FOREGROUND),
        setting(Some("comment"), COMMENT),
        setting(Some("string"), STRING),
        setting(Some("keyword"), KEYWORD),
        setting(Some("storage"), STORAGE),
        setting(Some("constant.language"), LANGUAGE_CONSTANT),
    ]
}

fn plugin(language_id: &str, grammar_json: &str) -> PluginGrammar {
    PluginGrammar::from_contribution(language_id, &[], grammar_json).unwrap()
}

fn colliding(language_id: &str, scope_name: &str) -> PluginGrammar {
    plugin(
        language_id,
        &format!("{{ \"scopeName\": \"{scope_name}\", {TRUE_AS_COMMENT_PATTERNS} }}"),
    )
}

fn tokenizer(requested_language_ids: &[&str], plugins: &[PluginGrammar]) -> TextmateTokenizer {
    TextmateTokenizer::with_plugin_grammars(
        requested_language_ids,
        plugins,
        &theme(),
        TokenizerLimits::default(),
    )
    .unwrap()
}

fn bundled_tokenizer(requested_language_ids: &[&str]) -> TextmateTokenizer {
    TextmateTokenizer::new(
        &bundled_grammar_set(requested_language_ids).unwrap(),
        &theme(),
        TokenizerLimits::default(),
    )
    .unwrap()
}

fn style(tokenizer: &TextmateTokenizer, color: &str) -> u32 {
    let index = tokenizer
        .color_map()
        .iter()
        .position(|listed| listed.eq_ignore_ascii_case(color))
        .unwrap();
    style_id(u32::try_from(index).unwrap(), 0)
}

fn spans(tokenizer: &mut TextmateTokenizer, language_id: &str, lines: &[&str]) -> Vec<Vec<u32>> {
    let mut state = None;
    lines
        .iter()
        .map(|line| {
            let tokenized = tokenizer
                .try_tokenize_line(language_id, line, state.as_ref())
                .unwrap();
            state = Some(tokenized.end_state);
            tokenized.spans
        })
        .collect()
}

fn line_spans(tokenizer: &mut TextmateTokenizer, language_id: &str, line: &str) -> Vec<u32> {
    spans(tokenizer, language_id, &[line]).remove(0)
}

fn literal_styled(tokenizer: &TextmateTokenizer, color: &str) -> Vec<u32> {
    let foreground = style(tokenizer, FOREGROUND);
    vec![
        0,
        foreground,
        JSON_LITERAL_START,
        style(tokenizer, color),
        JSON_LITERAL_START + 4,
        foreground,
    ]
}

#[test]
fn 플러그인_문법은_번들_문법과_함께_실려_그_언어의_줄을_토큰화한다() {
    let mut tokenizer = tokenizer(&CORE_AND_RUST, &[plugin("taide-ini", INI_GRAMMAR)]);
    let foreground = style(&tokenizer, FOREGROUND);
    assert_eq!(
        line_spans(&mut tokenizer, "taide-ini", INI_LINE),
        [
            0,
            foreground,
            INI_STRING_START,
            style(&tokenizer, STRING),
            INI_GAP_START,
            foreground,
            INI_COMMENT_START,
            style(&tokenizer, COMMENT),
        ]
    );

    let mut bundled = bundled_tokenizer(&CORE_AND_RUST);
    for (language_id, line) in [
        ("rust", RUST_LINE),
        ("json", JSON_LINE),
        ("markdown", "# title"),
    ] {
        assert_eq!(
            line_spans(&mut tokenizer, language_id, line),
            line_spans(&mut bundled, language_id, line),
            "{language_id}"
        );
    }
    assert_eq!(tokenizer.color_map(), bundled.color_map());
}

#[test]
fn 플러그인_문법이_끌어오는_번들_언어는_그_언어가_실린_경우에만_토큰화된다() {
    let host = plugin("taide-host", RUST_HOST_GRAMMAR);
    let mut with_rust = tokenizer(&CORE_AND_RUST, std::slice::from_ref(&host));
    let embedded = spans(&mut with_rust, "taide-host", &RUST_HOST_LINES);
    assert_eq!(embedded[0], [0, style(&with_rust, KEYWORD)], "host keyword");
    assert_eq!(
        embedded[EMBEDDED_LINE],
        line_spans(&mut bundled_tokenizer(&CORE_AND_RUST), "rust", RUST_LINE)
    );
    assert!(embedded[EMBEDDED_LINE].contains(&style(&with_rust, STRING)));

    let mut without_rust = tokenizer(&CORE, &[host]);
    assert_eq!(
        spans(&mut without_rust, "taide-host", &RUST_HOST_LINES)[EMBEDDED_LINE],
        [0, style(&without_rust, FOREGROUND)]
    );
}

#[test]
fn 번들_언어와_id가_같은_플러그인_문법은_그_언어의_문서를_차지하고_번들_scope는_그대로_남는다() {
    let mut tokenizer = tokenizer(&CORE, &[colliding("json", "source.taide-json")]);
    let mut bundled = bundled_tokenizer(&CORE);
    assert_eq!(
        line_spans(&mut tokenizer, "json", JSON_LINE),
        literal_styled(&tokenizer, COMMENT)
    );
    assert_eq!(
        line_spans(&mut bundled, "json", JSON_LINE),
        literal_styled(&bundled, LANGUAGE_CONSTANT)
    );
    assert_eq!(
        spans(&mut tokenizer, "markdown", &JSON_FENCE_LINES),
        spans(&mut bundled, "markdown", &JSON_FENCE_LINES)
    );
}

#[test]
fn 번들_언어와_scope만_같은_플러그인_문법은_문서에는_번들_문법이_쓰이고_다른_문법의_include에만_쓰인다()
 {
    let mut tokenizer = tokenizer(
        &CORE,
        &[
            colliding("taide-data", "source.json"),
            plugin("taide-host", JSON_HOST_GRAMMAR),
        ],
    );
    let mut bundled = bundled_tokenizer(&CORE);
    let bundled_line = line_spans(&mut bundled, "json", JSON_LINE);
    assert_eq!(bundled_line, literal_styled(&bundled, LANGUAGE_CONSTANT));
    assert_eq!(line_spans(&mut tokenizer, "json", JSON_LINE), bundled_line);
    assert_eq!(
        line_spans(&mut tokenizer, "taide-data", JSON_LINE),
        bundled_line
    );
    assert_eq!(
        line_spans(&mut tokenizer, "taide-host", JSON_LINE),
        literal_styled(&tokenizer, COMMENT)
    );
    assert_ne!(
        spans(&mut tokenizer, "markdown", &JSON_FENCE_LINES)[FENCED_LINE],
        spans(&mut bundled, "markdown", &JSON_FENCE_LINES)[FENCED_LINE]
    );
}

#[test]
fn 번들_언어와_id와_scope가_모두_같은_플러그인_문법은_번들_문법을_완전히_대신한다() {
    let mut tokenizer = tokenizer(&CORE, &[colliding("json", "source.json")]);
    let mut bundled = bundled_tokenizer(&CORE);
    assert_eq!(
        line_spans(&mut tokenizer, "json", JSON_LINE),
        literal_styled(&tokenizer, COMMENT)
    );
    let fenced = spans(&mut tokenizer, "markdown", &JSON_FENCE_LINES);
    assert!(fenced[FENCED_LINE].contains(&style(&tokenizer, COMMENT)));
    assert_ne!(
        fenced[FENCED_LINE],
        spans(&mut bundled, "markdown", &JSON_FENCE_LINES)[FENCED_LINE]
    );
    assert_eq!(
        line_spans(&mut tokenizer, "jsonc", JSON_LINE),
        line_spans(&mut bundled, "jsonc", JSON_LINE)
    );
}

#[test]
fn 번들_문법의_별칭과_id가_같은_플러그인_문법은_그_번들_언어가_실리면_번들_문법으로_토큰화된다() {
    let shell = colliding("bash", "source.taide-bash");
    let mut alone = tokenizer(&CORE, std::slice::from_ref(&shell));
    assert_eq!(
        line_spans(&mut alone, "bash", "true"),
        [0, style(&alone, COMMENT)]
    );

    let mut aliased = tokenizer(&CORE_AND_SHELL, &[shell]);
    assert_eq!(
        line_spans(&mut aliased, "bash", "true # note"),
        line_spans(
            &mut bundled_tokenizer(&CORE_AND_SHELL),
            "shellscript",
            "true # note"
        )
    );
}

#[test]
fn 자기_자신을_다시_여는_정상_문법은_순환으로_보지_않는다() {
    let mut tokenizer = tokenizer(&CORE, &[plugin("taide-nested", NESTED_GRAMMAR)]);
    let foreground = style(&tokenizer, FOREGROUND);
    assert_eq!(
        line_spans(&mut tokenizer, "taide-nested", NESTED_LINE),
        [
            0,
            foreground,
            NESTED_GROUP_START,
            style(&tokenizer, STRING),
            NESTED_GROUP_END,
            foreground,
        ]
    );
}

#[test]
fn 정규식을_컴파일하지_못하는_플러그인_문법은_그_언어의_줄만_실패하고_다른_언어는_그대로다() {
    let mut tokenizer = tokenizer(
        &CORE,
        &[
            plugin("taide-broken-regex", BROKEN_REGEX_GRAMMAR),
            plugin("taide-ini", INI_GRAMMAR),
        ],
    );
    assert!(matches!(
        tokenizer.try_tokenize_line("taide-broken-regex", "(", None),
        Err(SyntaxError::Tokenization(_))
    ));
    assert!(
        line_spans(&mut tokenizer, "taide-ini", INI_LINE).contains(&style(&tokenizer, COMMENT))
    );
    assert_eq!(
        line_spans(&mut tokenizer, "json", JSON_LINE),
        line_spans(&mut bundled_tokenizer(&CORE), "json", JSON_LINE)
    );
}

#[test]
fn include만으로_순환하는_플러그인_문법은_싣지_않고_다른_문법은_그대로_토큰화한다() {
    let mut tokenizer = tokenizer(
        &CORE,
        &[
            plugin("taide-cycle", CYCLE_GRAMMAR),
            plugin("taide-self-cycle", SELF_CYCLE_GRAMMAR),
            plugin("taide-mutual-first", FIRST_MUTUAL_WITH_ENTRY_GRAMMAR),
            plugin("taide-mutual-second", SECOND_MUTUAL_GRAMMAR),
            plugin("taide-misshapen", MISSHAPEN_GRAMMAR),
            plugin("taide-ini", INI_GRAMMAR),
            plugin("taide-nested", NESTED_GRAMMAR),
        ],
    );
    for language_id in [
        "taide-cycle",
        "taide-self-cycle",
        "taide-mutual-first",
        "taide-mutual-second",
        "taide-misshapen",
    ] {
        assert_eq!(
            tokenizer
                .try_tokenize_line(language_id, "x", None)
                .unwrap_err(),
            SyntaxError::UnknownLanguage(language_id.to_owned()),
            "{language_id}"
        );
    }
    assert!(
        line_spans(&mut tokenizer, "taide-ini", INI_LINE).contains(&style(&tokenizer, COMMENT))
    );
    assert!(
        line_spans(&mut tokenizer, "taide-nested", NESTED_LINE)
            .contains(&style(&tokenizer, STRING))
    );
    assert_eq!(
        line_spans(&mut tokenizer, "json", JSON_LINE),
        line_spans(&mut bundled_tokenizer(&CORE), "json", JSON_LINE)
    );
}

#[test]
fn 없는_규칙을_가리켜_닫히지_않는_include_고리는_순환으로_보지_않는다() {
    let mut tokenizer = tokenizer(
        &CORE,
        &[
            plugin("taide-mutual-first", FIRST_MUTUAL_GRAMMAR),
            plugin("taide-mutual-second", SECOND_MUTUAL_GRAMMAR),
        ],
    );
    assert_eq!(
        line_spans(&mut tokenizer, "taide-mutual-first", "x"),
        [0, style(&tokenizer, FOREGROUND)]
    );
    assert_eq!(
        line_spans(&mut tokenizer, "taide-mutual-second", "x"),
        [0, style(&tokenizer, FOREGROUND)]
    );
}

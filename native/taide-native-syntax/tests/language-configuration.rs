use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;

use serde::Deserialize;
use taide_native_editor::language_configuration::{
    EnterAction, FoldMarker, IndentAction, IndentMetadata, LanguageRules,
};
use taide_native_editor::syntax::TokenKind;
use taide_native_syntax::{JsRegex, MonacoLanguage, monaco_language, monaco_language_ids};

const REFERENCE_PATH: &str = "tests/fixtures/language-configurations/reference.json";
const EXPECTED_MONACO_VERSION: &str = "0.56.0";
const MONACO_REGISTERED_LANGUAGE_COUNT: usize = 23;
const UNCONFIGURED_LANGUAGE_IDS: [&str; 9] = [
    "typescriptreact",
    "javascriptreact",
    "jsonc",
    "toml",
    "shellscript",
    "erb",
    "heex",
    "haskell",
    "zig",
];
const INCREASE_BIT: u8 = 1;
const DECREASE_BIT: u8 = 2;
const INDENT_NEXT_LINE_BIT: u8 = 4;
const UNINDENTED_BIT: u8 = 8;
const NO_MARKER: char = '0';
const START_MARKER: char = '1';
const END_MARKER: char = '2';
const START_AND_END_MARKER: char = '3';
const MATCH: char = '1';

#[derive(Deserialize)]
struct Reference {
    version: String,
    languages: Vec<LanguageReference>,
}

#[derive(Deserialize)]
struct RegExpReference {
    path: String,
    source: String,
    flags: String,
    matches: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EnterBracketReference {
    open_source: String,
    close_source: String,
}

#[derive(Deserialize)]
struct SourceReference {
    source: String,
}

type EnterActionReference = (
    Option<usize>,
    usize,
    usize,
    String,
    Option<String>,
    Option<usize>,
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanguageReference {
    language_id: String,
    corpus: Vec<String>,
    reg_exps: Vec<RegExpReference>,
    enter_brackets: Option<Vec<EnterBracketReference>>,
    enter_splits: Option<Vec<Vec<usize>>>,
    enter_actions: Vec<EnterActionReference>,
    indent_metadata: Option<Vec<u8>>,
    bracket_reg_exp: Option<SourceReference>,
    without_brackets: Option<Vec<String>>,
    reversed_bracket_source: Option<String>,
    last_brackets: Option<Vec<Option<(usize, usize)>>>,
    tree_bracket_source: Option<String>,
    tree_brackets: Option<Vec<Vec<(usize, usize)>>>,
    fold_markers: Option<String>,
}

fn reference() -> Reference {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REFERENCE_PATH);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path:?}: {error}"))
}

fn language(language_id: &str) -> &'static MonacoLanguage {
    monaco_language(language_id)
        .unwrap_or_else(|error| panic!("{language_id}: {error:?}"))
        .unwrap_or_else(|| panic!("{language_id}: no language configuration"))
}

fn byte_offset(text: &str, utf16_offset: usize) -> usize {
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        if units >= utf16_offset {
            return byte;
        }
        units += character.len_utf16();
    }
    text.len()
}

fn byte_range(text: &str, (start, end): (usize, usize)) -> Range<usize> {
    byte_offset(text, start)..byte_offset(text, end)
}

fn unescaped_slashes(source: &str) -> String {
    source.replace(r"\/", "/")
}

fn indent_action(name: &str) -> IndentAction {
    match name {
        "None" => IndentAction::None,
        "Indent" => IndentAction::Indent,
        "IndentOutdent" => IndentAction::IndentOutdent,
        "Outdent" => IndentAction::Outdent,
        other => panic!("unknown indent action {other}"),
    }
}

fn metadata_bits(metadata: IndentMetadata) -> u8 {
    u8::from(metadata.increases) * INCREASE_BIT
        + u8::from(metadata.decreases) * DECREASE_BIT
        + u8::from(metadata.indents_next_line) * INDENT_NEXT_LINE_BIT
        + u8::from(metadata.is_unindented) * UNINDENTED_BIT
}

#[test]
fn 기준_자료는_고정된_monaco_판과_ts_가_등록하는_언어를_모두_담는다() {
    let reference = reference();
    assert_eq!(reference.version, EXPECTED_MONACO_VERSION);
    let referenced: Vec<&str> = reference
        .languages
        .iter()
        .map(|language| language.language_id.as_str())
        .collect();
    let mut configured = monaco_language_ids().unwrap();
    assert_eq!(configured.len(), MONACO_REGISTERED_LANGUAGE_COUNT);
    for language_id in &referenced {
        assert!(configured.contains(language_id), "{language_id}");
    }
    configured.retain(|language_id| !referenced.contains(language_id));
    assert_eq!(configured, Vec::<&str>::new());
    for language_id in UNCONFIGURED_LANGUAGE_IDS {
        assert!(
            monaco_language(language_id).unwrap().is_none(),
            "{language_id}"
        );
    }
    assert!(monaco_language("unknown-language").unwrap().is_none());
}

#[test]
fn 추출한_모든_정규식이_컴파일되고_js_와_같은_줄을_맞춘다() {
    let mut differences = Vec::new();
    let mut compared = 0;
    for language in reference().languages {
        for expression in &language.reg_exps {
            let regex = match JsRegex::new(&expression.source, &expression.flags) {
                Ok(regex) => regex,
                Err(error) => {
                    differences.push(format!(
                        "{} {}: {error:?}",
                        language.language_id, expression.path
                    ));
                    continue;
                }
            };
            for (line, expected) in language.corpus.iter().zip(expression.matches.chars()) {
                compared += 1;
                if regex.is_match(line) != (expected == MATCH) {
                    differences.push(format!(
                        "{} {} /{}/{} on {line:?}: JavaScript {expected}",
                        language.language_id, expression.path, expression.source, expression.flags
                    ));
                }
            }
        }
    }
    assert!(compared > 0);
    assert_eq!(differences, Vec::<String>::new());
}

#[test]
fn 괄호에서_만든_정규식_소스가_monaco_와_같다() {
    for reference in reference().languages {
        let sources = language(&reference.language_id).bracket_pattern_sources();
        let enter_brackets: Vec<(String, String)> = reference
            .enter_brackets
            .unwrap_or_default()
            .iter()
            .map(|bracket| {
                (
                    unescaped_slashes(&bracket.open_source),
                    unescaped_slashes(&bracket.close_source),
                )
            })
            .collect();
        assert_eq!(
            sources.enter_brackets, enter_brackets,
            "{}",
            reference.language_id
        );
        assert_eq!(
            sources.brackets,
            reference
                .bracket_reg_exp
                .map(|expression| unescaped_slashes(&expression.source)),
            "{}",
            reference.language_id
        );
        assert_eq!(
            sources.reversed_brackets,
            reference
                .reversed_bracket_source
                .map(|source| unescaped_slashes(&source)),
            "{}",
            reference.language_id
        );
        assert_eq!(
            sources.tree_brackets, reference.tree_bracket_source,
            "{}",
            reference.language_id
        );
    }
}

#[test]
fn enter_규칙이_monaco_의_on_enter_support_와_같은_동작을_고른다() {
    let mut differences = Vec::new();
    let mut compared = 0;
    for reference in reference().languages {
        let Some(splits) = &reference.enter_splits else {
            assert!(
                reference.enter_actions.is_empty(),
                "{}",
                reference.language_id
            );
            continue;
        };
        let rules = language(&reference.language_id);
        let expected: HashMap<(usize, usize), EnterAction> = reference
            .enter_actions
            .iter()
            .map(|(_, line, offset, action, append_text, remove_text)| {
                (
                    (*line, *offset),
                    EnterAction {
                        indent_action: indent_action(action),
                        append_text: append_text.clone(),
                        remove_text: *remove_text,
                    },
                )
            })
            .collect();
        for (index, line) in reference.corpus.iter().enumerate() {
            let previous = index
                .checked_sub(1)
                .map_or("", |previous| &reference.corpus[previous]);
            for offset in &splits[index] {
                let (before, after) = line.split_at(byte_offset(line, *offset));
                let actual = rules.enter_action(previous, before, after);
                compared += 1;
                if actual.as_ref() != expected.get(&(index, *offset)) {
                    differences.push(format!(
                        "{} {before:?}|{after:?} after {previous:?}: {actual:?}",
                        reference.language_id
                    ));
                }
            }
        }
    }
    assert!(compared > 0);
    assert_eq!(differences, Vec::<String>::new());
}

#[test]
fn 들여쓰기_규칙과_괄호_제거와_괄호_찾기가_monaco_와_같다() {
    let mut differences = Vec::new();
    for reference in reference().languages {
        let rules = language(&reference.language_id);
        let id = &reference.language_id;
        for (index, line) in reference.corpus.iter().enumerate() {
            let metadata = rules.indent_metadata(line).map(metadata_bits);
            let expected_metadata = reference.indent_metadata.as_ref().map(|all| all[index]);
            if metadata != expected_metadata {
                differences.push(format!("{id} indent metadata {line:?}: {metadata:?}"));
            }
            if let Some(expected) = &reference.without_brackets
                && rules.without_brackets(line) != expected[index]
            {
                differences.push(format!("{id} without brackets {line:?}"));
            }
            let last = rules.last_bracket(line);
            let expected_last = reference
                .last_brackets
                .as_ref()
                .and_then(|all| all[index])
                .map(|range| byte_range(line, range));
            if last != expected_last {
                differences.push(format!("{id} last bracket {line:?}: {last:?}"));
            }
            let found = rules.bracket_ranges(line);
            let expected_found: Vec<Range<usize>> = reference
                .tree_brackets
                .as_ref()
                .map(|all| {
                    all[index]
                        .iter()
                        .map(|range| byte_range(line, *range))
                        .collect()
                })
                .unwrap_or_default();
            if found != expected_found {
                differences.push(format!("{id} brackets {line:?}: {found:?}"));
            }
        }
    }
    assert_eq!(differences, Vec::<String>::new());
}

#[test]
fn 접기_표식이_monaco_의_합친_정규식과_같은_줄을_고른다() {
    let mut differences = Vec::new();
    let mut marked = 0;
    for reference in reference().languages {
        let rules = language(&reference.language_id);
        let Some(markers) = &reference.fold_markers else {
            assert!(
                reference
                    .corpus
                    .iter()
                    .all(|line| rules.fold_marker(line).is_none()),
                "{}",
                reference.language_id
            );
            continue;
        };
        for (line, expected) in reference.corpus.iter().zip(markers.chars()) {
            let expected_marker = match expected {
                NO_MARKER => None,
                START_MARKER | START_AND_END_MARKER => Some(FoldMarker::Start),
                END_MARKER => Some(FoldMarker::End),
                other => panic!("unknown marker {other}"),
            };
            let starts_region = matches!(expected, START_MARKER | START_AND_END_MARKER);
            marked += usize::from(expected_marker.is_some());
            if rules.fold_marker(line) != expected_marker
                || rules.starts_marker_region(line) != starts_region
            {
                differences.push(format!("{} {line:?}", reference.language_id));
            }
        }
    }
    assert!(marked > 0);
    assert_eq!(differences, Vec::<String>::new());
}

#[test]
fn 쌍_자료는_monaco_의_기본값_규칙으로_채운다() {
    let plaintext = language("plaintext").pairs();
    let opens = |pairs: &[taide_native_editor::language_configuration::AutoClosingPair]| {
        pairs
            .iter()
            .map(|pair| pair.open.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(opens(&plaintext.auto_closing_pairs), ["(", "[", "{"]);
    assert_eq!(plaintext.surrounding_close('<'), Some(">"));
    assert_eq!(plaintext.surrounding_close('`'), Some("`"));
    assert_eq!(plaintext.block_comment_start, None);
    assert!(language("plaintext").is_off_side());

    let typescript = language("typescript");
    let pairs = typescript.pairs();
    assert_eq!(
        opens(&pairs.auto_closing_pairs),
        ["{", "[", "(", "\"", "'", "`", "/**"]
    );
    assert_eq!(
        opens(&pairs.auto_closing_pairs).len(),
        pairs.surrounding_pairs.len()
    );
    let double_quote = &pairs.auto_closing_pairs[3];
    assert!(double_quote.allows(TokenKind::Comment));
    assert!(!double_quote.allows(TokenKind::String));
    assert_eq!(pairs.auto_close_before_quotes, ";:.,=}])> \n\t");
    assert_eq!(pairs.auto_close_before_brackets, "'\"`;:.,=}])> \n\t");
    assert_eq!(pairs.block_comment_start.as_deref(), Some("/*"));
    assert_eq!(typescript.line_comment(), Some("//"));
    assert!(typescript.word_pattern().is_some());
    assert!(!typescript.is_off_side());

    let json = language("json").pairs();
    assert!(
        json.auto_closing_pairs
            .iter()
            .all(|pair| !pair.allows(TokenKind::String))
    );
    assert!(language("python").is_off_side());
    assert!(language("yaml").is_off_side());
    assert!(language("ruby").indent_metadata("").is_some());
    assert!(language("rust").indent_metadata("").is_none());
}

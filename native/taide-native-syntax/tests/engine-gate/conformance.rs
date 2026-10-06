use std::collections::{BTreeSet, HashMap};

use taide_native_editor::syntax::TokenKind;
use taide_native_syntax::{
    FONT_STYLE_VARIANTS, LineState, SPAN_FIELDS, TextmateTokenizer, TokenizedLine,
    UNSTYLED_STYLE_ID, bundled_grammar_set, style_id,
};

use crate::reference::{
    SampleReference, THEME_IDS, ThemeReference, document_lines, sample_manifest, sample_reference,
    theme_reference, tokenizer, unlimited_time, utf16_len,
};

const METADATA_TOKEN_TYPE_OFFSET: u32 = 8;
const METADATA_TOKEN_TYPE_MASK: u32 = 0b11;
const METADATA_FONT_STYLE_OFFSET: u32 = 11;
const METADATA_FONT_STYLE_MASK: u32 = 0b1111;
const METADATA_FOREGROUND_OFFSET: u32 = 15;
const METADATA_FOREGROUND_MASK: u32 = 0b1_1111_1111;
const METADATA_COMMENT: u32 = 1;
const METADATA_STRING: u32 = 2;
const METADATA_REGEX: u32 = 3;
const REFERENCE_TOKEN_FIELDS: usize = 2;
const CORE_LANGUAGE_IDS: [&str; 3] = ["json", "jsonc", "markdown"];
const SCOPE_NAME_KEY: &str = "scopeName";
const UNMATCHED_SCOPE: &str = "";
const REPORTED_FAILURE_LIMIT: usize = 60;

struct FinalStyles {
    default_metadata: u32,
    metadata_by_style: HashMap<u32, u32>,
}

impl FinalStyles {
    fn new(theme: &ThemeReference) -> Self {
        Self {
            default_metadata: theme.default_final_metadata,
            metadata_by_style: theme
                .style_scopes
                .iter()
                .map(|style| {
                    (
                        style_id(style.color_index, style.font_style),
                        style.final_metadata,
                    )
                })
                .collect(),
        }
    }

    fn metadata(&self, style: u32) -> u32 {
        self.metadata_by_style
            .get(&style)
            .copied()
            .unwrap_or(self.default_metadata)
    }
}

#[derive(Default)]
struct Tally {
    lines: usize,
    spans: usize,
    span_mismatches: usize,
    final_mismatches: usize,
    kind_mismatches: usize,
    errors: usize,
}

impl Tally {
    fn mismatches(&self) -> usize {
        self.span_mismatches + self.final_mismatches + self.kind_mismatches + self.errors
    }

    fn add(&mut self, other: &Self) {
        self.lines += other.lines;
        self.spans += other.spans;
        self.span_mismatches += other.span_mismatches;
        self.final_mismatches += other.final_mismatches;
        self.kind_mismatches += other.kind_mismatches;
        self.errors += other.errors;
    }
}

fn metadata_style(metadata: u32) -> u32 {
    style_id(
        (metadata >> METADATA_FOREGROUND_OFFSET) & METADATA_FOREGROUND_MASK,
        (metadata >> METADATA_FONT_STYLE_OFFSET) & METADATA_FONT_STYLE_MASK,
    )
}

fn metadata_kind(metadata: u32) -> TokenKind {
    match (metadata >> METADATA_TOKEN_TYPE_OFFSET) & METADATA_TOKEN_TYPE_MASK {
        METADATA_COMMENT => TokenKind::Comment,
        METADATA_STRING => TokenKind::String,
        METADATA_REGEX => TokenKind::Regex,
        _ => TokenKind::Other,
    }
}

fn merged<T: Copy + PartialEq>(tokens: impl IntoIterator<Item = (usize, T)>) -> Vec<(usize, T)> {
    let mut result: Vec<(usize, T)> = Vec::new();
    for (start, value) in tokens {
        if result.last().map(|(_, last)| *last) != Some(value) {
            result.push((start, value));
        }
    }
    result
}

fn reference_pairs(tokens: &[u32]) -> impl Iterator<Item = (usize, u32)> + '_ {
    tokens
        .as_chunks::<REFERENCE_TOKEN_FIELDS>()
        .0
        .iter()
        .map(|[start, metadata]| (*start as usize, *metadata))
}

fn expected_spans(reference: &SampleReference, line: usize) -> Vec<(usize, u32)> {
    let raw = &reference.raw_tokens[line];
    if raw.is_empty() {
        return vec![(0, UNSTYLED_STYLE_ID)];
    }
    merged(reference_pairs(raw).map(|(start, metadata)| (start, metadata_style(metadata))))
}

fn actual_spans(line: &str, tokenized: &TokenizedLine) -> Vec<(usize, u32)> {
    tokenized
        .spans
        .as_chunks::<SPAN_FIELDS>()
        .0
        .iter()
        .map(|[start_byte, style]| (utf16_len(&line[..*start_byte as usize]), *style))
        .collect()
}

fn engine_color_map(tokenizer: &TextmateTokenizer) -> Vec<Option<String>> {
    tokenizer
        .color_map()
        .iter()
        .map(|color| (!color.is_empty()).then(|| color.clone()))
        .collect()
}

fn compare_document(
    label: &str,
    language_id: &str,
    lines: &[String],
    reference: &SampleReference,
    final_styles: &FinalStyles,
    tokenizer: &mut TextmateTokenizer,
    failures: &mut Vec<String>,
) -> Tally {
    let mut tally = Tally::default();
    let mut previous: Option<LineState> = None;
    for (index, line) in lines.iter().enumerate() {
        let tokenized = match tokenizer.try_tokenize_line(language_id, line, previous.as_ref()) {
            Ok(tokenized) => tokenized,
            Err(error) => {
                tally.errors += 1;
                failures.push(format!("{label}:{index} error {error:?}"));
                break;
            }
        };
        tally.lines += 1;
        let expected = expected_spans(reference, index);
        let actual = actual_spans(line, &tokenized);
        tally.spans += expected.len();
        if actual != expected {
            tally.span_mismatches += 1;
            failures.push(format!(
                "{label}:{index} spans\n  expected {expected:?}\n  actual   {actual:?}"
            ));
        }
        let expected_final: Vec<(usize, u32)> =
            reference_pairs(&reference.final_tokens[index]).collect();
        let actual_final = merged(
            actual
                .iter()
                .map(|(start, style)| (*start, final_styles.metadata(*style))),
        );
        if actual_final != expected_final {
            tally.final_mismatches += 1;
            failures.push(format!(
                "{label}:{index} final\n  expected {expected_final:?}\n  actual   {actual_final:?}"
            ));
        }
        let expected_kinds = merged(
            expected_final
                .iter()
                .map(|(start, metadata)| (*start, metadata_kind(*metadata))),
        );
        let actual_kinds: Vec<(usize, TokenKind)> = tokenized
            .kinds
            .iter()
            .map(|token| (utf16_len(&line[..token.start_byte]), token.kind))
            .collect();
        if actual_kinds != expected_kinds {
            tally.kind_mismatches += 1;
            failures.push(format!(
                "{label}:{index} kinds\n  expected {expected_kinds:?}\n  actual   {actual_kinds:?}"
            ));
        }
        if tokenized.is_stopped_early {
            tally.errors += 1;
            failures.push(format!(
                "{label}:{index} stopped early without a time limit"
            ));
        }
        previous = Some(tokenized.end_state);
    }
    tally
}

#[test]
fn 표본_전체의_토큰_경계와_최종_스타일이_ts_기준_자료와_일치한다() {
    let manifest = sample_manifest();
    let mut failures = Vec::new();
    let mut total = Tally::default();
    for theme_id in THEME_IDS {
        let theme = theme_reference(theme_id);
        let final_styles = FinalStyles::new(&theme);
        for sample in &manifest.samples {
            let label = format!("{theme_id}/{}", sample.id);
            let reference = sample_reference(theme_id, &sample.id);
            let lines = document_lines(&manifest, sample);
            let lengths: Vec<usize> = lines.iter().map(|line| utf16_len(line)).collect();
            assert_eq!(lengths, reference.line_utf16_lengths, "{label}");
            let requested: Vec<&str> = reference
                .requested_language_ids
                .iter()
                .map(String::as_str)
                .collect();
            let mut tokenizer = tokenizer(&theme, &requested, unlimited_time());
            let tally = compare_document(
                &label,
                &sample.language_id,
                &lines,
                &reference,
                &final_styles,
                &mut tokenizer,
                &mut failures,
            );
            println!(
                "{label}: lines {} spans {} span mismatches {} final mismatches {} kind mismatches {} errors {}",
                tally.lines,
                tally.spans,
                tally.span_mismatches,
                tally.final_mismatches,
                tally.kind_mismatches,
                tally.errors
            );
            total.add(&tally);
        }
    }
    println!(
        "total: lines {} spans {} span mismatches {} final mismatches {} kind mismatches {} errors {}",
        total.lines,
        total.spans,
        total.span_mismatches,
        total.final_mismatches,
        total.kind_mismatches,
        total.errors
    );
    assert_eq!(
        total.mismatches(),
        0,
        "{}",
        failures
            .iter()
            .take(REPORTED_FAILURE_LIMIT)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn 요청한_언어_집합이_ts와_같은_문법_집합을_싣는다() {
    let manifest = sample_manifest();
    for sample in &manifest.samples {
        let reference = sample_reference(THEME_IDS[0], &sample.id);
        let requested: Vec<&str> = reference
            .requested_language_ids
            .iter()
            .map(String::as_str)
            .collect();
        let loaded: BTreeSet<String> = bundled_grammar_set(&requested)
            .unwrap()
            .grammar_sources
            .iter()
            .map(|source| {
                let grammar: serde_json::Value = serde_json::from_str(source).unwrap();
                grammar[SCOPE_NAME_KEY].as_str().unwrap().to_owned()
            })
            .collect();
        let expected: BTreeSet<String> = reference.loaded_scope_names.iter().cloned().collect();
        assert_eq!(loaded, expected, "{}", sample.id);
    }
}

#[test]
fn 엔진의_색_표와_스타일별_scope_역조회와_토큰_종류가_ts_기준과_같다() {
    for theme_id in THEME_IDS {
        let theme = theme_reference(theme_id);
        let final_styles = FinalStyles::new(&theme);
        let tokenizer = tokenizer(&theme, &CORE_LANGUAGE_IDS, unlimited_time());
        assert_eq!(engine_color_map(&tokenizer), theme.color_map, "{theme_id}");
        let scope_by_style: HashMap<u32, &str> = theme
            .style_scopes
            .iter()
            .map(|style| {
                (
                    style_id(style.color_index, style.font_style),
                    style.scope.as_str(),
                )
            })
            .collect();
        let style_count = u32::try_from(theme.color_map.len()).unwrap() * FONT_STYLE_VARIANTS;
        for style in 0..style_count {
            let expected_scope = scope_by_style
                .get(&style)
                .copied()
                .unwrap_or(UNMATCHED_SCOPE);
            assert_eq!(
                tokenizer.monaco_scope(style),
                expected_scope,
                "{theme_id} style {style}"
            );
            assert_eq!(
                tokenizer.token_kind(style),
                metadata_kind(final_styles.metadata(style)),
                "{theme_id} style {style}"
            );
        }
    }
}

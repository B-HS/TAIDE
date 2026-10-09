use std::ops::Range;
use std::sync::Arc;

use serde::Deserialize;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::bracket_model::{BracketModel, BracketTokenData};
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::line_tokens::{LineTokens, TokenStyle, TokenStyleTable};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::syntax::TokenKind;
use taide_native_syntax::{MonacoLanguage, monaco_language};

const TAB_SIZE: u32 = 4;
const BYTE_LIMIT: usize = 1024 * 1024;
const HISTORY_LIMIT: usize = 8;
const CONFIGURATIONS: usize = 23;
const REFERENCE_CASES: usize = 230;
const DEEP_PAIRS: usize = 4096;
const LARGE_PAIR_ROWS: usize = 50_000;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reference {
    version: String,
    tab_size: u32,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    language_id: String,
    text: String,
    brackets: Vec<(usize, usize, i64, bool)>,
    pairs: Vec<Pair>,
    indents: Vec<usize>,
    active_indents: Vec<(usize, usize, usize)>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pair {
    open: (usize, usize),
    close: Option<(usize, usize)>,
    level: usize,
    minimum_indent: usize,
}

fn fixture(text: &str, language: &str) -> (EditorStore, DocumentId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 4,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/brackets.txt".into(),
            OpenedFile {
                path: "/synthetic/brackets.txt".into(),
                content: text.into(),
                language_id: language.into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    (store, document)
}

fn rules(language: &str) -> &'static MonacoLanguage {
    monaco_language(language).unwrap().unwrap()
}

fn analyze(store: &mut EditorStore, document: DocumentId, language: &str) -> Arc<BracketModel> {
    store
        .bracket_model(document, Some(rules(language)), TAB_SIZE, None)
        .unwrap()
}

fn style(kind: TokenKind) -> TokenStyle {
    TokenStyle {
        foreground: [255; 4],
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind,
    }
}

fn tokenized<'a>(
    lines: &'a LineTokens,
    styles: &'a TokenStyleTable,
) -> Option<BracketTokenData<'a>> {
    Some(BracketTokenData { lines, styles })
}

fn apply(store: &mut EditorStore, document: DocumentId, bytes: Range<usize>, text: &str) {
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes,
                    text: text.into(),
                }],
                group: UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

#[test]
fn 괄호_중첩과_불일치와_들여쓰기_230표본은_실제_monaco_23언어와_일치한다() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/bracket-display-reference.json")).unwrap();
    assert_eq!(reference.version, "0.56.0");
    assert_eq!(reference.tab_size, TAB_SIZE);
    assert_eq!(reference.cases.len(), REFERENCE_CASES);
    let languages: std::collections::HashSet<_> = reference
        .cases
        .iter()
        .map(|case| &case.language_id)
        .collect();
    assert_eq!(languages.len(), CONFIGURATIONS);
    for case in reference.cases {
        let (mut store, document) = fixture(&case.text, &case.language_id);
        let model = analyze(&mut store, document, &case.language_id);
        let actual: Vec<_> = model
            .brackets()
            .iter()
            .filter(|bracket| bracket.colorized)
            .map(|bracket| {
                (
                    bracket.bytes.start,
                    bracket.bytes.end,
                    bracket.level as i64,
                    bracket.invalid,
                )
            })
            .collect();
        let expected: Vec<_> = case
            .brackets
            .iter()
            .map(|&(start, end, level, invalid)| {
                (start, end, if invalid { 0 } else { level }, invalid)
            })
            .collect();
        let actual: Vec<_> = actual
            .into_iter()
            .map(|(start, end, level, invalid)| {
                (start, end, if invalid { 0 } else { level }, invalid)
            })
            .collect();
        assert_eq!(actual, expected, "{} {:?}", case.language_id, case.text);
        assert_eq!(
            model.pairs().len(),
            case.pairs.len(),
            "{} {:?}",
            case.language_id,
            case.text
        );
        for (actual, expected) in model.pairs().iter().zip(case.pairs) {
            assert_eq!((actual.open.start, actual.open.end), expected.open);
            assert_eq!(
                actual.close.as_ref().map(|close| (close.start, close.end)),
                expected.close
            );
            assert_eq!(actual.guide_level, expected.level);
            if actual.close.is_some() {
                assert_eq!(
                    actual.guide_column,
                    actual
                        .open_column
                        .min(actual.close_column)
                        .min(expected.minimum_indent),
                    "{} {:?}",
                    case.language_id,
                    case.text
                );
            }
        }
        for (line, expected) in case.indents.iter().enumerate() {
            assert_eq!(
                model.indent_level(line),
                *expected,
                "{} {:?} line {}",
                case.language_id,
                case.text,
                line
            );
            let active = model.active_indent(line, 0..case.indents.len());
            assert_eq!(
                (active.lines.start, active.lines.end, active.level),
                case.active_indents[line],
                "{} {:?} active line {}",
                case.language_id,
                case.text,
                line
            );
        }
    }
}

#[test]
fn 문자열_주석_정규식_토큰과_아직_정확하지_않은_줄의_괄호는_분석에서_제외한다() {
    let text = "() \"[]\" // {}\n/()/\n{\n}\n";
    let (mut store, document) = fixture(text, "javascript");
    let styles = TokenStyleTable::new(
        style(TokenKind::Other),
        vec![
            style(TokenKind::Other),
            style(TokenKind::String),
            style(TokenKind::Comment),
            style(TokenKind::Regex),
        ],
    );
    let mut lines = LineTokens::new(text.split('\n').count());
    lines.set_line(0, vec![0, 0, 3, 1, 7, 0, 8, 2], false);
    lines.set_line(1, vec![0, 3], false);
    lines.set_line(2, vec![0, 0], false);
    let model = store
        .bracket_model(
            document,
            Some(rules("javascript")),
            TAB_SIZE,
            tokenized(&lines, &styles),
        )
        .unwrap();
    assert_eq!(
        model
            .brackets()
            .iter()
            .map(|bracket| bracket.bytes.clone())
            .collect::<Vec<_>>(),
        [0..1, 1..2, 19..20]
    );
    assert_eq!(
        model
            .pairs()
            .iter()
            .filter(|pair| pair.close.is_some())
            .count(),
        1
    );
    assert!(!model.brackets().last().unwrap().invalid);
    lines.set_line(3, vec![0, 0], false);
    lines.set_line(4, Vec::new(), false);
    let model = store
        .bracket_model(
            document,
            Some(rules("javascript")),
            TAB_SIZE,
            tokenized(&lines, &styles),
        )
        .unwrap();
    assert_eq!(
        model
            .pairs()
            .iter()
            .filter(|pair| pair.close.is_some())
            .count(),
        2
    );
    assert!(model.brackets().iter().all(|bracket| !bracket.invalid));
}

#[test]
fn 캐시는_같은_토큰을_재분석하지_않고_줄_삽입과_종류_갱신을_따른다() {
    let (mut store, document) = fixture("{\n    ()\n}", "javascript");
    let model = analyze(&mut store, document, "javascript");
    let initial = model.analyzed_line_count();
    assert_eq!(initial, 3);
    let repeated = analyze(&mut store, document, "javascript");
    assert!(Arc::ptr_eq(&model, &repeated));
    assert_eq!(repeated.refresh_count(), 1);
    drop(repeated);
    drop(model);
    apply(&mut store, document, 6..6, "[x]\n    ");
    let updated = analyze(&mut store, document, "javascript");
    assert_eq!(updated.analyzed_line_count(), initial + 2);
    assert_eq!(
        updated
            .pairs()
            .iter()
            .filter(|pair| pair.close.is_some())
            .count(),
        3
    );
    let styles = TokenStyleTable::new(
        style(TokenKind::Other),
        vec![style(TokenKind::Other), style(TokenKind::Comment)],
    );
    let mut lines = LineTokens::new(4);
    for line in 0..4 {
        lines.set_line(line, vec![0, 0], false);
    }
    let first = store
        .bracket_model(
            document,
            Some(rules("javascript")),
            TAB_SIZE,
            tokenized(&lines, &styles),
        )
        .unwrap();
    let count = first.analyzed_line_count();
    lines.set_line(1, vec![0, 1], false);
    let changed = store
        .bracket_model(
            document,
            Some(rules("javascript")),
            TAB_SIZE,
            tokenized(&lines, &styles),
        )
        .unwrap();
    assert_eq!(changed.analyzed_line_count(), count + 1);
    assert_eq!(
        changed
            .pairs()
            .iter()
            .filter(|pair| pair.close.is_some())
            .count(),
        2
    );
    assert_eq!(
        first
            .pairs()
            .iter()
            .filter(|pair| pair.close.is_some())
            .count(),
        3
    );
    assert_eq!(
        store
            .bracket_model(document, Some(rules("json")), TAB_SIZE, None)
            .unwrap()
            .pairs()
            .iter()
            .filter(|pair| pair.close.is_some())
            .count(),
        2
    );
}

#[test]
fn 깊은_문서도_재귀_상한이나_스택_오버플로_없이_분석하고_안쪽_활성_짝을_찾는다() {
    let text = format!("{}x{}", "(".repeat(DEEP_PAIRS), ")".repeat(DEEP_PAIRS));
    let (mut store, document) = fixture(&text, "javascript");
    let model = analyze(&mut store, document, "javascript");
    assert_eq!(model.pairs().len(), DEEP_PAIRS);
    assert!(model.brackets().iter().all(|bracket| !bracket.invalid));
    assert_eq!(
        model.active_pair(DEEP_PAIRS).unwrap().guide_level,
        DEEP_PAIRS - 1
    );
    assert_eq!(model.pairs_in(DEEP_PAIRS..DEEP_PAIRS + 1).len(), DEEP_PAIRS);
    assert!(model.pairs_in(text.len() + 1..text.len() + 2).is_empty());
}

#[test]
fn 대형_문서의_먼_조회는_조상과_화면_짝만_반환하고_문서_종료로_캐시를_회수한다() {
    let text = format!("{{\n{}}}", "    ()\n".repeat(LARGE_PAIR_ROWS));
    let (mut store, document) = fixture(&text, "javascript");
    let model = analyze(&mut store, document, "javascript");
    let opening = text.rfind('(').unwrap();
    let pairs = model.pairs_in(opening..opening + 2);
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].open, 0..1);
    assert_eq!(pairs[1].open, opening..opening + 1);
    assert_eq!(
        model.active_pair(opening + 1).unwrap().open,
        opening..opening + 1
    );
    let weak = Arc::downgrade(&model);
    drop(pairs);
    drop(model);
    store.release_document(document).unwrap();
    assert!(weak.upgrade().is_none());
}

#[test]
fn 토큰_종류_테이블은_동일한_span과_generation에서도_새_분류로_분석을_바꾼다() {
    let (mut store, document) = fixture("([])", "javascript");
    let mut lines = LineTokens::new(1);
    lines.set_line(0, vec![0, 0], false);
    let code = TokenStyleTable::new(style(TokenKind::Other), vec![style(TokenKind::Other)]);
    let code_model = store
        .bracket_model(
            document,
            Some(rules("javascript")),
            TAB_SIZE,
            tokenized(&lines, &code),
        )
        .unwrap();
    assert_eq!(code_model.brackets().len(), 4);
    let strings = TokenStyleTable::new(style(TokenKind::String), vec![style(TokenKind::String)]);
    let string_model = store
        .bracket_model(
            document,
            Some(rules("javascript")),
            TAB_SIZE,
            tokenized(&lines, &strings),
        )
        .unwrap();
    assert!(string_model.brackets().is_empty());
    assert_eq!(
        string_model.analyzed_line_count(),
        code_model.analyzed_line_count() + 1
    );
}

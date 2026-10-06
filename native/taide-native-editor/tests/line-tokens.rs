use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, LARGE_FILE_BYTES, OpenedFile};
use taide_native_editor::change_journal::{ChangeSet, ChangesSince};
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::line_tokens::{LineTokens, TokenStyle, TokenStyleTable};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::syntax::TokenKind;

const DOCUMENT_COUNT: usize = 4;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 8;
const SYNTHETIC_PATH: &str = "/synthetic/main.rs";
const NUMBERED_LINE_PREFIX: char = 'L';
const UNNUMBERED_LINE_STYLE: u32 = 900;
const KEYWORD_STYLE: u32 = 10;
const NAME_STYLE: u32 = 11;
const OPERATOR_STYLE: u32 = 12;
const NUMBER_STYLE: u32 = 13;
const STATEMENT: &str = "let x = 1;";
const STATEMENT_SPANS: [u32; 8] = [
    0,
    KEYWORD_STYLE,
    4,
    NAME_STYLE,
    6,
    OPERATOR_STYLE,
    8,
    NUMBER_STYLE,
];
const UNKNOWN_STYLE_ID: u32 = 99;

#[test]
fn 새_저장소는_모든_줄이_무효이고_끝_상태가_바뀐_줄만_다음_줄을_무효로_만든다() {
    let mut tokens = LineTokens::new(4);
    assert_eq!(tokens.line_count(), 4);
    assert_eq!(tokens.first_invalid_line(), Some(0));
    assert_eq!(invalid_lines(&tokens), [(0, 4)]);
    assert!(tokens.spans(0).is_empty());
    tokens.set_line(0, vec![0, 7], true);
    assert_eq!(tokens.spans(0), &[0, 7]);
    assert_eq!(tokens.first_invalid_line(), Some(1));
    for line in 1..4 {
        tokens.set_line(line, vec![0, 7], true);
    }
    assert_eq!(tokens.first_invalid_line(), None);
    tokens.invalidate(1..2);
    assert_eq!(tokens.first_invalid_line(), Some(1));
    assert!(tokens.has_accurate_tokens(0));
    assert!(!tokens.has_accurate_tokens(1));
    assert!(!tokens.has_accurate_tokens(3));
    assert!(tokens.is_valid(3));
    tokens.set_line(1, vec![0, 9], false);
    assert_eq!(tokens.first_invalid_line(), None);
    tokens.set_line(1, vec![0, 9], true);
    assert_eq!(invalid_lines(&tokens), [(2, 3)]);
    tokens.set_line(2, vec![0, 9], false);
    tokens.set_line(3, vec![0, 9], true);
    assert_eq!(tokens.first_invalid_line(), None);
    assert!(tokens.spans(4).is_empty());
}

#[test]
fn 줄_삽입과_삭제_뒤에도_남은_토큰은_자기_줄을_따라간다() {
    let mut store = store();
    let document = open(&mut store, "L0\nL1\nL2\nL3\nL4");
    let mut tokens = tokenized(&store, document);
    assert_eq!(tokens.first_invalid_line(), None);

    apply(&mut store, document, vec![(6..6, "X\nY\n")], 0);
    assert_eq!(text(&store, document), "L0\nL1\nX\nY\nL2\nL3\nL4");
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(tokens.line_count(), 7);
    assert_eq!(invalid_lines(&tokens), [(2, 5)]);
    assert_eq!(tokens.spans(1), &[0, 1]);
    assert!(tokens.spans(2).is_empty());
    assert!(tokens.spans(3).is_empty());
    assert!(tokens.spans(4).is_empty());
    assert_eq!(tokens.spans(5), &[0, 3]);
    assert_eq!(tokens.spans(6), &[0, 4]);
    assert_tokens_follow_lines(&store, document, &tokens);

    retokenize_invalid(&store, document, &mut tokens);
    assert_eq!(tokens.first_invalid_line(), None);
    assert_eq!(tokens.spans(2), &[0, UNNUMBERED_LINE_STYLE]);
    assert_eq!(tokens.spans(4), &[0, 2]);

    apply(&mut store, document, vec![(3..6, "")], 1);
    assert_eq!(text(&store, document), "L0\nX\nY\nL2\nL3\nL4");
    assert!(follow(&store, document, 1, &mut tokens));
    assert_eq!(tokens.line_count(), 6);
    assert_eq!(invalid_lines(&tokens), [(1, 2)]);
    assert_eq!(tokens.spans(1), &[0, UNNUMBERED_LINE_STYLE]);
    assert_eq!(tokens.spans(3), &[0, 2]);
    assert_eq!(tokens.spans(5), &[0, 4]);
    assert_tokens_follow_lines(&store, document, &tokens);
}

#[test]
fn undo와_redo의_저널도_줄_이동과_첫_무효_줄을_맞춘다() {
    let mut store = store();
    let document = open(&mut store, "L0\nL1\nL2\nL3");
    let mut tokens = tokenized(&store, document);
    apply(&mut store, document, vec![(3..6, "")], 0);
    assert_eq!(text(&store, document), "L0\nL2\nL3");
    assert!(follow(&store, document, 0, &mut tokens));
    retokenize_invalid(&store, document, &mut tokens);

    assert!(store.undo(document).unwrap());
    assert_eq!(text(&store, document), "L0\nL1\nL2\nL3");
    assert!(follow(&store, document, 1, &mut tokens));
    assert_eq!(tokens.line_count(), 4);
    assert_eq!(invalid_lines(&tokens), [(1, 3)]);
    assert_eq!(tokens.spans(0), &[0, 0]);
    assert_eq!(tokens.spans(3), &[0, 3]);
    assert_tokens_follow_lines(&store, document, &tokens);
    retokenize_invalid(&store, document, &mut tokens);

    assert!(store.redo(document).unwrap());
    assert_eq!(text(&store, document), "L0\nL2\nL3");
    assert!(follow(&store, document, 2, &mut tokens));
    assert_eq!(tokens.line_count(), 3);
    assert_eq!(invalid_lines(&tokens), [(1, 2)]);
    assert_eq!(tokens.spans(2), &[0, 3]);
    assert_tokens_follow_lines(&store, document, &tokens);
}

#[test]
fn 한_줄_안의_삽입은_뒤_토큰을_밀고_경계에서는_앞_토큰을_늘린다() {
    assert_eq!(
        edited_statement_spans(vec![(5..5, "yz")]),
        vec![
            0,
            KEYWORD_STYLE,
            4,
            NAME_STYLE,
            8,
            OPERATOR_STYLE,
            10,
            NUMBER_STYLE
        ]
    );
    assert_eq!(
        edited_statement_spans(vec![(4..4, "yz")]),
        vec![
            0,
            KEYWORD_STYLE,
            6,
            NAME_STYLE,
            8,
            OPERATOR_STYLE,
            10,
            NUMBER_STYLE
        ]
    );
    assert_eq!(
        edited_statement_spans(vec![(0..0, "yz")]),
        vec![
            0,
            KEYWORD_STYLE,
            6,
            NAME_STYLE,
            8,
            OPERATOR_STYLE,
            10,
            NUMBER_STYLE
        ]
    );
    assert_eq!(
        edited_statement_spans(vec![(10..10, "yz")]),
        STATEMENT_SPANS.to_vec()
    );
}

#[test]
fn 한_줄_안의_삭제와_치환은_지워진_토큰을_없애고_남은_토큰을_당긴다() {
    assert_eq!(
        edited_statement_spans(vec![(0..2, "")]),
        vec![
            0,
            KEYWORD_STYLE,
            2,
            NAME_STYLE,
            4,
            OPERATOR_STYLE,
            6,
            NUMBER_STYLE
        ]
    );
    assert_eq!(
        edited_statement_spans(vec![(2..7, "")]),
        vec![0, KEYWORD_STYLE, 2, OPERATOR_STYLE, 3, NUMBER_STYLE]
    );
    assert_eq!(
        edited_statement_spans(vec![(4..5, "name")]),
        vec![
            0,
            KEYWORD_STYLE,
            8,
            NAME_STYLE,
            9,
            OPERATOR_STYLE,
            11,
            NUMBER_STYLE
        ]
    );
    assert_eq!(
        edited_statement_spans(vec![(0..10, "")]),
        vec![0, NUMBER_STYLE]
    );
}

#[test]
fn 같은_줄의_여러_편집은_뒤에서부터_반영해_앞_편집의_위치를_지킨다() {
    let mut store = store();
    let document = open(&mut store, "aa bb cc");
    let mut tokens = LineTokens::new(1);
    tokens.set_line(0, vec![0, 1, 3, 2, 6, 3], true);
    apply(
        &mut store,
        document,
        vec![(0..0, "X"), (3..3, "X"), (6..6, "X")],
        0,
    );
    assert_eq!(text(&store, document), "Xaa Xbb Xcc");
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(tokens.spans(0), &[0, 1, 5, 2, 9, 3]);
    assert_eq!(invalid_lines(&tokens), [(0, 1)]);
}

#[test]
fn 줄을_합치는_삭제는_마지막_줄의_남은_토큰을_첫_줄_뒤에_붙인다() {
    let mut store = store();
    let document = open(&mut store, "ab cd\nef gh\nzz");
    let mut tokens = LineTokens::new(3);
    tokens.set_line(0, vec![0, 1, 3, 2], true);
    tokens.set_line(1, vec![0, 3, 3, 4], true);
    tokens.set_line(2, vec![0, 5], true);
    apply(&mut store, document, vec![(4..8, "")], 0);
    assert_eq!(text(&store, document), "ab c gh\nzz");
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(tokens.line_count(), 2);
    assert_eq!(tokens.spans(0), &[0, 1, 3, 2, 4, 3, 5, 4]);
    assert_eq!(tokens.spans(1), &[0, 5]);
    assert_eq!(invalid_lines(&tokens), [(0, 1)]);
}

#[test]
fn 줄을_나누는_삽입은_삽입_위치_뒤의_토큰을_버리고_새_줄을_빈_토큰으로_둔다() {
    let mut store = store();
    let document = open(&mut store, "ab cd\nzz");
    let mut tokens = LineTokens::new(2);
    tokens.set_line(0, vec![0, 1, 3, 2], true);
    tokens.set_line(1, vec![0, 5], true);
    apply(&mut store, document, vec![(1..1, "X\nY")], 0);
    assert_eq!(text(&store, document), "aX\nYb cd\nzz");
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(tokens.line_count(), 3);
    assert_eq!(tokens.spans(0), &[0, 1]);
    assert!(tokens.spans(1).is_empty());
    assert_eq!(tokens.spans(2), &[0, 5]);
    assert_eq!(invalid_lines(&tokens), [(0, 2)]);
}

#[test]
fn 떨어진_두_편집은_각각의_무효_범위로_남고_사이_줄은_유효하다() {
    let mut store = store();
    let document = open(&mut store, "L0\nL1\nL2\nL3\nL4\nL5");
    let mut tokens = tokenized(&store, document);
    apply(
        &mut store,
        document,
        vec![(3..5, "L7"), (12..14, "L8\nL9")],
        0,
    );
    assert_eq!(text(&store, document), "L0\nL7\nL2\nL3\nL8\nL9\nL5");
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(invalid_lines(&tokens), [(1, 2), (4, 6)]);
    assert!(tokens.is_valid(2));
    assert!(tokens.is_valid(3));
    assert!(tokens.is_valid(6));
    assert_eq!(tokens.spans(2), &[0, 2]);
    assert_eq!(tokens.spans(6), &[0, 5]);
    tokens.set_line(1, vec![0, 7], false);
    assert_eq!(tokens.first_invalid_line(), Some(4));
}

#[test]
fn 줄_수가_맞지_않는_변경과_reset은_저장소를_전부_무효로_되돌린다() {
    let mut store = store();
    let document = open(&mut store, "L0\nL1\nL2");
    let mut tokens = tokenized(&store, document);
    apply(&mut store, document, vec![(2..2, "\nL5")], 0);
    apply(&mut store, document, vec![(0..0, "L6\n")], 1);
    let sets = tracked(&store, document, 0);
    assert!(!tokens.apply(&sets[1]));
    assert_eq!(tokens.line_count(), 5);
    assert_eq!(invalid_lines(&tokens), [(0, 5)]);
    assert!((0..5).all(|line| tokens.spans(line).is_empty()));

    let mut tokens = tokenized(&store, document);
    tokens.reset(2);
    assert_eq!(tokens.line_count(), 2);
    assert_eq!(invalid_lines(&tokens), [(0, 2)]);
    assert!(tokens.spans(0).is_empty());
}

#[test]
fn cr_뒤에_lf를_넣은_변경도_줄_수를_어긋나게_하지_않는다() {
    let mut store = store();
    let document = open(&mut store, "a\rb\nc");
    let mut tokens = LineTokens::new(3);
    for line in 0..3 {
        tokens.set_line(line, vec![0, 1], true);
    }
    apply(&mut store, document, vec![(2..2, "\n")], 0);
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(tokens.line_count(), 3);
    assert_eq!(tokens.spans(2), &[0, 1]);
    assert!(tokens.first_invalid_line().is_some());
}

#[test]
fn 스타일_표는_범위_밖_스타일_id에_기본_스타일을_돌려준다() {
    let default_style = TokenStyle {
        foreground: [1, 2, 3, u8::MAX],
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    };
    let comment_style = TokenStyle {
        foreground: [4, 5, 6, u8::MAX],
        is_italic: true,
        kind: TokenKind::Comment,
        ..default_style
    };
    let table = TokenStyleTable::new(default_style, vec![default_style, comment_style]);
    assert_eq!(table.len(), 2);
    assert_eq!(table.style(1), comment_style);
    assert_eq!(table.style(UNKNOWN_STYLE_ID), default_style);
    assert_eq!(table.default_style(), default_style);
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: LARGE_FILE_BYTES as usize,
    })
    .unwrap()
}

fn open(store: &mut EditorStore, content: &str) -> DocumentId {
    store
        .open_file(
            PathBuf::from(SYNTHETIC_PATH),
            OpenedFile {
                path: SYNTHETIC_PATH.into(),
                content: content.into(),
                language_id: "rust".into(),
                byte_size: content.len().try_into().unwrap(),
                line_count: content.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap()
}

fn text(store: &EditorStore, document: DocumentId) -> String {
    store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string()
}

fn apply(
    store: &mut EditorStore,
    document: DocumentId,
    edits: Vec<(std::ops::Range<usize>, &str)>,
    group: u64,
) -> u64 {
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply_separate(
            document,
            Transaction {
                revision,
                edits: edits
                    .into_iter()
                    .map(|(bytes, text)| Edit {
                        bytes,
                        text: text.into(),
                    })
                    .collect(),
                group: UndoGroup(group),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap()
}

fn invalid_lines(tokens: &LineTokens) -> Vec<(usize, usize)> {
    tokens
        .invalid_ranges()
        .iter()
        .map(|range| (range.start, range.end))
        .collect()
}

fn tracked(store: &EditorStore, document: DocumentId, revision: u64) -> Vec<ChangeSet> {
    match store.changes_since(document, revision).unwrap() {
        ChangesSince::Tracked(changes) => changes.cloned().collect(),
        ChangesSince::Lagged => panic!("journal lagged behind revision {revision}"),
    }
}

fn follow(
    store: &EditorStore,
    document: DocumentId,
    revision: u64,
    tokens: &mut LineTokens,
) -> bool {
    tracked(store, document, revision)
        .iter()
        .all(|set| tokens.apply(set))
}

fn line_style(line: &str) -> u32 {
    line.strip_prefix(NUMBERED_LINE_PREFIX)
        .and_then(|number| number.parse().ok())
        .unwrap_or(UNNUMBERED_LINE_STYLE)
}

fn tokenized(store: &EditorStore, document: DocumentId) -> LineTokens {
    let content = text(store, document);
    let mut tokens = LineTokens::new(content.split('\n').count());
    for (index, line) in content.split('\n').enumerate() {
        tokens.set_line(index, vec![0, line_style(line)], true);
    }
    tokens
}

fn retokenize_invalid(store: &EditorStore, document: DocumentId, tokens: &mut LineTokens) {
    let content = text(store, document);
    let lines: Vec<&str> = content.split('\n').collect();
    while let Some(line) = tokens.first_invalid_line() {
        tokens.set_line(line, vec![0, line_style(lines[line])], false);
    }
}

fn assert_tokens_follow_lines(store: &EditorStore, document: DocumentId, tokens: &LineTokens) {
    let content = text(store, document);
    assert_eq!(tokens.line_count(), content.split('\n').count());
    for (index, line) in content.split('\n').enumerate() {
        let spans = tokens.spans(index);
        assert!(
            !tokens.is_valid(index) || spans == [0, line_style(line)],
            "line {index} {line:?} holds {spans:?}"
        );
    }
}

fn edited_statement_spans(edits: Vec<(std::ops::Range<usize>, &str)>) -> Vec<u32> {
    let mut store = store();
    let document = open(&mut store, STATEMENT);
    let mut tokens = LineTokens::new(1);
    tokens.set_line(0, STATEMENT_SPANS.to_vec(), true);
    apply(&mut store, document, edits, 0);
    assert!(follow(&store, document, 0, &mut tokens));
    assert_eq!(invalid_lines(&tokens), [(0, 1)]);
    tokens.spans(0).to_vec()
}

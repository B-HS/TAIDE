use std::ops::Range;
use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, LARGE_FILE_BYTES, OpenedFile};
use taide_native_editor::change_journal::ChangesSince;
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

use super::{
    DocumentTokens, MAX_TOKENIZED_DOCUMENT_LINES, MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH,
    is_too_large_for_tokenization,
};
use crate::bundled_grammars::bundled_grammar_set;
use crate::textmate_tokenizer::{TextmateTokenizer, TokenizerLimits};
use crate::theme_settings::{ThemeSetting, ThemeStyle};

const LANGUAGE_ID: &str = "rust";
const SYNTHETIC_PATH: &str = "/synthetic/main.rs";
const DOCUMENT_COUNT: usize = 2;
const VIEW_COUNT: usize = 2;
const HISTORY_COUNT: usize = 8;
const SOURCE: &str = "fn main() {\n    let value = 1;\n    // note\n    let text = \"a\";\n    println!(\"{text}\");\n}\n\nfn other() {}";
const SOURCE_LINE_COUNT: usize = 8;

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

fn tokenizer() -> TextmateTokenizer {
    TextmateTokenizer::new(
        &bundled_grammar_set(&[LANGUAGE_ID]).unwrap(),
        &[
            setting(None, "#d4d4d4"),
            setting(Some("comment"), "#6a9955"),
            setting(Some("string"), "#ce9178"),
            setting(Some("keyword"), "#569cd6"),
            setting(Some("storage"), "#c586c0"),
        ],
        TokenizerLimits::default(),
    )
    .unwrap()
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
                language_id: LANGUAGE_ID.into(),
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

fn edit(
    store: &mut EditorStore,
    document: DocumentId,
    tokens: &mut DocumentTokens,
    edits: Vec<(Range<usize>, &str)>,
) {
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
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    follow(store, document, revision, tokens);
}

fn follow(store: &EditorStore, document: DocumentId, revision: u64, tokens: &mut DocumentTokens) {
    let ChangesSince::Tracked(changes) = store.changes_since(document, revision).unwrap() else {
        panic!("journal lagged behind revision {revision}");
    };
    for change in changes {
        assert!(tokens.apply(change));
    }
}

fn byte_of(content: &str, needle: &str) -> usize {
    content.find(needle).unwrap()
}

fn retokenize(
    tokens: &mut DocumentTokens,
    tokenizer: &mut TextmateTokenizer,
    content: &str,
) -> Vec<usize> {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut tokenized_lines = Vec::new();
    while let Some(plan) = tokens.plan() {
        let mut state = plan.start_state.clone();
        let mut line = plan.start_line;
        loop {
            let tokenized = tokenizer
                .try_tokenize_line(LANGUAGE_ID, lines[line], state.as_ref())
                .unwrap();
            let is_converged = plan
                .known_end_state(line)
                .is_some_and(|known| known.is_same(&tokenized.end_state));
            state = Some(tokenized.end_state.clone());
            tokens.accept(line, vec![tokenized]);
            tokenized_lines.push(line);
            line += 1;
            if is_converged || line >= lines.len() {
                break;
            }
        }
    }
    tokenized_lines
}

fn assert_matches_full_tokenization(tokens: &DocumentTokens, content: &str) {
    let mut tokenizer = tokenizer();
    let mut state = None;
    let lines: Vec<&str> = content.split('\n').collect();
    assert_eq!(tokens.tokens().line_count(), lines.len());
    assert_eq!(tokens.tokens().first_invalid_line(), None);
    for (index, line) in lines.iter().enumerate() {
        let tokenized = tokenizer
            .try_tokenize_line(LANGUAGE_ID, line, state.as_ref())
            .unwrap();
        assert_eq!(
            tokens.tokens().spans(index),
            tokenized.spans,
            "line {index} {line:?}"
        );
        state = Some(tokenized.end_state);
    }
}

fn tokenized_source() -> (EditorStore, DocumentId, DocumentTokens, TextmateTokenizer) {
    let mut store = store();
    let document = open(&mut store, SOURCE);
    let mut tokenizer = tokenizer();
    let mut tokens = DocumentTokens::new(SOURCE_LINE_COUNT);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, SOURCE),
        (0..SOURCE_LINE_COUNT).collect::<Vec<_>>()
    );
    assert_matches_full_tokenization(&tokens, SOURCE);
    (store, document, tokens, tokenizer)
}

#[test]
fn 처음_계획은_첫_줄에서_시작_상태_없이_출발하고_마지막_줄의_알려진_상태가_없다() {
    let mut tokens = DocumentTokens::new(SOURCE_LINE_COUNT);
    let plan = tokens.plan().unwrap();
    assert_eq!(plan.start_line, 0);
    assert!(plan.start_state.is_none());
    assert_eq!(plan.first_known_line, SOURCE_LINE_COUNT - 1);
    assert_eq!(plan.known_end_states.len(), 1);
    assert!(plan.known_end_state(SOURCE_LINE_COUNT - 1).is_none());
    assert!(plan.known_end_state(0).is_none());
}

#[test]
fn 끝_상태가_그대로인_한_줄_편집은_그_줄만_다시_토큰화한다() {
    let (mut store, document, mut tokens, mut tokenizer) = tokenized_source();
    let start = byte_of(SOURCE, "value");
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(start..start + "value".len(), "renamed")],
    );
    let plan = tokens.plan().unwrap();
    assert_eq!((plan.start_line, plan.first_known_line), (1, 1));
    assert!(plan.start_state.is_some());
    assert!(plan.known_end_state(1).is_some());
    let content = text(&store, document);
    assert_eq!(retokenize(&mut tokens, &mut tokenizer, &content), vec![1]);
    assert_matches_full_tokenization(&tokens, &content);
}

#[test]
fn 블록_주석을_열고_닫는_편집은_끝_상태가_달라진_줄을_모두_다시_토큰화한다() {
    let (mut store, document, mut tokens, mut tokenizer) = tokenized_source();
    let comment_line = byte_of(SOURCE, "    // note");
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(comment_line..comment_line, "/*")],
    );
    let opened = text(&store, document);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, &opened),
        (2..SOURCE_LINE_COUNT).collect::<Vec<_>>()
    );
    assert_matches_full_tokenization(&tokens, &opened);

    let call_end = byte_of(&opened, "println!(\"{text}\");") + "println!(\"{text}\");".len();
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(call_end..call_end, "*/")],
    );
    let closed = text(&store, document);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, &closed),
        (4..SOURCE_LINE_COUNT).collect::<Vec<_>>()
    );
    assert_matches_full_tokenization(&tokens, &closed);
}

#[test]
fn 줄을_나누고_합치는_편집은_바뀐_줄_범위에서_멈춘다() {
    let (mut store, document, mut tokens, mut tokenizer) = tokenized_source();
    let split = byte_of(SOURCE, "let text");
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(split..split, "let extra = 2;\n    ")],
    );
    let split_content = text(&store, document);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, &split_content),
        vec![3, 4]
    );
    assert_matches_full_tokenization(&tokens, &split_content);

    let removed_start = byte_of(&split_content, "    // note");
    let removed_end = byte_of(&split_content, "    let text");
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(removed_start..removed_end, "")],
    );
    let joined_content = text(&store, document);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, &joined_content),
        vec![2]
    );
    assert_matches_full_tokenization(&tokens, &joined_content);
}

#[test]
fn 떨어진_두_편집은_각_범위에서_따로_멈춘다() {
    let (mut store, document, mut tokens, mut tokenizer) = tokenized_source();
    let first = byte_of(SOURCE, "value");
    let second = byte_of(SOURCE, "other");
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(first..first, "new_"), (second..second, "an")],
    );
    let content = text(&store, document);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, &content),
        vec![1, 7]
    );
    assert_matches_full_tokenization(&tokens, &content);
}

#[test]
fn undo와_redo도_바뀐_줄에서_다시_토큰화를_시작해_전체와_같은_결과가_된다() {
    let (mut store, document, mut tokens, mut tokenizer) = tokenized_source();
    let comment_line = byte_of(SOURCE, "    // note");
    edit(
        &mut store,
        document,
        &mut tokens,
        vec![(comment_line..comment_line, "/*")],
    );
    let opened = text(&store, document);
    retokenize(&mut tokens, &mut tokenizer, &opened);

    assert!(store.undo(document).unwrap());
    follow(&store, document, 1, &mut tokens);
    assert_eq!(
        retokenize(&mut tokens, &mut tokenizer, SOURCE),
        (2..SOURCE_LINE_COUNT).collect::<Vec<_>>()
    );
    assert_matches_full_tokenization(&tokens, SOURCE);

    assert!(store.redo(document).unwrap());
    follow(&store, document, 2, &mut tokens);
    retokenize(&mut tokens, &mut tokenizer, &opened);
    assert_matches_full_tokenization(&tokens, &opened);
}

#[test]
fn 문서_한도는_utf16_길이_20mb와_30만_줄을_넘을_때부터_걸린다() {
    assert!(!is_too_large_for_tokenization(
        MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH,
        MAX_TOKENIZED_DOCUMENT_LINES
    ));
    assert!(is_too_large_for_tokenization(
        MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH + 1,
        1
    ));
    assert!(is_too_large_for_tokenization(
        0,
        MAX_TOKENIZED_DOCUMENT_LINES + 1
    ));
    assert_eq!(MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH, 20_971_520);
    assert_eq!(MAX_TOKENIZED_DOCUMENT_LINES, 300_000);
}

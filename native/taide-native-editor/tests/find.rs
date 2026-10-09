use std::time::Duration;

use ropey::Rope;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::editing::type_text;
use taide_native_editor::find::{
    FIND_MATCH_LIMIT, FindOptions, FindPattern, FindPatternCompiler, FindPatternError,
    FindPatternOptions, FindQuery, FindSeedOptions, is_multiline_regex_source, seed_find_text,
};
use taide_native_editor::find_replacement::{
    ReplacePattern, apply_replacement_edits, replacement_edits,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};

const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024 * 1024;

struct ForbiddenCompiler;

impl FindPatternCompiler for ForbiddenCompiler {
    fn compile(
        &self,
        _: &str,
        _: FindPatternOptions,
    ) -> Result<Box<dyn FindPattern>, FindPatternError> {
        panic!("literal case-sensitive search must not compile a pattern")
    }
}

fn literal(source: &str, whole_word: bool) -> FindQuery {
    FindQuery::new(
        source,
        FindOptions {
            match_case: true,
            whole_word,
            ..Default::default()
        },
        &ForbiddenCompiler,
    )
    .unwrap()
}

fn fixture(text: &str, read_only: bool) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 2,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/find.txt".into(),
            OpenedFile {
                path: "/synthetic/find.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, view)
}

fn snapshot(store: &EditorStore, view: ViewId) -> DocumentSnapshot {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
}

fn caret(byte: usize) -> SelectionSet {
    SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: byte,
            head: byte,
        }],
    }
}

#[test]
fn 찾기_시드는_주_선택과_단어와_정규식_이스케이프를_사용한다() {
    let (store, view) = fixture("a.b\ncat", false);
    let document = snapshot(&store, view);
    let selection = SelectionSet {
        primary: 1,
        selections: vec![
            Selection { anchor: 4, head: 7 },
            Selection { anchor: 3, head: 0 },
        ],
    };
    assert_eq!(
        seed_find_text(
            &document,
            &selection,
            None,
            FindSeedOptions {
                is_regex: true,
                ..Default::default()
            }
        ),
        Some("a\\.b".to_string())
    );
    assert_eq!(
        seed_find_text(&document, &caret(5), None, FindSeedOptions::default()),
        Some("cat".to_string())
    );
    assert!(
        seed_find_text(
            &document,
            &caret(5),
            None,
            FindSeedOptions {
                require_selection: true,
                ..Default::default()
            }
        )
        .is_none()
    );
    let selection = SelectionSet {
        primary: 0,
        selections: vec![Selection { anchor: 0, head: 7 }],
    };
    assert!(seed_find_text(&document, &selection, None, FindSeedOptions::default()).is_none());
    assert_eq!(
        seed_find_text(
            &document,
            &selection,
            None,
            FindSeedOptions {
                allow_multiline: true,
                ..Default::default()
            }
        ),
        Some("a.b\ncat".to_string())
    );
}

#[test]
fn 대량_모두_바꾸기는_강조_한도를_넘어도_단어_옵션을_보존하며_한번에_undo된다() {
    let count = FIND_MATCH_LIMIT + 1;
    let original = "cat concatenate ".repeat(count);
    let (mut store, view) = fixture(&original, false);
    let document = snapshot(&store, view);
    let edits = replacement_edits(
        &literal("cat", true),
        &document,
        &[],
        &ReplacePattern::new("dog", false),
        false,
        None,
    )
    .unwrap();
    assert_eq!(edits.len(), count);
    assert!(apply_replacement_edits(&mut store, view, document.revision, edits).unwrap());
    assert_eq!(
        snapshot(&store, view).rope.to_string(),
        "dog concatenate ".repeat(count)
    );
    assert!(store.undo(document.id).unwrap());
    assert_eq!(snapshot(&store, view).rope.to_string(), original);
    assert!(store.redo(document.id).unwrap());
    assert_eq!(
        snapshot(&store, view).rope.to_string(),
        "dog concatenate ".repeat(count)
    );
}

#[test]
fn 바꾸기는_crlf와_다른_뷰의_선택을_보존하며_undo로_복원한다() {
    let original = "cat\r\ncat";
    let (mut store, view) = fixture(original, false);
    let document = snapshot(&store, view);
    let other = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document.id,
        )
        .unwrap();
    let state = store.views().get(other).unwrap().clone();
    store
        .set_view_state(other, caret(original.len()), state.scroll, state.folds)
        .unwrap();
    let edits = replacement_edits(
        &literal("cat", false),
        &document,
        &[],
        &ReplacePattern::new("dogs\\nnext", true),
        false,
        None,
    )
    .unwrap();
    assert!(edits.iter().all(|edit| edit.text == "dogs\r\nnext"));
    assert!(apply_replacement_edits(&mut store, view, document.revision, edits).unwrap());
    let replaced = "dogs\r\nnext\r\ndogs\r\nnext";
    assert_eq!(snapshot(&store, view).rope.to_string(), replaced);
    assert_eq!(
        store.views().get(other).unwrap().selection,
        caret(replaced.len())
    );
    assert!(store.undo(document.id).unwrap());
    assert_eq!(snapshot(&store, view).rope.to_string(), original);
    assert_eq!(
        store.views().get(other).unwrap().selection,
        caret(original.len())
    );
}

#[test]
fn 오래된_revision과_읽기전용_문서는_치환하지_않는다() {
    let (mut store, view) = fixture("cat", false);
    let document = snapshot(&store, view);
    let edits = replacement_edits(
        &literal("cat", false),
        &document,
        &[],
        &ReplacePattern::new("dog", false),
        false,
        None,
    )
    .unwrap();
    type_text(&mut store, view, "!").unwrap();
    let text = snapshot(&store, view).rope.to_string();
    assert_eq!(
        apply_replacement_edits(&mut store, view, document.revision, edits),
        Err(EditorError::StaleRevision)
    );
    assert_eq!(snapshot(&store, view).rope.to_string(), text);
    let (mut store, view) = fixture("cat", true);
    let document = snapshot(&store, view);
    let edits = replacement_edits(
        &literal("cat", false),
        &document,
        &[],
        &ReplacePattern::new("dog", false),
        false,
        None,
    )
    .unwrap();
    assert_eq!(
        apply_replacement_edits(&mut store, view, document.revision, edits),
        Err(EditorError::ReadOnly)
    );
    assert_eq!(snapshot(&store, view).rope.to_string(), "cat");
}

#[test]
fn 선택_영역_치환과_시간초과는_범위_밖을_변경하지_않는다() {
    let (mut store, view) = fixture("cat cat cat", false);
    let document = snapshot(&store, view);
    let query = literal("cat", false);
    let pattern = ReplacePattern::new("dog", false);
    assert!(
        replacement_edits(
            &query,
            &document,
            &[],
            &pattern,
            false,
            Some(Duration::ZERO)
        )
        .is_err()
    );
    assert_eq!(snapshot(&store, view).rope.to_string(), "cat cat cat");
    let edits = replacement_edits(&query, &document, &[4..7], &pattern, false, None).unwrap();
    assert!(apply_replacement_edits(&mut store, view, document.revision, edits).unwrap());
    assert_eq!(snapshot(&store, view).rope.to_string(), "cat dog cat");
}

#[test]
fn 리터럴과_빈_검색은_정규식_엔진_없이_작동한다() {
    let rope = Rope::from_str("a.a aa a.a");
    let results = literal("a.a", false)
        .find_matches(&rope, &[], FIND_MATCH_LIMIT, None)
        .unwrap();
    assert_eq!(
        results
            .matches
            .iter()
            .map(|found| found.range.clone())
            .collect::<Vec<_>>(),
        [0..3, 7..10]
    );
    assert!(
        literal("", false)
            .find_matches(&rope, &[], FIND_MATCH_LIMIT, None)
            .unwrap()
            .matches
            .is_empty()
    );
}

#[test]
fn 단어_경계는_밑줄과_한글과_유니코드_구두점을_정규식_단어로_취급하지_않는다() {
    let rope = Rope::from_str("한cat cat— cat_ cat cat-cat");
    let results = literal("cat", true)
        .find_matches(&rope, &[], FIND_MATCH_LIMIT, None)
        .unwrap();
    let text = rope.to_string();
    let expected = text
        .match_indices("cat")
        .skip(3)
        .map(|(start, _)| start..start + "cat".len())
        .collect::<Vec<_>>();
    assert_eq!(
        results
            .matches
            .iter()
            .map(|found| found.range.clone())
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn 여러_선택_범위를_정렬하고_겹치는_일치를_한_번만_수집한다() {
    let rope = Rope::from_str("cat cat cat");
    let results = literal("cat", false)
        .find_matches(&rope, &[4..11, 0..7, 0..7], FIND_MATCH_LIMIT, None)
        .unwrap();
    assert_eq!(
        results
            .matches
            .iter()
            .map(|found| found.range.clone())
            .collect::<Vec<_>>(),
        [0..3, 4..7, 8..11]
    );
}

#[test]
fn 일치_한도와_시간_중단을_부분_결과에_표시한다() {
    let rope = Rope::from_str("cat cat cat");
    let query = literal("cat", false);
    let limited = query.find_matches(&rope, &[], 1, None).unwrap();
    assert_eq!(limited.matches.len(), 1);
    assert!(limited.limit_reached);
    assert!(!limited.timed_out);
    let timed = query
        .find_matches(&rope, &[], FIND_MATCH_LIMIT, Some(Duration::ZERO))
        .unwrap();
    assert!(timed.matches.is_empty());
    assert!(timed.timed_out);
    assert!(
        query
            .find_next(&rope, 0, true, Some(Duration::ZERO))
            .is_err()
    );
    assert!(
        query
            .find_previous(&rope, rope.len_bytes(), true, Some(Duration::ZERO))
            .is_err()
    );
}

#[test]
fn 잘못된_utf8_범위는_패닉_없이_거부한다() {
    let rope = Rope::from_str("한cat");
    let query = literal("cat", false);
    assert!(
        query
            .find_matches(&rope, &[1..rope.len_bytes()], FIND_MATCH_LIMIT, None)
            .is_err()
    );
    assert!(query.find_next(&rope, 1, true, None).is_err());
    assert!(
        query
            .find_previous(&rope, rope.len_bytes() + 1, true, None)
            .is_err()
    );
}

#[test]
fn 순환을_끄면_문서_양끝을_넘어_탐색하지_않는다() {
    let rope = Rope::from_str("cat\ncat");
    let query = literal("cat", false);
    assert!(
        query
            .find_next(&rope, rope.len_bytes(), false, None)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        query
            .find_next(&rope, rope.len_bytes(), true, None)
            .unwrap()
            .unwrap()
            .range,
        0..3
    );
    assert!(
        query
            .find_previous(&rope, 0, false, None)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        query
            .find_previous(&rope, 0, true, None)
            .unwrap()
            .unwrap()
            .range,
        4..7
    );
}

#[test]
fn 여러_줄_리터럴은_crlf를_lf로_검색하고_원래_바이트_범위로_복원한다() {
    let rope = Rope::from_str("한\r\ncat\r\ncat");
    let query = literal("한\ncat", false);
    let results = query
        .find_matches(&rope, &[], FIND_MATCH_LIMIT, None)
        .unwrap();
    assert_eq!(results.matches[0].range, 0..8);
    assert_eq!(results.matches[0].captures, [Some("한\ncat".to_string())]);
    let query = literal("cat\ncat", false);
    assert_eq!(
        query
            .find_next(&rope, 5, true, None)
            .unwrap()
            .unwrap()
            .range,
        5..13
    );
}

#[test]
fn 여러_줄_판정은_원본의_lf와_n_r_대문자_w_표기만_인식한다() {
    for source in ["a\nb", r"\n", r"\r", r"\W+"] {
        assert!(is_multiline_regex_source(source), "{source}");
    }
    for source in ["", r"\\n", r"\s+", r"\w+", "[^a]", "."] {
        assert!(!is_multiline_regex_source(source), "{source}");
    }
}

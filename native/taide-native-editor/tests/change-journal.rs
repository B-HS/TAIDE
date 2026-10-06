use std::path::PathBuf;

use taide_model::file::{EditorConfigOptions, FileSizeTier, LARGE_FILE_BYTES, OpenedFile};
use taide_native_editor::change_journal::{
    ChangeSet, ChangesSince, LinePoint, MAX_CHANGE_SET_SPANS, MAX_JOURNAL_ENTRIES,
};
use taide_native_editor::document::{DocumentId, Edit, EditorError, UndoGroup};
use taide_native_editor::store::{DocumentVersion, EditorLimits, EditorStore, Transaction};

const DOCUMENT_COUNT: usize = 4;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 8;
const SYNTHETIC_PATH: &str = "/synthetic/main.rs";
const WIDE_EDIT_LINES: usize = MAX_CHANGE_SET_SPANS + 44;

#[test]
fn 문서_버전_목록은_복제_없이_id와_revision과_언어를_돌려준다() {
    let mut store = store();
    assert_eq!(store.documents().versions().count(), 0);
    let document = open(&mut store, "one");
    apply(&mut store, document, vec![(3..3, "!")], 0);
    assert!(store.undo(document).unwrap());
    let versions: Vec<DocumentVersion<'_>> = store.documents().versions().collect();
    assert_eq!(
        versions,
        vec![DocumentVersion {
            id: document,
            revision: 2,
            language_id: "rust",
        }]
    );
}

#[test]
fn 편집_뒤_저널_span은_실제_변경_범위와_줄_위치를_덮는다() {
    let mut store = store();
    let document = open(&mut store, "ab\ncd\nef");
    apply(&mut store, document, vec![(1..4, "X")], 0);
    let sets = tracked(&store, document, 0);
    assert_eq!(sets.len(), 1);
    let set = &sets[0];
    assert_eq!((set.revision_before, set.revision_after), (0, 1));
    assert_eq!((set.line_count_before, set.line_count_after), (3, 2));
    assert_eq!(set.spans.len(), 1);
    let span = &set.spans[0];
    assert_eq!(
        (span.start_byte, span.old_end_byte, span.new_end_byte),
        (1, 4, 2)
    );
    assert_eq!(span.start, point(0, 1));
    assert_eq!(span.old_end, point(1, 1));
    assert_eq!(span.new_end, point(0, 2));
    assert_eq!(set.first_changed_line(), Some(0));
    assert_eq!(replay("ab\ncd\nef", "aXd\nef", set), "aXd\nef");
}

#[test]
fn 줄을_늘리는_편집은_새_끝_위치를_삽입한_마지막_줄로_기록한다() {
    let mut store = store();
    let document = open(&mut store, "ab\ncd");
    apply(&mut store, document, vec![(4..4, "1\n22\n333")], 0);
    let sets = tracked(&store, document, 0);
    let span = &sets[0].spans[0];
    assert_eq!(
        (span.start_byte, span.old_end_byte, span.new_end_byte),
        (4, 4, 12)
    );
    assert_eq!(span.start, point(1, 1));
    assert_eq!(span.old_end, point(1, 1));
    assert_eq!(span.new_end, point(3, 3));
    assert_eq!(
        (sets[0].line_count_before, sets[0].line_count_after),
        (2, 4)
    );
    assert_eq!(
        replay("ab\ncd", "ab\nc1\n22\n333d", &sets[0]),
        "ab\nc1\n22\n333d"
    );
}

#[test]
fn 한_transaction의_여러_편집은_편집_전_좌표의_오름차순_span으로_남는다() {
    let mut store = store();
    let before = "one\ntwo\nthree\nfour";
    let document = open(&mut store, before);
    apply(
        &mut store,
        document,
        vec![(14..14, "!\n?"), (0..3, "1"), (4..8, "")],
        0,
    );
    let after = text(&store, document);
    assert_eq!(after, "1\nthree\n!\n?four");
    let sets = tracked(&store, document, 0);
    let set = &sets[0];
    let starts: Vec<usize> = set.spans.iter().map(|span| span.start_byte).collect();
    assert_eq!(starts, vec![0, 4, 14]);
    assert_eq!(set.spans[0].start, point(0, 0));
    assert_eq!(set.spans[0].old_end, point(0, 3));
    assert_eq!(set.spans[0].new_end, point(0, 1));
    assert_eq!(set.spans[1].start, point(1, 0));
    assert_eq!(set.spans[1].old_end, point(2, 0));
    assert_eq!(set.spans[1].new_end, point(1, 0));
    assert_eq!(set.spans[2].start, point(3, 0));
    assert_eq!(set.spans[2].old_end, point(3, 0));
    assert_eq!(set.spans[2].new_end, point(4, 1));
    assert_eq!((set.line_count_before, set.line_count_after), (4, 4));
    assert_eq!(set.first_changed_line(), Some(0));
    assert_eq!(replay(before, &after, set), after);
}

#[test]
fn undo와_redo는_전후_문서의_공통_접두와_접미를_뺀_span_하나를_남긴다() {
    let mut store = store();
    let original = "alpha\nbeta\ngamma\ndelta";
    let document = open(&mut store, original);
    apply(
        &mut store,
        document,
        vec![(6..10, "B\nB"), (17..17, "+")],
        0,
    );
    let edited = text(&store, document);
    assert_eq!(edited, "alpha\nB\nB\ngamma\n+delta");
    assert!(store.undo(document).unwrap());
    assert_eq!(text(&store, document), original);
    let undo_sets = tracked(&store, document, 1);
    assert_eq!(undo_sets.len(), 1);
    let undo = &undo_sets[0];
    assert_eq!((undo.revision_before, undo.revision_after), (1, 2));
    assert_eq!(undo.spans.len(), 1);
    assert_eq!(
        (
            undo.spans[0].start_byte,
            undo.spans[0].old_end_byte,
            undo.spans[0].new_end_byte
        ),
        (6, 17, 17)
    );
    assert_eq!(undo.spans[0].start, point(1, 0));
    assert_eq!(undo.spans[0].old_end, point(4, 1));
    assert_eq!(undo.spans[0].new_end, point(3, 0));
    assert_eq!((undo.line_count_before, undo.line_count_after), (5, 4));
    assert_eq!(replay(&edited, original, undo), original);
    assert!(store.redo(document).unwrap());
    let redo_sets = tracked(&store, document, 2);
    let redo = &redo_sets[0];
    assert_eq!((redo.revision_before, redo.revision_after), (2, 3));
    assert_eq!(redo.spans.len(), 1);
    assert_eq!(redo.spans[0].start, point(1, 0));
    assert_eq!(redo.spans[0].old_end, point(3, 0));
    assert_eq!(redo.spans[0].new_end, point(4, 1));
    assert_eq!(replay(original, &edited, redo), edited);
    assert_eq!(tracked(&store, document, 0).len(), 3);
}

#[test]
fn undo_span은_여러_바이트_문자의_중간에서_끊기지_않는다() {
    let mut store = store();
    let document = open(&mut store, "가나");
    apply(&mut store, document, vec![(0..3, "각")], 0);
    assert_eq!(text(&store, document), "각나");
    assert!(store.undo(document).unwrap());
    let sets = tracked(&store, document, 1);
    let span = &sets[0].spans[0];
    assert_eq!(
        (span.start_byte, span.old_end_byte, span.new_end_byte),
        (0, 3, 3)
    );
    assert_eq!(replay("각나", "가나", &sets[0]), "가나");
}

#[test]
fn 내용이_같은_undo는_span_없는_변경으로_남는다() {
    let mut store = store();
    let document = open(&mut store, "same");
    apply(&mut store, document, vec![(0..4, "same")], 0);
    assert!(store.undo(document).unwrap());
    let sets = tracked(&store, document, 1);
    assert_eq!(sets.len(), 1);
    assert!(sets[0].spans.is_empty());
    assert_eq!(sets[0].first_changed_line(), None);
}

#[test]
fn 소비자가_저널_상한보다_뒤처지면_전체_무효화를_알린다() {
    let mut store = store();
    let document = open(&mut store, "");
    for group in 0..=MAX_JOURNAL_ENTRIES {
        apply(
            &mut store,
            document,
            vec![(0..0, "x")],
            group.try_into().unwrap(),
        );
    }
    assert!(matches!(
        store.changes_since(document, 0).unwrap(),
        ChangesSince::Lagged
    ));
    assert_eq!(tracked(&store, document, 1).len(), MAX_JOURNAL_ENTRIES);
    let current = store.documents().snapshot(document).unwrap().revision;
    assert!(tracked(&store, document, current).is_empty());
    assert!(matches!(
        store.changes_since(document, current + 1).unwrap(),
        ChangesSince::Lagged
    ));
}

#[test]
fn 저널에_남지_않는_revision_변경을_지난_소비자는_전체_무효화를_받는다() {
    let mut store = store();
    let document = open(&mut store, "one");
    apply(&mut store, document, vec![(3..3, "!")], 0);
    let saved = store.save_snapshot(document).unwrap();
    store.mark_saved(saved, None).unwrap();
    store
        .refresh_clean_file(document, &PathBuf::from(SYNTHETIC_PATH), file("reloaded"))
        .unwrap();
    let reloaded = store.documents().snapshot(document).unwrap().revision;
    assert_eq!(reloaded, 2);
    assert!(matches!(
        store.changes_since(document, 0).unwrap(),
        ChangesSince::Lagged
    ));
    assert!(matches!(
        store.changes_since(document, 1).unwrap(),
        ChangesSince::Lagged
    ));
    assert!(tracked(&store, document, reloaded).is_empty());
    apply(&mut store, document, vec![(0..0, "#")], 1);
    assert!(matches!(
        store.changes_since(document, 1).unwrap(),
        ChangesSince::Lagged
    ));
    let sets = tracked(&store, document, reloaded);
    assert_eq!(sets.len(), 1);
    assert_eq!((sets[0].revision_before, sets[0].revision_after), (2, 3));
}

#[test]
fn span이_많은_편집은_전체를_덮는_span_하나로_줄인다() {
    let mut store = store();
    let before = "x\n".repeat(WIDE_EDIT_LINES);
    let document = open(&mut store, &before);
    let edits = (0..WIDE_EDIT_LINES)
        .map(|line| (line * 2..line * 2, "y"))
        .collect();
    apply(&mut store, document, edits, 0);
    let after = text(&store, document);
    assert_eq!(after, "yx\n".repeat(WIDE_EDIT_LINES));
    let sets = tracked(&store, document, 0);
    assert_eq!(sets[0].spans.len(), 1);
    assert_eq!(sets[0].spans[0].start_byte, 0);
    assert_eq!(replay(&before, &after, &sets[0]), after);
}

#[test]
fn cr_뒤에_lf를_넣어_줄_수가_그대로인_편집도_줄_수가_맞는_span으로_남는다() {
    let mut store = store();
    let document = open(&mut store, "a\rb\nc");
    apply(&mut store, document, vec![(2..2, "\n")], 0);
    assert_eq!(text(&store, document), "a\r\nb\nc");
    let sets = tracked(&store, document, 0);
    let set = &sets[0];
    assert_eq!((set.line_count_before, set.line_count_after), (3, 3));
    assert_eq!(set.spans.len(), 1);
    let span = &set.spans[0];
    let removed_lines = span.old_end.line - span.start.line;
    let inserted_lines = span.new_end.line - span.start.line;
    assert_eq!(removed_lines, inserted_lines);
    assert_eq!(replay("a\rb\nc", "a\r\nb\nc", set), "a\r\nb\nc");
}

#[test]
fn 없는_문서의_저널은_찾을_수_없다() {
    let mut store = store();
    let document = open(&mut store, "gone");
    let revision = store.documents().snapshot(document).unwrap().revision;
    store.discard_document(document, revision).unwrap();
    assert!(matches!(
        store.changes_since(document, 0),
        Err(EditorError::NotFound)
    ));
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

fn file(content: &str) -> OpenedFile {
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
    }
}

fn open(store: &mut EditorStore, content: &str) -> DocumentId {
    store
        .open_file(PathBuf::from(SYNTHETIC_PATH), file(content))
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

fn point(line: usize, column: usize) -> LinePoint {
    LinePoint { line, column }
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

fn tracked(store: &EditorStore, document: DocumentId, revision: u64) -> Vec<ChangeSet> {
    match store.changes_since(document, revision).unwrap() {
        ChangesSince::Tracked(changes) => changes.cloned().collect(),
        ChangesSince::Lagged => panic!("journal lagged behind revision {revision}"),
    }
}

fn replay(before: &str, after: &str, set: &ChangeSet) -> String {
    let mut rebuilt = String::new();
    let mut copied_until = 0;
    let mut shift = 0isize;
    for span in &set.spans {
        assert!(copied_until <= span.start_byte);
        rebuilt.push_str(&before[copied_until..span.start_byte]);
        let after_start =
            usize::try_from(isize::try_from(span.start_byte).unwrap() + shift).unwrap();
        let inserted = span.new_end_byte - span.start_byte;
        rebuilt.push_str(&after[after_start..after_start + inserted]);
        shift += isize::try_from(inserted).unwrap()
            - isize::try_from(span.old_end_byte - span.start_byte).unwrap();
        copied_until = span.old_end_byte;
    }
    rebuilt.push_str(&before[copied_until..]);
    rebuilt
}

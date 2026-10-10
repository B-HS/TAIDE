use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::formatting::{minimal_edits, selection_ranges};
use taide_native_editor::lsp::{
    LspRange, TextEdit, apply_text_edits, byte_to_position, range_to_bytes,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};

const BYTE_LIMIT: usize = 512 * 1024;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;
const WORD_DEPTH: usize = 2;
const REPLACEMENT_WIDTH: usize = 1_000;

fn fixture(text: &str) -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_untitled(tab.clone(), text, "rust".into())
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab,
            },
            document,
        )
        .unwrap();
    (store, document, view)
}

fn full_edit(document: &DocumentSnapshot, text: &str) -> TextEdit {
    TextEdit::new(
        LspRange::new(
            Default::default(),
            byte_to_position(document, document.rope.len_bytes()).unwrap(),
        ),
        text.to_owned(),
    )
}

#[test]
fn 전체_포맷의_최소_편집은_이름_안의_커서와_선택_mirror_스크롤_undo를_보존한다() {
    let initial = "let value=1;\nlet next=2;\n";
    let expected = "let value = 1;\nlet next = 2;\n";
    let (mut store, document, view) = fixture(initial);
    let mirror = store
        .attach_view(
            ViewKey {
                window: "mirror".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let range = initial.find("next").unwrap()..initial.find("next").unwrap() + "ne".len();
    let scroll = ScrollPosition { x: 0.0, y: 1.0 };
    let selection = SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: range.start,
            head: range.end,
        }],
    };
    for owner in [view, mirror] {
        store
            .set_view_state(owner, selection.clone(), scroll.clone(), Vec::new())
            .unwrap();
    }
    let snapshot = store.documents().snapshot(document).unwrap();
    let edits = minimal_edits(&snapshot, vec![full_edit(&snapshot, expected)]).unwrap();
    assert!(edits.len() > 1);
    assert!(apply_text_edits(&mut store, &snapshot, Some(view), edits).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    let start = expected.find("next").unwrap();
    for owner in [view, mirror] {
        let current = store.views().get(owner).unwrap();
        assert_eq!(
            current.selection,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: start,
                    head: start + "ne".len()
                }]
            }
        );
        assert_eq!(current.scroll, scroll);
    }
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        initial
    );
    assert_eq!(store.views().get(view).unwrap().selection, selection);
}

#[test]
fn 동일한_포맷은_문서_revision_dirty와_undo를_만들지_않고_겹침은_거절한다() {
    let (mut store, document, view) = fixture("same");
    let snapshot = store.documents().snapshot(document).unwrap();
    let edits = minimal_edits(&snapshot, vec![full_edit(&snapshot, "same")]).unwrap();
    assert!(edits.is_empty());
    assert!(!apply_text_edits(&mut store, &snapshot, Some(view), edits).unwrap());
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(
        (after.rope, after.revision, after.dirty),
        (snapshot.rope.clone(), snapshot.revision, snapshot.dirty)
    );
    assert!(!store.undo(document).unwrap());
    assert_eq!(
        minimal_edits(
            &snapshot,
            vec![
                full_edit(&snapshot, "changed"),
                full_edit(&snapshot, "second")
            ]
        ),
        Err(EditorError::Overlap)
    );
}

#[test]
fn 선택_포맷은_빈_선택의_전체_줄과_역방향_겹침_utf16을_정렬하여_합친다() {
    let initial = "한é\r\nlast";
    let (store, document, _) = fixture(initial);
    let snapshot = store.documents().snapshot(document).unwrap();
    let split = initial.find("last").unwrap();
    let selections = SelectionSet {
        primary: 0,
        selections: vec![
            Selection { anchor: 0, head: 0 },
            Selection {
                anchor: split,
                head: "한".len(),
            },
            Selection {
                anchor: split,
                head: initial.len(),
            },
        ],
    };
    assert_eq!(
        selection_ranges(&snapshot, &selections).unwrap(),
        vec![LspRange::new(
            Default::default(),
            byte_to_position(&snapshot, initial.len()).unwrap()
        )]
    );
}

#[test]
fn 최소_편집은_작은_문자열_전체_조합과_보조문자_crlf를_원문_경계로_재구성한다() {
    let alphabet = ["a", "b", "한", "\r\n", "\u{1f600}"];
    let mut words = vec![String::new()];
    let mut level = vec![String::new()];
    for _ in 0..WORD_DEPTH {
        level = level
            .iter()
            .flat_map(|prefix| {
                alphabet
                    .iter()
                    .map(move |suffix| format!("{prefix}{suffix}"))
            })
            .collect();
        words.extend(level.clone());
    }
    for initial in &words {
        let (store, document, _) = fixture(initial);
        let snapshot = store.documents().snapshot(document).unwrap();
        for expected in &words {
            let edits = minimal_edits(&snapshot, vec![full_edit(&snapshot, expected)]).unwrap();
            let mut result = initial.clone();
            for edit in edits.iter().rev() {
                result.replace_range(
                    range_to_bytes(&snapshot, edit.range).unwrap(),
                    &edit.new_text,
                );
            }
            let expected = expected
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', snapshot.metadata.line_ending.as_str());
            assert_eq!(result, expected, "initial={initial:?}; edits={edits:?}");
        }
    }
}

#[test]
fn 다른_큰_교체는_제한된_최소화_뒤에도_유효한_단일_편집으로_완료한다() {
    let initial = "a".repeat(REPLACEMENT_WIDTH);
    let expected = "b".repeat(REPLACEMENT_WIDTH);
    let (mut store, document, view) = fixture(&initial);
    let snapshot = store.documents().snapshot(document).unwrap();
    let edits = minimal_edits(&snapshot, vec![full_edit(&snapshot, &expected)]).unwrap();
    assert_eq!(edits.len(), 1);
    assert!(apply_text_edits(&mut store, &snapshot, Some(view), edits).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
}

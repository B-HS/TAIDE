use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
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
const CRLF_MARKER_UNIT: usize = 77;

#[test]
fn raw_포맷의_utf16_마커는_crlf_중간에_커서를_남기지_않는다() {
    let initial = format!("{}\r\na", "a".repeat(REPLACEMENT_WIDTH));
    let replacement = format!(
        "{}\r\n{}",
        "b".repeat(CRLF_MARKER_UNIT - 1),
        "b".repeat(REPLACEMENT_WIDTH)
    );
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let path = "/synthetic/format-crlf.txt";
    let byte_size = u32::try_from(initial.len()).unwrap();
    let line_count = u32::try_from(initial.lines().count()).unwrap();
    let document = store
        .open_file(
            path.into(),
            OpenedFile {
                path: path.into(),
                content: initial,
                language_id: "plaintext".into(),
                byte_size,
                line_count,
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "crlf".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: CRLF_MARKER_UNIT,
                    head: CRLF_MARKER_UNIT,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    let before = store.documents().snapshot(document).unwrap();
    assert_eq!(before.metadata.line_ending.as_str(), "\r\n");
    assert_eq!(
        minimal_edits(&before, vec![full_edit(&before, &replacement)])
            .unwrap()
            .len(),
        1
    );
    assert!(
        taide_native_editor::formatting::apply_edits(
            &mut store,
            &before,
            Some(view),
            vec![full_edit(&before, &replacement)]
        )
        .unwrap()
    );
    let after = store.documents().snapshot(document).unwrap();
    let current = store.views().get(view).unwrap();
    let head = current.selection.selections[current.selection.primary].head;
    assert_eq!(
        taide_native_editor::lsp::position_to_byte(&after, byte_to_position(&after, head).unwrap())
            .unwrap(),
        head
    );
}

struct ReplacementCase {
    text: String,
    replacement: String,
    selections: Vec<[usize; 2]>,
    expected: Vec<[usize; 2]>,
}

#[test]
fn raw_전체_포맷은_원본_utf16_선택과_mirror와_undo를_보존한다() {
    let offsets = |value: &str| {
        value
            .split(';')
            .map(|selection| {
                let (anchor, head) = selection.split_once(',').unwrap();
                [
                    anchor.parse::<usize>().unwrap(),
                    head.parse::<usize>().unwrap(),
                ]
            })
            .collect::<Vec<_>>()
    };
    let cases = include_str!("fixtures/formatting-markers-reference.txt")
        .lines()
        .map(|line| {
            let mut fields = line.split('\t');
            ReplacementCase {
                text: fields.next().unwrap().to_owned(),
                replacement: fields.next().unwrap().to_owned(),
                selections: offsets(fields.next().unwrap()),
                expected: offsets(fields.next().unwrap()),
            }
        });
    for case in cases {
        let (mut store, document, view) = fixture(&case.text);
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
        let selections = |offsets: &[[usize; 2]]| SelectionSet {
            primary: 0,
            selections: offsets
                .iter()
                .map(|[anchor, head]| Selection {
                    anchor: *anchor,
                    head: *head,
                })
                .collect(),
        };
        let before = selections(&case.selections).normalized();
        for owner in [view, mirror] {
            store
                .set_view_state(owner, before.clone(), Default::default(), Vec::new())
                .unwrap();
        }
        let snapshot = store.documents().snapshot(document).unwrap();
        let edits =
            minimal_edits(&snapshot, vec![full_edit(&snapshot, &case.replacement)]).unwrap();
        assert_eq!(edits.len(), 1);
        assert!(
            taide_native_editor::formatting::apply_edits(
                &mut store,
                &snapshot,
                Some(view),
                vec![full_edit(&snapshot, &case.replacement)]
            )
            .unwrap()
        );
        for owner in [view, mirror] {
            assert_eq!(
                store.views().get(owner).unwrap().selection,
                selections(&case.expected).normalized()
            );
        }
        assert!(store.undo(document).unwrap());
        for owner in [view, mirror] {
            assert_eq!(store.views().get(owner).unwrap().selection, before);
        }
        assert!(store.redo(document).unwrap());
        for owner in [view, mirror] {
            assert_eq!(
                store.views().get(owner).unwrap().selection,
                selections(&case.expected).normalized()
            );
        }
    }
}

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

#[test]
fn 포맷_스크롤은_한번만_소비되고_커서_스크롤_편집_undo_변경에서_만료한다() {
    for change in ["none", "cursor", "scroll", "edit", "undo"] {
        let (mut store, document, view) = fixture("abc");
        store
            .set_view_state(
                view,
                SelectionSet::default(),
                ScrollPosition { x: 0.0, y: 1.0 },
                Vec::new(),
            )
            .unwrap();
        let before = store.documents().snapshot(document).unwrap();
        assert!(
            taide_native_editor::formatting::apply_edits(
                &mut store,
                &before,
                Some(view),
                vec![TextEdit::new(LspRange::default(), "\n".into())]
            )
            .unwrap()
        );
        let after = store.documents().snapshot(document).unwrap();
        let current = store.views().get(view).unwrap().clone();
        match change {
            "cursor" => {
                store
                    .set_view_state(
                        view,
                        SelectionSet {
                            primary: 0,
                            selections: vec![Selection {
                                anchor: after.rope.len_bytes(),
                                head: after.rope.len_bytes(),
                            }],
                        },
                        current.scroll,
                        current.folds,
                    )
                    .unwrap();
            }
            "scroll" => {
                store
                    .set_view_state(view, current.selection, Default::default(), current.folds)
                    .unwrap();
            }
            "edit" => {
                assert!(
                    apply_text_edits(
                        &mut store,
                        &after,
                        Some(view),
                        vec![TextEdit::new(LspRange::default(), ".".into())]
                    )
                    .unwrap()
                );
            }
            "undo" => {
                assert!(store.undo(document).unwrap());
            }
            "none" => (),
            _ => unreachable!(),
        }
        let request = store.take_formatting_scroll(view).unwrap();
        assert_eq!(request.is_some(), change == "none");
        if let Some(request) = request {
            assert_eq!(request.previous_revision, before.revision);
            assert_eq!(request.previous_head, 0);
            assert_eq!(request.revision, after.revision);
            assert_eq!(request.head, "\n".len());
        }
        assert!(store.take_formatting_scroll(view).unwrap().is_none());
    }
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

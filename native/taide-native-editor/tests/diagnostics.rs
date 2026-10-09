use std::sync::Arc;

use taide_model::ids::TabId;
use taide_native_editor::change_journal::MAX_JOURNAL_ENTRIES;
use taide_native_editor::diagnostics::{Marker, MarkerSet, Message, Severity, marker_range};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::lsp::Position;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

const BYTE_LIMIT: usize = 4096;

fn fixture(text: &str) -> (EditorStore, taide_native_editor::document::DocumentId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_untitled(TabId::new(), text, "rust".into())
        .unwrap();
    (store, id)
}

fn message(severity: Severity) -> Arc<Message> {
    Arc::new(Message {
        severity,
        text: "synthetic message".into(),
        source: Some("server".into()),
        code: Some("42".into()),
    })
}

#[test]
fn marker_범위는_utf16_힌트_빈줄_잘못된_범위를_안전하게_정규화한다() {
    let (store, id) = fixture("a\u{1f600}한\n\nend");
    let document = store.documents().snapshot(id).unwrap();
    let range = |start, end| lsp_types::Range::new(start, end);
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(0, 1), Position::new(0, 4)),
            Severity::Error
        ),
        Some(1..8)
    );
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(0, 2), Position::new(0, 3)),
            Severity::Warning
        ),
        Some(1..5)
    );
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(0, 0), Position::new(2, 3)),
            Severity::Hint
        ),
        Some(0..5)
    );
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(0, 0), Position::new(0, 2)),
            Severity::Error
        ),
        Some(0..5)
    );
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(0, 2), Position::new(0, 2)),
            Severity::Error
        ),
        Some(1..1)
    );
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(99, 0), Position::new(100, 0)),
            Severity::Error
        ),
        Some(document.rope.len_bytes()..document.rope.len_bytes())
    );
    assert_eq!(
        marker_range(
            &document,
            range(Position::new(1, 0), Position::new(1, 99)),
            Severity::Error
        ),
        Some(9..9)
    );
    assert!(
        marker_range(
            &document,
            range(Position::new(2, 0), Position::new(0, 0)),
            Severity::Error
        )
        .is_none()
    );
}

#[test]
fn marker_anchor는_편집_경계와_메시지를_보존하고_언어_신원_만료를_거절한다() {
    let (mut store, id) = fixture("abcdef");
    let before = store.documents().snapshot(id).unwrap();
    let payload = message(Severity::Error);
    let markers = MarkerSet::new(
        &before,
        vec![Marker {
            bytes: 2..4,
            message: payload.clone(),
        }],
    )
    .unwrap();
    store
        .apply(
            id,
            Transaction {
                revision: before.revision,
                edits: vec![Edit {
                    bytes: 2..2,
                    text: "X".into(),
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    let current = store.documents().snapshot(id).unwrap();
    let tracked = markers
        .tracked(
            &current,
            store.changes_since(id, markers.revision()).unwrap(),
        )
        .unwrap();
    assert_eq!(tracked.markers()[0].bytes, 3..5);
    assert!(Arc::ptr_eq(&tracked.markers()[0].message, &payload));
    let mut changed_language = current.clone();
    changed_language.metadata.language_id = "python".into();
    assert!(
        markers
            .tracked(
                &changed_language,
                store.changes_since(id, markers.revision()).unwrap()
            )
            .is_none()
    );
    changed_language.metadata.language_id = current.metadata.language_id.clone();
    changed_language.key =
        taide_native_editor::document::DocumentKey::File("/synthetic/renamed.rs".into());
    assert!(
        markers
            .tracked(
                &changed_language,
                store.changes_since(id, markers.revision()).unwrap()
            )
            .is_none()
    );
    for index in 0..=MAX_JOURNAL_ENTRIES {
        let snapshot = store.documents().snapshot(id).unwrap();
        store
            .apply(
                id,
                Transaction {
                    revision: snapshot.revision,
                    edits: vec![Edit {
                        bytes: 0..0,
                        text: "x".into(),
                    }],
                    group: UndoGroup(index as u64),
                    origin: None,
                    selection_after: None,
                },
            )
            .unwrap();
    }
    assert!(
        markers
            .tracked(
                &store.documents().snapshot(id).unwrap(),
                store.changes_since(id, markers.revision()).unwrap()
            )
            .is_none()
    );
    assert!(
        MarkerSet::new(
            &before,
            vec![Marker {
                bytes: 6..7,
                message: payload
            }]
        )
        .is_err()
    );
}

#[test]
fn 실제_monaco의_진단_표시범위_원본표본과_일치한다() {
    let mut lines = include_str!("fixtures/diagnostic-reference.txt").lines();
    assert_eq!(lines.next(), Some("0.56.0"));
    let mut count = 0;
    for line in lines {
        let fields = line.split('\t').collect::<Vec<_>>();
        let bytes = fields[0]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        let text = String::from_utf8(bytes).unwrap();
        let (store, id) = fixture(&text);
        let document = store.documents().snapshot(id).unwrap();
        let number = |index: usize| fields[index].parse::<u32>().unwrap();
        let severity = match number(1) {
            4 => Severity::Warning,
            2 => Severity::Information,
            1 => Severity::Hint,
            _ => Severity::Error,
        };
        let range = lsp_types::Range::new(
            Position::new(number(2), number(3)),
            Position::new(number(4), number(5)),
        );
        let actual = marker_range(&document, range, severity).unwrap();
        let actual = taide_native_editor::diagnostics::display_range(&document, actual, None);
        let expected = number(6) as usize..number(7) as usize;
        assert_eq!(actual, expected, "{text:?} {range:?} {severity:?}");
        count += 1;
    }
    assert_eq!(count, 684);
}

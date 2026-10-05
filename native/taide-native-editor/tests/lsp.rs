use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::EditorError;
use taide_native_editor::lsp::{
    Position, TextEdit, apply_text_edits, byte_to_position, position_to_byte,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewKey};

const PATH: &str = "/synthetic/lsp.txt";
const INITIAL: &str = "a😀한\r\ne\u{301}末\n";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;

fn edit(start: Position, end: Position, text: &str) -> TextEdit {
    TextEdit::new(lsp_types::Range::new(start, end), text.into())
}

#[test]
fn lsp_utf16_편집은_원본_좌표와_공유_선택_undo를_지키고_stale_overlap을_원자적으로_거절한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            PATH.into(),
            OpenedFile {
                path: PATH.into(),
                content: INITIAL.into(),
                language_id: "plaintext".into(),
                byte_size: INITIAL.len().try_into().unwrap(),
                line_count: INITIAL.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let views = ["main", "auxiliary"].map(|window| {
        store
            .attach_view(
                ViewKey {
                    window: window.into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap()
    });
    let before = store.documents().snapshot(document).unwrap();
    for (byte, position) in [
        (0, Position::new(0, 0)),
        ("a".len(), Position::new(0, 1)),
        ("a😀".len(), Position::new(0, 3)),
        ("a😀한".len(), Position::new(0, 4)),
        ("a😀한\r\ne\u{301}".len(), Position::new(1, 2)),
        (INITIAL.len(), Position::new(2, 0)),
    ] {
        assert_eq!(position_to_byte(&before, position).unwrap(), byte);
        assert_eq!(byte_to_position(&before, byte).unwrap(), position);
    }
    assert_eq!(
        position_to_byte(&before, Position::new(0, u32::MAX)).unwrap(),
        "a😀한".len()
    );
    assert_eq!(
        position_to_byte(&before, Position::new(u32::MAX, 0)).unwrap(),
        INITIAL.len()
    );
    assert_eq!(
        byte_to_position(&before, "a😀한\r".len()).unwrap(),
        Position::new(0, 4)
    );
    assert_eq!(
        position_to_byte(&before, Position::new(0, 2)),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(
        byte_to_position(&before, "a".len() + 1),
        Err(EditorError::InvalidBoundary)
    );
    store
        .set_view_state(
            views[1],
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: INITIAL.len(),
                    head: INITIAL.len(),
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    assert!(
        apply_text_edits(
            &mut store,
            &before,
            Some(views[0]),
            vec![
                edit(Position::new(1, 0), Position::new(1, 2), "next"),
                edit(Position::new(0, 1), Position::new(0, 3), "文"),
            ]
        )
        .unwrap()
    );
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.rope.to_string(), "a文한\r\nnext末\n");
    assert!(after.dirty);
    assert_eq!(
        store.views().get(views[1]).unwrap().selection.selections[0].head,
        after.rope.len_bytes()
    );
    assert_eq!(
        apply_text_edits(&mut store, &before, None, Vec::new()),
        Err(EditorError::StaleRevision)
    );
    for edits in [
        vec![
            edit(Position::new(0, 0), Position::new(0, 2), "first"),
            edit(Position::new(0, 1), Position::new(0, 3), "second"),
        ],
        vec![edit(Position::new(1, 1), Position::new(0, 1), "reversed")],
        vec![edit(
            Position::new(0, 0),
            Position::new(0, 0),
            &"x".repeat(BYTE_LIMIT + 1),
        )],
    ] {
        assert!(apply_text_edits(&mut store, &after, None, edits).is_err());
        let current = store.documents().snapshot(document).unwrap();
        assert_eq!(current.revision, after.revision);
        assert_eq!(current.rope, after.rope);
    }
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        before.rope
    );
    assert!(store.redo(document).unwrap());
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        after.rope
    );
    let requested = store.documents().snapshot(document).unwrap();
    assert!(!apply_text_edits(&mut store, &requested, None, Vec::new()).unwrap());
}

use super::*;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::ViewKey;

const TEXT: &str = "a\r\n  \u{1f600}end\r\n";
const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 64 * 1024;
const EMOJI_START: usize = 5;
const EMOJI_END: usize = 9;
const EMOJI_COLUMN: u64 = 5;
const PREVIOUS_X: f32 = 24.0;
const PREVIOUS_Y: f32 = 12.0;

fn fixture() -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/view-state.txt".into(),
            OpenedFile {
                path: "/synthetic/view-state.txt".into(),
                content: TEXT.into(),
                language_id: "plaintext".into(),
                byte_size: TEXT.len().try_into().unwrap(),
                line_count: TEXT.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: true,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, view)
}

#[test]
fn 잘못된_json과_배열_형식은_현재_선택_scroll과_readonly_문서를_보존한다() {
    for saved in [
        "{",
        "null",
        "[]",
        "[[],{}]",
        "{}",
        "{\"cursorState\":[],\"viewState\":[]}",
        "{\"cursorState\":[null],\"viewState\":{}}",
        "{\"cursorState\":[{\"position\":[2,3]}],\"viewState\":{}}",
        "{\"cursorState\":[{}],\"viewState\":{\"firstPosition\":[2,3]}}",
    ] {
        let (mut store, view) = fixture();
        let selection = SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: EMOJI_END,
                head: 1,
            }],
        };
        let scroll = ScrollPosition {
            x: PREVIOUS_X,
            y: PREVIOUS_Y,
        };
        store
            .set_view_state(view, selection.clone(), scroll.clone(), Vec::new())
            .unwrap();
        let document = store.views().get(view).unwrap().document;
        let original = store.documents().snapshot(document).unwrap();
        State::default()
            .restore(&mut store, view, Some(saved))
            .unwrap();
        let current = store.views().get(view).unwrap();
        assert_eq!(current.selection, selection, "{saved}");
        assert_eq!(current.scroll, scroll, "{saved}");
        assert!(store.take_scroll_restore(view).unwrap().is_none());
        let current = store.documents().snapshot(document).unwrap();
        assert_eq!(current.rope.to_string(), TEXT);
        assert_eq!(current.revision, original.revision);
        assert_eq!(current.dirty, original.dirty);
        assert!(current.metadata.read_only);
    }
}

#[test]
fn crlf_utf16_중간_scalar와_범위_초과는_독립_mirror의_유효한_위치로_복원된다() {
    let (mut store, view) = fixture();
    let document = store.views().get(view).unwrap().document;
    let mirror = store
        .attach_view(
            ViewKey {
                window: "other".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let mut state = State::default();
    let saved = serde_json::json!({
        "cursorState": [{
            "selectionStart": {"lineNumber": u64::MAX, "column": u64::MAX},
            "position": {"lineNumber": 2, "column": 4}
        }],
        "viewState": {"scrollLeft": -1, "scrollTopWithoutViewZones": -1}
    })
    .to_string();
    state.restore(&mut store, view, Some(&saved)).unwrap();
    assert_eq!(
        store.views().get(view).unwrap().selection.selections,
        vec![Selection {
            anchor: TEXT.len(),
            head: EMOJI_START,
        }]
    );
    assert_eq!(
        store.views().get(view).unwrap().scroll,
        ScrollPosition::default()
    );
    assert!(
        store
            .take_scroll_restore(view)
            .unwrap()
            .unwrap()
            .byte
            .is_none()
    );
    let saved = serde_json::json!({
        "cursorState": [{"position": {"lineNumber": 2, "column": 5}}],
        "viewState": {"firstPosition": {"lineNumber": 2, "column": 3}, "firstPositionDeltaTop": -1}
    })
    .to_string();
    state.restore(&mut store, mirror, Some(&saved)).unwrap();
    assert_eq!(
        store.views().get(mirror).unwrap().selection.selections,
        vec![Selection {
            anchor: EMOJI_END,
            head: EMOJI_END,
        }]
    );
    let request = store.take_scroll_restore(mirror).unwrap().unwrap();
    assert_eq!(request.byte, Some(EMOJI_START));
    assert_eq!(request.delta_top, -1.0);
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0].head,
        EMOJI_START
    );
    store.detach_view(view).unwrap();
    state.begin_frame(&store);
    assert_eq!(
        state.restore(&mut store, view, Some(&saved)),
        Err(EditorError::NotFound)
    );
    assert!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .metadata
            .read_only
    );
}

#[test]
fn 선택_시작의_누락된_축은_저장된_head의_같은_축을_사용한다() {
    for start in [
        serde_json::json!({}),
        serde_json::json!({"lineNumber": 2}),
        serde_json::json!({"column": EMOJI_COLUMN}),
    ] {
        let (mut store, view) = fixture();
        let saved = serde_json::json!({
            "cursorState": [{
                "selectionStart": start,
                "position": {"lineNumber": 2, "column": EMOJI_COLUMN}
            }],
            "viewState": {}
        })
        .to_string();
        State::default()
            .restore(&mut store, view, Some(&saved))
            .unwrap();
        assert_eq!(
            store.views().get(view).unwrap().selection.selections,
            vec![Selection {
                anchor: EMOJI_END,
                head: EMOJI_END
            }]
        );
    }
}

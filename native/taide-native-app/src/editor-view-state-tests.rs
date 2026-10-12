use super::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::layout::{PaneNode, Tab};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

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

#[test]
fn 캡처는_readonly의_최신_선택_primary와_raw_scroll을_보존하고_교체된_소유를_제외한다() {
    let (mut store, view) = fixture();
    let key = store.views().get(view).unwrap().key.clone();
    let document = store.views().get(view).unwrap().document;
    let original = store.documents().snapshot(document).unwrap();
    let kind = TabKind::File {
        path: "/synthetic/view-state.txt".into(),
    };
    let project = ProjectId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: key.pane.clone(),
        tabs: vec![Tab {
            id: key.tab.clone(),
            kind: kind.clone(),
            title: "synthetic".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(key.tab.clone()),
    };
    layout.focused_pane = key.pane.clone();
    let mut layouts = HashMap::from([(project.clone(), layout)]);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 0, head: 0 },
                    Selection {
                        anchor: EMOJI_END,
                        head: EMOJI_END,
                    },
                ],
            },
            ScrollPosition {
                x: PREVIOUS_X,
                y: PREVIOUS_Y,
            },
            Vec::new(),
        )
        .unwrap();
    let mut state = State::default();
    state.observed.insert(
        view,
        Observed {
            document,
            kind,
            anchor: None,
        },
    );
    let captured = state.capture(&store, &layouts, &key.window).unwrap();
    assert_eq!(captured.len(), 1);
    let saved = serde_json::Value::Object(captured[0].fields.clone());
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({"lineNumber": 2, "column": EMOJI_COLUMN})
    );
    assert_eq!(
        saved["cursorState"][1]["position"],
        serde_json::json!({"lineNumber": 1, "column": 1})
    );
    assert_eq!(
        saved["viewState"]["scrollTopWithoutViewZones"],
        serde_json::json!(PREVIOUS_Y)
    );
    let mut selection = store.views().get(view).unwrap().selection.clone();
    selection.selections[1] = Selection {
        anchor: EMOJI_START,
        head: EMOJI_START,
    };
    store
        .set_view_state(
            view,
            selection,
            ScrollPosition {
                x: PREVIOUS_X,
                y: PREVIOUS_Y,
            },
            Vec::new(),
        )
        .unwrap();
    let latest = state.capture(&store, &layouts, &key.window).unwrap();
    assert_eq!(
        latest[0].fields["cursorState"][0]["position"],
        serde_json::json!({"lineNumber": 2, "column": 3})
    );
    let current = store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), original.rope.to_string());
    assert_eq!(current.revision, original.revision);
    assert_eq!(current.dirty, original.dirty);
    if let PaneNode::Leaf { tabs, .. } = &mut layouts.get_mut(&project).unwrap().root {
        tabs[0].kind = TabKind::Untitled { index: 1 };
    }
    assert!(
        state
            .capture(&store, &layouts, &key.window)
            .unwrap()
            .is_empty()
    );
    store.detach_view(view).unwrap();
    state.begin_frame(&store);
    assert!(state.observed.is_empty());
    assert!(
        state
            .capture(&store, &layouts, &key.window)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn preview_메모리는_분리된_뷰를_복원하고_저장값_우선순위와_소유_수명을_보존한다() {
    for case in [
        "cached",
        "malformed",
        "persisted",
        "pane",
        "window",
        "stale",
        "disposed",
    ] {
        let (mut store, view) = fixture();
        let current = store.views().get(view).unwrap().clone();
        let document = store.documents().snapshot(current.document).unwrap();
        let cached = serde_json::json!({
            "cursorState": [{
                "selectionStart": {"lineNumber": 2, "column": EMOJI_COLUMN},
                "position": {"lineNumber": 2, "column": 3}
            }],
            "viewState": {"scrollLeft": PREVIOUS_X, "scrollTopWithoutViewZones": PREVIOUS_Y}
        })
        .to_string();
        let mut state = State::default();
        state.preview_models.insert(
            ModelViewKey {
                window: current.key.window.clone(),
                pane: current.key.pane.clone(),
                document: current.document,
            },
            CachedModelView {
                revision: document.revision + u64::from(case == "stale"),
                saved: PersistedViewState::parse(&cached).unwrap(),
            },
        );
        store.detach_view(view).unwrap();
        if case == "disposed" {
            store.release_document(current.document).unwrap();
            state.begin_frame(&store);
            assert!(state.preview_models.is_empty());
            continue;
        }
        state.begin_frame(&store);
        let mut key = current.key;
        key.tab = TabId::new();
        if case == "pane" {
            key.pane = PaneId::new();
        }
        if case == "window" {
            key.window = "other".into();
        }
        let reopened = store.attach_view(key, current.document).unwrap();
        let persisted = serde_json::json!({
            "cursorState": [{"position": {"lineNumber": 1, "column": 2}}],
            "viewState": {}
        })
        .to_string();
        let saved = match case {
            "malformed" => Some("{"),
            "persisted" => Some(persisted.as_str()),
            _ => None,
        };
        state.restore(&mut store, reopened, saved).unwrap();
        let expected = match case {
            "cached" | "malformed" => Selection {
                anchor: EMOJI_END,
                head: EMOJI_START,
            },
            "persisted" => Selection { anchor: 1, head: 1 },
            _ => Selection { anchor: 0, head: 0 },
        };
        assert_eq!(
            store.views().get(reopened).unwrap().selection.selections,
            vec![expected],
            "{case}"
        );
        if matches!(case, "cached" | "malformed") {
            assert_eq!(
                store.views().get(reopened).unwrap().scroll,
                ScrollPosition {
                    x: PREVIOUS_X,
                    y: PREVIOUS_Y
                }
            );
            assert!(state.preview_models.is_empty());
        }
        let current = store.documents().snapshot(current.document).unwrap();
        assert_eq!(current.rope.to_string(), TEXT);
        assert_eq!(current.revision, document.revision);
        assert_eq!(current.dirty, document.dirty);
        assert!(current.metadata.read_only);
    }
}

#[derive(Default)]
struct ApplyEvents(AtomicUsize);

impl EventSink for ApplyEvents {
    fn publish(&self, event: taide_model::app_event::AppEvent) {
        if matches!(
            event,
            taide_model::app_event::AppEvent::LayoutChanged { .. }
        ) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}

struct ApplyFixture {
    directory: PathBuf,
    services: Arc<AppServices>,
}

impl Drop for ApplyFixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn 캡처_저장은_미지의_필드와_contribution을_보존하고_중복과_닫힌_소유를_무시한다() {
    let project = ProjectId::new();
    let pane = PaneId::new();
    let tab = TabId::new();
    let directory = std::env::temp_dir().join(format!("taide-native-view-state-{project}"));
    std::fs::create_dir_all(&directory).unwrap();
    let events = Arc::new(ApplyEvents::default());
    let fixture = ApplyFixture {
        services: crate::bootstrap::services(
            AppState::new(AppPaths::new(directory.clone())),
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            events.clone(),
        ),
        directory,
    };
    let state = &fixture.services.state;
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: fixture.directory.to_str().unwrap().to_owned(),
            name: "synthetic".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let kind = TabKind::File {
        path: "/synthetic/view-state.txt".into(),
    };
    let contribution = serde_json::json!({"editor.contrib.folding": {"collapsedRegions": [{"startLineNumber": 1, "endLineNumber": 2}]}});
    let previous = serde_json::json!({"future": {"preserve": true}, "contributionsState": contribution, "cursorState": [], "viewState": {}}).to_string();
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: kind.clone(),
            title: "synthetic".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: Some(previous),
        }],
        active: Some(tab.clone()),
    };
    layout.focused_pane = pane;
    let initial = layout.revision;
    state.layouts.write().insert(project.clone(), layout);
    let captured = || {
        Captured {
        project: project.clone(), tab: tab.clone(), kind: kind.clone(),
        fields: serde_json::json!({"cursorState": [{"position": {"lineNumber": 2, "column": EMOJI_COLUMN}}], "viewState": {"scrollLeft": PREVIOUS_X, "scrollTopWithoutViewZones": PREVIOUS_Y}}).as_object().unwrap().clone(),
    }
    };
    apply(&fixture.services, &[captured()]).await.unwrap();
    let updated = state.layouts.read()[&project].clone();
    assert_eq!(updated.revision, initial + 1);
    assert_eq!(events.0.load(Ordering::SeqCst), 1);
    let saved = crate::tabs::tabs_in(&updated.root)[0]
        .view_state
        .as_deref()
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(saved).unwrap();
    assert_eq!(saved["future"], serde_json::json!({"preserve": true}));
    assert_eq!(saved["contributionsState"], contribution);
    state.dirty_layouts.write().clear();
    apply(&fixture.services, &[captured()]).await.unwrap();
    assert!(state.dirty_layouts.read().is_empty());
    assert_eq!(state.layouts.read()[&project].revision, updated.revision);
    assert_eq!(events.0.load(Ordering::SeqCst), 1);
    state.projects.write().remove(&project);
    apply(
        &fixture.services,
        &[Captured {
            fields: serde_json::json!({"cursorState": []})
                .as_object()
                .unwrap()
                .clone(),
            ..captured()
        }],
    )
    .await
    .unwrap();
    assert_eq!(state.layouts.read()[&project], updated);
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: fixture.directory.to_str().unwrap().to_owned(),
            name: "synthetic".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    if let PaneNode::Leaf { tabs, .. } = &mut state.layouts.write().get_mut(&project).unwrap().root
    {
        tabs[0].kind = TabKind::Untitled { index: 1 };
    }
    let replaced = state.layouts.read()[&project].clone();
    apply(&fixture.services, &[captured()]).await.unwrap();
    assert_eq!(state.layouts.read()[&project], replaced);
    let mut closed = replaced;
    taide_layout::service::close_tab(&mut closed, &tab).unwrap();
    state
        .layouts
        .write()
        .insert(project.clone(), closed.clone());
    apply(&fixture.services, &[captured()]).await.unwrap();
    assert_eq!(state.layouts.read()[&project], closed);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

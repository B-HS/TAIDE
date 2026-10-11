use super::*;
use taide_model::file::{EditorConfigOptions, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::{Selection, SelectionSet};

const TEXT: &str = "first\n  \u{1f600}value\nend";
const FILE_PATH: &str = "/synthetic/inlay-hints.rs";
const CAPACITY: usize = 4;
const BYTE_LIMIT: usize = 4096;
const STABLE_FRAMES: usize = 16;
const SCROLL_LINES: usize = 100;

fn fixture(
    tier: FileSizeTier,
    read_only: bool,
) -> (EditorStore, ViewId, ProjectId, ProviderIdentity) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: CAPACITY,
        max_views: CAPACITY,
        max_undo_groups: CAPACITY,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            FILE_PATH.into(),
            OpenedFile {
                path: FILE_PATH.into(),
                content: TEXT.into(),
                language_id: "rust".into(),
                byte_size: TEXT.len().try_into().unwrap(),
                line_count: TEXT.lines().count() as u32,
                tier,
                read_only,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "inlay-fixture".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (
        store,
        view,
        ProjectId::new(),
        ProviderIdentity {
            owner: crate::diagnostics::Owner::new(),
            generation: 1,
            capability_revision: 0,
        },
    )
}

fn context(project: &ProjectId, view: ViewId) -> Context {
    Context {
        project: project.clone(),
        owner: view,
        source: view,
        viewport: egui::ViewportId::ROOT,
        lines: 0..1,
    }
}

fn response(provider: ProviderIdentity) -> Response {
    let hints = serde_json::from_value(serde_json::json!([
        {"position":{"line":1,"character":4},"label":[{"value":"name: ","command":{"title":"ignored","command":"ignored"}},{"value":"T"}],"kind":2,"tooltip":{"kind":"markdown","value":"**parameter**"},"paddingLeft":true,"paddingRight":true,"textEdits":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"ignored"}]},
        {"position":{"line":0,"character":0},"label":": Type","kind":1},
        {"position":{"line":1,"character":3},"label":"invalid surrogate"},
        {"position":{"line":1,"character":100},"label":"invalid column"},
        {"position":{"line":100,"character":0},"label":"invalid line"},
        {"position":{"line":0,"character":0},"label":""},
        {"position":{"line":0,"character":0},"label":"invalid\nlabel"}
    ])).unwrap();
    Response {
        groups: vec![(provider, hints)],
    }
}

#[test]
fn label_parts와_utf16_종류_여백_툴팁을_받고_선택과_빈_응답은_반복_요청하지_않는다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let mut state = State::default();
    let providers = HashSet::from([provider]);
    let now = Instant::now();
    let request = state
        .observe(&store, context(&project, view), providers.clone(), now)
        .unwrap()
        .unwrap();
    assert!(state.accept(&store, &request, providers.clone(), Ok(response(provider))));
    let items = &state.entries[&(egui::ViewportId::ROOT, view)].items;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].kind, Some(Kind::Type));
    assert_eq!(items[1].byte, TEXT.find("value").unwrap());
    assert_eq!(items[1].label, "name: T");
    assert_eq!(items[1].kind, Some(Kind::Parameter));
    assert_eq!(items[1].tooltip.as_deref(), Some("**parameter**"));
    assert!(items[1].padding_left && items[1].padding_right);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: TEXT.len(),
                    head: TEXT.len(),
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    for _ in 0..STABLE_FRAMES {
        assert!(
            state
                .observe(
                    &store,
                    context(&project, view),
                    providers.clone(),
                    now + REQUEST_DEBOUNCE
                )
                .unwrap()
                .is_none()
        );
    }
    assert!(state.accept(
        &store,
        &request,
        providers.clone(),
        Ok(Response {
            groups: vec![(provider, Vec::new())]
        })
    ));
    for _ in 0..STABLE_FRAMES {
        assert!(
            state
                .observe(
                    &store,
                    context(&project, view),
                    providers.clone(),
                    now + REQUEST_DEBOUNCE
                )
                .unwrap()
                .is_none()
        );
    }
    assert!(state.accept(
        &store,
        &request,
        providers.clone(),
        Err(Failure::TransportClosed)
    ));
    assert!(
        state.entries[&(egui::ViewportId::ROOT, view)]
            .items
            .is_empty()
    );
    assert_eq!(
        store
            .documents()
            .snapshot(request.snapshot.id)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
}

#[test]
fn 편집과_공급자_세대_변경은_이전_응답을_거절하고_종료는_완료_요청도_취소한다() {
    let (mut store, view, project, mut provider) = fixture(FileSizeTier::Normal, false);
    let now = Instant::now();
    let mut state = State::default();
    let initial = state
        .observe(
            &store,
            context(&project, view),
            HashSet::from([provider]),
            now,
        )
        .unwrap()
        .unwrap();
    store
        .apply(
            initial.snapshot.id,
            Transaction {
                revision: initial.snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "x".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                now
            )
            .unwrap()
            .is_none()
    );
    assert!(initial.is_cancelled());
    assert!(!state.accept(
        &store,
        &initial,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    let edited = state
        .observe(
            &store,
            context(&project, view),
            HashSet::from([provider]),
            now + REQUEST_DEBOUNCE,
        )
        .unwrap()
        .unwrap();
    provider.generation += 1;
    state
        .observe(
            &store,
            context(&project, view),
            HashSet::from([provider]),
            now + REQUEST_DEBOUNCE,
        )
        .unwrap();
    assert!(edited.is_cancelled());
    let restarted = state
        .observe(
            &store,
            context(&project, view),
            HashSet::from([provider]),
            now + REQUEST_DEBOUNCE + REQUEST_DEBOUNCE,
        )
        .unwrap()
        .unwrap();
    assert!(state.accept(
        &store,
        &restarted,
        HashSet::from([provider]),
        Ok(Response { groups: Vec::new() })
    ));
    state.clear();
    assert!(restarted.is_cancelled());
    assert!(state.entries.is_empty() && state.seen.is_empty());
}

#[test]
fn mirror와_뷰포트는_독립되고_readonly는_표시하지만_큰_tier와_닫힌_owner는_회수한다() {
    for tier in [
        FileSizeTier::Normal,
        FileSizeTier::Large,
        FileSizeTier::ReadOnly,
    ] {
        let (mut store, view, project, provider) = fixture(tier, true);
        let mut state = State::default();
        let now = Instant::now();
        let request = state
            .observe(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                now,
            )
            .unwrap();
        if tier != FileSizeTier::Normal {
            assert!(request.is_none());
            continue;
        }
        let request = request.unwrap();
        let mirror = store
            .attach_view(
                ViewKey {
                    window: "inlay-peek".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                request.snapshot.id,
            )
            .unwrap();
        let mut peek = context(&project, mirror);
        peek.owner = view;
        peek.viewport = egui::ViewportId::from_hash_of("second");
        let preview = state
            .observe(&store, peek, HashSet::from([provider]), now)
            .unwrap()
            .unwrap();
        assert_eq!(state.entries.len(), 2);
        assert!(state.accept(
            &store,
            &preview,
            HashSet::from([provider]),
            Ok(response(provider))
        ));
        state.reject(&request);
        assert!(!preview.is_cancelled());
        store.detach_view(view).unwrap();
        state.reconcile(&store, |_| true, |_, _| HashSet::from([provider]));
        assert!(preview.is_cancelled());
        assert!(state.entries.is_empty());
        assert_eq!(
            store
                .documents()
                .snapshot(preview.snapshot.id)
                .unwrap()
                .rope
                .to_string(),
            TEXT
        );
    }
}

#[test]
fn 스크롤_범위는_위아래_30줄을_포함하고_변경_뒤_25ms에_한번만_요청한다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let document = store.views().get(view).unwrap().document;
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..TEXT.len(),
                    text: "a\n".repeat(SCROLL_LINES),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let now = Instant::now();
    let mut state = State::default();
    let mut current = context(&project, view);
    current.lines = RANGE_MARGIN..RANGE_MARGIN + 1;
    let first = state
        .observe(&store, current, HashSet::from([provider]), now)
        .unwrap()
        .unwrap();
    assert_eq!(first.range.start.line, 0);
    assert_eq!(first.range.end.line as usize, RANGE_MARGIN + RANGE_MARGIN);
    assert!(state.accept(
        &store,
        &first,
        HashSet::from([provider]),
        Ok(Response {
            groups: vec![(provider, serde_json::from_value(serde_json::json!([
        {"position":{"line":0,"character":0},"label":"outside"},
        {"position":{"line":1,"character":0},"label":"inside"}
    ])).unwrap())]
        })
    ));
    let shifted = || Context {
        lines: (RANGE_MARGIN + 1)..(RANGE_MARGIN + 2),
        ..context(&project, view)
    };
    assert!(
        state
            .observe(&store, shifted(), HashSet::from([provider]), now)
            .unwrap()
            .is_none()
    );
    assert!(first.is_cancelled());
    assert_eq!(
        state.entries[&(egui::ViewportId::ROOT, view)].items.len(),
        2
    );
    let second = state
        .observe(
            &store,
            shifted(),
            HashSet::from([provider]),
            now + REQUEST_DEBOUNCE,
        )
        .unwrap()
        .unwrap();
    assert_eq!(second.range.start.line, 1);
    assert!(state.accept(
        &store,
        &second,
        HashSet::from([provider]),
        Ok(Response {
            groups: vec![(provider, Vec::new())]
        })
    ));
    assert_eq!(
        state.entries[&(egui::ViewportId::ROOT, view)].items.len(),
        1
    );
    assert_eq!(
        state.entries[&(egui::ViewportId::ROOT, view)].items[0].label,
        "outside"
    );
    for _ in 0..STABLE_FRAMES {
        assert!(
            state
                .observe(
                    &store,
                    shifted(),
                    HashSet::from([provider]),
                    now + REQUEST_DEBOUNCE
                )
                .unwrap()
                .is_none()
        );
    }
}

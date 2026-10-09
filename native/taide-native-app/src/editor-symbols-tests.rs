use super::*;
use taide_model::file::{FileSizeTier, OpenedFile};
use taide_native_editor::store::{EditorLimits, EditorStore};

const BYTE_LIMIT: usize = 4096;
const PATH: &str = "/synthetic/symbols.rs";

fn fixture() -> (EditorStore, DocumentId, ProjectId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 2,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let content = "class\n  \u{1f600}method\nend";
    let document = store
        .open_file(
            PATH.into(),
            OpenedFile {
                path: PATH.into(),
                content: content.into(),
                byte_size: content.len().try_into().unwrap(),
                line_count: 3,
                language_id: "rust".into(),
                tier: FileSizeTier::Normal,
                read_only: true,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: Default::default(),
            },
        )
        .unwrap();
    (store, document, ProjectId::new())
}

fn model(snapshot: &DocumentSnapshot) -> Arc<DocumentSymbols> {
    let response = serde_json::from_value(serde_json::json!([{"name":"Class", "kind":5, "range":{"start":{"line":0,"character":0},"end":{"line":2,"character":3}}, "selectionRange":{"start":{"line":1,"character":4},"end":{"line":1,"character":10}}}])).unwrap();
    Arc::new(
        DocumentSymbols::new(
            snapshot,
            &taide_lsp::service::workspace_folder_uri(PATH)
                .parse()
                .unwrap(),
            Some(response),
        )
        .unwrap(),
    )
}

fn provider() -> ProviderIdentity {
    ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 0,
        capability_revision: 1,
    }
}

fn response(provider: Option<ProviderIdentity>, model: Arc<DocumentSymbols>) -> Response {
    Response {
        palette_provider: provider,
        palette: model.clone(),
        groups: provider
            .map(|provider| vec![Group { provider, model }])
            .unwrap_or_default(),
        error: None,
    }
}

#[test]
fn 실제_문서_심볼의_세대와_readonly_utf16_reveal은_프로젝트와_provider_수명을_따른다() {
    let (store, id, project) = fixture();
    let snapshot = store.documents().snapshot(id).unwrap();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let now = Instant::now();
    let mut state = State::default();
    let request = state
        .observe(&project, snapshot.clone(), providers.clone(), now)
        .unwrap();
    assert!(state.palette(Some(&project), Some(&snapshot)).is_pending);
    assert!(
        state
            .accept(
                &request,
                &snapshot,
                providers.clone(),
                Ok(response(Some(provider), model(&snapshot)))
            )
            .unwrap()
    );
    assert!(!state.palette(Some(&project), Some(&snapshot)).is_pending);
    assert!(state.sticky(&project, &snapshot).is_some());
    let position = state
        .position(&project, &snapshot, request.generation, 0)
        .unwrap();
    assert_eq!((position.line, position.column), (2.0, 5.0));
    assert!(
        state
            .position(&ProjectId::new(), &snapshot, request.generation, 0)
            .is_none()
    );
    assert!(
        state
            .position(&project, &snapshot, request.generation + 1, 0)
            .is_none()
    );
    let restarted = ProviderIdentity {
        generation: provider.generation + 1,
        ..provider
    };
    let next = state
        .observe(&project, snapshot.clone(), HashSet::from([restarted]), now)
        .unwrap();
    assert!(state.sticky(&project, &snapshot).is_none());
    assert!(
        state
            .position(&project, &snapshot, request.generation, 0)
            .is_none()
    );
    assert!(
        !state
            .accept(
                &next,
                &snapshot,
                HashSet::from([restarted]),
                Ok(response(Some(provider), model(&snapshot)))
            )
            .unwrap()
    );
    assert!(
        state
            .observe(
                &project,
                snapshot.clone(),
                HashSet::from([restarted]),
                Instant::now()
            )
            .is_some()
    );
    state.retain(&HashSet::new());
    assert!(state.model(&project, &snapshot).is_none());
}

#[test]
fn 편집은_요청을_취소하고_400ms_마지막_편집_뒤에만_갱신하며_늦은_회신을_버린다() {
    let (store, id, project) = fixture();
    let before = store.documents().snapshot(id).unwrap();
    let mut after = before.clone();
    after.revision += 1;
    after.rope.insert(0, "new\n");
    let now = Instant::now();
    let mut state = State::default();
    let first = state
        .observe(&project, before.clone(), HashSet::new(), now)
        .unwrap();
    assert!(
        state
            .observe(&project, after.clone(), HashSet::new(), now)
            .is_none()
    );
    assert!(first.is_cancelled());
    assert!(
        !state
            .accept(
                &first,
                &after,
                HashSet::new(),
                Ok(response(None, model(&before)))
            )
            .unwrap()
    );
    assert!(
        state
            .observe(
                &project,
                after.clone(),
                HashSet::new(),
                now + REFRESH_DEBOUNCE - Duration::from_millis(1)
            )
            .is_none()
    );
    let second = state
        .observe(
            &project,
            after.clone(),
            HashSet::new(),
            now + REFRESH_DEBOUNCE,
        )
        .unwrap();
    assert!(second.generation > first.generation);
    state.retain(&HashSet::new());
    assert!(second.is_cancelled());
    assert!(
        !state
            .accept(
                &second,
                &after,
                HashSet::new(),
                Err(Failure::TransportClosed)
            )
            .unwrap()
    );
}

#[test]
fn 미지원과_오류는_빈상태를_끝내고_언어와_capability_교체만_새로_요청한다() {
    let (store, id, project) = fixture();
    let snapshot = store.documents().snapshot(id).unwrap();
    let now = Instant::now();
    let mut state = State::default();
    let request = state
        .observe(&project, snapshot.clone(), HashSet::new(), now)
        .unwrap();
    let uri = taide_lsp::service::workspace_folder_uri(PATH)
        .parse()
        .unwrap();
    let empty = Arc::new(DocumentSymbols::new(&snapshot, &uri, None).unwrap());
    assert!(
        state
            .accept(
                &request,
                &snapshot,
                HashSet::new(),
                Ok(response(None, empty))
            )
            .unwrap()
    );
    assert!(!state.palette(Some(&project), Some(&snapshot)).is_pending);
    assert!(state.sticky(&project, &snapshot).is_none());
    assert!(
        state
            .observe(&project, snapshot.clone(), HashSet::new(), now)
            .is_none()
    );
    let providers = HashSet::from([provider()]);
    let second = state
        .observe(&project, snapshot.clone(), providers.clone(), now)
        .unwrap();
    assert_eq!(
        state.accept(
            &second,
            &snapshot,
            providers.clone(),
            Err(Failure::MalformedResponse)
        ),
        Err(Failure::MalformedResponse)
    );
    assert!(
        state
            .observe(&project, snapshot.clone(), providers.clone(), now)
            .is_none()
    );
    let mut changed = snapshot.clone();
    changed.metadata.language_id = "python".into();
    let third = state
        .observe(&project, changed.clone(), providers, now)
        .unwrap();
    assert!(
        !state
            .accept(
                &second,
                &changed,
                HashSet::new(),
                Ok(response(None, model(&snapshot)))
            )
            .unwrap()
    );
    state.failed(&third);
    assert!(!state.palette(Some(&project), Some(&changed)).is_pending);
}

#[test]
fn 고정줄은_최대_범위_provider를_택하고_갱신_뒤에도_선호_provider를_유지한다() {
    let (store, id, project) = fixture();
    let snapshot = store.documents().snapshot(id).unwrap();
    let first = provider();
    let second = provider();
    let providers = HashSet::from([first, second]);
    let payload = serde_json::from_value(serde_json::json!([{"name":"Broad", "kind":5, "range":{"start":{"line":0,"character":0},"end":{"line":2,"character":3}}, "selectionRange":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}}}])).unwrap();
    let broad = Arc::new(
        DocumentSymbols::new(
            &snapshot,
            &taide_lsp::service::workspace_folder_uri(PATH)
                .parse()
                .unwrap(),
            Some(payload),
        )
        .unwrap(),
    );
    let now = Instant::now();
    let mut state = State::default();
    let request = state
        .observe(&project, snapshot.clone(), providers.clone(), now)
        .unwrap();
    state
        .accept(
            &request,
            &snapshot,
            providers.clone(),
            Ok(Response {
                palette_provider: Some(first),
                palette: model(&snapshot),
                groups: vec![
                    Group {
                        provider: first,
                        model: model(&snapshot),
                    },
                    Group {
                        provider: second,
                        model: broad.clone(),
                    },
                ],
                error: None,
            }),
        )
        .unwrap();
    assert_eq!(
        state.entries[&(project.clone(), id)].preferred,
        Some(second.owner)
    );
    assert_eq!(
        state.model(&project, &snapshot).unwrap().symbols()[0].name,
        "Class"
    );
    assert_eq!(
        state
            .sticky(&project, &snapshot)
            .unwrap()
            .candidates(1..2, &[])[0]
            .scope
            .start_line,
        0
    );
    let mut edited = snapshot.clone();
    edited.revision += 1;
    assert!(
        state
            .observe(&project, edited.clone(), providers.clone(), now)
            .is_none()
    );
    let next = state
        .observe(
            &project,
            edited.clone(),
            providers.clone(),
            now + REFRESH_DEBOUNCE,
        )
        .unwrap();
    let narrow = model(&edited);
    state
        .accept(
            &next,
            &edited,
            providers.clone(),
            Ok(Response {
                palette_provider: Some(first),
                palette: narrow.clone(),
                groups: vec![
                    Group {
                        provider: first,
                        model: narrow.clone(),
                    },
                    Group {
                        provider: second,
                        model: narrow,
                    },
                ],
                error: None,
            }),
        )
        .unwrap();
    assert_eq!(
        state.entries[&(project.clone(), id)].preferred,
        Some(second.owner)
    );
    let changed = HashSet::from([first]);
    let next = state
        .observe(
            &project,
            edited.clone(),
            changed.clone(),
            now + REFRESH_DEBOUNCE,
        )
        .unwrap();
    state
        .accept(
            &next,
            &edited,
            changed,
            Ok(response(Some(first), model(&edited))),
        )
        .unwrap();
    assert_eq!(
        state.entries[&(project.clone(), id)].preferred,
        Some(first.owner)
    );
}

#[test]
fn 두_프로젝트의_같은_문서_요청은_서로를_취소하거나_섞지_않는다() {
    let (store, id, first_project) = fixture();
    let second_project = ProjectId::new();
    let snapshot = store.documents().snapshot(id).unwrap();
    let mut state = State::default();
    let now = Instant::now();
    let first = state
        .observe(&first_project, snapshot.clone(), HashSet::new(), now)
        .unwrap();
    let second = state
        .observe(&second_project, snapshot.clone(), HashSet::new(), now)
        .unwrap();
    assert!(!first.is_cancelled());
    assert!(!second.is_cancelled());
    assert!(
        state
            .accept(
                &first,
                &snapshot,
                HashSet::new(),
                Ok(response(None, model(&snapshot)))
            )
            .unwrap()
    );
    assert!(
        state
            .accept(
                &second,
                &snapshot,
                HashSet::new(),
                Ok(response(None, model(&snapshot)))
            )
            .unwrap()
    );
    assert!(
        state
            .position(&first_project, &snapshot, first.generation, 0)
            .is_some()
    );
    assert!(
        state
            .position(&second_project, &snapshot, second.generation, 0)
            .is_some()
    );
    assert!(
        state
            .position(&first_project, &snapshot, second.generation, 0)
            .is_none()
    );
}

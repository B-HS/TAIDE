use super::*;
use taide_model::file::{EditorConfigOptions, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::{Selection, SelectionSet};

const TEXT: &str = "first\n  \u{1f600}value\nend";
const FILE_PATH: &str = "/synthetic/code-lenses.rs";
const CAPACITY: usize = 4;
const BYTE_LIMIT: usize = 4096;
const STABLE_FRAMES: usize = 16;

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
                window: "lens-fixture".into(),
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
    }
}

fn request(
    state: &mut State,
    store: &EditorStore,
    project: &ProjectId,
    view: ViewId,
    providers: HashSet<ProviderIdentity>,
    now: Instant,
) -> Request {
    assert!(
        state
            .observe(store, context(project, view), providers.clone(), true, now)
            .unwrap()
            .is_none()
    );
    state
        .observe(
            store,
            context(project, view),
            providers,
            true,
            now + REQUEST_DELAY,
        )
        .unwrap()
        .unwrap()
}

fn lens(line: u32, character: u32, title: Option<&str>) -> lsp_types::CodeLens {
    lsp_types::CodeLens {
        range: lsp_types::Range::new(
            lsp_types::Position::new(line, character),
            lsp_types::Position::new(line, character),
        ),
        command: title.map(|title| lsp_types::Command {
            title: title.into(),
            command: "server.command".into(),
            arguments: Some(vec![serde_json::json!({"preserved":true})]),
        }),
        data: Some(serde_json::json!({"raw":{"row":line,"payload":"preserved"}})),
    }
}

#[test]
fn 공급자_순서와_utf16_범위를_보존하고_보이는_렌즈만_한번_resolve한다() {
    let (store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let other = ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        ..provider
    };
    let providers = HashSet::from([provider, other]);
    let now = Instant::now();
    let mut state = State::default();
    let request = request(&mut state, &store, &project, view, providers.clone(), now);
    let invalid = lens(1, 3, Some("invalid surrogate"));
    assert!(state.accept(
        &store,
        &request,
        providers,
        Ok(Response {
            groups: vec![
                (
                    provider,
                    true,
                    vec![
                        lens(1, 4, None),
                        invalid,
                        lens(100, 0, None),
                        lens(1, 100, None)
                    ]
                ),
                (
                    other,
                    false,
                    vec![lens(1, 0, Some("other provider")), lens(2, 0, None)]
                ),
            ]
        }),
        now
    ));
    let entry = &state.entries[&(egui::ViewportId::ROOT, view)];
    assert_eq!(entry.lenses.len(), 3);
    assert_eq!(entry.lenses[0].provider, provider);
    assert_eq!(entry.lenses[0].anchor, TEXT.find("value").unwrap());
    assert_eq!(entry.lenses[1].provider, other);
    assert!(
        state
            .resolve_visible(&store, egui::ViewportId::ROOT, view, &[0], now)
            .is_empty()
    );
    let resolves = state.resolve_visible(
        &store,
        egui::ViewportId::ROOT,
        view,
        &[0],
        now + REQUEST_DELAY,
    );
    assert_eq!(resolves.len(), 1);
    let original = resolves[0].lens.clone();
    let mut resolved = lens(0, 0, Some("resolved"));
    resolved.data = None;
    assert!(state.resolved(&store, &resolves[0], Ok(resolved)));
    let entry = &state.entries[&(egui::ViewportId::ROOT, view)];
    assert_eq!(entry.lenses[0].raw.range, original.range);
    assert_eq!(entry.lenses[0].raw.data, original.data);
    assert_eq!(entry.items[0].title.as_deref(), Some("resolved"));
    for _ in 0..STABLE_FRAMES {
        assert!(
            state
                .resolve_visible(
                    &store,
                    egui::ViewportId::ROOT,
                    view,
                    &[0, 1, 2],
                    now + REQUEST_DELAY
                )
                .is_empty()
        );
        assert!(
            state
                .observe(
                    &store,
                    context(&project, view),
                    HashSet::from([provider, other]),
                    true,
                    now + REQUEST_DELAY
                )
                .unwrap()
                .is_none()
        );
    }
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
fn 빈_오류_응답과_resolve_오류는_안정_프레임에서_재요청하지_않는다() {
    let (store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let providers = HashSet::from([provider]);
    let now = Instant::now();
    for response in [
        Ok(Response { groups: Vec::new() }),
        Err(Failure::MalformedRequest),
    ] {
        let mut state = State::default();
        let request = request(&mut state, &store, &project, view, providers.clone(), now);
        assert!(state.accept(&store, &request, providers.clone(), response, now));
        for _ in 0..STABLE_FRAMES {
            assert!(
                state
                    .observe(
                        &store,
                        context(&project, view),
                        providers.clone(),
                        true,
                        now + REQUEST_DELAY
                    )
                    .unwrap()
                    .is_none()
            );
            assert!(
                state
                    .resolve_visible(
                        &store,
                        egui::ViewportId::ROOT,
                        view,
                        &[0],
                        now + REQUEST_DELAY
                    )
                    .is_empty()
            );
        }
        assert_eq!(
            state.request(egui::ViewportId::ROOT, view).unwrap().token,
            request.token
        );
    }
    let mut state = State::default();
    let request = request(&mut state, &store, &project, view, providers.clone(), now);
    assert!(state.accept(
        &store,
        &request,
        providers,
        Ok(Response {
            groups: vec![(provider, true, vec![lens(0, 0, None)])]
        }),
        now
    ));
    let resolved = state
        .resolve_visible(
            &store,
            egui::ViewportId::ROOT,
            view,
            &[0],
            now + REQUEST_DELAY,
        )
        .pop()
        .unwrap();
    assert!(state.resolved(&store, &resolved, Err(Failure::TransportClosed)));
    assert!(
        state
            .resolve_visible(
                &store,
                egui::ViewportId::ROOT,
                view,
                &[0],
                now + REQUEST_DELAY
            )
            .is_empty()
    );
    assert!(
        state.entries[&(egui::ViewportId::ROOT, view)].items[0]
            .title
            .is_none()
    );
}

#[test]
fn 편집_설정_크기등급_공급자_변경과_종료는_이전_응답과_실행을_취소한다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let providers = HashSet::from([provider]);
    let now = Instant::now();
    let mut state = State::default();
    let original = request(&mut state, &store, &project, view, providers.clone(), now);
    store
        .apply(
            original.snapshot.id,
            Transaction {
                revision: original.snapshot.revision,
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
    assert!(!state.accept(
        &store,
        &original,
        providers.clone(),
        Ok(Response { groups: Vec::new() }),
        now
    ));
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                providers.clone(),
                true,
                now
            )
            .unwrap()
            .is_none()
    );
    assert!(original.is_cancelled());
    let edited = state
        .observe(
            &store,
            context(&project, view),
            providers.clone(),
            true,
            now + REQUEST_DELAY,
        )
        .unwrap()
        .unwrap();
    let other_project = ProjectId::new();
    state.refresh(&other_project, provider, now);
    assert!(!edited.is_cancelled());
    state.refresh(&project, provider, now);
    assert!(edited.is_cancelled());
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                providers.clone(),
                true,
                now + REQUEST_DELAY
            )
            .unwrap()
            .is_none()
    );
    let refreshed = state
        .observe(
            &store,
            context(&project, view),
            providers.clone(),
            true,
            now + REFRESH_DELAY + REQUEST_DELAY,
        )
        .unwrap()
        .unwrap();
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                providers.clone(),
                false,
                now
            )
            .unwrap()
            .is_none()
    );
    assert!(refreshed.is_cancelled());
    let current = request(&mut state, &store, &project, view, providers.clone(), now);
    state.reconcile(
        &store,
        |_| true,
        |_, _| {
            HashSet::from([ProviderIdentity {
                generation: provider.generation + 1,
                ..provider
            }])
        },
    );
    assert!(current.is_cancelled());
    let current = request(&mut state, &store, &project, view, providers, now);
    state.clear();
    assert!(current.is_cancelled());
    for tier in [FileSizeTier::Large, FileSizeTier::ReadOnly] {
        let (store, view, project, provider) = fixture(tier, true);
        assert!(
            state
                .observe(
                    &store,
                    context(&project, view),
                    HashSet::from([provider]),
                    true,
                    now + REQUEST_DELAY
                )
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn 현재줄_선택은_resolve를_기다리고_빈목록과_오래된_선택을_실행하지_않는다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, true);
    let now = Instant::now();
    let mut state = State::default();
    let request = request(
        &mut state,
        &store,
        &project,
        view,
        HashSet::from([provider]),
        now,
    );
    assert!(state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Ok(Response {
            groups: vec![(provider, true, vec![lens(1, 4, None)])]
        }),
        now
    ));
    let byte = TEXT.find("value").unwrap();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    assert!(state.begin_picker(&store, egui::ViewportId::ROOT, view));
    assert!(state.picker(&store).is_none());
    let resolve = state
        .resolve_visible(
            &store,
            egui::ViewportId::ROOT,
            view,
            &[],
            now + REQUEST_DELAY,
        )
        .pop()
        .unwrap();
    assert!(state.resolved(&store, &resolve, Ok(lens(0, 0, Some("Run")))));
    let picker = state.picker(&store).unwrap();
    assert_eq!(picker.titles, ["Run"]);
    let chosen = state.choose(&store, picker.generation, 0).unwrap();
    assert_eq!(chosen.provider, provider);
    assert_eq!(
        chosen.command.arguments,
        Some(vec![serde_json::json!({"preserved":true})])
    );
    assert!(
        state
            .execute(&store, egui::ViewportId::ROOT, view, 0)
            .is_none()
    );
    assert!(state.executed(&store, &chosen));
    assert!(state.begin_picker(&store, egui::ViewportId::ROOT, view));
    let stale = state.picker(&store).unwrap();
    state.clear();
    assert!(state.choose(&store, stale.generation, 0).is_none());
    assert!(!state.executed(&store, &chosen));
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

use super::*;
use taide_model::file::{EditorConfigOptions, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::UntokenizedLines;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::Selection;

const TEXT: &str = "alpha(x, y)\n second";
const FILE_PATH: &str = "/synthetic/selection-range.rs";
const CAPACITY: usize = 4;
const BYTE_LIMIT: usize = 4096;
const STABLE_FRAMES: usize = 16;
const RULES: LineCommandContext<'static> = LineCommandContext {
    indent: IndentOptions {
        tab_size: 4,
        insert_spaces: true,
    },
    language: None,
    syntax: &UntokenizedLines,
    compare: None,
    transforms: None,
    word_rules: None,
};

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
                line_count: TEXT.lines().count().try_into().unwrap(),
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
                window: "selection-fixture".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    set_selection(&mut store, view, &[(2, 2)]);
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

fn set_selection(store: &mut EditorStore, view: ViewId, ranges: &[(usize, usize)]) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: ranges
                    .iter()
                    .map(|(anchor, head)| Selection {
                        anchor: *anchor,
                        head: *head,
                    })
                    .collect(),
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn selection(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections
        .iter()
        .map(|selection| (selection.anchor, selection.head))
        .collect()
}

fn context(project: &ProjectId, owner: ViewId, source: ViewId) -> Context {
    Context {
        project: Some(project.clone()),
        owner,
        source,
        viewport: egui::ViewportId::ROOT,
    }
}

fn submit(
    state: &mut State,
    store: &mut EditorStore,
    project: &ProjectId,
    view: ViewId,
    provider: ProviderIdentity,
) -> Request {
    state
        .queue(store, context(project, view, view), true)
        .unwrap();
    state
        .before(
            store,
            context(project, view, view),
            HashSet::from([provider]),
            RULES,
        )
        .unwrap()
        .pop()
        .unwrap()
}

fn node(
    start: u32,
    end: u32,
    parent: Option<lsp_types::SelectionRange>,
) -> lsp_types::SelectionRange {
    lsp_types::SelectionRange {
        range: lsp_types::Range::new(
            lsp_types::Position::new(0, start),
            lsp_types::Position::new(0, end),
        ),
        parent: parent.map(Box::new),
    }
}

#[test]
fn 외부_범위는_단어와_합쳐_확장하고_재요청_없이_원선택까지_축소한다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let before = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    let mut state = State::default();
    let request = submit(&mut state, &mut store, &project, view, provider);
    assert_eq!(selection(&store, view), vec![(2, 2)]);
    assert!(state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Ok(Response {
            groups: vec![(provider, Some(vec![Some(node(0, 11, None))]))],
        })
    ));
    assert!(
        state
            .before(
                &mut store,
                context(&project, view, view),
                HashSet::from([provider]),
                RULES
            )
            .unwrap()
            .is_empty()
    );
    assert_eq!(selection(&store, view), vec![(0, 5)]);
    for expected in [vec![(0, 11)], vec![(0, TEXT.len())]] {
        state
            .queue(&store, context(&project, view, view), true)
            .unwrap();
        assert!(
            state
                .before(
                    &mut store,
                    context(&project, view, view),
                    HashSet::from([provider]),
                    RULES
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(selection(&store, view), expected);
    }
    for expected in [vec![(0, 11)], vec![(0, 5)], vec![(2, 2)]] {
        state
            .queue(&store, context(&project, view, view), false)
            .unwrap();
        assert!(
            state
                .before(
                    &mut store,
                    context(&project, view, view),
                    HashSet::from([provider]),
                    RULES
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(selection(&store, view), expected);
    }
    assert!(!request.is_cancelled());
    let after = store.documents().snapshot(before.id).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope.to_string(), before.rope.to_string());
    assert!(!after.dirty);
    state.clear(&mut store);
    assert!(request.is_cancelled());
    assert!(!has_smart_selection(&store, view));
}

#[test]
fn 위치별_null과_잘못된_위치수_공급자를_분리하고_부모는_유효한_범위만_합친다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, true);
    set_selection(&mut store, view, &[(2, 2), (9, 9)]);
    let mut state = State::default();
    let request = submit(&mut state, &mut store, &project, view, provider);
    let unknown = ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        ..provider
    };
    assert!(state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Ok(Response {
            groups: vec![
                (
                    unknown,
                    Some(vec![Some(node(0, 11, None)), Some(node(0, 11, None))])
                ),
                (provider, Some(vec![Some(node(0, 11, None))])),
                (
                    provider,
                    Some(vec![Some(node(99, 100, Some(node(0, 11, None)))), None])
                ),
            ],
        })
    ));
    state
        .before(
            &mut store,
            context(&project, view, view),
            HashSet::from([provider]),
            RULES,
        )
        .unwrap();
    assert_eq!(selection(&store, view), vec![(0, 5), (9, 10)]);
    state
        .queue(&store, context(&project, view, view), true)
        .unwrap();
    state
        .before(
            &mut store,
            context(&project, view, view),
            HashSet::from([provider]),
            RULES,
        )
        .unwrap();
    assert_eq!(selection(&store, view), vec![(0, 11)]);
    state
        .queue(&store, context(&project, view, view), false)
        .unwrap();
    state
        .before(
            &mut store,
            context(&project, view, view),
            HashSet::from([provider]),
            RULES,
        )
        .unwrap();
    assert_eq!(selection(&store, view), vec![(0, 5), (9, 10)]);
    assert!(
        store
            .documents()
            .snapshot(request.snapshot.id)
            .unwrap()
            .metadata
            .read_only
    );
}

#[test]
fn 대기와_오류_응답은_안정_프레임과_반복_명령에서_자동_재요청하지_않는다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let mut state = State::default();
    let request = submit(&mut state, &mut store, &project, view, provider);
    for _ in 0..STABLE_FRAMES {
        state
            .queue(&store, context(&project, view, view), true)
            .unwrap();
        assert!(
            state
                .before(
                    &mut store,
                    context(&project, view, view),
                    HashSet::from([provider]),
                    RULES
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            state.entries[&(egui::ViewportId::ROOT, view)].request.token,
            request.token
        );
    }
    assert!(state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Err(Failure::TransportClosed)
    ));
    state
        .before(
            &mut store,
            context(&project, view, view),
            HashSet::from([provider]),
            RULES,
        )
        .unwrap();
    assert_eq!(selection(&store, view), vec![(0, 5)]);
    for _ in 0..STABLE_FRAMES {
        assert!(
            state
                .before(
                    &mut store,
                    context(&project, view, view),
                    HashSet::from([provider]),
                    RULES
                )
                .unwrap()
                .is_empty()
        );
        assert!(!request.is_cancelled());
    }
}

#[test]
fn 수동_선택과_공급자_교체와_닫힘은_응답과_완료_캐시를_회수한다() {
    let (mut store, view, project, provider) = fixture(FileSizeTier::Normal, false);
    let mut state = State::default();
    let request = submit(&mut state, &mut store, &project, view, provider);
    set_selection(&mut store, view, &[(7, 7)]);
    state.after(&mut store, egui::ViewportId::ROOT, view);
    assert!(request.is_cancelled());
    assert!(!state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Ok(Response { groups: Vec::new() })
    ));
    let current = submit(&mut state, &mut store, &project, view, provider);
    assert!(state.accept(
        &store,
        &current,
        HashSet::from([provider]),
        Ok(Response { groups: Vec::new() })
    ));
    state
        .before(
            &mut store,
            context(&project, view, view),
            HashSet::from([provider]),
            RULES,
        )
        .unwrap();
    assert!(has_smart_selection(&store, view));
    let replacement = ProviderIdentity {
        generation: provider.generation + 1,
        ..provider
    };
    state.reconcile(&mut store, |_| true, |_, _| HashSet::from([replacement]));
    assert!(current.is_cancelled());
    assert!(!has_smart_selection(&store, view));
    let pending = submit(&mut state, &mut store, &project, view, replacement);
    state.reconcile(&mut store, |_| false, |_, _| HashSet::from([replacement]));
    assert!(pending.is_cancelled());
    assert!(state.entries.is_empty());
}

#[test]
fn 프로젝트_없음과_큰_tier는_로컬_선택을_유지하고_peek의_readonly_선택은_본문과_분리한다() {
    for tier in [
        FileSizeTier::Normal,
        FileSizeTier::Large,
        FileSizeTier::ReadOnly,
    ] {
        let (mut store, view, project, provider) = fixture(tier, true);
        let mut state = State::default();
        let local = Context {
            project: None,
            owner: view,
            source: view,
            viewport: egui::ViewportId::ROOT,
        };
        state.queue(&store, local, true).unwrap();
        assert!(
            state
                .before(
                    &mut store,
                    Context {
                        project: None,
                        owner: view,
                        source: view,
                        viewport: egui::ViewportId::ROOT
                    },
                    HashSet::from([provider]),
                    RULES
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(selection(&store, view), vec![(0, 5)]);
        assert!(state.entries.is_empty());
        set_selection(&mut store, view, &[(2, 2)]);
        let document = store.views().get(view).unwrap().document;
        let preview = store
            .attach_view(
                ViewKey {
                    window: "selection-preview".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap();
        set_selection(&mut store, preview, &[(7, 7)]);
        state
            .queue(&store, context(&project, view, preview), true)
            .unwrap();
        let submitted = state
            .before(
                &mut store,
                context(&project, view, preview),
                HashSet::from([provider]),
                RULES,
            )
            .unwrap();
        if tier == FileSizeTier::Normal {
            let request = &submitted[0];
            assert_eq!(request.source, preview);
            assert_eq!(request.owner, view);
            assert!(state.accept(
                &store,
                request,
                HashSet::from([provider]),
                Ok(Response { groups: Vec::new() })
            ));
            state
                .before(
                    &mut store,
                    context(&project, view, preview),
                    HashSet::from([provider]),
                    RULES,
                )
                .unwrap();
        } else {
            assert!(submitted.is_empty());
        }
        assert_eq!(selection(&store, view), vec![(2, 2)]);
        assert_eq!(selection(&store, preview), vec![(6, 7)]);
    }
}

use super::*;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::Selection;

const BYTE_LIMIT: usize = 4096;
const VIEW_LIMIT: usize = 4;
const COLOR_TOLERANCE: u8 = 2;
const SELECTION_ALPHA: u8 = 153;
const FILE_CONTENT: &str = "foo = foo\nfoo";
const FILE_PATH: &str = "/synthetic/highlights-readonly.txt";

fn opened_file(tier: FileSizeTier, read_only: bool) -> OpenedFile {
    OpenedFile {
        path: FILE_PATH.into(),
        content: FILE_CONTENT.into(),
        language_id: "plaintext".into(),
        byte_size: FILE_CONTENT.len().try_into().unwrap(),
        line_count: FILE_CONTENT.lines().count().try_into().unwrap(),
        tier,
        read_only,
        encoding_lossy: false,
        modified_ms: 0.0,
        editor_config: EditorConfigOptions::default(),
    }
}

fn file_fixture(
    tier: FileSizeTier,
    read_only: bool,
) -> (EditorStore, ViewId, ProviderIdentity, ProjectId) {
    let (mut store, _, provider, project) = fixture();
    let document = store
        .open_file(FILE_PATH.into(), opened_file(tier, read_only))
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
    (store, view, provider, project)
}

fn fixture() -> (EditorStore, ViewId, ProviderIdentity, ProjectId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: VIEW_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: VIEW_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_untitled(TabId::new(), "foo = foo\nfoo", "plaintext".into())
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
    let provider = ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    };
    (store, view, provider, ProjectId::new())
}

fn context(project: &ProjectId, view: ViewId) -> Context {
    Context {
        project: project.clone(),
        source: view,
        owner: view,
        viewport: egui::ViewportId::ROOT,
    }
}

fn select(store: &mut EditorStore, view: ViewId, anchor: usize, head: usize) {
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
}

fn request(
    state: &mut State,
    store: &EditorStore,
    view: ViewId,
    provider: ProviderIdentity,
    project: &ProjectId,
) -> Request {
    let now = Instant::now();
    assert!(
        state
            .observe(
                store,
                context(project, view),
                HashSet::from([provider]),
                now
            )
            .unwrap()
            .is_none()
    );
    state
        .observe(
            store,
            context(project, view),
            HashSet::from([provider]),
            now + REQUEST_DEBOUNCE,
        )
        .unwrap()
        .unwrap()
}

fn response(provider: ProviderIdentity) -> Response {
    Response {
        provider: Some(provider),
        highlights: vec![
            lsp_types::DocumentHighlight {
                range: LspRange::new(Position::new(0, 0), Position::new(0, 3)),
                kind: None,
            },
            lsp_types::DocumentHighlight {
                range: LspRange::new(Position::new(0, 6), Position::new(0, 9)),
                kind: Some(lsp_types::DocumentHighlightKind::READ),
            },
            lsp_types::DocumentHighlight {
                range: LspRange::new(Position::new(1, 0), Position::new(1, 3)),
                kind: Some(lsp_types::DocumentHighlightKind::WRITE),
            },
        ],
    }
}

fn colors() -> Colors {
    Colors {
        background: [Color32::RED, Color32::GREEN, Color32::BLUE],
        border: [Color32::WHITE; HIGHLIGHT_KIND_COUNT],
        overview: [Color32::GRAY; HIGHLIGHT_KIND_COUNT],
        minimap: Color32::LIGHT_GRAY,
    }
}

#[test]
fn 완료된_하이라이트_안의_커서_이동은_표시와_요청을_재사용한다() {
    let (mut store, view, provider, project) = fixture();
    let mut state = State::default();
    let initial = request(&mut state, &store, view, provider, &project);
    assert!(state.accept(
        &store,
        &initial,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    for caret in [1, 7, 12] {
        select(&mut store, view, caret, caret);
        state.reconcile(&store, |_| true, |_, _| HashSet::from([provider]));
        assert!(
            state
                .display(&store, egui::ViewportId::ROOT, view, colors())
                .is_some()
        );
        assert!(
            state
                .observe(
                    &store,
                    context(&project, view),
                    HashSet::from([provider]),
                    Instant::now() + REQUEST_DEBOUNCE,
                )
                .unwrap()
                .is_none()
        );
        assert_eq!(
            state.entries[&egui::ViewportId::ROOT].request.token,
            initial.token
        );
        assert!(!initial.is_cancelled());
    }
}

#[test]
fn closing_전체_뷰포트의_대기_완료_요청과_표시를_즉시_회수한다() {
    let (store, view, provider, project) = fixture();
    let mut state = State::default();
    let completed = request(&mut state, &store, view, provider, &project);
    assert!(state.accept(
        &store,
        &completed,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    let secondary = egui::ViewportId::from_hash_of("synthetic secondary");
    let now = Instant::now();
    state
        .observe(
            &store,
            Context {
                viewport: secondary,
                ..context(&project, view)
            },
            HashSet::from([provider]),
            now,
        )
        .unwrap();
    let pending = state.current_request(secondary).unwrap();
    assert!(!completed.is_cancelled());
    assert!(!pending.is_cancelled());
    state.clear();
    assert!(completed.is_cancelled());
    assert!(pending.is_cancelled());
    for viewport in [egui::ViewportId::ROOT, secondary] {
        assert!(state.current_request(viewport).is_none());
        assert!(state.display(&store, viewport, view, colors()).is_none());
    }
    assert!(state.sources.is_empty());
}

#[test]
fn navigation_boundary_기호로_시작하는_범위로_이동해도_표시를_유지한다() {
    let (mut store, view, provider, project) = fixture();
    let mut state = State::default();
    let initial = request(&mut state, &store, view, provider, &project);
    let mut result = response(provider);
    result.highlights.push(lsp_types::DocumentHighlight {
        range: LspRange::new(Position::new(0, 4), Position::new(0, 5)),
        kind: None,
    });
    assert!(state.accept(&store, &initial, HashSet::from([provider]), Ok(result)));
    assert!(
        state
            .navigate(&mut store, context(&project, view), HighlightCommand::Next)
            .unwrap()
    );
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0].head,
        4
    );
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                Instant::now()
            )
            .unwrap()
            .is_none()
    );
    assert!(state.has_highlights(&store, egui::ViewportId::ROOT, view));
    assert_eq!(
        state.entries[&egui::ViewportId::ROOT].request.token,
        initial.token
    );
}

#[test]
fn navigation_boundary_readonly_메타데이터의_문서는_내용을_바꾸지_않고_이동한다() {
    let (mut store, view, provider, project) = file_fixture(FileSizeTier::Normal, true);
    let document = store.views().get(view).unwrap().document;
    let before = store.documents().snapshot(document).unwrap();
    assert!(before.metadata.read_only);
    let mut state = State::default();
    let initial = request(&mut state, &store, view, provider, &project);
    assert!(state.accept(
        &store,
        &initial,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    assert!(
        state
            .navigate(&mut store, context(&project, view), HighlightCommand::Next)
            .unwrap()
    );
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0].head,
        6
    );
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope.to_string(), FILE_CONTENT);
    assert!(after.metadata.read_only);
}

#[test]
fn 하이라이트_이동은_정렬_중복_순환과_단일_커서_중앙_reveal을_보존한다() {
    let (mut store, view, provider, project) = fixture();
    let mut state = State::default();
    let initial = request(&mut state, &store, view, provider, &project);
    let mut result = response(provider);
    result.highlights.reverse();
    result.highlights.push(result.highlights[0].clone());
    assert!(state.accept(&store, &initial, HashSet::from([provider]), Ok(result)));
    let revision = initial.snapshot.revision;
    for (command, caret, bytes) in [
        (HighlightCommand::Next, 6, 6..9),
        (HighlightCommand::Next, 10, 10..13),
        (HighlightCommand::Next, 0, 0..3),
        (HighlightCommand::Previous, 10, 10..13),
        (HighlightCommand::Previous, 6, 6..9),
        (HighlightCommand::Previous, 0, 0..3),
    ] {
        assert!(
            state
                .navigate(&mut store, context(&project, view), command)
                .unwrap()
        );
        let current = store.views().get(view).unwrap().clone();
        assert_eq!(
            current.selection,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: caret,
                    head: caret
                }]
            }
        );
        let reveal = store.take_selection_reveal(view).unwrap().unwrap();
        assert_eq!(reveal.bytes, bytes);
        assert!(reveal.center_if_outside);
        assert_eq!(
            store
                .documents()
                .snapshot(current.document)
                .unwrap()
                .revision,
            revision
        );
        assert_eq!(
            state.entries[&egui::ViewportId::ROOT].request.token,
            initial.token
        );
    }
}

#[test]
fn mirror_하이라이트_이동은_대상_뷰만_옮기고_포커스_소유자를_유지한다() {
    let (mut store, view, provider, project) = fixture();
    let document = store.views().get(view).unwrap().document;
    let mirror = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let mut state = State::default();
    let initial = request(&mut state, &store, view, provider, &project);
    assert!(state.accept(
        &store,
        &initial,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    assert!(
        state
            .navigate(
                &mut store,
                Context {
                    source: mirror,
                    ..context(&project, view)
                },
                HighlightCommand::Next
            )
            .unwrap()
    );
    assert_eq!(
        store.views().get(view).unwrap().selection,
        SelectionSet::default()
    );
    assert_eq!(
        store.views().get(mirror).unwrap().selection.selections[0].head,
        6
    );
    state
        .observe(
            &store,
            Context {
                source: mirror,
                ..context(&project, view)
            },
            HashSet::from([provider]),
            Instant::now(),
        )
        .unwrap();
    assert_eq!(
        state.source_for_owner(&store, egui::ViewportId::ROOT, view),
        mirror
    );
    assert!(
        state
            .display(&store, egui::ViewportId::ROOT, view, colors())
            .is_some()
    );
    assert_eq!(state.entries[&egui::ViewportId::ROOT].request.owner, view);
}

#[test]
fn 명시적_trigger는_빈_결과를_다시_요청하고_250ms_표시_지연과_캐시를_보존한다() {
    let (store, view, provider, project) = fixture();
    let mut state = State::default();
    let initial = request(&mut state, &store, view, provider, &project);
    assert!(state.accept(
        &store,
        &initial,
        HashSet::from([provider]),
        Ok(Response {
            provider: Some(provider),
            highlights: Vec::new()
        })
    ));
    let now = Instant::now();
    let triggered = state
        .trigger(
            &store,
            context(&project, view),
            HashSet::from([provider]),
            now,
        )
        .unwrap()
        .unwrap();
    assert_ne!(triggered.token, initial.token);
    assert!(initial.is_cancelled());
    assert_eq!(
        state.entries[&egui::ViewportId::ROOT].render_after,
        now + TRIGGER_DELAY
    );
    assert!(state.accept(
        &store,
        &triggered,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    assert!(
        state
            .display(&store, egui::ViewportId::ROOT, view, colors())
            .is_none()
    );
    assert!(!state.has_highlights(&store, egui::ViewportId::ROOT, view));
    assert!(
        state
            .trigger(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                now
            )
            .unwrap()
            .is_none()
    );
    assert_eq!(
        state.entries[&egui::ViewportId::ROOT].request.token,
        triggered.token
    );
}

#[test]
fn 주_선택의_단어_조건과_50ms_debounce_및_동일_요청의_단일_제출을_보존한다() {
    let (mut store, view, provider, project) = fixture();
    let mut state = State::default();
    let now = Instant::now();
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
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                now + REQUEST_DEBOUNCE - Duration::from_nanos(1)
            )
            .unwrap()
            .is_none()
    );
    let first = state
        .observe(
            &store,
            context(&project, view),
            HashSet::from([provider]),
            now + REQUEST_DEBOUNCE,
        )
        .unwrap()
        .unwrap();
    assert_eq!(first.position, Position::new(0, 0));
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                now + REQUEST_DEBOUNCE
            )
            .unwrap()
            .is_none()
    );
    select(&mut store, view, 0, 9);
    assert!(
        state
            .observe(
                &store,
                context(&project, view),
                HashSet::from([provider]),
                now + REQUEST_DEBOUNCE
            )
            .unwrap()
            .is_none()
    );
    assert!(first.is_cancelled());
    assert!(
        state
            .display(&store, egui::ViewportId::ROOT, view, colors())
            .is_none()
    );
}

#[test]
fn 종류_기본값과_read_write_배경_테두리_overview_minimap을_같은_문서_mirror에_공유한다() {
    let (mut store, view, provider, project) = fixture();
    let document = store.views().get(view).unwrap().document;
    let mirror = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let mut state = State::default();
    let request = request(&mut state, &store, view, provider, &project);
    assert!(state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    for target in [view, mirror] {
        let display = state
            .display(&store, egui::ViewportId::ROOT, target, colors())
            .unwrap();
        assert_eq!(display.layer.items().len(), HIGHLIGHT_KIND_COUNT * 2);
        assert_eq!(display.borders.len(), HIGHLIGHT_KIND_COUNT);
        for (bytes, color) in [
            (0..3, Color32::RED),
            (6..9, Color32::GREEN),
            (10..13, Color32::BLUE),
        ] {
            assert!(display.layer.items().iter().any(|decoration| decoration.bytes == bytes && matches!(decoration.kind, DecorationKind::Inline(InlineStyle { background: Some(value), .. }) if value == color.to_srgba_unmultiplied())));
        }
    }
    assert!(!request.is_cancelled());
}

#[test]
fn 선택이_바뀌면_이전_응답을_거절하고_새_요청을_취소하지_않는다() {
    let (mut store, view, provider, project) = fixture();
    let mut state = State::default();
    let old = request(&mut state, &store, view, provider, &project);
    select(&mut store, view, 6, 6);
    let new = request(&mut state, &store, view, provider, &project);
    assert!(old.is_cancelled());
    assert!(!state.accept(
        &store,
        &old,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    state.reject(&old);
    assert!(!new.is_cancelled());
    assert!(state.accept(
        &store,
        &new,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
}

#[test]
fn 공급자와_활성_소유_변경은_대기와_완료_표시를_취소한다() {
    let (store, view, provider, project) = fixture();
    let mut state = State::default();
    let old = request(&mut state, &store, view, provider, &project);
    let changed = ProviderIdentity {
        capability_revision: 1,
        ..provider
    };
    state.reconcile(&store, |_| true, |_, _| HashSet::from([changed]));
    assert!(old.is_cancelled());
    let new = request(&mut state, &store, view, changed, &project);
    assert!(!state.accept(
        &store,
        &new,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    state.reconcile(&store, |_| false, |_, _| HashSet::from([changed]));
    assert!(new.is_cancelled());
}

#[test]
fn highlight_tier_동일_revision의_크기_등급_변경은_대기와_표시를_만료한다() {
    for tier in [FileSizeTier::Large, FileSizeTier::ReadOnly] {
        for completed in [false, true] {
            let (mut store, view, provider, project) = file_fixture(FileSizeTier::Normal, false);
            let mut state = State::default();
            let old = request(&mut state, &store, view, provider, &project);
            if completed {
                assert!(state.accept(
                    &store,
                    &old,
                    HashSet::from([provider]),
                    Ok(response(provider))
                ));
                assert!(state.has_highlights(&store, egui::ViewportId::ROOT, view));
            }
            store
                .observe_file(
                    old.snapshot.id,
                    std::path::Path::new(FILE_PATH),
                    opened_file(tier, tier == FileSizeTier::ReadOnly),
                )
                .unwrap();
            let snapshot = store.documents().snapshot(old.snapshot.id).unwrap();
            assert_eq!(snapshot.revision, old.snapshot.revision);
            assert_eq!(snapshot.metadata.tier, tier);
            assert!(!old.describes(&store), "{tier:?}, completed={completed}");
            assert!(
                state
                    .display(&store, egui::ViewportId::ROOT, view, colors())
                    .is_none()
            );
            state.reconcile(&store, |_| true, |_, _| HashSet::from([provider]));
            assert!(old.is_cancelled());
            assert!(!state.accept(
                &store,
                &old,
                HashSet::from([provider]),
                Ok(response(provider))
            ));
            assert!(
                state
                    .observe(
                        &store,
                        context(&project, view),
                        HashSet::new(),
                        Instant::now() + REQUEST_DEBOUNCE
                    )
                    .unwrap()
                    .is_none()
            );
            store
                .observe_file(
                    old.snapshot.id,
                    std::path::Path::new(FILE_PATH),
                    opened_file(FileSizeTier::Normal, true),
                )
                .unwrap();
            let restored = request(&mut state, &store, view, provider, &project);
            assert_ne!(restored.token, old.token);
            assert!(!restored.is_cancelled());
            assert!(
                store
                    .documents()
                    .snapshot(old.snapshot.id)
                    .unwrap()
                    .metadata
                    .read_only
            );
        }
    }
}

#[test]
fn 빈_결과와_오류는_표시를_비우며_매_프레임_재요청하지_않는다() {
    let (store, view, provider, project) = fixture();
    for result in [
        Ok(Response {
            provider: Some(provider),
            highlights: Vec::new(),
        }),
        Err(Failure::TimedOut),
    ] {
        let mut state = State::default();
        let request = request(&mut state, &store, view, provider, &project);
        assert!(state.accept(&store, &request, HashSet::from([provider]), result));
        assert!(
            state
                .display(&store, egui::ViewportId::ROOT, view, colors())
                .unwrap()
                .layer
                .items()
                .is_empty()
        );
        assert!(
            state
                .observe(
                    &store,
                    context(&project, view),
                    HashSet::from([provider]),
                    Instant::now() + REQUEST_DEBOUNCE
                )
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn 뷰_회수와_문서_편집은_표시와_오래된_응답을_거절한다() {
    let (mut store, view, provider, project) = fixture();
    let mut state = State::default();
    let request = request(&mut state, &store, view, provider, &project);
    let document = request.snapshot.id;
    store
        .apply(
            document,
            taide_native_editor::store::Transaction {
                revision: request.snapshot.revision,
                edits: vec![taide_native_editor::document::Edit {
                    bytes: 0..0,
                    text: "new ".into(),
                }],
                group: taide_native_editor::document::UndoGroup(1),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    assert!(!state.accept(
        &store,
        &request,
        HashSet::from([provider]),
        Ok(response(provider))
    ));
    assert!(
        state
            .display(&store, egui::ViewportId::ROOT, view, colors())
            .is_none()
    );
    store.detach_view(view).unwrap();
    state.reconcile(&store, |_| true, |_, _| HashSet::from([provider]));
    assert!(request.is_cancelled());
}

#[test]
fn minimap_기본_색은_monaco_selection_highlight의_less_prominent_원본_표본과_일치한다() {
    for (selected, background, expected) in [
        ("#45475a", "#1e1e2e", [53, 55, 69]),
        ("#bcc0cc", "#ffffff", [220, 222, 228]),
        ("#264f78", "#1e1e1e", [29, 59, 90]),
        ("#add6ff", "#ffffff", [219, 237, 255]),
    ] {
        let parse = |value| taide_native_ui::presentation::parse_color(value, "synthetic").unwrap();
        let actual =
            selection_highlight(parse(selected), parse(background)).to_srgba_unmultiplied();
        for (actual, expected) in actual[..expected.len()].iter().zip(expected) {
            assert!(
                actual.abs_diff(expected) <= COLOR_TOLERANCE,
                "{selected}/{background}: {actual} vs {expected}"
            );
        }
        assert_eq!(actual[expected.len()], SELECTION_ALPHA);
    }
}

#[test]
fn 테마의_종류별_색과_text_별칭_테두리_및_minimap_명시값을_보존한다() {
    let theme = serde_json::from_value::<ResolvedTheme>(serde_json::json!({
        "id":"synthetic", "name":"synthetic", "type":"dark",
        "colors": {
            "editor.wordHighlightBackground":"#123456",
            "editor.wordHighlightBorder":"#abcdef",
            "editor.wordHighlightStrongBackground":"#654321",
            "minimap.selectionOccurrenceHighlight":"#fedcba",
        }, "syntax":{}, "terminal":{},
    }))
    .unwrap();
    let colors = Colors::from_theme(&theme).unwrap();
    assert_eq!(colors.background[0], Color32::from_rgb(0x12, 0x34, 0x56));
    assert_eq!(colors.background[1], colors.background[0]);
    assert_eq!(
        colors.background[WRITE_KIND],
        Color32::from_rgb(0x65, 0x43, 0x21)
    );
    assert_eq!(colors.border[0], colors.border[1]);
    assert_eq!(colors.border[0], Color32::from_rgb(0xab, 0xcd, 0xef));
    assert_eq!(colors.minimap, Color32::from_rgb(0xfe, 0xdc, 0xba));
}

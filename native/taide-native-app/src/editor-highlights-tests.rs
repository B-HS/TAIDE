use super::*;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::Selection;

const BYTE_LIMIT: usize = 4096;
const VIEW_LIMIT: usize = 4;
const COLOR_TOLERANCE: u8 = 2;
const SELECTION_ALPHA: u8 = 153;

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

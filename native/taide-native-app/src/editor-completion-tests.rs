use super::*;
use taide_lsp::native::protocol::lsp_types::{CompletionItem, CompletionList, CompletionResponse};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::{Composition, ScrollPosition, Selection};

const BYTE_LIMIT: usize = 4096;
const OWNER_LIMIT: usize = 4;
const UNDO_LIMIT: usize = 2;
const PREPARATION_MARKER_LIMIT: usize = 128;
const PREPARATION_NESTING_LIMIT: usize = 32;
const PREPARATION_TAB_SIZE: u32 = 4;
const SMALL_PREPARATION_BYTE_LIMIT: usize = 4;

fn fixture() -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: OWNER_LIMIT,
        max_views: OWNER_LIMIT,
        max_undo_groups: UNDO_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_untitled(TabId::new(), "con", "rust".into())
        .unwrap();
    let view = attach(&mut store, document);
    (store, document, view)
}

fn attach(store: &mut EditorStore, document: DocumentId) -> ViewId {
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
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 3, head: 3 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    view
}

fn provider() -> ProviderIdentity {
    ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    }
}

fn context(project: &ProjectId, source: ViewId, owner: ViewId) -> Context {
    Context {
        project: project.clone(),
        source,
        owner,
        word: 0..3,
        viewport: eframe::egui::ViewportId::ROOT,
        automatic: false,
    }
}

#[test]
fn completion_cache의_완료후보는_편집과_현재입력에_맞춰_갱신하고_이전응답을_거절한다() {
    let (mut store, document, view) = fixture();
    let project = ProjectId::new();
    let identity = provider();
    let providers = HashSet::from([identity]);
    let projects = HashSet::from([project.clone()]);
    let mut state = State::default();
    let request = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    state
        .accept(
            &store,
            &request,
            providers.clone(),
            Ok(Response {
                groups: vec![group(&request, identity, 0, "console", false)],
            }),
        )
        .unwrap();
    let model = state.model(&store, view).unwrap().unwrap();
    assert_eq!(model.borrow_mut().filter("con", 0).len(), 1);
    taide_native_editor::editing::replace_selections(&mut store, view, "s", None).unwrap();
    state.reconcile(&store, &projects, |_| true, |_, _| providers.clone());
    let current = state.model(&store, view).unwrap();
    assert!(current.is_some());
    assert!(Rc::ptr_eq(&model, current.as_ref().unwrap()));
    assert!(request.is_cancelled());
    assert!(!state.is_current(&request));
    assert!(
        !state
            .accept(
                &store,
                &request,
                providers,
                Ok(Response { groups: Vec::new() })
            )
            .unwrap()
    );
    let clock = preparation_clock();
    let prepared = state
        .prepare_candidate(
            &store,
            view,
            request.token,
            0,
            preparation_options(),
            preparation_context(&clock),
        )
        .unwrap();
    assert_eq!(prepared.snippets[0].replace, 0..4);
    let owner = store.views().get(view).unwrap().clone();
    let revision = store.documents().snapshot(document).unwrap().revision;
    taide_native_editor::snippet_insertion::insert(
        &mut store,
        &owner,
        revision,
        prepared.snippets,
        preparation_options().limits,
    )
    .unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "console"
    );
}

#[test]
fn completion_cache는_불완전provider만_재요청하고_완료그룹과_표시token을_보존한다() {
    let (mut store, _, view) = fixture();
    let project = ProjectId::new();
    let complete = provider();
    let incomplete = provider();
    let providers = HashSet::from([complete, incomplete]);
    let projects = HashSet::from([project.clone()]);
    let mut state = State::default();
    let request = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    state
        .accept(
            &store,
            &request,
            providers.clone(),
            Ok(Response {
                groups: vec![
                    group(&request, complete, 0, "console", false),
                    group(&request, incomplete, 1, "constant", true),
                ],
            }),
        )
        .unwrap();
    let previous = state.model(&store, view).unwrap().unwrap();
    taide_native_editor::editing::replace_selections(&mut store, view, "s", None).unwrap();
    state.reconcile(&store, &projects, |_| true, |_, _| providers.clone());
    assert!(state.pending(&store, view));
    assert!(Rc::ptr_eq(
        &previous,
        &state.model(&store, view).unwrap().unwrap()
    ));
    assert_eq!(
        state.display(&store, view),
        Some((request.token, "cons", 1))
    );
    let queued = state.take_queued();
    assert_eq!(queued.len(), 1);
    assert!(state.take_queued().is_empty());
    let next = &queued[0];
    assert_eq!(next.providers, providers);
    assert_eq!(next.query_providers, HashSet::from([incomplete]));
    assert_ne!(next.token, request.token);
    assert!(request.is_cancelled());
    state
        .accept(
            &store,
            next,
            providers,
            Ok(Response {
                groups: vec![group(next, incomplete, 1, "consensus", false)],
            }),
        )
        .unwrap();
    assert!(!state.pending(&store, view));
    let model = state.model(&store, view).unwrap().unwrap();
    let ranked = model.borrow_mut().filter("cons", 0).to_vec();
    let labels = ranked
        .iter()
        .map(|row| {
            model
                .borrow()
                .candidate(row.candidate)
                .unwrap()
                .item
                .label
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, ["consensus", "console"]);
    for group in state.groups(&store, view).unwrap().values() {
        for item in &group.candidates.items {
            assert_eq!(item.revision, next.snapshot.revision);
        }
    }
    assert_eq!(
        state.display(&store, view),
        Some((request.token, "cons", 0))
    );
}

#[test]
fn completion_cache는_줄바꿈_공백_새단어에서_기존후보를_닫는다() {
    for text in ["\n", " ", "."] {
        let (mut store, _, view) = fixture();
        let project = ProjectId::new();
        let identity = provider();
        let providers = HashSet::from([identity]);
        let mut state = State::default();
        let request = state
            .begin(&store, context(&project, view, view), providers.clone())
            .unwrap();
        state
            .accept(
                &store,
                &request,
                providers,
                Ok(Response {
                    groups: vec![group(&request, identity, 0, "console", false)],
                }),
            )
            .unwrap();
        state.model(&store, view).unwrap();
        taide_native_editor::editing::replace_selections(&mut store, view, text, None).unwrap();
        assert!(!state.refresh_view(&store, view).unwrap());
        assert!(request.is_cancelled());
        assert!(state.display(&store, view).is_none());
        assert!(state.take_queued().is_empty());
    }
}

fn group(
    request: &Request,
    provider: ProviderIdentity,
    ordinal: usize,
    label: &str,
    is_incomplete: bool,
) -> Group {
    Group {
        provider,
        ordinal,
        candidates: Candidates::new(
            &request.snapshot,
            request.position,
            request.word,
            Some(CompletionResponse::List(CompletionList {
                is_incomplete,
                items: vec![CompletionItem {
                    label: label.into(),
                    ..Default::default()
                }],
            })),
        ),
    }
}

fn preparation_options() -> taide_native_editor::completion::PreparationOptions {
    taide_native_editor::completion::PreparationOptions {
        alternate: false,
        indent: taide_native_editor::indent::IndentOptions {
            tab_size: PREPARATION_TAB_SIZE,
            insert_spaces: true,
        },
        limits: taide_native_editor::snippet_syntax::ParseLimits {
            max_bytes: BYTE_LIMIT,
            max_markers: PREPARATION_MARKER_LIMIT,
            max_nesting: PREPARATION_NESTING_LIMIT,
        },
    }
}

fn preparation_clock() -> SnippetClock {
    SnippetClock {
        date: chrono::DateTime::parse_from_rfc3339("2026-10-10T12:00:00+09:00").unwrap(),
        timezone_name: Some("Asia/Seoul".into()),
    }
}

fn preparation_context(clock: &SnippetClock) -> PreparationContext<'_> {
    PreparationContext {
        model_path: "/work/example.tar.rs",
        language: taide_native_syntax::monaco_language("rust")
            .unwrap()
            .unwrap(),
        clipboard: Some("first\nsecond"),
        clipboard_spread: true,
        clock,
    }
}

fn supplied_candidate(
    store: &EditorStore,
    view: ViewId,
    body: &str,
    snippet: bool,
) -> (State, Request) {
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(store, context(&project, view, view), providers.clone())
        .unwrap();
    let mut group = group(&request, provider, 0, "proposal", false);
    group.candidates.items[0].item.insert_text = Some(body.to_owned());
    if snippet {
        group.candidates.items[0].item.insert_text_format =
            Some(taide_lsp::native::protocol::lsp_types::InsertTextFormat::SNIPPET);
    }
    state
        .accept(
            store,
            &request,
            providers,
            Ok(Response {
                groups: vec![group],
            }),
        )
        .unwrap();
    for clipboard_request in state.take_clipboard_requests(store) {
        assert!(state.accept_clipboard(store, &clipboard_request, "first\nsecond".into()));
    }
    (state, request)
}

#[test]
fn completion_preparation은_원본의_덮어쓴선택과_실제_변수_엔진을_사용한다() {
    let (mut store, document, view) = fixture();
    let body = "${TM_SELECTED_TEXT}|${TM_FILENAME_BASE}|${CLIPBOARD/(.*)/${1:/upcase}/s}|${CURRENT_YEAR}|${LINE_COMMENT}|${1:initial}|${1/(.*)/${1:/upcase}/}$0";
    let (mut state, request) = supplied_candidate(&store, view, body, true);
    let clock = preparation_clock();
    let before = store.documents().snapshot(document).unwrap();
    let mut prepared = state
        .prepare_candidate(
            &store,
            view,
            request.token,
            0,
            preparation_options(),
            preparation_context(&clock),
        )
        .unwrap();
    assert_eq!(
        before.revision,
        store.documents().snapshot(document).unwrap().revision
    );
    assert_eq!(
        prepared.snippets[0].expansion.text,
        "con|example.tar|FIRST\nSECOND|2026|//|initial|initial"
    );
    let expected_view = store.views().get(view).unwrap().clone();
    let insertion = taide_native_editor::snippet_insertion::insert(
        &mut store,
        &expected_view,
        before.revision,
        prepared.snippets,
        preparation_options().limits,
    )
    .unwrap();
    let mut session = taide_native_editor::snippet_session::Session::new(
        &store,
        insertion,
        preparation_options().indent,
        preparation_options().limits,
    )
    .unwrap();
    assert!(session.is_active());
    assert!(session.replace(&mut store, "changed", None).unwrap());
    assert!(
        session
            .step(&mut store, true, |request| prepared
                .transforms
                .evaluate(request.transform, request.value))
            .unwrap()
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con|example.tar|FIRST\nSECOND|2026|//|changed|CHANGED"
    );
    assert!(!session.is_active());
}

#[test]
fn completion_preparation은_plain_달러를_보존하고_수락만_문서를_변경한다() {
    let (mut store, document, view) = fixture();
    let (mut state, request) = supplied_candidate(&store, view, "$1 ${TM_FILENAME}", false);
    let clock = preparation_clock();
    let before = store.documents().snapshot(document).unwrap();
    let prepared = state
        .prepare_candidate(
            &store,
            view,
            request.token,
            0,
            preparation_options(),
            preparation_context(&clock),
        )
        .unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con"
    );
    let expected_view = store.views().get(view).unwrap().clone();
    taide_native_editor::snippet_insertion::insert(
        &mut store,
        &expected_view,
        before.revision,
        prepared.snippets,
        preparation_options().limits,
    )
    .unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "$1 ${TM_FILENAME}"
    );
    store.undo(document).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con"
    );
}

#[test]
fn completion_preparation은_다중커서의_덮어쓴선택과_클립보드행을_독립평가한다() {
    let (mut store, _, _) = fixture();
    let document = store
        .open_untitled(TabId::new(), "con\n  con", "rust".into())
        .unwrap();
    let view = attach(&mut store, document);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![
                    Selection { anchor: 3, head: 3 },
                    Selection { anchor: 9, head: 9 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let (mut state, request) = supplied_candidate(
        &store,
        view,
        "${CURSOR_INDEX}:${TM_SELECTED_TEXT}:${CLIPBOARD}",
        true,
    );
    let clock = preparation_clock();
    let prepared = state
        .prepare_candidate(
            &store,
            view,
            request.token,
            0,
            preparation_options(),
            preparation_context(&clock),
        )
        .unwrap();
    assert_eq!(prepared.snippets[0].expansion.text, "0:con:first");
    assert_eq!(prepared.snippets[1].expansion.text, "1:con:second");
    let expected_view = store.views().get(view).unwrap().clone();
    taide_native_editor::snippet_insertion::insert(
        &mut store,
        &expected_view,
        request.snapshot.revision,
        prepared.snippets,
        preparation_options().limits,
    )
    .unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "0:con:first\n  1:con:second"
    );
}

#[test]
fn completion_preparation은_오래된토큰_편집_용량과_닫힘을_거절한다() {
    let (mut store, document, view) = fixture();
    let (mut state, request) = supplied_candidate(&store, view, "${CLIPBOARD}${CLIPBOARD}", true);
    let clock = preparation_clock();
    assert!(matches!(
        state.prepare_candidate(
            &store,
            view,
            Uuid::new_v4(),
            0,
            preparation_options(),
            preparation_context(&clock)
        ),
        Err(EditorError::Refused)
    ));
    assert!(matches!(
        state.prepare_candidate(
            &store,
            view,
            request.token,
            OWNER_LIMIT,
            preparation_options(),
            preparation_context(&clock)
        ),
        Err(EditorError::InvalidBoundary)
    ));
    let mut options = preparation_options();
    options.limits.max_bytes = SMALL_PREPARATION_BYTE_LIMIT;
    assert!(matches!(
        state.prepare_candidate(
            &store,
            view,
            request.token,
            0,
            options,
            preparation_context(&clock)
        ),
        Err(EditorError::Capacity)
    ));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "con"
    );
    state.close(view);
    assert!(matches!(
        state.prepare_candidate(
            &store,
            view,
            request.token,
            0,
            preparation_options(),
            preparation_context(&clock)
        ),
        Err(EditorError::Refused)
    ));
    let (mut state, request) = supplied_candidate(&store, view, "fresh", false);
    store
        .apply(
            document,
            Transaction {
                revision: request.snapshot.revision,
                edits: vec![Edit {
                    bytes: 3..3,
                    text: "s".into(),
                }],
                selection_after: None,
                group: UndoGroup(0),
                origin: Some(view),
            },
        )
        .unwrap();
    assert!(matches!(
        state.prepare_candidate(
            &store,
            view,
            request.token,
            0,
            preparation_options(),
            preparation_context(&clock)
        ),
        Err(EditorError::Refused)
    ));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "cons"
    );
}

#[test]
fn completion_단어중간의_요청은_커서뒤_같은단어를_보존한다() {
    let (mut store, _, _) = fixture();
    let document = store
        .open_untitled(TabId::new(), "console", "rust".into())
        .unwrap();
    let view = attach(&mut store, document);
    let project = ProjectId::new();
    let mut context = context(&project, view, view);
    context.word = 0.."console".len();
    let mut state = State::default();
    let request = state.begin(&store, context, HashSet::new()).unwrap();
    assert_eq!(
        request.word,
        LspRange::new(Position::new(0, 0), Position::new(0, 3))
    );
    assert_eq!(
        request.replace_word,
        LspRange::new(Position::new(0, 0), Position::new(0, 7))
    );
}

#[test]
fn completion_요청교체는_이전_watch를_취소하고_늦은응답을_거절한다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let first = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    assert!(state.pending(&store, view));
    let next = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    assert!(first.is_cancelled());
    assert!(
        !state
            .accept(
                &store,
                &first,
                providers.clone(),
                Ok(Response {
                    groups: vec![group(&first, provider, 0, "old", false)]
                })
            )
            .unwrap()
    );
    assert!(
        state
            .accept(
                &store,
                &next,
                providers,
                Ok(Response {
                    groups: vec![group(&next, provider, 0, "current", true)]
                })
            )
            .unwrap()
    );
    assert!(!state.pending(&store, view));
    let groups = state.groups(&store, view).unwrap();
    assert_eq!(groups[&0].candidates.items[0].item.label, "current");
    assert!(groups[&0].candidates.is_incomplete);
    state.close(view);
    assert!(next.is_cancelled());
    assert!(state.groups(&store, view).is_none());
}

#[test]
fn completion_공급순서와_고유provider는_원본그룹과_문서형식을_보존한다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let first = provider();
    let second = provider();
    let foreign = provider();
    let providers = HashSet::from([first, second]);
    let mut state = State::default();
    let request = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    state
        .accept(
            &store,
            &request,
            providers,
            Ok(Response {
                groups: vec![
                    group(&request, second, 1, "second", false),
                    group(&request, foreign, 0, "foreign", false),
                    group(&request, first, 0, "first", false),
                    group(&request, first, 1, "duplicate", false),
                ],
            }),
        )
        .unwrap();
    let labels = state
        .groups(&store, view)
        .unwrap()
        .values()
        .flat_map(|group| {
            group
                .candidates
                .items
                .iter()
                .map(|item| item.item.label.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, ["first", "second"]);
}

#[test]
fn completion_편집_선택_조합_닫힘은_이전세대의_표시와_삽입을_만료시킨다() {
    let (mut store, document, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let projects = HashSet::from([project.clone()]);
    let mut state = State::default();
    let request = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    store
        .set_composition(
            view,
            Some(Composition {
                revision: 0,
                replace: 3..3,
                preedit: "한".into(),
            }),
        )
        .unwrap();
    assert!(!request.describes(&store));
    assert!(matches!(
        state.begin(&store, context(&project, view, view), providers.clone()),
        Err(EditorError::Refused)
    ));
    state.reconcile(&store, &projects, |_| true, |_, _| providers.clone());
    assert!(request.is_cancelled());
    store.set_composition(view, None).unwrap();
    let request = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 2, head: 2 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    assert!(
        !state
            .accept(
                &store,
                &request,
                providers.clone(),
                Ok(Response {
                    groups: vec![group(&request, provider, 0, "late", false)]
                })
            )
            .unwrap()
    );
    assert!(state.groups(&store, view).is_none());
    store
        .set_view_state(
            view,
            request.selection.clone(),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let request = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: 0,
                edits: vec![Edit {
                    bytes: 3..3,
                    text: "s".into(),
                }],
                group: UndoGroup(0),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    assert!(!request.describes(&store));
    state.reconcile(&store, &projects, |_| true, |_, _| providers.clone());
    assert!(request.is_cancelled());
    store.detach_view(view).unwrap();
    assert!(state.request(&store, view).is_none());
}

#[test]
fn completion_두뷰와_peek_owner의_수명은_서로_독립적이다() {
    let (mut store, document, view) = fixture();
    let project = ProjectId::new();
    let second = attach(&mut store, document);
    let peek_document = store
        .open_untitled(TabId::new(), "con", "rust".into())
        .unwrap();
    let peek = attach(&mut store, peek_document);
    let providers = HashSet::from([provider()]);
    let mut state = State::default();
    let body = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    let other = state
        .begin(&store, context(&project, second, second), providers.clone())
        .unwrap();
    let child = state
        .begin(&store, context(&project, peek, view), providers)
        .unwrap();
    state.close(second);
    assert!(other.is_cancelled());
    assert!(!body.is_cancelled());
    assert!(!child.is_cancelled());
    store.detach_view(view).unwrap();
    assert!(!child.describes(&store));
    assert!(state.request(&store, peek).is_none());
    state.clear();
    assert!(body.is_cancelled());
    assert!(child.is_cancelled());
}

#[test]
fn completion_프로젝트_provider_세대_실패와_빈결과는_오래된후보를_보존하지_않는다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let first = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    let changed = ProviderIdentity {
        generation: provider.generation + 1,
        ..provider
    };
    assert!(
        !state
            .accept(
                &store,
                &first,
                HashSet::from([changed]),
                Ok(Response { groups: Vec::new() })
            )
            .unwrap()
    );
    assert!(first.is_cancelled());
    let failed = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    assert_eq!(
        state.accept(
            &store,
            &failed,
            providers.clone(),
            Err(Failure::MalformedResponse)
        ),
        Err(Failure::MalformedResponse)
    );
    assert!(!state.pending(&store, view));
    assert!(state.groups(&store, view).unwrap().is_empty());
    let empty = state
        .begin(&store, context(&project, view, view), providers.clone())
        .unwrap();
    assert!(
        state
            .accept(
                &store,
                &empty,
                providers.clone(),
                Ok(Response { groups: Vec::new() })
            )
            .unwrap()
    );
    assert!(state.groups(&store, view).unwrap().is_empty());
    state.reconcile(&store, &HashSet::new(), |_| true, |_, _| providers.clone());
    assert!(empty.is_cancelled());
}

#[test]
fn completion_요청범위는_현재커서의_같은줄과_utf16_경계를_요구한다() {
    let (mut store, _, view) = fixture();
    let project = ProjectId::new();
    let mut state = State::default();
    let mut invalid = context(&project, view, view);
    invalid.word = 0..2;
    assert!(matches!(
        state.begin(&store, invalid, HashSet::new()),
        Err(EditorError::InvalidBoundary)
    ));
    let request = state
        .begin(&store, context(&project, view, view), HashSet::new())
        .unwrap();
    assert_eq!(request.position, Position::new(0, 3));
    assert_eq!(
        request.word,
        LspRange::new(Position::new(0, 0), Position::new(0, 3))
    );
    assert!(!state.pending(&store, view));
    assert_eq!(request.viewport, eframe::egui::ViewportId::ROOT);
    assert!(!request.automatic);
    let unicode_document = store
        .open_untitled(TabId::new(), "한\u{1f600}con", "rust".into())
        .unwrap();
    let unicode_view = attach(&mut store, unicode_document);
    store
        .set_view_state(
            unicode_view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: "한\u{1f600}con".len(),
                    head: "한\u{1f600}con".len(),
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let mut unicode = context(&project, unicode_view, unicode_view);
    unicode.word = "한\u{1f600}".len().."한\u{1f600}con".len();
    let request = state.begin(&store, unicode, HashSet::new()).unwrap();
    assert_eq!(
        request.word,
        LspRange::new(Position::new(0, 3), Position::new(0, 6))
    );
    let mut split = context(&project, unicode_view, unicode_view);
    split.word = "한".len() + 1.."한\u{1f600}con".len();
    assert!(matches!(
        state.begin(&store, split, HashSet::new()),
        Err(EditorError::InvalidBoundary)
    ));
    let multiline_document = store
        .open_untitled(TabId::new(), "con\ncon", "rust".into())
        .unwrap();
    let multiline_view = attach(&mut store, multiline_document);
    store
        .set_view_state(
            multiline_view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: "con\ncon".len(),
                    head: "con\ncon".len(),
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let mut multiline = context(&project, multiline_view, multiline_view);
    multiline.word = 0.."con\ncon".len();
    assert!(matches!(
        state.begin(&store, multiline, HashSet::new()),
        Err(EditorError::InvalidBoundary)
    ));
}

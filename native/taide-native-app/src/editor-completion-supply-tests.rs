use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde::Deserialize;
use taide_lsp::native::protocol::lsp_types::{CompletionList, CompletionResponse};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewKey};

use super::*;
use crate::editor_completion::{Context, Group, Response};
use crate::editor_symbols::ProviderIdentity;

const BYTE_LIMIT: usize = 100_000;
const OWNER_LIMIT: usize = 16;
const UNDO_LIMIT: usize = 2;
const TIMEOUT: Duration = Duration::from_secs(2);
const WORD_REFERENCE: &str = include_str!("fixtures/completion-words-reference.json");

#[derive(Deserialize)]
struct Reference {
    numeric: Vec<(String, bool)>,
    words: Vec<WordReference>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WordReference {
    language_id: String,
    leading: String,
    documents: Vec<String>,
    expected: Vec<String>,
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: OWNER_LIMIT,
        max_views: OWNER_LIMIT,
        max_undo_groups: UNDO_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

fn open(
    store: &mut EditorStore,
    text: &str,
    language: &str,
    head: Option<usize>,
) -> (DocumentId, Option<ViewId>) {
    let document = store
        .open_untitled(TabId::new(), text, language.into())
        .unwrap();
    let view = head.map(|head| {
        let view = store
            .attach_view(
                ViewKey {
                    window: "synthetic-completion".into(),
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
                    selections: vec![Selection { anchor: head, head }],
                },
                ScrollPosition::default(),
                Vec::new(),
            )
            .unwrap();
        view
    });
    (document, view)
}

fn begin(
    state: &mut State,
    store: &EditorStore,
    view: ViewId,
    word: std::ops::Range<usize>,
    providers: HashSet<ProviderIdentity>,
) -> Request {
    state
        .begin(
            store,
            Context {
                project: ProjectId::new(),
                source: view,
                owner: view,
                word,
                viewport: eframe::egui::ViewportId::ROOT,
                automatic: false,
            },
            providers,
        )
        .unwrap()
}

fn labels(candidates: &[Candidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|candidate| candidate.item.label.clone())
        .collect()
}

fn visible(state: &State, store: &EditorStore, view: ViewId) -> Vec<String> {
    state
        .candidates(store, view)
        .unwrap()
        .into_iter()
        .map(|candidate| candidate.item.label.clone())
        .collect()
}

#[test]
fn completion_숫자제외는_실제_javascript_number의_4680표본과_일치한다() {
    let reference = serde_json::from_str::<Reference>(WORD_REFERENCE).unwrap();
    for (text, expected) in reference.numeric {
        assert_eq!(is_number(&text), expected, "{text:?}");
    }
}

#[test]
fn completion_단어후보는_실제_monaco_worker의_165언어문서표본과_일치한다() {
    let reference = serde_json::from_str::<Reference>(WORD_REFERENCE).unwrap();
    for case in reference.words {
        let mut store = store();
        let mut state = State::default();
        let (_, view) = open(
            &mut store,
            &case.documents[0],
            &case.language_id,
            Some(case.leading.len()),
        );
        let view = view.unwrap();
        for text in &case.documents[1..] {
            open(&mut store, text, &case.language_id, Some(0));
        }
        let request = begin(
            &mut state,
            &store,
            view,
            0..case.leading.len(),
            HashSet::new(),
        );
        assert_eq!(
            labels(&words(
                &request,
                &word_documents(&store, &request),
                WORD_LIMIT,
                MODEL_SYNC_UTF16_LIMIT
            )),
            case.expected,
            "{} / {:?}",
            case.language_id,
            case.leading
        );
    }
}

#[test]
fn completion_단어공급은_현재문서우선과_같은언어의_열린문서만_수집한다() {
    let mut store = store();
    let (older, _) = open(&mut store, "older repeated", "rust", Some(0));
    let (current, view) = open(&mut store, "con current repeated", "rust", Some(3));
    let (later, _) = open(&mut store, "later repeated", "rust", Some(0));
    open(&mut store, "foreign", "javascript", Some(0));
    open(&mut store, "hover_helper", "rust", None);
    let mut state = State::default();
    let request = begin(&mut state, &store, view.unwrap(), 0..3, HashSet::new());
    let documents = word_documents(&store, &request);
    assert_eq!(
        documents
            .iter()
            .map(|document| document.id)
            .collect::<Vec<_>>(),
        vec![current, older, later]
    );
    assert_eq!(
        labels(&words(
            &request,
            &documents,
            WORD_LIMIT,
            MODEL_SYNC_UTF16_LIMIT
        )),
        vec!["current", "repeated", "older", "later"]
    );
}

#[test]
fn completion_단어중간은_앞부분삽입과_전체단어교체를_별도로_공급한다() {
    let mut store = store();
    let (_, view) = open(&mut store, "console concat", "rust", Some(3));
    let mut state = State::default();
    let request = begin(
        &mut state,
        &store,
        view.unwrap(),
        0.."console".len(),
        HashSet::new(),
    );
    let candidates = words(
        &request,
        &word_documents(&store, &request),
        WORD_LIMIT,
        MODEL_SYNC_UTF16_LIMIT,
    );
    assert_eq!(labels(&candidates), vec!["concat"]);
    assert_eq!(candidates[0].insert, 0..3);
    assert_eq!(candidates[0].replace, 0.."console".len());
    assert_eq!(
        candidates[0]
            .replacement_ranges(&request.snapshot, &request.selection, false)
            .unwrap(),
        vec![0..3]
    );
    assert_eq!(
        candidates[0]
            .replacement_ranges(&request.snapshot, &request.selection, true)
            .unwrap(),
        vec![0.."console".len()]
    );
}

#[test]
fn completion_단어상한과_utf16_문서상한과_취소를_수집중에_적용한다() {
    let mut store = store();
    let (_, view) = open(
        &mut store,
        "con alpha beta gamma delta",
        "plaintext",
        Some(3),
    );
    let (unicode, _) = open(&mut store, "\u{1f600} alpha", "plaintext", Some(0));
    let mut state = State::default();
    let request = begin(&mut state, &store, view.unwrap(), 0..3, HashSet::new());
    let documents = word_documents(&store, &request);
    assert_eq!(
        labels(&words(&request, &documents, 2, MODEL_SYNC_UTF16_LIMIT)),
        vec!["alpha", "beta"]
    );
    let unicode = store.documents().snapshot(unicode).unwrap();
    assert!(unicode.rope.len_bytes() > unicode.rope.len_utf16_cu());
    assert_eq!(
        labels(&words(
            &request,
            &documents,
            WORD_LIMIT,
            unicode.rope.len_utf16_cu()
        )),
        vec!["\u{1f600}", "alpha"]
    );
    state.close(request.source);
    assert!(request.is_cancelled());
    assert!(words(&request, &documents, WORD_LIMIT, MODEL_SYNC_UTF16_LIMIT).is_empty());
}

#[test]
fn completion_플러그인언어_사용자스니펫은_scope_다중prefix_설명과_본문을_보존한다() {
    let mut store = store();
    let (_, view) = open(&mut store, "con", "synthetic-plugin", Some(3));
    let mut state = State::default();
    let request = begin(&mut state, &store, view.unwrap(), 0..3, HashSet::new());
    let files = serde_json::from_str::<Vec<SnippetFile>>(r#"[
        {"fileName":"rust.json","snippets":{"Wrong":{"prefix":"rust","body":"wrong"}}},
        {"fileName":"synthetic-plugin.json","snippets":{"Plugin":{"prefix":["plug","pg",""],"body":["${1:value}","$0"],"description":["first","second"]}}},
        {"fileName":"global.code-snippets","snippets":{"Global":{"prefix":"global","body":"$1","scope":" rust , synthetic-plugin "},"Wrong":{"prefix":"wrong","body":"wrong","scope":"javascript"}}}
    ]"#).unwrap();
    let candidates = snippets(&request, &files);
    assert_eq!(labels(&candidates), vec!["plug", "pg", "global"]);
    assert_eq!(candidates[0].item.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(candidates[0].item.detail.as_deref(), Some("Plugin"));
    assert_eq!(
        candidates[0].item.documentation,
        Some(Documentation::String("first\nsecond".into()))
    );
    assert_eq!(candidates[0].text(), "${1:value}\n$0");
    assert!(candidates[0].is_snippet());
    assert_eq!(candidates[0].insert, 0..3);
    assert_eq!(candidates[0].replace, 0..3);
}

async fn supplied(state: &mut State, store: &EditorStore, request: &Request) {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            state.poll_supply(store);
            if state
                .entries
                .get(&request.source)
                .unwrap()
                .word_result
                .is_none()
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn completion_실제단어worker는_lsp와_스니펫그룹이_비었을때만_후보를_표시한다() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let mut store = store();
    let (_, view) = open(&mut store, "con console", "rust", Some(3));
    let view = view.unwrap();
    let mut state = State::default();
    let provider = ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    };
    let providers = HashSet::from([provider]);
    let request = begin(&mut state, &store, view, 0..3, providers.clone());
    let files = serde_json::from_str::<Vec<SnippetFile>>(
        r#"[{"fileName":"rust.json","snippets":{"User":{"prefix":"user","body":"$1"}}}]"#,
    )
    .unwrap();
    assert!(state.supply(&store, &request, &files, &tasks, Arc::new(|| {})));
    assert!(!state.supply(&store, &request, &files, &tasks, Arc::new(|| {})));
    runtime.block_on(supplied(&mut state, &store, &request));
    assert!(state.pending(&store, view));
    assert!(visible(&state, &store, view).is_empty());
    assert!(state.model(&store, view).unwrap().is_none());
    let candidates = super::super::Candidates::new(
        &request.snapshot,
        request.position,
        request.word,
        Some(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: vec![CompletionItem {
                label: "server".into(),
                ..Default::default()
            }],
        })),
    );
    state
        .accept(
            &store,
            &request,
            providers,
            Ok(Response {
                groups: vec![Group {
                    provider,
                    ordinal: 0,
                    candidates,
                }],
            }),
        )
        .unwrap();
    assert_eq!(visible(&state, &store, view), vec!["server", "user"]);
    assert!(!state.pending(&store, view));
    let model = state.model(&store, view).unwrap().unwrap();
    assert!(Rc::ptr_eq(
        &model,
        &state.model(&store, view).unwrap().unwrap()
    ));
    assert_eq!(model.borrow().len(), 2);
    assert!(model.borrow_mut().filter("con", 0).is_empty());
    let next = begin(&mut state, &store, view, 0..3, HashSet::from([provider]));
    state.supply(&store, &next, &[], &tasks, Arc::new(|| {}));
    runtime.block_on(supplied(&mut state, &store, &next));
    state
        .accept(
            &store,
            &next,
            HashSet::from([provider]),
            Ok(Response { groups: Vec::new() }),
        )
        .unwrap();
    assert_eq!(visible(&state, &store, view), vec!["console"]);
    let next_model = state.model(&store, view).unwrap().unwrap();
    assert!(!Rc::ptr_eq(&model, &next_model));
    let row = next_model.borrow_mut().filter("con", 0)[0].clone();
    assert_eq!(row.score.highlights(), vec![0..3]);
    assert_eq!(
        next_model
            .borrow()
            .candidate(row.candidate)
            .unwrap()
            .item
            .label,
        "console"
    );
    state.clear();
    assert!(state.model(&store, view).unwrap().is_none());
    runtime.block_on(tasks.shutdown());
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn completion_종료는_대기중인_단어worker와_늦은결과를_회수한다() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let (started, running) = std::sync::mpsc::channel();
    let (finish, blocked) = std::sync::mpsc::channel();
    let worker = tasks
        .spawn_blocking_transient_handle("synthetic-completion-blocker", move || {
            started.send(()).unwrap();
            blocked.recv().unwrap();
        })
        .unwrap();
    running.recv_timeout(TIMEOUT).unwrap();
    let mut store = store();
    let (_, view) = open(&mut store, "con console", "rust", Some(3));
    let mut state = State::default();
    let request = begin(&mut state, &store, view.unwrap(), 0..3, HashSet::new());
    let repaints = Arc::new(AtomicUsize::new(0));
    let wake = repaints.clone();
    assert!(state.supply(
        &store,
        &request,
        &[],
        &tasks,
        Arc::new(move || {
            wake.fetch_add(1, Ordering::Relaxed);
        })
    ));
    assert!(state.pending(&store, request.source));
    assert_eq!(tasks.tracked_count(), 2);
    state.clear();
    assert!(request.is_cancelled());
    finish.send(()).unwrap();
    runtime.block_on(async {
        worker.await.unwrap();
        tokio::time::timeout(TIMEOUT, async {
            while tasks.tracked_count() > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        tasks.shutdown().await;
    });
    assert!(!state.poll_supply(&store));
    assert_eq!(repaints.load(Ordering::Relaxed), 1);
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.candidates(&store, request.source).is_none());
}

use super::*;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::Selection;

const BYTE_LIMIT: usize = 4096;

#[test]
fn 문서_파일_링크는_monaco_줄열_fragment와_기본값_및_좌표_상한을_보존한다() {
    let project = ProjectId::new();
    let key = ViewKey {
        window: "synthetic".into(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    for (fragment, expected) in [
        ("", (1, 1)),
        ("73", (73, 1)),
        ("L73", (73, 1)),
        ("73,84", (73, 84)),
        ("L73,84-L83,52", (73, 84)),
        ("73-83", (73, 1)),
        ("heading", (1, 1)),
        ("0,0", (1, 1)),
    ] {
        let mut uri = url::Url::parse("file:///synthetic/file.rs").unwrap();
        uri.set_fragment(Some(fragment));
        let request = FileRequest::new(
            project.clone(),
            key.clone(),
            &uri,
            eframe::egui::ViewportId::ROOT,
        )
        .unwrap();
        assert_eq!((request.line, request.column), expected, "{fragment}");
        assert_eq!(request.path, "/synthetic/file.rs");
    }
    for target in [
        "https://example.com",
        "file:///synthetic/file.rs#4294967297",
        "file:///synthetic/file.rs#1,999999999999999999999999",
    ] {
        assert!(
            FileRequest::new(
                project.clone(),
                key.clone(),
                &url::Url::parse(target).unwrap(),
                eframe::egui::ViewportId::ROOT
            )
            .is_none()
        );
    }
}

fn fixture() -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 4,
        max_views: 4,
        max_undo_groups: 2,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_untitled(TabId::new(), "f(x, y)", "rust".into())
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
    (store, document, view)
}

fn provider() -> ProviderIdentity {
    ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    }
}

fn context(project: &ProjectId, source: ViewId, kind: Kind) -> Context {
    Context {
        project: project.clone(),
        source,
        owner: source,
        kind,
        byte: 0,
        fallback: 0..1,
        viewport: eframe::egui::ViewportId::ROOT,
        keyboard: false,
    }
}

fn hover(provider: ProviderIdentity) -> Response {
    Response::Hover {
        part: Some((
            0,
            HoverGroup {
                provider,
                part: HoverPart {
                    range: LspRange::new(Position::new(0, 0), Position::new(0, 1)),
                    contents: vec![Content::Markdown("**docs**".into())],
                },
            },
        )),
        complete: true,
    }
}

fn signature(provider: ProviderIdentity) -> Response {
    Response::Signature(Some(SignatureGroup {
        provider,
        help: lsp_types::SignatureHelp {
            active_signature: Some(0),
            active_parameter: Some(0),
            signatures: ["f(x, y)", "g(x, y)"]
                .into_iter()
                .map(|label| lsp_types::SignatureInformation {
                    label: label.into(),
                    documentation: Some(lsp_types::Documentation::String("signature docs".into())),
                    active_parameter: None,
                    parameters: Some(vec![lsp_types::ParameterInformation {
                        label: lsp_types::ParameterLabel::Simple("x".into()),
                        documentation: Some(lsp_types::Documentation::MarkupContent(
                            lsp_types::MarkupContent {
                                kind: lsp_types::MarkupKind::Markdown,
                                value: "**parameter docs**".into(),
                            },
                        )),
                    }]),
                })
                .collect(),
        },
    }))
}

#[test]
fn 호버와_서명_요청은_독립적이며_교체와_늦은_응답을_취소한다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let old = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    let signatures = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    assert!(!old.is_cancelled());
    let latest = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    assert!(old.is_cancelled());
    assert!(!signatures.is_cancelled());
    assert!(
        !state
            .accept(&store, &old, providers.clone(), Ok(hover(provider)))
            .unwrap()
    );
    assert!(
        state
            .accept(&store, &latest, providers.clone(), Ok(hover(provider)))
            .unwrap()
    );
    assert!(
        state
            .accept(&store, &signatures, providers, Ok(signature(provider)))
            .unwrap()
    );
    assert!(!state.pending(&store, view, Kind::Hover));
    assert!(
        matches!(state.payload(&store, view, Kind::Hover), Some(Payload::Hover(parts)) if parts.len() == 1)
    );
    state.close(view, Kind::Hover);
    assert!(latest.is_cancelled());
    assert!(!signatures.is_cancelled());
}

#[test]
fn 본문_편집과_선택변경은_호버와_서명_응답을_거절하고_회수한다() {
    for edit in [false, true] {
        let (mut store, document, view) = fixture();
        let project = ProjectId::new();
        let provider = provider();
        let providers = HashSet::from([provider]);
        let mut state = State::default();
        let request = state
            .begin(
                &store,
                context(&project, view, Kind::Hover),
                providers.clone(),
            )
            .unwrap();
        if edit {
            store
                .apply(
                    document,
                    Transaction {
                        revision: 0,
                        edits: vec![Edit {
                            bytes: 0..0,
                            text: "new ".into(),
                        }],
                        group: UndoGroup(0),
                        origin: Some(view),
                        selection_after: None,
                    },
                )
                .unwrap();
        } else {
            let current = store.views().get(view).unwrap().clone();
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection { anchor: 1, head: 1 }],
                    },
                    current.scroll,
                    current.folds,
                )
                .unwrap();
        }
        assert!(
            !state
                .accept(&store, &request, providers.clone(), Ok(hover(provider)))
                .unwrap()
        );
        assert!(state.payload(&store, view, Kind::Hover).is_none());
        state.reconcile(
            &store,
            &HashSet::from([project]),
            |_| true,
            |_, _, _| providers.clone(),
        );
        assert!(request.is_cancelled());
    }
}

#[test]
fn provider_세대와_등록변경은_예전_응답을_거절하고_다른_owner를_표시하지않는다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    for replacement in [
        ProviderIdentity {
            generation: 2,
            ..provider
        },
        ProviderIdentity {
            capability_revision: 1,
            ..provider
        },
    ] {
        let request = state
            .begin(
                &store,
                context(&project, view, Kind::Hover),
                providers.clone(),
            )
            .unwrap();
        assert!(
            !state
                .accept(
                    &store,
                    &request,
                    HashSet::from([replacement]),
                    Ok(hover(provider))
                )
                .unwrap()
        );
        assert!(request.is_cancelled());
    }
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    let foreign = ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        ..provider
    };
    assert!(
        state
            .accept(&store, &request, providers, Ok(hover(foreign)))
            .unwrap()
    );
    assert!(
        matches!(state.payload(&store, view, Kind::Hover), Some(Payload::Hover(parts)) if parts.is_empty())
    );
}

#[test]
fn 프로젝트_비활성_뷰와_preview의_원본문_닫힘은_요청을_회수한다() {
    let (mut store, _, view) = fixture();
    let project = ProjectId::new();
    let providers = HashSet::from([provider()]);
    let mut state = State::default();
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    state.reconcile(
        &store,
        &HashSet::from([project.clone()]),
        |_| false,
        |_, _, _| providers.clone(),
    );
    assert!(request.is_cancelled());
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    state.reconcile(
        &store,
        &HashSet::new(),
        |_| true,
        |_, _, _| providers.clone(),
    );
    assert!(request.is_cancelled());
    let preview_document = store
        .open_untitled(TabId::new(), "preview", "rust".into())
        .unwrap();
    let preview = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            preview_document,
        )
        .unwrap();
    let mut source = context(&project, preview, Kind::Hover);
    source.owner = view;
    let request = state.begin(&store, source, providers.clone()).unwrap();
    store.detach_view(view).unwrap();
    state.reconcile(
        &store,
        &HashSet::from([project]),
        |_| true,
        |_, _, _| providers.clone(),
    );
    assert!(request.is_cancelled());
    assert!(store.views().get(preview).is_some());
}

#[test]
fn 서명_순환은_문서_캐시를_보존하고_현재_서명의_인자문서를_선택한다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    state
        .accept(&store, &request, providers, Ok(signature(provider)))
        .unwrap();
    let Some(Payload::Signature(before)) = state.payload(&store, view, Kind::Signature) else {
        panic!("expected signature")
    };
    let cached = before.documentation[0].clone().unwrap();
    assert!(before.parameter_documentation().is_some());
    assert!(state.cycle(&store, view, true, true));
    let Some(Payload::Signature(after)) = state.payload(&store, view, Kind::Signature) else {
        panic!("expected signature")
    };
    assert_eq!(after.model.index(), 1);
    assert!(Arc::ptr_eq(
        after.documentation[0].as_ref().unwrap(),
        &cached
    ));
    assert!(state.cycle(&store, view, false, true));
}

#[test]
fn 비활성_서명의_문서는_이미지와_코드_공급에서_제외한다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    let Response::Signature(Some(mut group)) = signature(provider) else {
        panic!()
    };
    group.help.signatures[1].documentation = Some(lsp_types::Documentation::MarkupContent(
        lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: "![inactive](https://example.com/image.png)\n\n```rust\nfn inactive() {}\n```"
                .into(),
        },
    ));
    state
        .accept(
            &store,
            &request,
            providers,
            Ok(Response::Signature(Some(group))),
        )
        .unwrap();
    let contains_code = |state: &State| {
        state
            .entries
            .get(&(view, Kind::Signature))
            .unwrap()
            .documents()
            .iter()
            .any(|document| {
                document.blocks.iter().any(|block| {
                    matches!(
                        block,
                        taide_native_editor::documentation::Block::Code { .. }
                    )
                })
            })
    };
    assert!(!contains_code(&state));
    assert!(state.cycle(&store, view, true, true));
    assert!(contains_code(&state));
}

#[test]
fn 빈_서명과_오류는_대기를_끝내고_잘못된_문서_범위를_표시하지않는다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    assert!(state.pending(&store, view, Kind::Signature));
    state
        .accept(
            &store,
            &request,
            providers.clone(),
            Ok(Response::Signature(None)),
        )
        .unwrap();
    assert!(!state.pending(&store, view, Kind::Signature));
    assert!(state.payload(&store, view, Kind::Signature).is_none());
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    assert_eq!(
        state.accept(
            &store,
            &request,
            providers.clone(),
            Err(Failure::TransportClosed)
        ),
        Err(Failure::TransportClosed)
    );
    assert!(!state.pending(&store, view, Kind::Hover));
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    let Response::Hover {
        part: Some((ordinal, mut group)),
        ..
    } = hover(provider)
    else {
        unreachable!()
    };
    group.part.range.end = Position::new(0, 99);
    state
        .accept(
            &store,
            &request,
            providers,
            Ok(Response::Hover {
                part: Some((ordinal, group)),
                complete: true,
            }),
        )
        .unwrap();
    assert!(
        matches!(state.payload(&store, view, Kind::Hover), Some(Payload::Hover(parts)) if parts.is_empty())
    );
}

#[test]
fn 호버_부분응답은_즉시_유효하고_우선순위_정렬과_기존_문서_arc를_보존한다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let request = state
        .begin(
            &store,
            context(&project, view, Kind::Hover),
            providers.clone(),
        )
        .unwrap();
    let Response::Hover { part, .. } = hover(provider) else {
        unreachable!()
    };
    state
        .accept(
            &store,
            &request,
            providers.clone(),
            Ok(Response::Hover {
                part: part.map(|(_, part)| (1, part)),
                complete: false,
            }),
        )
        .unwrap();
    assert!(state.pending(&store, view, Kind::Hover));
    let Some(Payload::Hover(first)) = state.payload(&store, view, Kind::Hover) else {
        panic!("expected hover")
    };
    let cached = first[0].clone();
    let Response::Hover { part, .. } = hover(provider) else {
        unreachable!()
    };
    state
        .accept(
            &store,
            &request,
            providers.clone(),
            Ok(Response::Hover {
                part,
                complete: false,
            }),
        )
        .unwrap();
    let Some(Payload::Hover(second)) = state.payload(&store, view, Kind::Hover) else {
        panic!("expected hover")
    };
    assert_eq!(second.len(), 2);
    assert!(Arc::ptr_eq(&second[1], &cached));
    state
        .accept(
            &store,
            &request,
            providers,
            Ok(Response::Hover {
                part: None,
                complete: true,
            }),
        )
        .unwrap();
    assert!(!state.pending(&store, view, Kind::Hover));
}

#[test]
fn 활성_서명은_재요청_중_유지되고_빈_최신_응답에서_닫힌다() {
    let (store, _, view) = fixture();
    let project = ProjectId::new();
    let provider = provider();
    let providers = HashSet::from([provider]);
    let mut state = State::default();
    let first = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    state
        .accept(&store, &first, providers.clone(), Ok(signature(provider)))
        .unwrap();
    let second = state
        .begin(
            &store,
            context(&project, view, Kind::Signature),
            providers.clone(),
        )
        .unwrap();
    assert!(first.is_cancelled());
    assert!(state.pending(&store, view, Kind::Signature));
    assert!(state.payload(&store, view, Kind::Signature).is_some());
    state
        .accept(&store, &second, providers, Ok(Response::Signature(None)))
        .unwrap();
    assert!(state.payload(&store, view, Kind::Signature).is_none());
}

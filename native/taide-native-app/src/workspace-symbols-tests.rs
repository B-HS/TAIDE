use super::*;

fn provider(generation: u64) -> ProviderIdentity {
    ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation,
        capability_revision: 0,
    }
}

fn response(provider: ProviderIdentity, name: &str) -> Response {
    Response {
        groups: vec![Group {
            provider,
            symbols: vec![WorkspaceSymbol {
                name: name.into(),
                kind: lsp_types::SymbolKind::FUNCTION,
                container_name: "Container".into(),
                path: "/synthetic/a.rs".into(),
                line: 2,
                column: 5,
            }],
        }],
    }
}

#[test]
fn workspace_입력은_200ms_trailing과_원본_검색_좌표_순서를_보존한다() {
    let project = ProjectId::new();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let mut state = State::default();
    let now = Instant::now();
    assert!(
        state
            .observe(Some(&project), Some(" a "), providers.clone(), now)
            .is_none()
    );
    assert!(state.index(Some(&project)).is_pending);
    assert_eq!(state.delay(now), Some(SEARCH_DEBOUNCE));
    assert!(
        state
            .observe(
                Some(&project),
                Some(" a "),
                providers.clone(),
                now + SEARCH_DEBOUNCE - Duration::from_millis(1)
            )
            .is_none()
    );
    let request = state
        .observe(
            Some(&project),
            Some(" a "),
            providers.clone(),
            now + SEARCH_DEBOUNCE,
        )
        .unwrap();
    assert_eq!(
        (request.query.as_str(), request.term.as_str()),
        (" a ", "a")
    );
    assert!(
        state
            .observe(
                Some(&project),
                Some(" a "),
                providers.clone(),
                now + SEARCH_DEBOUNCE
            )
            .is_none()
    );
    let mut result = response(identity, "ServerRankedFirst");
    result
        .groups
        .extend(response(identity, "AnotherResult").groups);
    assert!(state.accept(&request, providers.clone(), Ok(result)));
    let index = state.index(Some(&project));
    assert!(!index.is_pending);
    assert_eq!(
        index
            .entries
            .unwrap()
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["ServerRankedFirst", "AnotherResult"]
    );
    assert_eq!(
        state
            .selected(&project, request.generation, 0)
            .unwrap()
            .column,
        5
    );
    assert!(!state.accept(&request, providers, Ok(Response::default())));
}

#[test]
fn workspace_취소와_프로젝트_서버_입력_교체는_이전_응답과_선택을_거절한다() {
    let project = ProjectId::new();
    let other = ProjectId::new();
    let identity = provider(1);
    let providers = HashSet::from([identity]);
    let now = Instant::now();
    let mut state = State::default();
    state.observe(Some(&project), Some("a"), providers.clone(), now);
    let first = state
        .observe(
            Some(&project),
            Some("a"),
            providers.clone(),
            now + SEARCH_DEBOUNCE,
        )
        .unwrap();
    state.observe(
        Some(&project),
        Some("ab"),
        providers.clone(),
        now + SEARCH_DEBOUNCE,
    );
    assert!(first.is_cancelled());
    assert!(!state.accept(&first, providers.clone(), Ok(response(identity, "Old"))));
    let current = state
        .observe(
            Some(&project),
            Some("ab"),
            providers.clone(),
            now + SEARCH_DEBOUNCE * 2,
        )
        .unwrap();
    let mut restarted = identity;
    restarted.generation += 1;
    state.observe(
        Some(&project),
        Some("ab"),
        HashSet::from([restarted]),
        now + SEARCH_DEBOUNCE * 2,
    );
    assert!(current.is_cancelled());
    let fresh = state
        .observe(
            Some(&project),
            Some("ab"),
            HashSet::from([restarted]),
            now + SEARCH_DEBOUNCE * 3,
        )
        .unwrap();
    assert!(state.accept(
        &fresh,
        HashSet::from([restarted]),
        Ok(response(restarted, "Fresh"))
    ));
    assert!(state.selected(&other, fresh.generation, 0).is_none());
    assert!(state.selected(&project, first.generation, 0).is_none());
    state.observe(Some(&other), Some("ab"), providers.clone(), now);
    assert!(state.selected(&project, fresh.generation, 0).is_none());
    state.observe(
        Some(&other),
        Some("\u{feff} \u{a0}"),
        providers.clone(),
        now,
    );
    assert!(!state.index(Some(&other)).is_pending);
    assert!(state.index(Some(&other)).entries.unwrap().is_empty());
    state.observe(None, Some("a"), providers, now);
    assert!(state.index(Some(&other)).query.is_none());
}

#[test]
fn workspace_정규화는_flat_nested_file과_utf16_위치를_보존하고_lazy_가상_손상을_제외한다() {
    let response: Option<lsp_types::WorkspaceSymbolResponse> = serde_json::from_value(serde_json::json!([
        {"name":"First","kind":12,"containerName":"Outer","location":{"uri":"file:///synthetic/a%20b.rs","range":{"start":{"line":4,"character":5},"end":{"line":4,"character":10}}}},
        {"name":"Lazy","kind":12,"location":{"uri":"file:///lazy.rs"}},
        {"name":"Virtual","kind":5,"location":{"uri":"jdt://virtual","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}}}},
        {"name":"Bad","kind":12,"location":{"uri":"file:///bad.rs","range":{"start":{"line":2,"character":0},"end":{"line":1,"character":0}}}},
        {"name":"Last","kind":6,"location":{"uri":"file:///last.rs","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}}}}
    ])).unwrap();
    let normalized = normalize(response);
    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0].name, "First");
    assert_eq!(normalized[0].path, "/synthetic/a b.rs");
    assert_eq!((normalized[0].line, normalized[0].column), (5, 6));
    assert_eq!(normalized[0].container_name, "Outer");
    assert_eq!(normalized[1].name, "Last");
    assert_eq!(normalized[1].kind, lsp_types::SymbolKind::METHOD);
    assert!(normalize(None).is_empty());
    let flat = serde_json::from_value(serde_json::json!([{"name":"Flat","kind":12,"location":{"uri":"file:///flat.rs","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}}}}])).unwrap();
    assert_eq!(normalize(flat)[0].name, "Flat");
}

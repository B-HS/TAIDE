use serde_json::{json, Value};
use taide_lsp_coordinator_spike::{CoordinatorProbe, Failure};

const REQUEST_TIMEOUT_MS: u64 = 500;
const BOOL_OR_OPTIONS: &[(&str, &str)] = &[
    ("textDocument/hover", "hoverProvider"),
    ("textDocument/definition", "definitionProvider"),
    ("textDocument/references", "referencesProvider"),
    ("textDocument/rename", "renameProvider"),
    ("textDocument/formatting", "documentFormattingProvider"),
    (
        "textDocument/rangeFormatting",
        "documentRangeFormattingProvider",
    ),
    ("textDocument/inlayHint", "inlayHintProvider"),
    ("textDocument/documentSymbol", "documentSymbolProvider"),
    (
        "textDocument/documentHighlight",
        "documentHighlightProvider",
    ),
    ("textDocument/selectionRange", "selectionRangeProvider"),
    ("textDocument/codeAction", "codeActionProvider"),
    ("textDocument/foldingRange", "foldingRangeProvider"),
    ("textDocument/implementation", "implementationProvider"),
    ("textDocument/typeDefinition", "typeDefinitionProvider"),
    ("textDocument/declaration", "declarationProvider"),
    ("workspace/symbol", "workspaceSymbolProvider"),
];
const OPTIONS_ONLY: &[(&str, &str)] = &[
    ("textDocument/completion", "completionProvider"),
    (
        "textDocument/onTypeFormatting",
        "documentOnTypeFormattingProvider",
    ),
    ("textDocument/signatureHelp", "signatureHelpProvider"),
    ("textDocument/diagnostic", "diagnosticProvider"),
    ("textDocument/codeLens", "codeLensProvider"),
    ("workspace/executeCommand", "executeCommandProvider"),
];

fn running(capabilities: Value) -> CoordinatorProbe {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    let init = coordinator.begin(0, json!({})).unwrap();
    coordinator
        .receive(
            0,
            1,
            json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":capabilities}}),
        )
        .unwrap();
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    coordinator
}

#[test]
fn 기존_기능_27개의_광고값이_없거나_잘못되면_pending을_만들지_않는다() {
    let mut absent = running(json!({}));
    for (method, _) in BOOL_OR_OPTIONS.iter().chain(OPTIONS_ONLY) {
        assert!(!absent.supports(method), "{method}");
        assert_eq!(
            absent.request(2, method, json!({}), None),
            Err(Failure::UnsupportedCapability)
        );
    }
    for method in [
        "textDocument/prepareRename",
        "textDocument/semanticTokens/full",
        "textDocument/semanticTokens/full/delta",
        "codeAction/resolve",
        "codeLens/resolve",
    ] {
        assert!(!absent.supports(method), "{method}");
        assert_eq!(
            absent.request(2, method, json!({}), None),
            Err(Failure::UnsupportedCapability)
        );
    }
    assert_eq!(absent.pending_count(), 0);
    for (method, field) in BOOL_OR_OPTIONS.iter().chain(OPTIONS_ONLY) {
        for value in [
            Value::Null,
            json!(false),
            json!("true"),
            json!(1),
            json!([]),
        ] {
            let mut capabilities = json!({});
            capabilities[field] = value;
            assert!(!running(capabilities).supports(method), "{method}/{field}");
        }
        let mut capabilities = json!({});
        capabilities[field] = json!({});
        assert!(running(capabilities).supports(method), "{method}/{field}");
    }
    for (method, field) in BOOL_OR_OPTIONS {
        let mut capabilities = json!({});
        capabilities[field] = json!(true);
        assert!(running(capabilities).supports(method), "{method}/{field}");
    }
    for (method, field) in OPTIONS_ONLY {
        let mut capabilities = json!({});
        capabilities[field] = json!(true);
        assert!(!running(capabilities).supports(method), "{method}/{field}");
    }
}

#[test]
fn resolve_prepare_및_semantic_delta는_세부_광고를_확인한다() {
    let mut coordinator = running(json!({
        "renameProvider":{"prepareProvider":true},
        "semanticTokensProvider":{"full":{"delta":true}},
        "codeActionProvider":{"resolveProvider":true},
        "codeLensProvider":{"resolveProvider":true}
    }));
    for method in [
        "textDocument/prepareRename",
        "textDocument/semanticTokens/full",
        "textDocument/semanticTokens/full/delta",
        "codeAction/resolve",
        "codeLens/resolve",
    ] {
        assert!(coordinator.supports(method), "{method}");
        let request = coordinator.request(2, method, json!({}), None).unwrap();
        coordinator
            .receive(
                0,
                3,
                json!({"jsonrpc":"2.0","id":request["id"],"result":null}),
            )
            .unwrap();
    }
    for value in [json!(false), Value::Null, json!("true"), json!(1)] {
        let coordinator = running(json!({
            "renameProvider":{"prepareProvider":value},
            "semanticTokensProvider":{"full":{"delta":value}},
            "codeActionProvider":{"resolveProvider":value},
            "codeLensProvider":{"resolveProvider":value}
        }));
        assert!(!coordinator.supports("textDocument/prepareRename"));
        assert!(!coordinator.supports("textDocument/semanticTokens/full/delta"));
        assert!(!coordinator.supports("codeAction/resolve"));
        assert!(!coordinator.supports("codeLens/resolve"));
    }
    let coordinator = running(json!({"semanticTokensProvider":{"full":true}}));
    assert!(coordinator.supports("textDocument/semanticTokens/full"));
    assert!(!coordinator.supports("textDocument/semanticTokens/full/delta"));
}

#[test]
fn restart는_이전_capability를_폐기하고_extension_method는_기존_정책을_유지한다() {
    let mut coordinator = running(json!({"hoverProvider":true}));
    assert!(coordinator.supports("textDocument/hover"));
    let pending = coordinator
        .request(2, "textDocument/hover", json!({}), None)
        .unwrap();
    let restarted = coordinator.restart(3, json!({})).unwrap();
    assert_eq!(restarted.completed[0].id, pending["id"].as_u64().unwrap());
    assert!(!coordinator.supports("textDocument/hover"));
    coordinator
        .receive(
            1,
            4,
            json!({"jsonrpc":"2.0","id":restarted.outgoing[0]["id"],"result":{"capabilities":{}}}),
        )
        .unwrap();
    assert!(coordinator.finish_replay(1).unwrap().is_empty());
    assert_eq!(
        coordinator.request(5, "textDocument/hover", json!({}), None),
        Err(Failure::UnsupportedCapability)
    );
    assert!(coordinator
        .request(5, "synthetic/customRequest", json!({}), None)
        .is_ok());
}

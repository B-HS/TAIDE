use serde_json::{json, Value};
use taide_lsp::native::protocol::lsp_types::{RegistrationParams, UnregistrationParams};
use taide_lsp::native::registration::MAX_DYNAMIC_REGISTRATIONS;
use taide_lsp::native::{DocumentMirror, Failure, LspCoordinator};

const REQUEST_TIMEOUT_MS: u64 = 500;
const URI: &str = "file:///synthetic/%ED%95%9C.rs";

fn running(client: Value) -> LspCoordinator {
    let mut coordinator = LspCoordinator::new(REQUEST_TIMEOUT_MS);
    let init = coordinator
        .begin(0, json!({"capabilities":client}))
        .unwrap();
    coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
    coordinator.finish_replay(0).unwrap();
    coordinator
        .open(DocumentMirror {
            uri: URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "합성".into(),
        })
        .unwrap();
    coordinator
}

fn registrations(value: Value) -> RegistrationParams {
    serde_json::from_value(json!({"registrations":value})).unwrap()
}
fn unregister(value: Value) -> UnregistrationParams {
    serde_json::from_value(json!({"unregisterations":value})).unwrap()
}

#[test]
fn dynamic_selector는_언어_scheme_glob과_unicode_문자_경계를_판정한다() {
    for (pattern, matches) in [
        ("**/?.rs", true),
        ("**/[한].{rs,ts}", true),
        ("**/[!0-9].rs", true),
        ("**/*.rs", true),
        ("**/{한,중}.rs", true),
        ("/synthetic/?.rs", true),
        ("/?.rs", false),
        ("*.rs", false),
        ("**/??.rs", false),
        ("**/[0-9].rs", false),
    ] {
        let mut coordinator =
            running(json!({"textDocument":{"hover":{"dynamicRegistration":true}}}));
        coordinator.register_capabilities(0, registrations(json!([{"id":"hover","method":"textDocument/hover","registerOptions":{"documentSelector":[{"language":"rust","scheme":"FILE","pattern":pattern}]}}]))).unwrap();
        assert!(coordinator.supports("textDocument/hover"));
        let request = coordinator.request(
            2,
            "textDocument/hover",
            json!({"textDocument":{"uri":URI}}),
            Some((URI, 0)),
        );
        assert_eq!(request.is_ok(), matches, "{pattern}");
        if !matches {
            assert_eq!(request, Err(Failure::UnsupportedCapability));
        }
    }
    for selector in [
        json!([]),
        json!([{"language":"plaintext"}]),
        json!([{"scheme":"untitled"}]),
    ] {
        let mut coordinator =
            running(json!({"textDocument":{"hover":{"dynamicRegistration":true}}}));
        coordinator.register_capabilities(0, registrations(json!([{"id":"hover","method":"textDocument/hover","registerOptions":{"documentSelector":selector}}]))).unwrap();
        assert_eq!(
            coordinator.request(2, "textDocument/hover", json!({}), Some((URI, 0))),
            Err(Failure::UnsupportedCapability)
        );
    }
    let mut coordinator = running(json!({"textDocument":{"hover":{"dynamicRegistration":true}}}));
    coordinator.register_capabilities(0, registrations(json!([{"id":"hover","method":"textDocument/hover","registerOptions":{"documentSelector":null}}]))).unwrap();
    assert!(coordinator
        .request(
            2,
            "textDocument/hover",
            json!({"textDocument":{"uri":URI}}),
            Some((URI, 0))
        )
        .is_ok());
    assert_eq!(
        coordinator.request(
            2,
            "textDocument/hover",
            json!({"textDocument":{"uri":"file:///outside.rs"}}),
            Some((URI, 0))
        ),
        Err(Failure::MalformedResponse)
    );
}

#[test]
fn dynamic_batch는_opt_in_원자성_id_method_상한과_unregistration을_지킨다() {
    let good = json!({"id":"hover","method":"textDocument/hover","registerOptions":{"documentSelector":[{"language":"rust"}]}});
    assert_eq!(
        running(json!({})).register_capabilities(0, registrations(json!([good]))),
        Err(Failure::UnsupportedCapability)
    );
    let mut coordinator = running(json!({"textDocument":{"hover":{"dynamicRegistration":true}}}));
    assert_eq!(
        coordinator.register_capabilities(0, registrations(json!([good, good]))),
        Err(Failure::MalformedResponse)
    );
    assert_eq!(coordinator.registration_count(), 0);
    assert_eq!(coordinator.capability_revision(), 0);
    for selector in [
        json!([{}]),
        json!([{"language":null}]),
        json!([{"pattern":"["}]),
    ] {
        assert!(coordinator.register_capabilities(0,registrations(json!([good,{"id":"invalid","method":"textDocument/hover","registerOptions":{"documentSelector":selector}}]))).is_err());
        assert_eq!(coordinator.registration_count(), 0);
    }
    coordinator
        .register_capabilities(0, registrations(json!([good])))
        .unwrap();
    assert_eq!(coordinator.capability_revision(), 1);
    assert_eq!(
        coordinator.unregister_capabilities(
            0,
            unregister(json!([{"id":"hover","method":"textDocument/definition"}]))
        ),
        Err(Failure::MalformedResponse)
    );
    assert_eq!(coordinator.registration_count(), 1);
    coordinator
        .unregister_capabilities(
            0,
            unregister(json!([{"id":"hover","method":"textDocument/hover"}])),
        )
        .unwrap();
    assert!(!coordinator.supports("textDocument/hover"));
    let values = (0..=MAX_DYNAMIC_REGISTRATIONS).map(|index| json!({"id":index.to_string(),"method":"textDocument/hover","registerOptions":{"documentSelector":null}})).collect::<Vec<_>>();
    assert_eq!(
        coordinator.register_capabilities(0, registrations(json!(values))),
        Err(Failure::Capacity)
    );
    assert_eq!(coordinator.registration_count(), 0);
}

#[test]
fn dynamic_옵션과_generation은_기능별_guard와_restart_폐기를_유지한다() {
    let mut coordinator = running(
        json!({"textDocument":{"rename":{"dynamicRegistration":true},"semanticTokens":{"dynamicRegistration":true}},"workspace":{"executeCommand":{"dynamicRegistration":true}}}),
    );
    coordinator.register_capabilities(0, registrations(json!([
        {"id":"rename","method":"textDocument/rename","registerOptions":{"documentSelector":null,"prepareProvider":true}},
        {"id":"semantic","method":"textDocument/semanticTokens","registerOptions":{"documentSelector":null,"legend":{"tokenTypes":[],"tokenModifiers":[]},"full":{"delta":true}}},
        {"id":"commands","method":"workspace/executeCommand","registerOptions":{"commands":["synthetic.allowed"]}}
    ]))).unwrap();
    assert!(coordinator.supports("textDocument/prepareRename"));
    assert!(coordinator.supports("textDocument/semanticTokens/full/delta"));
    assert_eq!(
        coordinator.request(
            2,
            "workspace/executeCommand",
            json!({"command":"synthetic.denied"}),
            None
        ),
        Err(Failure::UnsupportedCapability)
    );
    assert!(coordinator
        .request(
            2,
            "workspace/executeCommand",
            json!({"command":"synthetic.allowed"}),
            None
        )
        .is_ok());
    let restart = coordinator.restart(3, json!({"capabilities":{}})).unwrap();
    assert_eq!(coordinator.registration_count(), 0);
    assert_eq!(coordinator.capability_revision(), 0);
    assert_eq!(
        coordinator.register_capabilities(0, registrations(json!([]))),
        Err(Failure::StaleGeneration)
    );
    coordinator.receive(1,4,json!({"jsonrpc":"2.0","id":restart.outgoing[0]["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
    coordinator.finish_replay(1).unwrap();
    assert!(!coordinator.supports("textDocument/prepareRename"));
}

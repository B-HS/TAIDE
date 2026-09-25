use serde_json::{json, Value};
use taide_ide::protocol::{
    at_mentioned_notification, build_notification, diagnostic_json, encode, error_response, initialize_result, json_text_content,
    parse_incoming, selection_changed_notification, selection_json, success_response, text_content, tools_list_result,
    IdeSelectionSnapshot, RPC_METHOD_NOT_FOUND,
};
use taide_model::ide::{IdeDiagnostic, IdeDiagnosticSeverity};
use taide_model::ids::ProjectId;

#[test]
fn ide_mcp의_jsonrpc와_초기화_wire는_독립_crate에서_동일하다() {
    let incoming = parse_incoming(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#).unwrap();
    assert_eq!(incoming.id, Some(json!(1)));
    assert_eq!(incoming.method, "tools/list");
    assert_eq!(incoming.params, Value::Null);

    let success: Value = serde_json::from_str(&encode(&success_response(json!(1), json!({"ok": true})))).unwrap();
    assert_eq!(success, json!({"jsonrpc":"2.0","id":1,"result":{"ok":true}}));
    let error: Value = serde_json::from_str(&encode(&error_response(json!(1), RPC_METHOD_NOT_FOUND, "not found"))).unwrap();
    assert_eq!(error, json!({"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"not found"}}));

    let notification: Value = serde_json::from_str(&encode(&build_notification("at_mentioned", json!({"filePath":"/a.rs"})))).unwrap();
    assert_eq!(
        notification,
        json!({"jsonrpc":"2.0","method":"at_mentioned","params":{"filePath":"/a.rs"}})
    );
    assert_eq!(text_content("FILE_SAVED"), json!({"content":[{"type":"text","text":"FILE_SAVED"}]}));
    assert_eq!(
        json_text_content(json!({"success":true}))["content"][0]["text"],
        "{\"success\":true}"
    );

    let initialized = initialize_result(&json!({"protocolVersion":"2025-03-26"}), "TAIDE", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        initialized["serverInfo"],
        json!({"name":"TAIDE","version":env!("CARGO_PKG_VERSION")})
    );
    assert_eq!(initialized["protocolVersion"], "2025-03-26");
    let tools = tools_list_result();
    assert_eq!(tools["tools"].as_array().unwrap().len(), 12);
    assert_eq!(tools["tools"][0]["name"], "openFile");
}

#[test]
fn ide_mcp의_선택과_진단_wire는_독립_crate에서_동일하다() {
    let selection = IdeSelectionSnapshot {
        project_id: ProjectId::from("project".to_string()),
        path: "/project/a.rs".to_string(),
        text: "hello".to_string(),
        start_line: 1,
        start_character: 2,
        end_line: 1,
        end_character: 7,
        is_empty: false,
    };
    let selected: Value = serde_json::from_str(&selection_changed_notification(&selection)).unwrap();
    assert_eq!(selected["method"], "selection_changed");
    assert_eq!(selected["params"]["fileUrl"], "file:///project/a.rs");
    assert_eq!(selection_json(&selection)["selection"]["end"]["character"], 7);

    let mentioned: Value = serde_json::from_str(&at_mentioned_notification("/project/a.rs", 1, 2)).unwrap();
    assert_eq!(mentioned["params"], json!({"filePath":"/project/a.rs","lineStart":1,"lineEnd":2}));

    let diagnostic = IdeDiagnostic {
        path: "/project/a.rs".to_string(),
        severity: IdeDiagnosticSeverity::Error,
        start_line: 1,
        start_character: 2,
        end_line: 1,
        end_character: 7,
        message: "message".to_string(),
        source: Some("lsp".to_string()),
    };
    assert_eq!(
        diagnostic_json(&diagnostic),
        json!({"message":"message","severity":"Error","range":{"start":{"line":1,"character":2},"end":{"line":1,"character":7}},"source":"lsp"})
    );
}

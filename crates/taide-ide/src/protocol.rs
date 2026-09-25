use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use taide_model::ide::{IdeDiagnostic, IdeDiagnosticSeverity, IdeDiffOutcome};
use taide_model::ids::ProjectId;

const JSONRPC_VERSION: &str = "2.0";
const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

pub const RPC_METHOD_NOT_FOUND: i32 = -32601;
pub const RPC_INVALID_PARAMS: i32 = -32602;
pub const RPC_UNSUPPORTED: i32 = -32001;
pub const RPC_DIAGNOSTICS_NOT_READY: i32 = -32002;

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcIncoming {
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcErrorInfo {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: Value,
}

pub struct ToolError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdeSelectionSnapshot {
    pub project_id: ProjectId,
    pub path: String,
    pub text: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub is_empty: bool,
}

pub fn tool_error(code: i32, message: impl Into<String>) -> ToolError {
    ToolError { code, message: message.into() }
}

pub fn parse_incoming(text: &str) -> Result<JsonRpcIncoming, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn success_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse { jsonrpc: JSONRPC_VERSION, id, result: Some(result), error: None }
}

pub fn error_response(id: Value, code: i32, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse { jsonrpc: JSONRPC_VERSION, id, result: None, error: Some(JsonRpcErrorInfo { code, message: message.into() }) }
}

pub fn build_notification(method: impl Into<String>, params: Value) -> JsonRpcNotification {
    JsonRpcNotification { jsonrpc: JSONRPC_VERSION, method: method.into(), params }
}

pub fn encode<T: Serialize>(message: &T) -> String {
    serde_json::to_string(message).unwrap_or_default()
}

pub fn text_content(text: impl Into<String>) -> Value {
    json!({ "content": [ { "type": "text", "text": text.into() } ] })
}

pub fn json_text_content(value: Value) -> Value {
    text_content(value.to_string())
}

pub fn selection_changed_notification(selection: &IdeSelectionSnapshot) -> String {
    let params = json!({
        "text": selection.text,
        "filePath": selection.path,
        "fileUrl": format!("file://{}", selection.path),
        "selection": {
            "start": { "line": selection.start_line, "character": selection.start_character },
            "end": { "line": selection.end_line, "character": selection.end_character },
            "isEmpty": selection.is_empty,
        },
    });
    encode(&build_notification("selection_changed", params))
}

pub fn at_mentioned_notification(path: &str, line_start: u32, line_end: u32) -> String {
    let params = json!({ "filePath": path, "lineStart": line_start, "lineEnd": line_end });
    encode(&build_notification("at_mentioned", params))
}

const TOOL_DESCRIPTORS: &[(&str, &str)] = &[
    ("openFile", "Open a file in the editor and optionally select a range of text"),
    ("openDiff", "Open a diff view for a proposed file change (blocking until accepted, rejected, or the tab is closed)"),
    ("getCurrentSelection", "Get the current text selection in the active editor"),
    ("getLatestSelection", "Get the most recent text selection, even if not in the active editor"),
    ("getOpenEditors", "Get information about currently open editor tabs"),
    ("getWorkspaceFolders", "Get all workspace folders currently open"),
    ("getDiagnostics", "Get language diagnostics for a file, or all files if no uri is given"),
    ("checkDocumentDirty", "Check whether a document has unsaved changes"),
    ("saveDocument", "Save a document that has unsaved changes"),
    ("close_tab", "Close a tab by its display name"),
    ("closeAllDiffTabs", "Close all open diff tabs"),
    ("executeCode", "Execute code in a Jupyter kernel (not supported by TAIDE)"),
];

pub fn tools_list_result() -> Value {
    let tools: Vec<Value> = TOOL_DESCRIPTORS
        .iter()
        .map(|(name, description)| {
            json!({
                "name": name,
                "description": description,
                "inputSchema": { "type": "object", "properties": {}, "additionalProperties": true },
            })
        })
        .collect();
    json!({ "tools": tools })
}

pub fn initialize_result(params: &Value, server_name: &str, server_version: &str) -> Value {
    let protocol_version = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(MCP_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol_version,
        "serverInfo": { "name": server_name, "version": server_version },
        "capabilities": { "tools": {} },
    })
}

pub fn diff_outcome_text(outcome: IdeDiffOutcome) -> &'static str {
    match outcome {
        IdeDiffOutcome::Saved => "FILE_SAVED",
        IdeDiffOutcome::Rejected => "DIFF_REJECTED",
        IdeDiffOutcome::TabClosed => "TAB_CLOSED",
    }
}

fn diagnostic_severity_text(severity: IdeDiagnosticSeverity) -> &'static str {
    match severity {
        IdeDiagnosticSeverity::Error => "Error",
        IdeDiagnosticSeverity::Warning => "Warning",
        IdeDiagnosticSeverity::Info => "Information",
        IdeDiagnosticSeverity::Hint => "Hint",
    }
}

pub fn diagnostic_json(diagnostic: &IdeDiagnostic) -> Value {
    json!({
        "message": diagnostic.message,
        "severity": diagnostic_severity_text(diagnostic.severity),
        "range": {
            "start": { "line": diagnostic.start_line, "character": diagnostic.start_character },
            "end": { "line": diagnostic.end_line, "character": diagnostic.end_character },
        },
        "source": diagnostic.source,
    })
}

pub fn uri_to_path(uri: &str) -> String {
    uri.strip_prefix("file://").unwrap_or(uri).to_string()
}

pub fn selection_json(selection: &IdeSelectionSnapshot) -> Value {
    json!({
        "success": true,
        "text": selection.text,
        "filePath": selection.path,
        "selection": {
            "start": { "line": selection.start_line, "character": selection.start_character },
            "end": { "line": selection.end_line, "character": selection.end_character },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 요청_메시지를_파싱한다() {
        let text = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let incoming = parse_incoming(text).unwrap();
        assert_eq!(incoming.method, "tools/list");
        assert_eq!(incoming.id, Some(json!(1)));
    }

    #[test]
    fn id가_없으면_알림으로_파싱된다() {
        let text = r#"{"jsonrpc":"2.0","method":"selection_changed","params":{}}"#;
        let incoming = parse_incoming(text).unwrap();
        assert!(incoming.id.is_none());
    }

    #[test]
    fn 성공_응답은_result만_직렬화한다() {
        let response = success_response(json!(1), json!({"ok": true}));
        let value: Value = serde_json::from_str(&encode(&response)).unwrap();
        assert_eq!(value["result"]["ok"], json!(true));
        assert!(value.get("error").is_none());
    }

    #[test]
    fn 에러_응답은_error만_직렬화한다() {
        let response = error_response(json!(1), RPC_METHOD_NOT_FOUND, "not found");
        let value: Value = serde_json::from_str(&encode(&response)).unwrap();
        assert_eq!(value["error"]["code"], json!(RPC_METHOD_NOT_FOUND));
        assert!(value.get("result").is_none());
    }

    #[test]
    fn 알림은_id_필드가_없다() {
        let notification = build_notification("at_mentioned", json!({"filePath": "/a.rs"}));
        let value: Value = serde_json::from_str(&encode(&notification)).unwrap();
        assert!(value.get("id").is_none());
        assert_eq!(value["method"], json!("at_mentioned"));
    }

    #[test]
    fn mcp_텍스트_콘텐츠_포맷을_따른다() {
        let value = text_content("FILE_SAVED");
        assert_eq!(value, json!({"content": [{"type": "text", "text": "FILE_SAVED"}]}));
    }

    #[test]
    fn json_텍스트_콘텐츠는_문자열로_직렬화된_json을_담는다() {
        let value = json_text_content(json!({"success": true}));
        let text = value["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["success"], json!(true));
    }

    #[test]
    fn 잘못된_json은_파싱_에러를_반환한다() {
        assert!(parse_incoming("not json").is_err());
    }
}

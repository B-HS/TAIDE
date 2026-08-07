use std::collections::HashMap;
use std::path::Path;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;

use super::commands::{self, IdeSelectionSnapshot, IdeStore, PendingDiff, PendingSave};
use super::service;
use super::types::{IdeDiagnostic, IdeDiagnosticSeverity, IdeDiffOutcome, IDE_AUTH_HEADER_NAME, IDE_NAME, IDE_SAVE_TIMEOUT_MS};
use crate::domain::layout::commands as layout_commands;
use crate::domain::layout::service as layout_service;
use crate::domain::layout::types::{PaneNode, ProjectLayout, Tab, TabKind};
use crate::events::{IdeCloseTabRequested, IdeDiffRequested, IdeSaveRequested};
use crate::ids::ProjectId;
use crate::state::AppState;

const JSONRPC_VERSION: &str = "2.0";
const MCP_PROTOCOL_VERSION: &str = "2025-03-26";
const RPC_METHOD_NOT_FOUND: i32 = -32601;
const RPC_INVALID_PARAMS: i32 = -32602;
const RPC_UNSUPPORTED: i32 = -32001;
const RPC_DIAGNOSTICS_NOT_READY: i32 = -32002;

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

fn tool_error(code: i32, message: impl Into<String>) -> ToolError {
    ToolError {
        code,
        message: message.into(),
    }
}

pub fn parse_incoming(text: &str) -> Result<JsonRpcIncoming, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn success_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION,
        id,
        result: Some(result),
        error: None,
    }
}

pub fn error_response(id: Value, code: i32, message: impl Into<String>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION,
        id,
        result: None,
        error: Some(JsonRpcErrorInfo {
            code,
            message: message.into(),
        }),
    }
}

pub fn build_notification(method: impl Into<String>, params: Value) -> JsonRpcNotification {
    JsonRpcNotification {
        jsonrpc: JSONRPC_VERSION,
        method: method.into(),
        params,
    }
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
    (
        "openDiff",
        "Open a diff view for a proposed file change (blocking until accepted, rejected, or the tab is closed)",
    ),
    ("getCurrentSelection", "Get the current text selection in the active editor"),
    (
        "getLatestSelection",
        "Get the most recent text selection, even if not in the active editor",
    ),
    ("getOpenEditors", "Get information about currently open editor tabs"),
    ("getWorkspaceFolders", "Get all workspace folders currently open"),
    (
        "getDiagnostics",
        "Get language diagnostics for a file, or all files if no uri is given",
    ),
    ("checkDocumentDirty", "Check whether a document has unsaved changes"),
    ("saveDocument", "Save a document that has unsaved changes"),
    ("close_tab", "Close a tab by its display name"),
    ("closeAllDiffTabs", "Close all open diff tabs"),
    ("executeCode", "Execute code in a Jupyter kernel (not supported by TAIDE)"),
];

fn tools_list_result() -> Value {
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

fn initialize_result(params: &Value) -> Value {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(MCP_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol_version,
        "serverInfo": { "name": IDE_NAME, "version": env!("CARGO_PKG_VERSION") },
        "capabilities": { "tools": {} },
    })
}

fn diff_outcome_text(outcome: IdeDiffOutcome) -> &'static str {
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

fn diagnostic_json(diagnostic: &IdeDiagnostic) -> Value {
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

fn uri_to_path(uri: &str) -> String {
    uri.strip_prefix("file://").unwrap_or(uri).to_string()
}

fn selection_json(selection: &IdeSelectionSnapshot) -> Value {
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

fn find_file_tab(layouts: &HashMap<ProjectId, ProjectLayout>, path: &str) -> Option<Tab> {
    layouts
        .values()
        .flat_map(|layout| layout_service::collect_leaves(&layout.root))
        .find_map(|node| {
            let PaneNode::Leaf { tabs, .. } = node else { return None };
            tabs.iter()
                .find(|tab| matches!(&tab.kind, TabKind::File { path: p } if p == path))
                .cloned()
        })
}

/// `startText`/`endText`/`selectToEndOfLine`(텍스트 패턴으로 선택 영역을 지정하는 옵션)는
/// B2a 범위에서 구현하지 않는다 — 파일을 열고 프론트마다 유지되는 활성 selection 을 세팅하려면
/// B2b 의 에디터 인스턴스 접근이 필요하다.
async fn tool_open_file(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let Some(file_path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };
    let make_frontmost = arguments.get("makeFrontmost").and_then(Value::as_bool).unwrap_or(true);
    let preview = arguments.get("preview").and_then(Value::as_bool).unwrap_or(false);

    let state = app.state::<AppState>();
    let projects = state.projects.read().clone();
    let (project_id, resolved) = service::ensure_path_within_any_project(&projects, Path::new(file_path))
        .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;
    let path_string = resolved.to_string_lossy().to_string();
    let title = Path::new(&path_string)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path_string)
        .to_string();

    layout_commands::layout_open_tab(
        app.clone(),
        state,
        project_id,
        TabKind::File { path: path_string.clone() },
        title,
        None,
        preview,
    )
    .await
    .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;

    if make_frontmost {
        Ok(text_content(format!("Opened file: {path_string}")))
    } else {
        Ok(json_text_content(json!({
            "success": true,
            "filePath": path_string,
            "languageId": service::guess_language_id(&path_string),
        })))
    }
}

async fn tool_open_diff(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let old_path = arguments
        .get("old_file_path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(new_path_raw) = arguments.get("new_file_path").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "new_file_path is required"));
    };
    let new_contents = arguments
        .get("new_file_contents")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let tab_name = arguments
        .get("tab_name")
        .and_then(Value::as_str)
        .unwrap_or(new_path_raw)
        .to_string();

    let state = app.state::<AppState>();
    let projects = state.projects.read().clone();
    let Ok((project_id, resolved_new_path)) = service::ensure_path_within_any_project(&projects, Path::new(new_path_raw)) else {
        return Ok(text_content(diff_outcome_text(IdeDiffOutcome::Rejected)));
    };

    let request_id = uuid::Uuid::new_v4().to_string();
    let (responder, receiver) = oneshot::channel();
    app.state::<IdeStore>().insert_pending_diff(
        request_id.clone(),
        PendingDiff {
            project_id: project_id.clone(),
            new_path: resolved_new_path.clone(),
            responder,
        },
    );

    let _ = IdeDiffRequested {
        request_id,
        project_id,
        old_path,
        new_path: resolved_new_path.to_string_lossy().to_string(),
        new_contents,
        tab_name,
    }
    .emit(app);

    let (outcome, _content) = receiver.await.unwrap_or((IdeDiffOutcome::Rejected, None));
    Ok(text_content(diff_outcome_text(outcome)))
}

fn tool_get_current_selection(app: &AppHandle) -> Value {
    match app.state::<IdeStore>().current_selection() {
        Some(selection) => json_text_content(selection_json(&selection)),
        None => json_text_content(json!({ "success": false, "message": "No active editor found" })),
    }
}

fn tool_get_latest_selection(app: &AppHandle) -> Value {
    match app.state::<IdeStore>().latest_selection() {
        Some(selection) => json_text_content(selection_json(&selection)),
        None => json_text_content(json!({ "success": false, "message": "No selection available" })),
    }
}

fn tool_get_open_editors(app: &AppHandle) -> Value {
    let state = app.state::<AppState>();
    let layouts = state.layouts.read().clone();
    let tabs: Vec<Value> = service::open_editors_snapshot(&layouts)
        .into_iter()
        .map(|entry| {
            json!({
                "uri": format!("file://{}", entry.path),
                "isActive": entry.is_active,
                "label": entry.label,
                "languageId": entry.language_id,
                "isDirty": entry.is_dirty,
            })
        })
        .collect();
    json_text_content(json!({ "tabs": tabs }))
}

fn tool_get_workspace_folders(app: &AppHandle) -> Value {
    let state = app.state::<AppState>();
    let projects = state.projects.read().clone();
    let roots = service::workspace_folders(&projects);
    let folders: Vec<Value> = roots
        .iter()
        .map(|root| {
            json!({
                "name": Path::new(root).file_name().and_then(|name| name.to_str()).unwrap_or(root),
                "uri": format!("file://{root}"),
                "path": root,
            })
        })
        .collect();
    json_text_content(json!({
        "success": true,
        "folders": folders,
        "rootPath": roots.first().cloned().unwrap_or_default(),
    }))
}

fn tool_get_diagnostics(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let uri_path = arguments.get("uri").and_then(Value::as_str).map(uri_to_path);
    match app.state::<IdeStore>().diagnostics(uri_path.as_deref()) {
        Some(items) => {
            let mut by_path: HashMap<String, Vec<Value>> = HashMap::new();
            for item in &items {
                by_path.entry(item.path.clone()).or_default().push(diagnostic_json(item));
            }
            let payload: Vec<Value> = by_path
                .into_iter()
                .map(|(path, diagnostics)| json!({ "uri": format!("file://{path}"), "diagnostics": diagnostics }))
                .collect();
            Ok(json_text_content(Value::Array(payload)))
        }
        None => Err(tool_error(RPC_DIAGNOSTICS_NOT_READY, "diagnostics have not been published yet")),
    }
}

fn tool_check_document_dirty(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let Some(file_path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };
    let state = app.state::<AppState>();
    let layouts = state.layouts.read().clone();
    match find_file_tab(&layouts, file_path) {
        Some(tab) => Ok(json_text_content(json!({
            "success": true,
            "filePath": file_path,
            "isDirty": tab.dirty,
            "isUntitled": false,
        }))),
        None => Ok(json_text_content(
            json!({ "success": false, "message": format!("Document not open: {file_path}") }),
        )),
    }
}

async fn tool_save_document(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let Some(file_path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };

    let state = app.state::<AppState>();
    let projects = state.projects.read().clone();
    let Ok((project_id, resolved_path)) = service::ensure_path_within_any_project(&projects, Path::new(file_path)) else {
        return Ok(json_text_content(
            json!({ "success": false, "message": format!("Document not open: {file_path}") }),
        ));
    };
    let resolved_path_string = resolved_path.to_string_lossy().to_string();

    let layouts = state.layouts.read().clone();
    if find_file_tab(&layouts, &resolved_path_string).is_none() {
        return Ok(json_text_content(
            json!({ "success": false, "message": format!("Document not open: {file_path}") }),
        ));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let (responder, receiver) = oneshot::channel();
    app.state::<IdeStore>()
        .insert_pending_save(request_id.clone(), PendingSave { responder });

    let _ = IdeSaveRequested {
        request_id: request_id.clone(),
        project_id,
        path: resolved_path_string.clone(),
    }
    .emit(app);

    let saved = tokio::time::timeout(std::time::Duration::from_millis(IDE_SAVE_TIMEOUT_MS), receiver)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(false);

    if !saved {
        app.state::<IdeStore>().take_pending_save(&request_id);
    }

    Ok(json_text_content(json!({
        "success": saved,
        "filePath": resolved_path_string,
        "saved": saved,
        "message": if saved { "Document saved successfully" } else { "Failed to save document" },
    })))
}

fn tool_close_tab(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let Some(tab_name) = arguments.get("tab_name").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "tab_name is required"));
    };

    let state = app.state::<AppState>();
    let layouts = state.layouts.read().clone();
    let found = layouts
        .values()
        .find_map(|layout| layout_service::find_tab_by_title(&layout.root, tab_name));

    if let Some(tab_id) = found {
        if layout_commands::close_tab_and_finish(app, &state, &tab_id).is_ok() {
            let _ = IdeCloseTabRequested {
                tab_name: tab_name.to_string(),
            }
            .emit(app);
        }
    }

    Ok(text_content("TAB_CLOSED"))
}

fn tool_close_all_diff_tabs(app: &AppHandle) -> Value {
    let state = app.state::<AppState>();
    let layouts = state.layouts.read().clone();
    let mut closed = 0u32;

    for layout in layouts.values() {
        for tab_id in layout_service::collect_claude_diff_tab_ids(&layout.root) {
            if let Ok((_, closed_tab, _)) = layout_commands::close_tab_and_finish(app, &state, &tab_id) {
                closed += 1;
                let _ = IdeCloseTabRequested {
                    tab_name: closed_tab.tab.title.clone(),
                }
                .emit(app);
            }
        }
    }

    text_content(format!("CLOSED_{closed}_DIFF_TABS"))
}

pub async fn dispatch_tool_call(app: &AppHandle, name: &str, arguments: &Value) -> Result<Value, ToolError> {
    match name {
        "openFile" => tool_open_file(app, arguments).await,
        "openDiff" => tool_open_diff(app, arguments).await,
        "getCurrentSelection" => Ok(tool_get_current_selection(app)),
        "getLatestSelection" => Ok(tool_get_latest_selection(app)),
        "getOpenEditors" => Ok(tool_get_open_editors(app)),
        "getWorkspaceFolders" => Ok(tool_get_workspace_folders(app)),
        "getDiagnostics" => tool_get_diagnostics(app, arguments),
        "checkDocumentDirty" => tool_check_document_dirty(app, arguments),
        "saveDocument" => tool_save_document(app, arguments).await,
        "close_tab" => tool_close_tab(app, arguments),
        "closeAllDiffTabs" => Ok(tool_close_all_diff_tabs(app)),
        "executeCode" => Err(tool_error(RPC_UNSUPPORTED, "executeCode is not supported")),
        _ => Err(tool_error(RPC_METHOD_NOT_FOUND, format!("unknown tool: {name}"))),
    }
}

async fn handle_tools_call(app: &AppHandle, params: &Value) -> Result<Value, ToolError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| tool_error(RPC_INVALID_PARAMS, "missing tool name"))?;
    let empty = json!({});
    let arguments = params.get("arguments").unwrap_or(&empty);
    dispatch_tool_call(app, name, arguments).await
}

async fn handle_incoming(app: &AppHandle, incoming: JsonRpcIncoming) -> Option<JsonRpcResponse> {
    let id = incoming.id?;
    let response = match incoming.method.as_str() {
        "initialize" => success_response(id, initialize_result(&incoming.params)),
        "tools/list" => success_response(id, tools_list_result()),
        "tools/call" => match handle_tools_call(app, &incoming.params).await {
            Ok(value) => success_response(id, value),
            Err(error) => error_response(id, error.code, error.message),
        },
        "ping" => success_response(id, json!({})),
        _ => error_response(id, RPC_METHOD_NOT_FOUND, format!("method not found: {}", incoming.method)),
    };
    Some(response)
}

/// 헤더 콜백을 생산하는 함수. `handle_connection` 과 핸드셰이크 단위 테스트가 동일 로직을 쓰도록
/// 분리했다(테스트-운영 드리프트 방지).
/// tungstenite `Callback` 트레잇의 반환 타입(`Result<Response, ErrorResponse>`)이 고정돼 있어
/// `ErrorResponse`(= `http::Response<Option<String>>`, 136바이트) 크기를 줄일 수 없다.
#[allow(clippy::result_large_err)]
fn auth_callback(expected_token: String) -> impl FnOnce(&Request, Response) -> Result<Response, ErrorResponse> {
    move |request, response| {
        let provided = request
            .headers()
            .get(IDE_AUTH_HEADER_NAME)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        if service::constant_time_eq(provided.as_bytes(), expected_token.as_bytes()) {
            Ok(response)
        } else {
            let rejection: Result<ErrorResponse, _> = Response::builder().status(StatusCode::UNAUTHORIZED).body(None);
            Err(rejection.unwrap_or_else(|_| ErrorResponse::new(None)))
        }
    }
}

async fn handle_connection(app: AppHandle, stream: TcpStream, expected_token: String) {
    let ws_stream = match tokio_tungstenite::accept_hdr_async(stream, auth_callback(expected_token)).await {
        Ok(stream) => stream,
        Err(_) => return,
    };

    let (write_half, mut read_half) = ws_stream.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();

    let writer_handle = tauri::async_runtime::spawn(async move {
        let mut sink = write_half;
        while let Some(text) = out_rx.recv().await {
            if sink.send(Message::text(text)).await.is_err() {
                break;
            }
        }
    });

    let mut notify_rx = app.state::<IdeStore>().subscribe();
    let broadcast_out_tx = out_tx.clone();
    let forwarder_handle = tauri::async_runtime::spawn(async move {
        loop {
            match notify_rx.recv().await {
                Ok(message) => {
                    if broadcast_out_tx.send(message).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    commands::emit_status_changed(&app, app.state::<IdeStore>().client_connected());

    while let Some(message) = read_half.next().await {
        let Ok(message) = message else { break };
        match message {
            Message::Text(text) => {
                if let Ok(incoming) = parse_incoming(text.as_str()) {
                    if let Some(response) = handle_incoming(&app, incoming).await {
                        if out_tx.send(encode(&response)).is_err() {
                            break;
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    writer_handle.abort();
    forwarder_handle.abort();
    commands::emit_status_changed(&app, app.state::<IdeStore>().client_disconnected());
}

pub async fn accept_loop(app: AppHandle, listener: TcpListener, token: String) {
    loop {
        let Ok((stream, _addr)) = listener.accept().await else { break };
        let app_for_conn = app.clone();
        let token_for_conn = token.clone();
        let handle = tauri::async_runtime::spawn(async move {
            handle_connection(app_for_conn, stream, token_for_conn).await;
        });
        app.state::<IdeStore>().register_connection(handle);
    }
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

    fn handshake_request(token: Option<&str>) -> tokio_tungstenite::tungstenite::handshake::client::Request {
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let mut request = "ws://127.0.0.1/".into_client_request().unwrap();
        if let Some(token) = token {
            request.headers_mut().insert(IDE_AUTH_HEADER_NAME, token.parse().unwrap());
        }
        request
    }

    async fn spawn_test_listener(token: &str) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let expected = token.to_string();

        let handle = tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let _ = tokio_tungstenite::accept_hdr_async(stream, auth_callback(expected)).await;
            }
        });

        (addr, handle)
    }

    #[test]
    fn 올바른_토큰이면_핸드셰이크가_성공한다() {
        tauri::async_runtime::block_on(async {
            let token = "a".repeat(32);
            let (addr, _server) = spawn_test_listener(&token).await;

            let request = handshake_request(Some(&token));
            let mut request = request;
            *request.uri_mut() = format!("ws://{addr}/").parse().unwrap();

            let result = tokio_tungstenite::connect_async(request).await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn 토큰이_없으면_핸드셰이크가_거부된다() {
        tauri::async_runtime::block_on(async {
            let token = "b".repeat(32);
            let (addr, _server) = spawn_test_listener(&token).await;

            let mut request = handshake_request(None);
            *request.uri_mut() = format!("ws://{addr}/").parse().unwrap();

            let result = tokio_tungstenite::connect_async(request).await;
            assert!(result.is_err());
        });
    }

    #[test]
    fn 틀린_토큰이면_핸드셰이크가_거부된다() {
        tauri::async_runtime::block_on(async {
            let token = "c".repeat(32);
            let (addr, _server) = spawn_test_listener(&token).await;

            let mut request = handshake_request(Some("wrong-token-value"));
            *request.uri_mut() = format!("ws://{addr}/").parse().unwrap();

            let result = tokio_tungstenite::connect_async(request).await;
            assert!(result.is_err());
        });
    }
}

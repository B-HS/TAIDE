use std::collections::HashMap;
use std::path::Path;

use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use taide_ide::protocol::{
    diagnostic_json, diff_outcome_text, encode, error_response, initialize_result, json_text_content, parse_incoming, selection_json,
    success_response, text_content, tool_error, tools_list_result, uri_to_path, JsonRpcIncoming, JsonRpcResponse, ToolError,
    RPC_DIAGNOSTICS_NOT_READY, RPC_INVALID_PARAMS, RPC_METHOD_NOT_FOUND, RPC_UNSUPPORTED,
};
use taide_layout::service as layout_service;
use taide_model::app_event::AppEvent;
use taide_runtime::EventSink;
use tauri::{AppHandle, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::Message;

use super::service;
use super::store::{IdeStore, PendingDiff, PendingSave};
use super::types::{
    IdeDiffOutcome, IDE_ACCEPT_RETRY_DELAY_MS, IDE_AUTH_HEADER_NAME, IDE_DIFF_TIMEOUT_MS, IDE_HANDSHAKE_TIMEOUT_MS, IDE_NAME,
    IDE_SAVE_TIMEOUT_MS, MCP_SUBPROTOCOL,
};
use crate::domain::layout::types::{PaneNode, ProjectLayout, Tab, TabKind};
use crate::domain::project::types::Project;
use crate::error::AppResult;
use crate::ids::{ProjectId, TabId};
use crate::infra::root_guard;
use crate::platform::event_sink::TauriEventSink;
use crate::plugin_port::PluginRuntimePort;
use crate::state::AppState;

/// Layout lifecycle operations supplied by the application assembly for IDE MCP tools.
pub struct IdeLayoutActions {
    pub open_file_tab: fn(AppHandle, ProjectId, String, String, bool) -> BoxFuture<'static, AppResult<()>>,
    pub close_tab: fn(AppHandle, TabId) -> BoxFuture<'static, AppResult<Tab>>,
}

fn find_file_tab(layouts: &HashMap<ProjectId, ProjectLayout>, path: &str) -> Option<Tab> {
    layouts
        .values()
        .flat_map(layout_service::all_roots)
        .flat_map(layout_service::collect_leaves)
        .find_map(|node| {
            let PaneNode::Leaf { tabs, .. } = node else { return None };
            tabs.iter()
                .find(|tab| matches!(&tab.kind, TabKind::File { path: p } if p == path))
                .cloned()
        })
}

fn resolve_open_file_target(projects: &HashMap<ProjectId, Project>, file_path: &str) -> Result<(ProjectId, String, String), ToolError> {
    let (project_id, resolved) = service::ensure_path_within_any_project(projects, Path::new(file_path))
        .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;
    let path_string = resolved.to_string_lossy().to_string();
    root_guard::ensure_existing_file(&resolved, &path_string).map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;

    let title = Path::new(&path_string)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path_string)
        .to_string();

    Ok((project_id, path_string, title))
}

async fn tool_open_file(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let Some(file_path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };
    let make_frontmost = arguments.get("makeFrontmost").and_then(Value::as_bool).unwrap_or(true);
    let preview = arguments.get("preview").and_then(Value::as_bool).unwrap_or(false);

    let state = app.state::<AppState>();
    let projects = state.projects.read().clone();
    let (project_id, path_string, title) = resolve_open_file_target(&projects, file_path)?;

    (app.state::<IdeLayoutActions>().open_file_tab)(app.clone(), project_id, path_string.clone(), title, preview)
        .await
        .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;

    if make_frontmost {
        Ok(text_content(format!("Opened file: {path_string}")))
    } else {
        let language_overlays = (app.state::<PluginRuntimePort>().language_overlays)(app);
        Ok(json_text_content(json!({
            "success": true,
            "filePath": path_string,
            "languageId": service::guess_language_id(&path_string, &language_overlays),
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

    TauriEventSink(app).publish(AppEvent::IdeDiffRequested {
        request_id: request_id.clone(),
        project_id,
        old_path,
        new_path: resolved_new_path.to_string_lossy().to_string(),
        new_contents,
        tab_name,
    });

    let resolved = tokio::time::timeout(std::time::Duration::from_millis(IDE_DIFF_TIMEOUT_MS), receiver).await;
    let outcome = match resolved {
        Ok(received) => received.map(|(outcome, _content)| outcome).unwrap_or(IdeDiffOutcome::Rejected),
        Err(_) => {
            app.state::<IdeStore>().take_pending_diff(&request_id);
            IdeDiffOutcome::Rejected
        }
    };
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
    let language_overlays = (app.state::<PluginRuntimePort>().language_overlays)(app);
    let tabs: Vec<Value> = service::open_editors_snapshot(&layouts, &language_overlays)
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

    TauriEventSink(app).publish(AppEvent::IdeSaveRequested {
        request_id: request_id.clone(),
        project_id,
        path: resolved_path_string.clone(),
    });

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

async fn tool_close_tab(app: &AppHandle, arguments: &Value) -> Result<Value, ToolError> {
    let Some(tab_name) = arguments.get("tab_name").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "tab_name is required"));
    };

    let state = app.state::<AppState>();
    let found = {
        let layouts = state.layouts.read();
        layouts
            .values()
            .find_map(|layout| layout_service::all_roots(layout).find_map(|root| layout_service::find_tab_by_title(root, tab_name)))
    };

    if let Some(tab_id) = found {
        if let Ok(closed_tab) = (app.state::<IdeLayoutActions>().close_tab)(app.clone(), tab_id).await {
            TauriEventSink(app).publish(AppEvent::IdeCloseTabRequested {
                tab_name: tab_name.to_string(),
                request_id: layout_service::claude_diff_request_id(&closed_tab),
            });
        }
    }

    Ok(text_content("TAB_CLOSED"))
}

async fn tool_close_all_diff_tabs(app: &AppHandle) -> Value {
    let state = app.state::<AppState>();
    let layouts = state.layouts.read().clone();
    let mut closed = 0u32;

    for layout in layouts.values() {
        for tab_id in layout_service::all_roots(layout).flat_map(layout_service::collect_claude_diff_tab_ids) {
            if let Ok(closed_tab) = (app.state::<IdeLayoutActions>().close_tab)(app.clone(), tab_id).await {
                closed += 1;
                TauriEventSink(app).publish(AppEvent::IdeCloseTabRequested {
                    tab_name: closed_tab.title.clone(),
                    request_id: layout_service::claude_diff_request_id(&closed_tab),
                });
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
        "close_tab" => tool_close_tab(app, arguments).await,
        "closeAllDiffTabs" => Ok(tool_close_all_diff_tabs(app).await),
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
        "initialize" => success_response(id, initialize_result(&incoming.params, IDE_NAME, env!("CARGO_PKG_VERSION"))),
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

fn offers_mcp_subprotocol(request: &Request) -> bool {
    request
        .headers()
        .get_all(SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .any(|value| value.split(',').any(|protocol| protocol.trim() == MCP_SUBPROTOCOL))
}

#[allow(clippy::result_large_err)]
fn auth_callback(expected_token: String) -> impl FnOnce(&Request, Response) -> Result<Response, ErrorResponse> {
    move |request, mut response| {
        let provided = request
            .headers()
            .get(IDE_AUTH_HEADER_NAME)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        if !service::constant_time_eq(provided.as_bytes(), expected_token.as_bytes()) {
            let rejection: Result<ErrorResponse, _> = Response::builder().status(StatusCode::UNAUTHORIZED).body(None);
            return Err(rejection.unwrap_or_else(|_| ErrorResponse::new(None)));
        }

        if offers_mcp_subprotocol(request) {
            response
                .headers_mut()
                .insert(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static(MCP_SUBPROTOCOL));
        }

        Ok(response)
    }
}

fn emit_status_changed(app: &AppHandle, client_count: u32) {
    let ide = app.state::<IdeStore>();
    let mut status = ide.status();
    status.client_count = client_count;
    status.connected = client_count > 0;
    TauriEventSink(app).publish(AppEvent::IdeStatusChanged { status });
}

async fn handle_connection(app: AppHandle, stream: TcpStream, expected_token: String) {
    let handshake = tokio::time::timeout(
        std::time::Duration::from_millis(IDE_HANDSHAKE_TIMEOUT_MS),
        tokio_tungstenite::accept_hdr_async(stream, auth_callback(expected_token)),
    )
    .await;
    let ws_stream = match handshake {
        Ok(Ok(stream)) => {
            log::info!("IDE 핸드셰이크 성공");
            stream
        }
        Ok(Err(error)) => {
            log::warn!("IDE 핸드셰이크 거부: {error}");
            return;
        }
        Err(_) => {
            log::warn!("IDE 핸드셰이크 타임아웃({IDE_HANDSHAKE_TIMEOUT_MS}ms)");
            return;
        }
    };

    let (write_half, mut read_half) = ws_stream.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    let writer_handle = tauri::async_runtime::spawn(async move {
        let mut sink = write_half;
        while let Some(message) = out_rx.recv().await {
            if sink.send(message).await.is_err() {
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
                    if broadcast_out_tx.send(Message::text(message)).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let client_count = app.state::<IdeStore>().client_connected();
    log::info!("IDE 클라이언트 연결: count={client_count}");
    emit_status_changed(&app, client_count);

    let mut request_handles: Vec<tauri::async_runtime::JoinHandle<()>> = Vec::new();

    while let Some(message) = read_half.next().await {
        let Ok(message) = message else { break };
        match message {
            Message::Text(text) => {
                let Ok(incoming) = parse_incoming(text.as_str()) else { continue };
                let request_app = app.clone();
                let request_out_tx = out_tx.clone();
                request_handles.retain(|handle| !handle.inner().is_finished());
                request_handles.push(tauri::async_runtime::spawn(async move {
                    if let Some(response) = handle_incoming(&request_app, incoming).await {
                        let _ = request_out_tx.send(Message::text(encode(&response)));
                    }
                }));
            }
            Message::Ping(payload) => {
                let _ = out_tx.send(Message::Pong(payload));
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    for handle in &request_handles {
        handle.abort();
    }
    writer_handle.abort();
    forwarder_handle.abort();
    let client_count = app.state::<IdeStore>().client_disconnected();
    log::info!("IDE 클라이언트 연결 해제: count={client_count}");
    emit_status_changed(&app, client_count);
}

pub async fn accept_loop(app: AppHandle, listener: TcpListener, token: String) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                log::info!("IDE 연결 수락: {addr}");
                let app_for_conn = app.clone();
                let token_for_conn = token.clone();
                let handle = tauri::async_runtime::spawn(async move {
                    handle_connection(app_for_conn, stream, token_for_conn).await;
                });
                app.state::<IdeStore>().register_connection(handle);
            }
            Err(error) => {
                log::warn!("IDE accept 실패(계속): {error}");
                tokio::time::sleep(std::time::Duration::from_millis(IDE_ACCEPT_RETRY_DELAY_MS)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 존재하지_않는_경로의_open_file은_탭을_만들기_전에_거절된다() {
        let root = std::env::temp_dir().join(format!("taide-ide-open-file-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let existing = root.join("a.rs");
        std::fs::write(&existing, "fn main() {}\n").unwrap();

        let project_id = ProjectId::from("ide-open-file".to_string());
        let projects = HashMap::from([(
            project_id.clone(),
            Project {
                id: project_id,
                root: root.to_string_lossy().to_string(),
                name: "ide-open-file".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        )]);

        let Ok((_, _, title)) = resolve_open_file_target(&projects, existing.to_str().unwrap()) else {
            panic!("존재하는 파일은 통과해야 한다");
        };
        assert_eq!(title, "a.rs");

        let Err(error) = resolve_open_file_target(&projects, root.join("deleted.rs").to_str().unwrap()) else {
            panic!("사라진 경로는 open_tab_and_finish 에 닿기 전에 거절되어야 한다");
        };
        assert_eq!(error.code, RPC_INVALID_PARAMS);
        assert!(
            error.message.contains("file not found"),
            "layout_open_tab 과 같은 error.file.notFound 게이트를 타야 한다: {}",
            error.message
        );

        std::fs::remove_dir_all(&root).ok();
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

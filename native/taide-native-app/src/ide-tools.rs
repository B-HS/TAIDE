use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use taide_ide::protocol::{
    JsonRpcIncoming, JsonRpcResponse, RPC_DIAGNOSTICS_NOT_READY, RPC_INVALID_PARAMS,
    RPC_METHOD_NOT_FOUND, RPC_UNSUPPORTED, ToolError, diagnostic_json, diff_outcome_text,
    error_response, initialize_result, json_text_content, selection_json, success_response,
    text_content, tool_error, tools_list_result, uri_to_path,
};
use taide_ide::store::{PendingDiff, PendingSave};
use taide_ide::{lockfile::IDE_NAME, service};
use taide_layout::service as layout_service;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::ide::IdeDiffOutcome;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, ProjectLayout, Tab, TabKind};
use taide_runtime::AppServices;
use taide_runtime::layout_actions;
use tokio::sync::oneshot;

const IDE_SAVE_TIMEOUT: Duration = Duration::from_millis(5_000);
const IDE_DIFF_TIMEOUT: Duration = Duration::from_millis(600_000);

pub type OpenFileTabAction =
    fn(Arc<AppServices>, ProjectId, String, String, bool) -> BoxFuture<'static, AppResult<()>>;

pub type CloseTabAction =
    Arc<dyn Fn(Arc<AppServices>, TabId) -> BoxFuture<'static, AppResult<Tab>> + Send + Sync>;

pub struct LayoutActions {
    pub open_file_tab: OpenFileTabAction,
    pub close_tab: CloseTabAction,
}

impl LayoutActions {
    pub fn new(terminals: Arc<crate::terminal_host::Hub>) -> Self {
        Self {
            open_file_tab: |services, project, path, title, preview| {
                Box::pin(async move {
                    layout_actions::open_tab_and_finish(
                        services.events.as_ref(),
                        &services.state,
                        project,
                        TabKind::File { path },
                        title,
                        None,
                        preview,
                    )
                    .await
                    .map(|_| ())
                })
            },
            close_tab: Arc::new(move |services, tab| {
                let terminals = terminals.clone();
                Box::pin(async move {
                    layout_actions::close_tab_and_finish(
                        services.events.as_ref(),
                        &services.state,
                        &tab,
                        |tab| {
                            services.ide.reconcile_closed_tab(tab);
                            if let TabKind::Terminal { session_id, .. } = &tab.kind {
                                services.terminal.kill_session(session_id);
                                terminals.discard(session_id);
                            }
                        },
                    )
                    .await
                    .map(|(_, closed, _)| closed.tab)
                })
            }),
        }
    }
}

fn find_file_tab(layouts: &HashMap<ProjectId, ProjectLayout>, path: &str) -> Option<Tab> {
    layouts
        .values()
        .flat_map(layout_service::all_roots)
        .flat_map(layout_service::collect_leaves)
        .find_map(|node| {
            let PaneNode::Leaf { tabs, .. } = node else {
                return None;
            };
            tabs.iter()
                .find(|tab| matches!(&tab.kind, TabKind::File { path: open } if open == path))
                .cloned()
        })
}

fn language_overlays(services: &AppServices) -> Vec<taide_infra::language::LanguageOverlay> {
    let loaded =
        taide_plugin::service::ensure_loaded(&services.plugin, &services.state.paths.plugins_dir());
    taide_plugin::service::language_overlays(&loaded)
}

async fn open_file(
    services: Arc<AppServices>,
    actions: &LayoutActions,
    arguments: &Value,
) -> Result<Value, ToolError> {
    let Some(file_path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };
    let make_frontmost = arguments
        .get("makeFrontmost")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let preview = arguments
        .get("preview")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let projects = services.state.projects.read().clone();
    let (project, resolved) =
        service::ensure_path_within_any_project(&projects, Path::new(file_path))
            .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;
    let path = resolved.to_string_lossy().to_string();
    taide_infra::root_guard::ensure_existing_file(&resolved, &path)
        .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;
    let title = Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_string();
    (actions.open_file_tab)(services.clone(), project, path.clone(), title, preview)
        .await
        .map_err(|error| tool_error(RPC_INVALID_PARAMS, error.to_string()))?;
    if make_frontmost {
        return Ok(text_content(format!("Opened file: {path}")));
    }
    let overlays = language_overlays(&services);
    Ok(json_text_content(json!({
        "success": true,
        "filePath": path,
        "languageId": service::guess_language_id(&path, &overlays),
    })))
}

async fn open_diff(services: &AppServices, arguments: &Value) -> Result<Value, ToolError> {
    let old_path = arguments
        .get("old_file_path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(new_path) = arguments.get("new_file_path").and_then(Value::as_str) else {
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
        .unwrap_or(new_path)
        .to_string();
    let projects = services.state.projects.read().clone();
    let Ok((project_id, resolved)) =
        service::ensure_path_within_any_project(&projects, Path::new(new_path))
    else {
        return Ok(text_content(diff_outcome_text(IdeDiffOutcome::Rejected)));
    };
    let request_id = uuid::Uuid::new_v4().to_string();
    let (responder, receiver) = oneshot::channel();
    let _pending_owner = services.ide.insert_pending_diff_owned(
        request_id.clone(),
        PendingDiff {
            project_id: project_id.clone(),
            new_path: resolved.clone(),
            responder,
        },
    );
    services.events.publish(AppEvent::IdeDiffRequested {
        request_id: request_id.clone(),
        project_id,
        old_path,
        new_path: resolved.to_string_lossy().to_string(),
        new_contents,
        tab_name,
    });
    let outcome = match tokio::time::timeout(IDE_DIFF_TIMEOUT, receiver).await {
        Ok(received) => received
            .map(|(outcome, _)| outcome)
            .unwrap_or(IdeDiffOutcome::Rejected),
        Err(_) => {
            services.ide.take_pending_diff(&request_id);
            IdeDiffOutcome::Rejected
        }
    };
    Ok(text_content(diff_outcome_text(outcome)))
}

fn diagnostics(services: &AppServices, arguments: &Value) -> Result<Value, ToolError> {
    let uri_path = arguments
        .get("uri")
        .and_then(Value::as_str)
        .map(uri_to_path);
    let Some(items) = services.ide.diagnostics(uri_path.as_deref()) else {
        return Err(tool_error(
            RPC_DIAGNOSTICS_NOT_READY,
            "diagnostics have not been published yet",
        ));
    };
    let mut by_path: HashMap<String, Vec<Value>> = HashMap::new();
    for item in &items {
        by_path
            .entry(item.path.clone())
            .or_default()
            .push(diagnostic_json(item));
    }
    let payload = by_path
        .into_iter()
        .map(|(path, diagnostics)| json!({"uri": format!("file://{path}"), "diagnostics": diagnostics}))
        .collect();
    Ok(json_text_content(Value::Array(payload)))
}

fn check_dirty(services: &AppServices, arguments: &Value) -> Result<Value, ToolError> {
    let Some(path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };
    let layouts = services.state.layouts.read().clone();
    let Some(tab) = find_file_tab(&layouts, path) else {
        return Ok(json_text_content(
            json!({"success": false, "message": format!("Document not open: {path}")}),
        ));
    };
    Ok(json_text_content(
        json!({"success": true, "filePath": path, "isDirty": tab.dirty, "isUntitled": false}),
    ))
}

async fn save_document(services: &AppServices, arguments: &Value) -> Result<Value, ToolError> {
    let Some(path) = arguments.get("filePath").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "filePath is required"));
    };
    let projects = services.state.projects.read().clone();
    let Ok((project_id, resolved)) =
        service::ensure_path_within_any_project(&projects, Path::new(path))
    else {
        return Ok(json_text_content(
            json!({"success": false, "message": format!("Document not open: {path}")}),
        ));
    };
    let path = resolved.to_string_lossy().to_string();
    let layouts = services.state.layouts.read().clone();
    if find_file_tab(&layouts, &path).is_none() {
        let original = arguments
            .get("filePath")
            .and_then(Value::as_str)
            .unwrap_or_default();
        return Ok(json_text_content(
            json!({"success": false, "message": format!("Document not open: {original}")}),
        ));
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let (responder, receiver) = oneshot::channel();
    let _pending_owner = services
        .ide
        .insert_pending_save_owned(request_id.clone(), PendingSave { responder });
    services.events.publish(AppEvent::IdeSaveRequested {
        request_id: request_id.clone(),
        project_id,
        path: path.clone(),
    });
    let saved = tokio::time::timeout(IDE_SAVE_TIMEOUT, receiver)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or(false);
    if !saved {
        services.ide.take_pending_save(&request_id);
    }
    Ok(json_text_content(json!({
        "success": saved, "filePath": path, "saved": saved,
        "message": if saved { "Document saved successfully" } else { "Failed to save document" },
    })))
}

async fn close_tab(
    services: Arc<AppServices>,
    actions: &LayoutActions,
    arguments: &Value,
) -> Result<Value, ToolError> {
    let Some(title) = arguments.get("tab_name").and_then(Value::as_str) else {
        return Err(tool_error(RPC_INVALID_PARAMS, "tab_name is required"));
    };
    let tab = {
        let layouts = services.state.layouts.read();
        layouts.values().find_map(|layout| {
            layout_service::all_roots(layout)
                .find_map(|root| layout_service::find_tab_by_title(root, title))
        })
    };
    if let Some(tab) = tab
        && let Ok(closed) = (actions.close_tab)(services.clone(), tab).await
    {
        services.events.publish(AppEvent::IdeCloseTabRequested {
            tab_name: title.to_string(),
            request_id: layout_service::claude_diff_request_id(&closed),
        });
    }
    Ok(text_content("TAB_CLOSED"))
}

pub async fn dispatch(
    services: Arc<AppServices>,
    actions: &LayoutActions,
    name: &str,
    arguments: &Value,
) -> Result<Value, ToolError> {
    let Some(_operation) = services.tasks.begin_operation("native-ide-tool") else {
        return Err(tool_error(RPC_UNSUPPORTED, "IDE runtime is shutting down"));
    };
    if services.state.is_shutting_down() {
        return Err(tool_error(RPC_UNSUPPORTED, "IDE runtime is shutting down"));
    }
    match name {
        "openFile" => open_file(services, actions, arguments).await,
        "openDiff" => open_diff(&services, arguments).await,
        "getCurrentSelection" => Ok(match services.ide.current_selection() {
            Some(selection) => json_text_content(selection_json(&selection)),
            None => {
                json_text_content(json!({"success": false, "message": "No active editor found"}))
            }
        }),
        "getLatestSelection" => Ok(match services.ide.latest_selection() {
            Some(selection) => json_text_content(selection_json(&selection)),
            None => {
                json_text_content(json!({"success": false, "message": "No selection available"}))
            }
        }),
        "getOpenEditors" => {
            let layouts = services.state.layouts.read().clone();
            let overlays = language_overlays(&services);
            let tabs: Vec<Value> = service::open_editors_snapshot(&layouts, &overlays)
                .into_iter()
                .map(|entry| json!({
                    "uri": format!("file://{}", entry.path), "isActive": entry.is_active,
                    "label": entry.label, "languageId": entry.language_id, "isDirty": entry.is_dirty,
                }))
                .collect();
            Ok(json_text_content(json!({"tabs": tabs})))
        }
        "getWorkspaceFolders" => {
            let projects = services.state.projects.read().clone();
            let roots = service::workspace_folders(&projects);
            let folders: Vec<Value> = roots.iter().map(|root| json!({
                "name": Path::new(root).file_name().and_then(|name| name.to_str()).unwrap_or(root),
                "uri": format!("file://{root}"), "path": root,
            })).collect();
            Ok(json_text_content(
                json!({"success": true, "folders": folders, "rootPath": roots.first().cloned().unwrap_or_default()}),
            ))
        }
        "getDiagnostics" => diagnostics(&services, arguments),
        "checkDocumentDirty" => check_dirty(&services, arguments),
        "saveDocument" => save_document(&services, arguments).await,
        "close_tab" => close_tab(services, actions, arguments).await,
        "closeAllDiffTabs" => {
            let layouts = services.state.layouts.read().clone();
            let mut closed_count = 0u32;
            for layout in layouts.values() {
                for tab in layout_service::all_roots(layout)
                    .flat_map(layout_service::collect_claude_diff_tab_ids)
                {
                    if let Ok(closed) = (actions.close_tab)(services.clone(), tab).await {
                        closed_count += 1;
                        services.events.publish(AppEvent::IdeCloseTabRequested {
                            tab_name: closed.title.clone(),
                            request_id: layout_service::claude_diff_request_id(&closed),
                        });
                    }
                }
            }
            Ok(text_content(format!("CLOSED_{closed_count}_DIFF_TABS")))
        }
        "executeCode" => Err(tool_error(RPC_UNSUPPORTED, "executeCode is not supported")),
        _ => Err(tool_error(
            RPC_METHOD_NOT_FOUND,
            format!("unknown tool: {name}"),
        )),
    }
}

pub async fn handle_incoming(
    services: Arc<AppServices>,
    actions: &LayoutActions,
    incoming: JsonRpcIncoming,
    version: &str,
) -> Option<JsonRpcResponse> {
    let id = incoming.id?;
    let response = match incoming.method.as_str() {
        "initialize" => {
            success_response(id, initialize_result(&incoming.params, IDE_NAME, version))
        }
        "tools/list" => success_response(id, tools_list_result()),
        "tools/call" => {
            let result = match incoming.params.get("name").and_then(Value::as_str) {
                Some(name) => {
                    let empty = json!({});
                    let arguments = incoming.params.get("arguments").unwrap_or(&empty);
                    dispatch(services, actions, name, arguments).await
                }
                None => Err(tool_error(RPC_INVALID_PARAMS, "missing tool name")),
            };
            match result {
                Ok(value) => success_response(id, value),
                Err(error) => error_response(id, error.code, error.message),
            }
        }
        "ping" => success_response(id, json!({})),
        _ => error_response(
            id,
            RPC_METHOD_NOT_FOUND,
            format!("method not found: {}", incoming.method),
        ),
    };
    Some(response)
}

#[cfg(test)]
#[path = "ide-tools-tests.rs"]
mod tests;

use super::service;

fn workspace_folder_json(root: &str) -> serde_json::Value {
    serde_json::json!({
        "uri": service::workspace_folder_uri(root),
        "name": std::path::Path::new(root).file_name().and_then(|name| name.to_str()).unwrap_or("workspace"),
    })
}

/// Builds the workspace-folder notification with the same encoded root URIs used during initialization.
pub fn workspace_folders_notification(added: &[String], removed: &[String]) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didChangeWorkspaceFolders",
        "params": {
            "event": {
                "added": added.iter().map(|root| workspace_folder_json(root)).collect::<Vec<_>>(),
                "removed": removed.iter().map(|root| workspace_folder_json(root)).collect::<Vec<_>>(),
            }
        }
    })
    .to_string()
}

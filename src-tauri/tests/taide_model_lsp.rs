use taide_lib::domain::lsp::types::{LspCommandSpec, LspManifest, LspRootStrategy, LspSessionInfo, LspSpawnRequest};
use taide_model::lsp::{
    LspCommandSpec as ModelLspCommandSpec, LspManifest as ModelLspManifest, LspSessionInfo as ModelLspSessionInfo,
    LspSpawnRequest as ModelLspSpawnRequest,
};

#[test]
fn lsp_manifest의_기존_기본값과_command_wire를_유지한다() {
    let model: ModelLspManifest = serde_json::from_value(serde_json::json!({
        "servers": [{
            "id": "rust-analyzer",
            "name": "Rust Analyzer",
            "languageIds": ["rust"],
            "command": { "kind": "path", "bin": "rust-analyzer" },
            "install": { "strategy": "toolchain" }
        }]
    }))
    .expect("기존 LSP manifest");
    let facade: LspManifest = model;
    assert_eq!(facade.servers[0].root_strategy, LspRootStrategy::NearestMarker);
    assert!(facade.servers[0].root_markers.is_empty());

    let command: ModelLspCommandSpec = serde_json::from_value(serde_json::json!({
        "kind": "managed",
        "bin": "rust-analyzer"
    }))
    .expect("기존 LSP command");
    let facade: LspCommandSpec = command;
    assert!(facade.is_managed());
    assert!(facade.args().is_empty());
}

#[test]
fn lsp_spawn과_session의_기존_wire를_유지한다() {
    let spawn: ModelLspSpawnRequest = serde_json::from_value(serde_json::json!({
        "projectId": "prj-1",
        "serverId": "rust-analyzer",
        "root": "/repo",
        "owner": "main"
    }))
    .expect("기존 LSP spawn");
    let facade: LspSpawnRequest = spawn;
    assert_eq!(facade.server_id.as_str(), "rust-analyzer");

    let session: ModelLspSessionInfo = serde_json::from_value(serde_json::json!({
        "sessionId": "lsp-1",
        "projectId": "prj-1",
        "serverId": "rust-analyzer",
        "root": "/repo",
        "status": "running",
        "generation": 1
    }))
    .expect("기존 LSP session");
    let facade: LspSessionInfo = session;
    assert_eq!(facade.last_error, None);
    assert_eq!(facade.generation, 1);
}

use std::path::Path;
use std::time::Duration;

use taide_agent::{
    constants::{AGENT_PROTOCOL_VERSION, AGENT_PROTOCOL_VERSION_ENV_NAME, APP_VERSION_ENV_NAME},
    service,
};
use taide_ide::store::{should_wait_for_ide_ready, IdeStore};

use crate::AppState;

const IDE_READY_WAIT: Duration = Duration::from_millis(2_000);
const IDE_READY_POLL: Duration = Duration::from_millis(50);
const CLAUDE_CODE_SSE_PORT_ENV: &str = "CLAUDE_CODE_SSE_PORT";

pub fn editor_cli_path(target: &Path, executable: &Path) -> Option<String> {
    if std::fs::symlink_metadata(target).is_ok()
        && std::fs::canonicalize(target).is_ok_and(|resolved| service::is_cli_symlink_owned(&resolved))
    {
        return Some(target.to_string_lossy().into_owned());
    }
    let sidecar = service::resolve_cli_install_target(executable)?;
    sidecar.exists().then(|| sidecar.to_string_lossy().into_owned())
}

pub async fn resolve(state: &AppState, ide: &IdeStore, version: &str, editor_cli: Option<&str>) -> Vec<(String, String)> {
    let enabled = state.settings.read().ide_integration_enabled;
    let mut status = ide.status();
    let deadline = tokio::time::Instant::now() + IDE_READY_WAIT;
    while should_wait_for_ide_ready(enabled, status.running) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(IDE_READY_POLL).await;
        status = ide.status();
    }
    let mut env = Vec::new();
    if status.running {
        env.push((CLAUDE_CODE_SSE_PORT_ENV.into(), status.port.to_string()));
    }
    env.extend(service::build_editor_env_entries(editor_cli));
    env.extend([
        (AGENT_PROTOCOL_VERSION_ENV_NAME.into(), AGENT_PROTOCOL_VERSION.to_string()),
        (APP_VERSION_ENV_NAME.into(), version.into()),
    ]);
    env
}

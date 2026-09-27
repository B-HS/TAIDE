use tauri::{AppHandle, Manager};

use taide_ide::store::should_wait_for_ide_ready;

pub use taide_ide::protocol::IdeSelectionSnapshot;
pub use taide_ide::store::{IdeStore, PendingDiff, PendingSave, ShutdownState};

/// The Claude Code environment entries a newly spawned terminal should inherit — the SSE port of
/// the running IDE server, or nothing when it isn't running. Waits up to
/// [`super::types::IDE_READY_WAIT_MS`] (polling every [`super::types::IDE_READY_POLL_INTERVAL_MS`])
/// only while the integration is enabled but the server hasn't bound yet, so a terminal opened
/// right after app boot still gets the port. Registered by `lib.rs`'s assembly as the
/// `terminal::commands::PtySpawnEnvProvider` — the IDE domain owns its readiness semantics, the
/// terminal domain just injects whatever env the provider hands back (audit R8#10, T1-I §1.4).
pub async fn claude_terminal_env(app: &AppHandle) -> Vec<(String, String)> {
    use super::types::{CLAUDE_CODE_SSE_PORT_ENV, IDE_READY_POLL_INTERVAL_MS, IDE_READY_WAIT_MS};

    let ide_integration_enabled = app.state::<crate::state::AppState>().settings.read().ide_integration_enabled;
    let ide = app.state::<IdeStore>();
    let mut status = ide.status();

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(IDE_READY_WAIT_MS);
    while should_wait_for_ide_ready(ide_integration_enabled, status.running) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(IDE_READY_POLL_INTERVAL_MS)).await;
        status = ide.status();
    }

    if status.running {
        vec![(CLAUDE_CODE_SSE_PORT_ENV.to_string(), status.port.to_string())]
    } else {
        Vec::new()
    }
}

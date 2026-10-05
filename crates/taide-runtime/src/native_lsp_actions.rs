use serde_json::Value;
use taide_infra::lsp_proc::LspProcConfig;
use taide_lsp::native::session::{SessionClient, SessionOptions, SessionRunner};
use taide_lsp::native::Failure;
use taide_lsp::store::LspStore;

use crate::TaskSupervisor;

pub fn spawn_session(
    tasks: &TaskSupervisor,
    store: LspStore,
    config: LspProcConfig,
    initialize: Value,
    options: SessionOptions,
) -> Result<SessionClient, Failure> {
    let (client, runner) = SessionRunner::prepare(store, config, initialize, options)?;
    if !tasks.spawn_transient("native-lsp-session", runner.run()) {
        return Err(Failure::TransportClosed);
    }
    Ok(client)
}

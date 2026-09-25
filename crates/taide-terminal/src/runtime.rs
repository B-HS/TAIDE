use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::pty::{self, PtySession, PtySpawnConfig};
use taide_infra::terminal_scan::{OutputScanner, ScanOutcome};
use taide_model::error::AppResult;

use crate::session::TerminalSessionOutput;

/// Spawns a PTY and orders each chunk's instrumentation, replay storage, scanning, and delivery.
pub fn spawn_terminal_session<B, S, X>(
    config: PtySpawnConfig,
    output: Arc<TerminalSessionOutput>,
    on_chunk: B,
    on_scanned: S,
    on_exit: X,
) -> AppResult<PtySession>
where
    B: Fn(&[u8]) + Send + 'static,
    S: Fn(&ScanOutcome) + Send + 'static,
    X: FnOnce(Option<i32>) + Send + 'static,
{
    let scanner = Mutex::new(OutputScanner::new());

    pty::spawn(
        config,
        move |bytes| {
            on_chunk(bytes);
            output.append_and_broadcast(bytes);
            let outcome = scanner.lock().scan(bytes);
            on_scanned(&outcome);
        },
        on_exit,
    )
}

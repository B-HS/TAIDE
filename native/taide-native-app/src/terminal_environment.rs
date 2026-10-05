use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use taide_runtime::AppServices;

#[cfg(not(target_os = "windows"))]
const CLI_TARGET: &str = "/usr/local/bin/taide";
#[cfg(target_os = "windows")]
const CLI_TARGET: &str = "C:/Program Files/TAIDE/bin/taide.exe";

pub type Environment = Arc<
    dyn Fn(Arc<AppServices>) -> Pin<Box<dyn Future<Output = Vec<(String, String)>> + Send>>
        + Send
        + Sync,
>;

pub fn provider() -> Environment {
    Arc::new(|services| {
        Box::pin(async move {
            let editor = services
                .tasks
                .run_blocking_result("native-terminal-editor-env", || {
                    Ok(std::env::current_exe().ok().and_then(|executable| {
                        taide_runtime::terminal_env::editor_cli_path(
                            std::path::Path::new(CLI_TARGET),
                            &executable,
                        )
                    }))
                })
                .await
                .ok()
                .flatten();
            taide_runtime::terminal_env::resolve(
                &services.state,
                &services.ide,
                env!("CARGO_PKG_VERSION"),
                editor.as_deref(),
            )
            .await
        })
    })
}

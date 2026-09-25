use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use taide_infra::{lsp_install, lsp_proc};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::{LanguageServerSpec, LspServerId};
use taide_model::paths::AppPaths;

const LSP_SHUTDOWN_TIMEOUT_MS: u64 = 2_000;
const LSP_SHUTDOWN_POLL_INTERVAL_MS: u64 = 50;
const RESTART_BACKOFF_BASE_MS: u64 = 500;
pub const RESTART_BACKOFF_LIMIT: u32 = 3;
pub const HEALTHY_RESTART_WINDOW: Duration = Duration::from_secs(30);

fn managed_dir_for(paths: &AppPaths, server_id: &LspServerId) -> Option<PathBuf> {
    lsp_install::latest_installed_version(&paths.lsp_dir(), server_id.as_str())
        .map(|version| paths.lsp_server_version_dir(server_id.as_str(), &version))
}

fn resolved_args(spec: &LanguageServerSpec, root: &str, managed_dir: Option<&Path>) -> Vec<String> {
    let mut vars = vec![("workspaceDir", root.to_string())];
    if let Some(managed_dir) = managed_dir {
        vars.push(("serverDir", managed_dir.to_string_lossy().to_string()));
    }
    lsp_install::substitute_template_args(spec.command.args(), &vars)
}

pub fn resolve_process_config(
    paths: &AppPaths,
    spec: &LanguageServerSpec,
    root: &str,
    path_var: &OsStr,
) -> AppResult<lsp_proc::LspProcConfig> {
    let managed_dir = managed_dir_for(paths, &spec.id);
    let managed_relative_path = spec
        .install
        .download
        .as_ref()
        .and_then(|download| download.bin_path_in_archive.as_deref());
    let Some(resolved) = crate::service::resolve_spec_command(
        spec,
        Some(Path::new(root)),
        managed_dir.as_deref(),
        managed_relative_path,
        path_var,
    ) else {
        log::warn!(
            "lsp {}: executable not found (bin={}, root={root})",
            spec.id,
            spec.command.bin()
        );
        return Err(AppError::NotFound(format!(
            "language server executable not found: {}",
            spec.command.bin()
        )));
    };

    let args = resolved_args(spec, root, managed_dir.as_deref());
    if args
        .iter()
        .any(|arg| arg.contains('{') && arg.contains('}'))
    {
        return Err(AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.unresolvedArgTemplate",
            format!("{}: unresolved template left in launch args (managed directory may not be installed)", spec.id),
        )
        .with_arg("serverId", &spec.id));
    }

    Ok(lsp_proc::LspProcConfig {
        command: resolved.to_string_lossy().to_string(),
        args,
        cwd: PathBuf::from(root),
    })
}

pub fn spawn_language_server<D, X>(
    paths: &AppPaths,
    spec: &LanguageServerSpec,
    root: &str,
    on_message: D,
    on_exit: X,
) -> AppResult<Arc<lsp_proc::LspProcHandle>>
where
    D: Fn(String) + Send + 'static,
    X: FnOnce(Option<i32>, String) + Send + 'static,
{
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let config = resolve_process_config(paths, spec, root, &path_var)?;
    let command = config.command.clone();
    let args = config.args.clone();
    let handle = lsp_proc::spawn(config, on_message, on_exit)?;

    log::info!(
        "lsp {}: spawn {command} {args:?} cwd={root} pid={:?}",
        spec.id,
        handle.pid()
    );

    Ok(Arc::new(handle))
}

async fn wait_for_process_exit(proc: &lsp_proc::LspProcHandle, timeout_ms: u64) {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    while !proc.is_exited() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(tokio::time::Duration::from_millis(
            LSP_SHUTDOWN_POLL_INTERVAL_MS,
        ))
        .await;
    }
}

pub async fn shutdown_process(proc: &lsp_proc::LspProcHandle) {
    let shutdown_request =
        serde_json::json!({ "jsonrpc": "2.0", "id": "taide-shutdown", "method": "shutdown" })
            .to_string();
    let _ = proc.write_message(&shutdown_request).await;
    wait_for_process_exit(proc, LSP_SHUTDOWN_TIMEOUT_MS).await;

    let exit_notification = serde_json::json!({ "jsonrpc": "2.0", "method": "exit" }).to_string();
    let _ = proc.write_message(&exit_notification).await;
    wait_for_process_exit(proc, LSP_SHUTDOWN_TIMEOUT_MS).await;

    proc.kill();
}

pub fn restart_backoff_delay(restarts: u32) -> Option<Duration> {
    if restarts > RESTART_BACKOFF_LIMIT {
        return None;
    }
    Some(Duration::from_millis(
        RESTART_BACKOFF_BASE_MS * u64::from(restarts),
    ))
}

pub fn confirms_healthy_restart(
    entry_proc: &Mutex<Option<Arc<lsp_proc::LspProcHandle>>>,
    respawned: &Arc<lsp_proc::LspProcHandle>,
) -> bool {
    !respawned.is_exited()
        && entry_proc
            .lock()
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, respawned))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;
    use taide_infra::lsp_proc;

    use super::{confirms_healthy_restart, wait_for_process_exit, LSP_SHUTDOWN_TIMEOUT_MS};

    const FAST_EXIT_BOUND_MS: u64 = 1_000;
    const HANGING_PROCESS_WAIT_MS: u64 = 200;

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_for_process_exit는_프로세스가_먼저_종료하면_타임아웃보다_일찍_반환한다() {
        let config = lsp_proc::LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 0".to_string()],
            cwd: std::env::temp_dir(),
        };
        let proc =
            lsp_proc::spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공");

        let started = tokio::time::Instant::now();
        wait_for_process_exit(&proc, LSP_SHUTDOWN_TIMEOUT_MS).await;

        assert!(
            proc.is_exited(),
            "폴링이 반환했다면 프로세스는 이미 종료된 상태여야 한다"
        );
        assert!(
            started.elapsed() < tokio::time::Duration::from_millis(FAST_EXIT_BOUND_MS),
            "빨리 종료하는 프로세스는 2초 타임아웃을 다 기다리지 않고 일찍 반환해야 한다"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wait_for_process_exit는_계속_살아있는_프로세스에서_타임아웃까지_대기한다() {
        let config = lsp_proc::LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };
        let proc =
            lsp_proc::spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공");

        let started = tokio::time::Instant::now();
        wait_for_process_exit(&proc, HANGING_PROCESS_WAIT_MS).await;

        assert!(
            !proc.is_exited(),
            "타임아웃 안에 스스로 종료하지 않는 프로세스는 여전히 살아있어야 한다"
        );
        assert!(started.elapsed() >= tokio::time::Duration::from_millis(HANGING_PROCESS_WAIT_MS));

        proc.kill();
    }

    #[cfg(unix)]
    fn sleeping_proc() -> Arc<lsp_proc::LspProcHandle> {
        let config = lsp_proc::LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };
        Arc::new(
            lsp_proc::spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공"),
        )
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn confirms_healthy_restart는_같은_프로세스가_아직_살아있고_현재_슬롯에_설치되어_있으면_true를_반환한다(
    ) {
        let proc = sleeping_proc();
        let slot = Mutex::new(Some(proc.clone()));

        assert!(confirms_healthy_restart(&slot, &proc));

        proc.kill();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn confirms_healthy_restart는_슬롯이_다른_프로세스로_교체됐으면_false를_반환한다() {
        let respawned = sleeping_proc();
        let replaced_by_a_later_crash = sleeping_proc();
        let slot = Mutex::new(Some(replaced_by_a_later_crash.clone()));

        assert!(
            !confirms_healthy_restart(&slot, &respawned),
            "슬롯이 이미 다른(더 최근) respawn으로 교체됐다면 이 respawn의 건강 판정을 내리면 안 된다"
        );

        respawned.kill();
        replaced_by_a_later_crash.kill();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn confirms_healthy_restart는_프로세스가_이미_종료됐으면_false를_반환한다() {
        let proc = sleeping_proc();
        let slot = Mutex::new(Some(proc.clone()));
        proc.kill();

        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
        while !proc.is_exited() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        assert!(
            !confirms_healthy_restart(&slot, &proc),
            "건강 판정 창이 끝나기 전에 다시 죽었다면 restart_count를 리셋하면 안 된다"
        );
    }
}

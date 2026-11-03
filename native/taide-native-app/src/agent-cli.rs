use std::path::PathBuf;
use std::sync::Arc;

use taide_agent::service;
use taide_model::agent::CliInstallStatus;
use taide_model::error::{AppError, AppResult};
use taide_native_ui::intents::terminal_agent::CliAction;

const OSASCRIPT_PROGRAM: &str = "osascript";
const IS_SUPPORTED_PLATFORM: bool = cfg!(target_os = "macos");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Status,
    Install,
    Uninstall,
    ConnectExternalEditor,
}

impl From<CliAction> for Action {
    fn from(action: CliAction) -> Self {
        match action {
            CliAction::ConnectExternalEditor => Self::ConnectExternalEditor,
            CliAction::Install => Self::Install,
            CliAction::Uninstall => Self::Uninstall,
        }
    }
}

pub(crate) type Executable = Arc<dyn Fn() -> AppResult<PathBuf> + Send + Sync>;
pub(crate) type Osascript = Arc<dyn Fn(&str) -> AppResult<()> + Send + Sync>;

pub(crate) struct Ports {
    pub(crate) executable: Executable,
    pub(crate) link: PathBuf,
    pub(crate) osascript: Osascript,
}

impl Ports {
    pub(crate) fn system() -> Self {
        Self {
            executable: Arc::new(|| Ok(std::env::current_exe()?)),
            link: PathBuf::from(crate::agent_hooks::TAIDE_CLI_TARGET_PATH),
            osascript: Arc::new(run_osascript),
        }
    }
}

pub(crate) fn status(ports: &Ports) -> CliInstallStatus {
    taide_runtime::agent_host::cli_install_status(&ports.link)
}

fn install_target(ports: &Ports) -> AppResult<PathBuf> {
    service::resolve_cli_install_target(&(ports.executable)()?).ok_or_else(|| {
        AppError::InvalidArgument(
            "taide CLI shell command is only available in the installed TAIDE.app (not a dev build)"
                .into(),
        )
    })
}

pub(crate) fn install(ports: &Ports) -> AppResult<CliInstallStatus> {
    if !IS_SUPPORTED_PLATFORM {
        return Err(AppError::InvalidArgument(
            "taide CLI shell command install is only supported on macOS".into(),
        ));
    }
    let target = install_target(ports)?;
    let is_already_linked =
        std::fs::read_link(&ports.link).is_ok_and(|existing| existing == target);
    if !is_already_linked {
        (ports.osascript)(&service::build_cli_install_apple_script(
            &target,
            &ports.link,
        ))?;
    }
    Ok(status(ports))
}

pub(crate) fn uninstall(ports: &Ports) -> AppResult<CliInstallStatus> {
    if !IS_SUPPORTED_PLATFORM {
        return Err(AppError::InvalidArgument(
            "taide CLI shell command uninstall is only supported on macOS".into(),
        ));
    }
    install_target(ports)?;
    match std::fs::symlink_metadata(&ports.link) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(status(ports)),
        Err(error) => return Err(AppError::from(error)),
    }
    let is_owned = std::fs::read_link(&ports.link)
        .is_ok_and(|link_target| service::is_cli_symlink_owned(&link_target));
    if !is_owned {
        return Err(AppError::InvalidArgument(
            "the existing 'taide' command is not managed by TAIDE and was not removed".into(),
        ));
    }
    match std::fs::remove_file(&ports.link) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            (ports.osascript)(&service::build_cli_uninstall_apple_script(&ports.link))?;
        }
        Err(error) => return Err(AppError::from(error)),
    }
    Ok(status(ports))
}

pub(crate) fn run(ports: &Ports, action: Action) -> AppResult<CliInstallStatus> {
    match action {
        Action::Status => Ok(status(ports)),
        Action::Install => install(ports),
        Action::Uninstall => uninstall(ports),
        Action::ConnectExternalEditor => {
            let current = status(ports);
            if current.installed && !current.dangling {
                return Ok(current);
            }
            install(ports)
        }
    }
}

fn run_osascript(script: &str) -> AppResult<()> {
    let output = std::process::Command::new(OSASCRIPT_PROGRAM)
        .args(service::build_osascript_args(script))
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if service::is_osascript_user_cancelled(output.status.code(), &stderr) {
        return Ok(());
    }
    Err(AppError::Internal(format!("osascript failed: {stderr}")))
}

#[cfg(all(test, unix))]
#[path = "agent-cli-tests.rs"]
mod tests;

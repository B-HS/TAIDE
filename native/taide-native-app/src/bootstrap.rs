use std::path::{Component, PathBuf};
use std::process::Command;
use std::sync::Arc;

use taide_infra::external_url::validate_external_url;
use taide_infra::secret::SecretStoreState;
use taide_model::error::{AppError, AppResult};
use taide_model::paths::AppPaths;
use taide_runtime::{
    AppServices, AppState, EventSink, IdeSaveFile, PlatformServices, PlatformServicesState,
    RemoteDispatchLimiter, TaskSupervisor, project_actions, save_file_within_open_projects,
};

const REMOTE_CONCURRENT: usize = 128;
const SECRET_SERVICE: &str = "net.gumyo.taide.native-isolated";
const FILE_URL_SCHEME: &str = "file://";
const NOTIFICATION_SCRIPT: &str =
    "on run argv\ndisplay notification (item 2 of argv) with title (item 1 of argv)\nend run";

pub struct LaunchConfig {
    pub data_dir: PathBuf,
}

impl LaunchConfig {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> AppResult<Self> {
        let mut arguments = arguments.into_iter();
        if arguments.next().as_deref() != Some("--data-dir") {
            return Err(AppError::Forbidden(
                "native app requires --data-dir <absolute isolated directory>".into(),
            ));
        }
        let path = arguments
            .next()
            .ok_or_else(|| AppError::Forbidden("native data directory is missing".into()))?;
        if arguments.next().is_some() {
            return Err(AppError::Forbidden(
                "unexpected native launch argument".into(),
            ));
        }
        let data_dir = PathBuf::from(path);
        if !data_dir.is_absolute()
            || data_dir.parent().is_none()
            || data_dir
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(AppError::Forbidden(
                "native data directory must be absolute and must not be a filesystem root".into(),
            ));
        }
        Ok(Self { data_dir })
    }
}

pub async fn restore(
    config: LaunchConfig,
    tasks: &TaskSupervisor,
) -> AppResult<(AppState, Vec<String>)> {
    tasks
        .run_blocking_result("native-boot-restore", move || {
            std::fs::create_dir_all(&config.data_dir)?;
            let state = AppState::new(AppPaths::new(config.data_dir));
            let warnings = project_actions::restore_state(&state);
            Ok((state, warnings))
        })
        .await
}

pub fn services(
    state: AppState,
    tasks: TaskSupervisor,
    events: Arc<dyn EventSink>,
) -> Arc<AppServices> {
    let assembly = assemble(state, tasks, events);
    assembly.git_events.activate();
    assembly.services
}

pub struct Assembly {
    pub services: Arc<AppServices>,
    pub git_events: Arc<crate::event_relay::GitEvents>,
}

pub async fn connect_shell(
    state: AppState,
    tasks: TaskSupervisor,
    events: Arc<dyn EventSink>,
    repaint: Arc<dyn Fn() + Send + Sync>,
) -> AppResult<(Assembly, taide_native_ui::controller::ShellConnection)> {
    let mut assembly = assemble(state.clone(), tasks.clone(), events);
    let connection = taide_native_ui::controller::ShellController::connect(
        state,
        &tasks,
        assembly.services.events.clone(),
        repaint,
    )
    .await?;
    Arc::get_mut(&mut assembly.services)
        .ok_or_else(|| {
            AppError::Internal("native shell bootstrap services were shared early".into())
        })?
        .events = connection.events.clone();
    assembly.git_events.activate();
    Ok((assembly, connection))
}

pub fn assemble(state: AppState, tasks: TaskSupervisor, events: Arc<dyn EventSink>) -> Assembly {
    let mut services = AppServices::new(
        state,
        tasks,
        RemoteDispatchLimiter::new(REMOTE_CONCURRENT),
        PlatformServicesState::new(Arc::new(NativePlatform)),
        SecretStoreState::new(SECRET_SERVICE.into()),
        IdeSaveFile(save_file_within_open_projects),
        events.clone(),
    );
    let git_events = Arc::new(crate::event_relay::GitEvents::new(services.git.clone()));
    services.events = Arc::new(crate::event_relay::Relay::new(
        events,
        services.remote.clone(),
        git_events.clone(),
    ));
    Assembly {
        services: Arc::new(services),
        git_events,
    }
}

pub struct NativePlatform;

pub fn platform_url(url: &str) -> AppResult<String> {
    if let Some(path) = url.strip_prefix(FILE_URL_SCHEME) {
        return url::Url::from_file_path(std::path::Path::new(path))
            .map(String::from)
            .map_err(|()| AppError::InvalidArgument("native file URL must be absolute".into()));
    }
    validate_external_url(url)
}

impl PlatformServices for NativePlatform {
    fn open_path(&self, path: &std::path::Path) -> AppResult<()> {
        open(path.as_os_str(), false)
    }
    fn reveal_item_in_dir(&self, path: &std::path::Path) -> AppResult<()> {
        open(path.as_os_str(), true)
    }
    fn open_url(&self, url: &str) -> AppResult<()> {
        let validated = platform_url(url)?;
        open(std::ffi::OsStr::new(&validated), false)
    }
    fn send_notification(&self, title: &str, body: &str) -> AppResult<()> {
        if !cfg!(target_os = "macos") {
            return Err(AppError::Internal(
                "native notification platform is not connected".into(),
            ));
        }
        let status = Command::new("/usr/bin/osascript")
            .args(["-e", NOTIFICATION_SCRIPT, "--", title, body])
            .status()?;
        if !status.success() {
            return Err(AppError::Internal("native notification failed".into()));
        }
        Ok(())
    }
}

fn open(target: &std::ffi::OsStr, reveal: bool) -> AppResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(AppError::Internal(
            "native opener platform is not connected".into(),
        ));
    }
    let mut command = Command::new("/usr/bin/open");
    if reveal {
        command.arg("-R");
    }
    let status = command.arg("--").arg(target).status()?;
    if !status.success() {
        return Err(AppError::Internal("native opener failed".into()));
    }
    Ok(())
}

#[cfg(test)]
#[path = "bootstrap-shell-tests.rs"]
mod shell_tests;

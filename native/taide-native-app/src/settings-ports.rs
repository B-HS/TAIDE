use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use taide_infra::perf::PerfRegistry;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::{LspServerDetection, LspServerId};
use taide_runtime::agent_host::{self, AdminPrompt, CliCommandOutcome};
use taide_runtime::sync_actions::SyncGistPort;
use taide_runtime::sync_gist_http::SyncGistHttpPort;
use taide_runtime::{AppServices, lsp_actions};

const OSASCRIPT_PROGRAM: &str = "/usr/bin/osascript";
const OPEN_PROGRAM: &str = "/usr/bin/open";
const MACOS_NOTIFICATION_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.Notifications-Settings.extension";
const NOTIFICATION_SETTINGS_UNSUPPORTED_KEY: &str = "error.notification.settingsUnsupported";
const NOTIFICATION_SETTINGS_UNSUPPORTED: &str =
    "opening the notification settings is only supported on macOS";
const CLI_INSTALL_UNSUPPORTED: &str = "taide CLI shell command install is only supported on macOS";
const CLI_UNINSTALL_UNSUPPORTED: &str =
    "taide CLI shell command uninstall is only supported on macOS";

pub type DetectLspServers = Arc<dyn Fn(&Path) -> Vec<LspServerDetection> + Send + Sync>;

pub type ListFontFamilies = Arc<dyn Fn() -> Vec<taide_model::font::FontFamily> + Send + Sync>;

pub type GistFuture<'a, T> = Pin<Box<dyn Future<Output = AppResult<T>> + Send + 'a>>;

/// GitHub gist operations of Settings sync; tests replace them with fakes.
pub trait GistClient: Send + Sync {
    fn discover<'a>(
        &'a self,
        token: &'a str,
        preferred_id: Option<&'a str>,
    ) -> GistFuture<'a, Option<(String, String)>>;
    fn create<'a>(
        &'a self,
        token: &'a str,
        payload_json: &'a str,
    ) -> GistFuture<'a, (String, String)>;
    fn update<'a>(
        &'a self,
        token: &'a str,
        gist_id: &'a str,
        payload_json: &'a str,
    ) -> GistFuture<'a, String>;
    fn fetch<'a>(&'a self, token: &'a str, gist_id: &'a str) -> GistFuture<'a, (String, String)>;
}

impl GistClient for SyncGistHttpPort {
    fn discover<'a>(
        &'a self,
        token: &'a str,
        preferred_id: Option<&'a str>,
    ) -> GistFuture<'a, Option<(String, String)>> {
        Box::pin(self.discover_sync_gist(token, preferred_id))
    }

    fn create<'a>(
        &'a self,
        token: &'a str,
        payload_json: &'a str,
    ) -> GistFuture<'a, (String, String)> {
        Box::pin(self.create_gist(token, payload_json))
    }

    fn update<'a>(
        &'a self,
        token: &'a str,
        gist_id: &'a str,
        payload_json: &'a str,
    ) -> GistFuture<'a, String> {
        Box::pin(self.update_gist(token, gist_id, payload_json))
    }

    fn fetch<'a>(&'a self, token: &'a str, gist_id: &'a str) -> GistFuture<'a, (String, String)> {
        Box::pin(self.fetch_gist(token, gist_id))
    }
}

/// Adapts an injected [`GistClient`] to the shared sync policy.
#[derive(Clone)]
pub struct Gist(pub Arc<dyn GistClient>);

impl SyncGistPort for Gist {
    async fn discover_sync_gist(
        &self,
        token: &str,
        preferred_id: Option<&str>,
    ) -> AppResult<Option<(String, String)>> {
        self.0.discover(token, preferred_id).await
    }

    async fn create_gist(&self, token: &str, payload_json: &str) -> AppResult<(String, String)> {
        self.0.create(token, payload_json).await
    }

    async fn update_gist(
        &self,
        token: &str,
        gist_id: &str,
        payload_json: &str,
    ) -> AppResult<String> {
        self.0.update(token, gist_id, payload_json).await
    }

    async fn fetch_gist(&self, token: &str, gist_id: &str) -> AppResult<(String, String)> {
        self.0.fetch(token, gist_id).await
    }
}

pub type LspInstall = Arc<
    dyn Fn(Arc<AppServices>, LspServerId) -> Pin<Box<dyn Future<Output = AppResult<()>> + Send>>
        + Send
        + Sync,
>;

pub type CliCommand = Arc<dyn Fn() -> AppResult<CliCommandOutcome> + Send + Sync>;

pub type OpenNotificationSettings = Arc<dyn Fn() -> AppResult<()> + Send + Sync>;

/// Installs and removes the `taide` shell command.
pub struct CliPorts {
    pub install: CliCommand,
    pub uninstall: CliCommand,
}

impl Default for CliPorts {
    fn default() -> Self {
        Self {
            install: Arc::new(|| shell_command(CLI_INSTALL_UNSUPPORTED, agent_host::cli_install)),
            uninstall: Arc::new(|| {
                shell_command(CLI_UNINSTALL_UNSUPPORTED, agent_host::cli_uninstall)
            }),
        }
    }
}

type ShellCommandChange =
    fn(&Path, &Path, fn(&str) -> AppResult<AdminPrompt>) -> AppResult<CliCommandOutcome>;

fn shell_command(unsupported: &str, change: ShellCommandChange) -> AppResult<CliCommandOutcome> {
    if !cfg!(target_os = "macos") {
        return Err(AppError::InvalidArgument(unsupported.into()));
    }
    change(
        &std::env::current_exe()?,
        Path::new(crate::agent_hooks::TAIDE_CLI_TARGET_PATH),
        |script| agent_host::run_admin_osascript(Path::new(OSASCRIPT_PROGRAM), script),
    )
}

/// Opens the macOS notification settings pane through `open`; other systems are refused with the
/// original localized error.
pub fn open_notification_settings(
    is_macos: bool,
    open: impl FnOnce(&str) -> AppResult<()>,
) -> AppResult<()> {
    if !is_macos {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            NOTIFICATION_SETTINGS_UNSUPPORTED_KEY,
            NOTIFICATION_SETTINGS_UNSUPPORTED,
        ));
    }
    open(MACOS_NOTIFICATION_SETTINGS_URL)
}

fn open_system_url(url: &str) -> AppResult<()> {
    let status = std::process::Command::new(OPEN_PROGRAM)
        .arg("--")
        .arg(url)
        .status()?;
    if !status.success() {
        return Err(AppError::Internal("native opener failed".into()));
    }
    Ok(())
}

/// Host effects of the Settings sections; tests replace them with fakes.
pub struct Ports {
    pub detect_lsp_servers: DetectLspServers,
    pub lsp_install: LspInstall,
    pub(crate) pick_file: crate::file_dialogs::OpenPicker,
    pub gist: Gist,
    pub agents: Arc<crate::remote_agents::Ports>,
    pub cli: CliPorts,
    pub open_notification_settings: OpenNotificationSettings,
    pub perf: &'static PerfRegistry,
    pub font_families: ListFontFamilies,
}

impl Default for Ports {
    fn default() -> Self {
        Self {
            agents: Arc::new(crate::remote_agents::Ports::new()),
            cli: CliPorts::default(),
            open_notification_settings: Arc::new(|| {
                open_notification_settings(cfg!(target_os = "macos"), open_system_url)
            }),
            perf: taide_infra::perf::global(),
            detect_lsp_servers: Arc::new(|lsp_dir| {
                taide_lsp::service::detect_servers(
                    lsp_dir,
                    &std::env::var_os("PATH").unwrap_or_default(),
                )
            }),
            lsp_install: Arc::new(|services, server| {
                Box::pin(async move {
                    lsp_actions::lsp_install(
                        services.events.as_ref(),
                        &services.state,
                        &services.lsp_install,
                        &services.tasks,
                        server,
                    )
                    .await
                })
            }),
            pick_file: Arc::new(crate::file_dialogs::open),
            gist: Gist(Arc::new(SyncGistHttpPort::default())),
            font_families: Arc::new(taide_font::service::list_families),
        }
    }
}

#[cfg(test)]
#[path = "settings-ports-tests.rs"]
mod tests;

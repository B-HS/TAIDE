use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use taide_model::agent::{AgentHooksStatus, CliInstallStatus};
use taide_model::ai::{AiModelInfo, AiProviderId, AiTokenStatus};
use taide_model::app::PerfSnapshot;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LspInstallPhase, LspServerDetection, LspServerId};
use taide_model::notification::{NotificationDelivery, NotificationSuppressionReason};
use taide_model::plugin::LoadedPlugin;
use taide_model::remote::{RemoteLinkInfo, RemoteStatus};
use taide_model::settings::Settings;
use taide_model::sync::{SyncDownloadResult, SyncStatus};
use taide_model::theme::Theme;
use taide_model::vsix::VsixThemeCandidate;
use web_time::Instant;

use crate::settings_controls::HostedChange;

const STALE_TIME: Duration = Duration::from_secs(60);
const GC_TIME: Duration = Duration::from_secs(600);
const REMOTE_REFETCH_INTERVAL: Duration = Duration::from_secs(5);
const INSTALL_CANCELLED_KEY: &str = "error.lsp.installCancelled";
const PROVIDER_COUNT: usize = 3;
const VSIX_EVENT_LIMIT: usize = 16;
const OWNER_EVENT_LIMIT: usize = 16;
const JSON_INDENT: &[u8] = b"  ";
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// A credential typed into Settings. Its value is never formatted.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Secret(<redacted>)")
    }
}

/// The agent hook installation one Settings row reads and changes; user-level agents use an empty
/// project id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HookTarget {
    pub project: ProjectId,
    pub agent: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HookChange {
    Install,
    Uninstall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    LspServers,
    AiTokens,
    AiModels(AiProviderId),
    Plugins,
    SyncStatus,
    RemoteStatus,
    CliStatus,
    HooksStatus(HookTarget),
}

/// A host operation whose progress disables the control that started it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Busy {
    SyncConnect,
    SyncDisconnect,
    SyncUpload,
    SyncDownload,
    RemoteIssueLink,
    RemoteRevoke,
    RemotePassword,
    Settings,
    CliInstall,
    CliUninstall,
}

/// A host result addressed to the Settings owner whose action started it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnerEvent {
    SyncConflict,
    RemoteLinkIssued(String),
    PerfRead(String),
    PerfReset,
    PerfFailed,
}

/// The `taide` shell command state after an install or uninstall request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliOutcome {
    pub status: CliInstallStatus,
    pub is_cancelled: bool,
}

/// A file the Settings sections ask the host to choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickPurpose {
    PluginArchive,
    Vsix(u64),
}

#[derive(Debug)]
pub enum Action {
    LspInstall(LspServerId),
    LspCancel(LspServerId),
    CopyInstallHint(String),
    AiSetToken {
        provider: AiProviderId,
        token: Secret,
        sequence: u64,
    },
    AiClearToken(AiProviderId),
    PluginReload,
    PluginUninstall {
        id: String,
    },
    PluginInstall(PathBuf),
    VsixExtract {
        path: PathBuf,
        ticket: u64,
    },
    VsixSaveThemes {
        themes: Vec<Theme>,
        extension: String,
        ticket: u64,
    },
    VsixImportGrammars {
        path: PathBuf,
        ticket: u64,
    },
    SyncConnect(Secret),
    SyncDisconnect,
    SyncUpload,
    SyncDownload {
        force: bool,
        owner: u64,
    },
    RemoteIssueLink {
        owner: u64,
    },
    RemoteRevoke,
    RemoteSetPassword(Secret),
    RemoteClearPassword,
    UpdateSettings(HostedChange),
    CliInstall,
    CliUninstall,
    Hooks {
        target: HookTarget,
        change: HookChange,
    },
    OpenNotificationSettings,
    SendTestNotification {
        title: String,
        body: String,
        any_window_focused: bool,
    },
    PerfRead {
        ticket: u64,
    },
    PerfReset {
        ticket: u64,
    },
}

impl Action {
    fn busy(&self) -> Option<Busy> {
        match self {
            Self::SyncConnect(_) => Some(Busy::SyncConnect),
            Self::SyncDisconnect => Some(Busy::SyncDisconnect),
            Self::SyncUpload => Some(Busy::SyncUpload),
            Self::SyncDownload { .. } => Some(Busy::SyncDownload),
            Self::RemoteIssueLink { .. } => Some(Busy::RemoteIssueLink),
            Self::RemoteRevoke => Some(Busy::RemoteRevoke),
            Self::RemoteSetPassword(_) | Self::RemoteClearPassword => Some(Busy::RemotePassword),
            Self::UpdateSettings(_) => Some(Busy::Settings),
            Self::CliInstall => Some(Busy::CliInstall),
            Self::CliUninstall => Some(Busy::CliUninstall),
            _ => None,
        }
    }
}

/// The Settings switch whose label names why a test notification was suppressed.
pub const fn suppression_label(reason: NotificationSuppressionReason) -> &'static str {
    match reason {
        NotificationSuppressionReason::NotificationsDisabled => "settings.notificationsEnabled",
        NotificationSuppressionReason::CategoryDisabled => "settings.notificationsError",
        NotificationSuppressionReason::WindowFocused => "settings.notificationsOnlyWhenUnfocused",
    }
}

struct JsNumbers(serde_json::ser::PrettyFormatter<'static>);

impl serde_json::ser::Formatter for JsNumbers {
    fn begin_array<W: ?Sized + std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.0.begin_array(writer)
    }

    fn end_array<W: ?Sized + std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.0.end_array(writer)
    }

    fn begin_array_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        self.0.begin_array_value(writer, first)
    }

    fn end_array_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        self.0.end_array_value(writer)
    }

    fn begin_object<W: ?Sized + std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.0.begin_object(writer)
    }

    fn end_object<W: ?Sized + std::io::Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        self.0.end_object(writer)
    }

    fn begin_object_key<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> std::io::Result<()> {
        self.0.begin_object_key(writer, first)
    }

    fn begin_object_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        self.0.begin_object_value(writer)
    }

    fn end_object_value<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        self.0.end_object_value(writer)
    }

    fn write_f64<W: ?Sized + std::io::Write>(
        &mut self,
        writer: &mut W,
        value: f64,
    ) -> std::io::Result<()> {
        if value.fract() == 0.0 && value.abs() <= MAX_SAFE_INTEGER {
            return write!(writer, "{}", value as i64);
        }
        self.0.write_f64(writer, value)
    }
}

/// Renders a perf snapshot like `JSON.stringify(snapshot, null, 2)` in the original Settings.
pub fn perf_json(snapshot: &PerfSnapshot) -> AppResult<String> {
    let mut bytes = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(
        &mut bytes,
        JsNumbers(serde_json::ser::PrettyFormatter::with_indent(JSON_INDENT)),
    );
    serde::Serialize::serialize(snapshot, &mut serializer)
        .map_err(|error| AppError::Internal(error.to_string()))?;
    String::from_utf8(bytes).map_err(|error| AppError::Internal(error.to_string()))
}

/// Themes of one VSIX prepared by the host for the import dialog.
#[derive(Clone, Debug)]
pub struct VsixExtraction {
    pub path: PathBuf,
    pub extension: String,
    pub existing_ids: Vec<String>,
    pub candidates: Vec<VsixThemeCandidate>,
}

/// A host result addressed to the import dialog that started it.
#[derive(Debug)]
pub enum VsixEvent {
    Extracted(VsixExtraction),
    Saved,
    GrammarsImported(AppResult<()>),
}

#[derive(Debug)]
pub enum Request {
    Fetch { query: Query, generation: u64 },
    Act(Action),
}

#[derive(Debug)]
pub enum Reply {
    LspServers {
        generation: u64,
        result: AppResult<Vec<LspServerDetection>>,
    },
    AiTokens {
        generation: u64,
        result: AppResult<AiTokenStatus>,
    },
    AiModels {
        provider: AiProviderId,
        generation: u64,
        result: AppResult<Vec<AiModelInfo>>,
    },
    LspInstalled {
        server: LspServerId,
        result: AppResult<()>,
    },
    LspCancelled(AppResult<()>),
    HintCopied(AppResult<()>),
    AiTokenSaved {
        provider: AiProviderId,
        sequence: u64,
        result: AppResult<()>,
    },
    AiTokenCleared {
        provider: AiProviderId,
        result: AppResult<()>,
    },
    Plugins {
        generation: u64,
        result: AppResult<Vec<LoadedPlugin>>,
    },
    PluginsReloaded(AppResult<Vec<LoadedPlugin>>),
    PluginUninstalled(AppResult<Vec<LoadedPlugin>>),
    PluginInstalled(AppResult<()>),
    VsixExtracted {
        ticket: u64,
        result: AppResult<VsixExtraction>,
    },
    VsixThemesSaved {
        ticket: u64,
        count: usize,
        extension: String,
        result: AppResult<()>,
    },
    VsixGrammarsImported {
        ticket: u64,
        result: AppResult<()>,
    },
    SyncStatus {
        generation: u64,
        result: AppResult<SyncStatus>,
    },
    RemoteStatus {
        generation: u64,
        result: AppResult<RemoteStatus>,
    },
    SyncConnected(AppResult<SyncStatus>),
    SyncDisconnected(AppResult<SyncStatus>),
    SyncUploaded(AppResult<SyncStatus>),
    SyncDownloaded {
        force: bool,
        owner: u64,
        result: AppResult<SyncDownloadResult>,
    },
    RemoteLinkIssued {
        owner: u64,
        result: AppResult<(RemoteLinkInfo, AppResult<()>)>,
    },
    RemoteRevoked(AppResult<()>),
    RemotePasswordSet(AppResult<()>),
    RemotePasswordCleared(AppResult<()>),
    SettingsUpdated(AppResult<()>),
    CliStatus {
        generation: u64,
        result: AppResult<CliInstallStatus>,
    },
    CliInstalled(AppResult<CliOutcome>),
    CliUninstalled(AppResult<CliOutcome>),
    HooksStatus {
        target: HookTarget,
        generation: u64,
        result: AppResult<AgentHooksStatus>,
    },
    HooksChanged {
        target: HookTarget,
        change: HookChange,
        result: AppResult<AgentHooksStatus>,
    },
    NotificationSettingsOpened(AppResult<()>),
    TestNotification(AppResult<NotificationDelivery>),
    PerfRead {
        ticket: u64,
        result: AppResult<PerfSnapshot>,
    },
    PerfReset {
        ticket: u64,
        result: AppResult<()>,
    },
}

#[derive(Debug)]
pub enum Notice {
    Success(&'static str),
    SuccessWith {
        key: &'static str,
        arguments: Vec<(&'static str, String)>,
    },
    Warning(&'static str),
    Failure(&'static str),
    Error {
        error: AppError,
        fallback: Option<&'static str>,
    },
    SettingsFailed(AppError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub phase: LspInstallPhase,
    pub received_bytes: f64,
    pub total_bytes: Option<f64>,
    pub message: Option<String>,
}

impl Progress {
    pub fn is_failed(&self) -> bool {
        self.phase == LspInstallPhase::Failed
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.phase, LspInstallPhase::Done | LspInstallPhase::Failed)
    }
}

pub enum Status<'a, T> {
    Pending,
    Ready(&'a T),
    Failed(&'a AppError),
}

struct Slot<T> {
    value: Option<(AppResult<T>, Instant)>,
    generation: u64,
    fetching: bool,
    invalidated: bool,
    observed_at: Option<Instant>,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            value: None,
            generation: 0,
            fetching: false,
            invalidated: false,
            observed_at: None,
        }
    }
}

impl<T> Slot<T> {
    fn observe(
        &mut self,
        query: Query,
        stale_time: Option<Duration>,
        mounting: bool,
        now: Instant,
        requests: &mut Vec<Request>,
    ) {
        if mounting
            && !self.fetching
            && self
                .observed_at
                .is_some_and(|at| now.duration_since(at) >= GC_TIME)
        {
            self.value = None;
        }
        self.observed_at = Some(now);
        if self.fetching {
            return;
        }
        let stale = match &self.value {
            None => true,
            Some((Err(_), _)) => mounting,
            Some((Ok(_), at)) => {
                mounting && stale_time.is_some_and(|stale| now.duration_since(*at) >= stale)
            }
        };
        if !stale && !self.invalidated {
            return;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            return;
        };
        self.generation = generation;
        self.fetching = true;
        self.invalidated = false;
        requests.push(Request::Fetch { query, generation });
    }

    fn accept(&mut self, generation: u64, result: AppResult<T>, now: Instant) {
        if !self.fetching || generation != self.generation {
            return;
        }
        self.fetching = false;
        self.value = Some((result, now));
    }

    fn invalidate(&mut self) {
        self.invalidated = true;
    }

    fn refetch_due(&self, interval: Duration, now: Instant) -> Option<Duration> {
        if self.fetching {
            return None;
        }
        let (_, at) = self.value.as_ref()?;
        Some(interval.saturating_sub(now.duration_since(*at)))
    }

    fn replace(&mut self, value: T, now: Instant) {
        self.value = Some((Ok(value), now));
    }

    fn status(&self) -> Status<'_, T> {
        match &self.value {
            Some((Ok(value), _)) => Status::Ready(value),
            Some((Err(error), _)) => Status::Failed(error),
            None => Status::Pending,
        }
    }
}

/// Host-backed Settings data shared by every Settings owner of the application.
#[derive(Default)]
pub struct Data {
    lsp_servers: Slot<Vec<LspServerDetection>>,
    lsp_progress: BTreeMap<LspServerId, Progress>,
    ai_tokens: Slot<AiTokenStatus>,
    ai_models: [Slot<Vec<AiModelInfo>>; PROVIDER_COUNT],
    ai_token_saving: Option<(AiProviderId, u64)>,
    ai_token_sequence: u64,
    omlx_base_url: Option<Option<String>>,
    plugins: Slot<Vec<LoadedPlugin>>,
    is_reloading_plugins: bool,
    is_installing_plugin: bool,
    is_extracting_vsix: bool,
    pick: Option<PickPurpose>,
    vsix_ticket: u64,
    vsix_events: Vec<(u64, VsixEvent)>,
    sync: Slot<SyncStatus>,
    remote: Slot<RemoteStatus>,
    remote_access: Option<bool>,
    busy: BTreeMap<Busy, usize>,
    owner_ticket: u64,
    owner_events: Vec<(u64, OwnerEvent)>,
    cli: Slot<CliInstallStatus>,
    hooks: BTreeMap<HookTarget, Slot<AgentHooksStatus>>,
    hooks_busy: BTreeMap<(HookTarget, HookChange), usize>,
    is_any_window_focused: bool,
    is_perf_enabled: bool,
    requests: Vec<Request>,
}

fn provider_index(provider: AiProviderId) -> usize {
    match provider {
        AiProviderId::OllamaCloud => 0,
        AiProviderId::Codex => 1,
        AiProviderId::Omlx => 2,
    }
}

fn is_error_key(error: &AppError, key: &str) -> bool {
    matches!(error, AppError::Localized(localized) if localized.key == key)
}

impl Data {
    pub fn observe(&mut self, query: Query, mounting: bool, now: Instant) {
        let requests = &mut self.requests;
        match query {
            Query::LspServers => self
                .lsp_servers
                .observe(query, None, mounting, now, requests),
            Query::AiTokens => {
                self.ai_tokens
                    .observe(query, Some(STALE_TIME), mounting, now, requests)
            }
            Query::AiModels(provider) => self.ai_models[provider_index(provider)]
                .observe(query, None, mounting, now, requests),
            Query::Plugins => {
                self.plugins
                    .observe(query, Some(STALE_TIME), mounting, now, requests)
            }
            Query::SyncStatus => {
                self.sync
                    .observe(query, Some(STALE_TIME), mounting, now, requests)
            }
            Query::RemoteStatus => {
                if self
                    .remote
                    .refetch_due(REMOTE_REFETCH_INTERVAL, now)
                    .is_some_and(|due| due.is_zero())
                {
                    self.remote.invalidate();
                }
                self.remote
                    .observe(query, Some(STALE_TIME), mounting, now, requests)
            }
            Query::CliStatus => self
                .cli
                .observe(query, Some(STALE_TIME), mounting, now, requests),
            Query::HooksStatus(target) => self.hooks.entry(target.clone()).or_default().observe(
                Query::HooksStatus(target),
                Some(STALE_TIME),
                mounting,
                now,
                requests,
            ),
        }
    }

    pub fn cli_status(&self) -> Status<'_, CliInstallStatus> {
        self.cli.status()
    }

    pub fn hooks_status(&self, target: &HookTarget) -> Status<'_, AgentHooksStatus> {
        self.hooks.get(target).map_or(Status::Pending, Slot::status)
    }

    pub fn is_changing_hooks(&self, target: &HookTarget, change: HookChange) -> bool {
        self.hooks_busy.contains_key(&(target.clone(), change))
    }

    pub fn change_hooks(&mut self, target: HookTarget, change: HookChange) {
        *self.hooks_busy.entry((target.clone(), change)).or_default() += 1;
        self.act(Action::Hooks { target, change });
    }

    /// Records whether any application window has focus, which a test notification carries.
    pub fn observe_window_focus(&mut self, is_focused: bool) {
        self.is_any_window_focused = is_focused;
    }

    /// Records the instrumentation gate that shows the Performance section.
    pub fn observe_perf_gate(&mut self, is_enabled: bool) {
        self.is_perf_enabled = is_enabled;
    }

    pub fn is_perf_enabled(&self) -> bool {
        self.is_perf_enabled
    }

    pub fn send_test_notification(&mut self, title: String, body: String) {
        let any_window_focused = self.is_any_window_focused;
        self.act(Action::SendTestNotification {
            title,
            body,
            any_window_focused,
        });
    }

    pub fn sync_status(&self) -> Status<'_, SyncStatus> {
        self.sync.status()
    }

    pub fn remote_status(&self) -> Status<'_, RemoteStatus> {
        self.remote.status()
    }

    /// Time left until the observed remote status is fetched again, while no fetch is running.
    pub fn remote_refetch_in(&self, now: Instant) -> Option<Duration> {
        self.remote.refetch_due(REMOTE_REFETCH_INTERVAL, now)
    }

    pub fn record_sync_status(&mut self, status: SyncStatus, now: Instant) {
        self.sync.replace(status, now);
    }

    pub fn record_remote_status(&mut self, status: RemoteStatus, now: Instant) {
        self.remote.replace(status, now);
    }

    pub fn is_busy(&self, busy: Busy) -> bool {
        self.busy.contains_key(&busy)
    }

    fn finish(&mut self, busy: Busy) {
        if let Some(count) = self.busy.get_mut(&busy) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.busy.remove(&busy);
            }
        }
    }

    /// Identifies one Settings owner so host results of its actions come back to it.
    pub fn next_owner(&mut self) -> Option<u64> {
        self.owner_ticket = self.owner_ticket.checked_add(1)?;
        Some(self.owner_ticket)
    }

    pub fn take_owner_events(&mut self, owner: u64) -> Vec<OwnerEvent> {
        let (taken, kept) = std::mem::take(&mut self.owner_events)
            .into_iter()
            .partition(|(target, _)| *target == owner);
        self.owner_events = kept;
        taken.into_iter().map(|(_, event)| event).collect()
    }

    fn push_owner_event(&mut self, owner: u64, event: OwnerEvent) {
        if self.owner_events.len() >= OWNER_EVENT_LIMIT {
            self.owner_events.remove(0);
        }
        self.owner_events.push((owner, event));
    }

    pub fn plugins(&self) -> Status<'_, Vec<LoadedPlugin>> {
        self.plugins.status()
    }

    pub fn is_reloading_plugins(&self) -> bool {
        self.is_reloading_plugins
    }

    pub fn is_installing_plugin(&self) -> bool {
        self.is_installing_plugin
    }

    pub fn is_extracting_vsix(&self) -> bool {
        self.is_extracting_vsix
    }

    pub fn reload_plugins(&mut self) {
        self.is_reloading_plugins = true;
        self.act(Action::PluginReload);
    }

    /// Starts a VSIX import dialog session and returns the ticket its host results carry.
    pub fn next_vsix_ticket(&mut self) -> Option<u64> {
        self.vsix_ticket = self.vsix_ticket.checked_add(1)?;
        Some(self.vsix_ticket)
    }

    pub fn request_pick(&mut self, purpose: PickPurpose) {
        self.pick = Some(purpose);
    }

    pub fn take_pick(&mut self) -> Option<PickPurpose> {
        self.pick.take()
    }

    /// Continues a file pick; a cancelled pick starts nothing.
    pub fn picked(&mut self, purpose: PickPurpose, path: Option<PathBuf>) {
        let Some(path) = path else {
            return;
        };
        match purpose {
            PickPurpose::PluginArchive => {
                self.is_installing_plugin = true;
                self.act(Action::PluginInstall(path));
            }
            PickPurpose::Vsix(ticket) => {
                self.is_extracting_vsix = true;
                self.act(Action::VsixExtract { path, ticket });
            }
        }
    }

    pub fn take_vsix_events(&mut self, ticket: u64) -> Vec<VsixEvent> {
        let (taken, kept) = std::mem::take(&mut self.vsix_events)
            .into_iter()
            .partition(|(owner, _)| *owner == ticket);
        self.vsix_events = kept;
        taken.into_iter().map(|(_, event)| event).collect()
    }

    fn push_vsix_event(&mut self, ticket: u64, event: VsixEvent) {
        if self.vsix_events.len() >= VSIX_EVENT_LIMIT {
            self.vsix_events.remove(0);
        }
        self.vsix_events.push((ticket, event));
    }

    pub fn observe_settings(&mut self, settings: &Settings) {
        let current = settings.ai_omlx_base_url.clone();
        if let Some(previous) = &self.omlx_base_url
            && *previous != current
        {
            self.ai_tokens.invalidate();
            self.ai_models[provider_index(AiProviderId::Omlx)].invalidate();
        }
        self.omlx_base_url = Some(current);
        let access = settings.remote_access_enabled;
        if self
            .remote_access
            .is_some_and(|previous| previous != access)
        {
            self.remote.invalidate();
        }
        self.remote_access = Some(access);
    }

    pub fn lsp_servers(&self) -> Status<'_, Vec<LspServerDetection>> {
        self.lsp_servers.status()
    }

    pub fn lsp_progress(&self, server: &LspServerId) -> Option<&Progress> {
        self.lsp_progress.get(server)
    }

    pub fn ai_tokens(&self) -> Status<'_, AiTokenStatus> {
        self.ai_tokens.status()
    }

    pub fn ai_models(&self, provider: AiProviderId) -> Status<'_, Vec<AiModelInfo>> {
        self.ai_models[provider_index(provider)].status()
    }

    pub fn is_saving_ai_token(&self, provider: AiProviderId) -> bool {
        self.ai_token_saving
            .is_some_and(|(saving, _)| saving == provider)
    }

    pub fn act(&mut self, action: Action) {
        if let Some(busy) = action.busy() {
            *self.busy.entry(busy).or_default() += 1;
        }
        self.requests.push(Request::Act(action));
    }

    pub fn save_ai_token(&mut self, provider: AiProviderId, token: Secret) {
        let Some(sequence) = self.ai_token_sequence.checked_add(1) else {
            return;
        };
        self.ai_token_sequence = sequence;
        self.ai_token_saving = Some((provider, sequence));
        self.act(Action::AiSetToken {
            provider,
            token,
            sequence,
        });
    }

    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    pub fn record_progress(&mut self, server: LspServerId, progress: Progress) {
        self.lsp_progress.insert(server, progress);
    }

    pub fn has_active_install(&self) -> bool {
        self.lsp_progress.values().any(Progress::is_active)
    }

    pub fn accept(&mut self, reply: Reply, now: Instant) -> Option<Notice> {
        match reply {
            Reply::LspServers { generation, result } => {
                self.lsp_servers.accept(generation, result, now);
                None
            }
            Reply::AiTokens { generation, result } => {
                self.ai_tokens.accept(generation, result, now);
                None
            }
            Reply::AiModels {
                provider,
                generation,
                result,
            } => {
                self.ai_models[provider_index(provider)].accept(generation, result, now);
                None
            }
            Reply::LspInstalled { server, result } => match result {
                Ok(()) => {
                    self.lsp_servers.invalidate();
                    None
                }
                Err(error) => {
                    if self
                        .lsp_progress
                        .get(&server)
                        .is_some_and(Progress::is_active)
                    {
                        self.lsp_progress.remove(&server);
                    }
                    (!is_error_key(&error, INSTALL_CANCELLED_KEY)).then_some(Notice::Error {
                        error,
                        fallback: None,
                    })
                }
            },
            Reply::LspCancelled(result) => result.err().map(|error| Notice::Error {
                error,
                fallback: None,
            }),
            Reply::HintCopied(result) => Some(match result {
                Ok(()) => Notice::Success("settings.lspCommandCopied"),
                Err(_) => Notice::Failure("settings.lspCommandCopyFailed"),
            }),
            Reply::AiTokenSaved {
                provider,
                sequence,
                result,
            } => {
                if self
                    .ai_token_saving
                    .is_some_and(|(_, saving)| saving == sequence)
                {
                    self.ai_token_saving = None;
                }
                self.finish_token(provider, result, "settings.aiTokenSaveFailed")
            }
            Reply::AiTokenCleared { provider, result } => {
                self.finish_token(provider, result, "settings.aiTokenClearFailed")
            }
            Reply::Plugins { generation, result } => {
                self.plugins.accept(generation, result, now);
                None
            }
            Reply::PluginsReloaded(result) => {
                self.is_reloading_plugins = false;
                match result {
                    Ok(plugins) => {
                        self.plugins.replace(plugins, now);
                        None
                    }
                    Err(error) => Some(Notice::Error {
                        error,
                        fallback: None,
                    }),
                }
            }
            Reply::PluginUninstalled(result) => match result {
                Ok(plugins) => {
                    self.plugins.replace(plugins, now);
                    Some(Notice::Success("settings.pluginUninstallSuccess"))
                }
                Err(error) => Some(Notice::Error {
                    error,
                    fallback: Some("settings.pluginUninstallFailed"),
                }),
            },
            Reply::PluginInstalled(result) => {
                self.is_installing_plugin = false;
                match result {
                    Ok(()) => {
                        self.plugins.invalidate();
                        Some(Notice::Success("settings.pluginInstallSuccess"))
                    }
                    Err(error) => Some(Notice::Error {
                        error,
                        fallback: Some("settings.pluginInstallFailed"),
                    }),
                }
            }
            Reply::VsixExtracted { ticket, result } => {
                self.is_extracting_vsix = false;
                match result {
                    Ok(extraction) => {
                        self.push_vsix_event(ticket, VsixEvent::Extracted(extraction));
                        None
                    }
                    Err(error) => Some(Notice::Error {
                        error,
                        fallback: None,
                    }),
                }
            }
            Reply::VsixThemesSaved {
                ticket,
                count,
                extension,
                result,
            } => {
                self.push_vsix_event(ticket, VsixEvent::Saved);
                Some(match result {
                    Ok(()) => Notice::SuccessWith {
                        key: "settings.themeImportSuccess",
                        arguments: vec![("count", count.to_string()), ("extension", extension)],
                    },
                    Err(error) => Notice::Error {
                        error,
                        fallback: None,
                    },
                })
            }
            Reply::VsixGrammarsImported { ticket, result } => {
                let notice = result.is_ok().then(|| {
                    self.plugins.invalidate();
                    Notice::Success("settings.pluginImportVsixSuccess")
                });
                self.push_vsix_event(ticket, VsixEvent::GrammarsImported(result));
                notice
            }
            Reply::SyncStatus { generation, result } => {
                self.sync.accept(generation, result, now);
                None
            }
            Reply::RemoteStatus { generation, result } => {
                self.remote.accept(generation, result, now);
                None
            }
            Reply::SyncConnected(result) => {
                self.finish(Busy::SyncConnect);
                match result {
                    Ok(status) => {
                        self.sync.replace(status, now);
                        None
                    }
                    Err(error) => Some(Notice::Error {
                        error,
                        fallback: Some("settings.syncConnectFailed"),
                    }),
                }
            }
            Reply::SyncDisconnected(result) => {
                self.finish(Busy::SyncDisconnect);
                match result {
                    Ok(status) => {
                        self.sync.replace(status, now);
                        Some(Notice::Success("settings.syncDisconnected"))
                    }
                    Err(error) => Some(Notice::Error {
                        error,
                        fallback: Some("settings.syncDisconnectFailed"),
                    }),
                }
            }
            Reply::SyncUploaded(result) => {
                self.finish(Busy::SyncUpload);
                Some(match result {
                    Ok(status) => {
                        self.sync.replace(status, now);
                        Notice::Success("settings.syncUploadSuccess")
                    }
                    Err(_) => Notice::Failure("settings.syncUploadFailed"),
                })
            }
            Reply::SyncDownloaded {
                force,
                owner,
                result,
            } => {
                self.finish(Busy::SyncDownload);
                match result {
                    Ok(SyncDownloadResult::Applied { status }) => {
                        self.sync.replace(status, now);
                        Some(Notice::Success("settings.syncDownloadSuccess"))
                    }
                    Ok(SyncDownloadResult::Conflict { .. }) if !force => {
                        self.push_owner_event(owner, OwnerEvent::SyncConflict);
                        None
                    }
                    Ok(SyncDownloadResult::Conflict { .. }) => {
                        Some(Notice::Success("settings.syncDownloadSuccess"))
                    }
                    Err(_) => Some(Notice::Failure("settings.syncDownloadFailed")),
                }
            }
            Reply::RemoteLinkIssued { owner, result } => {
                self.finish(Busy::RemoteIssueLink);
                match result {
                    Ok((info, copied)) => {
                        self.push_owner_event(owner, OwnerEvent::RemoteLinkIssued(info.url));
                        copied.ok().map(|()| Notice::Success("remote.linkCopied"))
                    }
                    Err(_) => Some(Notice::Failure("remote.startFailed")),
                }
            }
            Reply::RemoteRevoked(result) => {
                self.finish(Busy::RemoteRevoke);
                result
                    .ok()
                    .map(|()| Notice::Success("remote.sessionsRevoked"))
            }
            Reply::RemotePasswordSet(result) => {
                self.finish(Busy::RemotePassword);
                Some(match result {
                    Ok(()) => {
                        self.remote.invalidate();
                        Notice::Success("remote.passwordSessionsRevoked")
                    }
                    Err(_) => Notice::Failure("remote.passwordSaveFailed"),
                })
            }
            Reply::RemotePasswordCleared(result) => {
                self.finish(Busy::RemotePassword);
                result.ok().map(|()| {
                    self.remote.invalidate();
                    Notice::Success("remote.passwordSessionsRevoked")
                })
            }
            Reply::SettingsUpdated(result) => {
                self.finish(Busy::Settings);
                result.err().map(Notice::SettingsFailed)
            }
            Reply::CliStatus { generation, result } => {
                self.cli.accept(generation, result, now);
                None
            }
            Reply::CliInstalled(result) => {
                self.finish(Busy::CliInstall);
                self.finish_cli(
                    result,
                    "settings.cliInstallSuccess",
                    "settings.cliInstallFailed",
                    now,
                )
            }
            Reply::CliUninstalled(result) => {
                self.finish(Busy::CliUninstall);
                self.finish_cli(
                    result,
                    "settings.cliUninstallSuccess",
                    "settings.cliUninstallFailed",
                    now,
                )
            }
            Reply::HooksStatus {
                target,
                generation,
                result,
            } => {
                if let Some(slot) = self.hooks.get_mut(&target) {
                    slot.accept(generation, result, now);
                }
                None
            }
            Reply::HooksChanged {
                target,
                change,
                result,
            } => {
                let key = (target, change);
                if let Some(count) = self.hooks_busy.get_mut(&key) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        self.hooks_busy.remove(&key);
                    }
                }
                match result {
                    Ok(status) => {
                        self.hooks.entry(key.0).or_default().replace(status, now);
                        None
                    }
                    Err(error) => Some(Notice::Error {
                        error,
                        fallback: None,
                    }),
                }
            }
            Reply::NotificationSettingsOpened(result) => result.err().map(|error| Notice::Error {
                error,
                fallback: None,
            }),
            Reply::TestNotification(result) => Some(match result {
                Ok(NotificationDelivery::Delivered) => Notice::Success("notification.enableHint"),
                Ok(NotificationDelivery::Suppressed(reason)) => {
                    Notice::Warning(suppression_label(reason))
                }
                Err(error) => Notice::Error {
                    error,
                    fallback: None,
                },
            }),
            Reply::PerfRead { ticket, result } => {
                match result.and_then(|snapshot| perf_json(&snapshot)) {
                    Ok(json) => {
                        self.push_owner_event(ticket, OwnerEvent::PerfRead(json));
                        None
                    }
                    Err(error) => {
                        self.push_owner_event(ticket, OwnerEvent::PerfFailed);
                        Some(Notice::Error {
                            error,
                            fallback: None,
                        })
                    }
                }
            }
            Reply::PerfReset { ticket, result } => match result {
                Ok(()) => {
                    self.push_owner_event(ticket, OwnerEvent::PerfReset);
                    None
                }
                Err(error) => {
                    self.push_owner_event(ticket, OwnerEvent::PerfFailed);
                    Some(Notice::Error {
                        error,
                        fallback: None,
                    })
                }
            },
        }
    }

    fn finish_cli(
        &mut self,
        result: AppResult<CliOutcome>,
        success: &'static str,
        failure: &'static str,
        now: Instant,
    ) -> Option<Notice> {
        match result {
            Ok(outcome) => {
                self.cli.replace(outcome.status, now);
                (!outcome.is_cancelled).then_some(Notice::Success(success))
            }
            Err(_) => Some(Notice::Failure(failure)),
        }
    }

    fn finish_token(
        &mut self,
        provider: AiProviderId,
        result: AppResult<()>,
        fallback: &'static str,
    ) -> Option<Notice> {
        match result {
            Ok(()) => {
                self.ai_tokens.invalidate();
                self.ai_models[provider_index(provider)].invalidate();
                None
            }
            Err(error) => Some(Notice::Error {
                error,
                fallback: Some(fallback),
            }),
        }
    }
}

impl Request {
    pub fn failed(&self, error: AppError) -> Reply {
        match self {
            Self::Fetch {
                query: Query::LspServers,
                generation,
            } => Reply::LspServers {
                generation: *generation,
                result: Err(error),
            },
            Self::Fetch {
                query: Query::AiTokens,
                generation,
            } => Reply::AiTokens {
                generation: *generation,
                result: Err(error),
            },
            Self::Fetch {
                query: Query::AiModels(provider),
                generation,
            } => Reply::AiModels {
                provider: *provider,
                generation: *generation,
                result: Err(error),
            },
            Self::Act(Action::LspInstall(server)) => Reply::LspInstalled {
                server: server.clone(),
                result: Err(error),
            },
            Self::Act(Action::LspCancel(_)) => Reply::LspCancelled(Err(error)),
            Self::Act(Action::CopyInstallHint(_)) => Reply::HintCopied(Err(error)),
            Self::Act(Action::AiSetToken {
                provider, sequence, ..
            }) => Reply::AiTokenSaved {
                provider: *provider,
                sequence: *sequence,
                result: Err(error),
            },
            Self::Act(Action::AiClearToken(provider)) => Reply::AiTokenCleared {
                provider: *provider,
                result: Err(error),
            },
            Self::Fetch {
                query: Query::Plugins,
                generation,
            } => Reply::Plugins {
                generation: *generation,
                result: Err(error),
            },
            Self::Act(Action::PluginReload) => Reply::PluginsReloaded(Err(error)),
            Self::Act(Action::PluginUninstall { .. }) => Reply::PluginUninstalled(Err(error)),
            Self::Act(Action::PluginInstall(_)) => Reply::PluginInstalled(Err(error)),
            Self::Act(Action::VsixExtract { ticket, .. }) => Reply::VsixExtracted {
                ticket: *ticket,
                result: Err(error),
            },
            Self::Act(Action::VsixSaveThemes {
                themes,
                extension,
                ticket,
            }) => Reply::VsixThemesSaved {
                ticket: *ticket,
                count: themes.len(),
                extension: extension.clone(),
                result: Err(error),
            },
            Self::Act(Action::VsixImportGrammars { ticket, .. }) => Reply::VsixGrammarsImported {
                ticket: *ticket,
                result: Err(error),
            },
            Self::Fetch {
                query: Query::SyncStatus,
                generation,
            } => Reply::SyncStatus {
                generation: *generation,
                result: Err(error),
            },
            Self::Fetch {
                query: Query::RemoteStatus,
                generation,
            } => Reply::RemoteStatus {
                generation: *generation,
                result: Err(error),
            },
            Self::Act(Action::SyncConnect(_)) => Reply::SyncConnected(Err(error)),
            Self::Act(Action::SyncDisconnect) => Reply::SyncDisconnected(Err(error)),
            Self::Act(Action::SyncUpload) => Reply::SyncUploaded(Err(error)),
            Self::Act(Action::SyncDownload { force, owner }) => Reply::SyncDownloaded {
                force: *force,
                owner: *owner,
                result: Err(error),
            },
            Self::Act(Action::RemoteIssueLink { owner }) => Reply::RemoteLinkIssued {
                owner: *owner,
                result: Err(error),
            },
            Self::Act(Action::RemoteRevoke) => Reply::RemoteRevoked(Err(error)),
            Self::Act(Action::RemoteSetPassword(_)) => Reply::RemotePasswordSet(Err(error)),
            Self::Act(Action::RemoteClearPassword) => Reply::RemotePasswordCleared(Err(error)),
            Self::Act(Action::UpdateSettings(_)) => Reply::SettingsUpdated(Err(error)),
            Self::Fetch {
                query: Query::CliStatus,
                generation,
            } => Reply::CliStatus {
                generation: *generation,
                result: Err(error),
            },
            Self::Fetch {
                query: Query::HooksStatus(target),
                generation,
            } => Reply::HooksStatus {
                target: target.clone(),
                generation: *generation,
                result: Err(error),
            },
            Self::Act(Action::CliInstall) => Reply::CliInstalled(Err(error)),
            Self::Act(Action::CliUninstall) => Reply::CliUninstalled(Err(error)),
            Self::Act(Action::Hooks { target, change }) => Reply::HooksChanged {
                target: target.clone(),
                change: *change,
                result: Err(error),
            },
            Self::Act(Action::OpenNotificationSettings) => {
                Reply::NotificationSettingsOpened(Err(error))
            }
            Self::Act(Action::SendTestNotification { .. }) => Reply::TestNotification(Err(error)),
            Self::Act(Action::PerfRead { ticket }) => Reply::PerfRead {
                ticket: *ticket,
                result: Err(error),
            },
            Self::Act(Action::PerfReset { ticket }) => Reply::PerfReset {
                ticket: *ticket,
                result: Err(error),
            },
        }
    }
}

#[cfg(test)]
#[path = "settings-sections-data-tests.rs"]
mod tests;

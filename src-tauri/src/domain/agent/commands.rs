use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tauri::{Manager, State};
use tauri_specta::Event;

use super::hooks;
use super::service;
use super::types::{
    AgentActivity, AgentHooksStatus, CliInstallStatus, DetectedAgent, ExternalOpenRequest, HookInstallScope, ProjectAgents,
    AGENT_NAME_CLAUDE, AGENT_PROTOCOL_VERSION, AGENT_PROTOCOL_VERSION_ENV_NAME, APP_VERSION_ENV_NAME, CLAUDE_VERSION_TIMEOUT_SECONDS,
};
use crate::domain::terminal::commands::TerminalStore;
use crate::error::{AppError, AppResult};
use crate::events::AgentStateChanged;
use crate::ids::ProjectId;
use crate::infra::terminal_scan::ScanOutcome;
use crate::state::AppState;

#[cfg(unix)]
pub(super) const TAIDE_CLI_TARGET_PATH: &str = "/usr/local/bin/taide";
#[cfg(windows)]
pub(super) const TAIDE_CLI_TARGET_PATH: &str = "C:/Program Files/TAIDE/bin/taide.exe";

#[derive(Default)]
struct AgentStoreInner {
    agents: HashMap<ProjectId, Vec<DetectedAgent>>,
    wait_markers: HashSet<String>,
    /// One entry per pty session that currently runs an agent, keyed by session id — created by the
    /// poll tick that first detects the agent, fed by the pty reader and by `pty_write`, dropped
    /// when the session no longer has one.
    signals: HashMap<String, service::AgentSessionSignals>,
    /// `pid -> agent name`, so a `ps` fork happens only on the tick a session's foreground pid
    /// actually changes rather than every 500ms tick forever. `None` is a cached answer too ("this
    /// pid is a shell, not an agent"); entries leave when their pid leaves the foreground set.
    #[cfg(unix)]
    process_names: HashMap<u32, Option<&'static str>>,
    pending_external_opens: Vec<ExternalOpenRequest>,
}

#[derive(Default)]
pub struct AgentStore(Mutex<AgentStoreInner>);

fn last_known_activity(inner: &AgentStoreInner, session_id: &str) -> AgentActivity {
    inner
        .agents
        .values()
        .flatten()
        .find(|agent| agent.session_id == session_id)
        .map(|agent| agent.activity)
        .unwrap_or(AgentActivity::Unknown)
}

impl AgentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn diff(&self, project_id: &ProjectId, current: &[DetectedAgent]) -> Option<Vec<DetectedAgent>> {
        let mut guard = self.0.lock();
        let previous = guard.agents.entry(project_id.clone()).or_default();
        if service::agents_changed(previous, current) {
            *previous = current.to_vec();
            Some(current.to_vec())
        } else {
            None
        }
    }

    pub fn agents_for(&self, project_id: &ProjectId) -> Vec<DetectedAgent> {
        self.0.lock().agents.get(project_id).cloned().unwrap_or_default()
    }

    /// Classifies one detected agent session from the signals collected since the last tick,
    /// starting a signal record for a session seen for the first time.
    ///
    /// The record restarts when the same session's foreground agent changed, which `prune_signals`
    /// cannot catch on its own: it only drops sessions that ran *no* agent on a tick, so a handoff
    /// with no shell in between would leave the old agent's dialog latch and signature table
    /// speaking for the new one.
    pub fn classify_session_activity(&self, session_id: &str, agent_name: &'static str) -> AgentActivity {
        let mut guard = self.0.lock();
        let previous = last_known_activity(&guard, session_id);
        let signals = guard
            .signals
            .entry(session_id.to_string())
            .or_insert_with(|| service::AgentSessionSignals::new(agent_name));
        if signals.agent_name != agent_name {
            *signals = service::AgentSessionSignals::new(agent_name);
        }
        service::classify_session(signals, previous, Instant::now())
    }

    /// Folds one scanned pty chunk into the session's signals. Runs on the pty reader thread for
    /// every chunk of every session, so a session without a detected agent costs one lock and one
    /// failed lookup and nothing else.
    fn record_scan(&self, session_id: &str, outcome: &ScanOutcome) {
        let mut guard = self.0.lock();
        let Some(signals) = guard.signals.get_mut(session_id) else {
            return;
        };
        let agent_name = signals.agent_name;
        service::apply_scan_to_signals(signals, outcome, agent_name, Instant::now());
    }

    fn record_input(&self, session_id: &str) {
        let mut guard = self.0.lock();
        let Some(signals) = guard.signals.get_mut(session_id) else {
            return;
        };
        service::note_input(signals, Instant::now());
    }

    pub fn prune_signals(&self, valid_session_ids: &HashSet<String>) {
        self.0.lock().signals.retain(|session_id, _| valid_session_ids.contains(session_id));
    }

    #[cfg(unix)]
    fn unresolved_pids(&self, pids: &[(String, u32)]) -> Vec<u32> {
        let guard = self.0.lock();
        let mut unresolved: Vec<u32> = pids
            .iter()
            .map(|(_, pid)| *pid)
            .filter(|pid| !guard.process_names.contains_key(pid))
            .collect();
        unresolved.sort_unstable();
        unresolved.dedup();
        unresolved
    }

    #[cfg(unix)]
    fn remember_process_names(&self, resolved: HashMap<u32, Option<&'static str>>) {
        self.0.lock().process_names.extend(resolved);
    }

    #[cfg(unix)]
    fn probes_for(&self, pids: Vec<(String, u32)>) -> Vec<service::DetectedAgentProbe> {
        let guard = self.0.lock();
        pids.into_iter()
            .filter_map(|(session_id, pid)| {
                let name = (*guard.process_names.get(&pid)?)?;
                Some(service::DetectedAgentProbe { session_id, name, pid })
            })
            .collect()
    }

    /// Drops name-cache entries for pids that are no longer any session's foreground process, so a
    /// long-running app does not accumulate one entry per command the user ever ran.
    pub fn retain_process_names(&self, live_pids: &HashSet<u32>) {
        #[cfg(unix)]
        self.0.lock().process_names.retain(|pid, _| live_pids.contains(pid));
        #[cfg(not(unix))]
        let _ = live_pids;
    }

    pub fn register_wait_marker(&self, marker: String) {
        self.0.lock().wait_markers.insert(marker);
    }

    pub fn forget_wait_marker(&self, marker: &str) {
        self.0.lock().wait_markers.remove(marker);
    }

    pub fn take_all_markers(&self) -> Vec<String> {
        std::mem::take(&mut self.0.lock().wait_markers).into_iter().collect()
    }

    /// Queues an external-open request (from a cold-start or single-instance `taide <file>`
    /// invocation) so the frontend can drain it once it has mounted and subscribed, even if it
    /// missed the corresponding `AgentExternalOpen` event.
    pub fn push_pending_external_open(&self, request: ExternalOpenRequest) {
        self.0.lock().pending_external_opens.push(request);
    }

    pub fn drain_pending_external_opens(&self) -> Vec<ExternalOpenRequest> {
        std::mem::take(&mut self.0.lock().pending_external_opens)
    }
}

#[derive(Clone)]
pub struct HooksServerInfo {
    pub port: u16,
    pub token: String,
}

#[derive(Default)]
struct AgentHooksStoreInner {
    server: Option<HooksServerInfo>,
    accept_handle: Option<tauri::async_runtime::JoinHandle<()>>,
    project_overrides: HashMap<(ProjectId, String), (AgentActivity, Instant)>,
}

#[derive(Default)]
pub struct AgentHooksStore(Mutex<AgentHooksStoreInner>);

impl AgentHooksStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn server_info(&self) -> Option<HooksServerInfo> {
        self.0.lock().server.clone()
    }

    pub fn set_server(&self, info: HooksServerInfo, accept_handle: tauri::async_runtime::JoinHandle<()>) {
        let mut guard = self.0.lock();
        guard.server = Some(info);
        guard.accept_handle = Some(accept_handle);
    }

    /// 서버 정보·accept 핸들을 회수하고 남은 hook override 도 버린다(배지 오염 방지).
    pub fn take_server(&self) -> Option<tauri::async_runtime::JoinHandle<()>> {
        let mut guard = self.0.lock();
        guard.server = None;
        guard.project_overrides.clear();
        guard.accept_handle.take()
    }

    pub fn set_project_override(&self, project_id: ProjectId, agent_name: String, activity: AgentActivity) {
        self.0
            .lock()
            .project_overrides
            .insert((project_id, agent_name), (activity, Instant::now()));
    }

    pub fn fresh_project_override(&self, project_id: &ProjectId, agent_name: &str) -> Option<AgentActivity> {
        let guard = self.0.lock();
        let key = (project_id.clone(), agent_name.to_string());
        let (activity, set_at) = guard.project_overrides.get(&key)?;
        if set_at.elapsed().as_millis() as u64 >= super::types::HOOK_OVERRIDE_STALE_MS {
            return None;
        }
        Some(*activity)
    }
}

pub fn cleanup_all_wait_markers(store: &AgentStore) {
    let temp_dir = std::env::temp_dir();
    for marker in store.take_all_markers() {
        if let Ok(path) = service::validate_wait_marker_path(&marker, &temp_dir) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn ensure_project_open(state: &AppState, project_id: &ProjectId) -> AppResult<()> {
    if state.projects.read().contains_key(project_id) {
        return Ok(());
    }
    Err(AppError::NotFound(format!("project not open: {project_id}")))
}

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<String> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

#[cfg(unix)]
const PS_OUTPUT_FORMAT: &str = "pid=,comm=,args=";
#[cfg(unix)]
const PS_PID_SEPARATOR: &str = ",";

/// Probes every pty session's foreground pid with **one** `ps` fork, not one per pid: this runs on
/// the `AGENT_POLL_INTERVAL_MS` tick for every open project, so the per-pid form spent a process
/// spawn per session per tick forever. `ps` prints nothing for a pid that has already exited (and
/// exits non-zero only when *none* of them resolved), so the exit status is deliberately not
/// consulted — a batch where some pids died still carries the survivors on stdout.
#[cfg(unix)]
fn resolve_process_infos(pids: &[u32]) -> HashMap<u32, service::ProcessInfo> {
    if pids.is_empty() {
        return HashMap::new();
    }

    let joined = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(PS_PID_SEPARATOR);
    let Ok(output) = std::process::Command::new("ps")
        .args(["-o", PS_OUTPUT_FORMAT, "-p", &joined])
        .output()
    else {
        return HashMap::new();
    };

    service::parse_ps_process_infos(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(windows)]
fn process_snapshot(system: &sysinfo::System) -> Vec<service::ProcessSnapshot> {
    system
        .processes()
        .values()
        .map(|process| service::ProcessSnapshot {
            pid: process.pid().as_u32(),
            parent_pid: process.parent().map(|pid| pid.as_u32()),
            name: service::strip_windows_exe_suffix(&process.name().to_string_lossy()).to_string(),
            cmdline: process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect()
}

/// Resolves the agent name (or its absence) for pids the name cache has no answer for yet.
#[cfg(unix)]
fn resolve_agent_names(pids: &[u32]) -> HashMap<u32, Option<&'static str>> {
    let infos = resolve_process_infos(pids);
    pids.iter()
        .map(|pid| {
            let name = infos
                .get(pid)
                .and_then(|info| service::detect_agent_name(&info.comm, &info.cmdline));
            (*pid, name)
        })
        .collect()
}

#[cfg(windows)]
pub fn detect_agents_for_pids(pids: Vec<(String, u32)>) -> Vec<service::DetectedAgentProbe> {
    if pids.is_empty() {
        return Vec::new();
    }

    let snapshot = process_snapshot(&sysinfo::System::new_all());

    pids.into_iter()
        .filter_map(|(session_id, shell_pid)| {
            let (agent_pid, name) = service::find_descendant_agent(&snapshot, shell_pid)?;
            Some(service::DetectedAgentProbe {
                session_id,
                name,
                pid: agent_pid,
            })
        })
        .collect()
}

/// Returns immediately for a project with no pty sessions instead of paying a blocking-pool
/// dispatch for a probe that can only come back empty — the common case for every project whose
/// terminal panel was never opened, on every poll tick.
#[cfg(unix)]
pub async fn detect_agents_for_pids_blocking(agents: &AgentStore, pids: Vec<(String, u32)>) -> AppResult<Vec<service::DetectedAgentProbe>> {
    if pids.is_empty() {
        return Ok(Vec::new());
    }

    let unresolved = agents.unresolved_pids(&pids);
    if !unresolved.is_empty() {
        let resolved = tauri::async_runtime::spawn_blocking(move || resolve_agent_names(&unresolved))
            .await
            .map_err(|error| AppError::Internal(error.to_string()))?;
        agents.remember_process_names(resolved);
    }

    Ok(agents.probes_for(pids))
}

/// The windows probe walks the whole process tree below each shell pid (the agent is a descendant
/// of it, not the pid itself), so there is no per-pid answer to cache the way the unix path has.
#[cfg(windows)]
pub async fn detect_agents_for_pids_blocking(
    _agents: &AgentStore,
    pids: Vec<(String, u32)>,
) -> AppResult<Vec<service::DetectedAgentProbe>> {
    if pids.is_empty() {
        return Ok(Vec::new());
    }

    tauri::async_runtime::spawn_blocking(move || detect_agents_for_pids(pids))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

/// The session's own signals decide. Only a session that has produced no signal at all falls back
/// to the hook bridge's project-scoped override, and Claude does not even do that: its events now
/// arrive in-band per session, so a stale project-wide override must not speak for it. Codex and
/// gemini keep the fallback — their HTTP hooks are unchanged this batch.
pub fn resolve_activity(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probe: &service::DetectedAgentProbe,
) -> AgentActivity {
    let activity = agents.classify_session_activity(&probe.session_id, probe.name);
    if activity != AgentActivity::Unknown || probe.name == AGENT_NAME_CLAUDE {
        return activity;
    }
    hooks_store.fresh_project_override(project_id, probe.name).unwrap_or(activity)
}

pub fn build_detected_agents(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probes: Vec<service::DetectedAgentProbe>,
) -> Vec<DetectedAgent> {
    probes
        .into_iter()
        .map(|probe| {
            let activity = resolve_activity(agents, hooks_store, project_id, &probe);
            DetectedAgent {
                session_id: probe.session_id,
                name: probe.name.to_string(),
                pid: probe.pid,
                activity,
            }
        })
        .collect()
}

/// Feeds one scanned pty chunk into the agent signals, if that session runs an agent. Wired from
/// the terminal domain through the assembly-owned `PtySessionObservers` (lib.rs) rather than
/// called across domains (architecture.md §2).
pub(crate) fn record_session_scan(app: &tauri::AppHandle, session_id: &str, outcome: &ScanOutcome) {
    let Some(agents) = app.try_state::<AgentStore>() else {
        return;
    };
    agents.record_scan(session_id, outcome);
}

/// Same wiring, for bytes going the other way: whoever is typing into this pty is watching it.
pub(crate) fn record_session_input(app: &tauri::AppHandle, session_id: &str) {
    let Some(agents) = app.try_state::<AgentStore>() else {
        return;
    };
    agents.record_input(session_id);
}

/// The protocol handshake every spawned terminal inherits: the hook commands TAIDE writes into
/// `settings.local.json` emit their event only when `TAIDE_AGENT_PROTOCOL_VERSION` is set, so the
/// same project opened in another terminal stays silent (contract §1.4). Registered by `lib.rs`'s
/// assembly on `terminal::commands::PtySpawnEnvProvider`, after [`editor_terminal_env`].
pub fn agent_protocol_env() -> Vec<(String, String)> {
    vec![
        (AGENT_PROTOCOL_VERSION_ENV_NAME.to_string(), AGENT_PROTOCOL_VERSION.to_string()),
        (APP_VERSION_ENV_NAME.to_string(), env!("CARGO_PKG_VERSION").to_string()),
    ]
}

/// Which emitter the installed Claude hooks use, decided once per app run.
///
/// The probe is one `claude --version` on the blocking pool with a deadline, because the answer is
/// only needed when hooks are installed or reconciled and a `claude` that hangs (or is not on
/// `PATH`) must not hold that up: every failure mode resolves to `DevTty`, which works on every
/// version. Cached in a `OnceLock` so reconciling ten projects forks once, not ten times.
pub(super) async fn resolve_claude_hook_emitter() -> service::HookEmitter {
    static EMITTER: OnceLock<service::HookEmitter> = OnceLock::new();

    if let Some(cached) = EMITTER.get() {
        return *cached;
    }
    let detected = detect_claude_hook_emitter().await;
    *EMITTER.get_or_init(|| detected)
}

async fn detect_claude_hook_emitter() -> service::HookEmitter {
    let probe = tauri::async_runtime::spawn_blocking(|| std::process::Command::new(AGENT_NAME_CLAUDE).arg(CLAUDE_VERSION_FLAG).output());

    let Ok(Ok(Ok(output))) = tokio::time::timeout(Duration::from_secs(CLAUDE_VERSION_TIMEOUT_SECONDS), probe).await else {
        return service::HookEmitter::DevTty;
    };

    let supported =
        service::parse_claude_version(&String::from_utf8_lossy(&output.stdout)).is_some_and(service::supports_terminal_sequence);
    if supported {
        service::HookEmitter::TerminalSequence
    } else {
        service::HookEmitter::DevTty
    }
}

const CLAUDE_VERSION_FLAG: &str = "--version";

fn resolve_cli_install_status() -> CliInstallStatus {
    let target = Path::new(TAIDE_CLI_TARGET_PATH);
    match std::fs::symlink_metadata(target) {
        Ok(_) => {
            let resolved = std::fs::canonicalize(target).ok().map(|path| path.to_string_lossy().to_string());
            let dangling = resolved.is_none();
            service::build_cli_install_status(TAIDE_CLI_TARGET_PATH, true, resolved, dangling)
        }
        Err(_) => service::build_cli_install_status(TAIDE_CLI_TARGET_PATH, false, None, false),
    }
}

/// The `taide` CLI to point an injected `EDITOR` at: the installed `/usr/local/bin/taide` symlink
/// when it resolves to our own CLI sidecar (same ownership check `agent_cli_uninstall` applies, so
/// a same-named binary TAIDE did not install is never made every terminal's editor), otherwise the
/// CLI sidecar shipped inside the running app bundle (so ctrl+g works even before the user installs
/// the shell command). `None` for a dev build that has neither.
fn resolve_editor_cli_path() -> Option<String> {
    let status = resolve_cli_install_status();
    let is_owned_symlink = status
        .resolved_path
        .as_deref()
        .is_some_and(|resolved| service::is_cli_symlink_owned(Path::new(resolved)));
    if status.installed && !status.dangling && is_owned_symlink {
        return Some(status.target_path);
    }
    let sidecar = service::resolve_cli_install_target(&std::env::current_exe().ok()?)?;
    sidecar.exists().then(|| sidecar.to_string_lossy().to_string())
}

/// The `EDITOR`/`VISUAL` environment entries every newly spawned terminal inherits, so Claude
/// Code's ctrl+g opens its temp file in this running TAIDE and waits for the tab to close instead
/// of falling back to `vi`. Registered by `lib.rs`'s assembly on
/// `terminal::commands::PtySpawnEnvProvider` next to `ide::store::claude_terminal_env`; an
/// unresolvable CLI path yields an empty vec, never a spawn failure
/// (`docs/features/agent-integration.md` §2.3).
pub fn editor_terminal_env() -> Vec<(String, String)> {
    service::build_editor_env_entries(resolve_editor_cli_path().as_deref())
}

/// Resolves the CLI sidecar symlink target from the running app's own executable path, rejecting
/// unbundled dev builds. Shared by install (needs the target) and uninstall (only needs the gate).
#[cfg(target_os = "macos")]
fn cli_install_target() -> AppResult<PathBuf> {
    let exe = std::env::current_exe()?;
    service::resolve_cli_install_target(&exe).ok_or_else(|| {
        AppError::InvalidArgument("taide CLI shell command is only available in the installed TAIDE.app (not a dev build)".to_string())
    })
}

#[cfg(target_os = "macos")]
fn run_cli_osascript(script: &str) -> AppResult<()> {
    let args = service::build_osascript_args(script);
    let output = std::process::Command::new("osascript").args(&args).output()?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if service::is_osascript_user_cancelled(output.status.code(), &stderr) {
        return Ok(());
    }
    Err(AppError::Internal(format!("osascript failed: {stderr}")))
}

fn settings_local_path(root: &str) -> PathBuf {
    Path::new(root).join(".claude").join("settings.local.json")
}

const NEW_HOOKS_FILE_MODE: u32 = 0o600;

fn read_json_file_rejecting_invalid(path: &Path) -> AppResult<serde_json::Value> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(AppError::from(error)),
    }
}

#[cfg(unix)]
fn existing_file_mode(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).ok().map(|metadata| metadata.permissions().mode() & 0o777)
}

fn write_hooks_file_preserving_mode(path: &Path, value: &serde_json::Value) -> AppResult<()> {
    let text = serde_json::to_string_pretty(value)?;

    #[cfg(unix)]
    let mode = existing_file_mode(path).unwrap_or(NEW_HOOKS_FILE_MODE);
    #[cfg(not(unix))]
    let mode = NEW_HOOKS_FILE_MODE;

    crate::infra::persist::write_atomic_with_mode(path, text.as_bytes(), mode)
}

pub(super) fn read_settings_local(root: &str) -> AppResult<serde_json::Value> {
    read_json_file_rejecting_invalid(&settings_local_path(root))
}

pub(super) fn write_settings_local(root: &str, value: &serde_json::Value) -> AppResult<()> {
    write_hooks_file_preserving_mode(&settings_local_path(root), value)
}

pub(super) fn read_user_level_hooks(path: &Path) -> AppResult<serde_json::Value> {
    read_json_file_rejecting_invalid(path)
}

pub(super) fn write_user_level_hooks(path: &Path, value: &serde_json::Value) -> AppResult<()> {
    write_hooks_file_preserving_mode(path, value)
}

pub(super) fn home_dir_env() -> Option<String> {
    std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok()
}

fn resolve_user_level_hooks_installed(agent_name: &str) -> AppResult<bool> {
    let home = home_dir_env();
    let path = service::user_level_hooks_path(agent_name, home.as_deref())?;
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let value: serde_json::Value = serde_json::from_str(&text)?;
            Ok(service::has_taide_marker_anywhere(&value))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::from(error)),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn agent_list(
    state: State<'_, AppState>,
    terminals: State<'_, TerminalStore>,
    agents: State<'_, AgentStore>,
    agent_hooks: State<'_, AgentHooksStore>,
    project_id: ProjectId,
) -> AppResult<ProjectAgents> {
    ensure_project_open(&state, &project_id)?;

    let pids = terminals.foreground_pids(&project_id);
    let probes = detect_agents_for_pids_blocking(&agents, pids).await?;

    let detected = build_detected_agents(&agents, &agent_hooks, &project_id, probes);
    Ok(ProjectAgents {
        project_id,
        agents: detected,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn agent_release_marker(state: State<'_, AppState>, agents: State<'_, AgentStore>, marker: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let path = service::validate_wait_marker_path(&marker, &std::env::temp_dir())?;

    let result = match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    };
    agents.forget_wait_marker(&marker);
    result
}

#[tauri::command]
#[specta::specta]
pub async fn agent_cli_status() -> AppResult<CliInstallStatus> {
    Ok(resolve_cli_install_status())
}

#[cfg(target_os = "macos")]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_install() -> AppResult<CliInstallStatus> {
    let target = cli_install_target()?;
    let source = Path::new(TAIDE_CLI_TARGET_PATH);

    let already_linked = std::fs::read_link(source).is_ok_and(|existing| existing == target);
    if !already_linked {
        run_cli_osascript(&service::build_cli_install_apple_script(&target, source))?;
    }

    Ok(resolve_cli_install_status())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_install() -> AppResult<CliInstallStatus> {
    Err(AppError::InvalidArgument(
        "taide CLI shell command install is only supported on macOS".to_string(),
    ))
}

#[cfg(target_os = "macos")]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_uninstall() -> AppResult<CliInstallStatus> {
    cli_install_target()?;
    let source = Path::new(TAIDE_CLI_TARGET_PATH);

    match std::fs::symlink_metadata(source) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(resolve_cli_install_status()),
        Err(error) => return Err(AppError::from(error)),
    }

    let owned = std::fs::read_link(source).is_ok_and(|link_target| service::is_cli_symlink_owned(&link_target));
    if !owned {
        return Err(AppError::InvalidArgument(
            "the existing 'taide' command is not managed by TAIDE and was not removed".to_string(),
        ));
    }

    match std::fs::remove_file(source) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            run_cli_osascript(&service::build_cli_uninstall_apple_script(source))?;
        }
        Err(error) => return Err(AppError::from(error)),
    }

    Ok(resolve_cli_install_status())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
#[specta::specta]
pub async fn agent_cli_uninstall() -> AppResult<CliInstallStatus> {
    Err(AppError::InvalidArgument(
        "taide CLI shell command uninstall is only supported on macOS".to_string(),
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn agent_pending_external_opens(agents: State<'_, AgentStore>) -> AppResult<Vec<ExternalOpenRequest>> {
    Ok(agents.drain_pending_external_opens())
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_status(state: State<'_, AppState>, project_id: ProjectId, agent_name: String) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    let installed = match scope {
        HookInstallScope::Project => {
            let root = project_root(&state, &project_id)?;
            let value = read_settings_local(&root)?;
            service::has_taide_hook_entries(&value)
        }
        HookInstallScope::User => resolve_user_level_hooks_installed(&agent_name)?,
    };
    Ok(AgentHooksStatus {
        agent_name,
        scope,
        installed,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_install(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    agent_name: String,
) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    if !state.settings.read().agent_hooks_enabled {
        return Err(AppError::InvalidArgument("agent hooks are disabled in settings".to_string()));
    }
    match scope {
        HookInstallScope::Project => {
            let root = project_root(&state, &project_id)?;
            let emitter = resolve_claude_hook_emitter().await;
            let value = read_settings_local(&root)?;
            let value = service::inject_taide_claude_command_hook_entries(value, emitter);
            write_settings_local(&root, &value)?;
            Ok(AgentHooksStatus {
                agent_name,
                scope,
                installed: true,
            })
        }
        HookInstallScope::User => {
            if !resolve_cli_install_status().installed {
                return Err(AppError::InvalidArgument("taide CLI is not installed yet".to_string()));
            }
            let server = hooks::ensure_hooks_server_started(&app).await?;
            let hook_url = hooks::build_hook_url(&server, &agent_name);
            let command = service::build_command_hook_shell_command(TAIDE_CLI_TARGET_PATH, &hook_url);
            let events = service::managed_hook_events_for(&agent_name);
            let timeout = service::user_level_hook_command_timeout(&agent_name);
            let path = service::user_level_hooks_path(&agent_name, home_dir_env().as_deref())?;
            let value = read_user_level_hooks(&path)?;
            let value = service::inject_taide_command_hook_entries(value, events, &command, timeout);
            write_user_level_hooks(&path, &value)?;
            Ok(AgentHooksStatus {
                agent_name,
                scope,
                installed: true,
            })
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn agent_hooks_uninstall(state: State<'_, AppState>, project_id: ProjectId, agent_name: String) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    match scope {
        HookInstallScope::Project => {
            let root = project_root(&state, &project_id)?;
            let value = read_settings_local(&root)?;
            if service::has_taide_hook_entries(&value) {
                let value = service::remove_taide_hook_entries(value);
                write_settings_local(&root, &value)?;
            }
        }
        HookInstallScope::User => {
            let path = service::user_level_hooks_path(&agent_name, home_dir_env().as_deref())?;
            let value = read_user_level_hooks(&path)?;
            if service::has_taide_marker_anywhere(&value) {
                let events = service::managed_hook_events_for(&agent_name);
                let value = service::remove_taide_command_hook_entries(value, events);
                write_user_level_hooks(&path, &value)?;
            }
        }
    }
    Ok(AgentHooksStatus {
        agent_name,
        scope,
        installed: false,
    })
}

pub(crate) async fn poll_agents(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let terminals = app.state::<TerminalStore>();
    let agents = app.state::<AgentStore>();
    let agent_hooks = app.state::<AgentHooksStore>();

    let project_ids: Vec<_> = state.projects.read().keys().cloned().collect();
    let mut valid_session_ids = HashSet::new();
    let mut live_pids = HashSet::new();

    for project_id in project_ids {
        let pids = terminals.foreground_pids(&project_id);
        live_pids.extend(pids.iter().map(|(_, pid)| *pid));

        let Ok(probes) = detect_agents_for_pids_blocking(&agents, pids).await else {
            continue;
        };
        let detected = build_detected_agents(&agents, &agent_hooks, &project_id, probes);
        valid_session_ids.extend(detected.iter().map(|agent| agent.session_id.clone()));

        if let Some(changed) = agents.diff(&project_id, &detected) {
            let _ = AgentStateChanged {
                project_id: project_id.clone(),
                agents: changed,
            }
            .emit(app);
        }
    }

    agents.prune_signals(&valid_session_ids);
    agents.retain_process_names(&live_pids);
}

/// Queues one CLI-originated open request — the cold-start argv below or a
/// `tauri-plugin-single-instance` relay (`lib.rs`), the only two places a path ever enters
/// `AppState::authorize_cli_opened_path`'s allowlist: the wait marker is registered for exit-time
/// cleanup, the path is let past the open-project boundary (Claude Code's Ctrl+G temp file lives
/// under the OS tmpdir, outside every root), and the request is queued for the frontend to drain
/// (`agent_pending_external_opens`). Emitting `AgentExternalOpen` stays with the single-instance
/// caller; a cold start has no subscribed frontend yet and relies on the boot-time drain alone.
pub(crate) fn queue_external_open(app_handle: &tauri::AppHandle, request: ExternalOpenRequest) {
    let agent_store = app_handle.state::<AgentStore>();
    if let Some(marker) = request.wait_marker.clone() {
        agent_store.register_wait_marker(marker);
    }
    app_handle.state::<AppState>().authorize_cli_opened_path(Path::new(&request.path));
    agent_store.push_pending_external_open(request);
}

/// Handles a cold-start `taide <file>` invocation. `tauri-plugin-single-instance` only forwards
/// argv to a *second* launch's callback, so a cold start that spawns the app fresh never reaches
/// it; this queues the request into `AgentStore` instead so the frontend can drain it on boot.
pub(crate) fn queue_cold_start_external_open(app_handle: &tauri::AppHandle) {
    let argv: Vec<String> = std::env::args().collect();
    let Some(request) = service::parse_cli_payload(&argv) else {
        return;
    };

    queue_external_open(app_handle, request);
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn temp_project_root(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("taide-agent-hooks-file-safety-{name}-{}", Uuid::new_v4()))
            .to_string_lossy()
            .to_string()
    }

    #[cfg(unix)]
    #[test]
    fn 프로세스_조회는_한_번의_ps_호출로_살아있는_pid를_해석한다() {
        let own = std::process::id();

        let infos = resolve_process_infos(&[own, own]);

        let info = infos.get(&own).expect("테스트 프로세스 자신의 pid 는 항상 살아 있다");
        assert!(!info.comm.is_empty());
        assert!(!info.cmdline.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn 조회할_pid가_없으면_ps를_호출하지_않는다() {
        assert!(resolve_process_infos(&[]).is_empty());
        assert!(resolve_agent_names(&[]).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn 에이전트가_아닌_pid도_이름_캐시에_남겨_다음_틱에_다시_묻지_않는다() {
        let own = std::process::id();
        let store = AgentStore::new();
        let pids = vec![("session-1".to_string(), own)];

        assert_eq!(store.unresolved_pids(&pids), vec![own]);
        store.remember_process_names(resolve_agent_names(&[own]));

        assert!(store.unresolved_pids(&pids).is_empty(), "에이전트가 아니라는 답도 캐시된 답이다");
        assert!(store.probes_for(pids).is_empty(), "테스트 러너는 에이전트가 아니다");

        store.retain_process_names(&HashSet::new());
        assert_eq!(
            store.unresolved_pids(&[("session-1".to_string(), own)]),
            vec![own],
            "전경 pid 집합을 벗어난 항목은 축출된다"
        );
    }

    #[test]
    fn 세션의_에이전트가_바뀌면_이전_에이전트의_신호를_버린다() {
        use crate::domain::agent::types::AGENT_NAME_CODEX;

        let store = AgentStore::new();
        let dialog = ScanOutcome {
            events: Vec::new(),
            text: "Do you want to proceed?".to_string(),
            overlap: String::new(),
        };

        store.classify_session_activity("session-1", AGENT_NAME_CLAUDE);
        store.record_scan("session-1", &dialog);
        assert_eq!(
            store.classify_session_activity("session-1", AGENT_NAME_CLAUDE),
            AgentActivity::AwaitingInput
        );

        assert_eq!(
            store.classify_session_activity("session-1", AGENT_NAME_CODEX),
            AgentActivity::Unknown,
            "에이전트가 바뀐 세션에 이전 에이전트의 다이얼로그 래치가 남으면 안 된다"
        );
    }

    #[test]
    fn 비_json_파일은_읽기_실패로_원본을_보존한다() {
        let root = temp_project_root("invalid-json-preserved");
        let path = settings_local_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ not json").unwrap();

        let result = read_settings_local(&root);

        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{ not json");

        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn 기존_파일의_권한을_보존해서_재작성한다() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_project_root("preserve-mode");
        let path = settings_local_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{}").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        write_settings_local(&root, &serde_json::json!({ "hooks": {} })).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644);

        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn 신규_파일은_0600_권한으로_생성된다() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_project_root("new-file-mode");
        let path = settings_local_path(&root);

        write_settings_local(&root, &serde_json::json!({ "hooks": {} })).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, NEW_HOOKS_FILE_MODE);

        std::fs::remove_dir_all(&root).ok();
    }
}

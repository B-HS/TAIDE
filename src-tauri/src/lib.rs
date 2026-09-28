pub mod constants;
pub mod domain;
pub mod error;
pub mod events;
pub mod ids;
pub mod infra;
pub mod paths;
pub mod platform;
mod plugin_port;
mod project_restore_port;
mod remote_gateway;
mod settings_port;
pub mod state;

use std::path::Path;
use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde_json::Value;
use taide_infra::language::LanguageOverlay;
use taide_model::app_event::AppEvent;
use taide_model::plugin::LoadedPlugin;
use taide_runtime::{layout_actions, AppServices, EventSink, ExitDrain, PlatformServicesState, TaskSupervisor};
use tauri::{AppHandle, Listener, Manager, State};
use tauri_specta::Event as _;
use tauri_specta::{collect_commands, collect_events, Builder};

use crate::domain::agent::commands::{AgentForegroundPids, AgentStore};
use crate::domain::ide::commands::IdeSaveFile;
use crate::domain::ide::server::IdeLayoutActions;
use crate::domain::ide::store::IdeStore;
use crate::domain::layout::service as layout_service;
use crate::domain::layout::types::{ProjectLayout, Tab, TabKind, TabWindowTarget};
use crate::domain::lsp::commands::{LspInstallStore, LspStore};
use crate::domain::plugin::service::PluginStore;
use crate::domain::remote::commands::{RemoteDispatchLimiter, RemoteStore};
use crate::domain::remote::dispatch::{ChannelFactory, RemoteDispatchPort};
use crate::domain::remote::types::REMOTE_DISPATCH_MAX_CONCURRENT;
use crate::domain::settings::types::Settings;
use crate::domain::terminal::commands::TerminalStore;
use crate::domain::window::commands::open_auxiliary_window;
use crate::domain::window::menu::MenuSources;
use crate::error::AppResult;
use crate::events::{
    AgentExternalOpen, AgentStateChanged, FsChanged, FsRescanRequired, GitRefsChanged, GitStatusChanged, HotExitFlushRequested,
    IdeCloseTabRequested, IdeDiffRequested, IdeSaveRequested, IdeStatusChanged, LayoutChanged, LspInstallProgress, LspSessionStatusChanged,
    ProjectActivated, ProjectClosed, ProjectGroupsChanged, ProjectListChanged, ProjectOpened, ProjectRecentCleared, RemoteStateChanged,
    SessionShellSlotsChanged, SettingsChanged, SyncStateChanged, TerminalCommandFinished, TerminalCwdChanged, TerminalExited,
    TerminalSpawned, ThemeChanged, WindowChromeChanged,
};
use crate::ids::{ProjectId, TabId};
use crate::infra::secret::SecretStoreState;
use crate::paths::AppPaths;
use crate::platform::event_sink::TauriEventSink;
use crate::platform::services::TauriPlatformServices;
use crate::platform::window_registry::WindowRegistry;
use crate::plugin_port::PluginRuntimePort;
use crate::project_restore_port::ProjectRestoreWatchers;
use crate::settings_port::SettingsApplyPort;
use crate::state::AppState;

#[cfg(any(debug_assertions, test))]
const BINDINGS_PATH: &str = "../src/shared/api/bindings.ts";

/// Statically assembles the [`domain::project::capability::ProjectCapability`] implementations
/// that `project_open`/`project_close` walk. **This list's order is the correctness contract, not
/// a style choice** — `ProjectCapabilities` walks it forward for both attach and detach, and the
/// order below reproduces exactly the sequence `project/commands.rs` used to hand-code:
///
/// - attach: layout load → file watcher → git watcher → IDE lockfile refresh → agent hooks
///   reconcile (spawned).
/// - detach: dirty-layout flush + removal **first** (before anything else observes the project as
///   gone — see `LayoutCapability::detach` for why the flush must precede the removal), then file
///   and git watcher removal, then the terminal pty reap, then git/tree cache eviction, then the
///   IDE lockfile refresh. `GitCacheCapability` is registered separately from
///   `GitWatcherCapability` purely so this single forward walk keeps the cache eviction after the
///   terminal reap, where the hand-coded sequence had it.
///
/// Pinned by the source-scan order test below (`T1-K` parity-test precedent).
fn project_capabilities() -> domain::project::capability::ProjectCapabilities {
    domain::project::capability::ProjectCapabilities::new(vec![
        Box::new(domain::layout::capability::LayoutCapability),
        Box::new(domain::file::capability::FileWatcherCapability),
        Box::new(domain::git::capability::GitWatcherCapability),
        Box::new(domain::terminal::capability::TerminalCapability),
        Box::new(domain::git::capability::GitCacheCapability),
        Box::new(domain::tree::capability::TreeCacheCapability),
        Box::new(domain::ide::capability::IdeLockfileCapability),
        Box::new(domain::agent::capability::AgentHooksCapability),
    ])
}

/// Assembles the integration reactions `settings::commands::apply_and_broadcast` runs when a
/// settings write flips a toggle — the assembly-owned wiring that replaced the settings domain's
/// direct ide/agent/remote calls (audit R5#6, T1-I §1.4). Registration order is the execution
/// order and reproduces the old hand-coded sequence: IDE server → agent hooks → remote access.
fn settings_toggle_observers() -> domain::settings::commands::SettingsToggleObservers {
    use domain::settings::commands::SettingsToggleFuture;

    domain::settings::commands::SettingsToggleObservers::new(vec![
        Box::new(|app, current, updated| -> SettingsToggleFuture<'_> {
            Box::pin(domain::ide::commands::apply_ide_integration_toggle(
                app,
                current.ide_integration_enabled,
                updated.ide_integration_enabled,
            ))
        }),
        Box::new(|app, current, updated| -> SettingsToggleFuture<'_> {
            Box::pin(domain::agent::hooks::apply_agent_hooks_toggle(
                app,
                current.agent_hooks_enabled,
                updated.agent_hooks_enabled,
            ))
        }),
        Box::new(|app, current, updated| -> SettingsToggleFuture<'_> {
            Box::pin(domain::remote::commands::apply_remote_access_toggle(
                app,
                current.remote_access_enabled,
                updated.remote_access_enabled,
            ))
        }),
    ])
}

fn apply_settings_from_port<'a>(app: &'a AppHandle, state: &'a AppState, next: Settings) -> BoxFuture<'a, AppResult<Settings>> {
    Box::pin(domain::settings::commands::apply_and_broadcast(app, state, next))
}

fn plugin_language_overlays(app: &AppHandle) -> Vec<LanguageOverlay> {
    let state = app.state::<AppState>();
    let store = app.state::<PluginStore>();
    let loaded = taide_plugin::service::ensure_loaded(&store, &state.paths.plugins_dir());
    taide_plugin::service::language_overlays(&loaded)
}

fn commit_staged_vsix_plugin(app: &AppHandle, temp_dir: &Path, staged_plugin_id: &str) -> AppResult<LoadedPlugin> {
    taide_runtime::plugin_actions::commit_staged_vsix_plugin(
        &app.state::<AppState>(),
        &app.state::<PluginStore>(),
        temp_dir,
        staged_plugin_id,
    )
}

fn project_restore_watchers() -> ProjectRestoreWatchers {
    ProjectRestoreWatchers {
        build_file: domain::file::capability::build_watcher_handle,
        build_git: domain::git::watch::build_git_watcher_handle,
        register_file: domain::file::capability::register_watcher_handle,
        register_git: domain::git::watch::register_git_watcher_handle,
    }
}

/// Assembles the pid → (kind, label) providers `system_usage_breakdown` consults, one closure per
/// owning domain, so `domain::system` never reads the terminal/agent/LSP stores directly (audit
/// R8#9, T1-I §1.4). Registration order is precedence: later providers overwrite earlier ones for
/// the same pid, so an agent detected inside a terminal's foreground pid set labels as Agent and
/// LSP labels apply last. Two deliberate differences from the old hand-coded collection: each
/// provider takes its own `state.projects` read snapshot (a project opened or closed between
/// providers can appear in one provider's map and not another's), and a cross-project pid
/// collision now always resolves by registration order where the old per-project interleave left
/// it to `HashMap` iteration order — a determinization, not a preserved artifact.
fn system_usage_label_providers() -> domain::system::commands::SystemUsageLabelProviders {
    use domain::system::types::SystemUsageProcessKind;

    domain::system::commands::SystemUsageLabelProviders::new(vec![
        Box::new(|app| {
            let state = app.state::<AppState>();
            let terminals = app.state::<TerminalStore>();
            let projects = state.projects.read();
            let mut labels = domain::system::commands::SystemUsageLabels::new();
            for (project_id, project) in projects.iter() {
                for (_, pid) in terminals.foreground_pids(project_id) {
                    labels.insert(pid, (SystemUsageProcessKind::Terminal, project.name.clone()));
                }
            }
            labels
        }),
        Box::new(|app| {
            let state = app.state::<AppState>();
            let agents = app.state::<AgentStore>();
            let projects = state.projects.read();
            let mut labels = domain::system::commands::SystemUsageLabels::new();
            for (project_id, project) in projects.iter() {
                for agent in agents.agents_for(project_id) {
                    labels.insert(
                        agent.pid,
                        (SystemUsageProcessKind::Agent, format!("{} · {}", agent.name, project.name)),
                    );
                }
            }
            labels
        }),
        Box::new(|app| {
            let server_pids = app.state::<LspStore>().server_pids();
            let state = app.state::<AppState>();
            let projects = state.projects.read();
            let mut labels = domain::system::commands::SystemUsageLabels::new();
            for (project_id, server_name, pid) in server_pids {
                let label = projects
                    .get(&project_id)
                    .map(|project| format!("{server_name} · {}", project.name))
                    .unwrap_or(server_name);
                labels.insert(pid, (SystemUsageProcessKind::Lsp, label));
            }
            labels
        }),
    ])
}

/// Wires `pty_spawn`'s extra-env hook to the three agent integrations that must be in a terminal's
/// environment before the shell starts: the Claude Code SSE port when the IDE server is (or comes)
/// up — `ide::store::claude_terminal_env` owns the readiness wait, the terminal domain only injects
/// the result (audit R8#10, T1-I §1.4) — `EDITOR` pointing at the `taide` CLI so ctrl+g opens
/// files in this window (`agent::commands::editor_terminal_env`, agent-integration.md §2.3), and
/// the in-band agent protocol handshake the installed hooks gate on
/// (`agent::commands::agent_protocol_env`). Each domain owns its own resolution; this assembly only
/// concatenates what they hand back.
fn pty_spawn_env_provider() -> domain::terminal::commands::PtySpawnEnvProvider {
    use domain::terminal::commands::PtySpawnEnvFuture;

    domain::terminal::commands::PtySpawnEnvProvider::new(Box::new(|app| -> PtySpawnEnvFuture<'_> {
        Box::pin(async move {
            let mut env = domain::ide::store::claude_terminal_env(app).await;
            env.extend(domain::agent::commands::editor_terminal_env());
            env.extend(domain::agent::commands::agent_protocol_env());
            env
        })
    }))
}

/// Wires a pty session's own traffic to the agent domain's activity signals: every scanned output
/// chunk and every user write reach `agent::commands`, which keeps them only for sessions that
/// actually run an agent. Assembly-owned for the same reason as the env provider above — the
/// terminal domain must not call into agent, which already reads terminal's foreground pid set
/// (architecture.md §2).
fn pty_session_observers() -> domain::terminal::commands::PtySessionObservers {
    use domain::terminal::commands::PtySessionSignal;

    domain::terminal::commands::PtySessionObservers::new(vec![Box::new(|app, session_id, signal| match signal {
        PtySessionSignal::Output(outcome) => domain::agent::commands::record_session_scan(app, session_id, outcome),
        PtySessionSignal::Input => domain::agent::commands::record_session_input(app, session_id),
    })])
}

fn open_ide_file_tab(
    app: AppHandle,
    project_id: ProjectId,
    path: String,
    title: String,
    preview: bool,
) -> BoxFuture<'static, AppResult<()>> {
    Box::pin(async move {
        let state = app.state::<AppState>();
        layout_service::open_tab_and_finish(&app, &state, project_id, TabKind::File { path }, title, None, preview)
            .await
            .map(|_| ())
    })
}

fn close_ide_tab(app: AppHandle, tab_id: TabId) -> BoxFuture<'static, AppResult<Tab>> {
    Box::pin(async move {
        let state = app.state::<AppState>();
        layout_service::close_tab_and_finish(&app, &state, &tab_id)
            .await
            .map(|(_, closed_tab, _)| closed_tab.tab)
    })
}

fn ide_layout_actions() -> IdeLayoutActions {
    IdeLayoutActions {
        open_file_tab: open_ide_file_tab,
        close_tab: close_ide_tab,
    }
}

fn save_ide_diff_file(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    domain::file::service::save_file_within_open_projects(state, path, content)
}

fn dispatch_remote_json(app: AppHandle, name: String, args: Value, channels: ChannelFactory) -> BoxFuture<'static, Result<String, Value>> {
    Box::pin(async move { remote_gateway::dispatch(&app, &name, args, channels).await })
}

fn dispatch_remote_raw(app: AppHandle, name: String, args: Value) -> BoxFuture<'static, Result<Vec<u8>, Value>> {
    Box::pin(async move { remote_gateway::dispatch_raw(&app, &name, args).await })
}

fn remote_dispatch_port() -> RemoteDispatchPort {
    RemoteDispatchPort {
        json: dispatch_remote_json,
        raw: dispatch_remote_raw,
    }
}

fn recent_menu_projects(app: &AppHandle) -> Vec<domain::project::types::Project> {
    let state = app.state::<AppState>();
    match domain::project::service::list_recent_projects(&state.paths) {
        Ok(projects) => projects,
        Err(error) => {
            log::warn!("최근 프로젝트 목록을 읽지 못해 메뉴를 비웁니다: {error}");
            Vec::new()
        }
    }
}

fn menu_label(app: &AppHandle, key: &str, fallback: &str) -> String {
    let language = app.state::<AppState>().settings.read().language.clone();
    let locale_id = domain::locale::service::builtin_locale_for_language(&language);
    domain::locale::service::lookup_builtin_message(locale_id, key)
        .or_else(|| domain::locale::service::lookup_builtin_message(domain::locale::service::BUILTIN_EN_ID, key))
        .unwrap_or_else(|| fallback.to_string())
}

fn menu_sources() -> MenuSources {
    MenuSources {
        recent_projects: recent_menu_projects,
        label: menu_label,
    }
}

fn foreground_pids_for_agent(app: &AppHandle, project_id: &ProjectId) -> Vec<(String, u32)> {
    app.state::<TerminalStore>().foreground_pids(project_id)
}

fn layout_tab_closed_observers() -> domain::layout::service::LayoutTabClosedObservers {
    domain::layout::service::LayoutTabClosedObservers::new(vec![
        Box::new(|app, tab| app.state::<IdeStore>().reconcile_closed_tab(tab)),
        Box::new(|app, tab| {
            if let domain::layout::types::TabKind::Terminal { session_id, .. } = &tab.kind {
                app.state::<TerminalStore>().kill_session(session_id);
            }
        }),
    ])
}

/// Moves a tab to the main window, an existing auxiliary window, or a new OS window.
/// For a new window, the OS window is opened before changing the layout and closed if the move fails.
/// Empty auxiliary windows are cleaned up after a successful move.
#[tauri::command]
#[specta::specta]
async fn layout_move_tab_to_window(
    app: AppHandle,
    state: State<'_, AppState>,
    windows: State<'_, WindowRegistry>,
    tab_id: TabId,
    target: TabWindowTarget,
) -> AppResult<ProjectLayout> {
    layout_actions::layout_move_tab_to_window(
        &TauriEventSink(&app),
        &state,
        &windows,
        tab_id,
        target,
        |project_id, slot| {
            let app = &app;
            let state = &state;
            let windows = &windows;
            async move {
                let info = open_auxiliary_window(app, state, windows, project_id, slot).await?;
                Ok(info.label)
            }
        },
        |label| {
            if let Some(webview_window) = app.get_webview_window(label) {
                let _ = webview_window.close();
            }
        },
    )
    .await
}

fn plan_return_of_auxiliary_window_tabs(app: &AppHandle, project_id: &ProjectId, window_slot: u32) {
    let tasks = (*app.state::<TaskSupervisor>()).clone();
    let app = app.clone();
    let project_id = project_id.clone();
    tasks.spawn_transient("auxiliary-tab-return", async move {
        let state = app.state::<AppState>();
        layout_actions::return_auxiliary_window_tabs(&TauriEventSink(&app), &state, project_id, window_slot).await;
    });
}

/// Routes one app-menu click to the domain that owns the action it stands for — the assembly's
/// half of the native menu. It lives here, not in `domain::window`, so the window domain never
/// calls `project::commands` itself (architecture.md §2; `tests/domain_boundaries.rs` enforces it),
/// the same shape as [`settings_toggle_observers`] and [`project_capabilities`].
///
/// Everything but Quit runs on a spawned task. This handler is called on the main thread, where
/// blocking on `project_open`'s mutation guard would stall the very event loop that open needs, and
/// where neither the recent list's disk read nor a project open belongs. The clicked row's root is
/// re-read at dispatch time (`recent_project_root`) rather than carried in the menu item: a
/// menu built minutes ago can name a project `Clear Recent` has since forgotten, and re-reading
/// turns that into a logged no-op instead of an open against a stale path.
///
/// The project commands are called directly rather than by asking a window to invoke IPC, so the
/// menu keeps working with **zero windows open** — the state macOS leaves the app in after the last
/// window closes, and the exact state in which "reopen a recent project" is most useful.
fn dispatch_menu_action(app: &tauri::AppHandle, action: domain::window::menu::MenuAction) {
    use domain::window::menu::MenuAction;

    match action {
        MenuAction::Quit => domain::window::commands::request_quit(app),
        MenuAction::ClearRecent => {
            let tasks = (*app.state::<TaskSupervisor>()).clone();
            let app = app.clone();
            tasks.spawn_transient("menu-clear-recent", async move {
                let state = app.state::<AppState>();
                if let Err(error) = domain::project::commands::project_forget_recent(app.clone(), state).await {
                    log::warn!("최근 항목 지우기에 실패했습니다: {error}");
                }
            });
        }
        MenuAction::OpenRecent(project_id) => {
            let tasks = (*app.state::<TaskSupervisor>()).clone();
            let app = app.clone();
            tasks.spawn_transient("menu-open-recent", async move {
                let Some(root) = recent_project_root(&app, &project_id) else {
                    log::warn!("최근 항목 메뉴가 가리키는 프로젝트 레코드를 찾지 못했습니다 (projectId={project_id})");
                    return;
                };
                let state = app.state::<AppState>();
                if let Err(error) = domain::project::commands::project_open(app.clone(), state, app.state(), root).await {
                    log::warn!("최근 항목 메뉴에서 프로젝트를 열지 못했습니다 (projectId={project_id}): {error}");
                }
            });
        }
        MenuAction::Ignored => {}
    }
}

fn recent_project_root(app: &AppHandle, project_id: &ProjectId) -> Option<String> {
    let state = app.state::<AppState>();
    domain::project::service::list_recent_projects(&state.paths)
        .ok()?
        .into_iter()
        .find(|project| &project.id == project_id)
        .map(|project| project.root)
}

fn schedule_menu_refresh(app: &AppHandle, name: &'static str, refresh: fn(&AppHandle)) {
    let tasks = (*app.state::<TaskSupervisor>()).clone();
    let handle = app.clone();
    let Some(refresh) = tasks.spawn_blocking_transient_handle(name, move || refresh(&handle)) else {
        return;
    };
    tasks.spawn_transient("menu-refresh-completion", async move {
        if let Err(error) = refresh.await {
            log::warn!("메뉴 갱신 작업이 완료되지 못했습니다: {error}");
        }
    });
}

/// Keeps the native app menu in step with the state it draws, by **subscribing** to the events the
/// project and settings domains already emit instead of having those domains call into
/// `domain::window` (architecture.md §2 — the assembly owns cross-domain wiring; the same
/// `listen_any` shape as `fanout_remote_events!`).
///
/// - `project:list-changed`/`project:activated` — every open, close, activation and
///   `project_forget_recent` changes which projects `File > Open Recent` should list, or their
///   order.
/// - `settings:changed` — a language change has to rebuild the **whole** menu, because a submenu's
///   title (`File`, `Open Recent`) is fixed when the submenu is built. The event carries the new
///   settings but not the old, so the listener remembers the language it last drew and rebuilds
///   only when that actually changed; a settings write that leaves the language alone (every
///   toggle, every editor preference) must not rebuild the menu.
///
/// Each reaction is handed to `spawn_blocking`: both rebuilds re-read the on-disk project history,
/// and a listener runs inline on the thread that emitted the event — which for
/// `project_close`/`project_activate` is still inside `AppState::begin_mutation`, where
/// architecture.md §2.1 forbids IO.
fn listen_for_app_menu_refresh(app: &tauri::AppHandle) {
    for event_name in [ProjectListChanged::NAME, ProjectActivated::NAME] {
        let recent_handle = app.clone();
        app.listen_any(event_name, move |_| {
            schedule_menu_refresh(&recent_handle, "menu-recent-refresh", domain::window::menu::refresh_recent_menu);
        });
    }

    let language_handle = app.clone();
    let drawn_language = parking_lot::Mutex::new(app.state::<AppState>().settings.read().language.clone());
    app.listen_any(SettingsChanged::NAME, move |event| {
        let Ok(changed) = serde_json::from_str::<SettingsChanged>(event.payload()) else {
            log::warn!("settings:changed 페이로드를 읽지 못해 메뉴 언어를 갱신하지 못했습니다");
            return;
        };

        let mut drawn = drawn_language.lock();
        if *drawn == changed.settings.language {
            return;
        }
        *drawn = changed.settings.language;
        drop(drawn);

        schedule_menu_refresh(
            &language_handle,
            "menu-language-refresh",
            domain::window::commands::refresh_app_menu,
        );
    });
}

/// `pty_spawn`/`pty_attach` take a `Channel` argument, so they are registered on the raw handler
/// (`RAW_CHANNEL_COMMANDS`) rather than in `collect_commands!`. Nothing collected here therefore
/// mentions [`domain::terminal::types::PtyAttachResult`], and specta would not export it; `.typ`
/// registers it explicitly so the renderer's hand-written `pty_attach` invoke keeps deriving its
/// result type from this definition instead of restating it.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            domain::app::commands::app_get_info,
            domain::project::commands::project_list,
            domain::project::commands::project_list_recent,
            domain::project::commands::project_forget_recent,
            domain::project::commands::project_get,
            domain::project::commands::project_get_active,
            domain::project::commands::project_open,
            domain::project::commands::project_close,
            domain::project::commands::project_activate,
            domain::project::commands::project_reorder,
            domain::project::commands::project_set_display,
            domain::project::commands::project_group_list,
            domain::project::commands::project_group_create,
            domain::project::commands::project_group_rename,
            domain::project::commands::project_group_set_color,
            domain::project::commands::project_group_set_collapsed,
            domain::project::commands::project_group_set_members,
            domain::project::commands::project_group_delete,
            domain::project::commands::project_group_reorder,
            domain::project::commands::project_group_open,
            domain::project::commands::project_open_in_slot,
            domain::project::commands::shell_slot_close,
            domain::project::commands::session_get_shell_state,
            domain::project::commands::session_focus_shell_slot,
            domain::project::commands::session_set_shell_slot_sizes,
            domain::project::commands::session_set_window_chrome,
            domain::layout::commands::layout_get,
            domain::layout::commands::layout_open_tab,
            domain::layout::commands::layout_close_tab,
            domain::layout::commands::layout_activate_tab,
            domain::layout::commands::layout_move_tab,
            domain::layout::commands::layout_split,
            domain::layout::commands::layout_open_tab_in_split,
            domain::layout::commands::layout_resize,
            domain::layout::commands::layout_focus_pane,
            domain::layout::commands::layout_pin_tab,
            domain::layout::commands::layout_set_preview,
            domain::layout::commands::layout_reopen_closed,
            domain::layout::commands::layout_set_view_state,
            domain::layout::commands::layout_set_dirty,
            domain::layout::commands::layout_set_terminal_session,
            domain::layout::commands::layout_open_untitled,
            domain::layout::commands::layout_convert_untitled,
            layout_move_tab_to_window,
            domain::layout::commands::layout_apply_path_change,
            domain::layout::commands::layout_set_shell_view,
            domain::file::commands::file_open,
            domain::file::commands::file_save,
            domain::file::commands::file_create,
            domain::file::commands::file_rename,
            domain::file::commands::file_delete,
            domain::file::commands::file_copy,
            domain::file::commands::file_mirror_dirty,
            domain::file::commands::file_list_mirrors,
            domain::file::commands::file_clear_mirror,
            domain::file::commands::file_prune_mirrors,
            domain::file::commands::file_mirror_untitled,
            domain::file::commands::file_list_untitled_mirrors,
            domain::file::commands::file_clear_untitled_mirror,
            domain::file::commands::file_prune_untitled_mirrors,
            domain::file::commands::file_flush_complete,
            domain::tree::commands::tree_rows,
            domain::tree::commands::tree_toggle,
            domain::tree::commands::tree_collapse_all,
            domain::tree::commands::tree_reveal,
            domain::tree::commands::tree_refresh,
            domain::search::commands::search_run,
            domain::search::commands::search_cancel,
            domain::search::commands::search_replace,
            domain::search::commands::search_list_files,
            domain::plugin::commands::plugin_list,
            domain::plugin::commands::plugin_reload,
            domain::plugin::commands::plugin_read_grammar,
            domain::plugin::commands::plugin_install,
            domain::plugin::commands::plugin_uninstall,
            domain::agent::commands::agent_list,
            domain::agent::commands::agent_release_marker,
            domain::agent::commands::agent_cli_status,
            domain::agent::commands::agent_hooks_status,
            domain::agent::commands::agent_hooks_install,
            domain::agent::commands::agent_hooks_uninstall,
            domain::agent::commands::agent_cli_install,
            domain::agent::commands::agent_cli_uninstall,
            domain::agent::commands::agent_pending_external_opens,
            domain::lsp::commands::lsp_spawn,
            domain::lsp::commands::lsp_send,
            domain::lsp::commands::lsp_stop,
            domain::lsp::commands::lsp_restart,
            domain::lsp::commands::lsp_confirm_reinitialize,
            domain::lsp::commands::lsp_report_reinitialize_failure,
            domain::lsp::commands::lsp_sessions,
            domain::lsp::commands::lsp_detect_servers,
            domain::lsp::commands::lsp_resolve_root,
            domain::lsp::commands::lsp_install,
            domain::lsp::commands::lsp_install_cancel,
            domain::git::commands::git_init,
            domain::git::commands::git_status,
            domain::git::commands::git_diff_file,
            domain::git::commands::git_diff_staged_text,
            domain::git::commands::git_show_file,
            domain::git::commands::git_log,
            domain::git::commands::git_ahead_behind,
            domain::git::commands::git_remotes,
            domain::git::commands::git_gutter,
            domain::git::commands::git_blame_range,
            domain::git::commands::git_stage,
            domain::git::commands::git_unstage,
            domain::git::commands::git_discard,
            domain::git::commands::git_commit,
            domain::git::commands::git_push,
            domain::git::commands::git_pull,
            domain::git::commands::git_fetch,
            domain::git::commands::git_undo_last_commit,
            domain::git::commands::git_branches,
            domain::git::commands::git_branch_create,
            domain::git::commands::git_branch_checkout,
            domain::git::commands::git_branch_delete,
            domain::git::commands::git_stash_list,
            domain::git::commands::git_stash_push,
            domain::git::commands::git_stash_apply,
            domain::git::commands::git_stash_drop,
            domain::git::commands::git_discard_hunk,
            domain::git::commands::git_current_user,
            domain::git::commands::git_conflict_sides,
            domain::git::commands::git_resolve_conflict,
            domain::git::commands::git_stage_hunk,
            domain::git::commands::git_unstage_hunk,
            domain::git::commands::git_stage_lines,
            domain::git::commands::git_unstage_lines,
            domain::git::commands::git_commit_files,
            domain::git::commands::git_file_log,
            domain::git::commands::git_revert_commit,
            domain::git::commands::git_tags,
            domain::git::commands::git_tag_create,
            domain::git::commands::git_tag_delete,
            domain::git::commands::git_checkout_remote_branch,
            domain::terminal::commands::pty_default_options,
            domain::terminal::commands::pty_write,
            domain::terminal::commands::pty_resize,
            domain::terminal::commands::pty_kill,
            domain::terminal::commands::pty_set_paused,
            domain::terminal::commands::pty_detach,
            domain::terminal::commands::terminal_sessions,
            domain::terminal::commands::shell_profiles,
            domain::terminal::commands::resolve_terminal_path,
            domain::terminal::commands::terminal_resolve_link_candidates,
            domain::task::commands::detect_tasks,
            domain::font::commands::font_list,
            domain::locale::commands::locale_list,
            domain::locale::commands::locale_get,
            domain::locale::commands::locale_get_current,
            domain::theme::commands::theme_list,
            domain::theme::commands::theme_get,
            domain::theme::commands::theme_get_current,
            domain::theme::commands::theme_save,
            domain::theme::commands::theme_delete,
            domain::snippet::commands::snippet_list,
            domain::snippet::commands::snippet_save,
            domain::snippet::commands::snippet_delete,
            domain::settings::commands::settings_get,
            domain::settings::commands::settings_update,
            domain::settings::commands::settings_set_theme,
            domain::system::commands::system_usage_get,
            domain::system::commands::system_usage_breakdown,
            domain::system::commands::system_open_path,
            domain::system::commands::system_reveal_path,
            domain::system::commands::system_open_in_browser,
            domain::system::commands::system_open_app_data_path,
            domain::system::commands::system_open_external_url,
            domain::ide::commands::ide_get_status,
            domain::ide::commands::ide_set_selection,
            domain::ide::commands::ide_clear_selection,
            domain::ide::commands::ide_publish_diagnostics,
            domain::ide::commands::ide_resolve_diff,
            domain::ide::commands::ide_resolve_save,
            domain::ide::commands::ide_notify_at_mention,
            domain::ai::commands::ai_token_status,
            domain::ai::commands::ai_set_token,
            domain::ai::commands::ai_clear_token,
            domain::ai::commands::ai_list_models,
            domain::ai::commands::ai_inline_complete,
            domain::ai::commands::ai_inline_edit,
            domain::ai::commands::ai_commit_message,
            domain::ai::commands::ai_request_cancel,
            domain::sync::commands::sync_status,
            domain::sync::commands::sync_connect,
            domain::sync::commands::sync_disconnect,
            domain::sync::commands::sync_upload,
            domain::sync::commands::sync_download,
            domain::vsix::commands::vsix_extract_themes,
            domain::vsix::commands::vsix_import_plugin,
            domain::remote::commands::remote_status,
            domain::remote::commands::remote_issue_link,
            domain::remote::commands::remote_revoke_sessions,
            domain::remote::commands::remote_set_password,
            domain::remote::commands::remote_clear_password,
            domain::window::commands::window_set_fullscreen,
            domain::app::commands::app_file_read,
            domain::app::commands::app_file_write,
            domain::notification::commands::notification_notify,
            domain::notification::commands::notification_open_system_settings,
            domain::app::commands::perf_snapshot,
            domain::app::commands::perf_reset,
        ])
        .events(collect_events![
            ProjectOpened,
            ProjectClosed,
            ProjectActivated,
            ProjectListChanged,
            ProjectGroupsChanged,
            ProjectRecentCleared,
            SessionShellSlotsChanged,
            WindowChromeChanged,
            LayoutChanged,
            ThemeChanged,
            FsChanged,
            FsRescanRequired,
            TerminalSpawned,
            TerminalExited,
            TerminalCwdChanged,
            TerminalCommandFinished,
            GitStatusChanged,
            GitRefsChanged,
            LspSessionStatusChanged,
            LspInstallProgress,
            AgentStateChanged,
            AgentExternalOpen,
            IdeStatusChanged,
            IdeDiffRequested,
            IdeSaveRequested,
            IdeCloseTabRequested,
            SyncStateChanged,
            RemoteStateChanged,
            HotExitFlushRequested,
            SettingsChanged
        ])
        .typ::<domain::terminal::types::PtyAttachResult>()
}

pub(crate) const RAW_CHANNEL_COMMANDS: &[&str] = &["pty_spawn", "pty_attach", "file_read_raw"];

/// Builds the `main` window by hand instead of letting Tauri auto-create it from
/// `tauri.conf.json`, which is why that entry carries `"create": false` (`tauri::app::setup` skips
/// every `WindowConfig` whose `create` is false). `WebviewWindowBuilder::from_config` reproduces
/// the *identical* configured window — label, size/min-size, `backgroundColor`, `visible: false`,
/// Overlay title bar — so the FOUC policy (`docs/features/window-chrome.md` §2) and the `main`
/// capability's label match are untouched; the only thing hand-building buys is the chance to
/// attach `platform::navigation_guard`'s two handlers, which the config has no way to express.
///
/// Called as the very first statement of `.setup`, i.e. at the same point in the boot sequence the
/// auto-created window occupied (Tauri creates config windows immediately before invoking the
/// setup hook), so nothing that reads `MAIN_WINDOW_LABEL` moves relative to it and the
/// window-creation latency the boot-watcher deferral protects
/// (`docs/acknowledge/2026-08-20-boot-watcher-defer-contract.md`) is unchanged. A failure here
/// propagates as a setup error — an app with no main window has nothing to fall back to.
fn create_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == constants::MAIN_WINDOW_LABEL)
        .cloned()
        .ok_or_else(|| format!("main window config not found: label={}", constants::MAIN_WINDOW_LABEL))?;

    platform::navigation_guard::apply_navigation_guard(
        tauri::WebviewWindowBuilder::from_config(app.handle(), &window_config)?,
        app.config(),
    )
    .build()?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Before anything that could be measured. The command-name universe handed to the registry is
    // the dispatch table's own — the same list `collect_commands_매크로_출력과_dispatch_테이블은_
    // 커맨드_이름_집합이_일치한다` pins to `collect_commands!` — so a new command starts being
    // counted the moment it is registered, with no second list to keep in sync.
    infra::perf::init(
        remote_gateway::IMPLEMENTED_JSON_COMMANDS
            .iter()
            .chain(RAW_CHANNEL_COMMANDS.iter())
            .copied(),
    );

    #[cfg(not(windows))]
    let path_env_fix_error = fix_path_env::fix().err();

    let builder = specta_builder();
    let mut exit_drain: Option<ExitDrain> = None;

    #[cfg(debug_assertions)]
    builder
        .export(specta_typescript::Typescript::default(), BINDINGS_PATH)
        .expect("failed to export typescript bindings");

    let specta_handler = builder.invoke_handler();
    let raw_channel_handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> = Box::new(tauri::generate_handler![
        domain::terminal::commands::pty_spawn,
        domain::terminal::commands::pty_attach,
        domain::file::commands::file_read_raw
    ]);

    let mut app = tauri::Builder::default();

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        app = app.plugin(tauri_plugin_single_instance::init(|app_handle, argv, _cwd| {
            if let Some(window) = app_handle.get_webview_window(constants::MAIN_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.set_focus();
            }

            if let Some(request) = domain::agent::service::parse_cli_payload(&argv) {
                domain::agent::commands::queue_external_open(app_handle, request.clone());
                TauriEventSink(app_handle).publish(AppEvent::AgentExternalOpen { request });
            }
        }));
    }

    let log_plugin = tauri_plugin_log::Builder::new()
        .level_for("tungstenite", log::LevelFilter::Warn)
        .level_for("tokio_tungstenite", log::LevelFilter::Warn)
        .build();

    app.plugin(log_plugin)
        .plugin(tauri_plugin_dialog::init())
        // Registered for its Rust-side `NotificationExt` only (`domain::notification::commands`).
        // The JS guest package is deliberately not installed: its API reads the `window.Notification`
        // shim this plugin's init script injects, which the remote mirror never receives, and every
        // send has to pass the app-wide focus/settings gate in Rust anyway. The capability grants
        // `notification:allow-is-permission-granted` alone — not for app code, but because that init
        // script invokes it on every window load without a `.catch`, and a denied invoke would land
        // in the file log as an unhandled rejection each boot
        // (`shared/lib/error-log-forwarding.ts`).
        .plugin(tauri_plugin_notification::init())
        // `open_js_links_on_click` (default `true`) injects a document-level click interceptor that
        // calls the plugin's own `openUrl` — a call this app's capability set deliberately never
        // grants (`2026-08-18-hand-qa-fix-contract.md` §"opener JS+capability 개방 기각"), so the
        // interceptor could only ever `preventDefault()` a link and drop it. Turning it off leaves
        // `app/providers/external-link-provider.tsx` as the single anchor-click owner.
        .plugin(tauri_plugin_opener::Builder::new().open_js_links_on_click(false).build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .map_label(domain::window::service::normalize_window_state_label)
                // Auxiliary windows are excluded from persisted position/size entirely (contract
                // §3.1's other blessed option — "보조 창 제외" — rather than the `map_label`
                // collapse alone): `map_label` still folds every `editor-<n>` label onto the same
                // `AUXILIARY_WINDOW_STATE_KEY` cache key (stopping `.window-state.json` from
                // growing one entry per window ever opened), but the plugin's `with_filter`
                // callback runs *after* that mapping, so filtering out that one shared key also
                // stops the plugin from restoring (or ever saving) a position/size for it —
                // without this, two auxiliary windows sharing that single cache key would restore
                // to the exact same saved geometry and open perfectly overlapping each other.
                .with_filter(|label| label != domain::window::types::AUXILIARY_WINDOW_STATE_KEY)
                .build(),
        )
        .register_uri_scheme_protocol("asset", |context, request| {
            let state = context.app_handle().state::<AppState>();
            let projects = state.projects.read();
            platform::asset_protocol::respond(&projects, request)
        })
        .on_menu_event(|app, event| dispatch_menu_action(app, domain::window::menu::menu_action(event.id().as_ref())))
        .invoke_handler(move |invoke| {
            // The one place every IPC call passes through by name, so a per-command invoke counter
            // costs nothing per command body (research 3b §4.2 — tauri's `InvokeResolver` is
            // `pub(crate)` and tauri-specta has no middleware hook, so this closure is also the
            // *only* place a generic hook can live; durations still need per-site `perf::span`).
            infra::perf::record_command(invoke.message.command());
            if RAW_CHANNEL_COMMANDS.contains(&invoke.message.command()) {
                raw_channel_handler(invoke)
            } else {
                specta_handler(invoke)
            }
        })
        .setup(move |app| {
            let setup_started = std::time::Instant::now();

            {
                let _span = infra::perf::span(infra::perf::SpanSlot::SetupMainWindow);
                create_main_window(app)?;
            }

            #[cfg(not(windows))]
            if let Some(error) = &path_env_fix_error {
                log::warn!("PATH 환경변수 보정 실패: {error}");
            } else {
                let path_entry_count = std::env::var_os("PATH")
                    .map(|path_var| std::env::split_paths(&path_var).count())
                    .unwrap_or_default();
                log::info!("PATH 환경변수 보정 완료: {path_entry_count} 항목");
            }

            {
                let _span = infra::perf::span(infra::perf::SpanSlot::SetupLocaleWarm);
                domain::locale::service::warm_builtin_catalogs();
            }
            builder.mount_events(app);

            let state_restore_span = infra::perf::span(infra::perf::SpanSlot::SetupStateRestore);
            let data_dir = app.path().app_data_dir().expect("app data dir unavailable");
            let state = AppState::new(AppPaths::new(data_dir));

            for warning in domain::project::commands::restore_state(&state) {
                log::warn!("{warning}");
            }

            let restored = domain::project::commands::projects_pending_watcher_restore(&state.projects.read(), &state.session.read());

            let platform = Arc::new(TauriPlatformServices(app.handle().clone()));
            let services = Arc::new(AppServices::new(
                state,
                TaskSupervisor::new(tauri::async_runtime::handle().inner().clone()),
                RemoteDispatchLimiter::new(REMOTE_DISPATCH_MAX_CONCURRENT),
                PlatformServicesState::new(platform.clone()),
                SecretStoreState::new(app.config().identifier.clone()),
                IdeSaveFile(save_ide_diff_file),
                platform,
            ));

            app.manage(services.state.clone());
            app.manage(menu_sources());
            app.manage(project_capabilities());
            app.manage(project_restore_watchers());
            app.manage(settings_toggle_observers());
            app.manage(SettingsApplyPort(apply_settings_from_port));
            app.manage(system_usage_label_providers());
            app.manage(pty_spawn_env_provider());
            app.manage(pty_session_observers());
            app.manage(services.tree.clone());
            app.manage(services.terminal.clone());
            app.manage(AgentForegroundPids(foreground_pids_for_agent));
            app.manage(services.git.clone());
            app.manage(services.lsp.clone());
            app.manage(services.lsp_install.clone());
            app.manage(services.search.clone());
            app.manage(services.plugin.clone());
            app.manage(PluginRuntimePort {
                language_overlays: plugin_language_overlays,
                commit_staged_import: commit_staged_vsix_plugin,
            });
            app.manage(services.agents.clone());
            app.manage(services.agent_hooks.clone());
            app.manage(services.system_usage.clone());
            app.manage(services.ide.clone());
            app.manage(services.ide_save_file.clone());
            app.manage(ide_layout_actions());
            app.manage(layout_tab_closed_observers());
            app.manage(services.ai_requests.clone());
            app.manage(services.secrets.clone());
            app.manage(remote_dispatch_port());
            app.manage(services.remote.clone());
            app.manage(services.remote_dispatch_limiter.clone());
            app.manage(services.platform.clone());
            app.manage(services.windows.clone());
            app.manage(services.tasks.clone());
            app.manage(services);
            drop(state_restore_span);

            app.set_menu(domain::window::commands::build_app_menu(app.handle())?)?;
            listen_for_app_menu_refresh(app.handle());

            let deferred_restore_span = infra::perf::span(infra::perf::SpanSlot::SetupDeferredRestore);
            domain::window::commands::restore_auxiliary_windows(app.handle());
            domain::project::commands::restore_project_watchers(app.handle(), restored);
            domain::remote::commands::refresh_password_configured_cache(app.handle());
            drop(deferred_restore_span);

            domain::agent::commands::queue_cold_start_external_open(app.handle());

            macro_rules! fanout_remote_events {
                ($($event:ty),+ $(,)?) => {
                    $({
                        let broadcast_handle = app.handle().clone();
                        app.listen_any(<$event as tauri_specta::Event>::NAME, move |event| {
                            let remote = broadcast_handle.state::<RemoteStore>();
                            if !remote.has_event_subscribers() {
                                return;
                            }
                            let frame = serde_json::json!({
                                "t": "event",
                                "event": <$event as tauri_specta::Event>::NAME,
                                "payload": event.payload(),
                            })
                            .to_string();
                            remote.broadcast_event(frame);
                        });
                    })+
                };
            }
            fanout_remote_events!(
                ProjectOpened,
                ProjectClosed,
                ProjectActivated,
                ProjectListChanged,
                ProjectGroupsChanged,
                ProjectRecentCleared,
                SessionShellSlotsChanged,
                WindowChromeChanged,
                LayoutChanged,
                ThemeChanged,
                FsChanged,
                FsRescanRequired,
                TerminalSpawned,
                TerminalExited,
                TerminalCwdChanged,
                TerminalCommandFinished,
                GitStatusChanged,
                GitRefsChanged,
                LspSessionStatusChanged,
                LspInstallProgress,
                AgentStateChanged,
                IdeStatusChanged,
                IdeDiffRequested,
                IdeSaveRequested,
                IdeCloseTabRequested,
                SyncStateChanged,
                RemoteStateChanged,
                SettingsChanged,
            );

            if app.state::<AppState>().settings.read().agent_hooks_enabled {
                let hooks_boot_handle = app.handle().clone();
                app.state::<TaskSupervisor>().spawn("agent-hooks-boot", async move {
                    domain::agent::hooks::reconcile_installed_hooks(&hooks_boot_handle).await;
                });
            }

            if app.state::<AppState>().settings.read().ide_integration_enabled {
                let ide_boot_handle = app.handle().clone();
                app.state::<TaskSupervisor>().spawn("ide-boot", async move {
                    if let Err(error) = domain::ide::commands::ide_start(
                        ide_boot_handle.clone(),
                        ide_boot_handle.state::<AppState>(),
                        ide_boot_handle.state::<IdeStore>(),
                    )
                    .await
                    {
                        log::warn!("IDE 연동 자동 시작 실패: {error}");
                    }
                });
            }

            if app.state::<AppState>().settings.read().remote_access_enabled {
                let remote_boot_handle = app.handle().clone();
                app.state::<TaskSupervisor>().spawn("remote-boot", async move {
                    if let Err(error) =
                        domain::remote::commands::remote_start(remote_boot_handle.clone(), remote_boot_handle.state::<RemoteStore>()).await
                    {
                        log::warn!("원격 접속 서버 자동 시작 실패: {error}");
                    }
                });
            }

            let ide_reconcile_handle = app.handle().clone();
            app.state::<TaskSupervisor>().spawn("ide-reconcile", async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(domain::ide::types::IDE_RECONCILE_INTERVAL_MS));
                loop {
                    ticker.tick().await;
                    domain::ide::commands::reconcile_stale_pending(&ide_reconcile_handle);
                }
            });

            let agent_handle = app.handle().clone();
            app.state::<TaskSupervisor>().spawn("agent-poll", async move {
                let interval = if cfg!(windows) {
                    domain::agent::types::AGENT_POLL_WINDOWS_MS
                } else {
                    domain::agent::types::AGENT_POLL_UNIX_MS
                };
                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval));
                loop {
                    ticker.tick().await;
                    domain::agent::commands::poll_agents(&agent_handle).await;
                }
            });

            let flush_state = (*app.state::<AppState>()).clone();
            let flush_tasks = (*app.state::<TaskSupervisor>()).clone();
            let flush_loop = layout_actions::flush_layouts_periodically(
                flush_state,
                flush_tasks,
                std::time::Duration::from_millis(domain::layout::service::LAYOUT_FLUSH_INTERVAL_MS),
            );
            app.state::<TaskSupervisor>().spawn("layout-flush", flush_loop);

            log::info!("setup 완료: elapsed_ms={}", setup_started.elapsed().as_millis());

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Destroyed => {
                domain::layout::service::flush_dirty_layouts(&window.state::<AppState>());
                if window.state::<AppState>().forget_hot_exit_flush_window(window.label()) {
                    window.app_handle().exit(0);
                }
                if let Some((project_id, window_slot)) = window.state::<WindowRegistry>().forget(window.label()) {
                    plan_return_of_auxiliary_window_tabs(&window.app_handle().clone(), &project_id, window_slot);
                }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if let Some((project_id, window_slot)) = domain::window::commands::handle_close_requested(window, api) {
                    plan_return_of_auxiliary_window_tabs(&window.app_handle().clone(), &project_id, window_slot);
                }
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |app_handle, event| {
            if matches!(&event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
                app_handle.state::<AppState>().begin_shutdown();
                app_handle.state::<domain::ai::commands::AiRequestStore>().shutdown();
                app_handle.state::<LspInstallStore>().shutdown();
                domain::layout::service::flush_dirty_layouts(&app_handle.state::<AppState>());
                app_handle.state::<TerminalStore>().shutdown();
                app_handle.state::<LspStore>().shutdown();
                domain::agent::commands::cleanup_all_wait_markers(&app_handle.state::<AgentStore>());
                domain::agent::hooks::stop_hooks_server(app_handle);
                domain::ide::commands::stop_server(app_handle, &app_handle.state::<IdeStore>());
                domain::remote::commands::stop_server(app_handle, &app_handle.state::<RemoteStore>());
                app_handle.state::<TaskSupervisor>().stop_all();
                app_handle.state::<domain::search::commands::SearchStore>().cancel_all();
            }
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                let exit_drain = exit_drain.get_or_insert_with(|| {
                    ExitDrain::new((*app_handle.state::<domain::ai::commands::AiRequestStore>()).clone())
                        .with_state((*app_handle.state::<AppState>()).clone())
                });
                if !exit_drain.is_ready() {
                    api.prevent_exit();
                    let handle = app_handle.clone();
                    let exit_code = code.unwrap_or(0);
                    exit_drain.begin(
                        tauri::async_runtime::handle().inner(),
                        (*app_handle.state::<TaskSupervisor>()).clone(),
                        (*app_handle.state::<LspInstallStore>()).clone(),
                        (*app_handle.state::<LspStore>()).clone(),
                        (*app_handle.state::<TerminalStore>()).clone(),
                        move || handle.exit(exit_code),
                    );
                }
            }
            if matches!(&event, tauri::RunEvent::Exit) {
                let exit_drain = exit_drain.get_or_insert_with(|| {
                    ExitDrain::new((*app_handle.state::<domain::ai::commands::AiRequestStore>()).clone())
                        .with_state((*app_handle.state::<AppState>()).clone())
                });
                if let Err(error) = tauri::async_runtime::block_on(exit_drain.wait_for_direct_exit(
                    (*app_handle.state::<TaskSupervisor>()).clone(),
                    (*app_handle.state::<LspInstallStore>()).clone(),
                    (*app_handle.state::<LspStore>()).clone(),
                    (*app_handle.state::<TerminalStore>()).clone(),
                )) {
                    log::error!("direct exit runtime drain failed: {error}");
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use regex::Regex;

    use super::*;

    /// Slices `source` to the text strictly between the first `start_marker` and the first
    /// `end_marker` that follows it — used to pull a single macro invocation's argument list (or a
    /// generated binding's object literal) out of a whole source file for parity tests below.
    fn extract_between<'a>(source: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
        let start = source
            .find(start_marker)
            .unwrap_or_else(|| panic!("시작 마커를 찾을 수 없습니다: {start_marker}"))
            + start_marker.len();
        let end = source[start..]
            .find(end_marker)
            .unwrap_or_else(|| panic!("종료 마커를 찾을 수 없습니다: {end_marker}"));
        &source[start..start + end]
    }

    fn identifier_set(block: &str) -> BTreeSet<String> {
        block
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// Reads `events.rs`'s own source and pairs every `Event`-derived struct's Rust type name with
    /// the `event_name` string declared right above it via `#[tauri_specta(event_name = "...")]` —
    /// the single point both the type-identifier-based comparisons (`collect_events!`,
    /// `fanout_remote_events!`) and the wire-name-based comparison (`bindings.ts`) below derive from.
    ///
    /// Coupled to `events.rs`'s exact formatting: the pattern requires `#[tauri_specta(event_name =
    /// "...")]` to be the attribute immediately preceding `pub struct Name` (only whitespace between
    /// them — no intervening `#[derive(...)]`/`#[serde(...)]` line). Every struct in `events.rs`
    /// currently declares `tauri_specta` last, right above `pub struct`, so this holds today; a
    /// struct that reordered its attributes would silently vanish from this map (the parity tests
    /// below would then fail loudly on a shrunk `declared` set, not pass with a wrong pairing) rather
    /// than the regex adapting to it. See
    /// `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` §1 T1-E.
    fn event_name_by_type() -> BTreeMap<String, String> {
        let pattern =
            Regex::new(r#"#\[tauri_specta\(event_name = "([^"]+)"\)\]\s*pub struct ([A-Za-z][A-Za-z0-9]*)"#).expect("유효한 정규식");
        pattern
            .captures_iter(include_str!("events.rs"))
            .map(|capture| (capture[2].to_string(), capture[1].to_string()))
            .collect()
    }

    #[test]
    fn typescript_바인딩을_생성한다() {
        specta_builder()
            .export(specta_typescript::Typescript::default(), BINDINGS_PATH)
            .expect("failed to export typescript bindings");
    }

    /// `X1#9` — pins the command-name parity baseline to what `collect_commands!` produces *right
    /// now* (exported to a throwaway temp file, never touching the committed `bindings.ts`), instead
    /// of trusting the checked-in `bindings.ts` to already be current. Without this, a `dispatch.rs`
    /// change that forgets to regenerate bindings could pass by comparing two equally-stale sources.
    #[test]
    fn collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다() {
        let temp_path = std::env::temp_dir().join(format!("taide-bindings-baseline-{}.ts", uuid::Uuid::new_v4()));
        specta_builder()
            .export(specta_typescript::Typescript::default(), &temp_path)
            .expect("failed to export typescript bindings for baseline parity test");
        let generated = std::fs::read_to_string(&temp_path).expect("생성된 바인딩을 읽지 못했습니다");
        let _ = std::fs::remove_file(&temp_path);

        let pattern = Regex::new(r#"__TAURI_INVOKE\(\s*"([a-zA-Z0-9_]+)""#).expect("유효한 정규식");
        let mut generated_names: BTreeSet<String> = pattern.captures_iter(&generated).map(|capture| capture[1].to_string()).collect();
        generated_names.extend(RAW_CHANNEL_COMMANDS.iter().map(|name| name.to_string()));

        let implemented_names: BTreeSet<String> = remote_gateway::IMPLEMENTED_JSON_COMMANDS
            .iter()
            .chain(RAW_CHANNEL_COMMANDS.iter())
            .map(|name| name.to_string())
            .collect();

        assert_eq!(
            generated_names, implemented_names,
            "collect_commands! 매크로가 지금 이 순간 생성하는 바인딩과 dispatch 커맨드 테이블이 어긋났습니다 — 커밋된 bindings.ts 가 낡아 있어도 이 테스트는 잡습니다"
        );
    }

    /// `X1#8` — the event types registered on the Tauri IPC layer (`collect_events!`, `lib.rs`) must
    /// be exactly the `Event`-derived structs declared in `events.rs`. Catches a struct added to one
    /// list and forgotten in the other.
    #[test]
    fn 이벤트_타입_목록은_events_rs와_collect_events_매크로에서_일치한다() {
        let declared: BTreeSet<String> = event_name_by_type().into_keys().collect();
        assert_eq!(declared.len(), 30, "events.rs 에 선언된 이벤트 구조체 수가 30종에서 벗어났습니다");

        let collected = identifier_set(extract_between(include_str!("lib.rs"), "collect_events![", "]"));

        assert_eq!(
            declared, collected,
            "events.rs 의 이벤트 구조체 집합과 lib.rs 의 collect_events! 인자 집합이 다릅니다"
        );
    }

    /// `X1#8` — `fanout_remote_events!` (`lib.rs`) deliberately omits two events from the
    /// remote-session broadcast: `HotExitFlushRequested`(데스크톱 창 종료 신호라 원격 세션에는
    /// 무의미) and `AgentExternalOpen`(T0 #14 — 원격 세션이 대기 중인 외부 열기 요청을 실시간으로
    /// 가로채면 안 된다. `remote_gateway.rs` 의 `deny_remote_agent_pending_external_opens`
    /// doc comment 참조). This test names that exception list explicitly so any *other* divergence
    /// from `collect_events!`'s set — the failure mode the exception list used to hide behind a
    /// plain code comment — fails loudly instead.
    #[test]
    fn fanout_remote_events_매크로는_events_rs에서_의도된_예외를_제외한_집합과_일치한다() {
        const INTENTIONALLY_EXCLUDED_FROM_REMOTE_FANOUT: &[&str] = &["HotExitFlushRequested", "AgentExternalOpen"];

        let declared: BTreeSet<String> = event_name_by_type().into_keys().collect();
        let fanned_out = identifier_set(extract_between(include_str!("lib.rs"), "fanout_remote_events!(", ")"));
        let excluded: BTreeSet<String> = INTENTIONALLY_EXCLUDED_FROM_REMOTE_FANOUT
            .iter()
            .map(|name| name.to_string())
            .collect();
        let expected: BTreeSet<String> = declared.difference(&excluded).cloned().collect();

        assert_eq!(
            fanned_out, expected,
            "fanout_remote_events! 가 의도된 예외 목록 밖의 이벤트를 빠뜨렸거나, 예외로 처리해야 할 이벤트를 원격으로 방송하고 있습니다"
        );
    }

    #[test]
    fn 주기적_레이아웃_flush_는_blocking_스레드에서_실행된다() {
        let adapter = extract_between(include_str!("lib.rs"), "let flush_state =", "log::info!(");
        assert!(adapter.contains("layout_actions::flush_layouts_periodically("));
        let tick_body = extract_between(
            include_str!("../../crates/taide-runtime/src/layout_actions.rs"),
            "pub async fn flush_layouts_periodically(",
            "\n}\n",
        );

        assert!(
            tick_body.contains("flush_dirty_layouts"),
            "주기 flush 태스크에서 flush_dirty_layouts 호출을 찾을 수 없습니다"
        );
        assert!(
            tick_body.contains("spawn_blocking_transient_handle"),
            "주기 flush 는 async 워커가 아니라 spawn_blocking 에서 실행돼야 합니다"
        );
        assert!(
            tick_body.contains("let _ = worker.await;"),
            "spawn_blocking 핸들을 await 하지 않으면 tick 이 겹쳐 flush 순서가 깨집니다"
        );
    }

    /// Pins [`project_capabilities`]'s registration order by scanning this file's own source —
    /// that order is the attach/detach traversal order `project_open`/`project_close` run, and the
    /// detach half is correctness-sensitive (see the function's doc). A reorder, addition, or
    /// removal must consciously update this expected list.
    #[test]
    fn project_capabilities_등록_순서는_close_순서_계약과_일치한다() {
        let registered: Vec<String> = extract_between(include_str!("lib.rs"), "ProjectCapabilities::new(vec![", "])")
            .split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect();

        let expected = [
            "Box::new(domain::layout::capability::LayoutCapability)",
            "Box::new(domain::file::capability::FileWatcherCapability)",
            "Box::new(domain::git::capability::GitWatcherCapability)",
            "Box::new(domain::terminal::capability::TerminalCapability)",
            "Box::new(domain::git::capability::GitCacheCapability)",
            "Box::new(domain::tree::capability::TreeCacheCapability)",
            "Box::new(domain::ide::capability::IdeLockfileCapability)",
            "Box::new(domain::agent::capability::AgentHooksCapability)",
        ];

        assert_eq!(
            registered, expected,
            "project_capabilities 의 등록 순서가 계약과 다릅니다 — 이 순서는 project_close 의 자원 회수 순서 그 자체입니다"
        );
    }

    #[test]
    fn 앱_종료는_검색_신규_입장을_닫고_현재_세션을_취소한다() {
        let source = include_str!("lib.rs");
        let shutdown = extract_between(
            source,
            "if matches!(&event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {",
            "\n            }\n            if let tauri::RunEvent::ExitRequested",
        );
        let stopped = shutdown.find("app_handle.state::<TaskSupervisor>().stop_all()").unwrap();
        let cancelled = shutdown
            .find("app_handle.state::<domain::search::commands::SearchStore>().cancel_all()")
            .unwrap();
        assert!(stopped < cancelled);
    }

    #[test]
    fn 직접_exit은_전체_자원_drain을_사용한다() {
        let source = include_str!("lib.rs");
        let direct_exit = extract_between(
            source,
            "if matches!(&event, tauri::RunEvent::Exit) {",
            "\n            }\n        });",
        );
        assert!(direct_exit.contains("wait_for_direct_exit("));
        assert!(direct_exit.contains("app_handle.state::<TaskSupervisor>()"));
        assert!(direct_exit.contains("app_handle.state::<LspInstallStore>()"));
        assert!(direct_exit.contains("app_handle.state::<LspStore>()"));
        assert!(direct_exit.contains("app_handle.state::<TerminalStore>()"));
    }

    #[test]
    fn watcher_종료_소유는_두_builder와_두_exit_경로에_연결된다() {
        let file = include_str!("domain/file/capability.rs");
        let git = include_str!("domain/git/watch.rs");
        let source = include_str!("lib.rs");
        let exit = extract_between(source, ".run(move |app_handle, event| {", "\n        });");

        assert!(file.contains("handle.with_stop_scheduler(move |stop| tracker.schedule(stop))"));
        assert!(git.contains("handle.with_stop_scheduler(move |stop| tracker.schedule(stop))"));
        assert_eq!(exit.matches(".with_state((*app_handle.state::<AppState>()).clone())").count(), 2);
    }

    #[test]
    fn 탭_닫기_후처리는_layout_기록_뒤_ide와_pty_순서로_조립된다() {
        let source = include_str!("lib.rs");
        let observers = extract_between(source, "fn layout_tab_closed_observers()", "/// Routes one app-menu");
        let ide_position = observers
            .find("app.state::<IdeStore>().reconcile_closed_tab(tab)")
            .expect("IDE 후처리 등록");
        let terminal_position = observers
            .find("app.state::<TerminalStore>().kill_session(session_id)")
            .expect("PTY 회수 등록");
        assert!(ide_position < terminal_position);

        let setup = extract_between(
            source,
            "app.manage(services.ide.clone());",
            "app.manage(services.ai_requests.clone());",
        );
        assert!(setup.contains("app.manage(layout_tab_closed_observers());"));

        let layout_source = include_str!("domain/layout/service.rs");
        assert!(layout_source.contains("layout_actions::close_tab_and_finish("));
        assert!(layout_source.contains("app.state::<LayoutTabClosedObservers>().notify(app, tab)"));
        let layout_actions = include_str!("../../crates/taide-runtime/src/layout_actions.rs");
        let close_body = extract_between(layout_actions, "pub async fn close_tab_and_finish", "\n#[cfg(test)]");
        let write_position = close_body.find("*state.layouts.write() = layouts;").expect("layout 기록");
        let notify_position = close_body.find("notify_tab_closed(&closed.tab);").expect("닫기 후처리");
        assert!(write_position < notify_position);
    }

    #[test]
    fn ide_mcp의_탭_수명주기는_조립부의_layout_경로를_공유한다() {
        let source = include_str!("lib.rs");
        let actions = extract_between(source, "fn open_ide_file_tab(", "fn layout_tab_closed_observers()");
        assert!(actions.contains("layout_service::open_tab_and_finish("));
        assert!(actions.contains("layout_service::close_tab_and_finish("));
        assert!(actions.contains(".map(|(_, closed_tab, _)| closed_tab.tab)"));
        assert!(actions.contains("open_file_tab: open_ide_file_tab"));
        assert!(actions.contains("close_tab: close_ide_tab"));

        let setup = extract_between(
            source,
            "app.manage(services.ide.clone());",
            "app.manage(services.ai_requests.clone());",
        );
        assert!(setup.contains("app.manage(ide_layout_actions());"));

        let ide_server = include_str!("domain/ide/server.rs");
        assert!(ide_server.contains("(app.state::<IdeLayoutActions>().open_file_tab)("));
        assert_eq!(ide_server.matches("(app.state::<IdeLayoutActions>().close_tab)(").count(), 2);
    }

    #[test]
    fn ide_diff_저장은_조립부의_파일_저장_경로를_사용한다() {
        let source = include_str!("lib.rs");
        assert!(source.contains("domain::file::service::save_file_within_open_projects(state, path, content)"));
        let setup = extract_between(source, "app.manage(services.ide.clone());", "app.manage(ide_layout_actions());");
        assert!(source.contains("IdeSaveFile(save_ide_diff_file)"));
        assert!(setup.contains("app.manage(services.ide_save_file.clone());"));

        let ide = include_str!("domain/ide/commands.rs");
        assert!(ide.contains("save_file: State<'_, IdeSaveFile>"));
        assert!(ide.contains("ide_actions::ide_resolve_diff(&state, &save_file, &ide, request_id, outcome, content).await"));

        let actions = include_str!("../../crates/taide-runtime/src/ide_actions.rs");
        assert!(actions.contains("match (save_file.0)(state, &pending.new_path, &content)"));

        let gateway = include_str!("remote_gateway.rs");
        let remote_call = extract_between(gateway, "ide::ide_resolve_diff(", "\n            .await,");
        assert_eq!(remote_call.matches("app.state(),").count(), 3);
    }

    #[test]
    fn app과_sync의_설정_적용은_조립부의_공통_경로를_사용한다() {
        let source = include_str!("lib.rs");
        assert!(source.contains("Box::pin(domain::settings::commands::apply_and_broadcast(app, state, next))"));
        let setup = extract_between(
            source,
            "app.manage(settings_toggle_observers());",
            "app.manage(system_usage_label_providers());",
        );
        assert!(setup.contains("app.manage(SettingsApplyPort(apply_settings_from_port));"));

        let app = include_str!("domain/app/commands.rs");
        let app_actions = include_str!("../../crates/taide-runtime/src/app_actions.rs");
        assert!(app_actions.contains("taide_settings::service::parse_settings_json(&content)"));
        assert!(app.contains("app_actions::app_file_write(&state, target, content,"));
        assert!(app.contains("app_actions::apply_settings_file(&state, settings,"));
        assert_eq!(app.matches("let apply_settings = apply_settings.0;").count(), 2);

        let sync = include_str!("domain/sync/commands.rs");
        let sync_actions = include_str!("../../crates/taide-runtime/src/sync_actions.rs");
        assert_eq!(sync_actions.matches("taide_settings::service::save_settings(").count(), 3);
        assert!(sync.contains("|settings| apply_settings(&app, &state, settings)"));
        assert!(sync_actions.contains("apply_settings(final_settings).await?"));
    }

    #[test]
    fn 파일_git_ide_vsix는_조립부의_플러그인_포트를_사용한다() {
        let source = include_str!("lib.rs");
        let setup = extract_between(
            source,
            "app.manage(services.plugin.clone());",
            "app.manage(services.agents.clone());",
        );
        assert!(setup.contains("app.manage(PluginRuntimePort {"));
        assert!(setup.contains("language_overlays: plugin_language_overlays,"));
        assert!(setup.contains("commit_staged_import: commit_staged_vsix_plugin,"));

        let file = include_str!("domain/file/commands.rs");
        assert!(file.contains("(plugins.language_overlays)(&app)"));
        let git = include_str!("domain/git/commands.rs");
        assert!(git.contains("(plugins.language_overlays)(&app)"));
        let ide = include_str!("domain/ide/server.rs");
        assert_eq!(ide.matches("(app.state::<PluginRuntimePort>().language_overlays)(app)").count(), 2);
        let vsix = include_str!("domain/vsix/commands.rs");
        assert!(vsix.contains("vsix_actions::vsix_import_plugin("));
        assert!(vsix.contains("let commit_staged_import = plugins.commit_staged_import;"));
        assert!(vsix.contains("commit_staged_import(&app, temp_dir, staged_plugin_id)"));
        let actions = include_str!("../../crates/taide-runtime/src/vsix_actions.rs");
        assert!(actions.find("state.begin_mutation()").unwrap() < actions.find("commit_staged_import(&staged.path").unwrap());
    }

    #[test]
    fn 프로젝트_복원_워처는_조립부_포트로_build와_register를_분리한다() {
        let source = include_str!("lib.rs");
        let provider = extract_between(source, "fn project_restore_watchers()", "fn system_usage_label_providers()");
        assert!(provider.contains("build_file: domain::file::capability::build_watcher_handle"));
        assert!(provider.contains("build_git: domain::git::watch::build_git_watcher_handle"));
        assert!(provider.contains("register_file: domain::file::capability::register_watcher_handle"));
        assert!(provider.contains("register_git: domain::git::watch::register_git_watcher_handle"));
        let setup = extract_between(
            source,
            "app.manage(project_capabilities());",
            "app.manage(settings_toggle_observers());",
        );
        assert!(setup.contains("app.manage(project_restore_watchers());"));

        let project = include_str!("domain/project/commands.rs");
        let actions = include_str!("../../crates/taide-runtime/src/project_actions.rs");
        assert!(project.contains("project_actions::restore_state(state)"));
        assert!(actions.contains("taide_layout::service::load_layout(&state.paths, &project.id)"));
        assert!(actions.contains("taide_settings::service::load_settings(&state.paths)"));
        let restore = extract_between(project, "pub(crate) fn restore_project_watchers(", "#[cfg(test)]");
        assert!(restore.contains(".map(taide_layout::service::open_file_paths)"));
        assert!(restore.find("let build_result =").unwrap() < restore.find("let _guard = state.begin_mutation().await;").unwrap());
        assert!(restore.find("let _guard = state.begin_mutation().await;").unwrap() < restore.find("register_file(&state").unwrap());
        assert!(restore.find("register_file(&state").unwrap() < restore.find("register_git(&state").unwrap());
    }

    #[test]
    fn 원격_websocket은_조립부의_json_raw_게이트웨이를_사용한다() {
        let source = include_str!("lib.rs");
        let port = extract_between(source, "fn dispatch_remote_json(", "fn layout_tab_closed_observers()");
        assert!(port.contains("remote_gateway::dispatch(&app, &name, args, channels).await"));
        assert!(port.contains("remote_gateway::dispatch_raw(&app, &name, args).await"));
        assert!(port.contains("json: dispatch_remote_json"));
        assert!(port.contains("raw: dispatch_remote_raw"));

        let setup = extract_between(
            source,
            "app.manage(services.secrets.clone());",
            "app.manage(services.remote_dispatch_limiter.clone());",
        );
        assert!(setup.contains("app.manage(remote_dispatch_port());"));

        let ws = include_str!("domain/remote/ws.rs");
        assert!(ws.contains("(app.state::<RemoteDispatchPort>().json)("));
        assert!(ws.contains("(app.state::<RemoteDispatchPort>().raw)("));
    }

    #[test]
    fn 앱_메뉴는_조립부의_최근_프로젝트와_번역_공급원을_사용한다() {
        let source = include_str!("lib.rs");
        let sources = extract_between(source, "fn recent_menu_projects(", "fn layout_tab_closed_observers()");
        assert!(sources.contains("domain::project::service::list_recent_projects(&state.paths)"));
        assert!(sources.contains("domain::locale::service::builtin_locale_for_language(&language)"));
        assert!(sources.contains("domain::locale::service::BUILTIN_EN_ID"));
        assert!(sources.contains("recent_projects: recent_menu_projects"));
        assert!(sources.contains("label: menu_label"));

        let setup = extract_between(source, "app.manage(services.state.clone());", "app.set_menu(");
        assert!(setup.contains("app.manage(menu_sources());"));

        let menu = include_str!("domain/window/menu.rs");
        assert!(menu.contains("(app.state::<MenuSources>().recent_projects)(app)"));
        assert!(menu.contains("(app.state::<MenuSources>().label)(app, key, fallback)"));

        let click = extract_between(source, "MenuAction::OpenRecent(project_id) => {", "MenuAction::Ignored => {}");
        assert!(click.contains("recent_project_root(&app, &project_id)"));
        assert!(source.contains("fn recent_project_root(app: &AppHandle, project_id: &ProjectId) -> Option<String>"));
    }

    #[test]
    fn 에이전트_감지는_조립부의_터미널_foreground_pid_공급원을_사용한다() {
        let source = include_str!("lib.rs");
        assert!(source.contains("app.state::<TerminalStore>().foreground_pids(project_id)"));
        let setup = extract_between(
            source,
            "app.manage(services.terminal.clone());",
            "app.manage(services.git.clone());",
        );
        assert!(setup.contains("app.manage(AgentForegroundPids(foreground_pids_for_agent));"));

        let agent = include_str!("domain/agent/commands.rs");
        assert!(agent.contains("foreground_pids: State<'_, AgentForegroundPids>"));
        assert!(agent.contains("app.state::<AgentForegroundPids>()"));
        assert_eq!(agent.matches("(foreground_pids.0)(").count(), 2);
    }

    #[test]
    fn 탭_창_이동은_조립부에서_창_생성과_rollback을_순서대로_수행한다() {
        let source = include_str!("lib.rs");
        let adapter = extract_between(
            source,
            "async fn layout_move_tab_to_window(",
            "fn plan_return_of_auxiliary_window_tabs(",
        );
        assert!(adapter.contains("layout_actions::layout_move_tab_to_window("));
        assert!(adapter.contains("open_auxiliary_window(app, state, windows, project_id, slot).await?"));
        assert!(adapter.contains("Ok(info.label)"));
        assert!(adapter.contains("app.get_webview_window(label)"));
        assert!(adapter.contains("let _ = webview_window.close()"));
        assert!(!adapter.contains("begin_mutation()"));
        let actions = include_str!("../../crates/taide-runtime/src/layout_actions.rs");
        let command = extract_between(
            actions,
            "pub async fn layout_move_tab_to_window<",
            "pub async fn return_auxiliary_window_tabs(",
        );
        let guard = command.find("state.begin_mutation().await").expect("mutation guard");
        let open = command
            .find("open_auxiliary_window(project_id.clone(), slot).await?")
            .expect("보조 창 생성");
        let move_tab = command.find("service::move_tab_to_new_window(").expect("탭 이동");
        let rollback = command.find("close_window(&label)").expect("생성 실패 rollback");
        let cleanup = command.find("cleanup_emptied_auxiliary_windows(").expect("빈 창 정리");
        let finish = command.find("finish_mutation(").expect("layout 완료");
        let write = command.find("*state.layouts.write() = layouts;").expect("layout 기록");
        assert!(guard < open);
        assert!(open < move_tab);
        assert!(move_tab < rollback);
        assert!(rollback < cleanup);
        assert!(cleanup < finish);
        assert!(finish < write);

        let commands = extract_between(
            source,
            "domain::layout::commands::layout_convert_untitled,",
            "domain::layout::commands::layout_apply_path_change,",
        );
        assert!(commands.contains("layout_move_tab_to_window,"));
    }

    #[test]
    fn 보조_창_닫힘은_조립부에서_미러_정리와_탭_복귀를_순서대로_수행한다() {
        let source = include_str!("lib.rs");
        let planner = extract_between(source, "fn plan_return_of_auxiliary_window_tabs(", "/// Routes one app-menu");
        assert!(planner.contains("spawn_transient(\"auxiliary-tab-return\""));
        assert!(
            planner.contains("layout_actions::return_auxiliary_window_tabs(&TauriEventSink(&app), &state, project_id, window_slot).await")
        );
        assert!(!planner.contains("begin_mutation()"));
        assert!(!planner.contains("list_mirrors("));
        let actions = include_str!("../../crates/taide-runtime/src/layout_actions.rs");
        let return_body = extract_between(actions, "pub async fn return_auxiliary_window_tabs(", "pub async fn layout_get(");
        let guard = return_body.find("state.begin_mutation().await").expect("mutation guard");
        let mirrors = return_body.find("taide_file::service::list_mirrors(").expect("mirror 조회");
        let clear_dirty = return_body
            .find("service::clear_auxiliary_window_phantom_dirty(")
            .expect("유령 dirty 정리");
        let return_tabs = return_body.find("service::return_auxiliary_window_tabs(").expect("탭 복귀");
        let write = return_body.find("*state.layouts.write() = layouts;").expect("layout 기록");
        assert!(return_body.contains("sink.publish(AppEvent::LayoutChanged { project_id, revision })"));
        let emit = return_body
            .find(".publish(AppEvent::LayoutChanged { project_id, revision })")
            .expect("변경 이벤트");
        assert!(guard < mirrors);
        assert!(mirrors < clear_dirty);
        assert!(clear_dirty < return_tabs);
        assert!(return_tabs < write);
        assert!(write < emit);

        let events = extract_between(
            source,
            ".on_window_event(|window, event| match event {",
            ".build(tauri::generate_context!())",
        );
        assert_eq!(events.matches("plan_return_of_auxiliary_window_tabs(").count(), 2);
        assert!(events.contains("domain::window::commands::handle_close_requested(window, api)"));

        let window_commands = include_str!("domain/window/commands.rs");
        let close_body = extract_between(window_commands, "fn handle_auxiliary_close_requested(", "/// Re-issues the close");
        assert!(close_body.contains("window.state::<WindowRegistry>().forget(window.label())"));
    }

    /// `Project.capabilities` 동작 고정 — the registry's `detected_kinds` is the field's single
    /// source (`project_open` injects it into `project::service::open_project`, which records the
    /// result verbatim), so this pins the exact values the real registry produces for a git and a
    /// non-git root: the same `[Git?, Terminal]` the old hand-coded detection in `open_project`
    /// recorded, proving the single-sourcing changed no serialized behavior.
    #[test]
    fn 등록된_capability_registry가_open_project의_capabilities를_결정한다() {
        use domain::project::types::CapabilityKind;

        let registry = project_capabilities();

        for git_repo in [true, false] {
            let data_dir = std::env::temp_dir().join(format!("taide-cap-parity-{}", uuid::Uuid::new_v4()));
            let workspace = data_dir.join("workspace");
            std::fs::create_dir_all(&workspace).expect("create workspace");
            if git_repo {
                std::fs::create_dir_all(workspace.join(".git")).expect("create .git");
            }

            let paths = AppPaths::new(data_dir.clone());
            let mut session = domain::project::types::SessionState::default();
            let mut projects = std::collections::HashMap::new();
            let opened = domain::project::service::open_project(&paths, &mut session, &mut projects, &workspace, true, |root| {
                registry.detected_kinds(root)
            })
            .expect("open project");

            let expected = if git_repo {
                vec![CapabilityKind::Git, CapabilityKind::Terminal]
            } else {
                vec![CapabilityKind::Terminal]
            };
            assert_eq!(
                opened.project.capabilities, expected,
                "git_repo={git_repo}: 레지스트리가 기록한 capabilities 가 기존 수기 검출과 다릅니다"
            );

            std::fs::remove_dir_all(&data_dir).ok();
        }
    }

    /// `X1#8` — the `event:name` wire strings the frontend subscribes to
    /// (`src/shared/api/bindings.ts`'s generated `events` export) must be exactly the `event_name`s
    /// declared on the Rust side.
    #[test]
    fn 이벤트_이름_문자열은_events_rs와_bindings_ts에서_일치한다() {
        let declared: BTreeSet<String> = event_name_by_type().into_values().collect();

        let bindings_source = include_str!("../../src/shared/api/bindings.ts");
        let events_block = extract_between(bindings_source, "export const events = {", "};");
        let pattern = Regex::new(r#"makeEvent<[A-Za-z0-9_]+>\("([a-zA-Z0-9:_-]+)"\)"#).expect("유효한 정규식");
        let bound: BTreeSet<String> = pattern.captures_iter(events_block).map(|capture| capture[1].to_string()).collect();

        assert_eq!(
            declared, bound,
            "events.rs 의 event_name 집합과 bindings.ts 의 events export 집합이 다릅니다"
        );
    }
}

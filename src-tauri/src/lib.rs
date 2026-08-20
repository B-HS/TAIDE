pub mod constants;
pub mod domain;
pub mod error;
pub mod events;
pub mod ids;
pub mod infra;
pub mod paths;
pub mod state;

use std::collections::{HashMap, HashSet};

use tauri::{Listener, Manager};
use tauri_specta::Event as _;
use tauri_specta::{collect_commands, collect_events, Builder};

use crate::domain::agent::commands::{AgentHooksStore, AgentStore};
use crate::domain::ai::commands::AiRequestStore;
use crate::domain::git::commands::GitStore;
use crate::domain::ide::store::IdeStore;
use crate::domain::lsp::commands::{LspInstallStore, LspStore};
use crate::domain::plugin::service::PluginStore;
use crate::domain::remote::commands::RemoteStore;
use crate::domain::search::commands::SearchStore;
use crate::domain::system::commands::SystemUsageStore;
use crate::domain::terminal::commands::TerminalStore;
use crate::domain::tree::commands::TreeStore;
use crate::domain::window::commands::WindowStore;
use crate::events::{
    AgentExternalOpen, AgentStateChanged, FsChanged, GitRefsChanged, GitStatusChanged, HotExitFlushRequested, IdeCloseTabRequested,
    IdeDiffRequested, IdeSaveRequested, IdeStatusChanged, LayoutChanged, LspInstallProgress, LspSessionStatusChanged, ProjectActivated,
    ProjectClosed, ProjectListChanged, ProjectOpened, RemoteStateChanged, SettingsChanged, SyncStateChanged, TerminalCwdChanged,
    TerminalExited, ThemeChanged,
};
use crate::ids::ProjectId;
use crate::infra::secret::SecretStoreState;
use crate::paths::AppPaths;
use crate::state::AppState;

const BINDINGS_PATH: &str = "../src/shared/api/bindings.ts";

/// A custom (non-predefined) menu item id for the app menu's Quit entry —
/// see [`handle_menu_event`] for why this can't be `PredefinedMenuItem::quit`.
const MENU_ID_QUIT: &str = "taide-quit";

fn build_app_menu(handle: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

    let quit_item = MenuItemBuilder::with_id(MENU_ID_QUIT, "Quit")
        .accelerator("CmdOrCtrl+Q")
        .build(handle)?;

    let app_menu = SubmenuBuilder::new(handle, "TAIDE")
        .item(&PredefinedMenuItem::about(handle, None, None)?)
        .separator()
        .item(&PredefinedMenuItem::services(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(handle, None)?)
        .item(&PredefinedMenuItem::hide_others(handle, None)?)
        .item(&PredefinedMenuItem::show_all(handle, None)?)
        .separator()
        .item(&quit_item)
        .build()?;

    let edit_menu = SubmenuBuilder::new(handle, "Edit")
        .item(&PredefinedMenuItem::undo(handle, None)?)
        .item(&PredefinedMenuItem::redo(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::cut(handle, None)?)
        .item(&PredefinedMenuItem::copy(handle, None)?)
        .item(&PredefinedMenuItem::paste(handle, None)?)
        .item(&PredefinedMenuItem::select_all(handle, None)?)
        .build()?;

    let window_menu = SubmenuBuilder::new(handle, "Window")
        .item(&PredefinedMenuItem::minimize(handle, None)?)
        .item(&PredefinedMenuItem::maximize(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(handle, None)?)
        .build()?;

    MenuBuilder::new(handle).items(&[&app_menu, &edit_menu, &window_menu]).build()
}

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

/// Wires `pty_spawn`'s extra-env hook to the IDE integration: every new terminal inherits the
/// Claude Code SSE port when the IDE server is (or comes) up — `ide::store::claude_terminal_env`
/// owns the readiness wait, the terminal domain only injects the result (audit R8#10, T1-I §1.4).
fn pty_spawn_env_provider() -> domain::terminal::commands::PtySpawnEnvProvider {
    use domain::terminal::commands::PtySpawnEnvFuture;

    domain::terminal::commands::PtySpawnEnvProvider::new(Box::new(|app| -> PtySpawnEnvFuture<'_> {
        Box::pin(domain::ide::store::claude_terminal_env(app))
    }))
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            domain::app::commands::app_get_info,
            domain::project::commands::project_list,
            domain::project::commands::project_list_recent,
            domain::project::commands::project_get,
            domain::project::commands::project_get_active,
            domain::project::commands::project_open,
            domain::project::commands::project_close,
            domain::project::commands::project_activate,
            domain::project::commands::project_reorder,
            domain::layout::commands::layout_get,
            domain::layout::commands::layout_open_tab,
            domain::layout::commands::layout_close_tab,
            domain::layout::commands::layout_activate_tab,
            domain::layout::commands::layout_move_tab,
            domain::layout::commands::layout_split,
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
            domain::layout::commands::layout_move_tab_to_window,
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
            domain::tree::commands::tree_reveal,
            domain::tree::commands::tree_refresh,
            domain::search::commands::search_run,
            domain::search::commands::search_cancel,
            domain::search::commands::search_replace,
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
        ])
        .events(collect_events![
            ProjectOpened,
            ProjectClosed,
            ProjectActivated,
            ProjectListChanged,
            LayoutChanged,
            ThemeChanged,
            FsChanged,
            TerminalExited,
            TerminalCwdChanged,
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
}

const LAYOUT_FLUSH_INTERVAL_MS: u64 = 2_000;

pub(crate) const RAW_CHANNEL_COMMANDS: &[&str] = &["pty_spawn", "pty_attach", "file_read_raw"];

fn flush_dirty_layouts(state: &AppState) {
    let dirty: Vec<_> = state.dirty_layouts.write().drain().collect();
    if dirty.is_empty() {
        return;
    }

    let snapshots: Vec<_> = {
        let layouts = state.layouts.read();
        dirty
            .into_iter()
            .filter_map(|project_id| match layouts.get(&project_id) {
                Some(layout) => Some((project_id, layout.clone())),
                None => {
                    log::warn!("dirty_layouts 에 등록된 프로젝트의 레이아웃을 찾을 수 없어 저장을 건너뜁니다: {project_id}");
                    None
                }
            })
            .collect()
    };

    for (project_id, layout) in snapshots {
        if let Err(error) = domain::layout::service::save_layout(&state.paths, &project_id, &layout) {
            log::warn!("레이아웃 저장 실패 ({project_id}): {error}");
        }
    }
}

/// Handles `CloseRequested` for an auxiliary editor window (`editor-<n>`).
/// Before Wave I every window funneled into the same hot-exit flush/exit
/// sequence `handle_close_requested` runs for the main window below, so
/// closing a second window silently terminated the whole app
/// (`docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §2.1).
/// The close is intentionally left un-prevented here so the OS's default
/// close proceeds and only this one window goes away. Both registry cleanups
/// are idempotent with the `Destroyed` handler below, which runs this same
/// cleanup again as a backstop for closes that don't go through
/// `CloseRequested` at all (e.g. a crash).
fn handle_auxiliary_close_requested(window: &tauri::Window<tauri::Wry>) {
    if window.state::<AppState>().forget_hot_exit_flush_window(window.label()) {
        window.app_handle().exit(0);
    }

    if let Some((project_id, window_slot)) = window.state::<WindowStore>().forget(window.label()) {
        domain::window::service::plan_return_of_auxiliary_window_tabs(&window.app_handle().clone(), &project_id, window_slot);
    }
}

/// Intercepts the window close request to give every open window a chance to
/// flush its own dirty editor models to the hot-exit mirror before the app
/// actually exits. Only the main window runs this path — see
/// [`handle_auxiliary_close_requested`] for `editor-*` windows, which close
/// immediately instead. Always defers the main window's close
/// (`prevent_close`) and lets either every expected window's own
/// `file_flush_complete` call or the timeout fallback below perform the real
/// `AppHandle::exit`, so the window is never destroyed by the OS's default
/// close path. `AppState::begin_hot_exit_flush` guards re-entrant close
/// attempts (e.g. mashing Cmd+Q) from emitting the flush event twice.
fn handle_close_requested(window: &tauri::Window<tauri::Wry>, api: &tauri::CloseRequestApi) {
    if domain::window::service::is_auxiliary_label(window.label()) {
        handle_auxiliary_close_requested(window);
        return;
    }

    api.prevent_close();

    let state = window.state::<AppState>();
    // `windows()` needs the `unstable` Tauri feature this project doesn't enable;
    // `webview_windows()` is the stable equivalent and every window here is a webview window.
    let expected_windows: HashSet<String> = window.webview_windows().into_keys().collect();
    if !state.begin_hot_exit_flush(expected_windows) {
        return;
    }

    // `Event::emit` broadcasts to every window/webview app-wide regardless of which handle it's
    // called through, so every currently-open auxiliary window already receives this alongside
    // the main window — no separate per-window fanout loop is needed here.
    let _ = HotExitFlushRequested {
        timeout_ms: constants::HOT_EXIT_FLUSH_TIMEOUT_MS as f64,
    }
    .emit(window);

    let app_handle = window.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(constants::HOT_EXIT_FLUSH_TIMEOUT_MS)).await;
        if app_handle.state::<AppState>().force_complete_hot_exit_flush() {
            log::warn!("hot exit flush timed out; exiting without every window's confirmation");
            app_handle.exit(0);
        }
    });
}

/// Routes the app menu's Quit item through the same `WindowEvent::CloseRequested`
/// flush handshake as clicking the window's close button, instead of the native
/// `NSApplication terminate:` binding `PredefinedMenuItem::quit` uses on macOS.
/// `terminate:` runs straight to `applicationWillTerminate` and `RunEvent::Exit`
/// with no `CloseRequested` (or preventable `ExitRequested`) event in between —
/// tao's macOS app delegate has no `applicationShouldTerminate:` hook to catch it
/// — so Cmd+Q would otherwise skip `handle_close_requested` entirely and drop
/// any hot-exit mirror writes still pending in the last debounce window.
fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    if event.id() != MENU_ID_QUIT {
        return;
    }
    if let Some(window) = app.get_webview_window(constants::MAIN_WINDOW_LABEL) {
        let _ = window.close();
    }
}

/// Recreates every `AuxWindowLayout` recorded across every restored (non-`root_missing`) project as
/// a real OS window — the boot-time half of "보조 창 레이아웃 영속·재시작 시 복원(창 재생성)"
/// (contract §3.2). Spawns one task per window rather than awaiting them serially so restoration
/// doesn't block the rest of `setup()`; each task still takes `AppState::begin_mutation` for the
/// duration of its own `window::commands::open_auxiliary_window` call, so concurrent restorations
/// serialize through that single guard instead of racing to coin the same `editor-<n>` label (see
/// that function's doc comment).
fn restore_auxiliary_windows(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let project_ids: Vec<ProjectId> = state
        .projects
        .read()
        .iter()
        .filter(|(_, project)| !project.root_missing)
        .map(|(project_id, _)| project_id.clone())
        .collect();

    let layouts = state.layouts.read();
    let restorations: Vec<(ProjectId, u32)> = project_ids
        .into_iter()
        .filter_map(|project_id| {
            layouts
                .get(&project_id)
                .map(|layout| (project_id, layout.auxiliary_windows.clone()))
        })
        .flat_map(|(project_id, windows)| windows.into_iter().map(move |window| (project_id.clone(), window.slot)))
        .collect();
    drop(layouts);

    for (project_id, window_slot) in restorations {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_handle.state::<AppState>();
            let _guard = state.begin_mutation().await;
            let windows = app_handle.state::<WindowStore>();
            if let Err(error) =
                domain::window::commands::open_auxiliary_window(&app_handle, &state, &windows, project_id.clone(), window_slot).await
            {
                log::warn!("보조 창 복원 실패 (projectId={project_id}, windowSlot={window_slot}): {error}");
            }
        });
    }
}

/// The exact snapshot [`restore_project_watchers`] attaches from — every restored project whose
/// root is still present on disk (`!root_missing`), paired with the root path its watcher needs.
/// Kept as its own pure function (rather than inline in `setup()`) so this selection is
/// unit-testable without a running `AppHandle` — see `restore_project_watchers`'s doc for why the
/// attach itself can't be (this codebase has no `tauri::test` mock-app harness — the same
/// constraint `domain::terminal::commands`'s own tests document and work around).
fn projects_pending_watcher_restore(projects: &HashMap<ProjectId, domain::project::types::Project>) -> Vec<(ProjectId, String)> {
    projects
        .values()
        .filter(|project| !project.root_missing)
        .map(|project| (project.id.clone(), project.root.clone()))
        .collect()
}

/// Re-attaches the file watcher and (where the project is a git repo) the git watcher for every
/// project [`projects_pending_watcher_restore`] selected, as a background task that starts only
/// after `app.manage(state)` in `setup()` below — so the multi-second `notify-debouncer-full`
/// `FileIdMap` walk each `attach_watcher`/`attach_git_watcher` call performs (the dominant
/// boot-latency cause identified in
/// `docs/acknowledge/2026-08-20-boot-watcher-defer-contract.md`) never delays window creation the
/// way the old fully-synchronous loop here did. `restore_state` and every `app.manage` call stay
/// synchronous in `setup()` — the tree and editor need the projects/layouts they populate
/// immediately — only the watcher attach itself moves here. `project_open`'s own synchronous
/// attach (via `ProjectCapabilities::attach_all`) is unchanged; this function only covers boot
/// restore.
///
/// Every fs change landing in the gap between window show and a project's own attach completing
/// here has nowhere to go: no watcher is listening yet, and `notify` never replays history once
/// one starts. `tree_rows` (`domain::tree::commands::rows_page_from_store`) tolerates this without
/// help — its cache starts empty every boot (`TreeStore::default()`), so the very first read for
/// any project always rescans disk directly regardless of watcher state, and only a *second*
/// gap-window change landing after that first read stays unseen until some later event touches the
/// same directory — the same bounded race `project_open`'s own synchronous attach already carries
/// between "watcher starts" and the debouncer's first tick landing. `git_status` has no such
/// self-healing re-read: `entities/git/git.query.ts`'s `GIT.PROJECT`-scoped cache only ever
/// refreshes on a `GitStatusChanged`/`GitRefsChanged` event or the global 60s `staleTime`
/// (`app/query-client.ts`), so a gap-window git change with no *further* change afterward would sit
/// stale far longer than that. The `git_watchers.read().contains_key(...)` branch below closes
/// exactly that gap with one synthetic refresh the instant this project's git watcher goes live —
/// a no-op for the frontend when nothing actually changed, since `invalidateQueries` against a
/// query nobody is displaying does nothing.
///
/// One `AppState::begin_mutation` guard per project, held across that project's own
/// `spawn_blocking`-dispatched attach exactly as `git_commit` holds it across its own
/// `spawn_blocking`-dispatched `git` subprocess call (`domain::git::commands::git_commit`, Phase E
/// T1H-C-02): the `FileIdMap` walk itself runs off the async reactor thread inside
/// `spawn_blocking`, while the guard still serializes this attach against a concurrent
/// `project_open`/`project_close` — the same protection the old fully-synchronous boot loop got for
/// free by running before `state` was even `app.manage`d. The `contains_key` check right after
/// acquiring the guard catches a project closed between the snapshot the caller took and this loop
/// actually reaching it; nothing else can shrink `state.projects` while the guard is held, so that
/// one check is enough — no second check is needed once the blocking attach returns.
///
/// Projects attach strictly one after another rather than fanned out per-project like
/// `restore_auxiliary_windows`: each project's watcher only ever touches that project's own
/// `state.watchers`/`state.git_watchers` entry, so nothing depends on relative order across
/// projects, and sequential keeps the guard's hold on any single `project_open`/`project_close`
/// call bounded to one project's walk instead of the whole restored set.
fn restore_project_watchers(app: &tauri::AppHandle, restored: Vec<(ProjectId, String)>) {
    let app_handle = app.clone();
    let project_count = restored.len();

    tauri::async_runtime::spawn(async move {
        let started = std::time::Instant::now();

        for (project_id, root) in restored {
            let state = app_handle.state::<AppState>();
            let _guard = state.begin_mutation().await;
            if !state.projects.read().contains_key(&project_id) {
                continue;
            }

            let blocking_handle = app_handle.clone();
            let blocking_project_id = project_id.clone();
            let attach_result = tauri::async_runtime::spawn_blocking(move || {
                let state = blocking_handle.state::<AppState>();
                domain::file::capability::attach_watcher(&blocking_handle, &state, &blocking_project_id, &root);
                domain::git::watch::attach_git_watcher(&blocking_handle, &state, &blocking_project_id, &root);
            })
            .await;

            match attach_result {
                Ok(()) => {
                    if state.git_watchers.read().contains_key(&project_id) {
                        let _ = GitStatusChanged {
                            project_id: project_id.clone(),
                        }
                        .emit(&app_handle);
                        let _ = GitRefsChanged {
                            project_id: project_id.clone(),
                        }
                        .emit(&app_handle);
                    }
                }
                Err(error) => log::warn!("복원 프로젝트 워처 attach 태스크가 실패했습니다 (projectId={project_id}): {error}"),
            }
        }

        log::info!(
            "복원 프로젝트 워처 attach 완료: projects={project_count}, elapsed_ms={}",
            started.elapsed().as_millis()
        );
    });
}

async fn poll_agents(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let terminals = app.state::<TerminalStore>();
    let agents = app.state::<AgentStore>();
    let agent_hooks = app.state::<AgentHooksStore>();

    let project_ids: Vec<_> = state.projects.read().keys().cloned().collect();
    let mut valid_session_ids = std::collections::HashSet::new();

    for project_id in project_ids {
        let pids = terminals.foreground_pids(&project_id);
        let Ok(probes) = domain::agent::commands::detect_agents_for_pids_blocking(pids).await else {
            continue;
        };
        let detected = domain::agent::commands::build_detected_agents(&agents, &agent_hooks, &project_id, probes);
        valid_session_ids.extend(detected.iter().map(|agent| agent.session_id.clone()));

        if let Some(changed) = agents.diff(&project_id, &detected) {
            let _ = AgentStateChanged {
                project_id: project_id.clone(),
                agents: changed,
            }
            .emit(app);
        }
    }

    agents.prune_activity(&valid_session_ids);
}

fn restore_state(state: &AppState) -> Vec<String> {
    let mut warnings = Vec::new();

    match domain::project::service::restore_session(&state.paths) {
        Ok((session, projects, session_warnings)) => {
            let mut layouts = state.layouts.write();
            for project in &projects {
                layouts.insert(project.id.clone(), domain::layout::service::load_layout(&state.paths, &project.id));
            }
            drop(layouts);

            *state.session.write() = session;
            *state.projects.write() = projects.into_iter().map(|project| (project.id.clone(), project)).collect();
            warnings.extend(session_warnings);
        }
        Err(error) => warnings.push(format!("세션 복원 실패: {error}")),
    }

    *state.settings.write() = domain::settings::service::load_settings(&state.paths);

    warnings
}

/// Handles a cold-start `taide <file>` invocation. `tauri-plugin-single-instance` only forwards
/// argv to a *second* launch's callback, so a cold start that spawns the app fresh never reaches
/// it; this queues the request into `AgentStore` instead so the frontend can drain it on boot.
fn queue_cold_start_external_open(app_handle: &tauri::AppHandle) {
    let argv: Vec<String> = std::env::args().collect();
    let Some(request) = domain::agent::service::parse_cli_payload(&argv) else {
        return;
    };

    let agent_store = app_handle.state::<AgentStore>();
    if let Some(marker) = request.wait_marker.clone() {
        agent_store.register_wait_marker(marker);
    }
    agent_store.push_pending_external_open(request);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

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
                let agent_store = app_handle.state::<AgentStore>();
                if let Some(marker) = request.wait_marker.clone() {
                    agent_store.register_wait_marker(marker);
                }
                agent_store.push_pending_external_open(request.clone());
                let _ = AgentExternalOpen { request }.emit(app_handle);
            }
        }));
    }

    let log_plugin = tauri_plugin_log::Builder::new()
        .level_for("tungstenite", log::LevelFilter::Warn)
        .level_for("tokio_tungstenite", log::LevelFilter::Warn)
        .build();

    app.plugin(log_plugin)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
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
            infra::asset_protocol::respond(&projects, request)
        })
        .on_menu_event(handle_menu_event)
        .invoke_handler(move |invoke| {
            if RAW_CHANNEL_COMMANDS.contains(&invoke.message.command()) {
                raw_channel_handler(invoke)
            } else {
                specta_handler(invoke)
            }
        })
        .setup(move |app| {
            let setup_started = std::time::Instant::now();

            domain::locale::service::warm_builtin_catalogs();
            builder.mount_events(app);
            app.set_menu(build_app_menu(app.handle())?)?;

            let data_dir = app.path().app_data_dir().expect("app data dir unavailable");
            let state = AppState::new(AppPaths::new(data_dir));

            for warning in restore_state(&state) {
                log::warn!("{warning}");
            }

            let restored = projects_pending_watcher_restore(&state.projects.read());

            app.manage(state);
            app.manage(project_capabilities());
            app.manage(settings_toggle_observers());
            app.manage(system_usage_label_providers());
            app.manage(pty_spawn_env_provider());
            app.manage(TreeStore::default());
            app.manage(TerminalStore::default());
            app.manage(GitStore::default());
            app.manage(LspStore::default());
            app.manage(LspInstallStore::default());
            app.manage(SearchStore::default());
            app.manage(PluginStore::default());
            app.manage(AgentStore::default());
            app.manage(AgentHooksStore::default());
            app.manage(SystemUsageStore::default());
            app.manage(IdeStore::default());
            app.manage(AiRequestStore::default());
            app.manage(SecretStoreState::default());
            app.manage(RemoteStore::default());
            app.manage(WindowStore::default());
            restore_auxiliary_windows(app.handle());
            restore_project_watchers(app.handle(), restored);
            domain::remote::commands::refresh_password_configured_cache(app.handle());

            queue_cold_start_external_open(app.handle());

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
                LayoutChanged,
                ThemeChanged,
                FsChanged,
                TerminalExited,
                TerminalCwdChanged,
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
                tauri::async_runtime::spawn(async move {
                    domain::agent::hooks::reconcile_installed_hooks(&hooks_boot_handle).await;
                });
            }

            if app.state::<AppState>().settings.read().ide_integration_enabled {
                let ide_boot_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
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
                tauri::async_runtime::spawn(async move {
                    if let Err(error) =
                        domain::remote::commands::remote_start(remote_boot_handle.clone(), remote_boot_handle.state::<RemoteStore>()).await
                    {
                        log::warn!("원격 접속 서버 자동 시작 실패: {error}");
                    }
                });
            }

            let ide_reconcile_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(domain::ide::types::IDE_RECONCILE_INTERVAL_MS));
                loop {
                    ticker.tick().await;
                    domain::ide::commands::reconcile_stale_pending(&ide_reconcile_handle);
                }
            });

            let agent_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let interval = if cfg!(windows) {
                    domain::agent::types::AGENT_POLL_WINDOWS_MS
                } else {
                    domain::agent::types::AGENT_POLL_UNIX_MS
                };
                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval));
                loop {
                    ticker.tick().await;
                    poll_agents(&agent_handle).await;
                }
            });

            let flush_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(std::time::Duration::from_millis(LAYOUT_FLUSH_INTERVAL_MS));
                loop {
                    ticker.tick().await;
                    flush_dirty_layouts(&flush_handle.state::<AppState>());
                }
            });

            log::info!("setup 완료: elapsed_ms={}", setup_started.elapsed().as_millis());

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Destroyed => {
                flush_dirty_layouts(&window.state::<AppState>());
                if window.state::<AppState>().forget_hot_exit_flush_window(window.label()) {
                    window.app_handle().exit(0);
                }
                if let Some((project_id, window_slot)) = window.state::<WindowStore>().forget(window.label()) {
                    domain::window::service::plan_return_of_auxiliary_window_tabs(&window.app_handle().clone(), &project_id, window_slot);
                }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => handle_close_requested(window, api),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
                flush_dirty_layouts(&app_handle.state::<AppState>());
                app_handle.state::<TerminalStore>().kill_all();
                app_handle.state::<LspStore>().kill_all();
                domain::agent::commands::cleanup_all_wait_markers(&app_handle.state::<AgentStore>());
                domain::ide::commands::stop_server(app_handle, &app_handle.state::<IdeStore>());
                domain::remote::commands::stop_server(app_handle, &app_handle.state::<RemoteStore>());
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

        let implemented_names: BTreeSet<String> = domain::remote::dispatch::IMPLEMENTED_JSON_COMMANDS
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
        assert_eq!(declared.len(), 23, "events.rs 에 선언된 이벤트 구조체 수가 23종에서 벗어났습니다");

        let collected = identifier_set(extract_between(include_str!("lib.rs"), "collect_events![", "]"));

        assert_eq!(
            declared, collected,
            "events.rs 의 이벤트 구조체 집합과 lib.rs 의 collect_events! 인자 집합이 다릅니다"
        );
    }

    /// `X1#8` — `fanout_remote_events!` (`lib.rs`) deliberately omits two events from the
    /// remote-session broadcast: `HotExitFlushRequested`(데스크톱 창 종료 신호라 원격 세션에는
    /// 무의미) and `AgentExternalOpen`(T0 #14 — 원격 세션이 대기 중인 외부 열기 요청을 실시간으로
    /// 가로채면 안 된다. `domain/remote/dispatch.rs` 의 `deny_remote_agent_pending_external_opens`
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
            let opened = domain::project::service::open_project(&paths, &mut session, &mut projects, &workspace, |root| {
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

    fn stub_project(id: &str, root: &str, root_missing: bool) -> domain::project::types::Project {
        domain::project::types::Project {
            id: ProjectId::from(id.to_string()),
            root: root.to_string(),
            name: id.to_string(),
            capabilities: Vec::new(),
            root_missing,
            last_opened_at: 0.0,
        }
    }

    /// Pins [`restore_project_watchers`]'s attach-target selection — the boundary the boot-watcher
    /// defer contract (`docs/acknowledge/2026-08-20-boot-watcher-defer-contract.md`) actually lets
    /// this codebase unit-test, since `attach_watcher`/`attach_git_watcher` themselves need a real
    /// `AppHandle` this codebase has no mock-app harness for.
    #[test]
    fn 워처_재부착_대상은_루트가_존재하는_복원_프로젝트만_포함한다() {
        let projects: HashMap<ProjectId, domain::project::types::Project> = [
            stub_project("prj-present", "/repo/present", false),
            stub_project("prj-missing", "/repo/missing", true),
        ]
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

        let pending = projects_pending_watcher_restore(&projects);

        assert_eq!(
            pending,
            vec![(ProjectId::from("prj-present".to_string()), "/repo/present".to_string())],
            "root_missing 프로젝트는 워처 재부착 대상에서 제외되어야 합니다"
        );
    }

    #[test]
    fn 복원된_프로젝트가_없으면_워처_재부착_대상도_비어있다() {
        let projects: HashMap<ProjectId, domain::project::types::Project> = HashMap::new();

        assert!(
            projects_pending_watcher_restore(&projects).is_empty(),
            "복원된 프로젝트가 없으면 재부착 대상도 없어야 합니다"
        );
    }
}

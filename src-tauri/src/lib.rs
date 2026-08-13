pub mod constants;
pub mod domain;
pub mod error;
pub mod events;
pub mod ids;
pub mod infra;
pub mod paths;
pub mod state;

use tauri::{Listener, Manager};
use tauri_specta::Event as _;
use tauri_specta::{collect_commands, collect_events, Builder};

use crate::domain::agent::commands::{AgentHooksStore, AgentStore};
use crate::domain::ai::commands::AiInlineStore;
use crate::domain::git::commands::GitStore;
use crate::domain::ide::commands::IdeStore;
use crate::domain::lsp::commands::{LspInstallStore, LspStore};
use crate::domain::plugin::commands::PluginStore;
use crate::domain::remote::commands::RemoteStore;
use crate::domain::search::commands::SearchStore;
use crate::domain::system::commands::SystemUsageStore;
use crate::domain::terminal::commands::TerminalStore;
use crate::domain::tree::commands::TreeStore;
use crate::events::{
    AgentExternalOpen, AgentStateChanged, AppReady, FsChanged, GitRefsChanged, GitStatusChanged, IdeCloseTabRequested, IdeDiffRequested,
    IdeSaveRequested, IdeStatusChanged, LayoutChanged, LspInstallProgress, LspSessionStatusChanged, ProjectActivated, ProjectClosed,
    ProjectFocusKindChanged, ProjectListChanged, ProjectOpened, RemoteStateChanged, SyncStateChanged, TerminalCwdChanged, TerminalExited,
    ThemeChanged,
};
use crate::infra::secret::SecretStoreState;
use crate::paths::AppPaths;
use crate::state::AppState;

const BINDINGS_PATH: &str = "../src/shared/api/bindings.ts";

fn build_app_menu(handle: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, PredefinedMenuItem, SubmenuBuilder};

    let app_menu = SubmenuBuilder::new(handle, "TAIDE")
        .item(&PredefinedMenuItem::about(handle, None, None)?)
        .separator()
        .item(&PredefinedMenuItem::services(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(handle, None)?)
        .item(&PredefinedMenuItem::hide_others(handle, None)?)
        .item(&PredefinedMenuItem::show_all(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(handle, None)?)
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

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            domain::app::commands::app_get_info,
            domain::project::commands::project_list,
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
            domain::file::commands::file_open,
            domain::file::commands::file_save,
            domain::file::commands::file_create,
            domain::file::commands::file_rename,
            domain::file::commands::file_delete,
            domain::file::commands::file_copy,
            domain::file::commands::file_mirror_dirty,
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
            domain::lsp::commands::lsp_sessions,
            domain::lsp::commands::lsp_detect_servers,
            domain::lsp::commands::lsp_resolve_root,
            domain::lsp::commands::lsp_install,
            domain::lsp::commands::lsp_install_cancel,
            domain::git::commands::git_init,
            domain::git::commands::git_status,
            domain::git::commands::git_diff_file,
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
            domain::terminal::commands::pty_default_options,
            domain::terminal::commands::pty_write,
            domain::terminal::commands::pty_resize,
            domain::terminal::commands::pty_kill,
            domain::terminal::commands::pty_set_paused,
            domain::terminal::commands::terminal_sessions,
            domain::terminal::commands::shell_profiles,
            domain::terminal::commands::resolve_terminal_path,
            domain::font::commands::font_list,
            domain::locale::commands::locale_list,
            domain::locale::commands::locale_get,
            domain::locale::commands::locale_get_current,
            domain::theme::commands::theme_list,
            domain::theme::commands::theme_get,
            domain::theme::commands::theme_get_current,
            domain::theme::commands::theme_save,
            domain::theme::commands::theme_delete,
            domain::settings::commands::settings_get,
            domain::settings::commands::settings_update,
            domain::settings::commands::settings_set_theme,
            domain::system::commands::system_usage_get,
            domain::system::commands::system_open_path,
            domain::system::commands::system_reveal_path,
            domain::system::commands::system_open_in_browser,
            domain::system::commands::system_open_app_data_path,
            domain::ide::commands::ide_get_status,
            domain::ide::commands::ide_start,
            domain::ide::commands::ide_stop,
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
            domain::ai::commands::ai_inline_cancel,
            domain::sync::commands::sync_status,
            domain::sync::commands::sync_connect,
            domain::sync::commands::sync_disconnect,
            domain::sync::commands::sync_upload,
            domain::sync::commands::sync_download,
            domain::vsix::commands::vsix_extract_themes,
            domain::remote::commands::remote_status,
            domain::remote::commands::remote_start,
            domain::remote::commands::remote_stop,
            domain::remote::commands::remote_issue_link,
            domain::remote::commands::remote_revoke_sessions,
        ])
        .events(collect_events![
            AppReady,
            ProjectOpened,
            ProjectClosed,
            ProjectActivated,
            ProjectListChanged,
            ProjectFocusKindChanged,
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
            RemoteStateChanged
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
            .filter_map(|project_id| layouts.get(&project_id).cloned().map(|layout| (project_id, layout)))
            .collect()
    };

    for (project_id, layout) in snapshots {
        if let Err(error) = domain::layout::service::save_layout(&state.paths, &project_id, &layout) {
            log::warn!("레이아웃 저장 실패 ({project_id}): {error}");
        }
    }
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
            if let Some(window) = app_handle.get_webview_window("main") {
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
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .invoke_handler(move |invoke| {
            if RAW_CHANNEL_COMMANDS.contains(&invoke.message.command()) {
                raw_channel_handler(invoke)
            } else {
                specta_handler(invoke)
            }
        })
        .setup(move |app| {
            builder.mount_events(app);
            app.set_menu(build_app_menu(app.handle())?)?;

            let data_dir = app.path().app_data_dir().expect("app data dir unavailable");
            let state = AppState::new(AppPaths::new(data_dir));

            for warning in restore_state(&state) {
                log::warn!("{warning}");
            }

            let restored: Vec<_> = state
                .projects
                .read()
                .values()
                .filter(|project| !project.root_missing)
                .map(|project| (project.id.clone(), project.root.clone()))
                .collect();
            for (project_id, root) in &restored {
                domain::project::commands::allow_asset_access(app.handle(), root);
                domain::project::commands::attach_watcher(app.handle(), &state, project_id, root);
                domain::project::commands::attach_git_watcher(app.handle(), &state, project_id, root);
            }

            app.manage(state);
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
            app.manage(AiInlineStore::default());
            app.manage(SecretStoreState::default());
            app.manage(RemoteStore::default());

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
                AppReady,
                ProjectOpened,
                ProjectClosed,
                ProjectActivated,
                ProjectListChanged,
                ProjectFocusKindChanged,
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

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                flush_dirty_layouts(&window.state::<AppState>());
            }
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
    use super::*;

    #[test]
    fn typescript_바인딩을_생성한다() {
        specta_builder()
            .export(specta_typescript::Typescript::default(), BINDINGS_PATH)
            .expect("failed to export typescript bindings");
    }
}

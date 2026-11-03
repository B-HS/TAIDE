use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, Ui};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId};
use taide_model::layout::{Tab, TabKind};
use taide_model::lsp::LspServerId;
use taide_model::notification::NotificationCategory;
use taide_model::remote::RemoteStatus;
use taide_model::sync::SyncStatus;
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::intents::settings::Intent;
use taide_native_ui::settings_sections_data as sections;
use taide_native_ui::shell::WindowScope;
use taide_runtime::{
    AppServices, agent_hook_actions, ai_actions, app_actions, layout_actions, lsp_actions,
    notification_actions, plugin_actions, remote_actions, settings_actions, sync_actions,
    system_actions, theme_actions, vsix_actions,
};

use crate::application::{AppSurfaces, NativeApplication, editor_error};
use crate::host::{ClipboardWriter, HostCommand, HostReply, HostWorker};
use crate::presentation;

const SECTIONS_DISCONNECTED: &str = "native Settings section host is disconnected or full";
const PLUGIN_ARCHIVE_FILTER: &str = "Plugin Archive";
const PLUGIN_ARCHIVE_EXTENSIONS: &[&str] = &["zip"];
const VSIX_FILTER: &str = "VSIX";
const VSIX_EXTENSIONS: &[&str] = &["vsix"];

pub enum Command {
    ReadAppFile(crate::app_file::ReadRequest),
    WriteAppFile(crate::app_file_write::WriteRequest),
    OpenAppFile {
        project: ProjectId,
        pane: PaneId,
        target: taide_model::app::AppFileTarget,
        title: String,
    },
    RefreshPresentation(Box<crate::presentation_refresh::Request>),
    ReadSettingsCatalog(crate::settings_view::Request),
    ReadSettingsResource {
        request: taide_native_ui::settings_resources::Request,
        ports: Arc<crate::settings_ports::Ports>,
    },
    ThemeEdit(crate::theme_edit::Command),
    SnippetEdit(taide_native_ui::snippet_edit::Request),
    ReadSnippetCatalog(taide_native_ui::snippet_catalog::Request),
    OpenSettings {
        project: ProjectId,
        pane: PaneId,
        title: String,
    },
    OpenSettingsFolder(taide_model::system::AppDataPathKind),
    SetEditorFontSize(u32),
    SetTerminalFontSize(u32),
    ToggleEditorMinimap,
    UpdateSettings(crate::settings_controls::Change),
    Sections {
        request: sections::Request,
        ports: Arc<crate::settings_ports::Ports>,
    },
}

pub enum Reply {
    AppFileWritten {
        request: crate::app_file_write::WriteRequest,
        result: AppResult<crate::app_file_write::PreparedWrite>,
    },
    AppFile {
        request: crate::app_file::ReadRequest,
        result: AppResult<crate::app_file::PreparedRead>,
    },
    ThemeEdit(crate::theme_edit::Reply),
    SnippetEdit(taide_native_ui::snippet_edit::Reply),
    SnippetCatalog(taide_native_ui::snippet_catalog::Reply),
    SettingsCatalog {
        request: crate::settings_view::Request,
        result: AppResult<crate::settings_view::Catalog>,
    },
    SettingsResource(taide_native_ui::settings_resources::Reply),
    Presentation {
        request: Box<crate::presentation_refresh::Request>,
        result: AppResult<crate::presentation_refresh::Resolved>,
    },
    Sections(sections::Reply),
}

pub(crate) struct State {
    pub(crate) settings_views: crate::settings_view::Views,
    pub(crate) app_file_views: crate::app_file_views::Views,
    pub(crate) settings_appearance: crate::settings_view::Appearance,
    pub(crate) resolved_theme: taide_model::theme::ResolvedTheme,
    pub(crate) preview_theme: Option<taide_model::theme::ResolvedTheme>,
    pub(crate) presentation_changes: Arc<crate::presentation_refresh::Changes>,
    pub(crate) presentation_refresh: crate::presentation_refresh::Refresh,
    pub(crate) presentation_failures: crate::status_error_banner::Failures,
    pub(crate) presentation_system_language: String,
    pub(crate) sections_ports: Arc<crate::settings_ports::Ports>,
    pub(crate) sections_events: Arc<Events>,
    pub(crate) reported_failures: ReportedFailures,
}

#[derive(Default)]
pub(crate) struct ReportedFailures {
    preview: Option<String>,
    app_file_reconcile: Option<String>,
}

fn unreported_failure(last: &mut Option<String>, failure: Option<String>) -> Option<String> {
    let is_new = failure.is_some() && *last != failure;
    *last = failure;
    if is_new { last.clone() } else { None }
}

#[derive(Default)]
pub(crate) struct Events {
    lsp_progress: std::sync::Mutex<std::collections::BTreeMap<LspServerId, sections::Progress>>,
    statuses: std::sync::Mutex<Statuses>,
}

#[derive(Default)]
struct Statuses {
    sync: Option<SyncStatus>,
    remote: Option<RemoteStatus>,
}

impl Events {
    pub(crate) fn record(&self, event: &AppEvent) {
        match event {
            AppEvent::LspInstallProgress {
                server_id,
                phase,
                received_bytes,
                total_bytes,
                message,
            } => {
                if let Ok(mut progress) = self.lsp_progress.lock() {
                    progress.insert(
                        server_id.clone(),
                        sections::Progress {
                            phase: *phase,
                            received_bytes: *received_bytes,
                            total_bytes: *total_bytes,
                            message: message.clone(),
                        },
                    );
                }
            }
            AppEvent::SyncStateChanged { status } => {
                if let Ok(mut statuses) = self.statuses.lock() {
                    statuses.sync = Some(status.clone());
                }
            }
            AppEvent::RemoteStateChanged { status } => {
                if let Ok(mut statuses) = self.statuses.lock() {
                    statuses.remote = Some(*status);
                }
            }
            _ => {}
        }
    }

    fn take(&self) -> std::collections::BTreeMap<LspServerId, sections::Progress> {
        self.lsp_progress
            .lock()
            .map(|mut progress| std::mem::take(&mut *progress))
            .unwrap_or_default()
    }

    fn take_statuses(&self) -> Statuses {
        self.statuses
            .lock()
            .map(|mut statuses| std::mem::take(&mut *statuses))
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub(crate) struct FrameOutput {
    pub(crate) load_settings: Vec<crate::settings_view::Request>,
    pub(crate) load_settings_resources: Vec<taide_native_ui::settings_resources::Request>,
    pub(crate) load_app_files: Vec<crate::app_file::ReadRequest>,
    pub(crate) settings_errors: Vec<AppError>,
    pub(crate) theme_errors: Vec<AppError>,
    pub(crate) snippet_notices: Vec<taide_native_ui::snippet_editor::Notice>,
}

pub(crate) fn is_settings_command(command: &HostCommand) -> bool {
    matches!(
        command,
        HostCommand::Commands(crate::domains::commands::Command::SetKeymapOverrides(_))
            | HostCommand::Settings(Command::SetEditorFontSize(_))
            | HostCommand::Settings(Command::ToggleEditorMinimap)
            | HostCommand::Settings(Command::UpdateSettings(_))
            | HostCommand::Settings(Command::OpenSettingsFolder(_))
    )
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let integrations = worker.integrations;
    let reconcile = &integrations.reconcile;
    match command {
        Command::WriteAppFile(request) => {
            let result = if request.owner().target == taide_model::app::AppFileTarget::Settings
                && reconcile.is_none()
            {
                Err(AppError::Forbidden(
                    "native settings save requires application integrations".into(),
                ))
            } else {
                let apply_services = services.clone();
                let reconcile = reconcile.clone();
                request
                    .execute(services, move |next| async move {
                        let reconcile = reconcile.ok_or_else(|| {
                            AppError::Forbidden(
                                "native settings save requires application integrations".into(),
                            )
                        })?;
                        taide_runtime::settings_actions::apply_and_broadcast(
                            &apply_services.state,
                            next,
                            |current, updated| {
                                (reconcile)(apply_services.clone(), current, updated)
                            },
                            apply_services.events.as_ref(),
                        )
                        .await
                    })
                    .await
            };
            Some(HostReply::Settings(Reply::AppFileWritten {
                request,
                result,
            }))
        }
        Command::ReadAppFile(request) => {
            let result = request.execute(services).await;
            Some(HostReply::Settings(Reply::AppFile { request, result }))
        }
        Command::OpenAppFile {
            project,
            pane,
            target,
            title,
        } => layout_actions::layout_open_tab(
            services.events.as_ref(),
            &services.state,
            project,
            TabKind::AppFile { target },
            title,
            Some(pane),
            false,
        )
        .await
        .err()
        .map(HostReply::Failed),
        Command::ThemeEdit(command) => Some(HostReply::Settings(Reply::ThemeEdit(
            command.execute(services).await,
        ))),
        Command::SnippetEdit(request) => Some(HostReply::Settings(Reply::SnippetEdit(
            request.execute(services).await,
        ))),
        Command::ReadSnippetCatalog(request) => Some(HostReply::Settings(Reply::SnippetCatalog(
            request.execute(services).await,
        ))),
        Command::OpenSettings {
            project,
            pane,
            title,
        } => layout_actions::layout_open_tab(
            services.events.as_ref(),
            &services.state,
            project,
            TabKind::Settings,
            title,
            Some(pane),
            false,
        )
        .await
        .err()
        .map(HostReply::Failed),
        Command::ReadSettingsCatalog(request) => {
            let state = services.state.clone();
            let work = request.clone();
            let result = services
                .tasks
                .run_blocking_result("native-settings-catalog", move || work.load(&state))
                .await;
            Some(HostReply::Settings(Reply::SettingsCatalog {
                request,
                result,
            }))
        }
        Command::ReadSettingsResource { request, ports } => {
            use taide_native_ui::settings_resources::{Kind, Reply};
            if !request.can_read(&services.state) {
                return Some(HostReply::Settings(
                    crate::domains::settings::Reply::SettingsResource(request.failed(
                        AppError::Forbidden("native application is shutting down".into()),
                    )),
                ));
            }
            let reply = match request.kind() {
                Kind::Fonts => {
                    let state = services.state.clone();
                    let work = request.clone();
                    let result = services
                        .tasks
                        .run_blocking_result("native-settings-fonts", move || {
                            if !work.can_read(&state) {
                                return Err(AppError::Forbidden(
                                    "native application is shutting down".into(),
                                ));
                            }
                            Ok((ports.font_families)())
                        })
                        .await;
                    Reply::Fonts { request, result }
                }
                Kind::Shells => Reply::Shells {
                    request,
                    result: taide_runtime::terminal_actions::shell_profiles().await,
                },
            };
            Some(HostReply::Settings(
                crate::domains::settings::Reply::SettingsResource(reply),
            ))
        }
        Command::OpenSettingsFolder(kind) => {
            let services = services.clone();
            let result = services
                .tasks
                .clone()
                .run_blocking_result("native-settings-folder", move || {
                    if services.state.is_shutting_down() {
                        return Err(crate::host::stopped());
                    }
                    system_actions::system_open_app_data_path(
                        &services.state,
                        services.platform.0.as_ref(),
                        kind,
                    )
                })
                .await;
            result.err().map(HostReply::SettingsFailed)
        }
        Command::RefreshPresentation(request) => {
            let state = services.state.clone();
            let inputs = request.inputs.clone();
            let result = services
                .tasks
                .run_blocking_result("native-presentation-refresh", move || {
                    Ok(inputs.load(&state))
                })
                .await;
            Some(HostReply::Settings(Reply::Presentation { request, result }))
        }
        Command::SetEditorFontSize(size) => {
            crate::host::update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    editor_font_size: Some(size),
                    ..Default::default()
                },
            )
            .await
        }
        Command::SetTerminalFontSize(size) => {
            crate::host::update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    terminal_font_size: Some(size),
                    ..Default::default()
                },
            )
            .await
        }
        Command::ToggleEditorMinimap => {
            let enabled = !services.state.settings.read().editor_minimap;
            crate::host::update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    editor_minimap: Some(enabled),
                    ..Default::default()
                },
            )
            .await
        }
        Command::UpdateSettings(change) => {
            let validation = match &change {
                crate::settings_controls::Change::Theme(id)
                | crate::settings_controls::Change::Language(id) => {
                    taide_infra::root_guard::ensure_safe_component(id)
                }
                _ => Ok(()),
            };
            if let Err(error) = validation {
                return Some(HostReply::SettingsFailed(error));
            }
            if let crate::settings_controls::Change::Theme(theme) = change {
                return match taide_runtime::settings_actions::settings_set_theme(
                    &services.state,
                    theme,
                    |current, updated| match reconcile {
                        Some(reconcile) => (reconcile)(services.clone(), current, updated),
                        None => Box::pin(async {}),
                    },
                    services.events.as_ref(),
                )
                .await
                {
                    Ok(_) => None,
                    Err(error) => Some(HostReply::SettingsFailed(error)),
                };
            }
            let patch = change
                .patch()
                .expect("non-theme control has a settings patch");
            crate::host::update_control_settings(services, reconcile, patch).await
        }
        Command::Sections { request, ports } => {
            let request = match request {
                sections::Request::Act(sections::Action::UpdateSettings(change)) => {
                    return Some(HostReply::Settings(Reply::Sections(
                        sections::Reply::SettingsUpdated(
                            update_settings(services, reconcile, change).await,
                        ),
                    )));
                }
                request => request,
            };
            let failed = request.failed(crate::host::stopped());
            let pending_services = services.clone();
            let clipboard = worker.clipboard.clone();
            let reconcile = reconcile.clone();
            let sender = worker.sender.clone();
            let repaint = worker.repaint.clone();
            if services
                .tasks
                .spawn_transient("native-settings-sections", async move {
                    let reply =
                        execute_sections(&pending_services, &clipboard, &ports, reconcile, request)
                            .await;
                    if sender
                        .send(HostReply::Settings(Reply::Sections(reply)))
                        .await
                        .is_ok()
                    {
                        repaint();
                    }
                })
            {
                None
            } else {
                Some(HostReply::Settings(Reply::Sections(failed)))
            }
        }
    }
}

async fn execute_sections(
    services: &Arc<AppServices>,
    clipboard: &ClipboardWriter,
    ports: &crate::settings_ports::Ports,
    reconcile: Option<crate::remote_preferences::Reconcile>,
    request: sections::Request,
) -> sections::Reply {
    match request {
        sections::Request::Fetch {
            query: sections::Query::LspServers,
            generation,
        } => {
            let state = services.state.clone();
            let detect = ports.detect_lsp_servers.clone();
            sections::Reply::LspServers {
                generation,
                result: services
                    .tasks
                    .run_blocking_result("native-settings-lsp-detect", move || {
                        Ok(detect(&state.paths.lsp_dir()))
                    })
                    .await,
            }
        }
        sections::Request::Fetch {
            query: sections::Query::AiTokens,
            generation,
        } => sections::Reply::AiTokens {
            generation,
            result: ai_actions::ai_token_status(&services.state, &services.secrets).await,
        },
        sections::Request::Fetch {
            query: sections::Query::AiModels(provider),
            generation,
        } => sections::Reply::AiModels {
            provider,
            generation,
            result: ai_actions::ai_list_models(&services.state, &services.secrets, provider).await,
        },
        sections::Request::Act(sections::Action::LspInstall(server)) => {
            sections::Reply::LspInstalled {
                result: (ports.lsp_install)(services.clone(), server.clone()).await,
                server,
            }
        }
        sections::Request::Act(sections::Action::LspCancel(server)) => {
            sections::Reply::LspCancelled(lsp_actions::lsp_install_cancel(
                &services.lsp_install,
                server,
            ))
        }
        sections::Request::Act(sections::Action::CopyInstallHint(text)) => {
            sections::Reply::HintCopied(
                crate::host::write_clipboard(services, clipboard, "native-settings-lsp-copy", text)
                    .await,
            )
        }
        sections::Request::Act(sections::Action::AiSetToken {
            provider,
            token,
            sequence,
        }) => sections::Reply::AiTokenSaved {
            provider,
            sequence,
            result: ai_actions::ai_set_token(&services.secrets, provider, token.expose()).await,
        },
        sections::Request::Act(sections::Action::AiClearToken(provider)) => {
            sections::Reply::AiTokenCleared {
                provider,
                result: ai_actions::ai_clear_token(&services.secrets, provider).await,
            }
        }
        sections::Request::Fetch {
            query: sections::Query::Plugins,
            generation,
        } => sections::Reply::Plugins {
            generation,
            result: plugin_actions::plugin_list(&services.state, &services.plugin).await,
        },
        sections::Request::Act(sections::Action::PluginReload) => sections::Reply::PluginsReloaded(
            plugin_actions::plugin_reload(&services.state, &services.plugin).await,
        ),
        sections::Request::Act(sections::Action::PluginUninstall { id }) => {
            sections::Reply::PluginUninstalled(
                plugin_actions::plugin_uninstall(&services.state, &services.plugin, id).await,
            )
        }
        sections::Request::Act(sections::Action::PluginInstall(path)) => {
            sections::Reply::PluginInstalled(
                plugin_actions::plugin_install(
                    &services.state,
                    &services.plugin,
                    &services.tasks,
                    path.to_string_lossy().into_owned(),
                )
                .await
                .map(|_| ()),
            )
        }
        sections::Request::Act(sections::Action::VsixExtract { path, ticket }) => {
            sections::Reply::VsixExtracted {
                ticket,
                result: extract_vsix(services, path).await,
            }
        }
        sections::Request::Act(sections::Action::VsixSaveThemes {
            themes,
            extension,
            ticket,
        }) => {
            let count = themes.len();
            let state = services.state.clone();
            sections::Reply::VsixThemesSaved {
                ticket,
                count,
                extension,
                result: services
                    .tasks
                    .run_blocking_result("native-settings-vsix-themes", move || {
                        for theme in themes {
                            theme_actions::theme_save(&state, theme)?;
                        }
                        Ok(())
                    })
                    .await,
            }
        }
        sections::Request::Act(sections::Action::VsixImportGrammars { path, ticket }) => {
            let state = services.state.clone();
            let store = services.plugin.clone();
            sections::Reply::VsixGrammarsImported {
                ticket,
                result: vsix_actions::vsix_import_plugin(
                    &services.state,
                    &services.tasks,
                    move |directory, id| {
                        plugin_actions::commit_staged_vsix_plugin(&state, &store, directory, id)
                    },
                    path.to_string_lossy().into_owned(),
                )
                .await
                .map(|_| ()),
            }
        }
        sections::Request::Fetch {
            query: sections::Query::SyncStatus,
            generation,
        } => sections::Reply::SyncStatus {
            generation,
            result: sync_actions::sync_status(&services.state, services.secrets.0.as_ref(), || {
                ports.gist.clone()
            })
            .await,
        },
        sections::Request::Fetch {
            query: sections::Query::RemoteStatus,
            generation,
        } => sections::Reply::RemoteStatus {
            generation,
            result: remote_actions::remote_status(&services.remote).await,
        },
        sections::Request::Act(sections::Action::SyncConnect(token)) => {
            let worker = services.clone();
            let gist = ports.gist.clone();
            sections::Reply::SyncConnected(
                services
                    .tasks
                    .run_nonabortable_result("native-settings-sync-connect", async move {
                        sync_actions::sync_connect(
                            &worker.state,
                            worker.secrets.0.as_ref(),
                            token.expose(),
                            || gist,
                            worker.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        sections::Request::Act(sections::Action::SyncDisconnect) => {
            let worker = services.clone();
            sections::Reply::SyncDisconnected(
                services
                    .tasks
                    .run_nonabortable_result("native-settings-sync-disconnect", async move {
                        sync_actions::sync_disconnect(
                            &worker.state,
                            worker.secrets.0.as_ref(),
                            worker.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        sections::Request::Act(sections::Action::SyncUpload) => {
            let worker = services.clone();
            let gist = ports.gist.clone();
            sections::Reply::SyncUploaded(
                services
                    .tasks
                    .run_nonabortable_result("native-settings-sync-upload", async move {
                        sync_actions::sync_upload(
                            &worker.state,
                            worker.secrets.0.as_ref(),
                            || gist,
                            worker.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        sections::Request::Act(sections::Action::SyncDownload { force, owner }) => {
            sections::Reply::SyncDownloaded {
                force,
                owner,
                result: download_sync(services, ports, reconcile, force).await,
            }
        }
        sections::Request::Act(sections::Action::RemoteIssueLink { owner }) => {
            let result =
                match remote_actions::remote_issue_link(&services.state, &services.remote).await {
                    Ok(link) => {
                        let copied = crate::host::write_clipboard(
                            services,
                            clipboard,
                            "native-settings-remote-link-copy",
                            link.url.clone(),
                        )
                        .await;
                        Ok((link, copied))
                    }
                    Err(error) => Err(error),
                };
            sections::Reply::RemoteLinkIssued { owner, result }
        }
        sections::Request::Act(sections::Action::RemoteRevoke) => sections::Reply::RemoteRevoked(
            remote_actions::remote_revoke_sessions(&services.remote).await,
        ),
        sections::Request::Act(sections::Action::RemoteSetPassword(password)) => {
            sections::Reply::RemotePasswordSet(
                remote_actions::remote_set_password(
                    &services.remote,
                    &services.secrets,
                    password.expose(),
                )
                .await,
            )
        }
        sections::Request::Act(sections::Action::RemoteClearPassword) => {
            sections::Reply::RemotePasswordCleared(
                remote_actions::remote_clear_password(&services.remote, &services.secrets).await,
            )
        }
        sections::Request::Act(sections::Action::UpdateSettings(change)) => {
            sections::Reply::SettingsUpdated(update_settings(services, &reconcile, change).await)
        }
        sections::Request::Fetch {
            query: sections::Query::CliStatus,
            generation,
        } => {
            let status = ports.agents.cli_status.clone();
            sections::Reply::CliStatus {
                generation,
                result: services
                    .tasks
                    .run_blocking_result("native-settings-cli-status", move || Ok(status()))
                    .await,
            }
        }
        sections::Request::Act(sections::Action::CliInstall) => sections::Reply::CliInstalled(
            change_shell_command(services, ports.cli.install.clone()).await,
        ),
        sections::Request::Act(sections::Action::CliUninstall) => sections::Reply::CliUninstalled(
            change_shell_command(services, ports.cli.uninstall.clone()).await,
        ),
        sections::Request::Fetch {
            query: sections::Query::HooksStatus(target),
            generation,
        } => sections::Reply::HooksStatus {
            result: agent_hook_actions::agent_hooks_status(
                &services.state,
                &services.tasks,
                target.project.clone(),
                target.agent.to_string(),
                || (ports.agents.resolve_home)(),
            )
            .await,
            target,
            generation,
        },
        sections::Request::Act(sections::Action::Hooks { target, change }) => {
            let agents = &ports.agents;
            let result = match change {
                sections::HookChange::Install => {
                    agent_hook_actions::agent_hooks_install(
                        &services.state,
                        &services.tasks,
                        target.project.clone(),
                        target.agent.to_string(),
                        agent_hook_actions::AgentHookInstallPorts::new(
                            || (agents.resolve_home)(),
                            || (agents.project_emitter)(services.clone()),
                            || (agents.cli_status)().installed,
                            || crate::agent_hooks::ensure_started(services.clone()),
                            &agents.cli_target,
                        ),
                    )
                    .await
                }
                sections::HookChange::Uninstall => {
                    agent_hook_actions::agent_hooks_uninstall(
                        &services.state,
                        &services.tasks,
                        target.project.clone(),
                        target.agent.to_string(),
                        || (agents.resolve_home)(),
                    )
                    .await
                }
            };
            sections::Reply::HooksChanged {
                target,
                change,
                result,
            }
        }
        sections::Request::Act(sections::Action::OpenNotificationSettings) => {
            let open = ports.open_notification_settings.clone();
            sections::Reply::NotificationSettingsOpened(
                services
                    .tasks
                    .run_blocking_result("native-settings-notification-settings", move || open())
                    .await,
            )
        }
        sections::Request::Act(sections::Action::SendTestNotification {
            title,
            body,
            any_window_focused,
        }) => {
            let worker = services.clone();
            sections::Reply::TestNotification(
                services
                    .tasks
                    .run_blocking_result("native-settings-test-notification", move || {
                        notification_actions::notification_notify(
                            &worker.state,
                            worker.platform.0.as_ref(),
                            NotificationCategory::Error,
                            &title,
                            &body,
                            || any_window_focused,
                        )
                    })
                    .await,
            )
        }
        sections::Request::Act(sections::Action::PerfRead { ticket }) => {
            sections::Reply::PerfRead {
                ticket,
                result: Ok(app_actions::perf_snapshot(ports.perf)),
            }
        }
        sections::Request::Act(sections::Action::PerfReset { ticket }) => {
            app_actions::perf_reset(ports.perf);
            sections::Reply::PerfReset {
                ticket,
                result: Ok(()),
            }
        }
    }
}

async fn change_shell_command(
    services: &Arc<AppServices>,
    change: crate::settings_ports::CliCommand,
) -> AppResult<sections::CliOutcome> {
    services
        .tasks
        .run_blocking_result("native-settings-cli-command", move || change())
        .await
        .map(|outcome| sections::CliOutcome {
            status: outcome.status,
            is_cancelled: outcome.is_cancelled,
        })
}

async fn update_settings(
    services: &Arc<AppServices>,
    reconcile: &Option<crate::remote_preferences::Reconcile>,
    change: taide_native_ui::settings_controls::HostedChange,
) -> AppResult<()> {
    settings_actions::settings_update(
        &services.state,
        change.patch(),
        |current, updated| match reconcile {
            Some(reconcile) => (reconcile)(services.clone(), current, updated),
            None => Box::pin(async {}),
        },
        services.events.as_ref(),
    )
    .await
    .map(|_| ())
}

async fn download_sync(
    services: &Arc<AppServices>,
    ports: &crate::settings_ports::Ports,
    reconcile: Option<crate::remote_preferences::Reconcile>,
    force: bool,
) -> AppResult<taide_model::sync::SyncDownloadResult> {
    let prepared =
        sync_actions::prepare_sync_download(&services.state, services.secrets.0.as_ref(), || {
            ports.gist.clone()
        })
        .await?;
    let worker = services.clone();
    services
        .tasks
        .run_nonabortable_result("native-settings-sync-download", async move {
            sync_actions::apply_sync_download(
                &worker.state,
                prepared,
                |next| {
                    settings_actions::apply_and_broadcast(
                        &worker.state,
                        next,
                        |current, updated| match &reconcile {
                            Some(reconcile) => (reconcile)(worker.clone(), current, updated),
                            None => Box::pin(async {}),
                        },
                        worker.events.as_ref(),
                    )
                },
                force,
                worker.events.as_ref(),
            )
            .await
        })
        .await
}

async fn extract_vsix(
    services: &Arc<AppServices>,
    path: std::path::PathBuf,
) -> AppResult<sections::VsixExtraction> {
    let extracted = vsix_actions::vsix_extract_themes(path.to_string_lossy().into_owned()).await?;
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result("native-settings-vsix-candidates", move || {
            let existing_ids = theme_actions::theme_list(&state)
                .map(|themes| themes.into_iter().map(|theme| theme.id).collect::<Vec<_>>())
                .unwrap_or_default();
            let candidates = vsix_actions::vsix_theme_candidates(&extracted, &existing_ids);
            Ok(sections::VsixExtraction {
                path,
                extension: extracted.extension.display_name,
                existing_ids,
                candidates,
            })
        })
        .await
}

impl NativeApplication {
    pub(crate) fn poll_settings_revisions(&mut self) {
        self.settings_state
            .settings_views
            .observe_theme_revision(self.settings_state.presentation_changes.theme_revision());
        self.settings_state
            .app_file_views
            .observe_settings_revision(
                self.settings_state.presentation_changes.settings_revision(),
            );
        let failure = self
            .settings_state
            .app_file_views
            .reconcile(&self.services.state, &mut self.store)
            .err()
            .map(|error| format!("native app file reconcile failed: {}", editor_error(error)));
        if let Some(message) = unreported_failure(
            &mut self.settings_state.reported_failures.app_file_reconcile,
            failure,
        ) {
            log::warn!("{message}");
        }
    }

    pub(crate) fn apply_settings_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::AppFileWritten { request, result } => {
                if self.closing.is_none()
                    && let Some(result) = self.settings_state.app_file_views.accept_write(
                        &request,
                        result,
                        &mut self.store,
                    )
                {
                    match result {
                        Ok(document) => {
                            if let Ok(snapshot) = self.store.documents().snapshot(document) {
                                for tab in crate::app_file_views::target_tabs(
                                    &self.services.state,
                                    request.owner().target,
                                ) {
                                    self.pending_dirty.insert(tab, snapshot.dirty);
                                }
                            }
                        }
                        Err(error) => self.toasts.app_file_failed(
                            &self.locale,
                            request.owner().target,
                            &error,
                            Instant::now(),
                        ),
                    }
                }
            }
            Reply::AppFile { request, result } => {
                if self.closing.is_none() {
                    self.settings_state
                        .app_file_views
                        .accept(&request, result, &mut self.store);
                }
            }
            Reply::ThemeEdit(reply) => {
                if self.closing.is_none()
                    && reply.owner().is_active(&self.services.state)
                    && let Some(error) = self.settings_state.settings_views.accept_theme(reply)
                {
                    self.toasts.ipc_error(&self.locale, &error, Instant::now());
                }
            }
            Reply::SnippetEdit(reply) => {
                if self.closing.is_none() {
                    self.settings_state.settings_views.accept_snippet(reply);
                }
            }
            Reply::SnippetCatalog(reply) => {
                if self.closing.is_none() {
                    self.settings_state
                        .settings_views
                        .accept_snippet_catalog(reply, Instant::now());
                }
            }
            Reply::SettingsCatalog { request, result } => {
                if self.closing.is_none() && request.is_active(&self.services.state) {
                    self.settings_state.settings_views.accept(&request, result);
                }
            }
            Reply::SettingsResource(reply) => {
                if self.closing.is_none() && reply.request().can_read(&self.services.state) {
                    self.settings_state.settings_views.accept_resource(reply);
                }
            }
            Reply::Presentation { request, result } => {
                let inputs = self.presentation_inputs(context);
                if self.settings_state.presentation_refresh.finish(
                    &request,
                    &inputs,
                    self.settings_state.presentation_changes.revision(),
                ) && self.closing.is_none()
                    && !self.is_exit_ready
                    && !self.services.state.is_shutting_down()
                {
                    match result {
                        Ok(resolved) => {
                            match resolved.theme.and_then(|theme| {
                                let appearances = crate::presentation_refresh::Appearances::new(
                                    &theme,
                                    &inputs.settings,
                                )?;
                                self.terminal_agent_state
                                    .terminal_views
                                    .set_palette(&appearances.terminal)?;
                                Ok((appearances, theme))
                            }) {
                                Ok((appearances, theme)) => {
                                    self.settings_state.resolved_theme = theme;
                                    self.settings_state.preview_theme = None;
                                    self.apply_presentation_appearances(context, appearances);
                                    self.settings_state.presentation_failures.theme = false;
                                }
                                Err(error) => {
                                    log::warn!("native theme load failed: {error}");
                                    self.settings_state.presentation_failures.theme = true;
                                }
                            }
                            match resolved.locale {
                                Ok(locale) => {
                                    self.locale = locale;
                                    self.settings_state.presentation_failures.locale = false;
                                }
                                Err(error) => {
                                    log::warn!("native locale load failed: {error}");
                                    self.settings_state.presentation_failures.locale = true;
                                }
                            }
                        }
                        Err(error) => {
                            log::warn!("native presentation load failed: {error}");
                            self.settings_state.presentation_failures =
                                crate::status_error_banner::Failures {
                                    theme: true,
                                    locale: true,
                                };
                        }
                    }
                    context.request_repaint();
                }
            }
            Reply::Sections(reply) => {
                if self.closing.is_none() {
                    self.accept_sections_reply(reply);
                    context.request_repaint();
                }
            }
        }
    }

    fn accept_sections_reply(&mut self, reply: sections::Reply) {
        let is_theme_save = matches!(reply, sections::Reply::VsixThemesSaved { .. });
        let notice = self
            .settings_state
            .settings_views
            .sections_data_mut()
            .accept(reply, Instant::now());
        if is_theme_save {
            self.settings_state.settings_views.invalidate_catalog();
        }
        let Some(notice) = notice else {
            return;
        };
        let now = Instant::now();
        match notice {
            sections::Notice::Success(key) => self
                .toasts
                .success(presentation::message(&self.locale, key, &[]), now),
            sections::Notice::SuccessWith { key, arguments } => {
                let arguments = arguments
                    .iter()
                    .map(|(name, value)| (*name, value.as_str()))
                    .collect::<Vec<_>>();
                self.toasts
                    .success(presentation::message(&self.locale, key, &arguments), now);
            }
            sections::Notice::Warning(key) => self
                .toasts
                .warning(presentation::message(&self.locale, key, &[]), now),
            sections::Notice::Failure(key) => self
                .toasts
                .error(presentation::message(&self.locale, key, &[]), now),
            sections::Notice::Error { error, fallback } => {
                let described = taide_native_ui::toast::describe_error(&self.locale, &error);
                let title = match fallback {
                    Some(key) if described.is_empty() => {
                        presentation::message(&self.locale, key, &[])
                    }
                    _ => described,
                };
                self.toasts.error(title, now);
            }
            sections::Notice::SettingsFailed(error) => {
                self.toasts.settings_failed(&self.locale, &error, now);
            }
        }
    }

    pub(crate) fn apply_settings_intent(
        &mut self,
        intent: Intent,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        match intent {
            Intent::OpenSettingsFile => {
                let project = match &self.shell.scope {
                    WindowScope::Main => snapshot.focused_project(),
                    WindowScope::Auxiliary { project, .. } => Some(project),
                };
                let command = project.and_then(|project| {
                    crate::app_file_views::settings_command(
                        project,
                        snapshot.layouts.get(project)?,
                        &self.shell.scope,
                    )
                });
                if let Some(command) = command {
                    self.submit(command);
                } else {
                    self.notify_open_project_first();
                }
            }
            Intent::OpenSettings => {
                let owner = match &self.shell.scope {
                    WindowScope::Main => snapshot.focused_project().and_then(|project| {
                        snapshot
                            .layouts
                            .get(project)
                            .map(|layout| (project.clone(), layout.focused_pane.clone()))
                    }),
                    WindowScope::Auxiliary { project, slot } => {
                        snapshot.layouts.get(project).and_then(|layout| {
                            layout
                                .auxiliary_windows
                                .iter()
                                .find(|window| window.slot == *slot)
                                .map(|window| (project.clone(), window.focused_pane.clone()))
                        })
                    }
                };
                if let Some((project, pane)) = owner {
                    self.submit(HostCommand::Settings(Command::OpenSettings {
                        project,
                        pane,
                        title: presentation::message(&self.locale, "settings.title", &[]),
                    }));
                } else {
                    self.notify_open_project_first();
                }
            }
            Intent::ToggleEditorMinimap => {
                self.submit(HostCommand::Settings(Command::ToggleEditorMinimap));
            }
            Intent::ChangeEditorFontSize { increase } => {
                let current = self.services.state.settings.read().editor_font_size;
                self.submit(HostCommand::Settings(Command::SetEditorFontSize(
                    presentation::next_editor_font_size(current, increase),
                )));
            }
        }
    }

    fn presentation_inputs(&self, context: &egui::Context) -> crate::presentation_refresh::Inputs {
        let system_theme = match context.system_theme() {
            Some(egui::Theme::Light) => "light",
            Some(egui::Theme::Dark) | None => "dark",
        };
        crate::presentation_refresh::Inputs::new(
            self.services.state.settings.read().clone(),
            system_theme,
            &self.settings_state.presentation_system_language,
        )
    }

    pub(crate) fn sync_theme_preview(&mut self, context: &egui::Context) {
        let preview = if self.closing.is_none() && !self.services.state.is_shutting_down() {
            self.settings_state
                .settings_views
                .preview(context.viewport_id(), &self.services.state)
                .cloned()
        } else {
            None
        };
        if preview == self.settings_state.preview_theme {
            return;
        }
        let theme = preview
            .as_ref()
            .unwrap_or(&self.settings_state.resolved_theme);
        let settings = self.services.state.settings.read().clone();
        let result = crate::presentation_refresh::Appearances::new(theme, &settings).and_then(
            |appearances| {
                self.terminal_agent_state
                    .terminal_views
                    .set_palette(&appearances.terminal)?;
                Ok(appearances)
            },
        );
        let failure = match result {
            Ok(appearances) => {
                self.apply_presentation_appearances(context, appearances);
                self.settings_state.preview_theme = preview;
                context.request_repaint();
                None
            }
            Err(error) => Some(format!("native theme preview failed: {error}")),
        };
        if let Some(message) =
            unreported_failure(&mut self.settings_state.reported_failures.preview, failure)
        {
            log::warn!("{message}");
        }
    }

    fn apply_presentation_appearances(
        &mut self,
        context: &egui::Context,
        appearances: crate::presentation_refresh::Appearances,
    ) {
        presentation::apply_visuals(context, &appearances.visuals);
        self.shell.colors = appearances.shell;
        self.editor.appearance = appearances.editor;
        self.find_appearance = appearances.find;
        self.editor_display_colors = appearances.editor_display;
        self.editor_bracket_colors = appearances.editor_brackets;
        self.editor_sticky_colors = appearances.editor_sticky;
        self.editor_minimap_colors = appearances.editor_minimap;
        self.editor_diagnostic_colors = appearances.editor_diagnostics;
        self.editor_overview_colors = appearances.editor_overview;
        self.editor_problem_colors = appearances.editor_problems;
        self.editor_location_colors = appearances.editor_locations;
        self.editor_documentation_colors = appearances.editor_documentation;
        self.editor_markdown_colors = appearances.editor_markdown;
        self.editor_completion_colors = appearances.editor_completion;
        self.editor_ai_edit_colors = appearances.editor_ai_edit;
        self.editor_highlight_colors = appearances.editor_highlights;
        self.editor_code_action_colors = appearances.editor_code_actions;
        self.editor_inlay_hint_colors = appearances.editor_inlay_hints;
        self.editor_code_lens_colors = appearances.editor_code_lenses;
        self.banner_appearance = appearances.banner;
        self.lsp_status_appearance = appearances.lsp_status;
        self.status_editor_appearance = appearances.status_editor;
        self.terminal_agent_state.status_ide_appearance = appearances.status_ide;
        self.shell_state.system_usage_appearance = appearances.system_usage;
        self.status_chord_appearance = appearances.status_chord;
        self.problems_appearance = appearances.problems;
        self.shell_state.zen_hint_colors = appearances.zen_hint;
        self.shell_state.file_drop_colors = appearances.file_drop;
        self.explorer_search_state.explorer_appearance = appearances.explorer;
        self.explorer_search_state.navigation_appearance = appearances.navigation;
        self.tooltip_appearance = appearances.tooltip;
        self.terminal_agent_state.terminal_appearance = appearances.terminal;
        self.commands_state
            .keybindings
            .set_appearance(appearances.keybindings);
        self.commands_state
            .palette
            .set_appearance(appearances.palette);
        self.settings_state.settings_appearance = appearances.settings;
        self.preview_state.pdf_appearance = appearances.pdf;
        self.preview_state.presentation_appearance = appearances.presentation;
        self.preview_state.spreadsheet_appearance = appearances.spreadsheet;
        self.toast_theme = appearances.toast;
    }

    pub(crate) fn intercept_settings_command(
        &mut self,
        command: HostCommand,
    ) -> ControlFlow<bool, HostCommand> {
        if let HostCommand::Settings(Command::SnippetEdit(request)) = &command
            && matches!(request.kind(), taide_native_ui::snippet_edit::Kind::List)
        {
            if !request.is_active() || !request.owner().is_active(&self.services.state) {
                self.settings_state
                    .settings_views
                    .accept_snippet(request.clone().failed(AppError::Forbidden(
                        "native snippet observer owner is no longer active".into(),
                    )));
                return ControlFlow::Break(false);
            }
            self.settings_state
                .settings_views
                .observe_snippet_list(request.clone(), Instant::now());
            return ControlFlow::Break(true);
        }
        ControlFlow::Continue(command)
    }

    pub(crate) fn save_settings_app_file(
        &mut self,
        tab: &taide_model::ids::TabId,
        document: Option<taide_native_editor::document::DocumentId>,
    ) -> bool {
        if let Some(owner) = self.settings_state.app_file_views.save_owner(tab, document) {
            match self
                .settings_state
                .app_file_views
                .begin_write(&owner, &mut self.store)
            {
                Ok(Some(request)) => {
                    if !self.submit(HostCommand::Settings(Command::WriteAppFile(
                        request.clone(),
                    ))) {
                        self.settings_state.app_file_views.cancel_write(&request);
                    }
                }
                Ok(None) => (),
                Err(error) => {
                    self.toasts
                        .app_file_failed(&self.locale, owner.target, &error, Instant::now())
                }
            }
            return true;
        }
        false
    }

    pub(crate) fn release_settings_app_file(&mut self, tab: &Tab) -> AppResult<()> {
        self.settings_state
            .app_file_views
            .release_closed(tab, &mut self.store)
            .map_err(editor_error)
    }

    pub(crate) fn pick_settings_files(&mut self, frame: &eframe::Frame) {
        let Some(purpose) = self
            .settings_state
            .settings_views
            .sections_data_mut()
            .take_pick()
        else {
            return;
        };
        let (filter_name, extensions, title) = match purpose {
            sections::PickPurpose::PluginArchive => (
                PLUGIN_ARCHIVE_FILTER,
                PLUGIN_ARCHIVE_EXTENSIONS,
                "settings.pluginInstallDialogTitle",
            ),
            sections::PickPurpose::Vsix(_) => (
                VSIX_FILTER,
                VSIX_EXTENSIONS,
                "settings.pluginImportVsixDialogTitle",
            ),
        };
        let path = (self.settings_state.sections_ports.pick_file)(
            frame,
            crate::file_dialogs::OpenRequest {
                filter_name,
                extensions,
                title: presentation::message(&self.locale, title, &[]),
            },
        );
        self.settings_state
            .settings_views
            .sections_data_mut()
            .picked(purpose, path);
    }

    pub(crate) fn tick_settings_reads(&mut self, context: &egui::Context) {
        let progress = self.settings_state.sections_events.take();
        if !progress.is_empty() {
            let data = self.settings_state.settings_views.sections_data_mut();
            for (server, progress) in progress {
                data.record_progress(server, progress);
            }
            context.request_repaint();
        }
        let statuses = self.settings_state.sections_events.take_statuses();
        let is_focused = self.is_any_window_focused(context);
        let is_perf_enabled = self.settings_state.sections_ports.perf.is_enabled();
        let data = self.settings_state.settings_views.sections_data_mut();
        data.observe_window_focus(is_focused);
        data.observe_perf_gate(is_perf_enabled);
        let now = Instant::now();
        if let Some(status) = statuses.sync {
            data.record_sync_status(status, now);
            context.request_repaint();
        }
        if let Some(status) = statuses.remote {
            data.record_remote_status(status, now);
            context.request_repaint();
        }
        if let Some(request) = self.settings_state.settings_views.next_snippet_read() {
            self.submit(HostCommand::Settings(Command::ReadSnippetCatalog(request)));
        }
        let inputs = self.presentation_inputs(context);
        if let Some(request) = self
            .settings_state
            .presentation_refresh
            .next(inputs, self.settings_state.presentation_changes.revision())
            && self.submit(HostCommand::Settings(Command::RefreshPresentation(
                Box::new(request.clone()),
            )))
        {
            self.settings_state.presentation_refresh.submitted(request);
        }
    }

    pub(crate) fn load_settings_requests(
        &mut self,
        load_settings: Vec<crate::settings_view::Request>,
        load_settings_resources: Vec<taide_native_ui::settings_resources::Request>,
        load_app_files: Vec<crate::app_file::ReadRequest>,
    ) {
        for request in load_settings {
            if !self.submit(HostCommand::Settings(Command::ReadSettingsCatalog(
                request.clone(),
            ))) {
                self.settings_state.settings_views.accept(
                    &request,
                    Err(AppError::Forbidden(
                        "native Settings host is disconnected or full".into(),
                    )),
                );
            }
        }
        for request in load_settings_resources {
            if !self.submit(HostCommand::Settings(Command::ReadSettingsResource {
                request: request.clone(),
                ports: self.settings_state.sections_ports.clone(),
            })) {
                self.settings_state
                    .settings_views
                    .accept_resource(request.failed(AppError::Forbidden(
                        "native Settings resource host is disconnected or full".into(),
                    )));
            }
        }
        for request in load_app_files {
            if !self.submit(HostCommand::Settings(Command::ReadAppFile(request.clone()))) {
                self.settings_state.app_file_views.accept(
                    &request,
                    Err(AppError::Forbidden(
                        "native app file host is disconnected or full".into(),
                    )),
                    &mut self.store,
                );
            }
        }
        let requests = self
            .settings_state
            .settings_views
            .sections_data_mut()
            .take_requests();
        for request in requests {
            let failed = request.failed(AppError::Forbidden(SECTIONS_DISCONNECTED.into()));
            let ports = self.settings_state.sections_ports.clone();
            if !self.submit(HostCommand::Settings(Command::Sections { request, ports })) {
                self.settings_state
                    .settings_views
                    .sections_data_mut()
                    .accept(failed, Instant::now());
            }
        }
    }
}

impl AppSurfaces<'_> {
    pub(crate) fn show_settings_tab(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        intents: &mut Vec<ShellIntent>,
    ) {
        if self.target == Some(pane) {
            *self.focused = None;
        }
        let settings = self.services.state.settings.read().clone();
        let is_perf_enabled = self.settings_views.sections_data().is_perf_enabled();
        let output = self.settings_views.show_with_host(
            ui,
            crate::settings_view::Owner {
                project: project.clone(),
                pane: pane.clone(),
                tab: tab.id.clone(),
            },
            &settings,
            self.locale,
            self.settings_appearance,
            crate::settings_view::sections_view::Host {
                format_time: crate::settings_sync_time::format_system_time,
                projects: self.open_projects,
                is_macos: cfg!(target_os = "macos"),
                is_dev_build: cfg!(debug_assertions),
                is_perf_enabled,
            },
        );
        self.tooltips
            .show_triggers(&output.tooltips, self.tooltip_appearance);
        if let Some(request) = output.load {
            self.settings_output.load_settings.push(request);
        }
        self.settings_output
            .load_settings_resources
            .extend(output.resources);
        self.commands.extend(
            output
                .changes
                .into_iter()
                .map(Command::UpdateSettings)
                .map(HostCommand::Settings),
        );
        self.commands.extend(
            output
                .themes
                .into_iter()
                .map(Command::ThemeEdit)
                .map(HostCommand::Settings),
        );
        self.commands.extend(
            output
                .snippets
                .into_iter()
                .map(Command::SnippetEdit)
                .map(HostCommand::Settings),
        );
        self.settings_output
            .snippet_notices
            .extend(output.snippet_notices);
        self.commands.extend(
            output
                .folders
                .into_iter()
                .map(Command::OpenSettingsFolder)
                .map(HostCommand::Settings),
        );
        if let Some(error) = output.error {
            self.settings_output.settings_errors.push(error);
        }
        if let Some(error) = output.theme_error {
            self.settings_output.theme_errors.push(error);
        }
        if output.open_keybindings {
            intents.push(ShellIntent::Commands(
                taide_native_ui::intents::commands::Intent::OpenKeybindings,
            ));
        }
        if output.open_settings_file
            && let Some(layout) = self.layouts.get(project)
            && let Some(command) =
                crate::app_file_views::settings_command(project, layout, self.scope)
        {
            self.commands.push(command);
        }
        if let Some((target, title)) = output.open_app_file
            && let Some(layout) = self.layouts.get(project)
            && let Some(command) =
                crate::app_file_views::open_command(project, layout, self.scope, target, title)
        {
            self.commands.push(command);
        }
    }

    pub(crate) fn show_app_file_tab(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        target: taide_model::app::AppFileTarget,
        intents: &mut Vec<ShellIntent>,
    ) {
        let owner = crate::app_file::Owner {
            project: project.clone(),
            pane: pane.clone(),
            tab: tab.id.clone(),
            target,
        };
        if let Some(request) = self
            .app_file_views
            .begin(&owner, self.store, ui.is_enabled())
        {
            self.settings_output.load_app_files.push(request);
        }
        if let Some(document) = self.app_file_views.document(&owner) {
            self.show_document(ui, pane, tab, document, None, intents);
        } else {
            let rect = ui.available_rect_before_wrap();
            ui.painter()
                .rect_filled(rect, 0.0, self.editor.appearance.background);
            if self.app_file_views.failed(&owner) {
                ui.allocate_ui_with_layout(
                    rect.size(),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.colored_label(
                            self.banner_appearance.error,
                            presentation::message(self.locale, "editor.openFailed", &[]),
                        );
                    },
                );
            } else {
                ui.allocate_rect(rect, egui::Sense::hover());
            }
        }
    }
}

#[cfg(test)]
#[path = "../application-settings-sections-tests.rs"]
mod settings_sections_tests;

#[cfg(test)]
#[path = "../application-settings-plugins-tests.rs"]
mod settings_plugins_tests;

#[cfg(test)]
#[path = "../application-settings-sync-remote-tests.rs"]
mod settings_sync_remote_tests;

#[cfg(test)]
#[path = "../application-settings-interface-tests.rs"]
mod settings_interface_tests;

#[cfg(test)]
#[path = "../application-settings-consumers-tests.rs"]
mod settings_consumers_tests;

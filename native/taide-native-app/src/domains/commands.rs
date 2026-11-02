use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId};
use taide_model::layout::TabKind;
use taide_native_editor::view::ViewKey;
use taide_native_ui::intents::commands::Intent;
use taide_native_ui::shell::WindowScope;

use crate::application::{NativeApplication, WINDOW_LABEL, editor_error};
use crate::command_registry::{CommandContext, DocumentEdit};
use crate::host::{HostCommand, HostReply, HostWorker};
use crate::presentation;

pub enum Command {
    RunTask(crate::editor_context_menu::TerminalDestination),
    DetectTasks {
        project: ProjectId,
        generation: u64,
    },
    SetKeymapOverrides(crate::keymap::catalog::Overrides),
    ListProjectFiles(ProjectId),
    OpenPaletteFile {
        project: ProjectId,
        pane: Option<PaneId>,
        path: String,
    },
}

pub enum Reply {
    TasksDetected {
        generation: u64,
        result: AppResult<Vec<taide_model::task::Task>>,
    },
    ProjectFiles {
        project: ProjectId,
        result: AppResult<Vec<String>>,
    },
    PaletteFileOpened {
        project: ProjectId,
        result: AppResult<()>,
    },
}

pub(crate) struct State {
    pub(crate) keybindings: crate::keybinding_editor::Editor,
    pub(crate) palette: crate::command_palette::Palette,
    pub(crate) palette_files: crate::command_palette::FileIndexes,
    pub(crate) palette_file_changes: Arc<crate::command_palette::FileIndexChanges>,
    pub(crate) palette_commands: Vec<String>,
    pub(crate) task_runner: Option<(u64, ProjectId)>,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let integrations = worker.integrations;
    let reconcile = &integrations.reconcile;
    match command {
        Command::RunTask(destination) => Some(HostReply::Editor(
            crate::domains::editor::Reply::EditorTerminalTarget(
                crate::editor_context_menu::destination_terminal_target(services, destination)
                    .await,
            ),
        )),
        Command::DetectTasks {
            project,
            generation,
        } => Some(HostReply::Commands(Reply::TasksDetected {
            generation,
            result: taide_runtime::task_actions::detect_tasks(
                &services.state,
                &services.tasks,
                project,
            )
            .await,
        })),
        Command::SetKeymapOverrides(overrides) => {
            let json = match overrides.json() {
                Ok(json) => json,
                Err(error) => return Some(HostReply::SettingsFailed(error)),
            };
            crate::host::update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    keymap_overrides: Some(json),
                    ..Default::default()
                },
            )
            .await
        }
        Command::ListProjectFiles(project) => {
            let result = taide_runtime::search_actions::search_list_files(
                &services.state,
                &services.tasks,
                project.clone(),
            )
            .await;
            Some(HostReply::Commands(Reply::ProjectFiles { project, result }))
        }
        Command::OpenPaletteFile {
            project,
            pane,
            path,
        } => {
            let result = async {
                integrations.flush_view_states(services).await?;
                crate::host::open_file_tab(services, project.clone(), pane, path, true).await
            }
            .await;
            Some(HostReply::Commands(Reply::PaletteFileOpened {
                project,
                result,
            }))
        }
    }
}

impl NativeApplication {
    pub(crate) fn apply_commands_reply(&mut self, reply: Reply) {
        match reply {
            Reply::TasksDetected { generation, result } => {
                let tasks = result.unwrap_or_else(|error| {
                    log::warn!("native task detection failed: {:?}", error.kind());
                    Vec::new()
                });
                self.commands_state.palette.show_tasks(generation, tasks);
            }
            Reply::ProjectFiles { project, result } => {
                self.commands_state
                    .palette_files
                    .accept(&project, result, Instant::now());
            }
            Reply::PaletteFileOpened { project, result } => {
                if let Err(error) = result {
                    if error.kind() == taide_model::error::AppErrorKind::NotFound {
                        self.commands_state.palette_files.invalidate(&project);
                    }
                    self.toasts.ipc_error(&self.locale, &error, Instant::now());
                }
            }
        }
    }

    pub(crate) fn apply_commands_intent(&mut self, intent: Intent, context: &egui::Context) {
        match intent {
            Intent::OpenKeybindings => self.commands_state.keybindings.open(context),
            Intent::OpenPalette(entry) => {
                self.indentation_request = None;
                self.commands_state.palette.open(context, entry);
            }
            Intent::OpenTaskRunner(project) => {
                let generation = self
                    .commands_state
                    .task_runner
                    .as_ref()
                    .map_or(0, |(generation, _)| *generation)
                    + 1;
                self.commands_state.task_runner = Some((generation, project.clone()));
                self.indentation_request = None;
                self.commands_state.palette.open_tasks(context, generation);
                if !self.submit(HostCommand::Commands(Command::DetectTasks {
                    project,
                    generation,
                })) {
                    self.commands_state
                        .palette
                        .show_tasks(generation, Vec::new());
                }
            }
        }
    }

    pub(crate) fn tick_commands_files(
        &mut self,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        for project in self.commands_state.palette_file_changes.take() {
            self.commands_state.palette_files.invalidate(&project);
        }
        self.commands_state
            .palette_files
            .retain(|project| snapshot.layouts.contains_key(project));
    }

    pub(crate) fn reconnect_commands(&mut self) {
        self.commands_state.palette_files.cancel_fetches();
    }

    pub(crate) fn show_commands_overlays(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        command_context: &CommandContext,
        is_enabled: bool,
    ) {
        let keybindings_enabled = is_enabled
            && !self.shell_state.system_usage.detail_open
            && self.pending_tab_close.is_none()
            && self.explorer_search_state.pending_entry_delete.is_none()
            && !self.explorer_search_state.deleting_entry
            && self.saving_untitled.is_none()
            && self.saving_missing_draft.is_none()
            && self.pending_disk_choice.is_none();
        let raw_overrides = self.services.state.settings.read().keymap_overrides.clone();
        match self.commands_state.keybindings.show(
            context,
            &self.locale,
            raw_overrides.as_deref(),
            keybindings_enabled && !self.commands_state.palette.is_open(),
        ) {
            Ok(output) => {
                output.show_tooltips(&self.locale, &self.tooltips, &self.tooltip_appearance);
                if output.started_capture {
                    self.terminal_agent_state
                        .terminal_views
                        .clear_keymap_chord(context);
                }
                for warning in output.warnings {
                    self.toasts.warning(warning, Instant::now());
                }
                for overrides in output.saves {
                    self.submit(HostCommand::Commands(Command::SetKeymapOverrides(
                        overrides,
                    )));
                }
            }
            Err(error) => self.status = Some(error.to_string()),
        }
        self.show_palette(
            context,
            snapshot,
            command_context,
            raw_overrides.as_deref(),
            keybindings_enabled,
        );
    }

    fn show_palette(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        command_context: &CommandContext,
        keymap_overrides: Option<&str>,
        enabled: bool,
    ) {
        if self.commands_state.palette.is_picking_code_lenses()
            && !self.editor_code_lenses.picker_is_current(&self.store)
        {
            self.commands_state.palette.dismiss(context);
            self.editor_code_lenses.cancel_picker();
        }
        if let Some(picker) = self.editor_code_lenses.picker(&self.store) {
            self.commands_state
                .palette
                .open_code_lenses(context, picker.generation, picker.titles);
        }
        let now = Instant::now();
        let project = command_context.active_project.as_ref();
        self.observe_workspace_symbols(context, project, enabled);
        if let Some(stale) = self.commands_state.palette_files.observe(
            project,
            self.commands_state.palette.observes_files(),
            now,
        ) && !self.submit(HostCommand::Commands(Command::ListProjectFiles(
            stale.clone(),
        ))) {
            self.commands_state.palette_files.accept(
                &stale,
                Err(AppError::Forbidden(
                    "native file index host is disconnected or full".into(),
                )),
                now,
            );
        }
        let active_editor = project
            .and_then(|project| {
                let layout = snapshot.layouts.get(project)?;
                crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)
            })
            .and_then(|(root, focused)| {
                taide_native_ui::snapshot::active_tab(root, focused).map(|tab| (focused, tab))
            });
        let active_file = active_editor.and_then(|(_, tab)| match &tab.kind {
            TabKind::File { path } => Some((tab, path.as_str())),
            _ => None,
        });
        let root = project
            .and_then(|project| snapshot.project(project))
            .map(|project| project.root.as_str());
        let active_document = active_editor.and_then(|(_, tab)| {
            let document = match &tab.kind {
                TabKind::File { path } => self.files.get(path).map(|file| file.id),
                TabKind::Untitled { .. } => self.untitled.get(&tab.id).map(|document| document.id),
                _ => None,
            }?;
            self.store.documents().snapshot(document).ok()
        });
        let mut symbols = self
            .editor_symbols
            .palette(project, active_document.as_ref());
        if let Some((_, path)) = active_file
            && active_document.is_none()
        {
            symbols.is_pending = !self.failed.contains_key(path);
        }
        self.commands_state
            .palette
            .set_reduced_motion(self.motion_preference.current().unwrap_or(false));
        let formatting_source = active_file.and_then(|(tab, _)| {
            self.focused_editor_source(
                context,
                &tab.id,
                active_document.as_ref().map(|document| document.id),
            )
        });
        let output = self.commands_state.palette.show(
            context,
            crate::command_palette::Scope {
                locale: &self.locale,
                commands: command_context,
                keymap_overrides,
                files: self.commands_state.palette_files.view(project, root),
                active_file: active_file.map(|(_, path)| path),
                symbols,
                workspace_symbols: self.explorer_search_state.workspace_symbols.index(project),
            },
            enabled,
        );
        let action = match output {
            Ok(output) => output.action,
            Err(error) => {
                self.status = Some(error.to_string());
                return;
            }
        };
        match action {
            Some(crate::command_palette::Action::CodeLensCommand { generation, index }) => {
                if let Some(request) =
                    self.editor_code_lenses
                        .choose(&self.store, generation, index)
                {
                    if self
                        .lsp
                        .as_ref()
                        .is_none_or(|lsp| lsp.execute_code_lens(request.clone()).is_err())
                    {
                        self.editor_code_lenses.executed(&self.store, &request);
                        self.status = Some(
                            editor_error(taide_native_editor::document::EditorError::NotFound)
                                .to_string(),
                        );
                    }
                    context.request_repaint();
                }
            }
            Some(crate::command_palette::Action::RunTask { generation, task }) => {
                let destination = self
                    .commands_state
                    .task_runner
                    .as_ref()
                    .filter(|(current, _)| *current == generation)
                    .and_then(|(_, project)| {
                        let layout = snapshot.layouts.get(project)?;
                        let (_, pane) =
                            crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)?;
                        Some(crate::editor_context_menu::TerminalDestination {
                            project: project.clone(),
                            scope: self.shell.scope.clone(),
                            pane: pane.clone(),
                            cwd: Some(task.cwd),
                            payload: format!("{}\n", task.command),
                            title: presentation::message(&self.locale, "terminal.title", &[]),
                        })
                    });
                if let Some(destination) = destination {
                    self.submit(HostCommand::Commands(Command::RunTask(destination)));
                }
            }
            Some(crate::command_palette::Action::IndentationSize(size)) => {
                if let Some(request) = self.indentation_request.take() {
                    let configuration = crate::presentation_refresh::indent_configuration(
                        &self.services.state.settings.read(),
                    );
                    if let Err(error) = request.apply(&mut self.store, configuration, size) {
                        self.status = Some(editor_error(error).to_string());
                    }
                    context.request_repaint();
                }
            }
            Some(crate::command_palette::Action::RunCommand(id)) => {
                let inline_command = if id == "ai.inlineEdit" {
                    Some(taide_native_editor::completion::Command::TriggerEdit)
                } else {
                    taide_native_editor::completion::Command::from_action(
                        id.strip_prefix("monaco.").unwrap_or(&id),
                    )
                    .filter(|command| {
                        matches!(
                            command,
                            taide_native_editor::completion::Command::TriggerEdit
                                | taide_native_editor::completion::Command::TriggerInline
                        )
                    })
                };
                if let Some(command) = inline_command
                    && let Some((source, _)) = formatting_source
                {
                    self.editor_completion.queue_command(source, command);
                    context.request_repaint();
                    return;
                }
                if id == "monaco.editor.action.showContextMenu" {
                    if let Some((source, owner)) = formatting_source {
                        if let Err(error) = crate::editor_context_menu::request_palette(
                            context,
                            &self.store,
                            source,
                            owner,
                        ) {
                            self.report(&editor_error(error));
                            return;
                        }
                        if let Some(owner) = self.store.views().get(owner) {
                            self.document_edits
                                .push((owner.key.tab.clone(), DocumentEdit::ContextMenu));
                        }
                    }
                    context.request_repaint();
                    return;
                }
                if matches!(
                    id.as_str(),
                    "monaco.editor.action.smartSelect.expand"
                        | "monaco.editor.action.smartSelect.shrink"
                ) {
                    if let Some((source, owner)) = formatting_source {
                        if let Err(error) = self.editor_selection_ranges.queue(
                            &self.store,
                            crate::editor_selection_ranges::Context {
                                project: project.cloned(),
                                owner,
                                source,
                                viewport: context.viewport_id(),
                            },
                            id == "monaco.editor.action.smartSelect.expand",
                        ) {
                            self.status = Some(editor_error(error).to_string());
                        }
                        context.request_repaint();
                    }
                    return;
                }
                if id == "monaco.codelens.showLensesInCurrentLine" {
                    if let Some((source, _)) = formatting_source {
                        self.editor_code_lenses.begin_picker(
                            &self.store,
                            context.viewport_id(),
                            source,
                        );
                        context.request_repaint();
                    }
                    return;
                }
                if let Some(command) =
                    taide_native_ui::editor_code_actions::Command::from_action(&id)
                    && let Some((tab, _)) = active_file
                {
                    self.request_editor_code_actions(
                        context,
                        &tab.id,
                        active_document.as_ref().map(|document| document.id),
                        command,
                        formatting_source,
                    );
                    context.request_repaint();
                    return;
                }
                if id == "monaco.editor.action.rename"
                    && let Some((tab, _)) = active_file
                {
                    self.request_editor_rename(
                        context,
                        &tab.id,
                        active_document.as_ref().map(|document| document.id),
                        formatting_source,
                    );
                    context.request_repaint();
                    return;
                }
                if let Some(command) = taide_native_editor::formatting::Command::from_action(
                    id.strip_prefix("monaco.").unwrap_or(&id),
                ) && let Some((tab, _)) = active_file
                {
                    self.request_editor_formatting(
                        context,
                        &tab.id,
                        active_document.as_ref().map(|document| document.id),
                        command,
                        formatting_source,
                    );
                    context.request_repaint();
                    return;
                }
                self.commands_state.palette_commands.push(id);
                context.request_repaint();
            }
            Some(crate::command_palette::Action::OpenFile(path)) => {
                let Some(project) = project else {
                    self.notify_open_project_first();
                    return;
                };
                let pane =
                    snapshot
                        .layouts
                        .get(project)
                        .and_then(|layout| match &self.shell.scope {
                            WindowScope::Main => {
                                crate::command_palette::file_tab_pane(layout, &path)
                            }
                            WindowScope::Auxiliary { .. } => crate::symbol_sidebar::window_tree(
                                project,
                                layout,
                                &self.shell.scope,
                            )
                            .map(|(_, focused)| focused.clone()),
                        });
                self.submit(HostCommand::Commands(Command::OpenPaletteFile {
                    project: project.clone(),
                    pane,
                    path,
                }));
            }
            Some(crate::command_palette::Action::OpenWorkspaceSymbol { generation, index }) => {
                if let Some(project) = project
                    && let Some(symbol) = self
                        .explorer_search_state
                        .workspace_symbols
                        .selected(project, generation, index)
                        .cloned()
                    && let Some(layout) = snapshot.layouts.get(project)
                    && let Some((_, focused)) =
                        crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)
                    && let Some(pane) = crate::workspace_symbol_host::destination(
                        layout,
                        project,
                        &self.shell.scope,
                        &symbol.path,
                    )
                {
                    self.submit(HostCommand::ExplorerSearch(
                        crate::domains::explorer_search::Command::OpenWorkspaceSymbol(
                            crate::workspace_symbol_host::Request {
                                project: project.clone(),
                                pane,
                                focused: focused.clone(),
                                revision: layout.revision,
                                scope: self.shell.scope.clone(),
                                path: symbol.path,
                                line: symbol.line,
                                column: symbol.column,
                                viewport: context.viewport_id(),
                            },
                        ),
                    ));
                }
            }
            Some(crate::command_palette::Action::RevealLine(target)) => {
                if let Some(project) = project
                    && let Some((tab, path)) = active_file
                {
                    self.reveals.queue_position(
                        crate::editor_reveal::Target {
                            project,
                            tab: &tab.id,
                            path,
                            viewport: context.viewport_id(),
                        },
                        crate::editor_reveal::Position {
                            line: target.line,
                            column: target.column,
                        },
                        &snapshot.layouts,
                        now,
                    );
                    context.request_repaint();
                }
            }
            Some(crate::command_palette::Action::RevealSymbol { generation, index }) => {
                if let (Some(project), Some((pane, tab)), Some(document)) =
                    (project, active_editor, active_document.as_ref())
                    && matches!(tab.kind, TabKind::Untitled { .. })
                    && let Some(selection) = self
                        .editor_symbols
                        .selection(project, document, generation, index)
                {
                    let key = ViewKey {
                        window: WINDOW_LABEL.into(),
                        pane: pane.clone(),
                        tab: tab.id.clone(),
                    };
                    match crate::editor_reveal::reveal_document(
                        &mut self.store,
                        &key,
                        document,
                        selection,
                    ) {
                        Ok(Some(view)) => {
                            if let Some(body) = crate::editor_context_menu::body(context, view) {
                                context.memory_mut(|memory| memory.request_focus(body));
                            }
                            context.request_repaint();
                        }
                        Ok(None) => {}
                        Err(error) => self.report(&editor_error(error)),
                    }
                }
                if let (Some(project), Some((tab, path)), Some(document)) =
                    (project, active_file, active_document.as_ref())
                    && let Some(position) = self
                        .editor_symbols
                        .position(project, document, generation, index)
                {
                    self.reveals.queue_position(
                        crate::editor_reveal::Target {
                            project,
                            tab: &tab.id,
                            path,
                            viewport: context.viewport_id(),
                        },
                        position,
                        &snapshot.layouts,
                        now,
                    );
                    context.request_repaint();
                }
            }
            None => (),
        }
        if !self.commands_state.palette.is_picking_indentation() {
            self.indentation_request = None;
        }
        self.observe_workspace_symbols(context, project, enabled);
    }
}

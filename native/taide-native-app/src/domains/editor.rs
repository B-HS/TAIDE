use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Ui};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_native_editor::document::{DiskChoice, DocumentId};
use taide_native_editor::store::{EditorStore, SaveSnapshot};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::conflict_banner::{BannerAction, BannerVariant};
use taide_native_ui::document_admission::{PreparedDocument, prepare_opened_document};
use taide_native_ui::editor_surface::{EditorRequest, EditorTokens, FoldControl};
use taide_native_ui::intents::editor::Intent;
use taide_native_ui::shell::WindowScope;
use taide_native_ui::surfaces::editor::EditorSurfaces;
use taide_runtime::{AppServices, layout_actions, native_file_actions};

use crate::application::{
    AppSurfaces, Draft, DraftTarget, FileDocument, NativeApplication, PendingFormatSave,
    PendingTabClose, TabClosePhase, UntitledDocument, WINDOW_LABEL, editor_error,
};
use crate::command_registry::{CommandContext, DocumentEdit, FoldCommand};
use crate::host::{HostCommand, HostReply, HostWorker, open_file_tab, stopped};
use crate::presentation;

pub enum Command {
    EditorContextMenu(crate::editor_context_menu::Request),
    EditorClipboard(crate::editor_context_menu::ClipboardRequest),
    RunEditorText(crate::editor_context_menu::TerminalRequest),
    WriteEditorText(crate::editor_context_menu::TerminalDelivery),
    CodeActions {
        project: ProjectId,
        source: taide_native_editor::view::ViewId,
        owner: taide_native_editor::view::ViewId,
        command: taide_native_ui::editor_code_actions::Command,
    },
    RenameEditor {
        project: taide_model::ids::ProjectId,
        source: taide_native_editor::view::ViewId,
        owner: taide_native_editor::view::ViewId,
    },
    FormatEditorInput {
        project: ProjectId,
        source: taide_native_editor::view::ViewId,
        owner: taide_native_editor::view::ViewId,
        input: taide_native_ui::editor_formatting_input::Input,
    },
    OpenDocumentationFile(crate::editor_documentation::FileRequest),
    OpenSymbolLocation(crate::symbol_location_host::Request),
    ReadPeekModels {
        request: crate::editor_locations::Request,
        paths: Vec<std::path::PathBuf>,
    },
    ReadCompletionClipboard(crate::editor_completion::Request),
    AutoSave {
        path: String,
        snapshot: SaveSnapshot,
    },
    MirrorDraft(crate::persistence::MirrorJob),
    SaveTracked {
        path: String,
        snapshot: SaveSnapshot,
        epoch: crate::persistence::DraftEpoch,
    },
    ObserveFile {
        path: String,
        document: taide_native_editor::document::DocumentId,
    },
    ChooseDisk {
        path: String,
        document: taide_native_editor::document::DocumentId,
        revision: u64,
        choice: taide_native_editor::document::DiskChoice,
    },
    CleanupFileMirror {
        project: ProjectId,
        canonical: std::path::PathBuf,
        expected: Option<taide_model::file::MirrorEntry>,
    },
    NewUntitled {
        project: ProjectId,
        pane: PaneId,
    },
    OpenUntitled(TabId),
    SaveUntitled {
        tab: TabId,
        destination: String,
        snapshot: SaveSnapshot,
    },
    CleanupUntitledMirror {
        project: ProjectId,
        tab: TabId,
        expected: Option<taide_model::file::UntitledMirrorEntry>,
    },
    PruneMirrors(ProjectId),
    SaveMissingDraft {
        tab: TabId,
        draft: crate::missing_draft::MissingDraft,
        destination: String,
    },
    CloseTab {
        tab: TabId,
        discard: bool,
    },
    SaveMirroredTab(TabId),
    OpenFileTab {
        project: ProjectId,
        pane: Option<PaneId>,
        path: String,
        preview: bool,
    },
    OpenFileToSide(taide_model::layout::OpenTabInSplitRequest),
    OpenProblem {
        project: ProjectId,
        path: String,
        line: u64,
        column: u64,
        viewport: eframe::egui::ViewportId,
    },
    OpenMarker(crate::editor_problems::Request),
    OpenBreadcrumbFile {
        source: crate::breadcrumbs::Source,
        path: String,
        viewport: eframe::egui::ViewportId,
    },
    OpenDocument(String),
    Save {
        path: String,
        snapshot: SaveSnapshot,
    },
    SetDirty {
        tab: TabId,
        dirty: bool,
    },
}

pub enum Reply {
    EditorTerminalTarget(
        taide_model::error::AppResult<crate::editor_context_menu::TerminalDelivery>,
    ),
    EditorClipboard {
        request: crate::editor_context_menu::ClipboardRequest,
        result: AppResult<Option<String>>,
    },
    DocumentationFileOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    SymbolLocationOpened {
        request: crate::symbol_location_host::Request,
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    PeekModels {
        request: crate::editor_locations::Request,
        files: Vec<AppResult<taide_model::file::OpenedFile>>,
    },
    BreadcrumbOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    ProblemOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    MarkerOpened {
        request: crate::editor_problems::Request,
        result: AppResult<crate::editor_problems::Opened>,
    },
    CompletionClipboard {
        request: crate::editor_completion::Request,
        result: AppResult<String>,
    },
    DraftMirrored {
        job: crate::persistence::MirrorJob,
        result: AppResult<bool>,
    },
    ObservedFile {
        path: String,
        document: taide_native_editor::document::DocumentId,
        result: AppResult<PreparedDocument>,
    },
    DiskChosen {
        document: taide_native_editor::document::DocumentId,
        revision: u64,
        choice: taide_native_editor::document::DiskChoice,
        result: AppResult<PreparedDocument>,
    },
    UntitledOpened {
        tab: TabId,
        result: AppResult<crate::untitled::PreparedUntitled>,
    },
    UntitledSaved {
        tab: TabId,
        result: AppResult<crate::untitled::PreparedUntitledSave>,
    },
    MissingDraft {
        path: String,
        prepared: crate::missing_draft::PreparedMissingDraft,
    },
    MissingDraftSaved {
        tab: TabId,
        path: String,
        result: AppResult<crate::missing_draft::SavedMissingDraft>,
    },
    Closed {
        tab: TabId,
        result: AppResult<crate::tabs::ClosedNativeTab>,
    },
    MirrorSaved {
        tab: TabId,
        result: AppResult<()>,
    },
    Opened {
        path: String,
        project: Option<ProjectId>,
        result: AppResult<PreparedDocument>,
    },
    Saved {
        snapshot: SaveSnapshot,
        result: AppResult<taide_model::file::OpenedFile>,
    },
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let clipboard = worker.clipboard;
    let read_clipboard = worker.read_clipboard;
    let integrations = worker.integrations;
    let terminals = integrations.terminals.as_ref();
    match command {
        Command::EditorContextMenu(_)
        | Command::FormatEditorInput { .. }
        | Command::RenameEditor { .. }
        | Command::CodeActions { .. } => None,
        Command::RunEditorText(request) => Some(HostReply::Editor(Reply::EditorTerminalTarget(
            crate::editor_context_menu::terminal_target(services, request).await,
        ))),
        Command::WriteEditorText(delivery) => {
            let result = async {
                let id = delivery.session(services).ok_or_else(|| {
                    AppError::NotFound("native editor terminal target is gone".into())
                })?;
                let session = terminals
                    .and_then(|terminals| terminals.tabs.hub().get(&id))
                    .filter(|session| {
                        session.metadata().project_id() == &delivery.project
                            && !session.is_finished()
                            && session.failure().is_none()
                    })
                    .ok_or_else(|| {
                        AppError::NotFound("native editor terminal session is unavailable".into())
                    })?;
                session.write_raw(services, delivery.payload).await
            }
            .await;
            result.err().map(HostReply::Failed)
        }
        Command::EditorClipboard(request) => {
            let owner = request.clone();
            let state = services.state.clone();
            let writer = clipboard.clone();
            let reader = read_clipboard.clone();
            let result = services
                .tasks
                .run_blocking_result("native-editor-menu-clipboard", move || {
                    if state.is_shutting_down()
                        || owner.is_cancelled()
                        || !owner.target.is_active(&state)
                    {
                        return Err(stopped());
                    }
                    let result = match &owner.operation {
                        crate::editor_context_menu::ClipboardOperation::Copy(text)
                        | crate::editor_context_menu::ClipboardOperation::Cut(text) => {
                            writer(&text.text)?;
                            None
                        }
                        crate::editor_context_menu::ClipboardOperation::Paste => {
                            let text = reader()?;
                            if text.len() > taide_model::file::REFUSED_FILE_BYTES as usize {
                                return Err(AppError::InvalidArgument(
                                    "native editor clipboard budget exceeded".into(),
                                ));
                            }
                            Some(text)
                        }
                    };
                    if state.is_shutting_down() || owner.is_cancelled() {
                        return Err(stopped());
                    }
                    Ok(result)
                })
                .await;
            Some(HostReply::Editor(Reply::EditorClipboard {
                request,
                result,
            }))
        }
        Command::OpenDocumentationFile(request) => {
            Some(HostReply::Editor(Reply::DocumentationFileOpened {
                result: request.open(services).await,
            }))
        }
        Command::OpenSymbolLocation(request) => {
            let result = crate::symbol_location_host::open(services, &request).await;
            Some(HostReply::Editor(Reply::SymbolLocationOpened {
                request,
                result,
            }))
        }
        Command::ReadPeekModels { request, paths } => {
            let files = futures_util::future::join_all(paths.into_iter().map(|path| {
                let request = &request;
                async move {
                    if request.is_cancelled() {
                        return Err(AppError::NotFound("native peek request expired".into()));
                    }
                    let path = path
                        .to_str()
                        .ok_or_else(|| {
                            AppError::InvalidArgument("native peek path is invalid".into())
                        })?
                        .to_owned();
                    let plugins = services.plugin.clone();
                    let plugins_dir = services.state.paths.plugins_dir();
                    taide_runtime::file_actions::file_open(
                        &services.state,
                        &services.tasks,
                        path,
                        move || {
                            taide_plugin::service::language_overlays(
                                &taide_plugin::service::ensure_loaded(&plugins, &plugins_dir),
                            )
                        },
                    )
                    .await
                }
            }))
            .await;
            Some(HostReply::Editor(Reply::PeekModels { request, files }))
        }
        Command::ReadCompletionClipboard(request) => {
            let clipboard = read_clipboard.clone();
            let state = services.state.clone();
            let owner = request.clone();
            let result = services
                .tasks
                .run_blocking_result("native-editor-completion-clipboard", move || {
                    if state.is_shutting_down() || owner.is_cancelled() {
                        return Err(stopped());
                    }
                    let text = clipboard()?;
                    if text.len() > taide_model::file::REFUSED_FILE_BYTES as usize {
                        return Err(AppError::InvalidArgument(
                            "native completion clipboard budget exceeded".into(),
                        ));
                    }
                    if state.is_shutting_down() || owner.is_cancelled() {
                        return Err(stopped());
                    }
                    Ok(text)
                })
                .await;
            Some(HostReply::Editor(Reply::CompletionClipboard {
                request,
                result,
            }))
        }
        Command::MirrorDraft(job) => {
            let result = crate::persistence::write_mirror(services, job.clone()).await;
            Some(HostReply::Editor(Reply::DraftMirrored { job, result }))
        }
        Command::SaveTracked {
            path,
            snapshot,
            epoch,
        } => {
            let result =
                crate::file_sync::save_snapshot(services, path, snapshot.clone(), Some(epoch))
                    .await;
            Some(HostReply::Editor(Reply::Saved { snapshot, result }))
        }
        Command::ObserveFile { path, document } => {
            let result = crate::file_sync::prepare_document(services, path.clone()).await;
            Some(HostReply::Editor(Reply::ObservedFile {
                path,
                document,
                result,
            }))
        }
        Command::ChooseDisk {
            path,
            document,
            revision,
            choice,
        } => {
            let result = crate::file_sync::prepare_document(services, path).await;
            Some(HostReply::Editor(Reply::DiskChosen {
                document,
                revision,
                choice,
                result,
            }))
        }
        Command::CleanupFileMirror {
            project,
            canonical,
            expected,
        } => match crate::file_sync::cleanup_mirror(services, project, canonical, expected).await {
            Ok(()) => None,
            Err(error) => Some(HostReply::Failed(error)),
        },
        Command::NewUntitled { project, pane } => {
            let result = async {
                integrations.flush_view_states(services).await?;
                layout_actions::layout_open_untitled(
                    services.events.as_ref(),
                    &services.state,
                    project,
                    Some(pane),
                )
                .await
            }
            .await;
            match result {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        Command::OpenUntitled(tab) => {
            let result = crate::untitled::prepare(services, tab.clone()).await;
            Some(HostReply::Editor(Reply::UntitledOpened { tab, result }))
        }
        Command::SaveUntitled {
            tab,
            destination,
            snapshot,
        } => {
            let result =
                crate::untitled::prepare_save(services, tab.clone(), destination, snapshot).await;
            Some(HostReply::Editor(Reply::UntitledSaved { tab, result }))
        }
        Command::CleanupUntitledMirror {
            project,
            tab,
            expected,
        } => match crate::untitled::cleanup_mirror(services, project, tab, expected).await {
            Ok(()) => None,
            Err(error) => Some(HostReply::Failed(error)),
        },
        Command::PruneMirrors(project) => {
            if let Err(error) = crate::mirror_sweep::prune(services, project).await {
                log::warn!("native mirror prune failed: {:?}", error.kind());
            }
            None
        }
        Command::SaveMissingDraft {
            tab,
            draft,
            destination,
        } => {
            let path = draft.mirror.path.clone();
            let result =
                crate::missing_draft::save(services, tab.clone(), draft, destination).await;
            Some(HostReply::Editor(Reply::MissingDraftSaved {
                tab,
                path,
                result,
            }))
        }
        Command::CloseTab { tab, discard } => {
            let result = async {
                integrations.flush_view_states(services).await?;
                let closed = crate::tabs::close(services, tab.clone(), discard).await?;
                if let TabKind::Terminal { session_id, .. } = &closed.tab.kind
                    && let Some(terminals) = terminals
                {
                    terminals.tabs.close(session_id).await?;
                }
                Ok(closed)
            }
            .await;
            Some(HostReply::Editor(Reply::Closed { tab, result }))
        }
        Command::SaveMirroredTab(tab) => {
            let result = crate::tabs::save_mirrored(services, tab.clone()).await;
            Some(HostReply::Editor(Reply::MirrorSaved { tab, result }))
        }
        Command::OpenProblem {
            project,
            path,
            line,
            column,
            viewport,
        } => {
            let result = open_problem(services, project, None, path, line, column, viewport).await;
            Some(HostReply::Editor(Reply::ProblemOpened { result }))
        }
        Command::OpenMarker(request) => {
            let result = open_marker(services, &request).await;
            Some(HostReply::Editor(Reply::MarkerOpened { request, result }))
        }
        Command::OpenBreadcrumbFile {
            source,
            path,
            viewport,
        } => {
            let result = crate::breadcrumb_host::open(services, &source, path, viewport).await;
            Some(HostReply::Editor(Reply::BreadcrumbOpened { result }))
        }
        Command::OpenFileTab {
            project,
            pane,
            path,
            preview,
        } => {
            let result = async {
                integrations.flush_view_states(services).await?;
                open_file_tab(services, project, pane, path, preview).await
            }
            .await;
            result.err().map(HostReply::Failed)
        }
        Command::OpenFileToSide(request) => {
            let result = async {
                integrations.flush_view_states(services).await?;
                layout_actions::layout_open_tab_in_split(
                    services.events.as_ref(),
                    &services.state,
                    request,
                )
                .await
            }
            .await;
            match result {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        Command::OpenDocument(path) => {
            let plugins = services.plugin.clone();
            let plugins_dir = services.state.paths.plugins_dir();
            let opened = native_file_actions::open_document_file(
                &services.state,
                &services.tasks,
                path.clone(),
                move || {
                    taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(
                        &plugins,
                        &plugins_dir,
                    ))
                },
            )
            .await;
            let mut project = None;
            let result = match opened {
                Ok(opened) => {
                    project = opened.project_id.clone();
                    prepare_opened_document(&services.state, &services.tasks, opened).await
                }
                Err(error) => match crate::missing_draft::prepare(
                    &services.state,
                    &services.tasks,
                    path.clone(),
                )
                .await
                {
                    Ok(Some(prepared)) => {
                        return Some(HostReply::Editor(Reply::MissingDraft { path, prepared }));
                    }
                    Ok(None) => Err(error),
                    Err(missing_error) => Err(missing_error),
                },
            };
            Some(HostReply::Editor(Reply::Opened {
                path,
                project,
                result,
            }))
        }
        Command::Save { path, snapshot } | Command::AutoSave { path, snapshot } => {
            let result =
                crate::file_sync::save_snapshot(services, path, snapshot.clone(), None).await;
            Some(HostReply::Editor(Reply::Saved { snapshot, result }))
        }
        Command::SetDirty { tab, dirty } => {
            match layout_actions::layout_set_dirty(
                services.events.as_ref(),
                &services.state,
                tab,
                dirty,
            )
            .await
            {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
    }
}

async fn open_problem(
    services: &AppServices,
    project: ProjectId,
    pane: Option<PaneId>,
    path: String,
    line: u64,
    column: u64,
    viewport: eframe::egui::ViewportId,
) -> AppResult<crate::terminal_tabs::OpenedFileLink> {
    let max_position = u64::from(u32::MAX) + 1;
    if line == 0 || column == 0 || line > max_position || column > max_position {
        return Err(AppError::InvalidArgument(
            "native problem position is outside the LSP coordinate range".into(),
        ));
    }
    let title = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_owned();
    let layout = layout_actions::layout_open_tab(
        services.events.as_ref(),
        &services.state,
        project.clone(),
        TabKind::File { path: path.clone() },
        title,
        pane.clone(),
        true,
    )
    .await?;
    let pane = pane.unwrap_or_else(|| layout.focused_pane.clone());
    let tab = taide_layout::service::all_roots(&layout)
        .find_map(|root| taide_native_ui::snapshot::active_tab(root, &pane))
        .filter(|tab| matches!(&tab.kind, TabKind::File { path: candidate } if candidate == &path))
        .ok_or_else(|| AppError::NotFound("native problem target file is closed".into()))?
        .id
        .clone();
    Ok(crate::terminal_tabs::OpenedFileLink {
        project,
        pane,
        tab,
        path,
        line: line as f64,
        column: column as f64,
        viewport,
        layout,
    })
}

async fn open_marker(
    services: &AppServices,
    request: &crate::editor_problems::Request,
) -> AppResult<crate::editor_problems::Opened> {
    let source = services
        .state
        .layouts
        .read()
        .get(&request.project)
        .filter(|layout| request.source_is_active(layout))
        .cloned()
        .ok_or_else(|| {
            AppError::NotFound("native problem source editor is closed or inactive".into())
        })?;
    match &request.key {
        taide_native_editor::document::DocumentKey::File(path) => Ok(open_problem(
            services,
            request.project.clone(),
            Some(request.source_key.pane.clone()),
            path.to_string_lossy().into_owned(),
            request.line,
            request.column,
            request.viewport,
        )
        .await?
        .into()),
        taide_native_editor::document::DocumentKey::Untitled(tab) => {
            let (project, target) = {
                let layouts = services.state.layouts.read();
                let project = taide_layout::service::locate_project_with_tab(&layouts, tab)?;
                let target = layouts
                    .get(&project)
                    .cloned()
                    .ok_or_else(|| AppError::NotFound("native problem project is closed".into()))?;
                (project, target)
            };
            let is_untitled = taide_layout::service::all_roots(&target)
                .flat_map(crate::tabs::tabs_in)
                .any(|entry| entry.id == *tab && matches!(entry.kind, TabKind::Untitled { .. }));
            if !is_untitled || !request.allows_destination(&source, &project, &target, tab) {
                return Err(AppError::NotFound(
                    "native problem target is closed or belongs to another window".into(),
                ));
            }
            let layout = layout_actions::layout_activate_tab(
                services.events.as_ref(),
                &services.state,
                tab.clone(),
            )
            .await?;
            let pane = taide_layout::service::all_roots(&layout)
                .find_map(|root| {
                    taide_layout::service::find_tab(root, tab).map(|(pane, _)| pane.clone())
                })
                .ok_or_else(|| AppError::NotFound("native problem target is closed".into()))?;
            if project != request.project {
                taide_runtime::project_actions::project_activate(
                    services.events.as_ref(),
                    &services.state,
                    project.clone(),
                )
                .await?;
            }
            Ok(crate::editor_problems::Opened {
                project,
                pane,
                tab: tab.clone(),
                key: request.key.clone(),
                viewport: request.viewport,
                layout,
            })
        }
        taide_native_editor::document::DocumentKey::AppFile(_) => Err(AppError::InvalidArgument(
            "native problem app-file navigation is not supported".into(),
        )),
    }
}

impl NativeApplication {
    pub(crate) fn apply_editor_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::PeekModels { request, files } => {
                if self.closing.is_none() && self.editor_locations.is_current(&request) {
                    for file in files.into_iter().flatten() {
                        let Ok((owner, _)) =
                            taide_infra::root_guard::resolve_owning_project_or_cli_opened(
                                &self.services.state.projects.read(),
                                &self.services.state.cli_opened_paths.read(),
                                std::path::Path::new(&file.path),
                            )
                        else {
                            continue;
                        };
                        if request.mode == taide_native_editor::symbol_locations::Mode::Hover
                            && !request.describes(&self.store, true)
                        {
                            continue;
                        }
                        match self
                            .peek_models
                            .admit(&mut self.store, file, Instant::now())
                        {
                            Ok(Some(document)) => self.peek_models.associate(document, owner),
                            Ok(None) => {}
                            Err(error) => self.status = Some(editor_error(error).to_string()),
                        }
                    }
                    context.request_repaint();
                }
            }
            Reply::SymbolLocationOpened { request, result } => {
                if self.closing.is_none()
                    && self.editor_locations.is_current_open(&request)
                    && request.source.describes(&self.store, true)
                {
                    self.editor_locations.cancel_open(request.source.source);
                    match result {
                        Ok(opened)
                            if crate::editor_problems::reply_is_current(
                                &opened,
                                &self.services.state.layouts.read(),
                            ) =>
                        {
                            self.controller.apply_layouts(HashMap::from([(
                                opened.project.clone(),
                                opened.layout.clone(),
                            )]));
                            self.reveals.queue(
                                &opened,
                                &self.services.state.layouts.read(),
                                Instant::now(),
                            );
                            self.editor_locations.queue_navigation(
                                &opened,
                                request.target.clone(),
                                Instant::now(),
                            );
                            if request.keep_peek {
                                self.editor_locations.rehome(
                                    request.source.source,
                                    &opened,
                                    Instant::now(),
                                );
                            } else if !request.side {
                                self.editor_locations.close(request.source.source);
                            }
                            context.request_repaint_after(crate::editor_reveal::REVEAL_TTL);
                        }
                        Ok(_) => {}
                        Err(error) => self.report(&error),
                    }
                }
            }
            Reply::ProblemOpened { result } => self.apply_file_link_reply(context, result),
            Reply::DocumentationFileOpened { result } | Reply::BreadcrumbOpened { result } => {
                self.apply_current_file_link_reply(context, result)
            }
            Reply::MarkerOpened { request, result } => {
                let owns_source = !matches!(
                    request.key,
                    taide_native_editor::document::DocumentKey::Untitled(_)
                ) || self
                    .services
                    .state
                    .layouts
                    .read()
                    .get(&request.project)
                    .is_some_and(|layout| request.source_is_owned(layout));
                if !owns_source {
                    self.editor_problems.failed(&request);
                } else if self.closing.is_none()
                    && !self.services.state.is_shutting_down()
                    && self.editor_problems.accepts(&request, &self.store)
                {
                    match result {
                        Ok(opened)
                            if crate::editor_problems::document_reply_is_current(
                                &opened,
                                &self.services.state.layouts.read(),
                            ) =>
                        {
                            match self.editor_problems.opened(
                                &request,
                                &opened,
                                &mut self.store,
                                &self.lsp_diagnostics,
                            ) {
                                Ok(true) => {
                                    self.controller.apply_layouts(HashMap::from([(
                                        opened.project,
                                        opened.layout,
                                    )]));
                                    context.request_repaint();
                                }
                                Ok(false) => {}
                                Err(error) => self.status = Some(editor_error(error).to_string()),
                            }
                        }
                        Ok(_) => self.editor_problems.failed(&request),
                        Err(error) => {
                            self.editor_problems.failed(&request);
                            self.report(&error);
                        }
                    }
                }
            }
            Reply::EditorClipboard { request, result } => {
                if self.closing.is_none()
                    && !self.services.state.is_shutting_down()
                    && request.target.is_active(&self.services.state)
                    && self.editor_clipboard.finish(&request, &self.store)
                {
                    match result {
                        Ok(text) => {
                            if let Err(error) = self.apply_editor_menu_clipboard(&request, text) {
                                self.report(&error);
                            }
                        }
                        Err(error) => self.report(&error),
                    }
                }
            }
            Reply::CompletionClipboard { request, result } => {
                let active = self
                    .services
                    .state
                    .layouts
                    .read()
                    .get(&request.project)
                    .is_some_and(|layout| {
                        self.active_window_scope(|scope| request.is_active(layout, scope))
                            .is_some()
                    });
                if self.closing.is_none()
                    && active
                    && !self.services.state.is_shutting_down()
                    && self.editor_completion.is_current(&request)
                    && request.describes(&self.store)
                {
                    match result {
                        Ok(text) => {
                            self.editor_completion
                                .accept_clipboard(&self.store, &request, text);
                        }
                        Err(error) => {
                            self.editor_completion.close(request.source);
                            self.report(&error);
                        }
                    }
                }
            }
            Reply::EditorTerminalTarget(result) => match result {
                Ok(delivery)
                    if self.closing.is_none() && delivery.session(&self.services).is_some() =>
                {
                    self.editor_terminal_writes.push(delivery)
                }
                Ok(_) => {}
                Err(error) => self.report(&error),
            },
            Reply::DraftMirrored { job, result } => {
                self.persistence.mirror_finished(
                    &job,
                    result.as_ref().is_ok_and(|applied| *applied),
                    Instant::now(),
                );
                if let Err(error) = result {
                    log::warn!("native draft mirror failed: {error}");
                }
            }
            Reply::ObservedFile {
                path,
                document,
                result,
            } => {
                self.observing_files.remove(&document);
                if self.store.documents().snapshot(document).is_err() {
                    return;
                }
                match result
                    .and_then(|prepared| prepared.commit_observation(&mut self.store, document))
                {
                    Ok(_) => {
                        let dirty = self
                            .store
                            .documents()
                            .snapshot(document)
                            .is_ok_and(|snapshot| snapshot.dirty);
                        if !dirty {
                            self.persistence.settled(document);
                        }
                        let tabs = self
                            .store
                            .views()
                            .for_document(document)
                            .map(|view| view.key.tab.clone())
                            .collect::<Vec<_>>();
                        for tab in tabs {
                            self.submit(HostCommand::Editor(Command::SetDirty { tab, dirty }));
                        }
                    }
                    Err(error) => {
                        if error.kind() == taide_model::error::AppErrorKind::NotFound {
                            self.persistence.disable_auto_save(document);
                        }
                        self.status = Some(format!("{path}: {error}"));
                    }
                }
            }
            Reply::DiskChosen {
                document,
                revision,
                choice,
                result,
            } => {
                self.pending_disk_choice = None;
                match result.and_then(|prepared| {
                    prepared.commit_disk_choice(
                        &mut self.store,
                        document,
                        revision,
                        choice,
                        self.services.events.as_ref(),
                    )
                }) {
                    Ok(committed) => {
                        if choice == DiskChoice::ViewDisk {
                            self.persistence.settled(document);
                        }
                        let tabs = self
                            .store
                            .views()
                            .for_document(document)
                            .map(|view| view.key.tab.clone())
                            .collect::<Vec<_>>();
                        for tab in tabs {
                            self.restore_notices.remove(&tab);
                        }
                        if choice == DiskChoice::ViewDisk
                            && let Some(project) = committed.project
                        {
                            self.submit(HostCommand::Editor(Command::CleanupFileMirror {
                                project,
                                canonical: committed.canonical,
                                expected: committed.mirror,
                            }));
                        }
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
            Reply::UntitledOpened { tab, result } => {
                self.untitled_loading.remove(&tab);
                match result.and_then(|prepared| {
                    let project = prepared.project.clone();
                    prepared
                        .commit(&mut self.store)
                        .map(|id| UntitledDocument { id, project })
                }) {
                    Ok(document) => {
                        let dirty = self
                            .store
                            .documents()
                            .snapshot(document.id)
                            .is_ok_and(|snapshot| snapshot.dirty);
                        self.untitled_failed.remove(&tab);
                        self.untitled.insert(tab.clone(), document);
                        self.submit(HostCommand::Editor(Command::SetDirty { tab, dirty }));
                    }
                    Err(error) => {
                        self.untitled_failed.insert(tab, error);
                    }
                }
            }
            Reply::UntitledSaved { tab, result } => {
                self.saving_untitled = None;
                match result.and_then(|prepared| prepared.commit(&mut self.store)) {
                    Ok(converted) => {
                        self.persistence.settled(converted.document);
                        self.pending_dirty.remove(&converted.source_tab);
                        self.pending_dirty
                            .insert(converted.file_tab.clone(), !converted.clean);
                        self.untitled.remove(&converted.source_tab);
                        self.untitled_failed.remove(&converted.source_tab);
                        self.untitled_loading.remove(&converted.source_tab);
                        if let Some(previous) = converted.merged_document {
                            self.persistence.settled(previous);
                            self.last_edited_views.remove(&previous);
                            for file in self.files.values_mut().filter(|file| file.id == previous) {
                                file.id = converted.document;
                            }
                        }
                        let project = taide_infra::root_guard::resolve_owning_project(
                            &self.services.state.projects.read(),
                            &converted.canonical,
                        )
                        .ok()
                        .map(|(project, _)| project);
                        if let Some(path) = converted.canonical.to_str() {
                            self.files.insert(
                                path.into(),
                                FileDocument {
                                    id: converted.document,
                                    project,
                                },
                            );
                            self.failed.remove(path);
                            self.loading.remove(path);
                        }
                        self.focused = None;
                        let file_tab = self
                            .services
                            .state
                            .layouts
                            .read()
                            .values()
                            .flat_map(taide_layout::service::all_roots)
                            .flat_map(crate::tabs::tabs_in)
                            .find(|candidate| candidate.id == converted.file_tab)
                            .cloned();
                        if let Some(pending) = self
                            .pending_tab_close
                            .as_mut()
                            .filter(|pending| pending.tab.id == tab)
                            && let Some(file_tab) = file_tab
                        {
                            pending.tab = file_tab;
                            pending.phase = if converted.clean {
                                TabClosePhase::Ready { discard: false }
                            } else {
                                TabClosePhase::Confirm
                            };
                        }
                        self.submit(HostCommand::Editor(Command::CleanupUntitledMirror {
                            project: converted.project,
                            tab: converted.source_tab,
                            expected: converted.mirror,
                        }));
                    }
                    Err(error) => {
                        self.report(&error);
                        if let Some(pending) = self
                            .pending_tab_close
                            .as_mut()
                            .filter(|pending| pending.tab.id == tab)
                        {
                            pending.phase = TabClosePhase::Confirm;
                        }
                    }
                }
            }
            Reply::MissingDraft { path, prepared } => {
                self.loading.remove(&path);
                self.loading_file_tabs.remove(&path);
                match prepared.commit() {
                    Ok(draft) => {
                        self.failed.remove(&path);
                        self.missing_drafts.insert(path.clone(), draft);
                        let tabs = self.controller.snapshot().layouts.values()
                            .flat_map(taide_layout::service::all_roots).flat_map(crate::tabs::tabs_in)
                            .filter(|tab| matches!(&tab.kind, TabKind::File { path: tab_path } if tab_path == &path))
                            .map(|tab| tab.id.clone()).collect::<Vec<_>>();
                        for tab in tabs {
                            self.submit(HostCommand::Editor(Command::SetDirty {
                                tab,
                                dirty: true,
                            }));
                        }
                    }
                    Err(error) => {
                        self.failed.insert(path, error);
                    }
                }
            }
            Reply::MissingDraftSaved { tab, path, result } => {
                self.saving_missing_draft = None;
                match result {
                    Ok(saved) => {
                        let key = taide_native_editor::document::DocumentKey::File(
                            saved.destination.clone(),
                        );
                        if let Some(document) = self.store.documents().find(&key)
                            && let Err(error) = self.store.refresh_clean_file(
                                document,
                                &saved.destination,
                                saved.opened,
                            )
                        {
                            self.status = Some(editor_error(error).to_string());
                        }
                        let paths = self
                            .missing_drafts
                            .iter()
                            .filter(|(_, draft)| draft.canonical == saved.source)
                            .map(|(path, _)| path.clone())
                            .collect::<Vec<_>>();
                        for path in paths {
                            self.missing_drafts.remove(&path);
                            self.failed.remove(&path);
                            self.loading.remove(&path);
                        }
                        self.failed.remove(&path);
                        if let Some(pending) = self
                            .pending_tab_close
                            .as_mut()
                            .filter(|pending| pending.tab.id == tab)
                        {
                            pending.phase = TabClosePhase::Ready { discard: false };
                        }
                    }
                    Err(error) => {
                        self.report(&error);
                        if let Some(pending) = self
                            .pending_tab_close
                            .as_mut()
                            .filter(|pending| pending.tab.id == tab)
                        {
                            pending.phase = TabClosePhase::Confirm;
                        }
                        if self.closing.is_none()
                            && !self.services.state.is_shutting_down()
                            && self.missing_drafts.contains_key(&path)
                            && self.loading.insert(path.clone())
                        {
                            self.loading_file_tabs.insert(path.clone(), tab.clone());
                            if !self
                                .submit(HostCommand::Editor(Command::OpenDocument(path.clone())))
                            {
                                self.loading.remove(&path);
                                self.loading_file_tabs.remove(&path);
                            }
                        }
                    }
                }
            }
            Reply::Closed { tab, result } => {
                let approved = self
                    .pending_tab_close
                    .as_ref()
                    .filter(|pending| pending.tab.id == tab)
                    .and_then(|pending| pending.approved_discard);
                let mut batch = if self
                    .pending_tab_close
                    .as_ref()
                    .is_some_and(|pending| pending.tab.id == tab)
                {
                    self.pending_tab_close
                        .take()
                        .and_then(|pending| pending.batch)
                } else {
                    None
                };
                match result {
                    Ok(closed) => {
                        self.restore_notices.remove(&tab);
                        self.pending_dirty.remove(&tab);
                        if let Err(error) = self.release_tab(closed, approved) {
                            self.report_close_failure(batch.as_mut(), &error);
                        }
                    }
                    Err(AppError::NotFound(_)) if batch.is_some() => (),
                    Err(error) => self.report_close_failure(batch.as_mut(), &error),
                }
                if let Some(batch) = batch {
                    self.advance_close_batch(batch);
                }
            }
            Reply::MirrorSaved { tab, result } => match result {
                Ok(()) => {
                    self.submit(HostCommand::Editor(Command::SetDirty {
                        tab: tab.clone(),
                        dirty: false,
                    }));
                    if let Some(pending) = self
                        .pending_tab_close
                        .as_mut()
                        .filter(|pending| pending.tab.id == tab)
                    {
                        pending.phase = TabClosePhase::Ready { discard: false };
                    }
                }
                Err(error) => {
                    self.report(&error);
                    if let Some(pending) = self
                        .pending_tab_close
                        .as_mut()
                        .filter(|pending| pending.tab.id == tab)
                    {
                        pending.phase = TabClosePhase::Confirm;
                    }
                }
            },
            Reply::Opened {
                path,
                project,
                result,
            } => {
                self.loading.remove(&path);
                let source_tab = self.loading_file_tabs.remove(&path);
                match result.and_then(|prepared| prepared.commit_file(&mut self.store)) {
                    Ok(taide_native_ui::document_admission::CommittedFile::Document(committed)) => {
                        self.failed.remove(&path);
                        self.refused.remove(&path);
                        self.peek_models.adopt(committed.document);
                        self.missing_drafts.remove(&path);
                        if let Some(conflict) = committed.restored_conflict
                            && let Some(tab) = source_tab
                        {
                            self.restore_notices.insert(
                                tab,
                                if conflict {
                                    BannerVariant::MirrorRestoredConflict
                                } else {
                                    BannerVariant::MirrorRestored
                                },
                            );
                        }
                        self.files.insert(
                            path,
                            FileDocument {
                                id: committed.document,
                                project,
                            },
                        );
                        let tabs = self.controller.snapshot().layouts.values().flat_map(|layout| crate::tabs::tabs_in(&layout.root).into_iter()
                            .chain(layout.auxiliary_windows.iter().flat_map(|window| crate::tabs::tabs_in(&window.root))))
                            .filter(|tab| matches!(&tab.kind, TabKind::File { path: tab_path } if self.files.get(tab_path).is_some_and(|file| file.id == committed.document)))
                            .map(|tab| tab.id.clone()).collect::<Vec<_>>();
                        let dirty = self
                            .store
                            .documents()
                            .snapshot(committed.document)
                            .is_ok_and(|document| document.dirty);
                        for tab in tabs {
                            self.submit(HostCommand::Editor(Command::SetDirty { tab, dirty }));
                        }
                    }
                    Ok(taide_native_ui::document_admission::CommittedFile::Refused) => {
                        self.failed.remove(&path);
                        self.missing_drafts.remove(&path);
                        self.refused.insert(path);
                    }
                    Err(error) => {
                        self.refused.remove(&path);
                        self.failed.insert(path, error);
                    }
                }
            }
            Reply::Saved { snapshot, result } => {
                let document = snapshot.document();
                let was_saved = result.is_ok();
                if was_saved {
                    self.reconcile_lsp();
                    if let Some(lsp) = &self.lsp
                        && let Err(error) = lsp.saved(document)
                    {
                        log::warn!("native save notification failed: {error}");
                    }
                }
                let key = snapshot.key().clone();
                match result.and_then(|file| {
                    self.store
                        .mark_saved(snapshot, Some(file.modified_ms))
                        .map_err(editor_error)?;
                    let taide_native_editor::document::DocumentKey::File(canonical) = key else {
                        return Err(editor_error(
                            taide_native_editor::document::EditorError::InvalidIdentity,
                        ));
                    };
                    self.store
                        .observe_file(document, &canonical, file)
                        .map_err(editor_error)?;
                    self.store
                        .documents()
                        .snapshot(document)
                        .map(|snapshot| !snapshot.dirty)
                        .map_err(editor_error)
                }) {
                    Ok(clean) => {
                        self.deleted_documents.remove(&document);
                        let tabs: Vec<_> = self
                            .store
                            .views()
                            .for_document(document)
                            .map(|view| view.key.tab.clone())
                            .collect();
                        let dirty = self
                            .store
                            .documents()
                            .snapshot(document)
                            .is_ok_and(|snapshot| snapshot.dirty);
                        for tab in tabs {
                            self.restore_notices.remove(&tab);
                            self.submit(HostCommand::Editor(Command::SetDirty { tab, dirty }));
                        }
                        if let Some(pending) = self.pending_tab_close.as_mut()
                            && matches!(pending.phase, TabClosePhase::Saving(Some(id)) if id == document)
                        {
                            if clean {
                                pending.phase = TabClosePhase::Ready { discard: false };
                            } else {
                                pending.phase = TabClosePhase::Confirm;
                                self.status =
                                    Some("native document changed during close save".into());
                            }
                        }
                    }
                    Err(error) => {
                        self.report(&error);
                        if let Some(pending) = self.pending_tab_close.as_mut()
                            && matches!(pending.phase, TabClosePhase::Saving(Some(id)) if id == document)
                        {
                            pending.phase = TabClosePhase::Confirm;
                        }
                    }
                }
                if let Ok(snapshot) = self.store.documents().snapshot(document) {
                    let mirror = self.draft_project(document).is_some();
                    let auto_save = if was_saved {
                        self.auto_save_delay(document)
                    } else {
                        Duration::ZERO
                    };
                    self.persistence.save_finished(
                        document,
                        Instant::now(),
                        snapshot.dirty,
                        mirror,
                        auto_save,
                    );
                } else {
                    self.persistence.settled(document);
                }
            }
        }
    }

    pub(crate) fn apply_editor_intent(
        &mut self,
        intent: Intent,
        context: &egui::Context,
        frame: &eframe::Frame,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        command_context: &CommandContext,
        keymap_documents: &HashMap<TabId, DocumentId>,
    ) {
        match intent {
            Intent::EditDocument { tab, edit } => {
                if edit == DocumentEdit::RunSelectedText {
                    let project = command_context.active_project.as_ref();
                    let owner = project
                        .and_then(|project| {
                            crate::symbol_sidebar::window_tree(
                                project,
                                snapshot.layouts.get(project)?,
                                &self.shell.scope,
                            )
                        })
                        .and_then(|(_, pane)| {
                            self.store.views().find(&ViewKey {
                                window: WINDOW_LABEL.into(),
                                pane: pane.clone(),
                                tab: tab.clone(),
                            })
                        });
                    if let Some(owner) = owner {
                        match crate::editor_context_menu::Request::run_selected_text(
                            context,
                            &self.store,
                            owner,
                            project,
                            &self.shell.scope,
                        ) {
                            Ok(request) => {
                                self.submit(HostCommand::Editor(Command::EditorContextMenu(
                                    request,
                                )));
                            }
                            Err(error) => self.report(&editor_error(error)),
                        }
                    }
                    return;
                }
                self.document_edits.push((tab, edit));
                context.request_repaint();
            }
            Intent::ChooseIndentation { tab, command } => {
                let focused = self
                    .commands_state
                    .palette
                    .source_focus()
                    .filter(|_| self.commands_state.palette.is_open())
                    .or_else(|| context.memory(|memory| memory.focused()));
                let source = focused
                    .and_then(|id| self.editor_keymap_targets.get(&(context.viewport_id(), id)))
                    .and_then(|(source, _)| {
                        let owner = self
                            .editor_locations
                            .preview_owner(*source)
                            .unwrap_or(*source);
                        let owner = self.store.views().get(owner)?;
                        (owner.key.tab == tab)
                            .then(|| self.store.views().get(*source).map(|view| view.document))
                            .flatten()
                    })
                    .or_else(|| keymap_documents.get(&tab).copied());
                let configuration = crate::presentation_refresh::indent_configuration(
                    &self.services.state.settings.read(),
                );
                match source.map(|document| {
                    crate::editor_indentation::Request::new(
                        &mut self.store,
                        document,
                        configuration,
                        command,
                    )
                }) {
                    Some(Ok((request, current_size))) => {
                        self.indentation_request = Some(request);
                        self.commands_state.palette.open_indentation(
                            context,
                            current_size,
                            configuration.defaults.tab_size,
                        );
                    }
                    result => {
                        self.indentation_request = None;
                        self.commands_state.palette.dismiss(context);
                        if let Some(Err(error)) = result {
                            self.status = Some(editor_error(error).to_string());
                        }
                    }
                }
            }
            Intent::FoldDocument { tab, command } => {
                self.fold_commands.push((tab, command));
                context.request_repaint();
            }
            Intent::NewUntitled { project, pane } => {
                self.submit(HostCommand::Editor(Command::NewUntitled { project, pane }));
            }
            Intent::FormatEditor { tab, command } => {
                self.request_editor_formatting(
                    context,
                    &tab,
                    keymap_documents.get(&tab).copied(),
                    command,
                    None,
                );
                context.request_repaint();
            }
            Intent::CodeActions { tab, command } => {
                self.request_editor_code_actions(
                    context,
                    &tab,
                    keymap_documents.get(&tab).copied(),
                    command,
                    None,
                );
                context.request_repaint();
            }
            Intent::RenameEditor { tab } => {
                self.request_editor_rename(
                    context,
                    &tab,
                    keymap_documents.get(&tab).copied(),
                    None,
                );
                context.request_repaint();
            }
            Intent::ToggleEditorStickyScroll => {
                self.editor_sticky_scroll.toggle(
                    self.services
                        .state
                        .settings
                        .read()
                        .editor_sticky_scroll_enabled,
                );
                context.request_repaint();
            }
            Intent::RequestSaveTab(tab) => {
                let document = keymap_documents.get(&tab).copied();
                self.request_tab_save(tab, document, frame);
            }
        }
    }

    pub(crate) fn intercept_editor_command(
        &mut self,
        command: HostCommand,
    ) -> ControlFlow<bool, HostCommand> {
        if let HostCommand::Editor(Command::EditorContextMenu(request)) = command {
            #[cfg(test)]
            let context = request.context.clone();
            let result = self.run_editor_context_menu(request);
            #[cfg(test)]
            context.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("native-editor-context-menu-result"),
                    format!("{result:?}"),
                )
            });
            return ControlFlow::Break(match result {
                Ok(handled) => handled,
                Err(error) => {
                    self.report(&error);
                    false
                }
            });
        }
        if let HostCommand::Editor(Command::CodeActions {
            project,
            source,
            owner,
            command,
        }) = command
        {
            self.request_editor_code_actions_from_source(
                project,
                source,
                owner,
                crate::editor_code_actions::Invocation {
                    command,
                    automatic: false,
                },
            );
            return ControlFlow::Break(true);
        }
        if let HostCommand::Editor(Command::RenameEditor {
            project,
            source,
            owner,
        }) = command
        {
            self.request_editor_rename_from_source(project, source, owner);
            return ControlFlow::Break(true);
        }
        if let HostCommand::Editor(Command::FormatEditorInput {
            project,
            source,
            owner,
            input,
        }) = command
        {
            return ControlFlow::Break(
                self.request_editor_input_formatting(project, source, owner, input),
            );
        }
        ControlFlow::Continue(command)
    }

    pub(crate) fn prepare_editor_submission(
        &mut self,
        command: HostCommand,
    ) -> ControlFlow<bool, (HostCommand, Option<DocumentId>)> {
        if let HostCommand::Editor(Command::SetDirty { tab, dirty }) = command {
            if self.has_layout_tab(&tab) {
                self.pending_dirty.insert(tab, dirty);
            }
            return ControlFlow::Break(true);
        }
        let command = match command {
            HostCommand::Editor(Command::SaveUntitled {
                tab,
                destination,
                snapshot,
            }) => {
                let document = snapshot.document();
                if self.pending_format_saves.contains_key(&document) {
                    return ControlFlow::Break(false);
                }
                let resumed = self.resumed_save_epochs.remove(&document);
                if resumed.as_ref().is_some_and(|epoch| !epoch.is_current()) {
                    return ControlFlow::Break(false);
                }
                let current = self.store.documents().snapshot(document).ok();
                let should_format = self.services.state.settings.read().format_on_save;
                if resumed.is_none()
                    && should_format
                    && let Some(current) = current.filter(|current| {
                        current.key == *snapshot.key()
                            && current.revision == snapshot.revision()
                            && crate::editor_formatting::is_native_eligible(current)
                    })
                {
                    let options = match self.save_formatting_options(document) {
                        Ok(options) => options,
                        Err(error) => {
                            self.report_save_failure(error, false);
                            return ControlFlow::Break(false);
                        }
                    };
                    let Some(epoch) = self.persistence.begin_save(document) else {
                        return ControlFlow::Break(false);
                    };
                    self.queue_native_save(
                        current,
                        PendingFormatSave::new(destination, false, epoch, Some(tab)),
                        options,
                    );
                    return ControlFlow::Break(true);
                }
                let snapshot = match self.prepare_untitled_cleanup(snapshot) {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        self.report_save_failure(error, false);
                        return ControlFlow::Break(false);
                    }
                };
                HostCommand::Editor(Command::SaveUntitled {
                    tab,
                    destination,
                    snapshot,
                })
            }
            other => other,
        };
        let mut tracked_save = None;
        let (save, command) = match command {
            HostCommand::Editor(Command::Save { path, snapshot }) => {
                (Some((path, snapshot, false)), None)
            }
            HostCommand::Editor(Command::AutoSave { path, snapshot }) => {
                (Some((path, snapshot, true)), None)
            }
            other => (None, Some(other)),
        };
        let command = if let Some((path, snapshot, auto_save)) = save {
            let document = snapshot.document();
            if self.pending_format_saves.contains_key(&document) {
                return ControlFlow::Break(false);
            }
            let (should_format, actions) = {
                let settings = self.services.state.settings.read();
                (
                    settings.format_on_save,
                    crate::lsp::SaveActionFlags {
                        fix_all: !auto_save && settings.fix_all_on_save,
                        organize_imports: !auto_save && settings.organize_imports_on_save,
                    },
                )
            };
            if !self.resumed_save_epochs.contains_key(&document)
                && (should_format || actions.fix_all || actions.organize_imports)
                && self
                    .store
                    .documents()
                    .snapshot(document)
                    .is_ok_and(|current| current.dirty && !current.metadata.read_only)
                && (self.lsp.is_some()
                    || should_format
                        && self
                            .store
                            .documents()
                            .snapshot(document)
                            .is_ok_and(|current| {
                                crate::editor_formatting::is_native_eligible(&current)
                            }))
            {
                self.reconcile_lsp();
                let Some(epoch) = self.persistence.begin_save(document) else {
                    return ControlFlow::Break(false);
                };
                let options = match self.save_formatting_options(document) {
                    Ok(options) => options,
                    Err(_) => {
                        self.persistence.submission_failed(document);
                        return ControlFlow::Break(false);
                    }
                };
                let current = match self.store.documents().snapshot(document) {
                    Ok(current)
                        if current.revision == snapshot.revision()
                            && &current.key == snapshot.key() =>
                    {
                        current
                    }
                    _ => {
                        self.persistence.submission_failed(document);
                        return ControlFlow::Break(false);
                    }
                };
                let native = should_format
                    && crate::editor_formatting::is_native_eligible(&current)
                    && self
                        .lsp
                        .as_ref()
                        .is_none_or(|lsp| !lsp.has_save_formatter(&current));
                let mut pending =
                    PendingFormatSave::new(path.clone(), auto_save, epoch.clone(), None);
                if native && self.lsp.is_none() {
                    self.queue_native_save(current, pending, options);
                    return ControlFlow::Break(true);
                }
                let result = self
                    .lsp
                    .as_ref()
                    .expect("save participation has a bridge")
                    .participate(
                        current.clone(),
                        (should_format && !native).then_some(options.clone()),
                        actions,
                    );
                match result {
                    Ok(()) => {
                        pending.native_options = native.then_some(options);
                        self.pending_format_saves.insert(document, pending);
                        return ControlFlow::Break(true);
                    }
                    Err(error) => {
                        log::warn!("native save participation failed: {error}");
                        if native {
                            self.queue_native_save(current, pending, options);
                            return ControlFlow::Break(true);
                        }
                        self.resumed_save_epochs.insert(document, epoch);
                    }
                }
            }
            let Some(epoch) = self
                .resumed_save_epochs
                .remove(&document)
                .or_else(|| self.persistence.begin_save(document))
            else {
                return ControlFlow::Break(false);
            };
            let view = self.save_view(document);
            let flags = {
                let settings = self.services.state.settings.read();
                taide_native_editor::save_cleanup::CleanupFlags {
                    trim_trailing_whitespace: settings.trim_trailing_whitespace_on_save,
                    insert_final_newline: settings.insert_final_newline_on_save,
                }
            };
            self.editor_syntax
                .supply_save_cleanup(&mut self.store, document, flags);
            let prepared =
                match crate::save::prepare(&mut self.store, snapshot, view, flags, auto_save) {
                    Ok(Some(prepared)) => prepared,
                    Ok(None) => {
                        self.persistence.settled(document);
                        for view in self.store.views().for_document(document) {
                            self.pending_dirty.insert(view.key.tab.clone(), false);
                        }
                        return ControlFlow::Break(true);
                    }
                    Err(error) => {
                        self.persistence.submission_failed(document);
                        self.report_save_failure(error, auto_save);
                        return ControlFlow::Break(false);
                    }
                };
            if prepared.changed {
                let mirror = self.draft_project(document).is_some();
                self.persistence.changed(
                    document,
                    prepared.snapshot.revision(),
                    Instant::now(),
                    mirror,
                    Duration::ZERO,
                );
                let dirty = self
                    .store
                    .documents()
                    .snapshot(document)
                    .is_ok_and(|snapshot| snapshot.dirty);
                for view in self.store.views().for_document(document) {
                    self.pending_dirty.insert(view.key.tab.clone(), dirty);
                }
            }
            tracked_save = Some(document);
            HostCommand::Editor(Command::SaveTracked {
                path,
                snapshot: prepared.snapshot,
                epoch,
            })
        } else if let Some(command) = command {
            command
        } else {
            return ControlFlow::Break(false);
        };
        ControlFlow::Continue((command, tracked_save))
    }
}

impl NativeApplication {
    fn save_view(&self, document: DocumentId) -> Option<ViewId> {
        self.last_edited_views
            .get(&document)
            .copied()
            .filter(|view| {
                self.store
                    .views()
                    .get(*view)
                    .is_some_and(|view| view.document == document)
            })
            .or_else(|| {
                self.store
                    .views()
                    .for_document(document)
                    .find(|view| {
                        self.focused.as_ref().is_some_and(|(pane, tab)| {
                            &view.key.pane == pane && &view.key.tab == tab
                        })
                    })
                    .map(|view| view.id)
            })
            .or_else(|| {
                self.store
                    .views()
                    .for_document(document)
                    .next()
                    .map(|view| view.id)
            })
    }

    pub(crate) fn focused_editor_source(
        &self,
        context: &egui::Context,
        tab: &TabId,
        fallback: Option<DocumentId>,
    ) -> Option<(ViewId, ViewId)> {
        let focused = self
            .commands_state
            .palette
            .source_focus()
            .filter(|_| self.commands_state.palette.is_open())
            .or_else(|| context.memory(|memory| memory.focused()));
        focused
            .and_then(|id| self.editor_keymap_targets.get(&(context.viewport_id(), id)))
            .and_then(|(source, _)| {
                let owner = self
                    .editor_locations
                    .preview_owner(*source)
                    .unwrap_or(*source);
                let owner_view = self.store.views().get(owner)?;
                (owner_view.key.tab == *tab && self.store.views().get(*source).is_some())
                    .then_some((*source, owner))
            })
            .or_else(|| {
                fallback.and_then(|document| {
                    self.store
                        .views()
                        .for_document(document)
                        .find(|view| view.key.tab == *tab)
                        .map(|view| (view.id, view.id))
                })
            })
    }

    pub(crate) fn request_editor_code_actions(
        &mut self,
        context: &egui::Context,
        tab: &TabId,
        fallback: Option<DocumentId>,
        command: taide_native_ui::editor_code_actions::Command,
        source: Option<(ViewId, ViewId)>,
    ) {
        let Some((source, owner)) =
            source.or_else(|| self.focused_editor_source(context, tab, fallback))
        else {
            return;
        };
        let snapshot = self.controller.snapshot();
        let Some(project) = snapshot.layouts.iter().find_map(|(project, layout)| {
            taide_layout::service::all_roots(layout)
                .any(|root| {
                    crate::tabs::tabs_in(root)
                        .iter()
                        .any(|candidate| candidate.id == *tab)
                })
                .then_some(project.clone())
        }) else {
            return;
        };
        self.request_editor_code_actions_from_source(
            project,
            source,
            owner,
            crate::editor_code_actions::Invocation {
                command,
                automatic: false,
            },
        );
    }

    pub(crate) fn request_editor_code_actions_from_source(
        &mut self,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        invocation: crate::editor_code_actions::Invocation,
    ) {
        if self.closing.is_some() || self.is_exit_ready || self.services.state.is_shutting_down() {
            return;
        }
        if source != owner && self.editor_locations.preview_owner(source) != Some(owner) {
            return;
        }
        let shell = self.controller.snapshot();
        if !self.store.views().get(owner).is_some_and(|owner| {
            shell.layouts.get(&project).is_some_and(|layout| {
                taide_layout::service::all_roots(layout).any(|root| {
                    taide_native_ui::snapshot::active_tab(root, &owner.key.pane)
                        .is_some_and(|tab| tab.id == owner.key.tab)
                })
            })
        }) {
            return;
        }
        self.reconcile_lsp();
        let Some(lsp) = &self.lsp else {
            return;
        };
        match self
            .editor_code_actions
            .begin(&self.store, project, source, owner, invocation, lsp)
        {
            Ok(_) | Err(taide_native_editor::document::EditorError::ReadOnly) => (),
            Err(error) => self.status = Some(editor_error(error).to_string()),
        }
    }

    pub(crate) fn request_editor_rename(
        &mut self,
        context: &egui::Context,
        tab: &TabId,
        fallback: Option<DocumentId>,
        source: Option<(ViewId, ViewId)>,
    ) {
        let Some((source, owner)) =
            source.or_else(|| self.focused_editor_source(context, tab, fallback))
        else {
            return;
        };
        let snapshot = self.controller.snapshot();
        let Some(project) = snapshot.layouts.iter().find_map(|(project, layout)| {
            taide_layout::service::all_roots(layout)
                .any(|root| {
                    crate::tabs::tabs_in(root)
                        .iter()
                        .any(|candidate| candidate.id == *tab)
                })
                .then_some(project.clone())
        }) else {
            return;
        };
        self.request_editor_rename_from_source(project, source, owner);
    }

    pub(crate) fn request_editor_rename_from_source(
        &mut self,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
    ) {
        if self.closing.is_some() || self.is_exit_ready || self.services.state.is_shutting_down() {
            return;
        }
        if source != owner && self.editor_locations.preview_owner(source) != Some(owner) {
            return;
        }
        let snapshot = self.controller.snapshot();
        let active = self.store.views().get(owner).is_some_and(|owner| {
            snapshot.layouts.get(&project).is_some_and(|layout| {
                taide_layout::service::all_roots(layout).any(|root| {
                    taide_native_ui::snapshot::active_tab(root, &owner.key.pane)
                        .is_some_and(|tab| tab.id == owner.key.tab)
                })
            })
        });
        if !active {
            return;
        }
        self.reconcile_lsp();
        let Some(lsp) = &self.lsp else {
            return;
        };
        match self
            .editor_rename
            .begin(&self.store, project, source, owner, lsp)
        {
            Ok(_) | Err(taide_native_editor::document::EditorError::ReadOnly) => (),
            Err(error) => self.status = Some(editor_error(error).to_string()),
        }
    }

    fn formatted_document(&mut self, document: DocumentId) {
        if let Ok(current) = self.store.documents().snapshot(document) {
            let mirror = self.draft_project(current.id).is_some();
            self.persistence.changed(
                current.id,
                current.revision,
                Instant::now(),
                mirror,
                Duration::ZERO,
            );
            for view in self.store.views().for_document(current.id) {
                self.pending_dirty.insert(view.key.tab.clone(), true);
            }
        }
        self.reconcile_lsp();
    }

    pub(crate) fn save_formatting_options(
        &mut self,
        document: DocumentId,
    ) -> Result<
        taide_lsp::native::protocol::lsp_types::FormattingOptions,
        taide_native_editor::document::EditorError,
    > {
        let configuration =
            crate::presentation_refresh::indent_configuration(&self.services.state.settings.read());
        let fallback = self.store.configure_indentation(document, configuration)?;
        let current = self.store.documents().snapshot(document)?;
        let indent = current.model_indentation(fallback).formatting_options();
        Ok(taide_lsp::native::protocol::lsp_types::FormattingOptions {
            tab_size: indent.tab_size,
            insert_spaces: indent.insert_spaces,
            ..Default::default()
        })
    }

    fn prepare_untitled_cleanup(
        &mut self,
        snapshot: taide_native_editor::store::SaveSnapshot,
    ) -> Result<taide_native_editor::store::SaveSnapshot, taide_native_editor::document::EditorError>
    {
        let document = snapshot.document();
        let current = self.store.documents().snapshot(document)?;
        if current.key != *snapshot.key() || current.revision != snapshot.revision() {
            return Err(taide_native_editor::document::EditorError::StaleRevision);
        }
        let settings = self.services.state.settings.read();
        let flags = taide_native_editor::save_cleanup::CleanupFlags {
            trim_trailing_whitespace: settings.trim_trailing_whitespace_on_save,
            insert_final_newline: settings.insert_final_newline_on_save,
        };
        drop(settings);
        let view = self.save_view(document);
        self.editor_syntax
            .supply_save_cleanup(&mut self.store, document, flags);
        let result =
            taide_native_editor::save_cleanup::run(&mut self.store, document, view, flags, false)?;
        if result.changed {
            self.formatted_document(document);
        }
        self.store.save_snapshot(document)
    }

    fn queue_native_save(
        &mut self,
        snapshot: taide_native_editor::document::DocumentSnapshot,
        mut pending: PendingFormatSave,
        options: taide_lsp::native::protocol::lsp_types::FormattingOptions,
    ) {
        let document = snapshot.id;
        pending.native_options = None;
        pending.native_token = Some(self.editor_formatting.begin_native_save(snapshot, options));
        self.pending_format_saves.insert(document, pending);
    }

    pub(crate) fn poll_native_formatting(&mut self) {
        for (request, result) in self
            .editor_formatting
            .tick_native_saves(&self.store, &self.services.tasks)
        {
            if self
                .pending_format_saves
                .get(&request.snapshot.id)
                .is_none_or(|pending| pending.native_token != Some(request.token))
            {
                continue;
            }
            let result = if self.services.state.settings.read().format_on_save {
                result.map(Some).map_err(|error| {
                    if error != taide_native_editor::document::EditorError::StaleRevision {
                        log::warn!("native JSON save formatter failed: {error:?}");
                    }
                    taide_lsp::native::Failure::StaleRevision
                })
            } else {
                Ok(None)
            };
            self.finish_formatted_save(request.snapshot, result);
        }
        for (request, result) in self.editor_formatting.tick_native(&self.services.tasks) {
            if self.closing.is_some()
                || self.services.state.is_shutting_down()
                || !request.enabled(&self.services.state.settings.read())
            {
                self.editor_formatting.reject(&request);
                continue;
            }
            let providers = self
                .lsp
                .as_ref()
                .map(|lsp| lsp.formatting_request_providers(&request))
                .unwrap_or_default();
            match self
                .editor_formatting
                .accept_native(&mut self.store, &request, providers, result)
            {
                Ok(true) => self.formatted_document(request.snapshot.id),
                Ok(false) => (),
                Err(error) => log::warn!("native JSON formatter edit failed: {error:?}"),
            }
        }
    }

    pub(crate) fn request_editor_formatting(
        &mut self,
        context: &egui::Context,
        tab: &TabId,
        fallback: Option<DocumentId>,
        command: taide_native_editor::formatting::Command,
        source: Option<(ViewId, ViewId)>,
    ) {
        let Some((source, owner)) =
            source.or_else(|| self.focused_editor_source(context, tab, fallback))
        else {
            return;
        };
        if !self
            .store
            .views()
            .get(owner)
            .is_some_and(|owner| owner.key.tab == *tab)
        {
            return;
        }
        let snapshot = self.controller.snapshot();
        let Some(project) = snapshot.layouts.iter().find_map(|(project, layout)| {
            crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)
                .into_iter()
                .flat_map(|(root, _)| crate::tabs::tabs_in(root))
                .any(|candidate| candidate.id == *tab)
                .then_some(project.clone())
        }) else {
            return;
        };
        let Some(document) = self.store.views().get(source).map(|source| source.document) else {
            return;
        };
        let configuration =
            crate::presentation_refresh::indent_configuration(&self.services.state.settings.read());
        let Ok(fallback) = self.store.configure_indentation(document, configuration) else {
            return;
        };
        self.reconcile_lsp();
        let providers = self
            .store
            .documents()
            .snapshot(document)
            .ok()
            .and_then(|snapshot| {
                self.lsp
                    .as_ref()
                    .map(|lsp| lsp.formatting_providers(&project, &snapshot, command))
            })
            .unwrap_or_default();
        match self.editor_formatting.begin(
            &self.store,
            project,
            source,
            owner,
            command,
            fallback,
            providers,
        ) {
            Ok(Some(request)) if request.native => context.request_repaint(),
            Ok(Some(request)) => {
                if let Some(lsp) = &self.lsp
                    && let Err(error) = lsp.format_editor(request.clone())
                {
                    self.editor_formatting.reject(&request);
                    self.status = Some(error.to_string());
                }
            }
            Ok(None) | Err(taide_native_editor::document::EditorError::ReadOnly) => (),
            Err(error) => log::warn!("native formatter request refused: {error:?}"),
        }
    }

    fn request_editor_input_formatting(
        &mut self,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        input: taide_native_ui::editor_formatting_input::Input,
    ) -> bool {
        if self.closing.is_some()
            || self.services.state.is_shutting_down()
            || (source != owner && self.editor_locations.preview_owner(source) != Some(owner))
        {
            return false;
        }
        let Some(owner_view) = self.store.views().get(owner) else {
            return false;
        };
        let shell = self.controller.snapshot();
        let valid_owner = shell.layouts.get(&project).is_some_and(|layout| {
            self.active_window_scope(|scope| {
                crate::symbol_sidebar::window_tree(&project, layout, scope).is_some_and(
                    |(root, _)| {
                        crate::tabs::tabs_in(root).iter().any(|tab| {
                            tab.id == owner_view.key.tab
                                && matches!(
                                    tab.kind,
                                    TabKind::File { .. } | TabKind::Untitled { .. }
                                )
                        })
                    },
                )
            })
            .is_some()
        });
        if !valid_owner {
            return false;
        }
        if !self.store.views().get(source).is_some_and(|source| {
            source.document == input.document
                && self
                    .store
                    .documents()
                    .snapshot(input.document)
                    .is_ok_and(|snapshot| snapshot.revision == input.revision)
        }) {
            return false;
        }
        let settings = self.services.state.settings.read();
        let enabled = match input.kind {
            taide_native_ui::editor_formatting_input::Kind::Type { .. } => {
                settings.editor_format_on_type
            }
            taide_native_ui::editor_formatting_input::Kind::Paste { .. } => {
                settings.editor_format_on_paste
            }
        };
        let configuration = crate::presentation_refresh::indent_configuration(&settings);
        drop(settings);
        if !enabled {
            return false;
        }
        let Ok(fallback) = self
            .store
            .configure_indentation(input.document, configuration)
        else {
            return false;
        };
        self.reconcile_lsp();
        let providers = self
            .store
            .documents()
            .snapshot(input.document)
            .ok()
            .and_then(|snapshot| {
                self.lsp
                    .as_ref()
                    .map(|lsp| lsp.automatic_formatting_providers(&project, &snapshot, &input.kind))
            })
            .unwrap_or_default();
        match self.editor_formatting.begin_automatic(
            &self.store,
            project,
            source,
            owner,
            &input,
            fallback,
            providers,
        ) {
            Ok(Some(request)) if request.native => true,
            Ok(Some(request)) => {
                if let Some(lsp) = &self.lsp {
                    if lsp.format_editor(request.clone()).is_ok() {
                        return true;
                    }
                    self.editor_formatting.reject(&request);
                }
                false
            }
            Ok(None) | Err(taide_native_editor::document::EditorError::ReadOnly) => false,
            Err(error) => {
                log::warn!("native input formatter request refused: {error:?}");
                false
            }
        }
    }

    pub(crate) fn reconcile_lsp(&mut self) {
        self.editor_clipboard
            .reconcile(&self.store, &self.services.state);
        if self.closing.is_some() || self.services.state.is_shutting_down() {
            self.editor_clipboard.clear();
            self.editor_formatting.clear();
            self.editor_rename.clear();
            self.editor_code_actions.clear();
            return;
        }
        self.lsp_diagnostics.retain_documents(
            self.store
                .documents()
                .snapshots()
                .map(|model| model.id)
                .collect(),
        );
        let scopes = self.window_scopes();
        let Some(lsp) = self.lsp.as_mut() else {
            self.editor_formatting
                .reconcile(&self.store, |_| HashSet::new());
            self.editor_rename.clear();
            self.editor_code_actions.clear();
            self.editor_symbols.retain(&HashSet::new());
            self.editor_folding.retain(&HashSet::new());
            return;
        };
        self.editor_formatting.reconcile(&self.store, |request| {
            if !request.enabled(&self.services.state.settings.read()) {
                return HashSet::new();
            }
            lsp.formatting_request_providers(request)
        });
        let rename_shell = self.controller.snapshot();
        self.editor_rename.reconcile(&self.store, lsp, |request| {
            self.store.views().get(request.owner).is_some_and(|owner| {
                rename_shell
                    .layouts
                    .get(&request.project)
                    .is_some_and(|layout| {
                        taide_layout::service::all_roots(layout).any(|root| {
                            taide_native_ui::snapshot::active_tab(root, &owner.key.pane)
                                .is_some_and(|tab| tab.id == owner.key.tab)
                        })
                    })
            })
        });
        self.editor_code_actions
            .reconcile(&self.store, lsp, |request| {
                self.store.views().get(request.owner).is_some_and(|owner| {
                    rename_shell
                        .layouts
                        .get(&request.project)
                        .is_some_and(|layout| {
                            scopes.iter().any(|scope| {
                                crate::symbol_sidebar::window_tree(&request.project, layout, scope)
                                    .is_some_and(|(root, _)| {
                                        taide_native_ui::snapshot::active_tab(root, &owner.key.pane)
                                            .is_some_and(|tab| tab.id == owner.key.tab)
                                    })
                            })
                        })
                }) && (request.source == request.owner
                    || self.editor_locations.preview_owner(request.source) == Some(request.owner))
            });
        if let Err(error) = lsp.reconcile_models(&mut self.store) {
            self.status = Some(error.to_string());
            return;
        }
        let shell = self.controller.snapshot();
        if let Err(error) = lsp.retain_projects(
            &shell
                .projects
                .iter()
                .map(|project| project.id.clone())
                .collect(),
        ) {
            self.status = Some(error.to_string());
            return;
        }
        let mut retained = HashSet::new();
        let mut fold_retained = HashSet::new();
        let visible: Vec<_> = scopes
            .iter()
            .flat_map(|scope| crate::lsp::visible_files(&shell, scope))
            .collect();
        for (project, path) in visible {
            if self.loading.contains(path)
                || self.failed.contains_key(path)
                || self.missing_drafts.contains_key(path)
            {
                continue;
            }
            let Some(file) = self.files.get(path) else {
                continue;
            };
            if !retained.insert((project.clone(), file.id)) {
                continue;
            }
            if let Ok(snapshot) = self.store.documents().snapshot(file.id) {
                if let Err(error) = lsp.sync(project.clone(), snapshot.clone()) {
                    self.status = Some(error.to_string());
                    continue;
                }
                if let Some(request) = self.editor_symbols.observe(
                    project,
                    snapshot.clone(),
                    lsp.symbol_providers(project, file.id),
                    Instant::now(),
                ) && let Err(error) = lsp.document_symbols(request.clone())
                {
                    self.editor_symbols.failed(&request);
                    self.status = Some(error.to_string());
                }
                if taide_native_ui::presentation::editor_folding(snapshot.metadata.tier) {
                    fold_retained.insert((project.clone(), file.id));
                    if let Some(request) = self.editor_folding.observe(
                        project,
                        snapshot,
                        lsp.symbol_providers(project, file.id),
                        Instant::now(),
                    ) && let Err(error) = lsp.syntax_folding(request.clone())
                    {
                        self.editor_folding.failed(&request);
                        self.status = Some(error.to_string());
                    }
                }
            }
        }
        for (project, document) in self.editor_locations.preview_bindings(&self.store) {
            if retained.contains(&(project.clone(), document))
                || !shell.projects.iter().any(|entry| entry.id == project)
            {
                continue;
            }
            let Ok(snapshot) = self.store.documents().snapshot(document) else {
                continue;
            };
            let taide_native_editor::document::DocumentKey::File(path) = &snapshot.key else {
                continue;
            };
            let within_root = taide_infra::root_guard::project_root(
                &self.services.state.projects.read(),
                &project,
            )
            .and_then(|root| taide_infra::root_guard::ensure_within_root(&root, path))
            .is_ok_and(|canonical| canonical == *path);
            if !within_root {
                continue;
            }
            if let Err(error) = lsp.sync(project.clone(), snapshot) {
                self.status = Some(error.to_string());
                continue;
            }
            retained.insert((project, document));
        }
        if let Err(error) = lsp.retain_bindings(&retained) {
            self.status = Some(error.to_string());
        }
        self.editor_symbols.retain(&retained);
        self.editor_folding.retain(&fold_retained);
    }

    pub(crate) fn clear_builtin_json(&mut self) {
        self.editor_json.clear();
        self.editor_json_validation.clear();
        self.shell_state.welcome.clear();
        self.lsp_diagnostics
            .retain_builtin(&self.store, HashSet::new());
        self.problem_opens.clear();
        self.editor_symbols.retain_builtin(&HashSet::new());
        self.editor_folding.retain_builtin(&HashSet::new());
    }

    pub(crate) fn reconcile_builtin_json(&mut self, context: &egui::Context) {
        if self.closing.is_some() || self.is_exit_ready || self.services.state.is_shutting_down() {
            self.clear_builtin_json();
            return;
        }
        for open in std::mem::take(&mut self.problem_opens) {
            let Ok(snapshot) = self.store.documents().snapshot(open.document) else {
                continue;
            };
            if snapshot.key != open.key || snapshot.revision != open.revision {
                continue;
            }
            let taide_native_editor::document::DocumentKey::Untitled(tab) = &open.key else {
                continue;
            };
            let destination =
                self.controller
                    .snapshot()
                    .layouts
                    .iter()
                    .find_map(|(project, layout)| {
                        let (root, _) =
                            crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)?;
                        let (pane, _) = taide_layout::service::find_tab(root, tab)?;
                        let actual = crate::tabs::tabs_in(root)
                            .into_iter()
                            .find(|entry| entry.id == *tab)?;
                        matches!(actual.kind, TabKind::Untitled { .. })
                            .then(|| (project.clone(), pane.clone()))
                    });
            let Some((project, pane)) = destination else {
                continue;
            };
            let key = ViewKey {
                window: WINDOW_LABEL.into(),
                pane,
                tab: tab.clone(),
            };
            if let Err(error) = self.store.attach_view(key.clone(), snapshot.id) {
                self.report(&editor_error(error));
                continue;
            }
            let position = taide_native_editor::lsp::Position::new(
                u32::try_from(open.line.saturating_sub(1)).unwrap_or(u32::MAX),
                u32::try_from(open.column.saturating_sub(1)).unwrap_or(u32::MAX),
            );
            let result =
                taide_native_editor::lsp::position_to_byte(&snapshot, position).and_then(|byte| {
                    crate::editor_reveal::reveal_document(
                        &mut self.store,
                        &key,
                        &snapshot,
                        byte..byte,
                    )
                });
            match result {
                Ok(Some(view)) => {
                    if matches!(self.shell.scope, WindowScope::Main)
                        && let Err(error) = self
                            .controller
                            .submit(ShellMutation::ActivateProject(project))
                    {
                        self.report(&error);
                        continue;
                    }
                    for command in [
                        ShellMutation::FocusPane(key.pane.clone()),
                        ShellMutation::ActivateTab(key.tab.clone()),
                    ] {
                        if let Err(error) = self.controller.submit(command) {
                            self.report(&error);
                        }
                    }
                    if let Some(body) = crate::editor_context_menu::body(context, view) {
                        context.memory_mut(|memory| memory.request_focus(body));
                    }
                    context.request_repaint();
                }
                Ok(None) => {}
                Err(error) => self.report(&editor_error(error)),
            }
        }
        let diagnostics = self.editor_json_validation.tick(
            &self.store,
            &self.services.tasks,
            context,
            Instant::now(),
        );
        self.lsp_diagnostics
            .retain_builtin(&self.store, self.editor_json_validation.documents());
        for (snapshot, result) in diagnostics {
            match result {
                Ok(markers) => {
                    let title = match &snapshot.key {
                        taide_native_editor::document::DocumentKey::File(path) => path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                        taide_native_editor::document::DocumentKey::Untitled(tab) => self
                            .controller
                            .snapshot()
                            .layouts
                            .values()
                            .flat_map(taide_layout::service::all_roots)
                            .flat_map(crate::tabs::tabs_in)
                            .find(|entry| entry.id == *tab)
                            .map(|entry| entry.title.clone())
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    if let Err(error) = self
                        .lsp_diagnostics
                        .publish_builtin(&snapshot, markers, title)
                    {
                        self.lsp_diagnostics.remove_builtin(snapshot.id);
                        self.report(&editor_error(error));
                    }
                }
                Err(error) => {
                    self.lsp_diagnostics.remove_builtin(snapshot.id);
                    self.report(&editor_error(error));
                }
            }
        }
        let completed = self
            .editor_json
            .tick(&self.store, &self.services.tasks, context);
        let symbol_documents = self.editor_json.symbol_documents();
        self.editor_symbols.retain_builtin(&symbol_documents);
        self.editor_folding
            .retain_builtin(&self.editor_json.documents());
        for (snapshot, result) in completed {
            match result {
                Ok(models) => {
                    if let Some(symbols) = models.symbols
                        && let Err(error) = self.editor_symbols.install_builtin(&snapshot, symbols)
                    {
                        self.editor_symbols.fail_builtin(&snapshot);
                        self.report(&editor_error(error));
                    }
                    self.editor_folding.install_builtin(&snapshot, models.folds);
                }
                Err(error) => {
                    if symbol_documents.contains(&snapshot.id) {
                        self.editor_symbols.fail_builtin(&snapshot);
                    }
                    self.report(&editor_error(error));
                }
            }
        }
    }

    pub(crate) fn dispatch_completion(&mut self, context: &egui::Context) {
        for request in self.editor_completion.take_queued() {
            let active = self
                .services
                .state
                .layouts
                .read()
                .get(&request.project)
                .is_some_and(|layout| {
                    self.active_window_scope(|scope| request.is_active(layout, scope))
                        .is_some()
                });
            if self.closing.is_some() || !active || !request.describes(&self.store) {
                if self.editor_completion.is_current(&request) {
                    self.editor_completion.close(request.source);
                }
                continue;
            }
            let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                lsp.completion_providers(&request.project, &request.snapshot)
            });
            if providers != request.providers {
                self.editor_completion.close(request.source);
                continue;
            }
            let repaint = context.clone();
            self.editor_completion.supply(
                &self.store,
                &request,
                self.settings_state.settings_views.snippet_catalog().files(),
                &self.services.tasks,
                Arc::new(move || repaint.request_repaint()),
            );
            if request.query_providers.is_empty() {
                if let Err(error) = self.editor_completion.accept(
                    &self.store,
                    &request,
                    providers,
                    Ok(crate::editor_completion::Response { groups: Vec::new() }),
                ) {
                    self.status = Some(format!("native editor completion: {error:?}"));
                }
            } else if let Some(lsp) = &self.lsp
                && let Err(error) = lsp.completion(request.clone())
            {
                self.editor_completion.close(request.source);
                self.status = Some(error.to_string());
            }
        }
        for request in self.editor_completion.take_clipboard_requests(&self.store) {
            let active = self
                .services
                .state
                .layouts
                .read()
                .get(&request.project)
                .is_some_and(|layout| {
                    self.active_window_scope(|scope| request.is_active(layout, scope))
                        .is_some()
                });
            if self.closing.is_some()
                || !active
                || !self.submit(HostCommand::Editor(Command::ReadCompletionClipboard(
                    request.clone(),
                )))
            {
                self.editor_completion.close(request.source);
            }
        }
    }

    pub(crate) fn consume_code_action_navigation(
        &mut self,
        finished: crate::editor_code_actions::Finished,
    ) {
        self.consume_editor_command_navigation(
            finished.request.project,
            finished.request.source,
            finished.provider,
            finished.navigation,
        );
    }

    fn consume_editor_command_navigation(
        &mut self,
        project: ProjectId,
        source: taide_native_editor::view::ViewId,
        provider: crate::editor_symbols::ProviderIdentity,
        navigation: crate::editor_code_actions::Navigation,
    ) {
        use taide_lsp::native::protocol::lsp_types;
        use taide_native_editor::symbol_locations::{Kind, Mode};
        let Some(view) = self.store.views().get(source) else {
            return;
        };
        let Ok(snapshot) = self.store.documents().snapshot(view.document) else {
            return;
        };
        let same = crate::editor_locations::file_path(&navigation.uri).is_some_and(|path| {
            snapshot.key == taide_native_editor::document::DocumentKey::File(path)
        });
        let position = if same {
            Some(navigation.position)
        } else {
            None
        };
        let providers = HashSet::from([provider]);
        let Ok(request) = self.editor_locations.begin(
            project,
            &self.store,
            source,
            Kind::References,
            navigation.mode,
            providers.clone(),
            position,
        ) else {
            return;
        };
        let response = crate::editor_locations::Response {
            groups: vec![crate::editor_locations::Group {
                provider: provider,
                targets: navigation.targets,
            }],
        };
        let Ok(Some(model)) =
            self.editor_locations
                .accept(&request, &self.store, providers, Ok(response))
        else {
            return;
        };
        if request.mode == Mode::GoTo {
            self.consume_location_model(request, model);
            return;
        }
        if same {
            self.editor_locations.show(source);
            return;
        }
        let Some((revision, scope)) = self.request_window(&request) else {
            return;
        };
        let Ok((source, token, cancelled)) = self.editor_locations.begin_open(&self.store, source)
        else {
            return;
        };
        let viewport = request.viewport;
        let target =
            taide_native_editor::symbol_locations::Target::from_location(lsp_types::Location {
                uri: navigation.uri,
                range: lsp_types::Range::new(navigation.position, navigation.position),
            });
        self.submit(HostCommand::Editor(Command::OpenSymbolLocation(
            crate::symbol_location_host::Request {
                source,
                token,
                cancelled,
                target,
                side: false,
                keep_peek: true,
                revision,
                scope,
                viewport,
            },
        )));
    }

    fn request_window(
        &self,
        request: &crate::editor_locations::Request,
    ) -> Option<(u32, WindowScope)> {
        let layouts = self.services.state.layouts.read();
        let layout = layouts.get(&request.project)?;
        let scope = self
            .active_window_scope(|scope| request.is_active(layout, scope))
            .unwrap_or_else(|| self.shell.scope.clone());
        Some((layout.revision, scope))
    }

    fn consume_location_model(
        &mut self,
        request: crate::editor_locations::Request,
        model: Arc<taide_native_editor::symbol_locations::Locations>,
    ) {
        use taide_native_editor::symbol_locations::{Kind, Mode};
        if request.mode == Mode::Hover {
            return;
        }
        if request.source == request.owner
            && (request.mode == Mode::Peek || model.targets().len() > 1)
        {
            self.editor_locations.show(request.source);
            return;
        }
        let Some(target) = model
            .first()
            .and_then(|index| model.targets().get(index))
            .cloned()
        else {
            self.status = Some(format!("No {} found", request.kind.title().to_lowercase()));
            self.editor_locations.close(request.source);
            return;
        };
        let same = crate::editor_locations::file_path(&target.uri).is_some_and(|path| {
            request.snapshot.key == taide_native_editor::document::DocumentKey::File(path)
        });
        if same
            && target.selection.start <= request.position
            && request.position <= target.selection.end
            && matches!(
                request.kind,
                Kind::Definition | Kind::Declaration | Kind::TypeDefinition
            )
        {
            if let Some(lsp) = self.lsp.as_ref() {
                let providers =
                    lsp.location_providers(&request.project, &request.snapshot, Kind::References);
                if !providers.is_empty()
                    && let Ok(next) = self.editor_locations.begin(
                        request.project.clone(),
                        &self.store,
                        request.source,
                        Kind::References,
                        request.mode,
                        providers,
                        Some(request.position),
                    )
                {
                    if let Err(error) = lsp.symbol_locations(next) {
                        self.status = Some(error.to_string());
                    }
                    return;
                }
            }
        }
        if same && request.mode != Mode::Aside {
            if let Err(error) = self.editor_locations.navigate(
                &mut self.store,
                request.source,
                &target,
                Instant::now(),
            ) {
                self.status = Some(editor_error(error).to_string());
            }
            self.editor_locations.close(request.source);
            return;
        }
        if let Some((revision, scope)) = self.request_window(&request) {
            let viewport = request.viewport;
            let side = request.mode == Mode::Aside;
            if let Ok((source, token, cancelled)) = self
                .editor_locations
                .begin_open(&self.store, request.source)
            {
                self.submit(HostCommand::Editor(Command::OpenSymbolLocation(
                    crate::symbol_location_host::Request {
                        source,
                        token,
                        cancelled,
                        target,
                        side,
                        keep_peek: false,
                        revision,
                        scope,
                        viewport,
                    },
                )));
            }
        }
    }

    fn is_current_code_lens_request(&self, request: &crate::editor_code_lenses::Request) -> bool {
        self.closing.is_none()
            && self.services.state.settings.read().editor_code_lens_enabled
            && request.describes(&self.store)
            && (request.source == request.owner
                || self
                    .editor_locations
                    .preview_views()
                    .contains(&request.source))
            && self
                .services
                .state
                .layouts
                .read()
                .get(&request.project)
                .is_some_and(|layout| {
                    self.active_window_scope(|scope| request.is_active(layout, scope))
                        .is_some()
                })
    }

    fn is_current_selection_range_request(
        &self,
        request: &crate::editor_selection_ranges::Request,
    ) -> bool {
        self.closing.is_none()
            && request.describes(&self.store)
            && (request.source == request.owner
                || self
                    .editor_locations
                    .preview_views()
                    .contains(&request.source))
            && self
                .services
                .state
                .layouts
                .read()
                .iter()
                .any(|(project, layout)| {
                    self.active_window_scope(|scope| request.is_active(project, layout, scope))
                        .is_some()
                })
    }

    pub(crate) fn poll_lsp(&mut self) {
        if let Some(bindings) = self
            .lsp
            .as_mut()
            .and_then(crate::lsp::LspBridge::diagnostic_bindings)
        {
            self.lsp_diagnostics.reconcile(bindings);
        }
        while let Some(reply) = self.lsp.as_mut().and_then(crate::lsp::LspBridge::poll) {
            match reply {
                reply @ (crate::lsp::Reply::WorkspaceSymbols { .. }
                | crate::lsp::Reply::ExplorerPasted(_)
                | crate::lsp::Reply::ExplorerCreated(_)
                | crate::lsp::Reply::ExplorerRenamed(_)
                | crate::lsp::Reply::ExplorerDeleteFinished { .. }) => {
                    self.apply_explorer_search_lsp_reply(reply)
                }
                crate::lsp::Reply::SelectionRanges { request, result } => {
                    let providers = request
                        .project
                        .as_ref()
                        .zip(self.lsp.as_ref())
                        .map_or_else(HashSet::new, |(project, lsp)| {
                            lsp.selection_range_providers(project, &request.snapshot)
                        });
                    if self.is_current_selection_range_request(&request) {
                        self.editor_selection_ranges.accept(
                            &self.store,
                            &request,
                            providers,
                            result,
                        );
                    }
                }
                crate::lsp::Reply::CodeLenses { request, result } => {
                    let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                        lsp.code_lens_providers(&request.project, &request.snapshot)
                    });
                    if self.is_current_code_lens_request(&request) {
                        self.editor_code_lenses.accept(
                            &self.store,
                            &request,
                            providers,
                            result,
                            Instant::now(),
                        );
                    }
                }
                crate::lsp::Reply::CodeLensResolved { request, result } => {
                    if self.is_current_code_lens_request(&request.request)
                        && self.lsp.as_ref().is_some_and(|lsp| {
                            lsp.code_lens_providers(
                                &request.request.project,
                                &request.request.snapshot,
                            )
                            .contains(&request.provider)
                        })
                    {
                        self.editor_code_lenses
                            .resolved(&self.store, &request, result);
                    }
                }
                crate::lsp::Reply::CodeLensExecuted { request, result } => {
                    if self.is_current_code_lens_request(&request.request)
                        && self.lsp.as_ref().is_some_and(|lsp| {
                            lsp.code_lens_providers(
                                &request.request.project,
                                &request.request.snapshot,
                            )
                            .contains(&request.provider)
                        })
                        && self.editor_code_lenses.executed(&self.store, &request)
                    {
                        match result {
                            Ok(Some(navigation)) => self.consume_editor_command_navigation(
                                request.request.project,
                                request.request.source,
                                request.provider,
                                navigation,
                            ),
                            Ok(None) => {}
                            Err(error) => self.status = Some(error.to_string()),
                        }
                    }
                }
                crate::lsp::Reply::CodeLensRefresh { project, provider } => {
                    self.editor_code_lenses
                        .refresh(&project, provider, Instant::now());
                }
                crate::lsp::Reply::InlayHints { request, result } => {
                    let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                        lsp.inlay_hint_providers(&request.project, &request.snapshot)
                    });
                    let active = (request.source == request.owner
                        || self
                            .editor_locations
                            .preview_views()
                            .contains(&request.source))
                        && self
                            .services
                            .state
                            .layouts
                            .read()
                            .get(&request.project)
                            .is_some_and(|layout| {
                                self.active_window_scope(|scope| request.is_active(layout, scope))
                                    .is_some()
                            });
                    if active && self.closing.is_none() {
                        self.editor_inlay_hints
                            .accept(&self.store, &request, providers, result);
                    } else {
                        self.editor_inlay_hints.reject(&request);
                    }
                }
                crate::lsp::Reply::Highlights { request, result } => {
                    let providers = request
                        .project
                        .as_ref()
                        .zip(self.lsp.as_ref())
                        .map_or_else(HashSet::new, |(project, lsp)| {
                            lsp.highlight_providers(project, &request.snapshot)
                        });
                    let active = request.project.as_ref().is_some_and(|project| {
                        self.services
                            .state
                            .layouts
                            .read()
                            .get(project)
                            .is_some_and(|layout| {
                                self.active_window_scope(|scope| request.is_active(layout, scope))
                                    .is_some()
                            })
                    });
                    if active && self.closing.is_none() {
                        self.editor_highlights
                            .accept(&self.store, &request, providers, result);
                    } else {
                        self.editor_highlights.reject(&request);
                    }
                }
                crate::lsp::Reply::Completion { request, result } => {
                    let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                        lsp.completion_providers(&request.project, &request.snapshot)
                    });
                    let active = self
                        .services
                        .state
                        .layouts
                        .read()
                        .get(&request.project)
                        .is_some_and(|layout| {
                            self.active_window_scope(|scope| request.is_active(layout, scope))
                                .is_some()
                        });
                    if active && self.closing.is_none() {
                        if let Err(error) =
                            self.editor_completion
                                .accept(&self.store, &request, providers, result)
                        {
                            self.status = Some(format!("native editor completion: {error:?}"));
                        }
                    } else if self.editor_completion.is_current(&request) {
                        self.editor_completion.close(request.source);
                    }
                }
                crate::lsp::Reply::Documentation { request, result } => {
                    let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                        lsp.documentation_providers(
                            &request.project,
                            &request.snapshot,
                            request.kind,
                        )
                    });
                    let active = self
                        .services
                        .state
                        .layouts
                        .read()
                        .get(&request.project)
                        .is_some_and(|layout| {
                            self.active_window_scope(|scope| request.is_active(layout, scope))
                                .is_some()
                        });
                    if active && self.closing.is_none() {
                        if let Err(error) = self.editor_documentation.accept(
                            &self.store,
                            &request,
                            providers,
                            result,
                        ) {
                            self.status = Some(format!("native editor documentation: {error:?}"));
                        }
                    } else if self.editor_documentation.is_current(&request) {
                        self.editor_documentation
                            .close(request.source, request.kind);
                    }
                }
                crate::lsp::Reply::SymbolLocations { request, result } => {
                    let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                        lsp.location_providers(&request.project, &request.snapshot, request.kind)
                    });
                    let active = self
                        .services
                        .state
                        .layouts
                        .read()
                        .get(&request.project)
                        .is_some_and(|layout| {
                            self.active_window_scope(|scope| request.is_active(layout, scope))
                                .is_some()
                        });
                    if active && self.closing.is_none() {
                        match self
                            .editor_locations
                            .accept(&request, &self.store, providers, result)
                        {
                            Ok(Some(model)) => self.consume_location_model(request, model),
                            Ok(None) => {}
                            Err(error) => {
                                self.status = Some(format!("native symbol locations: {error:?}"))
                            }
                        }
                    } else {
                        self.editor_locations.close_request(&request);
                    }
                }
                crate::lsp::Reply::SyntaxFolding { request, result } => {
                    if let Ok(current) = self.store.documents().snapshot(request.snapshot.id) {
                        let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                            lsp.symbol_providers(&request.project, current.id)
                        });
                        self.editor_folding
                            .accept(&request, &current, providers, result);
                    }
                }
                crate::lsp::Reply::DocumentSymbols { request, result } => {
                    if let Ok(current) = self.store.documents().snapshot(request.snapshot.id) {
                        let providers = self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                            lsp.symbol_providers(&request.project, current.id)
                        });
                        if let Err(error) = self
                            .editor_symbols
                            .accept(&request, &current, providers, result)
                        {
                            self.status = Some(format!("native document symbols: {error:?}"));
                        }
                    }
                }
                crate::lsp::Reply::RenamePrepare(event) => {
                    let result = if self.closing.is_some()
                        || self.workspace_busy()
                        || self.pending_tab_close.is_some()
                        || self.pending_disk_choice.is_some()
                        || self.saving_missing_draft.is_some()
                        || self.saving_untitled.is_some()
                        || !event.activity.is_active()
                    {
                        Err(AppError::Forbidden(
                            "native workspace editor is busy".into(),
                        ))
                    } else {
                        let paths = self
                            .files
                            .iter()
                            .map(|(path, file)| (path.clone(), file.id))
                            .collect();
                        crate::workspace_rename::prepare_documents(&self.store, &event.from, &paths)
                    };
                    if let Ok(documents) = &result {
                        self.workspace_activity = Some(event.activity);
                        let mut suspended = documents
                            .iter()
                            .map(|document| document.snapshot.id)
                            .collect::<HashSet<_>>();
                        suspended.extend(self.store.documents().snapshots().filter_map(|document| {
                            matches!(&document.key, taide_native_editor::document::DocumentKey::File(path) if path.starts_with(&event.to)).then_some(document.id)
                        }));
                        self.suspend_workspace_documents(suspended);
                    }
                    let _result = event.completion.send(result);
                }
                crate::lsp::Reply::Renamed(event) => {
                    let displaced = event
                        .documents
                        .iter()
                        .filter_map(|document| {
                            self.store
                                .documents()
                                .find(&taide_native_editor::document::DocumentKey::File(
                                    document.canonical.clone(),
                                ))
                                .filter(|target| *target != document.requested.id)
                                .map(|target| (target, document.requested.id))
                        })
                        .collect::<HashMap<_, _>>();
                    let result = crate::workspace_rename::commit_documents(&mut self.store, &event);
                    if let Ok(documents) = &result {
                        let files = std::mem::take(&mut self.files);
                        let mut moved_files = Vec::new();
                        for (path, mut file) in files {
                            if let Some(source) = displaced.get(&file.id) {
                                file.id = *source;
                            }
                            if let Some(next) = crate::workspace_rename::moved_path(
                                std::path::Path::new(&path),
                                &event.from,
                                &event.to,
                            ) {
                                moved_files.push((next.to_string_lossy().into_owned(), file));
                            } else {
                                self.files.insert(path, file);
                            }
                        }
                        self.files.extend(moved_files);
                        self.loading
                            .retain(|path| !std::path::Path::new(path).starts_with(&event.from));
                        self.failed
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.from));
                        self.loading_file_tabs
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.from));
                        self.missing_drafts
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.from));
                        self.missing_drafts.extend(event.survivor_drafts);
                        for document in displaced.keys() {
                            self.persistence.settled(*document);
                            self.last_edited_views.remove(document);
                            self.lsp_diagnostics.remove_document(*document);
                            self.observing_files.remove(document);
                            self.pending_file_observations.remove(document);
                        }
                        for document in documents {
                            self.observing_files.remove(&document.id);
                            self.pending_file_observations.remove(&document.id);
                            self.lsp_diagnostics.remove_document(document.id);
                            for view in self.store.views().for_document(document.id) {
                                self.pending_dirty
                                    .insert(view.key.tab.clone(), document.dirty);
                            }
                        }
                        self.controller.apply_layouts(event.layouts);
                        self.move_explorer_search_entries(&event.from, &event.to);
                        self.reconcile_lsp();
                        if let Some(error) = event.warnings.first() {
                            self.status = Some(error.to_string());
                        }
                    } else if let Err(error) = &result {
                        self.status = Some(error.to_string());
                    }
                    let _result = event.completion.send(result);
                }
                crate::lsp::Reply::DeletePrepare(event) => {
                    let result = if self.closing.is_some()
                        || self.workspace_busy()
                        || self.pending_tab_close.is_some()
                        || self.pending_disk_choice.is_some()
                        || self.saving_missing_draft.is_some()
                        || self.saving_untitled.is_some()
                        || !event.activity.is_active()
                    {
                        Err(AppError::Forbidden(
                            "native workspace editor is busy".into(),
                        ))
                    } else {
                        let additional = self
                            .files
                            .iter()
                            .filter(|(path, _)| std::path::Path::new(path).starts_with(&event.path))
                            .map(|(_, file)| file.id)
                            .collect::<HashSet<_>>();
                        crate::workspace_delete::prepare_documents(
                            &self.store,
                            &event.path,
                            &additional,
                        )
                    };
                    if let Ok(documents) = &result {
                        self.workspace_activity = Some(event.activity);
                        self.suspend_workspace_documents(
                            documents.iter().map(|document| document.id).collect(),
                        );
                    }
                    let _result = event.completion.send(result);
                }
                crate::lsp::Reply::ExplorerDeletePrepare(event)
                | crate::lsp::Reply::ExplorerMovePrepare(event) => {
                    let result = if self.closing.is_some()
                        || self.workspace_busy()
                        || self.pending_tab_close.is_some()
                        || self.pending_disk_choice.is_some()
                        || self.saving_missing_draft.is_some()
                        || self.saving_untitled.is_some()
                        || !event.activity.is_active()
                    {
                        Err(AppError::Forbidden(
                            "native workspace editor is busy".into(),
                        ))
                    } else {
                        let additional = self
                            .files
                            .iter()
                            .filter(|(path, _)| std::path::Path::new(path).starts_with(&event.path))
                            .map(|(_, file)| file.id)
                            .collect();
                        Ok(crate::explorer_delete::prepare_documents(
                            &self.store,
                            &event.path,
                            &additional,
                        ))
                    };
                    if let Ok(documents) = &result {
                        self.workspace_activity = Some(event.activity);
                        self.suspend_workspace_documents(
                            documents.iter().map(|document| document.id).collect(),
                        );
                    }
                    let _result = event.completion.send(result);
                }
                crate::lsp::Reply::ExplorerDeleted(event)
                | crate::lsp::Reply::ExplorerMoved(event) => {
                    self.deleted_documents
                        .extend(event.documents.iter().map(|document| document.id));
                    let retained = self
                        .files
                        .iter()
                        .filter(|(path, _)| !std::path::Path::new(path).starts_with(&event.path))
                        .map(|(_, file)| file.id)
                        .collect();
                    let result = crate::explorer_delete::commit_documents(
                        &mut self.store,
                        &event,
                        &retained,
                    );
                    if let Ok(released) = &result {
                        self.files
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        for document in released {
                            self.deleted_documents.remove(document);
                            self.persistence.settled(*document);
                            self.last_edited_views.remove(document);
                            self.lsp_diagnostics.remove_document(*document);
                            self.observing_files.remove(document);
                            self.pending_file_observations.remove(document);
                        }
                        self.loading
                            .retain(|path| !std::path::Path::new(path).starts_with(&event.path));
                        self.failed
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        self.loading_file_tabs
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        self.missing_drafts
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        self.missing_drafts.extend(event.drafts);
                        let tabs = event
                            .layouts
                            .values()
                            .flat_map(taide_layout::service::all_roots)
                            .flat_map(crate::tabs::tabs_in)
                            .map(|tab| tab.id.clone())
                            .collect::<HashSet<_>>();
                        self.pending_dirty.retain(|tab, _| tabs.contains(tab));
                        self.restore_notices.retain(|tab, _| tabs.contains(tab));
                        self.controller.apply_layouts(event.layouts);
                        self.reconcile_lsp();
                    } else if let Err(error) = &result {
                        self.status = Some(error.to_string());
                    }
                    let _result = event.completion.send(result);
                }
                crate::lsp::Reply::Deleted(event) => {
                    let retained = self
                        .files
                        .iter()
                        .filter(|(path, _)| !std::path::Path::new(path).starts_with(&event.path))
                        .map(|(_, file)| file.id)
                        .collect::<HashSet<_>>();
                    let result = crate::workspace_delete::commit_documents(
                        &mut self.store,
                        &event,
                        &retained,
                    );
                    if let Ok(released) = &result {
                        self.files
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        for document in released {
                            self.persistence.settled(*document);
                            self.last_edited_views.remove(document);
                            self.lsp_diagnostics.remove_document(*document);
                            self.observing_files.remove(document);
                            self.pending_file_observations.remove(document);
                        }
                        self.loading
                            .retain(|path| !std::path::Path::new(path).starts_with(&event.path));
                        self.failed
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        self.loading_file_tabs
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        self.missing_drafts
                            .retain(|path, _| !std::path::Path::new(path).starts_with(&event.path));
                        let tabs = event
                            .layouts
                            .values()
                            .flat_map(taide_layout::service::all_roots)
                            .flat_map(crate::tabs::tabs_in)
                            .map(|tab| tab.id.clone())
                            .collect::<HashSet<_>>();
                        self.pending_dirty.retain(|tab, _| tabs.contains(tab));
                        self.restore_notices.retain(|tab, _| tabs.contains(tab));
                        self.controller.apply_layouts(event.layouts);
                        self.reconcile_lsp();
                    } else if let Err(error) = &result {
                        self.status = Some(error.to_string());
                    }
                    let _result = event.completion.send(result);
                }
                crate::lsp::Reply::DocumentQuery { path, completion } => {
                    let document = self
                        .store
                        .documents()
                        .find(&taide_native_editor::document::DocumentKey::File(path))
                        .and_then(|document| self.store.documents().snapshot(document).ok());
                    let _result = completion.send(document);
                }
                crate::lsp::Reply::WorkspaceEdit(event) => {
                    if self.workspace_busy() {
                        let _result = event.completion.send(false);
                        continue;
                    }
                    let outcome = crate::lsp_workspace::apply_open(&mut self.store, &event);
                    for document in outcome.changed {
                        if let Ok(snapshot) = self.store.documents().snapshot(document) {
                            let mirror = self.draft_project(document).is_some();
                            self.persistence.changed(
                                document,
                                snapshot.revision,
                                Instant::now(),
                                mirror,
                                Duration::ZERO,
                            );
                            for view in self.store.views().for_document(document) {
                                self.pending_dirty
                                    .insert(view.key.tab.clone(), snapshot.dirty);
                            }
                        }
                    }
                    self.reconcile_lsp();
                    let applied = outcome.failure.is_none();
                    if let Some(error) = outcome.failure {
                        self.status = Some(format!("native workspace edit: {error:?}"));
                    }
                    let _result = event.completion.send(applied);
                }
                crate::lsp::Reply::Synced { .. } => {}
                crate::lsp::Reply::Failed { error, .. } => self.status = Some(error.to_string()),
                crate::lsp::Reply::Diagnostics {
                    owner,
                    document,
                    revision,
                    diagnostics,
                } => {
                    if let Some(bindings) = self
                        .lsp
                        .as_mut()
                        .and_then(crate::lsp::LspBridge::diagnostic_bindings)
                    {
                        self.lsp_diagnostics.reconcile(bindings);
                    }
                    if let Ok(snapshot) = self.store.documents().snapshot(document)
                        && snapshot.revision == revision
                    {
                        self.lsp_diagnostics
                            .publish(owner, &snapshot, diagnostics.diagnostics);
                    }
                }
                crate::lsp::Reply::EditorFormatted { request, result } => {
                    let providers = self
                        .lsp
                        .as_ref()
                        .filter(|_| request.enabled(&self.services.state.settings.read()))
                        .map(|lsp| lsp.formatting_request_providers(&request))
                        .unwrap_or_default();
                    match self.editor_formatting.accept(
                        &mut self.store,
                        &request,
                        providers,
                        result,
                    ) {
                        Ok(true) => self.formatted_document(request.snapshot.id),
                        Ok(false) => (),
                        Err(error) => log::warn!("native formatter edit failed: {error:?}"),
                    }
                }
                crate::lsp::Reply::Formatted { snapshot, result } => {
                    self.finish_formatted_save(snapshot, result)
                }
            }
        }
    }

    fn finish_formatted_save(
        &mut self,
        snapshot: taide_native_editor::document::DocumentSnapshot,
        result: Result<
            Option<Vec<taide_lsp::native::protocol::lsp_types::TextEdit>>,
            taide_lsp::native::Failure,
        >,
    ) {
        let document = snapshot.id;
        let Some(mut pending) = self.pending_format_saves.remove(&document) else {
            return;
        };
        if !pending.epoch.is_current() {
            if pending.untitled.is_some() {
                self.saving_untitled = None;
            }
            self.persistence.submission_failed(document);
            return;
        }
        let should_format = self.services.state.settings.read().format_on_save;
        if let Some(options) = pending.native_options.take()
            && should_format
        {
            self.queue_native_save(snapshot, pending, options);
            return;
        }
        match result {
            Ok(Some(edits)) => {
                let view = self.save_view(document);
                match taide_native_editor::formatting::apply_edits(
                    &mut self.store,
                    &snapshot,
                    view,
                    edits,
                ) {
                    Ok(true) => {
                        if let Ok(current) = self.store.documents().snapshot(document) {
                            let mirror = self.draft_project(document).is_some();
                            self.persistence.changed(
                                document,
                                current.revision,
                                Instant::now(),
                                mirror,
                                Duration::ZERO,
                            );
                            for view in self.store.views().for_document(document) {
                                self.pending_dirty.insert(view.key.tab.clone(), true);
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(taide_native_editor::document::EditorError::StaleRevision) => {}
                    Err(error) => {
                        log::warn!("native save formatter edit failed: {error:?}")
                    }
                }
            }
            Ok(None) | Err(taide_lsp::native::Failure::StaleRevision) => {}
            Err(error) => log::warn!("native save formatter failed: {error:?}"),
        }
        self.resumed_save_epochs.insert(document, pending.epoch);
        match self.store.save_snapshot(document) {
            Ok(snapshot) => {
                let command = if let Some(tab) = pending.untitled.clone() {
                    HostCommand::Editor(Command::SaveUntitled {
                        tab,
                        destination: pending.path,
                        snapshot,
                    })
                } else if pending.auto_save {
                    HostCommand::Editor(Command::AutoSave {
                        path: pending.path,
                        snapshot,
                    })
                } else {
                    HostCommand::Editor(Command::Save {
                        path: pending.path,
                        snapshot,
                    })
                };
                if !self.submit(command) {
                    if pending.untitled.is_some() {
                        self.saving_untitled = None;
                    }
                    self.persistence.submission_failed(document);
                    if let Some(close) = self.pending_tab_close.as_mut()
                        && matches!(close.phase, TabClosePhase::Saving(Some(id)) if id == document)
                    {
                        close.phase = TabClosePhase::Confirm;
                    }
                }
            }
            Err(error) => {
                self.resumed_save_epochs.remove(&document);
                if pending.untitled.is_some() {
                    self.saving_untitled = None;
                }
                self.persistence.submission_failed(document);
                self.report_save_failure(error, pending.auto_save);
                if let Some(close) = self.pending_tab_close.as_mut()
                    && matches!(close.phase, TabClosePhase::Saving(Some(id)) if id == document)
                {
                    close.phase = TabClosePhase::Confirm;
                }
            }
        }
    }

    pub(crate) fn workspace_busy(&self) -> bool {
        self.workspace_activity
            .as_ref()
            .is_some_and(crate::workspace_activity::Activity::is_active)
    }

    fn suspend_workspace_documents(&mut self, documents: HashSet<DocumentId>) {
        for document in documents {
            self.workspace_suspended.insert(document);
            self.persistence.settled(document);
            if let Some(pending) = self.pending_format_saves.remove(&document) {
                pending.epoch.invalidate();
            }
            if let Some(epoch) = self.resumed_save_epochs.remove(&document) {
                epoch.invalidate();
            }
        }
    }

    pub(crate) fn run_editor_context_menu(
        &mut self,
        request: crate::editor_context_menu::Request,
    ) -> AppResult<bool> {
        #[cfg(test)]
        {
            let trace = (
                request.invocation.action.id().to_owned(),
                request.matches(&self.store),
                request.is_active(&self.services.state),
            );
            request.context.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("native-editor-context-menu-invocation"),
                    trace,
                )
            });
        }
        if self.closing.is_some()
            || self.services.state.is_shutting_down()
            || !request.matches(&self.store)
            || !request.is_active(&self.services.state)
        {
            return Ok(false);
        }
        use taide_native_ui::editor_context_menu::Action;
        let source = request.invocation.session.source;
        let owner = request.invocation.session.owner;
        let view =
            self.store.views().get(source).ok_or_else(|| {
                editor_error(taide_native_editor::document::EditorError::NotFound)
            })?;
        let snapshot = self
            .store
            .documents()
            .snapshot(view.document)
            .map_err(editor_error)?;
        match request.invocation.action {
            action @ (Action::Copy | Action::Cut | Action::Paste) => {
                use crate::editor_context_menu::ClipboardOperation;
                let operation = if action == Action::Paste {
                    ClipboardOperation::Paste
                } else {
                    let Some(text) = taide_native_editor::editing::clipboard_text(
                        &self.store,
                        source,
                        request.context.os() == egui::os::OperatingSystem::Windows,
                    )
                    .map_err(editor_error)?
                    else {
                        return Ok(false);
                    };
                    if action == Action::Cut {
                        ClipboardOperation::Cut(text)
                    } else {
                        ClipboardOperation::Copy(text)
                    }
                };
                let clipboard = self.editor_clipboard.begin(request, operation);
                let submitted = self.submit(HostCommand::Editor(Command::EditorClipboard(
                    clipboard.clone(),
                )));
                if !submitted {
                    self.editor_clipboard.finish(&clipboard, &self.store);
                }
                Ok(submitted)
            }
            Action::ToggleMinimap => Ok(self.submit(HostCommand::Settings(
                crate::domains::settings::Command::ToggleEditorMinimap,
            ))),
            Action::RunSelectedText => {
                let selection = view.selection.selections[view.selection.primary];
                let text = if selection.anchor == selection.head {
                    snapshot
                        .rope
                        .line(snapshot.rope.byte_to_line(selection.head))
                        .to_string()
                        .trim_end_matches(['\r', '\n'])
                        .to_owned()
                } else {
                    snapshot
                        .rope
                        .byte_slice(
                            selection.anchor.min(selection.head)
                                ..selection.anchor.max(selection.head),
                        )
                        .to_string()
                };
                let title = presentation::message(&self.locale, "terminal.title", &[]);
                Ok(self.submit(HostCommand::Editor(Command::RunEditorText(
                    crate::editor_context_menu::TerminalRequest {
                        target: request,
                        payload: format!("{text}\n"),
                        title,
                    },
                ))))
            }
            Action::Command(id) => {
                use crate::command_registry::{DocumentEdit, Execution, Run};
                let execution = crate::command_registry::registry()?
                    .command(&format!("monaco.{id}"))
                    .map(|command| command.execution);
                match execution {
                    Some(Execution::Native(Run::RenameEditor)) => {
                        if let Some(project) = request.project {
                            self.request_editor_rename_from_source(project, source, owner);
                            return Ok(true);
                        }
                    }
                    Some(Execution::Native(Run::CodeActions(command))) => {
                        if let Some(project) = request.project {
                            self.request_editor_code_actions_from_source(
                                project,
                                source,
                                owner,
                                crate::editor_code_actions::Invocation {
                                    command,
                                    automatic: false,
                                },
                            );
                            return Ok(true);
                        }
                    }
                    Some(Execution::Native(Run::FormatEditor(command))) => {
                        self.request_editor_formatting(
                            &request.context,
                            &request.owner_key.tab,
                            Some(snapshot.id),
                            command,
                            Some((source, owner)),
                        );
                        return Ok(true);
                    }
                    Some(Execution::Native(Run::EditDocument(DocumentEdit::Cursor(command)))) => {
                        let context = taide_native_editor::line_commands::LineCommandContext {
                            indent: self.editor.indent_options(&snapshot),
                            language: None,
                            syntax: &taide_native_editor::language_configuration::UntokenizedLines,
                            compare: None,
                            transforms: None,
                            word_rules: crate::editor_syntax::language_rules(
                                &snapshot.metadata.language_id,
                            )
                            .or_else(|| crate::editor_syntax::language_rules("plaintext")),
                        };
                        taide_native_editor::cursor_commands::run_cursor_command(
                            &mut self.store,
                            source,
                            command,
                            context,
                        )
                        .map_err(editor_error)?;
                        request.context.request_repaint();
                        return Ok(true);
                    }
                    Some(Execution::Native(Run::EditDocument(DocumentEdit::Location(
                        taide_native_editor::symbol_locations::Command::Request { kind, mode },
                    )))) => {
                        let Some((project, lsp)) = request.project.zip(self.lsp.as_ref()) else {
                            return Ok(false);
                        };
                        let providers = lsp.location_providers(&project, &snapshot, kind);
                        if providers.is_empty() {
                            return Ok(false);
                        }
                        if mode == taide_native_editor::symbol_locations::Mode::Peek
                            && self
                                .editor_locations
                                .current(source)
                                .is_some_and(|session| {
                                    session.shown
                                        && session.request.kind == kind
                                        && session.request.selection == view.selection
                                })
                        {
                            self.editor_locations.close(source);
                            return Ok(true);
                        }
                        let mut location = self
                            .editor_locations
                            .begin(project, &self.store, source, kind, mode, providers, None)
                            .map_err(editor_error)?;
                        self.editor_locations
                            .set_viewport(&mut location, request.invocation.session.viewport);
                        if lsp.symbol_locations(location.clone()).is_err() {
                            self.editor_locations.close_request(&location);
                            return Ok(false);
                        }
                        return Ok(true);
                    }
                    _ => {
                        return Err(AppError::Internal(format!(
                            "native editor menu command is unavailable: {id}"
                        )));
                    }
                }
                Ok(false)
            }
        }
    }

    fn apply_editor_menu_clipboard(
        &mut self,
        request: &crate::editor_context_menu::ClipboardRequest,
        text: Option<String>,
    ) -> AppResult<()> {
        use crate::editor_context_menu::ClipboardOperation;
        let source = request.target.invocation.session.source;
        let owner = request.target.invocation.session.owner;
        let view = self
            .store
            .views()
            .get(source)
            .ok_or_else(|| editor_error(taide_native_editor::document::EditorError::NotFound))?
            .clone();
        let snapshot = self
            .store
            .documents()
            .snapshot(view.document)
            .map_err(editor_error)?;
        let mut formatting = None;
        let changed = match &request.operation {
            ClipboardOperation::Copy(copied) => {
                taide_native_ui::editor_surface::remember_clipboard(
                    &request.target.context,
                    copied.clone(),
                );
                false
            }
            ClipboardOperation::Cut(copied) => {
                let changed = taide_native_editor::editing::cut(&mut self.store, source)
                    .map_err(editor_error)?;
                taide_native_ui::editor_surface::remember_clipboard(
                    &request.target.context,
                    copied.clone(),
                );
                changed
            }
            ClipboardOperation::Paste => {
                let text = text.ok_or_else(|| {
                    AppError::Internal("native editor paste response has no text".into())
                })?;
                let selection = view.selection.selections[view.selection.primary];
                let start = taide_native_editor::lsp::byte_to_position(
                    &snapshot,
                    selection.anchor.min(selection.head),
                )
                .map_err(editor_error)?;
                let clipboard = taide_native_ui::editor_surface::clipboard_source(
                    &request.target.context,
                    &text,
                );
                self.store
                    .set_composition(source, None)
                    .map_err(editor_error)?;
                let changed = taide_native_editor::editing::paste(
                    &mut self.store,
                    source,
                    &text,
                    clipboard.as_ref(),
                )
                .map_err(editor_error)?;
                if changed {
                    formatting = taide_native_ui::editor_formatting_input::Input::pasted(
                        &self.store,
                        source,
                        start,
                    )
                    .map_err(editor_error)?;
                }
                changed
            }
        };
        if changed {
            self.editor_menu_changes.insert(snapshot.id, source);
            let current = self
                .store
                .documents()
                .snapshot(snapshot.id)
                .map_err(editor_error)?;
            let tabs: Vec<_> =
                if let taide_native_editor::document::DocumentKey::AppFile(target) = current.key {
                    self.settings_state.app_file_views.changed(snapshot.id);
                    crate::app_file_views::target_tabs(&self.services.state, target)
                } else {
                    self.store
                        .views()
                        .for_document(snapshot.id)
                        .map(|view| view.key.tab.clone())
                        .collect()
                };
            for tab in tabs {
                self.pending_dirty.insert(tab, current.dirty);
            }
            if let Some(project) = &request.target.project {
                if let Some(input) = formatting {
                    self.submit(HostCommand::Editor(Command::FormatEditorInput {
                        project: project.clone(),
                        source,
                        owner,
                        input,
                    }));
                }
            }
        }
        request.target.context.request_repaint();
        Ok(())
    }

    pub(crate) fn flush_dirty(&mut self) -> bool {
        let queued = std::mem::take(&mut self.editor_terminal_writes);
        for delivery in queued {
            if self.closing.is_some() {
                continue;
            }
            let Some(id) = delivery.session(&self.services) else {
                continue;
            };
            if self.has_terminal_agent_session(&id) {
                self.submit(HostCommand::Editor(Command::WriteEditorText(delivery)));
            } else {
                self.editor_terminal_writes.push(delivery);
            }
        }
        let pending: Vec<_> = self
            .pending_dirty
            .iter()
            .map(|(tab, dirty)| (tab.clone(), *dirty))
            .collect();
        for (tab, dirty) in pending {
            if !self.has_layout_tab(&tab) {
                self.pending_dirty.remove(&tab);
                continue;
            }
            let result = self
                .bridge
                .as_ref()
                .ok_or_else(|| AppError::Forbidden("native host is closing".into()))
                .and_then(|bridge| {
                    bridge.submit(HostCommand::Editor(Command::SetDirty {
                        tab: tab.clone(),
                        dirty,
                    }))
                });
            if let Err(error) = result {
                self.report_once(&error);
                return false;
            }
            self.pending_dirty.remove(&tab);
        }
        true
    }

    pub(crate) fn has_layout_tab(&self, tab: &TabId) -> bool {
        self.services.state.layouts.read().values().any(|layout| {
            taide_layout::service::all_roots(layout)
                .any(|root| taide_layout::service::find_tab(root, tab).is_some())
        })
    }

    pub(crate) fn capture_view_states(&mut self) -> crate::editor_view_state_sync::Captures {
        self.editor_view_state
            .capture(
                &self.store,
                &self.services.state.layouts.read(),
                WINDOW_LABEL,
            )
            .map(Arc::from)
            .map_err(editor_error)
    }

    pub(crate) fn fulfill_view_states(&mut self) {
        if self.view_state_flush.has_pending() {
            let captured = self.capture_view_states();
            self.view_state_flush.publish(captured);
        }
    }

    pub(crate) fn request_tab_close(&mut self, tab: TabId) {
        if self.pending_tab_close.is_some() {
            return;
        }
        let snapshot = self.controller.snapshot();
        let candidate = snapshot
            .layouts
            .values()
            .flat_map(|layout| {
                crate::tabs::tabs_in(&layout.root).into_iter().chain(
                    layout
                        .auxiliary_windows
                        .iter()
                        .flat_map(|window| crate::tabs::tabs_in(&window.root)),
                )
            })
            .find(|candidate| candidate.id == tab)
            .cloned();
        let Some(tab) = candidate else {
            return;
        };
        if tab.pinned {
            crate::toast::pinned_close_blocked(
                &mut self.toasts,
                &self.locale,
                &tab.title,
                Instant::now(),
            );
            return;
        }
        let dirty = self.is_dirty_close_tab(&tab);
        self.pending_tab_close = Some(PendingTabClose {
            tab,
            approved_discard: None,
            batch: None,
            automatic_choice: None,
            phase: if dirty {
                TabClosePhase::Confirm
            } else {
                TabClosePhase::Ready { discard: false }
            },
        });
    }

    fn is_dirty_close_tab(&self, tab: &Tab) -> bool {
        if let TabKind::File { path } = &tab.kind {
            self.missing_drafts.contains_key(path)
                || self
                    .files
                    .get(path)
                    .and_then(|file| self.store.documents().snapshot(file.id).ok())
                    .map(|doc| doc.dirty)
                    .unwrap_or(tab.dirty)
        } else if matches!(tab.kind, TabKind::Untitled { .. }) {
            self.untitled
                .get(&tab.id)
                .and_then(|document| self.store.documents().snapshot(document.id).ok())
                .map(|snapshot| snapshot.dirty)
                .unwrap_or(tab.dirty)
        } else {
            false
        }
    }

    pub(crate) fn request_close_tabs(&mut self, ids: Vec<TabId>) {
        if self.pending_tab_close.is_some() {
            return;
        }
        let snapshot = self.controller.snapshot();
        let tabs = ids
            .iter()
            .filter_map(|id| {
                snapshot
                    .layouts
                    .values()
                    .flat_map(taide_layout::service::all_roots)
                    .flat_map(crate::tabs::tabs_in)
                    .find(|tab| &tab.id == id)
            })
            .map(|tab| {
                let mut tab = tab.clone();
                tab.dirty = self.is_dirty_close_tab(&tab);
                tab
            })
            .collect();
        if let Some(batch) = crate::tab_close_batch::Batch::new(tabs) {
            self.advance_close_batch(batch);
        }
    }

    fn advance_close_batch(&mut self, mut batch: crate::tab_close_batch::Batch) {
        use crate::tab_close_batch::Task;
        let Some(task) = batch.next() else {
            if let Some(failure) = batch.failure {
                self.toasts.error(failure, Instant::now());
            }
            return;
        };
        let (tab, phase, approved_discard, automatic_choice) = match task {
            Task::Confirm(tab) => (tab, TabClosePhase::Confirm, None, None),
            Task::Prepare(tab, choice) => (tab, TabClosePhase::Confirm, None, Some(choice)),
            Task::Close(prepared) => (
                prepared.tab,
                TabClosePhase::Ready {
                    discard: prepared.discard,
                },
                prepared.approved,
                None,
            ),
        };
        self.pending_tab_close = Some(PendingTabClose {
            tab,
            phase,
            approved_discard,
            automatic_choice,
            batch: Some(batch),
        });
    }

    fn cancel_failed_batch_prepare(&mut self) {
        if self
            .pending_tab_close
            .as_ref()
            .is_some_and(PendingTabClose::failed_batch_prepare)
        {
            self.pending_tab_close = None;
        }
    }

    pub(crate) fn request_tab_save(
        &mut self,
        tab: TabId,
        document: Option<DocumentId>,
        frame: &eframe::Frame,
    ) {
        if self.save_settings_app_file(&tab, document) {
            return;
        }
        let snapshot = self.controller.snapshot();
        let tab = snapshot
            .layouts
            .values()
            .flat_map(|layout| crate::tabs::tabs_in(&layout.root))
            .find(|candidate| candidate.id == tab);
        let Some(tab) = tab else {
            return;
        };
        if let TabKind::File { path } = &tab.kind
            && document.is_some_and(|id| self.files.get(path).is_none_or(|file| file.id != id))
        {
            return;
        }
        match crate::save::keymap_request(tab, document, &mut self.store) {
            Ok(Some(crate::save::KeymapSave::File { path, snapshot })) => {
                self.submit(HostCommand::Editor(Command::Save { path, snapshot }));
            }
            Ok(Some(crate::save::KeymapSave::Untitled(tab))) => self.save_untitled(&tab, frame),
            Ok(None) => (),
            Err(error) => self.report_save_failure(error, false),
        }
    }

    fn release_tab(
        &mut self,
        closed: crate::tabs::ClosedNativeTab,
        approved: Option<(DocumentId, u64)>,
    ) -> AppResult<()> {
        if matches!(closed.tab.kind, TabKind::AppFile { .. }) {
            return self.release_settings_app_file(&closed.tab);
        }
        if matches!(closed.tab.kind, TabKind::Untitled { .. }) {
            if let Some(document) = self.untitled.get(&closed.tab.id) {
                let id = document.id;
                let views = self
                    .store
                    .views()
                    .for_document(id)
                    .map(|view| view.id)
                    .collect::<Vec<_>>();
                for view in views {
                    self.store.detach_view(view).map_err(editor_error)?;
                }
                if closed.discarded {
                    let expected = approved
                        .filter(|(document, _)| *document == id)
                        .map(|(_, revision)| revision)
                        .unwrap_or(0);
                    self.store
                        .discard_document(id, expected)
                        .map_err(editor_error)?;
                } else {
                    self.store.release_document(id).map_err(editor_error)?;
                }
                self.untitled.remove(&closed.tab.id);
                self.persistence.settled(id);
                self.last_edited_views.remove(&id);
            }
            self.untitled_failed.remove(&closed.tab.id);
            self.untitled_loading.remove(&closed.tab.id);
            return Ok(());
        }
        let TabKind::File { path } = &closed.tab.kind else {
            return Ok(());
        };
        self.release_preview_tab(&closed, path);
        if !closed.has_remaining_file {
            let source = self
                .missing_drafts
                .get(path)
                .map(|draft| draft.canonical.clone());
            if let Some(source) = source {
                self.missing_drafts
                    .retain(|_, draft| draft.canonical != source);
            }
        }
        let Some(file) = self.files.get(path) else {
            if !closed.has_remaining_file {
                self.failed.remove(path);
                self.refused.remove(path);
            }
            return Ok(());
        };
        let document = file.id;
        let views: Vec<_> = self
            .store
            .views()
            .for_document(document)
            .filter(|view| view.key.tab == closed.tab.id)
            .map(|view| view.id)
            .collect();
        for view in views {
            self.store.detach_view(view).map_err(editor_error)?;
        }
        if closed.has_remaining_file {
            return Ok(());
        }
        if closed.discarded {
            let revision = match approved {
                Some((id, revision)) if id == document => revision,
                Some(_) => {
                    return Err(editor_error(
                        taide_native_editor::document::EditorError::InvalidIdentity,
                    ));
                }
                None => 0,
            };
            self.store
                .discard_document(document, revision)
                .map_err(editor_error)?;
        } else {
            self.store
                .release_document(document)
                .map_err(editor_error)?;
        }
        self.files.retain(|_, file| file.id != document);
        self.persistence.settled(document);
        self.last_edited_views.remove(&document);
        self.failed.remove(path);
        Ok(())
    }

    pub(crate) fn tab_close_dialog(&mut self, context: &egui::Context, frame: &eframe::Frame) {
        self.cancel_failed_batch_prepare();
        let Some(pending) = self.pending_tab_close.as_ref() else {
            return;
        };
        let tab = pending.tab.clone();
        let busy = !matches!(pending.phase, TabClosePhase::Confirm);
        let automatic_choice = pending.automatic_choice;
        let choice = if automatic_choice.is_some() {
            automatic_choice
        } else if matches!(
            pending.phase,
            TabClosePhase::Confirm | TabClosePhase::Saving(_)
        ) {
            let titles = pending
                .batch
                .as_ref()
                .map(|batch| batch.titles().to_vec())
                .unwrap_or_else(|| vec![tab.title.clone()]);
            crate::close_dialog::show_titles(context, &self.locale, &titles, busy)
        } else {
            None
        };
        if let Some(pending) = self.pending_tab_close.as_mut() {
            pending.automatic_choice = None;
        }
        if matches!(
            choice,
            Some(
                crate::close_dialog::CloseChoice::Save | crate::close_dialog::CloseChoice::Discard
            )
        ) && self
            .pending_tab_close
            .as_ref()
            .and_then(|pending| pending.batch.as_ref())
            .is_some_and(|batch| !batch.is_preparing())
        {
            if let Some(mut batch) = self
                .pending_tab_close
                .take()
                .and_then(|pending| pending.batch)
            {
                batch.choose(choice.unwrap());
                self.advance_close_batch(batch);
            }
            context.request_repaint();
            return;
        }
        match choice {
            Some(crate::close_dialog::CloseChoice::Cancel) => {
                self.pending_tab_close = None;
            }
            Some(crate::close_dialog::CloseChoice::Discard) => {
                if self
                    .pending_tab_close
                    .as_ref()
                    .is_some_and(|pending| pending.batch.is_some())
                {
                    self.pending_dirty.insert(tab.id.clone(), false);
                }
                let approved = if let TabKind::File { path } = &tab.kind {
                    self.files
                        .get(path)
                        .and_then(|file| self.store.documents().snapshot(file.id).ok())
                        .map(|snapshot| (snapshot.id, snapshot.revision))
                } else {
                    self.untitled
                        .get(&tab.id)
                        .and_then(|document| self.store.documents().snapshot(document.id).ok())
                        .map(|snapshot| (snapshot.id, snapshot.revision))
                };
                if let Some(pending) = self.pending_tab_close.as_mut() {
                    pending.phase = TabClosePhase::Ready { discard: true };
                    pending.approved_discard = approved;
                }
            }
            Some(crate::close_dialog::CloseChoice::Save) => {
                if matches!(tab.kind, TabKind::Untitled { .. }) {
                    self.save_untitled(&tab.id, frame);
                    self.cancel_failed_batch_prepare();
                    return;
                }
                if let TabKind::File { path } = &tab.kind
                    && self.missing_drafts.contains_key(path)
                {
                    self.save_missing_draft(&tab.id, frame);
                    self.cancel_failed_batch_prepare();
                    return;
                }
                let result = match &tab.kind {
                    TabKind::File { path } => {
                        if let Some(document) = self.files.get(path) {
                            let id = document.id;
                            self.store
                                .save_snapshot(id)
                                .map_err(crate::toast::save_error)
                                .map(|snapshot| {
                                    (
                                        Some(id),
                                        HostCommand::Editor(Command::Save {
                                            path: path.clone(),
                                            snapshot,
                                        }),
                                    )
                                })
                        } else {
                            Ok((
                                None,
                                HostCommand::Editor(Command::SaveMirroredTab(tab.id.clone())),
                            ))
                        }
                    }
                    _ => Err(AppError::Forbidden(
                        "native untitled Save As is not connected".into(),
                    )),
                };
                match result {
                    Ok((document, command)) => {
                        if self.submit(command)
                            && let Some(pending) = self.pending_tab_close.as_mut()
                        {
                            pending.phase = if document.is_some_and(|id| {
                                self.store
                                    .documents()
                                    .snapshot(id)
                                    .is_ok_and(|snapshot| !snapshot.dirty)
                            }) {
                                TabClosePhase::Ready { discard: false }
                            } else {
                                TabClosePhase::Saving(document)
                            };
                        }
                    }
                    Err(error) => self.report(&error),
                }
            }
            None => {}
        }
        let ready = self.pending_tab_close.as_ref().and_then(|pending| {
            if let TabClosePhase::Ready { discard } = pending.phase {
                return Some((pending.tab.id.clone(), discard));
            }
            None
        });
        if let Some((tab, discard)) = ready {
            if self
                .pending_tab_close
                .as_ref()
                .and_then(|pending| pending.batch.as_ref())
                .is_some_and(|batch| batch.is_preparing())
            {
                if let Some(pending) = self.pending_tab_close.take()
                    && let Some(mut batch) = pending.batch
                {
                    batch.prepared(crate::tab_close_batch::Prepared {
                        tab: pending.tab,
                        discard,
                        approved: pending.approved_discard,
                    });
                    self.advance_close_batch(batch);
                }
                context.request_repaint();
                return;
            }
            if self.flush_dirty()
                && self.submit(HostCommand::Editor(Command::CloseTab { tab, discard }))
            {
                if let Some(pending) = self.pending_tab_close.as_mut() {
                    pending.phase = TabClosePhase::Closing;
                }
            } else {
                context.request_repaint();
            }
        }
    }

    pub(crate) fn save_untitled(&mut self, tab: &TabId, frame: &eframe::Frame) {
        if self.saving_untitled.is_some() || self.saving_missing_draft.is_some() {
            return;
        }
        let Some(document) = self.untitled.get(tab) else {
            self.status = Some("native untitled document is not loaded".into());
            return;
        };
        let document_id = document.id;
        let project = document.project.clone();
        let root = self
            .services
            .state
            .projects
            .read()
            .get(&project)
            .map(|project| project.root.clone());
        let title = self
            .controller
            .snapshot()
            .layouts
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .find(|candidate| &candidate.id == tab)
            .map(|candidate| candidate.title.clone());
        let (Some(root), Some(title)) = (root, title) else {
            return;
        };
        let selected = (self.save_picker)(
            frame,
            crate::file_dialogs::SaveRequest {
                directory: root.into(),
                name: title,
                title: presentation::message(&self.locale, "tab.saveAsTitle", &[]),
            },
        );
        let Some(selected) = selected else {
            return;
        };
        let target = match taide_infra::root_guard::resolve_owning_project(
            &self.services.state.projects.read(),
            &selected,
        ) {
            Ok((_, target)) => target,
            Err(error) => {
                self.toasts.ipc_error(&self.locale, &error, Instant::now());
                return;
            }
        };
        if let Some(existing) = self
            .store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(target))
            && self
                .store
                .documents()
                .snapshot(existing)
                .is_ok_and(|snapshot| snapshot.dirty)
        {
            self.status = Some("save the destination's unsaved draft before replacing it".into());
            return;
        }
        let destination = match selected.into_os_string().into_string() {
            Ok(path) => path,
            Err(_) => {
                self.status = Some("native untitled save destination is not UTF-8".into());
                return;
            }
        };
        if !self.flush_dirty() {
            return;
        }
        let snapshot = match self.store.save_snapshot(document_id) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.report_save_failure(error, false);
                return;
            }
        };
        if self.submit(HostCommand::Editor(Command::SaveUntitled {
            tab: tab.clone(),
            destination,
            snapshot,
        })) {
            self.saving_untitled = Some(tab.clone());
            if let Some(pending) = self
                .pending_tab_close
                .as_mut()
                .filter(|pending| &pending.tab.id == tab)
            {
                pending.phase = TabClosePhase::Saving(Some(document_id));
            }
        }
    }

    pub(crate) fn save_missing_draft(&mut self, tab: &TabId, frame: &eframe::Frame) {
        if self.saving_missing_draft.is_some() {
            return;
        }
        let path = self
            .controller
            .snapshot()
            .layouts
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .find_map(|candidate| {
                if &candidate.id != tab {
                    return None;
                }
                match &candidate.kind {
                    TabKind::File { path } => Some(path.clone()),
                    _ => None,
                }
            });
        let Some(path) = path else {
            return;
        };
        if self.loading.contains(&path) {
            return;
        }
        let Some(draft) = self.missing_drafts.get(&path).cloned() else {
            return;
        };
        let original = std::path::Path::new(&path);
        let Some(parent) = original.parent() else {
            return;
        };
        let Some(name) = original.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        let selected = (self.save_picker)(
            frame,
            crate::file_dialogs::SaveRequest {
                directory: parent.into(),
                name: name.into(),
                title: presentation::message(&self.locale, "tab.saveAsTitle", &[]),
            },
        );
        let Some(selected) = selected else {
            return;
        };
        let target = match taide_infra::root_guard::resolve_owning_project(
            &self.services.state.projects.read(),
            &selected,
        ) {
            Ok((_, target)) => target,
            Err(error) => {
                self.toasts.ipc_error(&self.locale, &error, Instant::now());
                return;
            }
        };
        if let Some(document) = self
            .store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(target))
            && self
                .store
                .documents()
                .snapshot(document)
                .is_ok_and(|snapshot| snapshot.dirty)
        {
            self.status = Some("save the destination's unsaved draft before replacing it".into());
            return;
        }
        let destination = match selected.into_os_string().into_string() {
            Ok(path) => path,
            Err(_) => {
                self.status = Some("native save destination is not UTF-8".into());
                return;
            }
        };
        if !self.flush_dirty() {
            return;
        }
        if self.submit(HostCommand::Editor(Command::SaveMissingDraft {
            tab: tab.clone(),
            draft,
            destination,
        })) {
            self.saving_missing_draft = Some(tab.clone());
            if let Some(pending) = self
                .pending_tab_close
                .as_mut()
                .filter(|pending| &pending.tab.id == tab)
            {
                pending.phase = TabClosePhase::Saving(None);
            }
        }
    }

    pub(crate) fn release_project_documents(&mut self, project: &ProjectId) {
        let live_tabs = self
            .services
            .state
            .layouts
            .read()
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .map(|tab| tab.id.clone())
            .collect::<HashSet<_>>();
        let documents = self
            .files
            .values()
            .filter(|file| file.project.as_ref() == Some(project))
            .map(|file| file.id)
            .chain(
                self.untitled
                    .values()
                    .filter(|draft| &draft.project == project)
                    .map(|draft| draft.id),
            )
            .collect::<HashSet<_>>();
        for document in documents {
            let closed_views = self
                .store
                .views()
                .for_document(document)
                .filter(|view| !live_tabs.contains(&view.key.tab))
                .map(|view| view.id)
                .collect::<Vec<_>>();
            for view in closed_views {
                if let Err(error) = self.store.detach_view(view) {
                    self.report(&editor_error(error));
                }
            }
            if self.store.views().for_document(document).next().is_some() {
                let owners = self
                    .files
                    .iter()
                    .filter(|(_, file)| file.id == document)
                    .map(|(path, _)| {
                        let owner = taide_infra::root_guard::resolve_owning_project(
                            &self.services.state.projects.read(),
                            std::path::Path::new(path),
                        )
                        .ok()
                        .map(|(owner, _)| owner);
                        (path.clone(), owner)
                    })
                    .collect::<Vec<_>>();
                for (path, owner) in owners {
                    if let Some(file) = self.files.get_mut(&path) {
                        file.project = owner;
                    }
                }
                continue;
            }
            let released = match self.store.documents().snapshot(document) {
                Ok(snapshot) if snapshot.dirty => {
                    self.store.discard_document(document, snapshot.revision)
                }
                Ok(_) => self.store.release_document(document),
                Err(_) => Ok(()),
            };
            if let Err(error) = released {
                self.report(&editor_error(error));
                continue;
            }
            self.files.retain(|_, file| file.id != document);
            self.untitled.retain(|_, draft| draft.id != document);
            self.persistence.settled(document);
            self.last_edited_views.remove(&document);
            self.lsp_diagnostics.remove_document(document);
            self.observing_files.remove(&document);
            self.pending_file_observations.remove(&document);
        }
    }

    pub(crate) fn drafts(&self) -> AppResult<Vec<Draft>> {
        let mut seen = HashSet::new();
        let mut drafts = Vec::new();
        for (path, document) in &self.files {
            if !seen.insert(document.id) {
                continue;
            }
            let snapshot = self
                .store
                .documents()
                .snapshot(document.id)
                .map_err(editor_error)?;
            if !snapshot.dirty {
                continue;
            }
            let project = document.project.clone().ok_or_else(|| {
                AppError::Forbidden("save the external native document before closing".into())
            })?;
            drafts.push(Draft {
                project,
                target: DraftTarget::File(path.clone()),
                content: snapshot.rope.to_string(),
            });
        }
        for (tab, document) in &self.untitled {
            let snapshot = self
                .store
                .documents()
                .snapshot(document.id)
                .map_err(editor_error)?;
            if snapshot.dirty {
                drafts.push(Draft {
                    project: document.project.clone(),
                    target: DraftTarget::Untitled(tab.clone()),
                    content: snapshot.rope.to_string(),
                });
            }
        }
        Ok(drafts)
    }

    pub(crate) fn choose_disk(&mut self, tab: &TabId, action: BannerAction) {
        if action == BannerAction::Dismiss {
            self.restore_notices.remove(tab);
            return;
        }
        if self.pending_disk_choice.is_some() || self.pending_tab_close.is_some() {
            return;
        }
        let selected = self
            .controller
            .snapshot()
            .layouts
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .find_map(|candidate| {
                if &candidate.id != tab {
                    return None;
                }
                match &candidate.kind {
                    TabKind::File { path } => Some(path.clone()),
                    _ => None,
                }
            });
        let Some(path) = selected else {
            return;
        };
        let Some(file) = self.files.get(&path) else {
            return;
        };
        let document = file.id;
        let live = match self.store.documents().snapshot(document) {
            Ok(live) => live,
            Err(error) => {
                self.status = Some(editor_error(error).to_string());
                return;
            }
        };
        let choice = match action {
            BannerAction::ViewDisk => DiskChoice::ViewDisk,
            BannerAction::KeepMine => DiskChoice::KeepMine,
            BannerAction::Dismiss => return,
        };
        if !self.flush_dirty() {
            return;
        }
        if self.submit(HostCommand::Editor(Command::ChooseDisk {
            path,
            document,
            revision: live.revision,
            choice,
        })) {
            self.pending_disk_choice = Some(document);
        }
    }

    pub(crate) fn queue_project_observations(&mut self, project: &ProjectId) {
        for file in self
            .files
            .values()
            .filter(|file| file.project.as_ref() == Some(project))
        {
            if let Ok(snapshot) = self.store.documents().snapshot(file.id)
                && let taide_native_editor::document::DocumentKey::File(canonical) = snapshot.key
                && let Some(path) = canonical.to_str()
            {
                self.pending_file_observations.insert(file.id, path.into());
            }
        }
    }

    pub(crate) fn flush_observations(&mut self) {
        if self.closing.is_some()
            || self.is_exit_ready
            || self.services.state.is_shutting_down()
            || self.pending_disk_choice.is_some()
            || self.pending_tab_close.is_some()
            || self.saving_untitled.is_some()
            || self.saving_missing_draft.is_some()
        {
            return;
        }
        let pending = self
            .pending_file_observations
            .iter()
            .map(|(document, path)| (*document, path.clone()))
            .collect::<Vec<_>>();
        for (document, path) in pending {
            if self.observing_files.contains(&document) {
                continue;
            }
            if self.store.documents().snapshot(document).is_err() {
                self.pending_file_observations.remove(&document);
                continue;
            }
            if self.submit(HostCommand::Editor(Command::ObserveFile { path, document })) {
                self.pending_file_observations.remove(&document);
                self.observing_files.insert(document);
            } else {
                break;
            }
        }
    }

    pub(crate) fn draft_project(&self, document: DocumentId) -> Option<ProjectId> {
        self.files
            .values()
            .find(|file| file.id == document)
            .and_then(|file| file.project.clone())
            .or_else(|| {
                self.untitled
                    .values()
                    .find(|draft| draft.id == document)
                    .map(|draft| draft.project.clone())
            })
            .or_else(|| self.peek_models.project(document))
    }

    pub(crate) fn auto_save_delay(&self, document: DocumentId) -> Duration {
        if self.peek_models.owns(document) {
            return Duration::ZERO;
        }
        if self.deleted_documents.contains(&document) {
            return Duration::ZERO;
        }
        if self
            .store
            .documents()
            .snapshot(document)
            .is_ok_and(|snapshot| {
                self.missing_drafts.values().any(|draft| {
                    snapshot.key
                        == taide_native_editor::document::DocumentKey::File(draft.canonical.clone())
                })
            })
        {
            return Duration::ZERO;
        }
        if self
            .store
            .documents()
            .snapshot(document)
            .is_ok_and(|snapshot| {
                matches!(
                    snapshot.key,
                    taide_native_editor::document::DocumentKey::File(_)
                ) && !snapshot.metadata.read_only
            })
        {
            return Duration::from_millis(u64::from(
                self.services.state.settings.read().auto_save_delay_ms,
            ));
        }
        Duration::ZERO
    }

    pub(crate) fn flush_persistence(&mut self, context: &egui::Context) {
        if self.closing.is_some()
            || self.is_exit_ready
            || self.services.state.is_shutting_down()
            || self.pending_tab_close.is_some()
            || self.pending_disk_choice.is_some()
            || self.saving_untitled.is_some()
            || self.saving_missing_draft.is_some()
        {
            return;
        }
        let now = Instant::now();
        for due in self.persistence.due(now) {
            if self.observing_files.contains(&due.document) {
                continue;
            }
            let Ok(snapshot) = self.store.documents().snapshot(due.document) else {
                self.persistence.settled(due.document);
                continue;
            };
            if due.save {
                if let taide_native_editor::document::DocumentKey::File(canonical) = &snapshot.key
                    && !snapshot.metadata.read_only
                    && !self.auto_save_delay(due.document).is_zero()
                {
                    let path = self
                        .files
                        .iter()
                        .find(|(_, file)| file.id == due.document)
                        .map(|(path, _)| path.clone())
                        .or_else(|| canonical.to_str().map(String::from));
                    if let Some(path) = path {
                        match self.store.save_snapshot(due.document) {
                            Ok(snapshot) => {
                                if !self.submit(HostCommand::Editor(Command::AutoSave {
                                    path,
                                    snapshot,
                                })) {
                                    context
                                        .request_repaint_after(crate::persistence::MIRROR_DEBOUNCE);
                                    return;
                                }
                            }
                            Err(error) => self.report_save_failure(error, true),
                        }
                    }
                    continue;
                }
                self.persistence.disable_auto_save(due.document);
            }
            if due.mirror
                && let Some(project) = self.draft_project(due.document)
            {
                match self.store.documents().snapshot(due.document) {
                    Ok(snapshot) => {
                        if let Some(job) = self.persistence.mirror_job(snapshot, project) {
                            if !self.submit(HostCommand::Editor(Command::MirrorDraft(job.clone())))
                            {
                                context.request_repaint_after(crate::persistence::MIRROR_DEBOUNCE);
                                return;
                            }
                            self.persistence.mirror_started(&job);
                        }
                    }
                    Err(error) => log::warn!("native draft mirror snapshot failed: {error:?}"),
                }
            }
        }
        if let Some(deadline) = self.persistence.next_wake() {
            let wait = deadline.saturating_duration_since(now);
            context.request_repaint_after(if wait.is_zero() {
                crate::persistence::MIRROR_DEBOUNCE
            } else {
                wait
            });
        }
    }

    pub(crate) fn reconcile_editor_features(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        scopes: &[WindowScope],
    ) -> (Vec<(TabId, DocumentEdit)>, Vec<(TabId, FoldCommand)>) {
        let document_edits = std::mem::take(&mut self.document_edits);
        self.editor_find
            .retain(|view, _| self.store.views().get(*view).is_some());
        self.editor_problems.retain(&self.store);
        let projects = snapshot
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect();
        self.editor_locations
            .reconcile(&self.store, &projects, |project, document, kind| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.location_providers(project, document, kind)
                })
            });
        self.editor_locations
            .retain_active(&snapshot.layouts, scopes);
        self.editor_locations.detach_retired(&mut self.store);
        let inlay_hint_previews = self.editor_locations.preview_views();
        self.editor_selection_ranges.reconcile(
            &mut self.store,
            |request| {
                self.closing.is_none()
                    && (request.source == request.owner
                        || inlay_hint_previews.contains(&request.source))
                    && snapshot.layouts.iter().any(|(project, layout)| {
                        projects.contains(project)
                            && scopes
                                .iter()
                                .any(|scope| request.is_active(project, layout, scope))
                    })
            },
            |project, document| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.selection_range_providers(project, document)
                })
            },
        );
        let code_lens_enabled = self.services.state.settings.read().editor_code_lens_enabled;
        self.editor_code_lenses.reconcile(
            &self.store,
            |request| {
                code_lens_enabled
                    && self.closing.is_none()
                    && (request.source == request.owner
                        || inlay_hint_previews.contains(&request.source))
                    && projects.contains(&request.project)
                    && snapshot
                        .layouts
                        .get(&request.project)
                        .is_some_and(|layout| {
                            scopes.iter().any(|scope| request.is_active(layout, scope))
                        })
            },
            |project, document| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.code_lens_providers(project, document)
                })
            },
        );
        self.editor_inlay_hints.reconcile(
            &self.store,
            |request| {
                self.closing.is_none()
                    && (request.source == request.owner
                        || inlay_hint_previews.contains(&request.source))
                    && projects.contains(&request.project)
                    && snapshot
                        .layouts
                        .get(&request.project)
                        .is_some_and(|layout| {
                            scopes.iter().any(|scope| request.is_active(layout, scope))
                        })
            },
            |project, document| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.inlay_hint_providers(project, document)
                })
            },
        );
        self.editor_highlights.reconcile(
            &self.store,
            |request| {
                self.closing.is_none()
                    && request.project.as_ref().is_none_or(|project| {
                        projects.contains(project)
                            && snapshot.layouts.get(project).is_some_and(|layout| {
                                scopes.iter().any(|scope| request.is_active(layout, scope))
                            })
                    })
            },
            |project, document| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.highlight_providers(project, document)
                })
            },
        );
        self.editor_documentation.reconcile(
            &self.store,
            &projects,
            |request| {
                snapshot
                    .layouts
                    .get(&request.project)
                    .is_some_and(|layout| {
                        scopes.iter().any(|scope| request.is_active(layout, scope))
                    })
            },
            |project, document, kind| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.documentation_providers(project, document, kind)
                })
            },
        );
        self.editor_completion.reconcile(
            &self.store,
            &projects,
            |request| {
                snapshot
                    .layouts
                    .get(&request.project)
                    .is_some_and(|layout| {
                        scopes.iter().any(|scope| request.is_active(layout, scope))
                    })
            },
            |project, document| {
                self.lsp.as_ref().map_or_else(HashSet::new, |lsp| {
                    lsp.completion_providers(project, document)
                })
            },
        );
        if self.editor_completion.poll_supply(&self.store) {
            context.request_repaint();
        }
        self.editor_completion
            .inline
            .reconcile_credentials(&self.services);
        self.editor_completion.inline.reconcile(
            &self.store,
            &self.services.state.settings.read(),
            |origin| {
                projects.contains(&origin.project)
                    && snapshot.layouts.get(&origin.project).is_some_and(|layout| {
                        scopes.iter().any(|scope| {
                            crate::symbol_sidebar::window_tree(&origin.project, layout, scope)
                                .is_some_and(|(root, _)| {
                                    taide_native_ui::snapshot::active_tab(
                                        root,
                                        &origin.owner_key.pane,
                                    )
                                    .is_some_and(|tab| tab.id == origin.owner_key.tab)
                                })
                        })
                    })
            },
        );
        let cancelled_edits =
            self.editor_completion
                .inline_edit
                .reconcile(&self.store, &self.services, |origin| {
                    projects.contains(&origin.project)
                        && snapshot.layouts.get(&origin.project).is_some_and(|layout| {
                            scopes.iter().any(|scope| {
                                crate::symbol_sidebar::window_tree(&origin.project, layout, scope)
                                    .is_some_and(|(root, _)| {
                                        taide_native_ui::snapshot::active_tab(
                                            root,
                                            &origin.owner_key.pane,
                                        )
                                        .is_some_and(|tab| tab.id == origin.owner_key.tab)
                                    })
                            })
                        })
                });
        if cancelled_edits > 0 {
            self.editor_completion
                .edit_notices
                .push(crate::editor_completion::EditNotice::Cancelled);
        }
        for notice in std::mem::take(&mut self.editor_completion.edit_notices) {
            match notice {
                crate::editor_completion::EditNotice::Cancelled => self.toasts.info(
                    presentation::message(&self.locale, "ai.inlineEditCancelled", &[]),
                    Instant::now(),
                ),
                crate::editor_completion::EditNotice::Empty => self.toasts.error(
                    presentation::message(&self.locale, "ai.inlineEditEmptyResponse", &[]),
                    Instant::now(),
                ),
                crate::editor_completion::EditNotice::Failed(error) => self.toasts.notify(
                    taide_native_ui::toast::Kind::Error,
                    presentation::message(&self.locale, "ai.inlineEditFailed", &[]),
                    taide_native_ui::toast::Options {
                        description: Some(crate::toast::describe_error(&self.locale, &error)),
                        ..Default::default()
                    },
                    Instant::now(),
                ),
            }
        }
        if let Err(error) = self.editor_documentation.prepare_code(
            &mut self.store,
            &mut self.editor_syntax,
            self.services.plugin.0.read().as_deref().unwrap_or_default(),
        ) {
            self.status = Some(editor_error(error).to_string());
        }
        self.editor_documentation
            .prepare_images(&self.store, &context, &self.services);
        if let Err(error) = self.editor_completion.prepare_code(
            &mut self.store,
            &mut self.editor_syntax,
            self.services.plugin.0.read().as_deref().unwrap_or_default(),
        ) {
            self.status = Some(editor_error(error).to_string());
        }
        self.editor_completion
            .prepare_images(&self.store, &context, &self.services);
        self.peek_models.sweep(
            &mut self.store,
            &self.editor_locations.preview_views(),
            Instant::now(),
        );
        if let Some(delay) = self.peek_models.next_expiry(Instant::now()) {
            context.request_repaint_after(delay);
        }
        (document_edits, std::mem::take(&mut self.fold_commands))
    }
}

impl EditorSurfaces for AppSurfaces<'_> {
    fn problems_open(&self, slot: &ShellSlotId) -> bool {
        self.problems.is_open(slot)
    }
    fn problems_panel(&mut self, ui: &mut Ui, project: &ProjectId, slot: &ShellSlotId) {
        let opens = match self.problems.show_panel(
            ui,
            slot,
            self.diagnostics,
            self.locale,
            self.problems_appearance,
        ) {
            Ok(open) => open,
            Err(error) => {
                *self.status = Some(error.to_string());
                return;
            }
        };
        for open in opens {
            if matches!(
                open.key,
                taide_native_editor::document::DocumentKey::Untitled(_)
            ) {
                self.problem_opens.push(open);
                ui.ctx().request_repaint();
                continue;
            }
            let taide_native_editor::document::DocumentKey::File(path) = open.key else {
                continue;
            };
            self.commands
                .push(HostCommand::Editor(Command::OpenProblem {
                    project: project.clone(),
                    path: path.to_string_lossy().into_owned(),
                    line: open.line,
                    column: open.column,
                    viewport: ui.ctx().viewport_id(),
                }));
        }
    }
    fn tab_content(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        intents: &mut Vec<ShellIntent>,
    ) {
        if matches!(tab.kind, TabKind::Settings) {
            self.show_settings_tab(ui, project, pane, tab, intents);
            return;
        }
        if let TabKind::AppFile { target } = tab.kind {
            self.show_app_file_tab(ui, project, pane, tab, target, intents);
            return;
        }
        if let TabKind::Terminal { session_id, .. } = &tab.kind {
            self.show_terminal_tab(ui, pane, tab, session_id, intents);
            return;
        }
        if matches!(tab.kind, TabKind::Untitled { .. }) {
            if let Some(error) = self.untitled_failed.get(&tab.id) {
                ui.label(error.to_string());
                return;
            }
            let Some(document) = self.untitled.get(&tab.id) else {
                if !self.untitled_loading.contains(&tab.id) {
                    self.load_untitled.push(tab.id.clone());
                }
                ui.spinner();
                return;
            };
            self.show_document(ui, pane, tab, document.id, None, intents);
            return;
        }
        match &tab.kind {
            TabKind::SearchEditor { .. } => {
                self.show_search_editor_tab(ui, tab);
                return;
            }
            TabKind::Diff { .. } => {
                self.show_diff_tab(ui, tab);
                return;
            }
            TabKind::ClaudeDiff { .. } => {
                self.show_claude_diff_tab(ui, tab);
                return;
            }
            _ => {}
        }
        let TabKind::File { path } = &tab.kind else {
            self.show_unavailable_tab(ui, tab);
            return;
        };
        if let crate::open_with::Surface::Preview(kind) = self.open_with.surface(path) {
            self.show_preview_tab(ui, pane, tab, path, kind, intents);
            return;
        }
        if let Some(draft) = self.missing_drafts.get(path) {
            ui.horizontal(|ui| {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    presentation::message(self.locale, "editor.sourceDeleted", &[]),
                );
                if ui
                    .button(presentation::message(
                        self.locale,
                        "editor.saveDraftAs",
                        &[],
                    ))
                    .clicked()
                {
                    self.save_missing_drafts.push(tab.id.clone());
                }
            });
            egui::ScrollArea::vertical()
                .id_salt(("native-missing-draft", &tab.id))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(&draft.mirror.content).monospace());
                });
            return;
        }
        if self.refused.contains(path) {
            let (_, command) = crate::file_loading::show_refused(ui, self.locale, path);
            if let Some(command) = command {
                self.commands.push(command);
            }
            return;
        }
        if let Some(error) = self.failed.get(path) {
            ui.allocate_ui_with_layout(
                ui.available_size(),
                egui::Layout::top_down(egui::Align::Center).with_main_align(egui::Align::Center),
                |ui| {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        crate::toast::describe_error(self.locale, error),
                    );
                },
            );
            return;
        }
        let Some(document) = self.files.get(path) else {
            if !self.loading.contains(path) {
                self.load_files.push((path.clone(), tab.id.clone()));
            }
            ui.spinner();
            return;
        };
        self.show_document(ui, pane, tab, document.id, Some(path), intents);
    }
    fn tab_icon(
        &mut self,
        ui: &Ui,
        rect: egui::Rect,
        tab: &Tab,
        agent_color: Option<egui::Color32>,
        title_color: egui::Color32,
    ) {
        let (glyph, color) = crate::problems_icons::tab(&tab.kind, agent_color.is_some());
        let color = agent_color.unwrap_or_else(|| {
            color.map_or(title_color, |color| {
                self.explorer_appearance.file_color(color)
            })
        });
        if let Err(error) = self.explorer_icons.paint(ui, rect, glyph, color, 0.0) {
            *self.status = Some(error.to_string());
        }
    }
}

impl AppSurfaces<'_> {
    pub(crate) fn show_document(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        tab: &Tab,
        document: DocumentId,
        path: Option<&str>,
        intents: &mut Vec<ShellIntent>,
    ) {
        let key = ViewKey {
            window: WINDOW_LABEL.into(),
            pane: pane.clone(),
            tab: tab.id.clone(),
        };
        let result = self
            .store
            .attach_view(key, document)
            .map_err(editor_error)
            .and_then(|view| {
                self.editor_view_state.restore(self.store, view, tab.view_state.as_deref()).map_err(editor_error)?;
                let indent_configuration = crate::presentation_refresh::indent_configuration(
                    &self.services.state.settings.read(),
                );
                let indent = self.store.configure_indentation(document, indent_configuration)
                    .map_err(editor_error)?;
                let snapshot = self
                    .store
                    .documents()
                    .snapshot(document)
                    .map_err(editor_error)?;
                self.show_breadcrumbs(ui, pane, tab, view, &snapshot, intents)?;
                let mut focus = Some(pane) == self.target
                    && self
                        .focused
                        .as_ref()
                        .is_none_or(|(old_pane, old_tab)| old_pane != pane || old_tab != &tab.id);
                if path.is_some() {
                    if snapshot.metadata.read_only {
                        let key = if snapshot.metadata.lossy {
                            "editor.readOnlyLossyEncoding"
                        } else {
                            "editor.readOnlyLargeFile"
                        };
                        ui.colored_label(
                            self.banner_appearance.warning,
                            presentation::message(self.locale, key, &[]),
                        );
                    }
                    let variant = if self.store.has_disk_conflict(document).unwrap_or(false) {
                        Some(BannerVariant::ChangedOnDisk)
                    } else {
                        self.restore_notices.get(&tab.id).copied()
                    };
                    if let Some(variant) = variant {
                        let output = taide_native_ui::conflict_banner::show(
                            ui,
                            self.locale,
                            self.banner_appearance,
                            variant,
                        );
                        if let Some(action) = output.action {
                            self.banner_actions.push((tab.id.clone(), action));
                        }
                    }
                }
                self.editor_markdown.toolbar(ui, pane, &snapshot.metadata.language_id,
                    &presentation::message(self.locale, "editor.toggleMarkdownPreview", &[]));
                let (editor_rect, _) = self.editor_markdown.regions(ui, pane,
                    &snapshot.metadata.language_id,
                    self.services.state.settings.read().resizer_thickness as f32,
                    self.editor_markdown_colors.border);
                let parent_ui = ui;
                let mut editor_ui = parent_ui.new_child(egui::UiBuilder::new().id(parent_ui.id()).max_rect(editor_rect));
                editor_ui.set_clip_rect(editor_rect.intersect(parent_ui.clip_rect()));
                let ui = &mut editor_ui;
                let settings = self.services.state.settings.read();
                let mut editor_presentation =
                    crate::presentation_refresh::editor_presentation(&settings);
                editor_presentation.options.colors = Some(self.editor_display_colors);
                editor_presentation.options.bracket_colors = Some(self.editor_bracket_colors);
                editor_presentation.options.sticky_colors = Some(self.editor_sticky_colors);
                let editor_project = self.layouts.iter().find_map(|(project, layout)| {
                    taide_layout::service::all_roots(layout).any(|root| taide_layout::service::find_leaf(root, pane).is_some()).then_some(project)
                });
                let syntax_folds = editor_project.and_then(|project| self.editor_folding.model(project, &snapshot))
                    .or_else(|| self.editor_folding.builtin_model(&snapshot));
                editor_presentation.options.sticky_model = editor_project.and_then(|project| self.editor_symbols.sticky(project, &snapshot))
                    .or_else(|| self.editor_symbols.builtin_sticky(&snapshot));
                editor_presentation.options.minimap_colors = Some(self.editor_minimap_colors);
                editor_presentation.options.diagnostic_colors = Some(self.editor_diagnostic_colors);
                editor_presentation.options.overview_colors = Some(self.editor_overview_colors);
                editor_presentation.options.problem_colors = Some(self.editor_problem_colors);
                editor_presentation.options.location_colors = Some(self.editor_location_colors);
                editor_presentation.options.documentation_colors = Some(self.editor_documentation_colors);
                editor_presentation.options.completion_colors = Some(self.editor_completion_colors);
                editor_presentation.options.sticky_scroll = self.editor_sticky_scroll;
                editor_presentation.options.sticky_toggle_label = Some(presentation::message(self.locale, "settings.editorStickyScroll", &[]));
                drop(settings);
                editor_presentation.options.folding =
                    taide_native_ui::presentation::editor_folding(snapshot.metadata.tier);
                editor_presentation.options.bracket_pair_colorization &=
                    taide_native_ui::presentation::editor_folding(snapshot.metadata.tier);
                let editor = self.editor.with_indent(indent);
                if ui.is_enabled()
                    && let Some(path) = path
                    && let Some(position) =
                        self.reveals
                            .consume(&tab.id, path, ui.ctx().viewport_id(), Instant::now())
                {
                    if let Some(target) = self.editor_locations.take_navigation(&tab.id, path, ui.ctx().viewport_id(), Instant::now()) {
                        self.editor_locations.navigate(self.store, view, &target, Instant::now()).map_err(editor_error)?;
                    } else {
                    let tokens = self.editor_syntax.tokens(self.store, document);
                    editor
                        .reveal_tokenized(
                            ui,
                            self.store,
                            view,
                            position.line,
                            position.column,
                            &editor_presentation,
                            tokens,
                        )
                        .map_err(editor_error)?;
                    }
                    focus = true;
                }
                if let Some(path) = path && let Some(project) = editor_project {
                    self.editor_locations.adopt_pending(self.store, view, project, path, ui.ctx().viewport_id(), Instant::now()).map_err(editor_error)?;
                }
                let mut next = 0;
                let mut problem_provider = crate::editor_problems::Provider {
                    state: &mut *self.editor_problems, diagnostics: self.diagnostics,
                    project: self.layouts.iter().find(|(_, layout)| taide_layout::service::all_roots(layout).any(|root| taide_layout::service::find_leaf(root, pane).is_some())).map(|(project, _)| project.clone()),
                    commands: &mut *self.commands, viewport: ui.ctx().viewport_id(), scope: self.scope,
                };
                if ui.is_enabled() {
                    for (_, edit) in self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && matches!(edit, DocumentEdit::Problem(_))) {
                        if let DocumentEdit::Problem(command) = edit && let Err(error) = taide_native_ui::editor_problems::Provider::execute(&mut problem_provider, self.store, view, command) {
                            *self.status = Some(editor_error(error).to_string());
                        }
                    }
                }
                let mut edit_errors = Vec::new();
                let text_resources = crate::editor_command_text::resources()?;
                let compare = |left: &str, right: &str| text_resources.compare(left, right);
                let preview_document = self.editor_locations.prepare_preview(self.store, view).map_err(editor_error)?;
                let preview_tokens = preview_document.and_then(|document| self.editor_syntax.peek_tokens(self.store, document));
                let hover_tokens = self.editor_locations.hover_document(self.store, view).and_then(|document| self.editor_syntax.peek_tokens(self.store, document));
                let preview_presentation = editor_presentation.clone();
                let navigation_highlight = self.editor_locations.highlight(self.store, view, Instant::now(), self.editor_location_colors.highlight.to_array());
                if let Some((_, delay)) = &navigation_highlight { ui.ctx().request_repaint_after(*delay); }
                let mut location_commands = Vec::new();
                let documentation_state = std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_documentation));
                let highlight_consumer = crate::editor_highlights::Consumer {
                    state: std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_highlights)),
                    lsp: self.lsp,
                    context: ui.ctx().clone(),
                    colors: self.editor_highlight_colors,
                };
                if ui.is_enabled() {
                    let highlight_commands = self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && matches!(edit, DocumentEdit::Highlight(_)))
                        .filter_map(|(_, edit)| if let DocumentEdit::Highlight(command) = edit { Some(command) } else { None }).collect::<Vec<_>>();
                    for command in highlight_commands {
                        let source = highlight_consumer.state.borrow().source_for_owner(self.store, ui.ctx().viewport_id(), view);
                        if let Err(error) = highlight_consumer.execute(self.store, editor_project, view, source, command) {
                            *self.status = Some(editor_error(error).to_string());
                        }
                    }
                }
                let completion_consumer = crate::editor_completion::Consumer {
                    state: std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_completion)),
                    services: self.services,
                    files: self.settings_views.snippet_catalog().snapshot().unwrap_or_default(),
                    locale: Some(self.locale), edit_colors: Some(self.editor_ai_edit_colors),
                    context: ui.ctx().clone(),
                };
                let code_lens_consumer = crate::editor_code_lenses::Consumer {
                    state: std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_code_lenses)),
                    lsp: self.lsp, context: ui.ctx().clone(), colors: self.editor_code_lens_colors,
                    enabled: self.services.state.settings.read().editor_code_lens_enabled,
                };
                let selection_range_consumer = crate::editor_selection_ranges::Consumer {
                    state: std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_selection_ranges)),
                    lsp: self.lsp, context: ui.ctx().clone(),
                };
                let selection_range_preview = self.editor_locations.current(view).and_then(|session| session.preview);
                let selection_range_source = ui.memory(|memory| memory.focused()).and_then(|id| self.editor_keymap_targets.get(&(ui.ctx().viewport_id(), id))).and_then(|(source, _)| {
                    (*source == view || selection_range_preview == Some(*source)).then_some(*source)
                }).unwrap_or(view);
                if ui.is_enabled() {
                    for _ in self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && *edit == DocumentEdit::CodeLens) {
                        let preview = self.editor_locations.current(view).and_then(|session| session.preview);
                        let source = ui.memory(|memory| memory.focused()).and_then(|id| self.editor_keymap_targets.get(&(ui.ctx().viewport_id(), id))).and_then(|(source, _)| {
                            (*source == view || preview == Some(*source)).then_some(*source)
                        }).unwrap_or(view);
                        code_lens_consumer.before(self.store, editor_project, view, source);
                        code_lens_consumer.state.borrow_mut().begin_picker(self.store, ui.ctx().viewport_id(), source);
                        ui.ctx().request_repaint();
                    }
                }
                let inlay_consumer = crate::editor_inlay_hints::Consumer {
                    state: std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_inlay_hints)),
                    lsp: self.lsp, context: ui.ctx().clone(), colors: self.editor_inlay_hint_colors,
                };
                let mut documentation_host_commands = Vec::new();
                let mut documentation_provider = crate::editor_documentation::Provider {
                    state: documentation_state.clone(), project: editor_project.cloned(), owner: view,
                    owner_key: self.store.views().get(view).ok_or_else(|| editor_error(taide_native_editor::document::EditorError::NotFound))?.key.clone(),
                    lsp: self.lsp, viewport: ui.ctx().viewport_id(), commands: &mut documentation_host_commands,
                    appearance: &editor.appearance, language: snapshot.metadata.language_id.clone(),
                };
                let documentation_commands = if ui.is_enabled() {
                    self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && matches!(edit, DocumentEdit::Documentation(_)))
                        .filter_map(|(_, edit)| if let DocumentEdit::Documentation(command) = edit { Some(command) } else { None }).collect::<Vec<_>>()
                } else { Vec::new() };
                let completion_commands = if ui.is_enabled() {
                    self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && matches!(edit, DocumentEdit::Completion(_)))
                        .filter_map(|(_, edit)| if let DocumentEdit::Completion(command) = edit { Some(command) } else { None }).collect::<Vec<_>>()
                } else { Vec::new() };
                let mut preview_focus_targets = Vec::new();
                let mut preview_shown_lines = Vec::new();
                let preview_overrides = self.services.state.settings.read().keymap_overrides.clone();
                let mut location_provider = crate::editor_locations::Provider {
                    state: &mut *self.editor_locations, models: self.peek_models, project: editor_project.cloned(), lsp: self.lsp,
                    symbols: Some(self.editor_symbols), folding: Some(self.editor_folding), diagnostics: Some(self.diagnostics),
                    layout: editor_project.and_then(|project| self.layouts.get(project)), scope: self.scope,
                    commands: &mut location_commands, viewport: ui.ctx().viewport_id(), editor: &editor,
                    indentation: indent_configuration,
                    presentation: &preview_presentation, tokens: preview_tokens, hover_tokens, shown_lines: &mut preview_shown_lines, changed: &mut *self.changed_documents,
                    overrides: preview_overrides.as_deref(), focus_targets: &mut preview_focus_targets,
                    find_history: None, find_appearance: None,
                    documentation: Some(documentation_state),
                    completion: Some(completion_consumer.clone()),
                    highlights: Some(highlight_consumer.clone()),
                    inlay_hints: Some(inlay_consumer.clone()),
                    code_lenses: Some(code_lens_consumer.clone()),
                    selection_ranges: Some(selection_range_consumer.clone()),
                    rename: Some(crate::editor_rename_controller::Consumer(std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_rename)))),
                    code_actions: Some(crate::editor_code_actions_controller::Consumer(std::rc::Rc::new(std::cell::RefCell::new(&mut *self.editor_code_actions)))),
                    code_action_colors: Some(self.editor_code_action_colors),
                };
                let mut code_action_provider = crate::editor_code_actions_controller::Provider { consumer: location_provider.code_actions.as_ref().expect("code action consumer").clone(), lsp: self.lsp, context: ui.ctx().clone(), font: editor.appearance.font.clone(), tab_size: editor.indent_options(&snapshot).tab_size };
                let mut rename_provider = crate::editor_rename_controller::Provider {
                    consumer: location_provider.rename.as_ref().expect("rename consumer").clone(), lsp: self.lsp,
                };
                if ui.is_enabled() {
                    for (_, edit) in self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && matches!(edit, DocumentEdit::Location(_))) {
                        if let DocumentEdit::Location(command) = edit && let Err(error) = taide_native_ui::editor_locations::Provider::execute(&mut location_provider, self.store, view, command) {
                            *self.status = Some(editor_error(error).to_string());
                        }
                    }
                }
                let syntax_lease =
                    crate::editor_syntax::SyntaxLease::new(&mut *self.editor_syntax, document);
                let language = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
                    .map(
                        |rules| taide_native_editor::language_configuration::Language {
                            rules,
                            syntax: &syntax_lease,
                        },
                    );
                let mut completion_host_commands = Vec::new();
                if ui.is_enabled() {
                    selection_range_consumer.before(self.store, editor_project, view, view,
                        taide_native_editor::line_commands::LineCommandContext {
                            indent: editor.indent_options(&snapshot), language, syntax: &syntax_lease,
                            compare: Some(&compare), transforms: Some(&text_resources.transforms),
                            word_rules: language.map(|language| language.rules).or_else(|| crate::editor_syntax::language_rules("plaintext")),
                        }).map_err(editor_error)?;
                }
                let mut completion_provider = crate::editor_completion::Provider {
                    consumer: completion_consumer.clone(), project: editor_project.cloned(), owner: view,
                    owner_key: self.store.views().get(view).ok_or_else(|| editor_error(taide_native_editor::document::EditorError::NotFound))?.key.clone(),
                    lsp: self.lsp, viewport: ui.ctx().viewport_id(), commands: &mut completion_host_commands,
                    editor: &editor, syntax: &syntax_lease, language: snapshot.metadata.language_id.clone(),
                    model_path: match &snapshot.key {
                        taide_native_editor::document::DocumentKey::File(path) => path.to_string_lossy().into_owned(),
                        taide_native_editor::document::DocumentKey::Untitled(tab) => tab.to_string(),
                        taide_native_editor::document::DocumentKey::AppFile(_) => path.unwrap_or(&tab.title).to_owned(),
                    },
                };
                let mut menu_keyboard_open = false;
                if ui.is_enabled() {
                    for _ in self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && *edit == DocumentEdit::ContextMenu) {
                        let preview = location_provider.state.current(view).and_then(|session| session.preview);
                        let request = crate::editor_context_menu::take_palette(ui.ctx(), view);
                        if request.as_ref().is_some_and(|request| {
                            !request.matches(self.store, request.source, view, ui.ctx().viewport_id())
                        }) {
                            continue;
                        }
                        let source = request.map(|request| request.source).or_else(|| {
                            ui.memory(|memory| memory.focused()).and_then(|id| {
                                self.editor_keymap_targets.get(&(ui.ctx().viewport_id(), id))
                                    .map(|(source, _)| *source).filter(|source| Some(*source) == preview)
                            })
                        }).unwrap_or(view);
                        if source == view {
                            menu_keyboard_open = true;
                        } else if Some(source) == preview
                            && let Some(body) = crate::editor_context_menu::body(ui.ctx(), source)
                        {
                            taide_native_ui::editor_context_menu::request_keyboard(ui.ctx(), body, usize::MAX);
                        }
                    }
                }
                let edited = ui.is_enabled()
                    && crate::command_dispatch::apply_document_edits_with_cursor(
                        self.store,
                        view,
                        &tab.id,
                        taide_native_editor::line_commands::LineCommandContext {
                            indent: editor.indent_options(&snapshot),
                            language,
                            syntax: &syntax_lease,
                            compare: Some(&compare),
                            transforms: Some(&text_resources.transforms),
                            word_rules: language
                                .map(|language| language.rules)
                                .or_else(|| crate::editor_syntax::language_rules("plaintext")),
                        },
                        indent_configuration,
                        self.document_edits,
                        &mut edit_errors,
                        |store, command, context| {
                            if matches!(command, taide_native_editor::cursor_commands::CursorCommand::Expand | taide_native_editor::cursor_commands::CursorCommand::Shrink) {
                                selection_range_consumer.queue(store, editor_project, view, selection_range_source,
                                    command == taide_native_editor::cursor_commands::CursorCommand::Expand)?;
                                if selection_range_source == view {
                                    selection_range_consumer.before(store, editor_project, view, view, context)?;
                                }
                                return Ok(false);
                            }
                            taide_native_editor::cursor_commands::run_cursor_command(store, view, command, context)
                        },
                    );
                self.command_errors
                    .extend(edit_errors.into_iter().map(editor_error));
                let find = self.editor_find.entry(view).or_default();
                let compiler = taide_native_syntax::MonacoFindPatternCompiler;
                let rules = language.map(|language| language.rules).or_else(|| crate::editor_syntax::language_rules("plaintext"));
                let mut find_edited = false;
                if ui.is_enabled() {
                    for (_, edit) in self.document_edits.extract_if(.., |(owner, edit)| owner == &tab.id && matches!(edit, DocumentEdit::Find(_))) {
                        if let DocumentEdit::Find(command) = edit {
                            match find.execute(command, self.store, view, &compiler, rules) {
                                Ok(changed) => find_edited |= changed,
                                Err(error) => *self.status = Some(error.to_string()),
                            }
                        }
                    }
                }
                let before_find = self.store.documents().snapshot(document).map_err(editor_error)?.revision;
                let mut find_keymap_index = 0;
                let find_output = taide_native_ui::editor_find_widget::show(ui, find, self.find_history, self.store, view, &compiler, rules, self.find_appearance, |ui, event, composing| {
                    let index = crate::keymap::event_index(ui.ctx(), event, &mut find_keymap_index);
                    match self.terminal_views.route_find_keymap(
                        crate::keymap::Route {
                            context: ui.ctx(), event, index,
                            scope: crate::keymap::Context { terminal: false, editor: true },
                            composing,
                            overrides: self.services.state.settings.read().keymap_overrides.as_deref(),
                        }, self.keymap_actions, self.target.is_some(),
                    ) {
                        Ok(handled) => handled,
                        Err(error) => { *self.status = Some(error.to_string()); true },
                    }
                })
                    .map_err(|error| AppError::Internal(error.to_string()))?;
                find_edited |= before_find != self.store.documents().snapshot(document).map_err(editor_error)?.revision;
                focus |= find_output.request_editor_focus;
                if find_output.focused { *self.focused = Some((pane.clone(), tab.id.clone())); }
                editor_presentation.options.bracket_widget_focus = find_output.focused;
                if find_output.reserved_height > 0.0 { ui.allocate_space(egui::vec2(ui.available_width(), find_output.reserved_height)); }
                let find_decorations = find.decorations(&self.store.views().get(view).ok_or_else(|| editor_error(taide_native_editor::document::EditorError::NotFound))?.selection,
                    self.find_appearance.highlight, self.find_appearance.current_match, self.find_appearance.scope);
                let scroll_decorations = find.scroll_decorations(self.editor_overview_colors);
                let document_highlights = highlight_consumer.display(self.store, view);
                let find_layers = find_decorations.iter().chain(scroll_decorations.iter()).chain(navigation_highlight.as_ref().map(|(layer, _)| layer)).chain(document_highlights.as_ref().map(|display| &display.layer)).collect::<Vec<_>>();
                editor_presentation.options.diagnostics = self.diagnostics.display(self.store, &self.store.documents().snapshot(document).map_err(editor_error)?);
                let fold_commands = if ui.is_enabled() {
                    crate::command_dispatch::take_fold_commands(&tab.id, self.fold_commands)
                } else {
                    Vec::new()
                };
                let terminal_views = &mut *self.terminal_views;
                let keymap_actions = &mut *self.keymap_actions;
                let services = self.services;
                let status = &mut *self.status;
                let has_focused_shell = self.target.is_some();
                let find_visible = find.visible;
                let find_read_only = snapshot.metadata.read_only;
                let document_edits = &mut *self.document_edits;
                let chevrons = &mut *self.explorer_icons;
                let mut fold_control_error = None;
                let mut paint_fold_control = |ui: &Ui, control: FoldControl| {
                    let painted = chevrons.paint(
                        ui,
                        control.rect,
                        crate::problems_icons::Glyph::ChevronRight,
                        control.color,
                        control.chevron_rotation,
                    );
                    if let Err(error) = painted {
                        fold_control_error.get_or_insert(error);
                    }
                };
                location_provider.find_history = Some(self.find_history);
                location_provider.find_appearance = Some(self.find_appearance);
                let inlay_hints = inlay_consumer.display(self.store, view);
                let code_lenses = code_lens_consumer.before(self.store, editor_project, view, view);
                let output = editor
                    .show_request_with_code_lenses(
                        ui,
                        self.store,
                        view,
                        EditorRequest {
                            request_focus: focus,
                            keymap: |ui: &Ui, event: &egui::Event, composing: bool| {
                                let index = crate::keymap::event_index(ui.ctx(), event, &mut next);
                                let before = keymap_actions.len();
                                let routed = terminal_views.route_editor_keymap(
                                    crate::keymap::Route {
                                        context: ui.ctx(),
                                        event,
                                        index,
                                        scope: crate::keymap::Context {
                                            terminal: false,
                                            editor: true,
                                        },
                                        composing,
                                        overrides: services
                                            .state
                                            .settings
                                            .read()
                                            .keymap_overrides
                                            .as_deref(),
                                    },
                                    keymap_actions,
                                    has_focused_shell,
                                    &completion_consumer,
                                    view,
                                );
                                if keymap_actions.get(before).is_some_and(|id| id == "monaco.editor.action.showContextMenu") {
                                    keymap_actions.remove(before);
                                    taide_native_ui::editor_context_menu::request_keyboard(ui.ctx(), ui.make_persistent_id(("native-code-editor", view)), index);
                                }
                                match routed {
                                    Ok(true) => true,
                                    Ok(false) => {
                                        if !composing && let Some(command) = taide_native_ui::editor_find_widget::editor_shortcut(event, ui.ctx().os().is_mac(), find_visible) && (!command.requires_write() || !find_read_only) {
                                            document_edits.push((tab.id.clone(), DocumentEdit::Find(command)));
                                            ui.ctx().request_repaint();
                                            return true;
                                        }
                                        false
                                    },
                                    Err(error) => {
                                        *status = Some(error.to_string());
                                        true
                                    }
                                }
                            },
                            route: |response: &egui::Response| {
                                response.ctx.keyboard_input_route(response.id)
                            },
                            presentation: &editor_presentation,
                            tokens: tokens_supplier(|store| syntax_lease.frame_tokens(store)),
                            language,
                            decorations: &find_layers,
                            fold_commands: &fold_commands,
                            syntax_folds: syntax_folds.clone(),
                            documentation: Some(&mut documentation_provider),
                            documentation_commands: &documentation_commands,
                            completion: Some(&mut completion_provider),
                            completion_commands: &completion_commands,
                            fold_controls: Some(&mut paint_fold_control),
                            problems: Some(&mut problem_provider),
                            locations: Some(&mut location_provider),
                        },
                        |_, _, _, _, _| false,
                        Some((&mut rename_provider, self.find_appearance.into())),
                        Some((&mut code_action_provider, self.editor_code_action_colors)),
                        inlay_hints,
                        code_lenses,
                    )
                    .map_err(editor_error)?;
                let menu = crate::editor_context_menu::show(
                    ui, self.store,
                    taide_native_ui::editor_context_menu::Request {
                        source: view, owner: view, response: &output.response, geometry: &output.geometry,
                        keyboard_open: menu_keyboard_open, excluded: &[],
                    },
                    crate::editor_context_menu::Environment {
                        project: editor_project, lsp: self.lsp, locale: Some(self.locale),
                        overrides: preview_overrides.as_deref(), scope: self.scope, embedded: false,
                    }, self.commands,
                ).map_err(editor_error)?;
                if menu.opened {
                    *self.focused = Some((pane.clone(), tab.id.clone()));
                    intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
                }
                self.commands.append(&mut location_commands);
                self.commands.append(&mut documentation_host_commands);
                self.commands.append(&mut completion_host_commands);
                if matches!(tab.kind, TabKind::File { .. } | TabKind::Untitled { .. }) && let Some(project) = editor_project {
                    self.commands.extend(output.formatting_inputs.into_iter().map(|input| HostCommand::Editor(Command::FormatEditorInput { project: project.clone(), source: view, owner: view, input })));
                }
                if let Some(display) = &document_highlights { display.paint(ui, &output.geometry); }
                inlay_consumer.observe(self.store, editor_project, view, view, output.rendered_lines.clone());
                self.editor_view_state.observe(self.store, view, &tab.kind, &output.geometry);
                code_lens_consumer.after(self.store, view, &output.code_lenses);
                selection_range_consumer.after(self.store, view);
                if let TabKind::File { path } = &tab.kind && let Some(project) = editor_project {
                    self.editor_ide_selection.observe(self.store, project, self.scope, view, path, Instant::now());
                }
                if output.response.has_focus() || ui.memory(|memory| output.focus_ids.iter().any(|id| {
                    memory.has_focus(*id) && !preview_focus_targets.iter().any(|(target, _)| target == id)
                })) {
                    highlight_consumer.observe(self.store, editor_project, view, view);
                    code_action_provider.consumer.observe(self.store, editor_project, view, view, self.lsp, Instant::now());
                    if let Some(delay) = code_action_provider.consumer.0.borrow().next_refresh(Instant::now()) { ui.ctx().request_repaint_after(delay); }
                }
                if let Some(error) = fold_control_error {
                    *self.status = Some(error.to_string());
                }
                self.editor_syntax
                    .show_lines(document, output.rendered_lines.clone());
                for (document, lines) in preview_shown_lines { self.editor_syntax.show_lines(document, lines); }
                if output.toggle_sticky_scroll {
                    intents.push(ShellIntent::Editor(
                        taide_native_ui::intents::editor::Intent::ToggleEditorStickyScroll,
                    ));
                }
                self.keymap_documents.insert(tab.id.clone(), document);
                if output.response.enabled() {
                    for id in &output.focus_ids {
                        self.editor_keymap_targets.insert(
                            (ui.ctx().viewport_id(), *id),
                            (preview_focus_targets.iter().find(|(target, _)| target == id).map_or(view, |(_, preview)| *preview), ui.ctx().cumulative_frame_nr()),
                        );
                    }
                    self.editor_keymap_targets.insert(
                        (ui.ctx().viewport_id(), output.response.id),
                        (view, ui.ctx().cumulative_frame_nr()),
                    );
                }
                let ai_control_focused = ui.memory(|memory| memory.focused()).is_some_and(|id| taide_native_ui::editor_ai_edit::is_control(ui.ctx(), output.response.id, id));
                if !ai_control_focused && (output.response.has_focus() || ui.memory(|memory| output.focus_ids.iter().any(|id| memory.has_focus(*id)))) {
                    *self.keymap_editor_scope = Some(
                        self.store
                            .views()
                            .get(view)
                            .is_some_and(|view| view.composition.is_some()),
                    );
                }
                if focus {
                    *self.focused = Some((pane.clone(), tab.id.clone()));
                }
                if output.response.clicked() {
                    intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
                }
                if output.changed || edited || find_edited || self.changed_documents.contains_key(&document) {
                    if matches!(tab.kind, TabKind::AppFile { .. }) {
                        self.app_file_views.changed(document);
                    }
                    self.changed_documents.insert(document, view);
                    let snapshot = self
                        .store
                        .documents()
                        .snapshot(document)
                        .map_err(editor_error)?;
                    let tabs: Vec<_> = if let TabKind::AppFile { target } = tab.kind {
                        crate::app_file_views::target_tabs(&self.services.state, target)
                    } else {
                        self.store
                            .views()
                            .for_document(document)
                            .map(|view| view.key.tab.clone())
                            .collect()
                    };
                    for tab in tabs {
                        self.commands
                            .push(HostCommand::Editor(Command::SetDirty {
                                tab,
                                dirty: snapshot.dirty,
                            }));
                    }
                }
                for error in output.errors {
                    *self.status = Some(format!("native editor: {error:?}"));
                }
                let latest = self.store.documents().snapshot(document).map_err(editor_error)?;
                let owner = &self.store.views().get(view).ok_or_else(|| editor_error(taide_native_editor::document::EditorError::NotFound))?.key;
                self.editor_markdown.show(parent_ui, pane, owner, editor_project, &latest,
                    &editor.appearance, self.editor_markdown_colors, self.locale, self.services, self.commands);
                Ok(())
            });
        if let Err(error) = result {
            *self.status = Some(error.to_string());
        }
    }
}

fn tokens_supplier<'tokens, Supplier>(supplier: Supplier) -> Supplier
where
    Supplier: FnOnce(&EditorStore) -> Option<EditorTokens<'tokens>>,
{
    supplier
}

use std::sync::Arc;

use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::TabKind;
use taide_model::tree::TreeRowPage;
use taide_native_editor::store::SaveSnapshot;
use taide_native_ui::document_admission::{PreparedDocument, prepare_opened_document};
use taide_runtime::{
    AppServices, layout_actions, native_file_actions, system_actions, tree_actions,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub const HOST_COMMAND_CAPACITY: usize = 64;
const TERMINAL_CLIPBOARD_BYTES: usize = 64 * 1024;

pub type ClipboardWriter = Arc<dyn Fn(&str) -> AppResult<()> + Send + Sync>;
pub type ClipboardReader = Arc<dyn Fn() -> AppResult<String> + Send + Sync>;

pub enum HostCommand {
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
    ReadSettingsResource(taide_native_ui::settings_resources::Request),
    ThemeEdit(crate::theme_edit::Command),
    SnippetEdit(taide_native_ui::snippet_edit::Request),
    ReadSnippetCatalog(taide_native_ui::snippet_catalog::Request),
    OpenSettings {
        project: ProjectId,
        pane: PaneId,
        title: String,
    },
    OpenSettingsFolder(taide_model::system::AppDataPathKind),
    ResolveTerminalFileLinks {
        owner: crate::terminal_surface::PasteTarget,
        source: crate::terminal_tabs::MenuTarget,
        request: crate::terminal_file_links::Request,
    },
    OpenTerminalFile {
        owner: crate::terminal_surface::PasteTarget,
        source: crate::terminal_tabs::MenuTarget,
        link: crate::terminal_file_links::ResolvedFileLink,
    },
    OpenTerminalUrl {
        owner: crate::terminal_surface::PasteTarget,
        source: crate::terminal_tabs::MenuTarget,
        uri: String,
    },
    TerminalMenu {
        target: crate::terminal_tabs::MenuTarget,
        operation: crate::terminal_tabs::MenuOperation,
        title: String,
    },
    ReadTerminalClipboard(crate::terminal_surface::PasteTarget),
    ReadCompletionClipboard(crate::editor_completion::Request),
    RestartTerminal {
        tab: TabId,
        size: taide_native_terminal::Size,
        ports: crate::terminal_dispatch::EffectPorts,
    },
    ResizeTerminal {
        session: String,
        size: taide_native_terminal::Size,
    },
    AttachTerminal {
        tab: TabId,
        size: taide_native_terminal::Size,
        ports: crate::terminal_dispatch::EffectPorts,
    },
    NewTerminal {
        project: ProjectId,
        pane: PaneId,
        title: String,
    },
    SetEditorFontSize(u32),
    SetTerminalFontSize(u32),
    ToggleEditorMinimap,
    UpdateSettings(crate::settings_controls::Change),
    SetKeymapOverrides(crate::keymap::catalog::Overrides),
    ReadHwpPreview(crate::preview_hwp::Request),
    ReadPresentationPreview(crate::preview_presentation::Request),
    ReadSpreadsheetPreview(crate::preview_spreadsheet::Request),
    CopyText(String),
    CopyTerminalSelection(String),
    RevealPath(String),
    OpenInBrowser(String),
    OpenPath(String),
    ReadPreview(crate::preview::Request),
    ReadPdfPreview(crate::preview_pdf::Request),
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
    OpenProject(String),
    RestoreWatchers,
    RefreshTree {
        project: ProjectId,
        dirs: crate::events::TreeInvalidation,
    },
    OpenFileTab {
        project: ProjectId,
        pane: Option<PaneId>,
        path: String,
        preview: bool,
    },
    OpenFileToSide(taide_model::layout::OpenTabInSplitRequest),
    ListProjectFiles(ProjectId),
    OpenPaletteFile {
        project: ProjectId,
        pane: Option<PaneId>,
        path: String,
    },
    OpenProblem {
        project: ProjectId,
        path: String,
        line: u64,
        column: u64,
        viewport: eframe::egui::ViewportId,
    },
    OpenMarker(crate::editor_problems::Request),
    OpenWorkspaceSymbol(crate::workspace_symbol_host::Request),
    OpenBreadcrumbFile {
        source: crate::breadcrumbs::Source,
        path: String,
        viewport: eframe::egui::ViewportId,
    },
    RevealBreadcrumbTree(crate::breadcrumbs::Source),
    OpenDocument(String),
    Save {
        path: String,
        snapshot: SaveSnapshot,
    },
    TreeRows {
        project: ProjectId,
        offset: u32,
    },
    TreeToggle {
        project: ProjectId,
        path: String,
    },
    TreeCollapse(ProjectId),
    SetDirty {
        tab: TabId,
        dirty: bool,
    },
}

pub enum HostReply {
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
    WorkspaceSymbolOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    BreadcrumbOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    BreadcrumbTree {
        source: crate::breadcrumbs::Source,
        result: AppResult<TreeRowPage>,
    },
    AppFileWritten {
        request: crate::app_file_write::WriteRequest,
        result: AppResult<crate::app_file_write::PreparedWrite>,
    },
    ProblemOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    MarkerOpened {
        request: crate::editor_problems::Request,
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    ProjectFiles {
        project: ProjectId,
        result: AppResult<Vec<String>>,
    },
    PaletteFileOpened {
        project: ProjectId,
        result: AppResult<()>,
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
    TerminalFileLinks {
        owner: crate::terminal_surface::PasteTarget,
        request: crate::terminal_file_links::Request,
        result: AppResult<Vec<Option<String>>>,
    },
    TerminalFileOpened {
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    },
    TerminalUrlOpened {
        result: AppResult<()>,
    },
    TerminalClipboard {
        target: crate::terminal_surface::PasteTarget,
        result: AppResult<String>,
    },
    CompletionClipboard {
        request: crate::editor_completion::Request,
        result: AppResult<String>,
    },
    TerminalResized {
        session: String,
        result: AppResult<bool>,
    },
    TerminalAttached {
        tab: TabId,
        result: AppResult<String>,
    },
    HwpSourceReady(crate::preview_hwp::Request),
    HwpPreview {
        request: crate::preview_hwp::Request,
        result: Result<crate::preview_hwp::Page, crate::preview::Failure>,
    },
    SpreadsheetSourceReady(crate::preview_spreadsheet::Request),
    SpreadsheetPreview {
        request: crate::preview_spreadsheet::Request,
        result: Result<crate::preview_spreadsheet::Workbook, crate::preview::Failure>,
    },
    PresentationSourceReady(crate::preview_presentation::Request),
    PresentationPreview {
        request: crate::preview_presentation::Request,
        result: Result<crate::preview_presentation::Outline, crate::preview::Failure>,
    },
    PdfSourceReady(crate::preview_pdf::Request),
    PdfPreview {
        request: crate::preview_pdf::Request,
        result: Result<crate::preview_pdf::Page, crate::preview_pdf::Failure>,
    },
    Preview {
        request: crate::preview::Request,
        result: AppResult<crate::preview::Raster>,
    },
    CopiedText {
        result: AppResult<()>,
    },
    TerminalSelectionCopied {
        result: AppResult<()>,
    },
    SystemFinished {
        result: AppResult<()>,
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
    TreeSynced {
        project: ProjectId,
        result: AppResult<TreeRowPage>,
        errors: Vec<AppError>,
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
    Tree {
        project: ProjectId,
        offset: u32,
        result: AppResult<TreeRowPage>,
    },
    TreeToggled {
        project: ProjectId,
        path: String,
        result: AppResult<TreeRowPage>,
    },
    SettingsFailed(AppError),
    Failed(AppError),
}

pub struct HostBridge {
    state: taide_runtime::AppState,
    commands: mpsc::Sender<HostCommand>,
    replies: mpsc::Receiver<HostReply>,
    worker: JoinHandle<()>,
}

pub struct Terminals {
    pub tabs: Arc<crate::terminal_tabs::Tabs>,
    pub environment: crate::terminal_environment::Environment,
}

struct HostIntegrations {
    terminals: Option<Terminals>,
    reconcile: Option<crate::remote_preferences::Reconcile>,
}

impl HostBridge {
    pub fn connect(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> AppResult<Self> {
        Self::connect_with_clipboard(services, repaint, clipboard_writer())
    }

    pub fn connect_with_clipboard(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
    ) -> AppResult<Self> {
        Self::connect_with_ports(services, repaint, clipboard, None)
    }

    pub fn connect_with_terminals(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        terminals: Terminals,
    ) -> AppResult<Self> {
        Self::connect_with_ports(services, repaint, clipboard_writer(), Some(terminals))
    }

    pub fn connect_with_application_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        terminals: Terminals,
        ports: &crate::application_ports::Ports,
    ) -> AppResult<Self> {
        Self::connect_with_settings_ports(
            services,
            repaint,
            clipboard_writer(),
            clipboard_reader(),
            Some(terminals),
            ports.reconcile.clone(),
        )
    }

    pub fn connect_with_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        terminals: Option<Terminals>,
    ) -> AppResult<Self> {
        Self::connect_with_clipboard_ports(
            services,
            repaint,
            clipboard,
            clipboard_reader(),
            terminals,
        )
    }

    pub fn connect_with_clipboard_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        read_clipboard: ClipboardReader,
        terminals: Option<Terminals>,
    ) -> AppResult<Self> {
        Self::connect_with_integrations(
            services,
            repaint,
            clipboard,
            read_clipboard,
            HostIntegrations {
                terminals,
                reconcile: None,
            },
        )
    }

    pub fn connect_with_settings_ports(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        read_clipboard: ClipboardReader,
        terminals: Option<Terminals>,
        reconcile: crate::remote_preferences::Reconcile,
    ) -> AppResult<Self> {
        Self::connect_with_integrations(
            services,
            repaint,
            clipboard,
            read_clipboard,
            HostIntegrations {
                terminals,
                reconcile: Some(reconcile),
            },
        )
    }

    fn connect_with_integrations(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        clipboard: ClipboardWriter,
        read_clipboard: ClipboardReader,
        integrations: HostIntegrations,
    ) -> AppResult<Self> {
        let (commands, mut receiver) = mpsc::channel(HOST_COMMAND_CAPACITY);
        let (sender, replies) = mpsc::channel(HOST_COMMAND_CAPACITY);
        let tasks = services.tasks.clone();
        let state = services.state.clone();
        let worker = tasks
            .spawn_transient_handle("native-host-commands", async move {
                while let Some(command) = receiver.recv().await {
                    if services.state.is_shutting_down() {
                        break;
                    }
                    let reply = dispatch(
                        &services,
                        command,
                        &clipboard,
                        &read_clipboard,
                        &sender,
                        &repaint,
                        &integrations,
                    )
                    .await;
                    if let Some(reply) = reply
                        && sender.send(reply).await.is_err()
                    {
                        break;
                    }
                    repaint();
                }
            })
            .ok_or_else(stopped)?;
        Ok(Self {
            state,
            commands,
            replies,
            worker,
        })
    }

    pub fn submit(&self, command: HostCommand) -> AppResult<()> {
        let command = match command {
            HostCommand::SnippetEdit(request) => {
                HostCommand::SnippetEdit(request.admit(&self.state)?)
            }
            command => command,
        };
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    AppError::Internal("native host command queue is full".into())
                }
                mpsc::error::TrySendError::Closed(_) => stopped(),
            })
    }

    pub fn poll(&mut self) -> Option<HostReply> {
        self.replies.try_recv().ok()
    }

    pub fn pending_replies(&self) -> usize {
        self.replies.len()
    }

    pub fn disconnect(self) -> JoinHandle<()> {
        drop(self.commands);
        drop(self.replies);
        self.worker
    }
}

fn clipboard_writer() -> ClipboardWriter {
    let clipboard = std::sync::Mutex::new(None);
    Arc::new(move |text| {
        let unavailable = || AppError::Internal("native clipboard write failed".into());
        let mut clipboard = clipboard.lock().map_err(|_| unavailable())?;
        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new().map_err(|_| unavailable())?);
        }
        clipboard
            .as_mut()
            .ok_or_else(unavailable)?
            .set_text(text)
            .map_err(|_| unavailable())
    })
}

fn clipboard_reader() -> ClipboardReader {
    let clipboard = std::sync::Mutex::new(None);
    Arc::new(move || {
        let unavailable = || AppError::Internal("native clipboard read failed".into());
        let mut clipboard = clipboard.lock().map_err(|_| unavailable())?;
        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new().map_err(|_| unavailable())?);
        }
        match clipboard.as_mut().ok_or_else(unavailable)?.get_text() {
            Ok(text) => Ok(text),
            Err(arboard::Error::ContentNotAvailable) => Ok(String::new()),
            Err(_) => Err(unavailable()),
        }
    })
}

async fn dispatch(
    services: &Arc<AppServices>,
    command: HostCommand,
    clipboard: &ClipboardWriter,
    read_clipboard: &ClipboardReader,
    sender: &mpsc::Sender<HostReply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
    integrations: &HostIntegrations,
) -> Option<HostReply> {
    let terminals = integrations.terminals.as_ref();
    let reconcile = &integrations.reconcile;
    match command {
        HostCommand::FormatEditorInput { .. } | HostCommand::RenameEditor { .. } => None,
        HostCommand::OpenDocumentationFile(request) => Some(HostReply::DocumentationFileOpened {
            result: request.open(services).await,
        }),
        HostCommand::OpenSymbolLocation(request) => {
            let result = crate::symbol_location_host::open(services, &request).await;
            Some(HostReply::SymbolLocationOpened { request, result })
        }
        HostCommand::ReadPeekModels { request, paths } => {
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
            Some(HostReply::PeekModels { request, files })
        }
        HostCommand::WriteAppFile(request) => {
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
            Some(HostReply::AppFileWritten { request, result })
        }
        HostCommand::ReadAppFile(request) => {
            let result = request.execute(services).await;
            Some(HostReply::AppFile { request, result })
        }
        HostCommand::OpenAppFile {
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
        HostCommand::ThemeEdit(command) => {
            Some(HostReply::ThemeEdit(command.execute(services).await))
        }
        HostCommand::SnippetEdit(request) => {
            Some(HostReply::SnippetEdit(request.execute(services).await))
        }
        HostCommand::ReadSnippetCatalog(request) => {
            Some(HostReply::SnippetCatalog(request.execute(services).await))
        }
        HostCommand::OpenSettings {
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
        HostCommand::ReadSettingsCatalog(request) => {
            let state = services.state.clone();
            let work = request.clone();
            let result = services
                .tasks
                .run_blocking_result("native-settings-catalog", move || work.load(&state))
                .await;
            Some(HostReply::SettingsCatalog { request, result })
        }
        HostCommand::ReadSettingsResource(request) => {
            use taide_native_ui::settings_resources::{Kind, Reply};
            if !request.can_read(&services.state) {
                return Some(HostReply::SettingsResource(request.failed(
                    AppError::Forbidden("native application is shutting down".into()),
                )));
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
                            Ok(taide_font::service::list_families())
                        })
                        .await;
                    Reply::Fonts { request, result }
                }
                Kind::Shells => Reply::Shells {
                    request,
                    result: taide_runtime::terminal_actions::shell_profiles().await,
                },
            };
            Some(HostReply::SettingsResource(reply))
        }
        HostCommand::OpenSettingsFolder(kind) => {
            let services = services.clone();
            let result = services
                .tasks
                .clone()
                .run_blocking_result("native-settings-folder", move || {
                    if services.state.is_shutting_down() {
                        return Err(stopped());
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
        HostCommand::RefreshPresentation(request) => {
            let state = services.state.clone();
            let inputs = request.inputs.clone();
            let result = services
                .tasks
                .run_blocking_result("native-presentation-refresh", move || {
                    Ok(inputs.load(&state))
                })
                .await;
            Some(HostReply::Presentation { request, result })
        }
        HostCommand::ResolveTerminalFileLinks {
            owner,
            source,
            request,
        } => {
            let hub = terminals.map(|terminals| terminals.tabs.hub().clone());
            let services_for_task = services.clone();
            let task_owner = owner.clone();
            let task_request = request.clone();
            let result = services
                .tasks
                .run_blocking_result("native-terminal-file-links", move || {
                    authorize_terminal_link(
                        &services_for_task,
                        hub.as_deref(),
                        &task_owner,
                        &source,
                    )?;
                    if task_request.cwd().len() > crate::terminal_links::MAX_LINK_BYTES
                        || task_request.candidates().len()
                            > crate::terminal_file_links::MAX_CANDIDATES
                        || task_request.candidates().iter().any(|candidate| {
                            candidate.len() > crate::terminal_links::MAX_LINK_BYTES
                        })
                    {
                        return Err(AppError::InvalidArgument(
                            "native terminal file link request exceeds its limit".into(),
                        ));
                    }
                    let projects = services_for_task.state.projects.read().clone();
                    Ok(taide_terminal::service::resolve_link_candidates(
                        &projects,
                        task_request.cwd(),
                        task_request.candidates(),
                    ))
                })
                .await;
            Some(HostReply::TerminalFileLinks {
                owner,
                request,
                result,
            })
        }
        HostCommand::OpenTerminalFile {
            owner,
            source,
            link,
        } => {
            let result = match terminals {
                Some(terminals) => terminals.tabs.open_file_link(owner, source, link).await,
                None => Err(AppError::Internal(
                    "native terminal file link host is unavailable".into(),
                )),
            };
            Some(HostReply::TerminalFileOpened { result })
        }
        HostCommand::OpenTerminalUrl { owner, source, uri } => {
            let hub = terminals.map(|terminals| terminals.tabs.hub().clone());
            let services_for_task = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-terminal-open-url", move || {
                    authorize_terminal_link(&services_for_task, hub.as_deref(), &owner, &source)?;
                    if uri.len() > crate::terminal_links::MAX_LINK_BYTES {
                        return Err(AppError::InvalidArgument(
                            "native terminal URL exceeds its byte limit".into(),
                        ));
                    }
                    system_actions::system_open_external_url(
                        services_for_task.platform.0.as_ref(),
                        &uri,
                    )
                })
                .await;
            Some(HostReply::TerminalUrlOpened { result })
        }
        HostCommand::TerminalMenu {
            target,
            operation,
            title,
        } => {
            let tab = target.tab.clone();
            let result = match terminals {
                Some(terminals) => terminals.tabs.menu(target, operation, title).await,
                None => Err(AppError::Internal(
                    "native terminal menu host is unavailable".into(),
                )),
            };
            match result {
                Ok(Some(closed)) => Some(HostReply::Closed {
                    tab,
                    result: Ok(closed),
                }),
                Ok(None) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        HostCommand::ReadCompletionClipboard(request) => {
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
            Some(HostReply::CompletionClipboard { request, result })
        }
        HostCommand::ReadTerminalClipboard(target) => {
            let clipboard = read_clipboard.clone();
            let state = services.state.clone();
            let owner = target.clone();
            let result = services
                .tasks
                .run_blocking_result("native-terminal-read-clipboard", move || {
                    if state.is_shutting_down() || !owner.is_alive() {
                        return Err(stopped());
                    }
                    let text = clipboard()?;
                    if text.capacity() > TERMINAL_CLIPBOARD_BYTES {
                        return Err(AppError::InvalidArgument(
                            "native terminal clipboard budget exceeded".into(),
                        ));
                    }
                    Ok(text)
                })
                .await;
            Some(HostReply::TerminalClipboard { target, result })
        }
        HostCommand::RestartTerminal { tab, size, ports } => {
            let result = match terminals {
                Some(terminals) => {
                    terminals
                        .tabs
                        .restart(
                            tab.clone(),
                            size,
                            (terminals.environment)(services.clone()),
                            ports,
                        )
                        .await
                }
                None => Err(AppError::Forbidden(
                    "native terminal host is unavailable".into(),
                )),
            };
            Some(HostReply::TerminalAttached { tab, result })
        }
        HostCommand::ResizeTerminal { session, size } => {
            let result = match terminals.and_then(|terminals| terminals.tabs.hub().get(&session)) {
                Some(terminal) => terminal.resize(services, size).await,
                None => Err(AppError::NotFound(
                    "native terminal session no longer exists".into(),
                )),
            };
            Some(HostReply::TerminalResized { session, result })
        }
        HostCommand::AttachTerminal { tab, size, ports } => {
            let result = match terminals {
                Some(terminals) => {
                    terminals
                        .tabs
                        .attach(
                            tab.clone(),
                            size,
                            (terminals.environment)(services.clone()),
                            ports,
                        )
                        .await
                }
                None => Err(AppError::Forbidden(
                    "native terminal host is unavailable".into(),
                )),
            };
            Some(HostReply::TerminalAttached { tab, result })
        }
        HostCommand::SetEditorFontSize(size) => {
            update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    editor_font_size: Some(size),
                    ..Default::default()
                },
            )
            .await
        }
        HostCommand::SetTerminalFontSize(size) => {
            update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    terminal_font_size: Some(size),
                    ..Default::default()
                },
            )
            .await
        }
        HostCommand::ToggleEditorMinimap => {
            let enabled = !services.state.settings.read().editor_minimap;
            update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    editor_minimap: Some(enabled),
                    ..Default::default()
                },
            )
            .await
        }
        HostCommand::UpdateSettings(change) => {
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
            update_control_settings(services, reconcile, patch).await
        }
        HostCommand::SetKeymapOverrides(overrides) => {
            let json = match overrides.json() {
                Ok(json) => json,
                Err(error) => return Some(HostReply::SettingsFailed(error)),
            };
            update_control_settings(
                services,
                reconcile,
                taide_model::settings::SettingsPatch {
                    keymap_overrides: Some(json),
                    ..Default::default()
                },
            )
            .await
        }
        HostCommand::NewTerminal {
            project,
            pane,
            title,
        } => {
            match layout_actions::layout_open_tab(
                services.events.as_ref(),
                &services.state,
                project,
                TabKind::Terminal {
                    session_id: String::new(),
                    cwd: None,
                },
                title,
                Some(pane),
                false,
            )
            .await
            {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        HostCommand::ReadHwpPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_hwp::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::HwpSourceReady(progress))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::HwpPreview { request, result })
        }
        HostCommand::ReadSpreadsheetPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_spreadsheet::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::SpreadsheetSourceReady(progress))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::SpreadsheetPreview { request, result })
        }
        HostCommand::ReadPresentationPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_presentation::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::PresentationSourceReady(progress))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::PresentationPreview { request, result })
        }
        HostCommand::ReadPreview(request) => {
            let result = crate::preview::read(services, &request).await;
            Some(HostReply::Preview { request, result })
        }
        HostCommand::ReadPdfPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_pdf::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::PdfSourceReady(progress))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::PdfPreview { request, result })
        }
        HostCommand::OpenPath(path) => {
            let task_services = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-preview-open-path", move || {
                    if task_services.state.is_shutting_down() {
                        return Err(stopped());
                    }
                    system_actions::system_open_path(
                        &task_services.state,
                        task_services.platform.0.as_ref(),
                        &path,
                    )
                })
                .await;
            Some(HostReply::SystemFinished { result })
        }
        HostCommand::CopyText(text) => {
            let result =
                write_clipboard(services, clipboard, "native-explorer-copy-path", text).await;
            Some(HostReply::CopiedText { result })
        }
        HostCommand::CopyTerminalSelection(text) => {
            let result =
                write_clipboard(services, clipboard, "native-terminal-copy-selection", text).await;
            Some(HostReply::TerminalSelectionCopied { result })
        }
        HostCommand::RevealPath(path) => {
            let task_services = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-explorer-reveal-path", move || {
                    if task_services.state.is_shutting_down() {
                        return Err(stopped());
                    }
                    system_actions::system_reveal_path(
                        &task_services.state,
                        task_services.platform.0.as_ref(),
                        &path,
                    )
                })
                .await;
            Some(HostReply::SystemFinished { result })
        }
        HostCommand::OpenInBrowser(path) => {
            let task_services = services.clone();
            let result = services
                .tasks
                .run_blocking_result("native-explorer-open-browser", move || {
                    if task_services.state.is_shutting_down() {
                        return Err(stopped());
                    }
                    system_actions::system_open_in_browser(
                        &task_services.state,
                        task_services.platform.0.as_ref(),
                        &path,
                    )
                })
                .await;
            Some(HostReply::SystemFinished { result })
        }
        HostCommand::MirrorDraft(job) => {
            let result = crate::persistence::write_mirror(services, job.clone()).await;
            Some(HostReply::DraftMirrored { job, result })
        }
        HostCommand::SaveTracked {
            path,
            snapshot,
            epoch,
        } => {
            let result =
                crate::file_sync::save_snapshot(services, path, snapshot.clone(), Some(epoch))
                    .await;
            Some(HostReply::Saved { snapshot, result })
        }
        HostCommand::ObserveFile { path, document } => {
            let result = crate::file_sync::prepare_document(services, path.clone()).await;
            Some(HostReply::ObservedFile {
                path,
                document,
                result,
            })
        }
        HostCommand::ChooseDisk {
            path,
            document,
            revision,
            choice,
        } => {
            let result = crate::file_sync::prepare_document(services, path).await;
            Some(HostReply::DiskChosen {
                document,
                revision,
                choice,
                result,
            })
        }
        HostCommand::CleanupFileMirror {
            project,
            canonical,
            expected,
        } => match crate::file_sync::cleanup_mirror(services, project, canonical, expected).await {
            Ok(()) => None,
            Err(error) => Some(HostReply::Failed(error)),
        },
        HostCommand::NewUntitled { project, pane } => match layout_actions::layout_open_untitled(
            services.events.as_ref(),
            &services.state,
            project,
            Some(pane),
        )
        .await
        {
            Ok(_) => None,
            Err(error) => Some(HostReply::Failed(error)),
        },
        HostCommand::OpenUntitled(tab) => {
            let result = crate::untitled::prepare(services, tab.clone()).await;
            Some(HostReply::UntitledOpened { tab, result })
        }
        HostCommand::SaveUntitled {
            tab,
            destination,
            snapshot,
        } => {
            let result =
                crate::untitled::prepare_save(services, tab.clone(), destination, snapshot).await;
            Some(HostReply::UntitledSaved { tab, result })
        }
        HostCommand::CleanupUntitledMirror {
            project,
            tab,
            expected,
        } => match crate::untitled::cleanup_mirror(services, project, tab, expected).await {
            Ok(()) => None,
            Err(error) => Some(HostReply::Failed(error)),
        },
        HostCommand::SaveMissingDraft {
            tab,
            draft,
            destination,
        } => {
            let path = draft.mirror.path.clone();
            let result =
                crate::missing_draft::save(services, tab.clone(), draft, destination).await;
            Some(HostReply::MissingDraftSaved { tab, path, result })
        }
        HostCommand::CloseTab { tab, discard } => {
            let result = async {
                let closed = crate::tabs::close(services, tab.clone(), discard).await?;
                if let TabKind::Terminal { session_id, .. } = &closed.tab.kind
                    && let Some(terminals) = terminals
                {
                    terminals.tabs.close(session_id).await?;
                }
                Ok(closed)
            }
            .await;
            Some(HostReply::Closed { tab, result })
        }
        HostCommand::SaveMirroredTab(tab) => {
            let result = crate::tabs::save_mirrored(services, tab.clone()).await;
            Some(HostReply::MirrorSaved { tab, result })
        }
        HostCommand::RefreshTree { project, dirs } => {
            let dirs = match dirs {
                Some(dirs) => dirs,
                None => {
                    let root = services
                        .state
                        .projects
                        .read()
                        .get(&project)
                        .map(|project| project.root.clone());
                    let mut dirs = std::collections::BTreeSet::new();
                    if let Some(root) = root {
                        dirs.insert(root);
                    }
                    if let Ok(page) = tree_actions::tree_rows(
                        &services.state,
                        &services.tree,
                        &services.tasks,
                        project.clone(),
                        0,
                        None,
                    )
                    .await
                    {
                        dirs.extend(
                            page.rows
                                .into_iter()
                                .filter(|row| row.expanded)
                                .map(|row| row.path),
                        );
                    }
                    dirs
                }
            };
            let mut errors = Vec::new();
            for dir in dirs {
                if let Err(error) = tree_actions::tree_refresh(
                    &services.state,
                    &services.tree,
                    &services.tasks,
                    project.clone(),
                    dir,
                )
                .await
                {
                    errors.push(error);
                }
            }
            let result = tree_actions::tree_rows(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                0,
                None,
            )
            .await;
            Some(HostReply::TreeSynced {
                project,
                result,
                errors,
            })
        }
        HostCommand::OpenProject(path) => {
            match crate::projects::NativeProjects::new(services.clone())
                .open(path)
                .await
            {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        HostCommand::RestoreWatchers => {
            match crate::projects::NativeProjects::new(services.clone())
                .restore_watchers()
                .await
            {
                Ok(()) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        HostCommand::OpenProblem {
            project,
            path,
            line,
            column,
            viewport,
        } => {
            let result = open_problem(services, project, None, path, line, column, viewport).await;
            Some(HostReply::ProblemOpened { result })
        }
        HostCommand::OpenMarker(request) => {
            let active = services
                .state
                .layouts
                .read()
                .get(&request.project)
                .is_some_and(|layout| request.source_is_active(layout));
            let result = if active {
                open_problem(
                    services,
                    request.project.clone(),
                    Some(request.source_key.pane.clone()),
                    request.path.clone(),
                    request.line,
                    request.column,
                    request.viewport,
                )
                .await
            } else {
                Err(AppError::NotFound(
                    "native problem source editor is closed or inactive".into(),
                ))
            };
            Some(HostReply::MarkerOpened { request, result })
        }
        HostCommand::OpenWorkspaceSymbol(request) => Some(HostReply::WorkspaceSymbolOpened {
            result: crate::workspace_symbol_host::open(services, request).await,
        }),
        HostCommand::OpenBreadcrumbFile {
            source,
            path,
            viewport,
        } => {
            let result = crate::breadcrumb_host::open(services, &source, path, viewport).await;
            Some(HostReply::BreadcrumbOpened { result })
        }
        HostCommand::RevealBreadcrumbTree(source) => {
            let active = services
                .state
                .layouts
                .read()
                .get(&source.project)
                .is_some_and(|layout| source.is_active(layout));
            let result = if active {
                tree_actions::tree_reveal(
                    &services.state,
                    &services.tree,
                    &services.tasks,
                    source.project.clone(),
                    source.path.clone(),
                )
                .await
            } else {
                Err(AppError::NotFound(
                    "native breadcrumb source is closed or inactive".into(),
                ))
            };
            Some(HostReply::BreadcrumbTree { source, result })
        }
        HostCommand::OpenFileTab {
            project,
            pane,
            path,
            preview,
        } => open_file_tab(services, project, pane, path, preview)
            .await
            .err()
            .map(HostReply::Failed),
        HostCommand::ListProjectFiles(project) => {
            let result = taide_runtime::search_actions::search_list_files(
                &services.state,
                &services.tasks,
                project.clone(),
            )
            .await;
            Some(HostReply::ProjectFiles { project, result })
        }
        HostCommand::OpenPaletteFile {
            project,
            pane,
            path,
        } => {
            let result = open_file_tab(services, project.clone(), pane, path, true).await;
            Some(HostReply::PaletteFileOpened { project, result })
        }
        HostCommand::OpenFileToSide(request) => {
            match layout_actions::layout_open_tab_in_split(
                services.events.as_ref(),
                &services.state,
                request,
            )
            .await
            {
                Ok(_) => None,
                Err(error) => Some(HostReply::Failed(error)),
            }
        }
        HostCommand::OpenDocument(path) => {
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
                    Ok(Some(prepared)) => return Some(HostReply::MissingDraft { path, prepared }),
                    Ok(None) => Err(error),
                    Err(missing_error) => Err(missing_error),
                },
            };
            Some(HostReply::Opened {
                path,
                project,
                result,
            })
        }
        HostCommand::Save { path, snapshot } | HostCommand::AutoSave { path, snapshot } => {
            let result =
                crate::file_sync::save_snapshot(services, path, snapshot.clone(), None).await;
            Some(HostReply::Saved { snapshot, result })
        }
        HostCommand::TreeRows { project, offset } => {
            let result = tree_actions::tree_rows(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                offset,
                None,
            )
            .await;
            Some(HostReply::Tree {
                project,
                offset,
                result,
            })
        }
        HostCommand::TreeToggle { project, path } => {
            let result = tree_actions::tree_toggle(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
                path.clone(),
            )
            .await;
            Some(HostReply::TreeToggled {
                project,
                path,
                result,
            })
        }
        HostCommand::TreeCollapse(project) => {
            let result = tree_actions::tree_collapse_all(
                &services.state,
                &services.tree,
                &services.tasks,
                project.clone(),
            )
            .await;
            Some(HostReply::Tree {
                project,
                offset: 0,
                result,
            })
        }
        HostCommand::SetDirty { tab, dirty } => {
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

async fn open_file_tab(
    services: &AppServices,
    project: ProjectId,
    pane: Option<PaneId>,
    path: String,
    preview: bool,
) -> AppResult<()> {
    let title = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_owned();
    layout_actions::layout_open_tab(
        services.events.as_ref(),
        &services.state,
        project,
        TabKind::File { path },
        title,
        pane,
        preview,
    )
    .await
    .map(drop)
}

async fn write_clipboard(
    services: &AppServices,
    clipboard: &ClipboardWriter,
    task: &'static str,
    text: String,
) -> AppResult<()> {
    let clipboard = clipboard.clone();
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result(task, move || {
            if state.is_shutting_down() {
                return Err(stopped());
            }
            clipboard(&text)
        })
        .await
}

pub(crate) fn authorize_terminal_link(
    services: &AppServices,
    hub: Option<&crate::terminal_host::Hub>,
    owner: &crate::terminal_surface::PasteTarget,
    source: &crate::terminal_tabs::MenuTarget,
) -> AppResult<()> {
    if services.state.is_shutting_down() || !owner.is_alive() || !owner.matches(source) {
        return Err(stopped());
    }
    let session = hub
        .and_then(|hub| hub.get(&source.session))
        .ok_or_else(stopped)?;
    if session.metadata().project_id() != &source.project
        || !session
            .snapshot(|snapshot| snapshot.phase == taide_native_terminal::session::Phase::Running)?
        || !services.state.projects.read().contains_key(&source.project)
        || !services
            .state
            .layouts
            .read()
            .get(&source.project)
            .is_some_and(|layout| crate::terminal_tabs::menu_tab(layout, source).is_some())
    {
        return Err(stopped());
    }
    Ok(())
}

async fn update_control_settings(
    services: &Arc<AppServices>,
    reconcile: &Option<crate::remote_preferences::Reconcile>,
    patch: taide_model::settings::SettingsPatch,
) -> Option<HostReply> {
    match taide_runtime::settings_actions::settings_update(
        &services.state,
        patch,
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
    }
}

fn stopped() -> AppError {
    AppError::Forbidden("native host command admission is closed".into())
}

pub async fn flush_layouts(services: &AppServices) -> AppResult<()> {
    let state = services.state.clone();
    let operation = services
        .tasks
        .begin_operation("native-close-layout-flush")
        .ok_or_else(stopped)?;
    let guard = state.begin_owned_mutation().await;
    services
        .tasks
        .run_blocking_result("native-close-layout-flush", move || {
            let _operation = operation;
            let _guard = guard;
            let dirty: Vec<_> = state.dirty_layouts.read().iter().cloned().collect();
            for project in dirty {
                let layout = state.layouts.read().get(&project).cloned().ok_or_else(|| {
                    AppError::Internal("native dirty layout snapshot is missing".into())
                })?;
                taide_layout::service::save_layout(&state.paths, &project, &layout)?;
                state.dirty_layouts.write().remove(&project);
            }
            Ok(())
        })
        .await
}

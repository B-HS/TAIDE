use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Ui};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::file::REFUSED_FILE_BYTES;
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_model::locale::ResolvedLocale;
use taide_model::tree::TreeRowPage;
use taide_native_editor::document::DiskChoice;
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::conflict_banner::{BannerAction, BannerAppearance, BannerVariant};
use taide_native_ui::controller::ShellController;
use taide_native_ui::editor_surface::{EditorRequest, EditorTokens, FoldControl, NativeEditor};
use taide_native_ui::shell::{NativeShell, ShellSurfaces, WindowScope};
use taide_runtime::{AppServices, AppState, EventSink, ExitDrain, TaskSupervisor, file_actions};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::bootstrap;
use crate::command_registry::{CommandContext, DocumentEdit, FoldCommand};
use crate::host::{HostBridge, HostCommand, HostReply};
use crate::presentation;

const WINDOW_LABEL: &str = "native-main";
const APPLICATION_NAME: &str = "TAIDE";
const MAX_DOCUMENTS: usize = 256;
const MAX_VIEWS: usize = 512;
const MAX_UNDO_GROUPS: usize = 200;
const MAX_TERMINAL_SESSIONS: usize = 64;
const TERMINAL_FRAME_BYTES: usize = 4 * 1024 * 1024;
const TERMINAL_QUEUE_COUNT: usize = 64;
const TERMINAL_FRAME_VISITS: usize = 1_000_000;
const TERMINAL_WRITER_BYTES: usize = 1024 * 1024;
const WEB_CONNECTIONS: usize = 16;
const WEB_HEADER_TIMEOUT: Duration = Duration::from_secs(5);
const WEB_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const WEB_HELPER_TIMEOUT: Duration = Duration::from_secs(30);
const UNAVAILABLE_TAB_FONT_SIZE: f32 = 14.0;
const UNAVAILABLE_TAB_TEXT_OPACITY: f32 = 0.6;

struct PaintSink {
    context: egui::Context,
    trees: Arc<crate::events::TreeChanges>,
    file_indexes: Arc<crate::command_palette::FileIndexChanges>,
    presentation: Arc<crate::presentation_refresh::Changes>,
}

impl EventSink for PaintSink {
    fn publish(&self, event: AppEvent) {
        self.trees.record(&event);
        self.file_indexes.record(&event);
        self.presentation.record(&event);
        self.context.request_repaint();
    }
}

struct FileDocument {
    id: DocumentId,
    project: Option<ProjectId>,
}

struct UntitledDocument {
    id: DocumentId,
    project: ProjectId,
}

enum TabClosePhase {
    Confirm,
    Saving(Option<DocumentId>),
    Ready { discard: bool },
    Closing,
}

struct PendingTabClose {
    tab: Tab,
    phase: TabClosePhase,
    approved_discard: Option<(DocumentId, u64)>,
    batch: Option<crate::tab_close_batch::Batch>,
    automatic_choice: Option<crate::close_dialog::CloseChoice>,
}

impl PendingTabClose {
    fn failed_batch_prepare(&self) -> bool {
        matches!(self.phase, TabClosePhase::Confirm)
            && self.automatic_choice.is_none()
            && self
                .batch
                .as_ref()
                .is_some_and(|batch| batch.is_preparing())
    }
}

struct PendingFormatSave {
    path: String,
    auto_save: bool,
    epoch: crate::persistence::DraftEpoch,
}

pub struct NativeApplication {
    runtime: Runtime,
    services: Arc<AppServices>,
    application_ports: Arc<crate::application_ports::Ports>,
    controller: ShellController,
    bridge: Option<HostBridge>,
    terminals: Arc<crate::terminal_tabs::Tabs>,
    terminal_views: crate::terminal_surface::Views,
    terminal_appearance: crate::terminal_surface::Appearance,
    terminal_fonts: crate::terminal_fonts::Loader,
    web_bridge: Option<crate::preview_web_host::Bridge>,
    helper_executable: PathBuf,
    web_previews: crate::preview_web_cache::Cache,
    web_views: crate::preview_web_view::Views,
    lsp: Option<crate::lsp::LspBridge>,
    pending_format_saves: HashMap<DocumentId, PendingFormatSave>,
    workspace_activity: Option<crate::workspace_activity::Activity>,
    workspace_suspended: HashSet<DocumentId>,
    deleted_documents: HashSet<DocumentId>,
    resumed_save_epochs: HashMap<DocumentId, crate::persistence::DraftEpoch>,
    lsp_diagnostics: crate::diagnostics::Store,
    lsp_status_appearance: crate::lsp::status::Appearance,
    status_editor_appearance: crate::status_editor::Appearance,
    status_ide_appearance: crate::status_ide::Appearance,
    status_ide_icons: crate::status_ide::Icons,
    system_usage: crate::system_usage::Sampler,
    system_usage_appearance: crate::system_usage_view::Appearance,
    system_usage_icon: crate::system_usage_view::Icon,
    status_chord_appearance: crate::status_chord::Appearance,
    problems: crate::problems::Views,
    tooltips: crate::tooltips::Provider,
    tooltip_appearance: crate::tooltips::Appearance,
    problems_appearance: crate::problems::Appearance,
    explorer_appearance: crate::explorer::Appearance,
    explorer_icons: crate::problems_icons::Icons,
    shell: NativeShell,
    zen_fullscreen_state: crate::zen::Fullscreen,
    keybindings: crate::keybinding_editor::Editor,
    palette: crate::command_palette::Palette,
    palette_files: crate::command_palette::FileIndexes,
    palette_file_changes: Arc<crate::command_palette::FileIndexChanges>,
    palette_commands: Vec<String>,
    settings_views: crate::settings_view::Views,
    app_file_views: crate::app_file_views::Views,
    settings_appearance: crate::settings_view::Appearance,
    resolved_theme: taide_model::theme::ResolvedTheme,
    preview_theme: Option<taide_model::theme::ResolvedTheme>,
    toasts: crate::toast::Toasts,
    toast_theme: taide_model::theme::ThemeType,
    motion_preference: crate::motion_preference::Preference,
    presentation_changes: Arc<crate::presentation_refresh::Changes>,
    presentation_refresh: crate::presentation_refresh::Refresh,
    presentation_system_language: String,
    editor: NativeEditor,
    editor_find: HashMap<ViewId, taide_native_ui::editor_find::EditorFind>,
    find_history: taide_native_ui::editor_find_widget::FindHistory,
    find_appearance: taide_native_ui::editor_find_widget::FindAppearance,
    editor_display_colors: taide_native_ui::editor_display::EditorDisplayColors,
    editor_bracket_colors: taide_native_ui::editor_brackets::EditorBracketColors,
    editor_sticky_colors: taide_native_ui::editor_sticky_scroll::EditorStickyColors,
    editor_sticky_scroll: taide_native_ui::editor_sticky_scroll::StickySetting,
    editor_syntax: crate::editor_syntax::EditorSyntax,
    editor_keymap_targets: HashMap<(egui::ViewportId, egui::Id), (ViewId, u64)>,
    banner_appearance: BannerAppearance,
    restore_notices: HashMap<TabId, BannerVariant>,
    loading_file_tabs: HashMap<String, TabId>,
    pending_disk_choice: Option<DocumentId>,
    pending_file_observations: HashMap<DocumentId, String>,
    observing_files: HashSet<DocumentId>,
    locale: ResolvedLocale,
    store: EditorStore,
    files: HashMap<String, FileDocument>,
    open_with: crate::open_with::Registry,
    previews: crate::preview::Cache,
    hwp_previews: crate::preview_hwp::Cache,
    pdf_previews: crate::preview_pdf::Cache,
    pdf_appearance: crate::preview_pdf_surface::Appearance,
    presentation_previews: crate::preview_presentation_cache::Cache,
    presentation_appearance: crate::preview_presentation_surface::Appearance,
    spreadsheet_previews: crate::preview_spreadsheet_cache::Cache,
    spreadsheet_appearance: crate::preview_spreadsheet_surface::Appearance,
    untitled: HashMap<TabId, UntitledDocument>,
    untitled_loading: HashSet<TabId>,
    untitled_failed: HashMap<TabId, AppError>,
    saving_untitled: Option<TabId>,
    missing_drafts: HashMap<String, crate::missing_draft::MissingDraft>,
    saving_missing_draft: Option<TabId>,
    loading: HashSet<String>,
    failed: HashMap<String, AppError>,
    trees: HashMap<ProjectId, TreeRowPage>,
    explorers: HashMap<ProjectId, crate::explorer::Explorer>,
    explorer_clipboards: crate::explorer_clipboard_owners::ClipboardOwners,
    pending_entry_delete: Option<crate::delete_dialog::Confirmation>,
    deleting_entry: bool,
    tree_loading: HashSet<ProjectId>,
    tree_changes: Arc<crate::events::TreeChanges>,
    pending_dirty: HashMap<TabId, bool>,
    persistence: crate::persistence::Persistence,
    last_edited_views: HashMap<DocumentId, ViewId>,
    pending_tab_close: Option<PendingTabClose>,
    focused: Option<(PaneId, TabId)>,
    reveals: crate::editor_reveal::Reveals,
    document_edits: Vec<(TabId, DocumentEdit)>,
    fold_commands: Vec<(TabId, FoldCommand)>,
    status: Option<String>,
    closing: Option<oneshot::Receiver<AppResult<()>>>,
    close_request_active: bool,
    is_exit_ready: bool,
}

impl NativeApplication {
    pub fn new(
        context: &eframe::CreationContext<'_>,
        runtime: Runtime,
        state: AppState,
        tasks: TaskSupervisor,
        helper_executable: PathBuf,
        mut warnings: Vec<String>,
    ) -> AppResult<Self> {
        let egui = context.egui_ctx.clone();
        let repaint_context = egui.clone();
        let repaint: Arc<dyn Fn() + Send + Sync> =
            Arc::new(move || repaint_context.request_repaint());
        let tree_changes = Arc::new(crate::events::TreeChanges::default());
        let palette_file_changes = Arc::new(crate::command_palette::FileIndexChanges::default());
        let presentation_changes = Arc::new(crate::presentation_refresh::Changes::default());
        let (assembly, connection) = runtime.block_on(bootstrap::connect_shell(
            state.clone(),
            tasks.clone(),
            Arc::new(PaintSink {
                context: egui.clone(),
                trees: tree_changes.clone(),
                file_indexes: palette_file_changes.clone(),
                presentation: presentation_changes.clone(),
            }),
            repaint.clone(),
        ))?;
        let services = assembly.services;
        let system_theme = match egui.system_theme() {
            Some(egui::Theme::Light) => "light",
            Some(egui::Theme::Dark) | None => "dark",
        };
        let system_language = sys_locale::get_locale().unwrap_or_default();
        let presentation_inputs = crate::presentation_refresh::Inputs::new(
            state.settings.read().clone(),
            system_theme,
            &system_language,
        );
        let presentation_revision = presentation_changes.revision();
        let loaded_inputs = presentation_inputs.clone();
        let theme_state = state.clone();
        let font_families = crate::editor_fonts::Families::new(&state.settings.read());
        let requested_fonts = font_families.clone();
        let (resolved, fonts) = runtime.block_on(tasks.run_blocking_result(
            "native-presentation-load",
            move || {
                Ok((
                    loaded_inputs.load(&theme_state),
                    crate::terminal_fonts::load(&requested_fonts)?,
                ))
            },
        ))?;
        let theme = resolved.theme?;
        let locale = resolved.locale?;
        egui.set_fonts(fonts.definitions);
        warnings.extend(fonts.warnings);
        for warning in &warnings {
            log::warn!("{warning}");
        }
        let appearances =
            crate::presentation_refresh::Appearances::new(&theme, &presentation_inputs.settings)?;
        presentation::apply_visuals(&egui, &appearances.visuals);
        let shell = NativeShell {
            scope: WindowScope::Main,
            colors: appearances.shell,
            has_title_bar: cfg!(target_os = "macos"),
        };
        let editor = NativeEditor {
            appearance: appearances.editor,
        };
        let banner_appearance = appearances.banner;
        let pdf_appearance = appearances.pdf;
        let presentation_appearance = appearances.presentation;
        let spreadsheet_appearance = appearances.spreadsheet;
        let terminal_appearance = appearances.terminal;
        let mut terminal_views = crate::terminal_surface::Views::default();
        terminal_views.set_palette(&terminal_appearance)?;
        let terminals = Arc::new(crate::terminal_tabs::Tabs::new(
            services.clone(),
            crate::terminal_host::Limits {
                sessions: MAX_TERMINAL_SESSIONS,
                core: Default::default(),
                frames: crate::terminal_frames::Limits {
                    bytes: TERMINAL_FRAME_BYTES,
                    count: TERMINAL_QUEUE_COUNT,
                    visits: TERMINAL_FRAME_VISITS,
                },
                writer: crate::terminal_writer::Limits {
                    bytes: TERMINAL_WRITER_BYTES,
                    count: TERMINAL_QUEUE_COUNT,
                },
            },
        )?);
        let application_ports = Arc::new(crate::application_ports::Ports::with_loading_assets(
            &services,
            assembly.git_events,
            crate::remote_terminal::Ports {
                terminals: terminals.hub().clone(),
                environment: crate::terminal_environment::provider(),
                history: Arc::new(|services, _| {
                    services.state.settings.read().terminal_scrollback as usize
                }),
                effects: terminal_views.remote_effects(&egui),
            },
            Arc::new(crate::remote_assets::Catalog::packaged(&helper_executable)?),
            taide_model::app::AppInfo {
                name: APPLICATION_NAME.into(),
                version: env!("CARGO_PKG_VERSION").into(),
                platform: std::env::consts::OS.into(),
                arch: std::env::consts::ARCH.into(),
            },
        ));
        let host = HostBridge::connect_with_application_ports(
            services.clone(),
            repaint.clone(),
            crate::host::Terminals {
                tabs: terminals.clone(),
                environment: crate::terminal_environment::provider(),
            },
            &application_ports,
        )?;
        let web_bridge = connect_web(
            services.clone(),
            &egui,
            helper_executable.clone(),
            &pdf_appearance,
        )?;
        let editor_syntax =
            crate::editor_syntax::EditorSyntax::connect(&services.tasks, repaint.clone())?;
        let lsp = crate::lsp::LspBridge::connect(
            services.clone(),
            std::env::var_os("PATH").unwrap_or_default(),
            repaint,
        )?;
        let mut usage_labels = crate::remote_utilities::domain_label_providers();
        usage_labels.push(lsp.usage_labels());
        let system_usage = crate::system_usage::Sampler::new(crate::remote_utilities::Ports::new(
            &services,
            usage_labels,
        ));
        host.submit(HostCommand::RestoreWatchers)?;
        let bridge = Some(host);
        let editor_sticky_scroll = taide_native_ui::editor_sticky_scroll::StickySetting::new(
            services.state.settings.read().editor_sticky_scroll_enabled,
        );
        let mut store = EditorStore::new(EditorLimits {
            max_documents: MAX_DOCUMENTS,
            max_views: MAX_VIEWS,
            max_undo_groups: MAX_UNDO_GROUPS,
            max_document_bytes: REFUSED_FILE_BYTES as usize,
        })
        .map_err(editor_error)?;
        store.track_document_disposals();
        let zen_fullscreen_state = crate::zen::Fullscreen::new(
            state.session.read().window_chrome.zen,
            state.settings.read().zen_fullscreen,
        );
        let collation_locale = sys_locale::get_locale()
            .unwrap_or_else(|| "en-US".into())
            .replace('_', "-");
        let keybindings = crate::keybinding_editor::Editor::with_appearance(
            appearances.keybindings,
            &collation_locale,
            cfg!(target_os = "macos"),
        )?;
        let palette = crate::command_palette::Palette::with_appearance(
            appearances.palette,
            &collation_locale,
            cfg!(target_os = "macos"),
        )?;
        let toasts = crate::toast::Toasts::with_actions()?;
        let toast_theme = theme.theme_type;
        let tooltips = crate::tooltips::Provider::default();
        let mut settings_views = crate::settings_view::Views::default();
        settings_views.set_font_painter(crate::font_preview::Renderer::new(services.tasks.clone()));
        let application = Self {
            runtime,
            services,
            application_ports,
            controller: connection.controller,
            bridge,
            terminals,
            terminal_views,
            terminal_appearance,
            terminal_fonts: crate::terminal_fonts::Loader::new(font_families),
            web_bridge: Some(web_bridge),
            helper_executable,
            web_previews: crate::preview_web_cache::Cache::default(),
            web_views: crate::preview_web_view::Views::default(),
            lsp: Some(lsp),
            pending_format_saves: HashMap::new(),
            workspace_activity: None,
            workspace_suspended: HashSet::new(),
            deleted_documents: HashSet::new(),
            resumed_save_epochs: HashMap::new(),
            lsp_diagnostics: crate::diagnostics::Store::default(),
            lsp_status_appearance: appearances.lsp_status,
            status_editor_appearance: appearances.status_editor,
            status_ide_appearance: appearances.status_ide,
            status_ide_icons: crate::status_ide::Icons::with_tooltips(tooltips.clone())?,
            system_usage,
            system_usage_appearance: appearances.system_usage,
            system_usage_icon: crate::system_usage_view::Icon::new()?,
            status_chord_appearance: appearances.status_chord,
            problems: crate::problems::Views::with_tooltips(&collation_locale, tooltips.clone())?,
            tooltips,
            tooltip_appearance: appearances.tooltip,
            problems_appearance: appearances.problems,
            explorer_appearance: appearances.explorer,
            explorer_icons: crate::problems_icons::Icons::new()?,
            shell,
            zen_fullscreen_state,
            keybindings,
            palette,
            palette_files: Default::default(),
            palette_file_changes,
            palette_commands: Vec::new(),
            settings_views,
            app_file_views: crate::app_file_views::Views::default(),
            settings_appearance: appearances.settings,
            resolved_theme: theme,
            preview_theme: None,
            toasts,
            toast_theme,
            motion_preference: crate::motion_preference::Preference::new(egui.clone()),
            presentation_changes,
            presentation_refresh: crate::presentation_refresh::Refresh::new(
                presentation_inputs,
                presentation_revision,
            ),
            presentation_system_language: system_language,
            editor,
            editor_find: HashMap::new(),
            find_history: Default::default(),
            find_appearance: appearances.find,
            editor_display_colors: appearances.editor_display,
            editor_bracket_colors: appearances.editor_brackets,
            editor_sticky_colors: appearances.editor_sticky,
            editor_sticky_scroll,
            editor_syntax,
            editor_keymap_targets: HashMap::new(),
            banner_appearance,
            restore_notices: HashMap::new(),
            loading_file_tabs: HashMap::new(),
            pending_disk_choice: None,
            pending_file_observations: HashMap::new(),
            observing_files: HashSet::new(),
            locale,
            store,
            files: HashMap::new(),
            open_with: crate::open_with::Registry::default(),
            previews: crate::preview::Cache::default(),
            hwp_previews: crate::preview_hwp::Cache::default(),
            pdf_previews: crate::preview_pdf::Cache::default(),
            pdf_appearance,
            presentation_previews: crate::preview_presentation_cache::Cache::default(),
            presentation_appearance,
            spreadsheet_previews: crate::preview_spreadsheet_cache::Cache::default(),
            spreadsheet_appearance,
            untitled: HashMap::new(),
            untitled_loading: HashSet::new(),
            untitled_failed: HashMap::new(),
            saving_untitled: None,
            missing_drafts: HashMap::new(),
            saving_missing_draft: None,
            loading: HashSet::new(),
            failed: HashMap::new(),
            trees: HashMap::new(),
            explorers: HashMap::new(),
            explorer_clipboards: crate::explorer_clipboard_owners::ClipboardOwners::new(
                egui.viewport_id(),
            ),
            pending_entry_delete: None,
            deleting_entry: false,
            tree_loading: HashSet::new(),
            tree_changes,
            pending_dirty: HashMap::new(),
            persistence: crate::persistence::Persistence::default(),
            last_edited_views: HashMap::new(),
            pending_tab_close: None,
            focused: None,
            reveals: Default::default(),
            document_edits: Vec::new(),
            fold_commands: Vec::new(),
            status: None,
            closing: None,
            close_request_active: false,
            is_exit_ready: false,
        };
        application.runtime.block_on(
            application
                .application_ports
                .start(application.services.clone()),
        );
        crate::application_ports::start_agent_poll(application.services.clone());
        Ok(application)
    }

    fn poll(&mut self, context: &egui::Context) {
        while let Some(reply) = self
            .web_bridge
            .as_mut()
            .and_then(crate::preview_web_host::Bridge::poll)
        {
            self.reconcile_previews();
            match reply {
                crate::preview_web_host::Reply::SourceReady(request) => {
                    self.web_previews.source_ready(&request)
                }
                crate::preview_web_host::Reply::Prepared { request, result } => {
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.web_previews.cancelled(&request);
                    } else {
                        let other_bytes = self
                            .previews
                            .retained_bytes()
                            .saturating_add(self.hwp_previews.retained_bytes())
                            .saturating_add(self.pdf_previews.retained_bytes())
                            .saturating_add(self.presentation_previews.retained_bytes())
                            .saturating_add(self.spreadsheet_previews.retained_bytes());
                        self.web_previews.accept(request, result, other_bytes);
                    }
                }
            }
        }
        if let Some(error) = self.controller.take_error() {
            self.report(&error);
        }
        self.settings_views
            .observe_theme_revision(self.presentation_changes.theme_revision());
        self.app_file_views
            .observe_settings_revision(self.presentation_changes.settings_revision());
        if let Err(error) = self
            .app_file_views
            .reconcile(&self.services.state, &mut self.store)
        {
            self.status = Some(editor_error(error).to_string());
        }
        while let Some(reply) = self.bridge.as_mut().and_then(HostBridge::poll) {
            match reply {
                HostReply::AppFileWritten { request, result } => {
                    if self.closing.is_none()
                        && let Some(result) =
                            self.app_file_views
                                .accept_write(&request, result, &mut self.store)
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
                HostReply::AppFile { request, result } => {
                    if self.closing.is_none() {
                        self.app_file_views
                            .accept(&request, result, &mut self.store);
                    }
                }
                HostReply::ThemeEdit(reply) => {
                    if self.closing.is_none()
                        && reply.owner().is_active(&self.services.state)
                        && let Some(error) = self.settings_views.accept_theme(reply)
                    {
                        self.toasts.ipc_error(&self.locale, &error, Instant::now());
                    }
                }
                HostReply::SnippetEdit(reply) => {
                    if self.closing.is_none() {
                        self.settings_views.accept_snippet(reply);
                    }
                }
                HostReply::SnippetCatalog(reply) => {
                    if self.closing.is_none() {
                        self.settings_views
                            .accept_snippet_catalog(reply, Instant::now());
                    }
                }
                HostReply::SettingsCatalog { request, result } => {
                    if self.closing.is_none() && request.is_active(&self.services.state) {
                        self.settings_views.accept(&request, result);
                    }
                }
                HostReply::SettingsResource(reply) => {
                    if self.closing.is_none() && reply.request().can_read(&self.services.state) {
                        self.settings_views.accept_resource(reply);
                    }
                }
                HostReply::Presentation { request, result } => {
                    let inputs = self.presentation_inputs(context);
                    if self.presentation_refresh.finish(
                        &request,
                        &inputs,
                        self.presentation_changes.revision(),
                    ) && self.closing.is_none()
                        && !self.is_exit_ready
                        && !self.services.state.is_shutting_down()
                    {
                        match result {
                            Ok(resolved) => {
                                match resolved.theme.and_then(|theme| {
                                    let appearances =
                                        crate::presentation_refresh::Appearances::new(
                                            &theme,
                                            &inputs.settings,
                                        )?;
                                    self.terminal_views.set_palette(&appearances.terminal)?;
                                    Ok((appearances, theme))
                                }) {
                                    Ok((appearances, theme)) => {
                                        self.resolved_theme = theme;
                                        self.preview_theme = None;
                                        presentation::apply_visuals(context, &appearances.visuals);
                                        self.shell.colors = appearances.shell;
                                        self.editor.appearance = appearances.editor;
                                        self.find_appearance = appearances.find;
                                        self.editor_display_colors = appearances.editor_display;
                                        self.editor_bracket_colors = appearances.editor_brackets;
                                        self.editor_sticky_colors = appearances.editor_sticky;
                                        self.banner_appearance = appearances.banner;
                                        self.lsp_status_appearance = appearances.lsp_status;
                                        self.status_editor_appearance = appearances.status_editor;
                                        self.status_ide_appearance = appearances.status_ide;
                                        self.system_usage_appearance = appearances.system_usage;
                                        self.status_chord_appearance = appearances.status_chord;
                                        self.problems_appearance = appearances.problems;
                                        self.explorer_appearance = appearances.explorer;
                                        self.tooltip_appearance = appearances.tooltip;
                                        self.terminal_appearance = appearances.terminal;
                                        self.keybindings.set_appearance(appearances.keybindings);
                                        self.palette.set_appearance(appearances.palette);
                                        self.settings_appearance = appearances.settings;
                                        self.pdf_appearance = appearances.pdf;
                                        self.presentation_appearance = appearances.presentation;
                                        self.spreadsheet_appearance = appearances.spreadsheet;
                                        self.toast_theme = appearances.toast;
                                    }
                                    Err(error) => self.status = Some(error.to_string()),
                                }
                                match resolved.locale {
                                    Ok(locale) => self.locale = locale,
                                    Err(error) => self.status = Some(error.to_string()),
                                }
                            }
                            Err(error) => self.status = Some(error.to_string()),
                        }
                    }
                }
                HostReply::TerminalFileLinks {
                    owner,
                    request,
                    result,
                } => {
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        self.terminal_views
                            .accept_file_links(&owner, &request, result);
                    }
                }
                HostReply::TerminalFileOpened { result } | HostReply::ProblemOpened { result } => {
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        match result {
                            Ok(opened) => {
                                if let Some(layout) = self
                                    .services
                                    .state
                                    .layouts
                                    .read()
                                    .get(&opened.project)
                                    .cloned()
                                {
                                    self.controller.apply_layouts(HashMap::from([(
                                        opened.project.clone(),
                                        layout,
                                    )]));
                                }
                                self.reveals.queue(
                                    &opened,
                                    &self.services.state.layouts.read(),
                                    Instant::now(),
                                );
                                context.request_repaint_after(crate::editor_reveal::REVEAL_TTL);
                            }
                            Err(error) => self.report(&error),
                        }
                    }
                }
                HostReply::TerminalClipboard { target, result } => {
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        match result {
                            Ok(text) => {
                                if let Err(error) = self.terminal_views.accept_paste(
                                    &target,
                                    &text,
                                    self.terminals.hub(),
                                    &self.services,
                                ) {
                                    log::warn!("native terminal paste failed: {:?}", error.kind());
                                }
                            }
                            Err(error) if target.is_alive() => {
                                log::warn!(
                                    "native terminal clipboard read failed: {:?}",
                                    error.kind()
                                )
                            }
                            Err(_) => {}
                        }
                    }
                }
                HostReply::TerminalAttached { tab, result } => {
                    self.terminal_views.attached(
                        tab,
                        &result,
                        self.terminals.hub(),
                        &self.services,
                    );
                    if let Err(error) = result {
                        self.report(&error);
                    }
                }
                HostReply::TerminalResized { session, result } => {
                    self.terminal_views.resized(&session);
                    if let Err(error) = result {
                        log::warn!("native terminal resize failed: {error}");
                    }
                }
                HostReply::HwpSourceReady(request) => {
                    self.reconcile_previews();
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        self.hwp_previews.source_ready(&request);
                    }
                }
                HostReply::HwpPreview { request, result } => {
                    self.reconcile_previews();
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.hwp_previews.cancelled(&request);
                    } else {
                        self.hwp_previews.accept(
                            context,
                            request,
                            result,
                            self.previews
                                .retained_bytes()
                                .saturating_add(self.pdf_previews.retained_bytes())
                                .saturating_add(self.presentation_previews.retained_bytes())
                                .saturating_add(self.spreadsheet_previews.retained_bytes())
                                .saturating_add(self.web_previews.retained_bytes()),
                        );
                    }
                }
                HostReply::SpreadsheetSourceReady(request) => {
                    self.reconcile_previews();
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        self.spreadsheet_previews.source_ready(&request);
                    }
                }
                HostReply::SpreadsheetPreview { request, result } => {
                    self.reconcile_previews();
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.spreadsheet_previews.cancelled(&request);
                    } else {
                        self.spreadsheet_previews.accept(
                            request,
                            result,
                            self.previews
                                .retained_bytes()
                                .saturating_add(self.pdf_previews.retained_bytes())
                                .saturating_add(self.presentation_previews.retained_bytes())
                                .saturating_add(self.hwp_previews.retained_bytes())
                                .saturating_add(self.web_previews.retained_bytes()),
                        );
                    }
                }
                HostReply::PresentationSourceReady(request) => {
                    self.reconcile_previews();
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        self.presentation_previews.source_ready(&request);
                    }
                }
                HostReply::PresentationPreview { request, result } => {
                    self.reconcile_previews();
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.presentation_previews.cancelled(&request);
                    } else {
                        self.presentation_previews.accept(
                            request,
                            result,
                            self.previews
                                .retained_bytes()
                                .saturating_add(self.pdf_previews.retained_bytes())
                                .saturating_add(self.spreadsheet_previews.retained_bytes())
                                .saturating_add(self.hwp_previews.retained_bytes())
                                .saturating_add(self.web_previews.retained_bytes()),
                        );
                    }
                }
                HostReply::PdfSourceReady(request) => {
                    self.reconcile_previews();
                    if self.closing.is_none() && !self.services.state.is_shutting_down() {
                        self.pdf_previews.source_ready(&request);
                    }
                }
                HostReply::PdfPreview { request, result } => {
                    self.reconcile_previews();
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.pdf_previews.cancelled(&request);
                    } else {
                        self.pdf_previews.accept(
                            context,
                            request,
                            result,
                            self.previews
                                .retained_bytes()
                                .saturating_add(self.presentation_previews.retained_bytes())
                                .saturating_add(self.spreadsheet_previews.retained_bytes())
                                .saturating_add(self.hwp_previews.retained_bytes())
                                .saturating_add(self.web_previews.retained_bytes()),
                        );
                    }
                }
                HostReply::Preview { request, result } => {
                    self.reconcile_previews();
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.previews.cancelled(&request);
                    } else {
                        self.previews.accept_with_other_bytes(
                            context,
                            request,
                            result,
                            self.pdf_previews
                                .retained_bytes()
                                .saturating_add(self.presentation_previews.retained_bytes())
                                .saturating_add(self.spreadsheet_previews.retained_bytes())
                                .saturating_add(self.hwp_previews.retained_bytes())
                                .saturating_add(self.web_previews.retained_bytes()),
                        );
                    }
                }
                HostReply::DraftMirrored { job, result } => {
                    self.persistence.mirror_finished(
                        &job,
                        result.as_ref().is_ok_and(|applied| *applied),
                        Instant::now(),
                    );
                    if let Err(error) = result {
                        log::warn!("native draft mirror failed: {error}");
                    }
                }
                HostReply::ObservedFile {
                    path,
                    document,
                    result,
                } => {
                    self.observing_files.remove(&document);
                    if self.store.documents().snapshot(document).is_err() {
                        continue;
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
                                self.submit(HostCommand::SetDirty { tab, dirty });
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
                HostReply::DiskChosen {
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
                                self.submit(HostCommand::CleanupFileMirror {
                                    project,
                                    canonical: committed.canonical,
                                    expected: committed.mirror,
                                });
                            }
                        }
                        Err(error) => self.status = Some(error.to_string()),
                    }
                }
                HostReply::UntitledOpened { tab, result } => {
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
                            self.submit(HostCommand::SetDirty { tab, dirty });
                        }
                        Err(error) => {
                            self.untitled_failed.insert(tab, error);
                        }
                    }
                }
                HostReply::UntitledSaved { tab, result } => {
                    self.saving_untitled = None;
                    match result.and_then(|prepared| prepared.commit(&mut self.store)) {
                        Ok(converted) => {
                            self.persistence.settled(converted.document);
                            self.untitled.remove(&converted.source_tab);
                            self.untitled_failed.remove(&converted.source_tab);
                            self.untitled_loading.remove(&converted.source_tab);
                            if let Some(previous) = converted.merged_document {
                                self.persistence.settled(previous);
                                self.last_edited_views.remove(&previous);
                                for file in
                                    self.files.values_mut().filter(|file| file.id == previous)
                                {
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
                            self.submit(HostCommand::CleanupUntitledMirror {
                                project: converted.project,
                                tab: converted.source_tab,
                                expected: converted.mirror,
                            });
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
                HostReply::MissingDraft { path, prepared } => {
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
                                self.submit(HostCommand::SetDirty { tab, dirty: true });
                            }
                        }
                        Err(error) => {
                            self.failed.insert(path, error);
                        }
                    }
                }
                HostReply::MissingDraftSaved { tab, path, result } => {
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
                        }
                    }
                }
                HostReply::Closed { tab, result } => {
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
                HostReply::MirrorSaved { tab, result } => match result {
                    Ok(()) => {
                        self.submit(HostCommand::SetDirty {
                            tab: tab.clone(),
                            dirty: false,
                        });
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
                HostReply::TreeSynced {
                    project,
                    result,
                    errors,
                } => {
                    match result {
                        Ok(page) => {
                            self.trees.insert(project, page);
                        }
                        Err(error) => self.status = Some(error.to_string()),
                    }
                    if !errors.is_empty() {
                        self.status = Some(
                            errors
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join("\n"),
                        );
                    }
                }
                HostReply::Opened {
                    path,
                    project,
                    result,
                } => {
                    self.loading.remove(&path);
                    let source_tab = self.loading_file_tabs.remove(&path);
                    match result.and_then(|prepared| prepared.commit_with_notice(&mut self.store)) {
                        Ok(committed) => {
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
                                self.submit(HostCommand::SetDirty { tab, dirty });
                            }
                        }
                        Err(error) => {
                            self.failed.insert(path, error);
                        }
                    }
                }
                HostReply::Saved { snapshot, result } => {
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
                        let taide_native_editor::document::DocumentKey::File(canonical) = key
                        else {
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
                                self.submit(HostCommand::SetDirty { tab, dirty });
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
                HostReply::Tree {
                    project,
                    offset,
                    result,
                } => {
                    self.tree_loading.remove(&project);
                    match result {
                        Ok(page) if offset == 0 => {
                            self.trees.insert(project, page);
                        }
                        Ok(page) => {
                            if let Some(current) = self.trees.get_mut(&project) {
                                current.rows.extend(page.rows);
                                current.total = page.total;
                            }
                        }
                        Err(error) => self.status = Some(error.to_string()),
                    }
                }
                HostReply::TreeToggled {
                    project,
                    path,
                    result,
                } => {
                    if let Some(explorer) = self.explorers.get_mut(&project) {
                        explorer.toggle_finished(&path, result.as_ref().ok());
                    }
                    match result {
                        Ok(page) => {
                            self.trees.insert(project, page);
                        }
                        Err(error) => self.status = Some(error.to_string()),
                    }
                }
                HostReply::CopiedText { result } => crate::toast::copy_finished(
                    &mut self.toasts,
                    &self.locale,
                    crate::toast::CopyOrigin::Explorer,
                    &result,
                    Instant::now(),
                ),
                HostReply::TerminalSelectionCopied { result } => crate::toast::copy_finished(
                    &mut self.toasts,
                    &self.locale,
                    crate::toast::CopyOrigin::Terminal,
                    &result,
                    Instant::now(),
                ),
                HostReply::TerminalUrlOpened { result } => {
                    if result.is_err() {
                        crate::toast::open_link_failed(
                            &mut self.toasts,
                            &self.locale,
                            Instant::now(),
                        );
                    }
                }
                HostReply::SystemFinished { result } => {
                    if let Err(error) = result {
                        self.report(&error);
                    }
                }
                HostReply::SettingsFailed(error) => {
                    self.toasts
                        .settings_failed(&self.locale, &error, Instant::now())
                }
                HostReply::ProjectFiles { project, result } => {
                    self.palette_files.accept(&project, result, Instant::now());
                }
                HostReply::PaletteFileOpened { project, result } => {
                    if let Err(error) = result {
                        if error.kind() == taide_model::error::AppErrorKind::NotFound {
                            self.palette_files.invalidate(&project);
                        }
                        self.toasts.ipc_error(&self.locale, &error, Instant::now());
                    }
                }
                HostReply::Failed(error) => self.report(&error),
            }
        }
    }

    fn report(&mut self, error: &AppError) {
        self.toasts.ipc_error(&self.locale, error, Instant::now());
    }

    fn report_close_failure(
        &mut self,
        batch: Option<&mut crate::tab_close_batch::Batch>,
        error: &AppError,
    ) {
        match batch {
            Some(batch) => {
                batch
                    .failure
                    .get_or_insert_with(|| crate::toast::describe_error(&self.locale, error));
            }
            None => self.report(error),
        }
    }

    fn report_once(&mut self, error: &AppError) {
        crate::toast::error_once(&mut self.toasts, &self.locale, error, Instant::now());
    }

    fn report_save_failure(
        &mut self,
        error: taide_native_editor::document::EditorError,
        auto_save: bool,
    ) {
        crate::toast::save_failed(
            &mut self.toasts,
            &self.locale,
            error,
            auto_save,
            Instant::now(),
        );
    }

    fn notify_open_project_first(&mut self) {
        crate::toast::open_project_first(&mut self.toasts, &self.locale, Instant::now());
    }

    fn report_create_failure(
        &mut self,
        project: &ProjectId,
        request: &crate::explorer::CreateRequest,
        error: &AppError,
    ) {
        let message = crate::toast::describe_error(&self.locale, error);
        if let Some(explorer) = self.explorers.get_mut(project) {
            explorer.create_finished(request, Err(message.clone()));
        }
        crate::toast::entry_failed(
            &mut self.toasts,
            &self.locale,
            message,
            crate::toast::Action::RetryCreate {
                project: project.clone(),
                request: request.clone(),
            },
            Instant::now(),
        );
    }

    fn report_rename_failure(
        &mut self,
        project: &ProjectId,
        request: &crate::explorer::RenameRequest,
        error: &AppError,
    ) {
        let message = crate::toast::describe_error(&self.locale, error);
        if let Some(explorer) = self.explorers.get_mut(project) {
            explorer.rename_finished(request, Err(message.clone()));
        }
        crate::toast::entry_failed(
            &mut self.toasts,
            &self.locale,
            message,
            crate::toast::Action::RetryRename {
                project: project.clone(),
                request: request.clone(),
            },
            Instant::now(),
        );
    }

    fn create_entry(&mut self, project: ProjectId, request: crate::explorer::CreateRequest) {
        let result = self
            .lsp
            .as_ref()
            .ok_or_else(|| AppError::Internal("native workspace host is unavailable".into()))
            .and_then(|bridge| bridge.create_entry(project.clone(), request.clone()));
        if let Err(error) = result {
            self.report_create_failure(&project, &request, &error);
        }
    }

    fn rename_entry(&mut self, project: ProjectId, request: crate::explorer::RenameRequest) {
        let result = self
            .lsp
            .as_ref()
            .ok_or_else(|| AppError::Internal("native workspace host is unavailable".into()))
            .and_then(|bridge| bridge.rename_entry(project.clone(), request.clone()));
        if let Err(error) = result {
            self.report_rename_failure(&project, &request, &error);
        }
    }

    fn run_toast_actions(&mut self) {
        for action in self.toasts.take_actions() {
            if !self.flush_dirty() {
                return;
            }
            match action {
                crate::toast::Action::RetryCreate { project, request } => {
                    let retried = self.trees.get(&project).and_then(|page| {
                        self.explorers.get_mut(&project)?.retry_create(
                            &request,
                            &page.rows,
                            &self.locale,
                        )
                    });
                    if let Some(request) = retried {
                        self.create_entry(project, request);
                    }
                }
                crate::toast::Action::RetryRename { project, request } => {
                    let retried = self.trees.get(&project).and_then(|page| {
                        self.explorers.get_mut(&project)?.retry_rename(
                            &request,
                            &page.rows,
                            &self.locale,
                        )
                    });
                    if let Some(request) = retried {
                        self.rename_entry(project, request);
                    }
                }
            }
        }
    }

    fn submit(&mut self, command: HostCommand) -> bool {
        if let HostCommand::SnippetEdit(request) = &command
            && matches!(request.kind(), taide_native_ui::snippet_edit::Kind::List)
        {
            if !request.is_active() || !request.owner().is_active(&self.services.state) {
                self.settings_views
                    .accept_snippet(request.clone().failed(AppError::Forbidden(
                        "native snippet observer owner is no longer active".into(),
                    )));
                return false;
            }
            self.settings_views
                .observe_snippet_list(request.clone(), Instant::now());
            return true;
        }
        let is_settings = matches!(
            &command,
            HostCommand::SetKeymapOverrides(_)
                | HostCommand::SetEditorFontSize(_)
                | HostCommand::UpdateSettings(_)
                | HostCommand::OpenSettingsFolder(_)
        );
        if let HostCommand::SetDirty { tab, dirty } = command {
            self.pending_dirty.insert(tab, dirty);
            return true;
        }
        let mut tracked_save = None;
        let (save, command) = match command {
            HostCommand::Save { path, snapshot } => (Some((path, snapshot, false)), None),
            HostCommand::AutoSave { path, snapshot } => (Some((path, snapshot, true)), None),
            other => (None, Some(other)),
        };
        let command = if let Some((path, snapshot, auto_save)) = save {
            let document = snapshot.document();
            if self.pending_format_saves.contains_key(&document) {
                return false;
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
                && self.lsp.is_some()
            {
                self.reconcile_lsp();
                let Some(epoch) = self.persistence.begin_save(document) else {
                    return false;
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
                        return false;
                    }
                };
                let settings = self.services.state.settings.read();
                let config = current.metadata.editor_config;
                let indent = taide_native_editor::indent::resolve(
                    &config,
                    taide_native_editor::indent::IndentOptions {
                        tab_size: settings.editor_tab_size,
                        insert_spaces: settings.editor_insert_spaces,
                    },
                );
                let options = taide_lsp::native::protocol::lsp_types::FormattingOptions {
                    tab_size: indent.tab_size,
                    insert_spaces: indent.insert_spaces,
                    ..Default::default()
                };
                drop(settings);
                let result = self
                    .lsp
                    .as_ref()
                    .expect("native language server bridge is connected")
                    .participate(current, should_format.then_some(options), actions);
                match result {
                    Ok(()) => {
                        self.pending_format_saves.insert(
                            document,
                            PendingFormatSave {
                                path,
                                auto_save,
                                epoch,
                            },
                        );
                        return true;
                    }
                    Err(error) => {
                        log::warn!("native save participation failed: {error}");
                        self.resumed_save_epochs.insert(document, epoch);
                    }
                }
            }
            let Some(epoch) = self
                .resumed_save_epochs
                .remove(&document)
                .or_else(|| self.persistence.begin_save(document))
            else {
                return false;
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
                        return true;
                    }
                    Err(error) => {
                        self.persistence.submission_failed(document);
                        self.report_save_failure(error, auto_save);
                        return false;
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
            HostCommand::SaveTracked {
                path,
                snapshot: prepared.snapshot,
                epoch,
            }
        } else if let Some(command) = command {
            command
        } else {
            return false;
        };
        let file_resolution = match &command {
            HostCommand::ResolveTerminalFileLinks { owner, request, .. } => {
                Some((owner.clone(), request.clone()))
            }
            _ => None,
        };
        let theme_operation = match &command {
            HostCommand::ThemeEdit(command) => Some(command.clone()),
            _ => None,
        };
        let snippet_operation = match &command {
            HostCommand::SnippetEdit(request) => Some(request.clone()),
            _ => None,
        };
        let snippet_catalog = match &command {
            HostCommand::ReadSnippetCatalog(request) => Some(request.clone()),
            _ => None,
        };
        let result = self
            .bridge
            .as_ref()
            .ok_or_else(|| AppError::Forbidden("native host is closing".into()))
            .and_then(|bridge| bridge.submit(command));
        if let Err(error) = result {
            if let Some(request) = snippet_catalog {
                self.settings_views
                    .accept_snippet_catalog(request.failed(error), Instant::now());
                return false;
            }
            if let Some(request) = snippet_operation {
                self.settings_views.accept_snippet(request.failed(error));
                return false;
            }
            if let Some(command) = theme_operation {
                self.toasts.ipc_error(&self.locale, &error, Instant::now());
                self.settings_views.accept_theme(command.failed(error));
                return false;
            }
            if let Some((owner, request)) = file_resolution {
                self.terminal_views.cancel_file_links(&owner, &request);
            }
            if let Some(document) = tracked_save {
                self.persistence.submission_failed(document);
            }
            if is_settings {
                self.toasts
                    .settings_failed(&self.locale, &error, Instant::now());
            } else {
                self.report_once(&error);
            }
            return false;
        }
        true
    }

    fn flush_dirty(&mut self) -> bool {
        let pending: Vec<_> = self
            .pending_dirty
            .iter()
            .map(|(tab, dirty)| (tab.clone(), *dirty))
            .collect();
        for (tab, dirty) in pending {
            let result = self
                .bridge
                .as_ref()
                .ok_or_else(|| AppError::Forbidden("native host is closing".into()))
                .and_then(|bridge| {
                    bridge.submit(HostCommand::SetDirty {
                        tab: tab.clone(),
                        dirty,
                    })
                });
            if let Err(error) = result {
                self.report_once(&error);
                return false;
            }
            self.pending_dirty.remove(&tab);
        }
        true
    }

    fn close(&mut self, context: &egui::Context) {
        if self.closing.is_some()
            || self.workspace_busy()
            || self.is_exit_ready
            || self.pending_tab_close.is_some()
            || self.saving_missing_draft.is_some()
            || self.saving_untitled.is_some()
            || self.pending_disk_choice.is_some()
            || !self.pending_format_saves.is_empty()
        {
            return;
        }
        if !self.flush_dirty() {
            context.request_repaint();
            return;
        }
        self.app_file_views.clear();
        let bridge = self.bridge.take().map(HostBridge::disconnect);
        let lsp = self.lsp.take().map(crate::lsp::LspBridge::disconnect);
        let web = self
            .web_bridge
            .take()
            .map(crate::preview_web_host::Bridge::disconnect);
        let syntax = self.editor_syntax.disconnect();
        self.web_previews.invalidate_all();
        self.web_views.clear();
        let drafts = self.drafts();
        let services = self.services.clone();
        let repaint = context.clone();
        let (sender, receiver) = oneshot::channel();
        self.closing = Some(receiver);
        self.runtime.spawn(async move {
            let (result, ()) = tokio::join!(
                shutdown(services, bridge, lsp, web, drafts),
                crate::editor_syntax::finished(syntax),
            );
            drop(sender.send(result));
            repaint.request_repaint();
        });
    }

    fn request_tab_close(&mut self, tab: TabId) {
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

    fn request_close_tabs(&mut self, ids: Vec<TabId>) {
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

    fn request_tab_save(
        &mut self,
        tab: TabId,
        document: Option<DocumentId>,
        frame: &eframe::Frame,
    ) {
        if let Some(owner) = self.app_file_views.save_owner(&tab, document) {
            match self.app_file_views.begin_write(&owner, &mut self.store) {
                Ok(Some(request)) => {
                    if !self.submit(HostCommand::WriteAppFile(request.clone())) {
                        self.app_file_views.cancel_write(&request);
                    }
                }
                Ok(None) => (),
                Err(error) => {
                    self.toasts
                        .app_file_failed(&self.locale, owner.target, &error, Instant::now())
                }
            }
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
                self.submit(HostCommand::Save { path, snapshot });
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
            return self
                .app_file_views
                .release_closed(&closed.tab, &mut self.store)
                .map_err(editor_error);
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
        if let Some(layout) = self.services.state.layouts.read().get(&closed.project) {
            self.open_with.close_path(path, layout);
        }
        if !closed.has_remaining_file {
            self.previews.invalidate(path);
            self.cancel_web(Some(std::path::Path::new(path)));
            self.web_previews.invalidate(path);
        }
        self.pdf_previews.reconcile(&self.pdf_preview_paths());
        self.presentation_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Presentation));
        self.spreadsheet_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Spreadsheet));
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

    fn tab_close_dialog(&mut self, context: &egui::Context, frame: &eframe::Frame) {
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
                                        HostCommand::Save {
                                            path: path.clone(),
                                            snapshot,
                                        },
                                    )
                                })
                        } else {
                            Ok((None, HostCommand::SaveMirroredTab(tab.id.clone())))
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
            if self.flush_dirty() && self.submit(HostCommand::CloseTab { tab, discard }) {
                if let Some(pending) = self.pending_tab_close.as_mut() {
                    pending.phase = TabClosePhase::Closing;
                }
            } else {
                context.request_repaint();
            }
        }
    }

    fn save_untitled(&mut self, tab: &TabId, frame: &eframe::Frame) {
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
        let selected = rfd::FileDialog::new()
            .set_parent(frame)
            .set_directory(root)
            .set_file_name(title)
            .set_title(presentation::message(&self.locale, "tab.saveAsTitle", &[]))
            .save_file();
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
        if self.submit(HostCommand::SaveUntitled {
            tab: tab.clone(),
            destination,
            snapshot,
        }) {
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

    fn save_missing_draft(&mut self, tab: &TabId, frame: &eframe::Frame) {
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
        let selected = rfd::FileDialog::new()
            .set_parent(frame)
            .set_directory(parent)
            .set_file_name(name)
            .set_title(presentation::message(&self.locale, "tab.saveAsTitle", &[]))
            .save_file();
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
        if self.submit(HostCommand::SaveMissingDraft {
            tab: tab.clone(),
            draft,
            destination,
        }) {
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

    fn drafts(&self) -> AppResult<Vec<Draft>> {
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

    fn choose_disk(&mut self, tab: &TabId, action: BannerAction) {
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
        if self.submit(HostCommand::ChooseDisk {
            path,
            document,
            revision: live.revision,
            choice,
        }) {
            self.pending_disk_choice = Some(document);
        }
    }

    fn queue_project_observations(&mut self, project: &ProjectId) {
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

    fn flush_observations(&mut self) {
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
            if self.submit(HostCommand::ObserveFile { path, document }) {
                self.pending_file_observations.remove(&document);
                self.observing_files.insert(document);
            } else {
                break;
            }
        }
    }

    fn draft_project(&self, document: DocumentId) -> Option<ProjectId> {
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
    }

    fn auto_save_delay(&self, document: DocumentId) -> Duration {
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

    fn flush_persistence(&mut self, context: &egui::Context) {
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
                                if !self.submit(HostCommand::AutoSave { path, snapshot }) {
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
                            if !self.submit(HostCommand::MirrorDraft(job.clone())) {
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

    fn reconcile_lsp(&mut self) {
        if self.closing.is_some() || self.services.state.is_shutting_down() {
            return;
        }
        self.lsp_diagnostics.retain_documents(
            self.store
                .documents()
                .snapshots()
                .map(|model| model.id)
                .collect(),
        );
        let Some(lsp) = self.lsp.as_mut() else {
            return;
        };
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
        for (_, path) in crate::lsp::visible_files(&shell, &self.shell.scope) {
            if self.loading.contains(path)
                || self.failed.contains_key(path)
                || self.missing_drafts.contains_key(path)
            {
                continue;
            }
            let Some(file) = self.files.get(path) else {
                continue;
            };
            let Some(project) = &file.project else {
                continue;
            };
            if !retained.insert(file.id) {
                continue;
            }
            if let Ok(snapshot) = self.store.documents().snapshot(file.id)
                && let Err(error) = lsp.sync(project.clone(), snapshot)
            {
                self.status = Some(error.to_string());
            }
        }
        if let Err(error) = lsp.retain(&retained) {
            self.status = Some(error.to_string());
        }
    }

    fn poll_lsp(&mut self) {
        if let Some(bindings) = self
            .lsp
            .as_mut()
            .and_then(crate::lsp::LspBridge::diagnostic_bindings)
        {
            self.lsp_diagnostics.reconcile(bindings);
        }
        while let Some(reply) = self.lsp.as_mut().and_then(crate::lsp::LspBridge::poll) {
            match reply {
                crate::lsp::Reply::ExplorerPasted(event) => {
                    let path = match event.result {
                        Ok(pasted) => {
                            if self
                                .services
                                .state
                                .projects
                                .read()
                                .contains_key(&event.project)
                            {
                                self.trees.insert(event.project.clone(), pasted.page);
                                Some(pasted.path)
                            } else {
                                None
                            }
                        }
                        Err(error) => {
                            self.report(&error);
                            None
                        }
                    };
                    self.explorer_clipboards.paste_finished(
                        event.request.owner,
                        event.clear_cut,
                        path,
                    );
                }
                crate::lsp::Reply::ExplorerCreated(event) => match event.result {
                    Ok(created) => {
                        if self
                            .services
                            .state
                            .projects
                            .read()
                            .contains_key(&event.project)
                        {
                            self.trees.insert(event.project.clone(), created.page);
                            if let Some(layout) = created.layout {
                                self.controller.apply_layouts(HashMap::from([(
                                    event.project.clone(),
                                    layout,
                                )]));
                            }
                            if let Some(explorer) = self.explorers.get_mut(&event.project) {
                                explorer.create_finished(&event.request, Ok(()));
                            }
                        }
                    }
                    Err(error) => {
                        self.report_create_failure(&event.project, &event.request, &error)
                    }
                },
                crate::lsp::Reply::ExplorerRenamed(event) => match event.result {
                    Ok(page) => {
                        self.trees.insert(event.project.clone(), page);
                        if let Some(explorer) = self.explorers.get_mut(&event.project) {
                            explorer.rename_finished(&event.request, Ok(()));
                        }
                    }
                    Err(error) => {
                        self.report_rename_failure(&event.project, &event.request, &error)
                    }
                },
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
                        for explorer in self.explorers.values_mut() {
                            explorer.moved(&event.from, &event.to);
                        }
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
                crate::lsp::Reply::ExplorerDeleteFinished { request, result } => {
                    self.deleting_entry = false;
                    match result {
                        Ok(_) => {
                            let parent = std::path::Path::new(&request.path)
                                .parent()
                                .and_then(|parent| parent.to_str())
                                .map(String::from);
                            if self
                                .services
                                .state
                                .projects
                                .read()
                                .contains_key(&request.project)
                            {
                                self.submit(HostCommand::RefreshTree {
                                    project: request.project,
                                    dirs: parent
                                        .map(|parent| std::collections::BTreeSet::from([parent])),
                                });
                            }
                        }
                        Err(error) => self.report(&error),
                    }
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
                crate::lsp::Reply::Formatted { snapshot, result } => {
                    let document = snapshot.id;
                    let Some(pending) = self.pending_format_saves.remove(&document) else {
                        continue;
                    };
                    match result {
                        Ok(Some(edits)) => {
                            let view = self.save_view(document);
                            match taide_native_editor::lsp::apply_text_edits(
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
                            let command = if pending.auto_save {
                                HostCommand::AutoSave {
                                    path: pending.path,
                                    snapshot,
                                }
                            } else {
                                HostCommand::Save {
                                    path: pending.path,
                                    snapshot,
                                }
                            };
                            if !self.submit(command) {
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
            }
        }
    }

    fn workspace_busy(&self) -> bool {
        self.workspace_activity
            .as_ref()
            .is_some_and(crate::workspace_activity::Activity::is_active)
    }

    fn reconcile_previews(&mut self) {
        let invalidation = self.tree_changes.take_previews();
        if invalidation.all {
            self.cancel_web(None);
            self.previews.invalidate_all();
            self.hwp_previews.invalidate_all();
            self.pdf_previews.invalidate_all();
            self.presentation_previews.invalidate_all();
            self.spreadsheet_previews.invalidate_all();
            self.web_previews.invalidate_all();
        } else {
            for path in invalidation.paths {
                self.cancel_web(Some(std::path::Path::new(&path)));
                self.previews.invalidate(&path);
                self.hwp_previews.invalidate(&path);
                self.pdf_previews.invalidate(&path);
                self.presentation_previews.invalidate(&path);
                self.spreadsheet_previews.invalidate(&path);
                self.web_previews.invalidate(&path);
            }
            let projects = self.services.state.projects.read();
            for project in invalidation.projects {
                if let Some(project) = projects.get(&project) {
                    self.cancel_web(Some(std::path::Path::new(&project.root)));
                    self.previews.invalidate_root(&project.root);
                    self.hwp_previews.invalidate_root(&project.root);
                    self.pdf_previews.invalidate_root(&project.root);
                    self.presentation_previews.invalidate_root(&project.root);
                    self.spreadsheet_previews.invalidate_root(&project.root);
                    self.web_previews.invalidate_root(&project.root);
                }
            }
        }
        let layouts = self.services.state.layouts.read().clone();
        self.open_with.reconcile(&layouts);
        let projects = self.services.state.projects.read();
        let paths = layouts
            .iter()
            .filter(|(project, _)| projects.contains_key(*project))
            .flat_map(|(_, layout)| taide_layout::service::all_roots(layout))
            .flat_map(crate::tabs::tabs_in)
            .filter_map(|tab| match &tab.kind {
                TabKind::File { path }
                    if self.open_with.surface(path)
                        == crate::open_with::Surface::Preview(
                            crate::open_with::PreviewKind::Image,
                        ) =>
                {
                    Some(path.clone())
                }
                _ => None,
            })
            .collect();
        self.previews.retain(&paths);
        drop(projects);
        self.pdf_previews.reconcile(&self.pdf_preview_paths());
        self.hwp_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Hwp));
        self.presentation_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Presentation));
        self.spreadsheet_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Spreadsheet));
        let paths = self.web_preview_paths();
        if self
            .web_previews
            .active_request()
            .is_some_and(|request| !paths.values().any(|path| path == &request.path))
        {
            self.cancel_web(None);
        }
        self.web_previews.reconcile(&paths);
    }

    fn cancel_web(&self, changed: Option<&std::path::Path>) {
        if let Some(request) = self.web_previews.active_request()
            && changed.is_none_or(|path| std::path::Path::new(&request.path).starts_with(path))
            && let Some(bridge) = &self.web_bridge
        {
            bridge.cancel_through(request.token);
        }
    }

    fn pdf_preview_paths(&self) -> HashMap<TabId, String> {
        self.preview_tab_paths(crate::open_with::PreviewKind::Pdf)
    }

    fn web_preview_paths(&self) -> HashMap<TabId, String> {
        [
            crate::open_with::PreviewKind::Html,
            crate::open_with::PreviewKind::Audio,
            crate::open_with::PreviewKind::Video,
        ]
        .into_iter()
        .flat_map(|kind| self.preview_tab_paths(kind))
        .collect()
    }

    fn preview_tab_paths(&self, kind: crate::open_with::PreviewKind) -> HashMap<TabId, String> {
        let layouts = self.services.state.layouts.read().clone();
        let projects = self.services.state.projects.read();
        layouts
            .iter()
            .filter(|(project, _)| projects.contains_key(*project))
            .flat_map(|(_, layout)| taide_layout::service::all_roots(layout))
            .flat_map(crate::tabs::tabs_in)
            .filter_map(|tab| match &tab.kind {
                TabKind::File { path }
                    if self.open_with.surface(path) == crate::open_with::Surface::Preview(kind) =>
                {
                    Some((tab.id.clone(), path.clone()))
                }
                _ => None,
            })
            .collect()
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

    fn presentation_inputs(&self, context: &egui::Context) -> crate::presentation_refresh::Inputs {
        let system_theme = match context.system_theme() {
            Some(egui::Theme::Light) => "light",
            Some(egui::Theme::Dark) | None => "dark",
        };
        crate::presentation_refresh::Inputs::new(
            self.services.state.settings.read().clone(),
            system_theme,
            &self.presentation_system_language,
        )
    }

    fn show_palette(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        command_context: &CommandContext,
        keymap_overrides: Option<&str>,
        enabled: bool,
    ) {
        let now = Instant::now();
        let project = command_context.active_project.as_ref();
        if let Some(stale) = self
            .palette_files
            .observe(project, self.palette.observes_files(), now)
            && !self.submit(HostCommand::ListProjectFiles(stale.clone()))
        {
            self.palette_files.accept(
                &stale,
                Err(AppError::Forbidden(
                    "native file index host is disconnected or full".into(),
                )),
                now,
            );
        }
        let active_file = snapshot.focused_tab().and_then(|tab| match &tab.kind {
            TabKind::File { path } => Some((tab, path.as_str())),
            _ => None,
        });
        let root = project
            .and_then(|project| snapshot.project(project))
            .map(|project| project.root.as_str());
        self.palette
            .set_reduced_motion(self.motion_preference.current().unwrap_or(false));
        let output = self.palette.show(
            context,
            crate::command_palette::Scope {
                locale: &self.locale,
                commands: command_context,
                keymap_overrides,
                files: self.palette_files.view(project, root),
                active_file: active_file.map(|(_, path)| path),
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
            Some(crate::command_palette::Action::RunCommand(id)) => {
                self.palette_commands.push(id);
                context.request_repaint();
            }
            Some(crate::command_palette::Action::OpenFile(path)) => {
                let Some(project) = project else {
                    self.notify_open_project_first();
                    return;
                };
                let pane = snapshot
                    .layouts
                    .get(project)
                    .and_then(|layout| crate::command_palette::file_tab_pane(layout, &path));
                self.submit(HostCommand::OpenPaletteFile {
                    project: project.clone(),
                    pane,
                    path,
                });
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
            None => (),
        }
    }

    fn toast_interaction_enabled(&self) -> bool {
        self.closing.is_none()
            && !self.system_usage.detail_open
            && !self.keybindings.is_open()
            && !self.palette.is_open()
            && self.pending_tab_close.is_none()
            && self.pending_entry_delete.is_none()
    }

    fn sync_theme_preview(&mut self, context: &egui::Context) {
        let preview = if self.closing.is_none() && !self.services.state.is_shutting_down() {
            self.settings_views
                .preview(context.viewport_id(), &self.services.state)
                .cloned()
        } else {
            None
        };
        if preview == self.preview_theme {
            return;
        }
        let theme = preview.as_ref().unwrap_or(&self.resolved_theme);
        let settings = self.services.state.settings.read().clone();
        let result = crate::presentation_refresh::Appearances::new(theme, &settings).and_then(
            |appearances| {
                self.terminal_views.set_palette(&appearances.terminal)?;
                Ok(appearances)
            },
        );
        match result {
            Ok(appearances) => {
                presentation::apply_visuals(context, &appearances.visuals);
                self.shell.colors = appearances.shell;
                self.editor.appearance = appearances.editor;
                self.find_appearance = appearances.find;
                self.editor_display_colors = appearances.editor_display;
                self.editor_bracket_colors = appearances.editor_brackets;
                self.editor_sticky_colors = appearances.editor_sticky;
                self.banner_appearance = appearances.banner;
                self.lsp_status_appearance = appearances.lsp_status;
                self.status_editor_appearance = appearances.status_editor;
                self.status_ide_appearance = appearances.status_ide;
                self.system_usage_appearance = appearances.system_usage;
                self.status_chord_appearance = appearances.status_chord;
                self.problems_appearance = appearances.problems;
                self.explorer_appearance = appearances.explorer;
                self.tooltip_appearance = appearances.tooltip;
                self.terminal_appearance = appearances.terminal;
                self.keybindings.set_appearance(appearances.keybindings);
                self.palette.set_appearance(appearances.palette);
                self.settings_appearance = appearances.settings;
                self.pdf_appearance = appearances.pdf;
                self.presentation_appearance = appearances.presentation;
                self.spreadsheet_appearance = appearances.spreadsheet;
                self.toast_theme = appearances.toast;
                self.preview_theme = preview;
                context.request_repaint();
            }
            Err(error) => self.status = Some(error.to_string()),
        }
    }

    fn background_tick(&mut self, context: &egui::Context) {
        self.toasts
            .set_reduced_motion(self.motion_preference.current().unwrap_or(false));
        let toast_enabled = self.toast_interaction_enabled();
        let toast_position = self.services.state.settings.read().toast_position.clone();
        let toast_now = Instant::now();
        self.toasts
            .set_position(context, &toast_position, toast_now, toast_enabled);
        self.toasts.tick(context, toast_now, toast_enabled);
        if self.closing.is_some() || self.services.state.is_shutting_down() {
            self.terminal_views.cancel_inputs();
            self.reveals.clear();
        } else if let Err(error) =
            self.terminal_views
                .flush_inputs(self.terminals.hub(), &self.services, context)
        {
            self.status = Some(error.to_string());
        }
        if self.closing.is_none() && !self.is_exit_ready && !self.services.state.is_shutting_down()
        {
            let families = crate::editor_fonts::Families::new(&self.services.state.settings.read());
            if let Some(result) =
                self.terminal_fonts
                    .update(families, context, &self.services.tasks)
            {
                match result {
                    Ok(warnings) => {
                        for warning in warnings {
                            log::warn!("{warning}");
                        }
                    }
                    Err(error) => self.status = Some(error.to_string()),
                }
            }
        } else {
            self.terminal_fonts.cancel();
        }
        let font_size = self.services.state.settings.read().terminal_font_size;
        if self.terminal_appearance.set_font_size(font_size) {
            context.request_repaint();
        }
        let editor_font_size = self.services.state.settings.read().editor_font_size;
        if presentation::update_editor_font_size(&mut self.editor.appearance, editor_font_size) {
            context.request_repaint();
        }
        match self.terminals.hub().configure_cursors() {
            Ok(true) => context.request_repaint(),
            Ok(false) => {}
            Err(error) => self.status = Some(error.to_string()),
        }
        self.reconcile_previews();
        self.poll(context);
        self.sync_theme_preview(context);
        if self.closing.is_some() || self.is_exit_ready || self.services.state.is_shutting_down() {
            self.presentation_refresh.cancel();
            self.settings_views.clear();
            self.app_file_views.clear();
        } else {
            self.editor_syntax.follow_plugins(
                &self.services.state,
                &self.services.plugin,
                &self.services.tasks,
            );
            let syntax_theme = self.preview_theme.as_ref().unwrap_or(&self.resolved_theme);
            if let Some(delay) = self
                .editor_syntax
                .tick(&self.store, syntax_theme, Instant::now())
            {
                context.request_repaint_after(delay);
            }
            if let Some(request) = self.settings_views.next_snippet_read() {
                self.submit(HostCommand::ReadSnippetCatalog(request));
            }
            let inputs = self.presentation_inputs(context);
            if let Some(request) = self
                .presentation_refresh
                .next(inputs, self.presentation_changes.revision())
                && self.submit(HostCommand::RefreshPresentation(Box::new(request.clone())))
            {
                self.presentation_refresh.submitted(request);
            }
        }
        self.reveals
            .reconcile(&self.services.state.layouts.read(), Instant::now());
        self.explorer_clipboards
            .reconcile(self.controller.snapshot().shell.tree.as_ref());
        self.problems
            .reconcile(self.controller.snapshot().shell.tree.as_ref());
        self.reconcile_lsp();
        self.poll_lsp();
        self.terminal_views.chord_status(context, Instant::now());
        let (show_usage, hide_in_zen) = {
            let settings = self.services.state.settings.read();
            (settings.show_system_usage, settings.zen_hide_status_bar)
        };
        let status_mounted = !(self.services.state.session.read().window_chrome.zen && hide_in_zen);
        let usage_visible = status_mounted && show_usage;
        if !status_mounted {
            self.system_usage.unmount(Instant::now());
        }
        if self.closing.is_some() || self.is_exit_ready {
            self.system_usage.detail_open = false;
        }
        self.system_usage.tick(
            &self.services,
            context,
            usage_visible && self.closing.is_none() && !self.is_exit_ready,
            Instant::now(),
        );
        if !self.workspace_busy() {
            self.workspace_activity = None;
            for document in std::mem::take(&mut self.workspace_suspended) {
                if let Ok(snapshot) = self.store.documents().snapshot(document)
                    && snapshot.dirty
                {
                    let mirror = self.draft_project(document).is_some();
                    let auto_save = self.auto_save_delay(document);
                    self.persistence.changed(
                        document,
                        snapshot.revision,
                        Instant::now(),
                        mirror,
                        auto_save,
                    );
                }
            }
        }
        if let Some(receiver) = &mut self.closing {
            match receiver.try_recv() {
                Ok(Ok(())) => {
                    self.is_exit_ready = true;
                    self.closing = None;
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Ok(Err(error)) => {
                    self.status = Some(error.to_string());
                    self.closing = None;
                    if !self.services.state.is_shutting_down() {
                        self.loading.clear();
                        self.tree_loading.clear();
                        self.palette_files.cancel_fetches();
                        self.previews.reset_pending();
                        self.hwp_previews.reset_pending();
                        self.pdf_previews.reset_pending();
                        self.presentation_previews.reset_pending();
                        self.spreadsheet_previews.reset_pending();
                        match connect_web(
                            self.services.clone(),
                            context,
                            self.helper_executable.clone(),
                            &self.pdf_appearance,
                        ) {
                            Ok(bridge) => self.web_bridge = Some(bridge),
                            Err(error) => self.status = Some(error.to_string()),
                        }
                        let repaint_context = context.clone();
                        match HostBridge::connect_with_application_ports(
                            self.services.clone(),
                            Arc::new(move || repaint_context.request_repaint()),
                            crate::host::Terminals {
                                tabs: self.terminals.clone(),
                                environment: crate::terminal_environment::provider(),
                            },
                            &self.application_ports,
                        ) {
                            Ok(bridge) => {
                                self.bridge = Some(bridge);
                                let repaint_context = context.clone();
                                match crate::lsp::LspBridge::connect(
                                    self.services.clone(),
                                    std::env::var_os("PATH").unwrap_or_default(),
                                    Arc::new(move || repaint_context.request_repaint()),
                                ) {
                                    Ok(lsp) => self.lsp = Some(lsp),
                                    Err(error) => self.status = Some(error.to_string()),
                                }
                                let repaint_context = context.clone();
                                match crate::editor_syntax::EditorSyntax::connect(
                                    &self.services.tasks,
                                    Arc::new(move || repaint_context.request_repaint()),
                                ) {
                                    Ok(syntax) => self.editor_syntax = syntax,
                                    Err(error) => self.status = Some(error.to_string()),
                                }
                                self.observing_files.clear();
                                self.loading_file_tabs.clear();
                                self.untitled_loading.clear();
                                self.persistence.cancel_inflight();
                                let documents = self
                                    .files
                                    .values()
                                    .map(|file| file.id)
                                    .chain(self.untitled.values().map(|document| document.id))
                                    .collect::<HashSet<_>>();
                                for document in documents {
                                    if let Ok(snapshot) = self.store.documents().snapshot(document)
                                        && snapshot.dirty
                                    {
                                        let mirror = self.draft_project(document).is_some();
                                        self.persistence.changed(
                                            document,
                                            snapshot.revision,
                                            Instant::now(),
                                            mirror,
                                            Duration::ZERO,
                                        );
                                    }
                                }
                            }
                            Err(error) => self.status = Some(error.to_string()),
                        }
                    }
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.closing = None;
                    self.status = Some("native exit owner stopped before completion".into());
                }
            }
        }
        let close_requested = context.input(|input| input.viewport().close_requested());
        if close_requested && !self.is_exit_ready {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.close_request_active {
                self.close_request_active = true;
                self.close(context);
            }
        } else if !close_requested {
            self.close_request_active = false;
        }
        let snapshot = self.controller.snapshot();
        for project in self.palette_file_changes.take() {
            self.palette_files.invalidate(&project);
        }
        self.palette_files
            .retain(|project| snapshot.layouts.contains_key(project));
        if self.closing.is_none() && !self.is_exit_ready && !self.services.state.is_shutting_down()
        {
            let open_projects: HashSet<_> = snapshot.layouts.keys().cloned().collect();
            for (project, dirs) in self.tree_changes.take(open_projects.iter().cloned()) {
                if !open_projects.contains(&project) {
                    continue;
                }
                if self.submit(HostCommand::RefreshTree {
                    project: project.clone(),
                    dirs: dirs.clone(),
                }) {
                    self.queue_project_observations(&project);
                } else {
                    self.tree_changes.merge(project, dirs);
                    context.request_repaint();
                }
            }
        }
        if !self.workspace_busy() {
            self.flush_observations();
            self.flush_persistence(context);
        }
    }
}

impl eframe::App for NativeApplication {
    fn raw_input_hook(&mut self, context: &egui::Context, input: &mut egui::RawInput) {
        self.terminal_views.raw_input(context, input);
    }

    fn raw_input_hook_with_replay(
        &mut self,
        context: &egui::Context,
        input: &mut egui::RawInput,
        replayed_events: usize,
    ) {
        self.terminal_views
            .raw_input_with_replay(context, input, replayed_events);
    }

    fn logic(&mut self, context: &egui::Context, _: &mut eframe::Frame) {
        self.background_tick(context);
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        if !context.input(|input| input.raw.focused) {
            self.terminal_views.clear_keymap_chord(&context);
        }
        if let Some(bridge) = &self.web_bridge
            && bridge.set_media_appearance(media_appearance(&self.pdf_appearance))
        {
            if let Some(request) = self.web_previews.active_request()
                && matches!(
                    crate::open_with::preview_kind(&request.path),
                    Some(
                        crate::open_with::PreviewKind::Audio | crate::open_with::PreviewKind::Video
                    )
                )
            {
                bridge.cancel_through(request.token);
            }
            self.web_previews.invalidate_media();
        }
        let snapshot = self.controller.snapshot();
        self.explorer_clipboards
            .reconcile(snapshot.shell.tree.as_ref());
        if self.closing.is_none() && !self.services.state.is_shutting_down() {
            self.zen_fullscreen_state.reconcile(
                &context,
                snapshot.shell.window_chrome.zen,
                self.services.state.settings.read().zen_fullscreen,
            );
        }
        let terminal_tabs = snapshot
            .layouts
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .filter(|tab| matches!(tab.kind, TabKind::Terminal { .. }))
            .collect::<Vec<_>>();
        let live_tabs = terminal_tabs.iter().map(|tab| tab.id.clone()).collect();
        let live_sessions = terminal_tabs
            .iter()
            .filter_map(|tab| match &tab.kind {
                TabKind::Terminal { session_id, .. } => Some(session_id.clone()),
                _ => None,
            })
            .collect();
        self.terminal_views.retain(&live_tabs, &live_sessions);
        let target = snapshot
            .focused_project()
            .and_then(|project| snapshot.layouts.get(project))
            .map(|layout| layout.focused_pane.clone());
        let focused_view = snapshot
            .focused_tab()
            .zip(target.as_ref())
            .map(|(tab, pane)| ViewKey {
                window: WINDOW_LABEL.into(),
                pane: pane.clone(),
                tab: tab.id.clone(),
            });
        let command_context = crate::command_dispatch::context(
            &snapshot,
            &self.shell.scope,
            crate::command_dispatch::active_editor_actions(&self.store, focused_view.as_ref()),
        );
        self.terminal_views
            .set_command_context(command_context.clone());
        let mut document_edits = std::mem::take(&mut self.document_edits);
        self.editor_find
            .retain(|view, _| self.store.views().get(*view).is_some());
        let mut fold_commands = std::mem::take(&mut self.fold_commands);
        let mut command_errors = Vec::new();
        let mut commands = Vec::new();
        let mut load_settings = Vec::new();
        let mut load_settings_resources = Vec::new();
        let mut load_app_files = Vec::new();
        let mut settings_errors = Vec::new();
        let mut theme_errors = Vec::new();
        let mut snippet_notices = Vec::new();
        self.settings_views.begin_frame();
        let mut keymap_actions = Vec::new();
        let mut keymap_editor_scope = None;
        let mut keymap_documents = HashMap::new();
        let mut rename_entries = Vec::new();
        let mut paste_entries = Vec::new();
        let mut create_entries = Vec::new();
        let mut delete_entries = Vec::new();
        let mut load_files = Vec::new();
        let mut load_previews = Vec::new();
        let mut load_pdf_previews = Vec::new();
        let mut load_hwp_previews = Vec::new();
        let mut load_presentation_previews = Vec::new();
        let mut load_spreadsheet_previews = Vec::new();
        let mut load_web_previews = Vec::new();
        let mut web_placements = Vec::new();
        let mut load_trees = Vec::new();
        let mut save_missing_drafts = Vec::new();
        let mut load_untitled = Vec::new();
        let mut banner_actions = Vec::new();
        let mut changed_documents = HashMap::new();
        let mut new_focus = self.focused.clone();
        let mut status = self.status.take();
        let is_workspace_busy = self.workspace_busy();
        let is_enabled = self.closing.is_none()
            && !self.system_usage.detail_open
            && self.pending_entry_delete.is_none()
            && !self.deleting_entry
            && !is_workspace_busy
            && !self.services.state.is_shutting_down()
            && self.pending_tab_close.is_none()
            && self.saving_missing_draft.is_none()
            && self.saving_untitled.is_none()
            && self.pending_disk_choice.is_none();
        let viewport = context.viewport_id();
        let pass = context.cumulative_frame_nr();
        self.editor_keymap_targets
            .retain(|(owner_viewport, _), (view, previous)| {
                (*owner_viewport != viewport
                    || *previous == pass
                    || previous.checked_add(1) == Some(pass))
                    && self.store.views().get(*view).is_some_and(|view| {
                        snapshot
                            .layouts
                            .values()
                            .flat_map(taide_layout::service::all_roots)
                            .any(|root| {
                                taide_native_ui::snapshot::active_tab(root, &view.key.pane)
                                    .is_some_and(|tab| tab.id == view.key.tab)
                            })
                    })
            });
        if is_enabled
            && !self.keybindings.is_capturing()
            && context.input(|input| input.raw.focused)
            && let Err(error) = self.terminal_views.capture_window_keymap(
                &context,
                self.services
                    .state
                    .settings
                    .read()
                    .keymap_overrides
                    .as_deref(),
                &mut keymap_actions,
                snapshot.focused_project().is_some(),
                |id| {
                    let (view, _) = self.editor_keymap_targets.get(&(viewport, id))?;
                    Some(self.store.views().get(*view)?.composition.is_some())
                },
            )
        {
            status = Some(error.to_string());
        }
        self.tooltips.begin_frame(&context);
        let lsp_summary = snapshot
            .focused_project()
            .and_then(|project| self.lsp.as_ref().and_then(|lsp| lsp.summary(project)));
        let mut surfaces = AppSurfaces {
            tooltips: &self.tooltips,
            tooltip_appearance: &self.tooltip_appearance,
            problems: &mut self.problems,
            problems_appearance: &self.problems_appearance,
            explorer_appearance: &self.explorer_appearance,
            explorer_icons: &mut self.explorer_icons,
            diagnostics: &self.lsp_diagnostics,
            focused_slot: snapshot.shell.focused.as_ref(),
            chord_status: self.terminal_views.chord_status(&context, Instant::now()),
            status_chord_appearance: &self.status_chord_appearance,
            system_usage: &mut self.system_usage,
            system_usage_appearance: &self.system_usage_appearance,
            system_usage_icon: &mut self.system_usage_icon,
            status_editor_appearance: &self.status_editor_appearance,
            status_ide_appearance: &self.status_ide_appearance,
            status_ide_icons: &mut self.status_ide_icons,
            status_view: focused_view,
            command_context: &command_context,
            document_edits: &mut document_edits,
            fold_commands: &mut fold_commands,
            command_errors: &mut command_errors,
            lsp_summary,
            lsp_status_appearance: &self.lsp_status_appearance,
            app_file_views: &mut self.app_file_views,
            load_app_files: &mut load_app_files,
            settings_views: &mut self.settings_views,
            settings_appearance: &self.settings_appearance,
            load_settings: &mut load_settings,
            load_settings_resources: &mut load_settings_resources,
            settings_errors: &mut settings_errors,
            theme_errors: &mut theme_errors,
            snippet_notices: &mut snippet_notices,
            terminals: &self.terminals,
            terminal_views: &mut self.terminal_views,
            keymap_actions: &mut keymap_actions,
            keymap_editor_scope: &mut keymap_editor_scope,
            editor_keymap_targets: &mut self.editor_keymap_targets,
            keymap_documents: &mut keymap_documents,
            terminal_appearance: &self.terminal_appearance,
            services: &self.services,
            locale: &self.locale,
            store: &mut self.store,
            editor: &self.editor,
            editor_find: &mut self.editor_find,
            find_history: &mut self.find_history,
            find_appearance: &self.find_appearance,
            editor_display_colors: self.editor_display_colors,
            editor_bracket_colors: self.editor_bracket_colors,
            editor_sticky_colors: self.editor_sticky_colors,
            editor_sticky_scroll: self.editor_sticky_scroll.synchronize(
                self.services
                    .state
                    .settings
                    .read()
                    .editor_sticky_scroll_enabled,
            ),
            editor_syntax: &mut self.editor_syntax,
            banner_appearance: &self.banner_appearance,
            restore_notices: &self.restore_notices,
            banner_actions: &mut banner_actions,
            changed_documents: &mut changed_documents,
            files: &self.files,
            open_with: &mut self.open_with,
            previews: &mut self.previews,
            load_previews: &mut load_previews,
            pdf_previews: &mut self.pdf_previews,
            hwp_previews: &mut self.hwp_previews,
            load_hwp_previews: &mut load_hwp_previews,
            pdf_appearance: &self.pdf_appearance,
            load_pdf_previews: &mut load_pdf_previews,
            presentation_previews: &mut self.presentation_previews,
            presentation_appearance: &self.presentation_appearance,
            load_presentation_previews: &mut load_presentation_previews,
            spreadsheet_previews: &mut self.spreadsheet_previews,
            spreadsheet_appearance: &self.spreadsheet_appearance,
            load_spreadsheet_previews: &mut load_spreadsheet_previews,
            web_previews: &self.web_previews,
            load_web_previews: &mut load_web_previews,
            web_placements: &mut web_placements,
            untitled: &self.untitled,
            untitled_loading: &self.untitled_loading,
            untitled_failed: &self.untitled_failed,
            load_untitled: &mut load_untitled,
            missing_drafts: &self.missing_drafts,
            save_missing_drafts: &mut save_missing_drafts,
            loading: &self.loading,
            failed: &self.failed,
            trees: &self.trees,
            tree_loading: &self.tree_loading,
            explorers: &mut self.explorers,
            explorer_clipboards: &mut self.explorer_clipboards,
            rename_entries: &mut rename_entries,
            paste_entries: &mut paste_entries,
            create_entries: &mut create_entries,
            delete_entries: &mut delete_entries,
            projects: &snapshot.projects,
            layouts: &snapshot.layouts,
            scope: &self.shell.scope,
            commands: &mut commands,
            load_files: &mut load_files,
            load_trees: &mut load_trees,
            target: target.as_ref(),
            focused: &mut new_focus,
            reveals: &mut self.reveals,
            status: &mut status,
        };
        let mut intents = ui
            .add_enabled_ui(
                is_enabled && !self.keybindings.is_open() && !self.palette.is_open(),
                |ui| self.shell.show(ui, &snapshot, &mut surfaces),
            )
            .inner;
        self.focused = new_focus;
        self.status = status;
        if self.system_usage.detail_open
            && crate::system_usage_view::show_detail(
                &context,
                &self.locale,
                &self.system_usage_appearance,
                self.system_usage.processes(),
            )
        {
            self.system_usage.detail_open = false;
            context.request_repaint();
        }
        self.settings_views.finish_frame();
        self.sync_theme_preview(&context);
        for error in settings_errors {
            self.toasts
                .settings_failed(&self.locale, &error, Instant::now());
        }
        for error in theme_errors.into_iter().chain(command_errors) {
            self.toasts.ipc_error(&self.locale, &error, Instant::now());
        }
        for notice in snippet_notices {
            self.toasts.snippet(&self.locale, notice, Instant::now());
        }
        for request in load_settings {
            if !self.submit(HostCommand::ReadSettingsCatalog(request.clone())) {
                self.settings_views.accept(
                    &request,
                    Err(AppError::Forbidden(
                        "native Settings host is disconnected or full".into(),
                    )),
                );
            }
        }
        for request in load_settings_resources {
            if !self.submit(HostCommand::ReadSettingsResource(request.clone())) {
                self.settings_views
                    .accept_resource(request.failed(AppError::Forbidden(
                        "native Settings resource host is disconnected or full".into(),
                    )));
            }
        }
        for request in load_app_files {
            if !self.submit(HostCommand::ReadAppFile(request.clone())) {
                self.app_file_views.accept(
                    &request,
                    Err(AppError::Forbidden(
                        "native app file host is disconnected or full".into(),
                    )),
                    &mut self.store,
                );
            }
        }
        if let Err(error) =
            self.terminal_views
                .finish_frame(self.terminals.hub(), &self.services, &context)
        {
            self.status = Some(error.to_string());
        }
        if is_enabled && !self.system_usage.detail_open {
            if !self.keybindings.is_capturing() && context.input(|input| input.raw.focused) {
                let scope = crate::keymap::Context {
                    terminal: self.terminal_views.has_keyboard_focus(&context),
                    editor: keymap_editor_scope.is_some(),
                };
                self.keybindings.observe_closed(scope, Instant::now());
                if let Err(error) = self.terminal_views.route_window_keys(
                    &context,
                    scope,
                    keymap_editor_scope == Some(true),
                    self.services
                        .state
                        .settings
                        .read()
                        .keymap_overrides
                        .as_deref(),
                    &mut keymap_actions,
                    snapshot.focused_project().is_some(),
                ) {
                    self.status = Some(error.to_string());
                }
            }
            keymap_actions.append(&mut self.palette_commands);
            intents.extend(keymap_actions.into_iter().filter_map(|action| {
                crate::command_dispatch::intent(&action, &command_context, &snapshot)
            }));
            let now = Instant::now();
            for (document, view) in changed_documents {
                self.last_edited_views.insert(document, view);
                if let Ok(snapshot) = self.store.documents().snapshot(document) {
                    let mirror = self.draft_project(document).is_some();
                    let auto_save = self.auto_save_delay(document);
                    self.persistence
                        .changed(document, snapshot.revision, now, mirror, auto_save);
                }
            }
            if !self.keybindings.is_capturing()
                && !self.palette.is_open()
                && context
                    .input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::N))
                && let Some(project) = snapshot.focused_project()
                && let Some(pane) = target.as_ref()
            {
                commands.push(HostCommand::NewUntitled {
                    project: project.clone(),
                    pane: pane.clone(),
                });
            }
            for tab in load_untitled {
                if self.untitled_loading.insert(tab.clone())
                    && !self.submit(HostCommand::OpenUntitled(tab.clone()))
                {
                    self.untitled_loading.remove(&tab);
                    context.request_repaint();
                }
            }
            for tab in save_missing_drafts {
                self.save_missing_draft(&tab, frame);
            }
            for (path, tab) in load_files {
                if self.loading.insert(path.clone()) {
                    self.loading_file_tabs.insert(path.clone(), tab);
                    if !self.submit(HostCommand::OpenDocument(path.clone())) {
                        self.loading.remove(&path);
                        self.loading_file_tabs.remove(&path);
                        context.request_repaint();
                    }
                }
            }
            for (path, max_side) in load_previews {
                if let Some(request) = self.previews.begin(&path, max_side)
                    && !self.submit(HostCommand::ReadPreview(request.clone()))
                {
                    self.previews.cancelled(&request);
                    context.request_repaint();
                }
            }
            for (tab, path, max_side) in load_pdf_previews {
                if let Some(request) = self.pdf_previews.begin(&tab, &path, max_side)
                    && !self.submit(HostCommand::ReadPdfPreview(request.clone()))
                {
                    self.pdf_previews.cancelled(&request);
                    context.request_repaint();
                }
            }
            for (tab, path, max_side) in load_hwp_previews {
                if let Some(request) = self.hwp_previews.begin(&tab, &path, max_side)
                    && !self.submit(HostCommand::ReadHwpPreview(request.clone()))
                {
                    self.hwp_previews.cancelled(&request);
                    context.request_repaint();
                }
            }
            for (tab, path) in load_presentation_previews {
                if let Some(request) = self.presentation_previews.begin(&tab, &path)
                    && !self.submit(HostCommand::ReadPresentationPreview(request.clone()))
                {
                    self.presentation_previews.cancelled(&request);
                    context.request_repaint();
                }
            }
            for (tab, path) in load_spreadsheet_previews {
                if let Some(request) = self.spreadsheet_previews.begin(&tab, &path)
                    && !self.submit(HostCommand::ReadSpreadsheetPreview(request.clone()))
                {
                    self.spreadsheet_previews.cancelled(&request);
                    break;
                }
            }
            for path in load_web_previews {
                if let Some(request) = self.web_previews.begin(&path) {
                    let result = self
                        .web_bridge
                        .as_ref()
                        .ok_or_else(|| {
                            AppError::Forbidden("native web preview host is disconnected".into())
                        })
                        .and_then(|bridge| bridge.submit(request.clone()));
                    if let Err(error) = result {
                        self.web_previews.accept(request, Err(error), 0);
                    }
                }
            }
            for project in load_trees {
                if self.tree_loading.insert(project.clone())
                    && !self.submit(HostCommand::TreeRows {
                        project: project.clone(),
                        offset: 0,
                    })
                {
                    self.tree_loading.remove(&project);
                    context.request_repaint();
                }
            }
            let mut actions = Vec::new();
            for command in commands {
                if matches!(
                    command,
                    HostCommand::SetDirty { .. }
                        | HostCommand::ThemeEdit(_)
                        | HostCommand::SnippetEdit(_)
                ) {
                    self.submit(command);
                } else {
                    actions.push(command);
                }
            }
            if !self.flush_dirty() {
                self.terminal_views.submission_failed();
                for (project, request) in create_entries {
                    if let Some(explorer) = self.explorers.get_mut(&project) {
                        explorer.create_finished(
                            &request,
                            Err("native dirty tab update is unavailable".into()),
                        );
                    }
                }
                for (project, request) in rename_entries {
                    if let Some(explorer) = self.explorers.get_mut(&project) {
                        explorer.rename_finished(
                            &request,
                            Err("native dirty tab update is unavailable".into()),
                        );
                    }
                }
                context.request_repaint();
                return;
            }
            for (project, request) in create_entries {
                self.create_entry(project, request);
            }
            for (project, request) in rename_entries {
                self.rename_entry(project, request);
            }
            for (project, request) in paste_entries {
                let result = self
                    .lsp
                    .as_ref()
                    .ok_or_else(|| {
                        AppError::Internal("native workspace host is unavailable".into())
                    })
                    .and_then(|bridge| bridge.paste_entry(project, request));
                if let Err(error) = result {
                    self.report(&error);
                }
            }
            for command in actions {
                if !self.submit(command) {
                    self.terminal_views.submission_failed();
                    break;
                }
            }
            for intent in intents {
                match intent {
                    ShellIntent::OpenSettingsFile => {
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
                    ShellIntent::EditDocument { tab, edit } => {
                        self.document_edits.push((tab, edit));
                        context.request_repaint();
                    }
                    ShellIntent::FoldDocument { tab, command } => {
                        self.fold_commands.push((tab, command));
                        context.request_repaint();
                    }
                    ShellIntent::OpenSettings => {
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
                                        .map(|window| {
                                            (project.clone(), window.focused_pane.clone())
                                        })
                                })
                            }
                        };
                        if let Some((project, pane)) = owner {
                            self.submit(HostCommand::OpenSettings {
                                project,
                                pane,
                                title: presentation::message(&self.locale, "settings.title", &[]),
                            });
                        } else {
                            self.notify_open_project_first();
                        }
                    }
                    ShellIntent::OpenKeybindings => self.keybindings.open(&context),
                    ShellIntent::OpenPalette(entry) => self.palette.open(&context, entry),
                    ShellIntent::NewTerminal { project, pane } => {
                        self.submit(HostCommand::NewTerminal {
                            project,
                            pane,
                            title: presentation::message(&self.locale, "terminal.title", &[]),
                        });
                    }
                    ShellIntent::NewUntitled { project, pane } => {
                        self.submit(HostCommand::NewUntitled { project, pane });
                    }
                    ShellIntent::ShowOpenProjectNotice => self.notify_open_project_first(),
                    ShellIntent::ToggleEditorStickyScroll => {
                        self.editor_sticky_scroll.toggle(
                            self.services
                                .state
                                .settings
                                .read()
                                .editor_sticky_scroll_enabled,
                        );
                        context.request_repaint();
                    }
                    ShellIntent::ChangeEditorFontSize { increase } => {
                        let current = self.services.state.settings.read().editor_font_size;
                        self.submit(HostCommand::SetEditorFontSize(
                            presentation::next_editor_font_size(current, increase),
                        ));
                    }
                    ShellIntent::RequestCloseTab(tab) => self.request_tab_close(tab),
                    ShellIntent::RequestCloseTabs(tabs) => self.request_close_tabs(tabs),
                    ShellIntent::RequestSaveTab(tab) => {
                        let document = keymap_documents.get(&tab).copied();
                        self.request_tab_save(tab, document, frame);
                    }
                    ShellIntent::OpenFolder => {
                        if let Some(path) = rfd::FileDialog::new().set_parent(frame).pick_folder() {
                            match path.into_os_string().into_string() {
                                Ok(path) => {
                                    self.submit(HostCommand::OpenProject(path));
                                }
                                Err(_) => {
                                    self.status = Some("native project path is not UTF-8".into())
                                }
                            }
                        }
                    }
                    ShellIntent::OpenFile { project, pane } => {
                        let root = self
                            .services
                            .state
                            .projects
                            .read()
                            .get(&project)
                            .map(|project| project.root.clone());
                        if let Some(root) = root
                            && let Some(path) = rfd::FileDialog::new()
                                .set_parent(frame)
                                .set_directory(root)
                                .pick_file()
                        {
                            match path.into_os_string().into_string() {
                                Ok(path) => {
                                    self.submit(HostCommand::OpenFileTab {
                                        project,
                                        pane: Some(pane),
                                        path,
                                        preview: false,
                                    });
                                }
                                Err(_) => {
                                    self.status = Some("native file path is not UTF-8".into())
                                }
                            }
                        }
                    }
                    ShellIntent::Mutate(command) => {
                        if let Err(error) = self.controller.submit(command) {
                            self.report(&error);
                        }
                    }
                }
            }
            for (tab, action) in banner_actions {
                self.choose_disk(&tab, action);
            }
            if let Some(request) = delete_entries.into_iter().next() {
                self.pending_entry_delete = Some(crate::delete_dialog::Confirmation::new(request));
            }
            self.flush_observations();
        }
        let keybindings_enabled = is_enabled
            && !self.system_usage.detail_open
            && self.pending_tab_close.is_none()
            && self.pending_entry_delete.is_none()
            && !self.deleting_entry
            && self.saving_untitled.is_none()
            && self.saving_missing_draft.is_none()
            && self.pending_disk_choice.is_none();
        let raw_overrides = self.services.state.settings.read().keymap_overrides.clone();
        match self.keybindings.show(
            &context,
            &self.locale,
            raw_overrides.as_deref(),
            keybindings_enabled && !self.palette.is_open(),
        ) {
            Ok(output) => {
                output.show_tooltips(&self.locale, &self.tooltips, &self.tooltip_appearance);
                if output.started_capture {
                    self.terminal_views.clear_keymap_chord(&context);
                }
                for warning in output.warnings {
                    self.toasts.warning(warning, Instant::now());
                }
                for overrides in output.saves {
                    self.submit(HostCommand::SetKeymapOverrides(overrides));
                }
            }
            Err(error) => self.status = Some(error.to_string()),
        }
        self.show_palette(
            &context,
            &snapshot,
            &command_context,
            raw_overrides.as_deref(),
            keybindings_enabled,
        );
        self.tooltips.finish_frame(&context);
        if let Some(pending) = self.pending_entry_delete.as_mut() {
            let choice = pending.show(&context, &self.locale).choice;
            if let Some(confirmed) = choice {
                let request = self
                    .pending_entry_delete
                    .take()
                    .expect("delete confirmation exists")
                    .request;
                context.memory_mut(|memory| {
                    memory.request_focus(egui::Id::new(("native-tree-focus", &request.project)))
                });
                if confirmed && self.flush_dirty() {
                    let result = self
                        .lsp
                        .as_ref()
                        .ok_or_else(|| {
                            AppError::Internal("native workspace host is unavailable".into())
                        })
                        .and_then(|bridge| bridge.delete_entry(request));
                    match result {
                        Ok(()) => self.deleting_entry = true,
                        Err(error) => self.report(&error),
                    }
                }
                context.request_repaint();
            }
        }
        self.tab_close_dialog(&context, frame);
        if crate::zen::escape(
            &context,
            snapshot.shell.window_chrome.zen,
            is_enabled
                && !self.keybindings.is_open()
                && !self.palette.is_open()
                && self.pending_tab_close.is_none()
                && self.pending_entry_delete.is_none()
                && !self.deleting_entry
                && self.saving_missing_draft.is_none()
                && self.saving_untitled.is_none()
                && self.pending_disk_choice.is_none(),
            keymap_editor_scope == Some(true),
        ) && let Err(error) = self.controller.submit(ShellMutation::SetWindowChrome(
            taide_model::project::WindowChromePatch {
                zen: Some(false),
                ..Default::default()
            },
        )) {
            self.report(&error);
        }
        if let Err(error) =
            self.terminal_views
                .finish_frame(self.terminals.hub(), &self.services, &context)
        {
            self.status = Some(error.to_string());
        }
        self.flush_persistence(&context);
        let live = self
            .web_preview_paths()
            .into_iter()
            .filter_map(|(tab, path)| {
                self.web_previews
                    .source(&path)
                    .cloned()
                    .map(|source| (tab, source))
            })
            .collect();
        let web_enabled = is_enabled
            && !self.keybindings.is_open()
            && !self.palette.is_open()
            && self.pending_tab_close.is_none()
            && self.pending_entry_delete.is_none()
            && !egui::Popup::is_any_open(&context)
            && !egui::DragAndDrop::has_any_payload(&context);
        if let Err(error) =
            self.web_views
                .sync(frame, &context, &live, &web_placements, web_enabled)
        {
            self.web_views.clear();
            for placement in web_placements {
                if let Some(path) = self.web_preview_paths().get(&placement.tab) {
                    self.web_previews.renderer_failed(path);
                }
            }
            self.status = Some(error.to_string());
        }
        let toast_position = self.services.state.settings.read().toast_position.clone();
        let toast_enabled = self.toast_interaction_enabled();
        if let Err(error) = self.toasts.show(
            &context,
            self.toast_theme,
            &toast_position,
            Instant::now(),
            toast_enabled,
        ) {
            self.status = Some(error.to_string());
        }
        self.run_toast_actions();
    }

    fn on_exit(&mut self) {
        if self.is_exit_ready {
            return;
        }
        let pending_close = self.closing.take();
        let bridge = self.bridge.take().map(HostBridge::disconnect);
        let lsp = self.lsp.take().map(crate::lsp::LspBridge::disconnect);
        let web = self
            .web_bridge
            .take()
            .map(crate::preview_web_host::Bridge::disconnect);
        let syntax = self.editor_syntax.disconnect();
        self.web_previews.invalidate_all();
        self.web_views.clear();
        let drafts = self.drafts();
        let services = self.services.clone();
        self.runtime.block_on(async move {
            let result = if let Some(pending) = pending_close {
                pending.await.unwrap_or_else(|error| {
                    Err(AppError::Internal(format!(
                        "native pending exit completion failed: {error}"
                    )))
                })
            } else {
                tokio::join!(
                    shutdown(services.clone(), bridge, lsp, web, drafts),
                    crate::editor_syntax::finished(syntax),
                )
                .0
            };
            if let Err(error) = finish_direct_exit(&services, result).await {
                log::error!("native direct exit failed: {:?}", error.kind());
            }
        });
    }
}

enum DraftTarget {
    File(String),
    Untitled(TabId),
}

struct Draft {
    project: ProjectId,
    target: DraftTarget,
    content: String,
}

async fn shutdown(
    services: Arc<AppServices>,
    bridge: Option<tokio::task::JoinHandle<()>>,
    lsp: Option<tokio::task::JoinHandle<()>>,
    web: Option<tokio::task::JoinHandle<()>>,
    drafts: AppResult<Vec<Draft>>,
) -> AppResult<()> {
    let (host_result, lsp_result, web_result) = tokio::join!(
        async {
            if let Some(worker) = bridge {
                worker.await.map_err(|error| {
                    AppError::Internal(format!("native host completion failed: {error}"))
                })?;
            }
            Ok::<_, AppError>(())
        },
        async {
            if let Some(worker) = lsp {
                worker.await.map_err(|error| {
                    AppError::Internal(format!("native language server completion failed: {error}"))
                })?;
            }
            Ok::<_, AppError>(())
        },
        async {
            if let Some(worker) = web {
                worker.await.map_err(|_| {
                    AppError::Internal("native web preview host did not finish".into())
                })?;
            }
            Ok::<_, AppError>(())
        },
    );
    host_result?;
    lsp_result?;
    web_result?;
    for draft in drafts? {
        match draft.target {
            DraftTarget::File(path) => {
                file_actions::file_mirror_dirty(
                    &services.state,
                    &services.tasks,
                    draft.project,
                    path,
                    draft.content,
                )
                .await?;
            }
            DraftTarget::Untitled(tab) => {
                let state = services.state.clone();
                services
                    .tasks
                    .run_blocking_result("native-untitled-exit-mirror", move || {
                        taide_infra::root_guard::project_root(
                            &state.projects.read(),
                            &draft.project,
                        )?;
                        taide_infra::root_guard::ensure_safe_component(tab.as_str())?;
                        taide_file::service::mirror_untitled(
                            &state.paths,
                            &draft.project,
                            &tab,
                            &draft.content,
                        )
                    })
                    .await?;
            }
        }
    }
    crate::host::flush_layouts(&services).await?;
    drain_services(&services).await
}

async fn finish_direct_exit(services: &Arc<AppServices>, result: AppResult<()>) -> AppResult<()> {
    if result.is_ok() {
        return result;
    }
    drain_services(services).await?;
    result
}

async fn drain_services(services: &Arc<AppServices>) -> AppResult<()> {
    services.state.begin_shutdown();
    taide_runtime::agent_actions::cleanup_all_wait_markers(&services.agents);
    crate::application_ports::Ports::stop_services(services);
    services.search.cancel_all();
    let drain = ExitDrain::new(services.ai_requests.clone()).with_state(services.state.clone());
    drain
        .wait_for_direct_exit(
            services.tasks.clone(),
            services.lsp_install.clone(),
            services.lsp.clone(),
            services.terminal.clone(),
        )
        .await
}

struct AppSurfaces<'a> {
    tooltips: &'a crate::tooltips::Provider,
    tooltip_appearance: &'a crate::tooltips::Appearance,
    problems: &'a mut crate::problems::Views,
    problems_appearance: &'a crate::problems::Appearance,
    explorer_appearance: &'a crate::explorer::Appearance,
    explorer_icons: &'a mut crate::problems_icons::Icons,
    diagnostics: &'a crate::diagnostics::Store,
    focused_slot: Option<&'a ShellSlotId>,
    chord_status: crate::keymap::ChordStatus,
    status_chord_appearance: &'a crate::status_chord::Appearance,
    system_usage: &'a mut crate::system_usage::Sampler,
    system_usage_appearance: &'a crate::system_usage_view::Appearance,
    system_usage_icon: &'a mut crate::system_usage_view::Icon,
    status_editor_appearance: &'a crate::status_editor::Appearance,
    status_ide_appearance: &'a crate::status_ide::Appearance,
    status_ide_icons: &'a mut crate::status_ide::Icons,
    status_view: Option<ViewKey>,
    command_context: &'a CommandContext,
    document_edits: &'a mut Vec<(TabId, DocumentEdit)>,
    fold_commands: &'a mut Vec<(TabId, FoldCommand)>,
    command_errors: &'a mut Vec<AppError>,
    lsp_summary: Option<crate::lsp::status::Summary>,
    lsp_status_appearance: &'a crate::lsp::status::Appearance,
    app_file_views: &'a mut crate::app_file_views::Views,
    load_app_files: &'a mut Vec<crate::app_file::ReadRequest>,
    settings_views: &'a mut crate::settings_view::Views,
    settings_appearance: &'a crate::settings_view::Appearance,
    load_settings: &'a mut Vec<crate::settings_view::Request>,
    load_settings_resources: &'a mut Vec<taide_native_ui::settings_resources::Request>,
    settings_errors: &'a mut Vec<AppError>,
    theme_errors: &'a mut Vec<AppError>,
    snippet_notices: &'a mut Vec<taide_native_ui::snippet_editor::Notice>,
    terminals: &'a crate::terminal_tabs::Tabs,
    terminal_views: &'a mut crate::terminal_surface::Views,
    keymap_actions: &'a mut Vec<String>,
    keymap_editor_scope: &'a mut Option<bool>,
    editor_keymap_targets: &'a mut HashMap<(egui::ViewportId, egui::Id), (ViewId, u64)>,
    keymap_documents: &'a mut HashMap<TabId, DocumentId>,
    terminal_appearance: &'a crate::terminal_surface::Appearance,
    services: &'a AppServices,
    locale: &'a ResolvedLocale,
    store: &'a mut EditorStore,
    editor: &'a NativeEditor,
    editor_find: &'a mut HashMap<ViewId, taide_native_ui::editor_find::EditorFind>,
    find_history: &'a mut taide_native_ui::editor_find_widget::FindHistory,
    find_appearance: &'a taide_native_ui::editor_find_widget::FindAppearance,
    editor_display_colors: taide_native_ui::editor_display::EditorDisplayColors,
    editor_bracket_colors: taide_native_ui::editor_brackets::EditorBracketColors,
    editor_sticky_colors: taide_native_ui::editor_sticky_scroll::EditorStickyColors,
    editor_sticky_scroll: bool,
    editor_syntax: &'a mut crate::editor_syntax::EditorSyntax,
    banner_appearance: &'a BannerAppearance,
    restore_notices: &'a HashMap<TabId, BannerVariant>,
    banner_actions: &'a mut Vec<(TabId, BannerAction)>,
    changed_documents: &'a mut HashMap<DocumentId, ViewId>,
    files: &'a HashMap<String, FileDocument>,
    open_with: &'a mut crate::open_with::Registry,
    previews: &'a mut crate::preview::Cache,
    load_previews: &'a mut Vec<(String, usize)>,
    pdf_previews: &'a mut crate::preview_pdf::Cache,
    hwp_previews: &'a mut crate::preview_hwp::Cache,
    load_hwp_previews: &'a mut Vec<(TabId, String, usize)>,
    pdf_appearance: &'a crate::preview_pdf_surface::Appearance,
    load_pdf_previews: &'a mut Vec<(TabId, String, usize)>,
    presentation_previews: &'a mut crate::preview_presentation_cache::Cache,
    presentation_appearance: &'a crate::preview_presentation_surface::Appearance,
    load_presentation_previews: &'a mut Vec<(TabId, String)>,
    spreadsheet_previews: &'a mut crate::preview_spreadsheet_cache::Cache,
    spreadsheet_appearance: &'a crate::preview_spreadsheet_surface::Appearance,
    load_spreadsheet_previews: &'a mut Vec<(TabId, String)>,
    web_previews: &'a crate::preview_web_cache::Cache,
    load_web_previews: &'a mut Vec<String>,
    web_placements: &'a mut Vec<crate::preview_web_view::Placement>,
    untitled: &'a HashMap<TabId, UntitledDocument>,
    untitled_loading: &'a HashSet<TabId>,
    untitled_failed: &'a HashMap<TabId, AppError>,
    load_untitled: &'a mut Vec<TabId>,
    missing_drafts: &'a HashMap<String, crate::missing_draft::MissingDraft>,
    save_missing_drafts: &'a mut Vec<TabId>,
    loading: &'a HashSet<String>,
    failed: &'a HashMap<String, AppError>,
    trees: &'a HashMap<ProjectId, TreeRowPage>,
    tree_loading: &'a HashSet<ProjectId>,
    explorers: &'a mut HashMap<ProjectId, crate::explorer::Explorer>,
    explorer_clipboards: &'a mut crate::explorer_clipboard_owners::ClipboardOwners,
    rename_entries: &'a mut Vec<(ProjectId, crate::explorer::RenameRequest)>,
    paste_entries: &'a mut Vec<(ProjectId, crate::explorer_clipboard::Request)>,
    create_entries: &'a mut Vec<(ProjectId, crate::explorer::CreateRequest)>,
    delete_entries: &'a mut Vec<crate::explorer_delete::Request>,
    projects: &'a [taide_model::project::ProjectRef],
    layouts: &'a HashMap<ProjectId, taide_model::layout::ProjectLayout>,
    scope: &'a WindowScope,
    commands: &'a mut Vec<HostCommand>,
    load_files: &'a mut Vec<(String, TabId)>,
    load_trees: &'a mut Vec<ProjectId>,
    target: Option<&'a PaneId>,
    focused: &'a mut Option<(PaneId, TabId)>,
    reveals: &'a mut crate::editor_reveal::Reveals,
    status: &'a mut Option<String>,
}

impl ShellSurfaces for AppSurfaces<'_> {
    fn text(&self, key: &str) -> String {
        presentation::message(self.locale, key, &[])
    }
    fn text_with_args(&self, key: &str, arguments: &[(&str, &str)]) -> String {
        presentation::message(self.locale, key, arguments)
    }
    fn branch(&self, _: &ProjectId) -> Option<&str> {
        None
    }
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
            self.commands.push(HostCommand::OpenProblem {
                project: project.clone(),
                path: open.path,
                line: open.line,
                column: open.column,
                viewport: ui.ctx().viewport_id(),
            });
        }
    }
    fn explorer(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        slot: &ShellSlotId,
        _: &mut Vec<ShellIntent>,
    ) {
        let owner = match self.explorer_clipboards.mount(slot) {
            Ok(owner) => owner,
            Err(error) => {
                *self.status = Some(error.to_string());
                return;
            }
        };
        let Some(page) = self.trees.get(project) else {
            if !self.tree_loading.contains(project) {
                self.load_trees.push(project.clone());
            }
            ui.spinner();
            return;
        };
        let explorer = self.explorers.entry(project.clone()).or_default();
        explorer.replace_clipboard(self.explorer_clipboards.clipboard(owner).cloned());
        if let Some(path) = self.explorer_clipboards.take_reveal(owner) {
            explorer.paste_finished(false, Some(path));
        }
        if let Some(project) = self.projects.iter().find(|entry| &entry.id == project) {
            explorer.root = Some(project.root.clone());
            explorer.title = Some(project.name.clone());
        }
        let output = explorer.show_with_icons(
            ui,
            project,
            page,
            self.locale,
            crate::explorer::RowIcons {
                glyphs: &mut *self.explorer_icons,
                appearance: self.explorer_appearance,
            },
        );
        if let Some(error) = &output.icon_error {
            *self.status = Some(error.to_string());
        }
        crate::explorer_toolbar::show_tooltips(
            &output,
            self.locale,
            self.tooltips,
            self.tooltip_appearance,
        );
        for action in output.actions {
            match action {
                crate::explorer::Action::Toggle(path) => {
                    self.commands.push(HostCommand::TreeToggle {
                        project: project.clone(),
                        path,
                    })
                }
                crate::explorer::Action::Open { path, preview } => {
                    self.commands.push(HostCommand::OpenFileTab {
                        project: project.clone(),
                        pane: None,
                        path,
                        preview,
                    })
                }
                crate::explorer::Action::OpenToSide(row) => {
                    if let Some(request) = crate::explorer::open_to_side_request(
                        project,
                        &row,
                        self.layouts.get(project),
                        self.scope,
                    ) {
                        self.commands.push(HostCommand::OpenFileToSide(request));
                    }
                }
                crate::explorer::Action::OpenWith { row, mode } => {
                    self.open_with.set(row.path.clone(), mode);
                    self.commands.push(HostCommand::OpenFileTab {
                        project: project.clone(),
                        pane: None,
                        path: row.path,
                        preview: false,
                    });
                }
                crate::explorer::Action::Rename(request) => {
                    self.rename_entries.push((project.clone(), request))
                }
                crate::explorer::Action::Create(request) => {
                    self.create_entries.push((project.clone(), request))
                }
                crate::explorer::Action::Paste(mut request) => {
                    request.owner = Some(owner);
                    self.paste_entries.push((project.clone(), request))
                }
                crate::explorer::Action::RequestDelete(row) => {
                    self.delete_entries.push(crate::explorer_delete::Request {
                        project: project.clone(),
                        path: row.path,
                        name: row.name,
                    })
                }
                crate::explorer::Action::CopyText(text) => {
                    self.commands.push(HostCommand::CopyText(text));
                }
                crate::explorer::Action::RevealPath(path) => {
                    self.commands.push(HostCommand::RevealPath(path));
                }
                crate::explorer::Action::OpenInBrowser(path) => {
                    self.commands.push(HostCommand::OpenInBrowser(path));
                }
                crate::explorer::Action::Refresh => self.commands.push(HostCommand::RefreshTree {
                    project: project.clone(),
                    dirs: None,
                }),
                crate::explorer::Action::Collapse => self
                    .commands
                    .push(HostCommand::TreeCollapse(project.clone())),
            }
        }
        self.explorer_clipboards
            .replace(owner, explorer.clipboard().cloned());
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
            if self.target == Some(pane) {
                *self.focused = None;
            }
            let settings = self.services.state.settings.read().clone();
            let output = self.settings_views.show(
                ui,
                crate::settings_view::Owner {
                    project: project.clone(),
                    pane: pane.clone(),
                    tab: tab.id.clone(),
                },
                &settings,
                self.locale,
                self.settings_appearance,
            );
            self.tooltips
                .show_triggers(&output.tooltips, self.tooltip_appearance);
            if let Some(request) = output.load {
                self.load_settings.push(request);
            }
            self.load_settings_resources.extend(output.resources);
            self.commands
                .extend(output.changes.into_iter().map(HostCommand::UpdateSettings));
            self.commands
                .extend(output.themes.into_iter().map(HostCommand::ThemeEdit));
            self.commands
                .extend(output.snippets.into_iter().map(HostCommand::SnippetEdit));
            self.snippet_notices.extend(output.snippet_notices);
            self.commands.extend(
                output
                    .folders
                    .into_iter()
                    .map(HostCommand::OpenSettingsFolder),
            );
            if let Some(error) = output.error {
                self.settings_errors.push(error);
            }
            if let Some(error) = output.theme_error {
                self.theme_errors.push(error);
            }
            if output.open_keybindings {
                intents.push(ShellIntent::OpenKeybindings);
            }
            if output.open_settings_file
                && let Some(layout) = self.layouts.get(project)
                && let Some(command) =
                    crate::app_file_views::settings_command(project, layout, self.scope)
            {
                self.commands.push(command);
            }
            return;
        }
        if let TabKind::AppFile { target } = tab.kind {
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
                self.load_app_files.push(request);
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
            return;
        }
        if let TabKind::Terminal { session_id, .. } = &tab.kind {
            let request_focus = self.target == Some(pane)
                && self
                    .focused
                    .as_ref()
                    .is_none_or(|(old_pane, old_tab)| old_pane != pane || old_tab != &tab.id);
            let keymap_actions = &mut *self.keymap_actions;
            let has_focused_shell = self.target.is_some();
            let command_context = self.command_context;
            match self.terminal_views.show_with_keymap(
                ui,
                crate::terminal_surface::Request {
                    pane,
                    tab: &tab.id,
                    session_id,
                    hub: self.terminals.hub(),
                    services: self.services,
                    appearance: self.terminal_appearance,
                    locale: self.locale,
                    request_focus,
                    commands: self.commands,
                },
                |action| {
                    if !crate::command_dispatch::accepts(action, has_focused_shell, command_context)
                    {
                        return false;
                    }
                    keymap_actions.push(action.into());
                    true
                },
            ) {
                Ok(response) => {
                    if response.has_focus() {
                        *self.focused = Some((pane.clone(), tab.id.clone()));
                    }
                    if response.clicked() {
                        intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
                    }
                }
                Err(error) => *self.status = Some(error.to_string()),
            }
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
        let TabKind::File { path } = &tab.kind else {
            let rect = ui.available_rect_before_wrap();
            ui.painter()
                .rect_filled(rect, 0.0, self.editor.appearance.background);
            ui.allocate_ui_with_layout(
                rect.size(),
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{}\n{}",
                                tab.title,
                                presentation::message(self.locale, "tab.contentUnavailable", &[])
                            ))
                            .size(UNAVAILABLE_TAB_FONT_SIZE)
                            .color(
                                self.editor
                                    .appearance
                                    .foreground
                                    .gamma_multiply(UNAVAILABLE_TAB_TEXT_OPACITY),
                            ),
                        )
                        .halign(egui::Align::Center),
                    );
                },
            );
            return;
        };
        if let crate::open_with::Surface::Preview(kind) = self.open_with.surface(path) {
            if self.target == Some(pane) {
                *self.focused = None;
            }
            let response = ui
                .allocate_ui(ui.available_size(), |ui| {
                    if kind == crate::open_with::PreviewKind::Hwp {
                        let output = crate::preview_hwp_surface::show_with_tooltips(
                            ui,
                            self.hwp_previews,
                            &tab.id,
                            path,
                            &tab.title,
                            self.locale,
                            self.pdf_appearance,
                        );
                        self.tooltips
                            .show_triggers(&output.tooltips, self.tooltip_appearance);
                        if output.external {
                            self.commands.push(HostCommand::OpenPath(path.clone()));
                        }
                        self.load_hwp_previews.push((
                            tab.id.clone(),
                            path.clone(),
                            ui.input(|input| input.max_texture_side),
                        ));
                        return;
                    }
                    if kind == crate::open_with::PreviewKind::Pdf {
                        let output = crate::preview_pdf_surface::show_with_tooltips(
                            ui,
                            self.pdf_previews,
                            &tab.id,
                            path,
                            &tab.title,
                            self.locale,
                            self.pdf_appearance,
                        );
                        self.tooltips
                            .show_triggers(&output.tooltips, self.tooltip_appearance);
                        if output.external {
                            self.commands.push(HostCommand::OpenPath(path.clone()));
                        }
                        self.load_pdf_previews.push((
                            tab.id.clone(),
                            path.clone(),
                            ui.input(|input| input.max_texture_side),
                        ));
                        return;
                    }
                    if kind == crate::open_with::PreviewKind::Presentation {
                        if crate::preview_presentation_surface::show(
                            ui,
                            self.presentation_previews,
                            &tab.id,
                            path,
                            &tab.title,
                            self.locale,
                            self.presentation_appearance,
                        ) {
                            self.commands.push(HostCommand::OpenPath(path.clone()));
                        }
                        self.load_presentation_previews
                            .push((tab.id.clone(), path.clone()));
                        return;
                    }
                    if kind == crate::open_with::PreviewKind::Spreadsheet {
                        if crate::preview_spreadsheet_surface::show(
                            ui,
                            self.spreadsheet_previews,
                            &tab.id,
                            path,
                            &tab.title,
                            self.locale,
                            self.spreadsheet_appearance,
                        ) {
                            self.commands.push(HostCommand::OpenPath(path.clone()));
                        }
                        self.load_spreadsheet_previews
                            .push((tab.id.clone(), path.clone()));
                        return;
                    }
                    if kind != crate::open_with::PreviewKind::Image {
                        if matches!(
                            kind,
                            crate::open_with::PreviewKind::Html
                                | crate::open_with::PreviewKind::Audio
                                | crate::open_with::PreviewKind::Video
                        ) {
                            if let Some(source) = self.web_previews.source(path) {
                                let bounds =
                                    ui.available_rect_before_wrap().intersect(ui.clip_rect());
                                self.web_placements
                                    .push(crate::preview_web_view::Placement {
                                        tab: tab.id.clone(),
                                        source: source.clone(),
                                        bounds,
                                        background: self
                                            .pdf_appearance
                                            .background
                                            .to_srgba_unmultiplied(),
                                        focused: self.target == Some(pane),
                                    });
                                ui.allocate_rect(bounds, egui::Sense::hover());
                            } else {
                                let appearance = crate::preview_status::Appearance {
                                    background: self.pdf_appearance.background,
                                    border: self.pdf_appearance.border,
                                    foreground: self.pdf_appearance.foreground,
                                    muted: self.pdf_appearance.muted,
                                };
                                if crate::preview_status::show(
                                    ui,
                                    &tab.id,
                                    self.web_previews.error(path),
                                    &tab.title,
                                    self.locale,
                                    &appearance,
                                    "preview.notSupported",
                                ) {
                                    self.commands.push(HostCommand::OpenPath(path.clone()));
                                }
                                self.load_web_previews.push(path.clone());
                            }
                            return;
                        }
                        ui.label(format!(
                            "native preview provider is not connected: {kind:?}"
                        ));
                        return;
                    }
                    if let Some(error) = self.previews.error(path) {
                        ui.label(presentation::message(
                            self.locale,
                            "preview.notSupported",
                            &[],
                        ));
                        ui.label(&tab.title);
                        ui.label(error.to_string());
                        if ui
                            .button(presentation::message(
                                self.locale,
                                "preview.openExternally",
                                &[],
                            ))
                            .clicked()
                        {
                            self.commands.push(HostCommand::OpenPath(path.clone()));
                        }
                        return;
                    }
                    if let Some(texture) = self.previews.advance(path, ui.ctx()) {
                        crate::preview::show_image(
                            ui,
                            texture,
                            path.rsplit('/').next().unwrap_or_default(),
                        );
                        return;
                    }
                    self.load_previews
                        .push((path.clone(), ui.input(|input| input.max_texture_side)));
                })
                .response;
            if response.contains_pointer() && ui.input(|input| input.pointer.any_pressed()) {
                intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
            }
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
        if let Some(error) = self.failed.get(path) {
            ui.label(error.to_string());
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
    fn tab_icon(&mut self, ui: &Ui, rect: egui::Rect, tab: &Tab, title_color: egui::Color32) {
        let (glyph, color) = crate::problems_icons::tab(&tab.kind);
        let color = color.map_or(title_color, |color| {
            self.explorer_appearance.file_color(color)
        });
        if let Err(error) = self.explorer_icons.paint(ui, rect, glyph, color, 0.0) {
            *self.status = Some(error.to_string());
        }
    }
    fn status_bar(&mut self, ui: &mut Ui, _: Option<&ProjectId>, _: &mut Vec<ShellIntent>) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = crate::lsp::status::ITEM_GAP;
            if let Err(error) = self.problems.show_status(
                ui,
                self.focused_slot,
                self.diagnostics,
                self.locale,
                self.problems_appearance,
            ) {
                *self.status = Some(error.to_string());
            }
            crate::lsp::status::show(
                ui,
                self.locale,
                self.lsp_status_appearance,
                self.lsp_summary,
            );
            if let Err(error) = crate::status_ide::show(
                ui,
                self.locale,
                self.status_ide_appearance,
                self.status_ide_icons,
                self.services.ide.status(),
            ) {
                *self.status = Some(error.to_string());
            }
            if let Some(status) = &self.status {
                ui.label(status);
            }
            crate::status_chord::show(
                ui,
                self.locale,
                self.status_chord_appearance,
                &self.chord_status,
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let settings = self.services.state.settings.read();
                let editor_font_size = settings.editor_font_size;
                let terminal_font_size = settings.terminal_font_size;
                drop(settings);
                crate::status_editor::show_font(
                    ui,
                    self.locale,
                    self.status_editor_appearance,
                    crate::status_editor::Target::Terminal,
                    terminal_font_size,
                    self.commands,
                    self.tooltips,
                );
                crate::status_editor::show_font(
                    ui,
                    self.locale,
                    self.status_editor_appearance,
                    crate::status_editor::Target::Editor,
                    editor_font_size,
                    self.commands,
                    self.tooltips,
                );
                match crate::system_usage_view::show_status(
                    ui,
                    self.locale,
                    self.system_usage_appearance,
                    self.system_usage_icon,
                    self.system_usage.summary(),
                    self.tooltips,
                ) {
                    Ok(Some(response)) if response.clicked() => {
                        self.system_usage.detail_open = true;
                        ui.ctx().request_repaint();
                    }
                    Ok(_) => {}
                    Err(error) => *self.status = Some(error.to_string()),
                }
                crate::status_editor::show_cursor(
                    ui,
                    self.locale,
                    self.status_editor_appearance,
                    crate::status_editor::cursor(self.store, self.status_view.as_ref()),
                );
            });
        });
    }
}

impl AppSurfaces<'_> {
    fn show_document(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        tab: &Tab,
        document: DocumentId,
        path: Option<&str>,
        intents: &mut Vec<ShellIntent>,
    ) {
        if path.is_some() {
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
            if let Ok(snapshot) = self.store.documents().snapshot(document)
                && snapshot.metadata.read_only
            {
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
        }
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
                let mut focus = Some(pane) == self.target
                    && self
                        .focused
                        .as_ref()
                        .is_none_or(|(old_pane, old_tab)| old_pane != pane || old_tab != &tab.id);
                let snapshot = self
                    .store
                    .documents()
                    .snapshot(document)
                    .map_err(editor_error)?;
                let settings = self.services.state.settings.read();
                let indent = taide_native_editor::indent::resolve(
                    &snapshot.metadata.editor_config,
                    taide_native_editor::indent::IndentOptions {
                        tab_size: settings.editor_tab_size,
                        insert_spaces: settings.editor_insert_spaces,
                    },
                );
                let mut editor_presentation =
                    crate::presentation_refresh::editor_presentation(&settings);
                editor_presentation.options.colors = Some(self.editor_display_colors);
                editor_presentation.options.bracket_colors = Some(self.editor_bracket_colors);
                editor_presentation.options.sticky_colors = Some(self.editor_sticky_colors);
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
                    focus = true;
                }
                let mut next = 0;
                let mut edit_errors = Vec::new();
                let text_resources = crate::editor_command_text::resources()?;
                let compare = |left: &str, right: &str| text_resources.compare(left, right);
                let syntax_lease =
                    crate::editor_syntax::SyntaxLease::new(&mut *self.editor_syntax, document);
                let language = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
                    .map(
                        |rules| taide_native_editor::language_configuration::Language {
                            rules,
                            syntax: &syntax_lease,
                        },
                    );
                let edited = ui.is_enabled()
                    && crate::command_dispatch::apply_document_edits(
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
                        self.document_edits,
                        &mut edit_errors,
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
                if find_output.reserved_height > 0.0 { ui.allocate_space(egui::vec2(ui.available_width(), find_output.reserved_height)); }
                let find_decorations = find.decorations(&self.store.views().get(view).ok_or_else(|| editor_error(taide_native_editor::document::EditorError::NotFound))?.selection,
                    self.find_appearance.highlight.to_array(), self.find_appearance.current_match.to_array(), self.find_appearance.scope.to_array());
                let find_layers = find_decorations.iter().collect::<Vec<_>>();
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
                let output = editor
                    .show_request(
                        ui,
                        self.store,
                        view,
                        EditorRequest {
                            request_focus: focus,
                            keymap: |ui: &Ui, event: &egui::Event, composing: bool| {
                                let index = crate::keymap::event_index(ui.ctx(), event, &mut next);
                                match terminal_views.route_keymap(
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
                                ) {
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
                            fold_controls: Some(&mut paint_fold_control),
                        },
                    )
                    .map_err(editor_error)?;
                if let Some(error) = fold_control_error {
                    *self.status = Some(error.to_string());
                }
                self.editor_syntax
                    .show_lines(document, output.rendered_lines.clone());
                if output.toggle_sticky_scroll {
                    intents.push(ShellIntent::ToggleEditorStickyScroll);
                }
                self.keymap_documents.insert(tab.id.clone(), document);
                if output.response.enabled() {
                    for id in &output.focus_ids {
                        self.editor_keymap_targets.insert(
                            (ui.ctx().viewport_id(), *id),
                            (view, ui.ctx().cumulative_frame_nr()),
                        );
                    }
                    self.editor_keymap_targets.insert(
                        (ui.ctx().viewport_id(), output.response.id),
                        (view, ui.ctx().cumulative_frame_nr()),
                    );
                }
                if output.response.has_focus() || ui.memory(|memory| output.focus_ids.iter().any(|id| memory.has_focus(*id))) {
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
                if output.changed || edited || find_edited {
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
                        self.commands.push(HostCommand::SetDirty {
                            tab,
                            dirty: snapshot.dirty,
                        });
                    }
                }
                for error in output.errors {
                    *self.status = Some(format!("native editor: {error:?}"));
                }
                Ok(())
            });
        if let Err(error) = result {
            *self.status = Some(error.to_string());
        }
    }
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Internal(format!("native editor: {error:?}"))
}

fn tokens_supplier<'tokens, Supplier>(supplier: Supplier) -> Supplier
where
    Supplier: FnOnce(&EditorStore) -> Option<EditorTokens<'tokens>>,
{
    supplier
}

fn connect_web(
    services: Arc<AppServices>,
    context: &egui::Context,
    executable: PathBuf,
    appearance: &crate::preview_pdf_surface::Appearance,
) -> AppResult<crate::preview_web_host::Bridge> {
    let repaint = context.clone();
    let bridge = crate::preview_web_host::Bridge::connect_lazy(
        services,
        Arc::new(move || repaint.request_repaint()),
        executable,
        WEB_HELPER_TIMEOUT,
        crate::preview_web_http::Limits {
            connections: WEB_CONNECTIONS,
            sources: MAX_DOCUMENTS,
            header_timeout: WEB_HEADER_TIMEOUT,
            idle_timeout: WEB_IDLE_TIMEOUT,
        },
    )?;
    bridge.set_media_appearance(media_appearance(appearance));
    Ok(bridge)
}

fn media_appearance(
    appearance: &crate::preview_pdf_surface::Appearance,
) -> crate::preview_web_media::Appearance {
    crate::preview_web_media::Appearance {
        background: appearance.background.to_srgba_unmultiplied(),
        foreground: appearance.foreground.to_srgba_unmultiplied(),
        muted: appearance.muted.to_srgba_unmultiplied(),
    }
}

#[cfg(test)]
#[path = "application-exit-tests.rs"]
mod exit_tests;

#[cfg(all(test, unix))]
#[path = "application-startup-tests.rs"]
mod startup_tests;

#[cfg(test)]
mod close_all_tests {
    use super::*;
    use crate::close_dialog::CloseChoice;
    use crate::tab_close_batch::{Batch, Task};

    #[test]
    fn close_all_cancel은_저장준비의_실패와_진행중을_구별하고_묶음_dialog를_닫는다() {
        const SCREEN: [f32; 2] = [800.0, 600.0];
        const NEXT_FRAME: f64 = 0.1;
        let make_tab = |title: &str| Tab {
            id: TabId::new(),
            kind: TabKind::Untitled { index: 1 },
            title: title.into(),
            pinned: false,
            preview: false,
            dirty: true,
            view_state: None,
        };
        let first = make_tab("first draft");
        let second = make_tab("second draft");
        let mut batch = Batch::new(vec![first.clone(), second.clone()]).unwrap();
        batch.choose(CloseChoice::Save);
        assert!(matches!(
            batch.next(),
            Some(Task::Prepare(_, CloseChoice::Save))
        ));
        let mut pending = PendingTabClose {
            tab: first,
            phase: TabClosePhase::Confirm,
            approved_discard: None,
            automatic_choice: Some(CloseChoice::Save),
            batch: Some(batch),
        };
        assert!(!pending.failed_batch_prepare());
        pending.automatic_choice = None;
        assert!(pending.failed_batch_prepare());
        pending.phase = TabClosePhase::Saving(None);
        assert!(!pending.failed_batch_prepare());
        pending.phase = TabClosePhase::Ready { discard: false };
        assert!(!pending.failed_batch_prepare());
        pending.phase = TabClosePhase::Confirm;
        let titles = pending.batch.as_ref().unwrap().titles().to_vec();
        pending.batch = None;
        assert!(!pending.failed_batch_prepare());
        let paths = taide_model::paths::AppPaths::new(
            std::env::temp_dir().join(format!("taide-close-all-locale-{}", ProjectId::new())),
        );
        let state = AppState::new(paths);
        for language in ["en", "ko", "ja"] {
            let locale =
                taide_runtime::locale_actions::locale_get_current(&state, language).unwrap();
            let context = egui::Context::default();
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                ..Default::default()
            };
            let mut output = context.run_ui(raw.clone(), |ui| {
                assert_eq!(
                    crate::close_dialog::show_titles(ui.ctx(), &locale, &titles, false),
                    None
                )
            });
            output.textures_delta.clear();
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(NEXT_FRAME),
                    events: vec![egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    }],
                    ..raw
                },
                |ui| {
                    assert_eq!(
                        crate::close_dialog::show_titles(ui.ctx(), &locale, &titles, false),
                        Some(CloseChoice::Cancel)
                    )
                },
            );
            output.textures_delta.clear();
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains("first draft, second draft") && !text.galley.text().contains("{{count}}"))));
        }
        assert!(!state.paths.data_dir.exists());
    }
}

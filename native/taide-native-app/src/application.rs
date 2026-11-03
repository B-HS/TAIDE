use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Ui};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::file::REFUSED_FILE_BYTES;
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::Tab;
#[cfg(test)]
use taide_model::layout::TabKind;
use taide_model::locale::ResolvedLocale;
use taide_model::tree::TreeRowPage;
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::conflict_banner::{BannerAction, BannerAppearance, BannerVariant};
use taide_native_ui::controller::ShellController;
use taide_native_ui::editor_surface::NativeEditor;
use taide_native_ui::shell::{NativeShell, ShellSurfaces, WindowScope};
use taide_runtime::{AppServices, AppState, EventSink, ExitDrain, TaskSupervisor, file_actions};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::bootstrap;
use crate::command_registry::{CommandContext, DocumentEdit, FoldCommand};
use crate::host::{HostBridge, HostCommand, HostReply};
use crate::presentation;

#[path = "domains/mod.rs"]
pub mod domains;

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
const UNAVAILABLE_TAB_FONT_SIZE: f32 = 14.0;
const UNAVAILABLE_TAB_TEXT_OPACITY: f32 = 0.6;

struct PaintSink {
    context: egui::Context,
    trees: Arc<crate::events::TreeChanges>,
    file_indexes: Arc<crate::command_palette::FileIndexChanges>,
    presentation: Arc<crate::presentation_refresh::Changes>,
    title_branches: Arc<crate::title_branch::Changes>,
    app_menu: Arc<crate::app_menu::Changes>,
    providers: Arc<crate::provider_events::Inbox>,
}

impl EventSink for PaintSink {
    fn publish(&self, event: AppEvent) {
        self.trees.record(&event);
        self.file_indexes.record(&event);
        self.presentation.record(&event);
        self.title_branches.record(&event);
        self.app_menu.record(&event);
        self.providers.record(&event);
        self.context.request_repaint();
    }
}

/// Construction-time replacements for host ports that tests must not reach for real.
#[derive(Default)]
pub(crate) struct Overrides {
    pub(crate) secret_store: Option<taide_infra::secret::SecretStoreState>,
    pub(crate) platform: Option<Arc<dyn taide_runtime::PlatformServices>>,
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
    native_options: Option<taide_lsp::native::protocol::lsp_types::FormattingOptions>,
    native_token: Option<uuid::Uuid>,
    untitled: Option<TabId>,
}

impl PendingFormatSave {
    fn new(
        path: String,
        auto_save: bool,
        epoch: crate::persistence::DraftEpoch,
        untitled: Option<TabId>,
    ) -> Self {
        Self {
            path,
            auto_save,
            epoch,
            native_options: None,
            native_token: None,
            untitled,
        }
    }
}

pub struct NativeApplication {
    runtime: Runtime,
    services: Arc<AppServices>,
    application_ports: Arc<crate::application_ports::Ports>,
    controller: ShellController,
    bridge: Option<HostBridge>,
    terminal_agent_state: domains::terminal_agent::State,
    preview_state: domains::preview::State,
    save_picker: crate::file_dialogs::SavePicker,
    lsp: Option<crate::lsp::LspBridge>,
    pending_format_saves: HashMap<DocumentId, PendingFormatSave>,
    workspace_activity: Option<crate::workspace_activity::Activity>,
    workspace_suspended: HashSet<DocumentId>,
    deleted_documents: HashSet<DocumentId>,
    resumed_save_epochs: HashMap<DocumentId, crate::persistence::DraftEpoch>,
    lsp_diagnostics: crate::diagnostics::Store,
    editor_symbols: crate::editor_symbols::State,
    editor_folding: crate::editor_folding::State,
    editor_json: crate::editor_json::State,
    editor_json_validation: crate::editor_json_validation::State,
    shell_state: domains::shell::State,
    problem_opens: Vec<crate::problems::Open>,
    editor_locations: crate::editor_locations::State,
    editor_documentation: crate::editor_documentation::State,
    editor_completion: crate::editor_completion::State,
    editor_highlights: crate::editor_highlights::State,
    editor_inlay_hints: crate::editor_inlay_hints::State,
    editor_code_lenses: crate::editor_code_lenses::State,
    editor_selection_ranges: crate::editor_selection_ranges::State,
    editor_view_state: crate::editor_view_state::State,
    editor_markdown: crate::editor_markdown::State,
    view_state_flush: Arc<crate::editor_view_state_sync::Shared>,
    editor_clipboard: crate::editor_context_menu::Clipboard,
    editor_menu_changes: HashMap<DocumentId, ViewId>,
    editor_terminal_writes: Vec<crate::editor_context_menu::TerminalDelivery>,
    peek_models: crate::peek_models::Models,
    explorer_search_state: domains::explorer_search::State,
    lsp_status_appearance: crate::lsp::status::Appearance,
    status_editor_appearance: crate::status_editor::Appearance,
    status_chord_appearance: crate::status_chord::Appearance,
    problems: crate::problems::Views,
    tooltips: crate::tooltips::Provider,
    tooltip_appearance: crate::tooltips::Appearance,
    problems_appearance: crate::problems::Appearance,
    shell: NativeShell,
    mirror_sweeps: crate::mirror_sweep::MirrorSweeps,
    commands_state: domains::commands::State,
    indentation_request: Option<crate::editor_indentation::Request>,
    editor_formatting: crate::editor_formatting::State,
    editor_rename: crate::editor_rename_controller::Controller,
    editor_code_actions: crate::editor_code_actions_controller::Controller,
    settings_state: domains::settings::State,
    toasts: crate::toast_layers::ToastLayers,
    toast_theme: taide_model::theme::ThemeType,
    motion_preference: crate::motion_preference::Preference,
    editor: NativeEditor,
    editor_find: HashMap<ViewId, taide_native_ui::editor_find::EditorFind>,
    editor_problems: crate::editor_problems::State,
    find_history: taide_native_ui::editor_find_widget::FindHistory,
    find_appearance: taide_native_ui::editor_find_widget::FindAppearance,
    editor_display_colors: taide_native_ui::editor_display::EditorDisplayColors,
    editor_bracket_colors: taide_native_ui::editor_brackets::EditorBracketColors,
    editor_sticky_colors: taide_native_ui::editor_sticky_scroll::EditorStickyColors,
    editor_minimap_colors: taide_native_ui::editor_minimap::EditorMinimapColors,
    editor_diagnostic_colors: taide_native_ui::editor_diagnostics::DiagnosticColors,
    editor_overview_colors: taide_native_ui::editor_overview::OverviewColors,
    editor_problem_colors: taide_native_ui::editor_problems::Colors,
    editor_location_colors: taide_native_ui::editor_locations::Colors,
    editor_documentation_colors: taide_native_ui::editor_documentation::Colors,
    editor_markdown_colors: taide_native_ui::editor_markup::Colors,
    editor_completion_colors: taide_native_ui::editor_completion::Colors,
    editor_ai_edit_colors: taide_native_ui::editor_ai_edit::Colors,
    editor_highlight_colors: crate::editor_highlights::Colors,
    editor_code_action_colors: taide_native_ui::editor_code_actions::Colors,
    editor_inlay_hint_colors: taide_native_ui::editor_inlay_hints::Colors,
    editor_code_lens_colors: taide_native_ui::editor_code_lenses::Colors,
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
    untitled: HashMap<TabId, UntitledDocument>,
    untitled_loading: HashSet<TabId>,
    untitled_failed: HashMap<TabId, AppError>,
    saving_untitled: Option<TabId>,
    missing_drafts: HashMap<String, crate::missing_draft::MissingDraft>,
    saving_missing_draft: Option<TabId>,
    loading: HashSet<String>,
    failed: HashMap<String, AppError>,
    refused: HashSet<String>,
    git_state: domains::git::State,
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
        warnings: Vec<String>,
    ) -> AppResult<Self> {
        Self::new_with_secret_store(
            context,
            runtime,
            state,
            tasks,
            helper_executable,
            warnings,
            None,
        )
    }

    #[cfg(target_os = "macos")]
    pub fn with_app_menu(mut self, context: &egui::Context) -> Self {
        let model = self.initial_app_menu_model();
        match crate::app_menu_bar::install(context.clone(), &model) {
            Ok((bar, events)) => {
                self.shell_state
                    .app_menu
                    .connect_installed(Box::new(bar), events, model)
            }
            Err(error) => log::warn!("native app menu install failed: {:?}", error.kind()),
        }
        self
    }

    pub(crate) fn new_with_secret_store(
        context: &eframe::CreationContext<'_>,
        runtime: Runtime,
        state: AppState,
        tasks: TaskSupervisor,
        helper_executable: PathBuf,
        warnings: Vec<String>,
        secret_store: Option<taide_infra::secret::SecretStoreState>,
    ) -> AppResult<Self> {
        Self::new_with_overrides(
            context,
            runtime,
            state,
            tasks,
            helper_executable,
            warnings,
            Overrides {
                secret_store,
                ..Overrides::default()
            },
        )
    }

    pub(crate) fn new_with_overrides(
        context: &eframe::CreationContext<'_>,
        runtime: Runtime,
        state: AppState,
        tasks: TaskSupervisor,
        helper_executable: PathBuf,
        mut warnings: Vec<String>,
        overrides: Overrides,
    ) -> AppResult<Self> {
        let egui = context.egui_ctx.clone();
        let repaint_context = egui.clone();
        let repaint: Arc<dyn Fn() + Send + Sync> =
            Arc::new(move || repaint_context.request_repaint());
        let tree_changes = Arc::new(crate::events::TreeChanges::default());
        let palette_file_changes = Arc::new(crate::command_palette::FileIndexChanges::default());
        let presentation_changes = Arc::new(crate::presentation_refresh::Changes::default());
        let title_branch_changes = Arc::new(crate::title_branch::Changes::default());
        let app_menu_changes = Arc::new(crate::app_menu::Changes::default());
        let provider_events = Arc::new(crate::provider_events::Inbox::default());
        let (mut assembly, connection) = runtime.block_on(bootstrap::connect_shell(
            state.clone(),
            tasks.clone(),
            Arc::new(PaintSink {
                context: egui.clone(),
                trees: tree_changes.clone(),
                file_indexes: palette_file_changes.clone(),
                presentation: presentation_changes.clone(),
                title_branches: title_branch_changes.clone(),
                app_menu: app_menu_changes.clone(),
                providers: provider_events.clone(),
            }),
            repaint.clone(),
        ))?;
        if overrides.secret_store.is_some() || overrides.platform.is_some() {
            let services = Arc::get_mut(&mut assembly.services).ok_or_else(|| {
                AppError::Internal("native credentials were shared before initialization".into())
            })?;
            if let Some(secret_store) = overrides.secret_store {
                services.secrets = secret_store;
            }
            if let Some(platform) = overrides.platform {
                services.platform = taide_runtime::PlatformServicesState(platform);
            }
        }
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
        let crate::status_error_banner::Startup {
            theme,
            locale,
            appearances,
            failures: presentation_failures,
        } = crate::status_error_banner::startup(&state, resolved, &presentation_inputs.settings)?;
        egui.set_fonts(fonts.definitions);
        warnings.extend(fonts.warnings);
        for warning in &warnings {
            log::warn!("{warning}");
        }
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
        let view_state_flush = application_ports.view_states.clone();
        view_state_flush.bind(repaint.clone());
        let before_mutation = view_state_flush.clone();
        connection
            .controller
            .set_before_mutation(Arc::new(move |state, events| {
                let capture = before_mutation.clone();
                Box::pin(async move { capture.flush_state(&state, events.as_ref()).await })
            }));
        let host = HostBridge::connect_with_application_ports(
            services.clone(),
            repaint.clone(),
            crate::host::Terminals {
                tabs: terminals.clone(),
                environment: crate::terminal_environment::provider(),
            },
            &application_ports,
        )?;
        let web_bridge = domains::preview::connect_web(
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
        host.submit(HostCommand::ExplorerSearch(
            domains::explorer_search::Command::RestoreWatchers,
        ))?;
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
        let toasts = crate::toast_layers::ToastLayers::new()?;
        let toast_theme = theme.theme_type;
        let tooltips = crate::tooltips::Provider::default();
        let mut settings_views = crate::settings_view::Views::default();
        settings_views.set_font_painter(crate::font_preview::Renderer::new(services.tasks.clone()));
        let project_order: Vec<ProjectId> = state
            .session
            .read()
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect();
        let auxiliary_windows = crate::auxiliary_windows::Windows::restore(
            &project_order,
            &state.projects.read(),
            &state.layouts.read(),
            &services.windows,
        );
        let status_ide_icons = crate::status_ide::Icons::with_tooltips(tooltips.clone())?;
        let system_usage_icon = crate::system_usage_view::Icon::new()?;
        let application = Self {
            runtime,
            services,
            application_ports,
            controller: connection.controller,
            bridge,
            terminal_agent_state: domains::terminal_agent::State {
                terminals,
                terminal_views,
                terminal_appearance,
                terminal_fonts: crate::terminal_fonts::Loader::new(font_families),
                editor_ide_selection: crate::editor_ide_selection::State::default(),
                status_ide_appearance: appearances.status_ide,
                status_ide_icons,
                command_colors: None,
                cli_status: None,
            },
            preview_state: domains::preview::State {
                web_bridge: Some(web_bridge),
                helper_executable,
                web_previews: crate::preview_web_cache::Cache::default(),
                web_views: crate::preview_web_view::Views::default(),
                open_with: crate::open_with::Registry::default(),
                previews: crate::preview::Cache::default(),
                hwp_previews: crate::preview_hwp::Cache::default(),
                pdf_previews: crate::preview_pdf::Cache::default(),
                pdf_appearance,
                presentation_previews: crate::preview_presentation_cache::Cache::default(),
                presentation_appearance,
                spreadsheet_previews: crate::preview_spreadsheet_cache::Cache::default(),
                spreadsheet_appearance,
            },
            save_picker: Arc::new(crate::file_dialogs::save),
            lsp: Some(lsp),
            pending_format_saves: HashMap::new(),
            workspace_activity: None,
            workspace_suspended: HashSet::new(),
            deleted_documents: HashSet::new(),
            resumed_save_epochs: HashMap::new(),
            lsp_diagnostics: crate::diagnostics::Store::default(),
            editor_symbols: crate::editor_symbols::State::default(),
            editor_folding: crate::editor_folding::State::default(),
            editor_json: crate::editor_json::State::default(),
            editor_json_validation: crate::editor_json_validation::State::default(),
            problem_opens: Vec::new(),
            editor_locations: crate::editor_locations::State::default(),
            editor_documentation: crate::editor_documentation::State::default(),
            editor_completion: crate::editor_completion::State::default(),
            editor_highlights: crate::editor_highlights::State::default(),
            editor_inlay_hints: crate::editor_inlay_hints::State::default(),
            editor_code_lenses: crate::editor_code_lenses::State::default(),
            editor_selection_ranges: crate::editor_selection_ranges::State::default(),
            editor_view_state: crate::editor_view_state::State::default(),
            editor_markdown: crate::editor_markdown::State::default(),
            view_state_flush,
            editor_clipboard: crate::editor_context_menu::Clipboard::default(),
            editor_menu_changes: HashMap::new(),
            editor_terminal_writes: Vec::new(),
            peek_models: crate::peek_models::Models::default(),
            lsp_status_appearance: appearances.lsp_status,
            status_editor_appearance: appearances.status_editor,
            status_chord_appearance: appearances.status_chord,
            problems: crate::problems::Views::with_tooltips(&collation_locale, tooltips.clone())?,
            tooltips,
            tooltip_appearance: appearances.tooltip,
            problems_appearance: appearances.problems,
            explorer_search_state: domains::explorer_search::State {
                workspace_symbols: crate::workspace_symbols::State::default(),
                explorer_appearance: appearances.explorer,
                explorer_icons: crate::problems_icons::Icons::new()?,
                navigation_appearance: appearances.navigation,
                navigation_icons: crate::navigation_icons::Icons::new()?,
                symbol_sidebars: crate::symbol_sidebar::Views::default(),
                breadcrumbs: crate::breadcrumbs::Views::default(),
                trees: HashMap::new(),
                explorers: HashMap::new(),
                explorer_clipboards: crate::explorer_clipboard_owners::ClipboardOwners::new(
                    egui.viewport_id(),
                ),
                pending_entry_delete: None,
                deleting_entry: false,
                tree_loading: HashSet::new(),
                tree_changes,
            },
            shell,
            shell_state: domains::shell::State {
                welcome: crate::welcome::State::default(),
                app_menu: crate::app_menu::State::new(app_menu_changes),
                open_path_dialog: Default::default(),
                project_display_dialog: Default::default(),
                project_group_dialog: Default::default(),
                project_group_delete: Default::default(),
                system_usage,
                system_usage_appearance: appearances.system_usage,
                system_usage_icon,
                zen_hint: crate::zen::Hint::default(),
                zen_hint_colors: appearances.zen_hint,
                file_drop_colors: appearances.file_drop,
                auxiliary_windows,
                window_state: crate::window_state::Tracker::new(&state.paths.data_dir),
                auxiliary_pass_root: None,
                overlay_viewport: egui::ViewportId::ROOT,
                zen_fullscreen_state,
                provider_events,
                notifications: Default::default(),
                ide_sync: Default::default(),
                main_window_zoomed: None,
            },
            mirror_sweeps: crate::mirror_sweep::MirrorSweeps::default(),
            commands_state: domains::commands::State {
                keybindings,
                palette,
                palette_files: Default::default(),
                palette_file_changes,
                palette_commands: Vec::new(),
                task_runner: None,
            },
            indentation_request: None,
            editor_formatting: crate::editor_formatting::State::new(egui.clone()),
            editor_rename: Default::default(),
            editor_code_actions: Default::default(),
            settings_state: domains::settings::State {
                settings_views,
                app_file_views: crate::app_file_views::Views::default(),
                settings_appearance: appearances.settings,
                resolved_theme: theme,
                preview_theme: None,
                presentation_changes,
                presentation_refresh: crate::presentation_refresh::Refresh::new(
                    presentation_inputs,
                    presentation_revision,
                ),
                presentation_failures,
                presentation_system_language: system_language,
            },
            toasts,
            toast_theme,
            motion_preference: crate::motion_preference::Preference::new(egui.clone()),
            editor,
            editor_find: HashMap::new(),
            editor_problems: crate::editor_problems::State::default(),
            find_history: Default::default(),
            find_appearance: appearances.find,
            editor_display_colors: appearances.editor_display,
            editor_bracket_colors: appearances.editor_brackets,
            editor_sticky_colors: appearances.editor_sticky,
            editor_minimap_colors: appearances.editor_minimap,
            editor_diagnostic_colors: appearances.editor_diagnostics,
            editor_overview_colors: appearances.editor_overview,
            editor_problem_colors: appearances.editor_problems,
            editor_location_colors: appearances.editor_locations,
            editor_documentation_colors: appearances.editor_documentation,
            editor_markdown_colors: appearances.editor_markdown,
            editor_completion_colors: appearances.editor_completion,
            editor_ai_edit_colors: appearances.editor_ai_edit,
            editor_highlight_colors: appearances.editor_highlights,
            editor_code_action_colors: appearances.editor_code_actions,
            editor_inlay_hint_colors: appearances.editor_inlay_hints,
            editor_code_lens_colors: appearances.editor_code_lenses,
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
            untitled: HashMap::new(),
            untitled_loading: HashSet::new(),
            untitled_failed: HashMap::new(),
            saving_untitled: None,
            missing_drafts: HashMap::new(),
            saving_missing_draft: None,
            loading: HashSet::new(),
            failed: HashMap::new(),
            refused: HashSet::new(),
            git_state: domains::git::State {
                title_branch_changes,
                title_branches: Default::default(),
            },
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
        self.poll_preview_web();
        if let Some(error) = self.controller.take_error() {
            self.report(&error);
        }
        self.poll_settings_revisions();
        while let Some(reply) = self.bridge.as_mut().and_then(HostBridge::poll) {
            match reply {
                HostReply::Shell(reply) => self.apply_shell_reply(context, reply),
                HostReply::Commands(reply) => self.apply_commands_reply(reply),
                HostReply::Editor(reply) => self.apply_editor_reply(context, reply),
                HostReply::ExplorerSearch(reply) => {
                    self.apply_explorer_search_reply(context, reply)
                }
                HostReply::Git(reply) => self.apply_git_reply(context, reply),
                HostReply::TerminalAgent(reply) => self.apply_terminal_agent_reply(context, reply),
                HostReply::Settings(reply) => self.apply_settings_reply(context, reply),
                HostReply::Preview(reply) => self.apply_preview_reply(context, reply),
                HostReply::SettingsFailed(error) => {
                    self.toasts
                        .settings_failed(&self.locale, &error, Instant::now())
                }
                HostReply::Failed(error) => self.report(&error),
            }
        }
    }

    fn apply_file_link_reply(
        &mut self,
        context: &egui::Context,
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    ) {
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
                        self.controller
                            .apply_layouts(HashMap::from([(opened.project.clone(), layout)]));
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

    fn apply_current_file_link_reply(
        &mut self,
        context: &egui::Context,
        result: AppResult<crate::terminal_tabs::OpenedFileLink>,
    ) {
        if self.closing.is_none() && !self.services.state.is_shutting_down() {
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
                    context.request_repaint_after(crate::editor_reveal::REVEAL_TTL);
                }
                Ok(_) => {}
                Err(error) => self.report(&error),
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

    fn run_toast_actions(&mut self) {
        for action in self.toasts.take_actions() {
            if !self.flush_dirty() {
                return;
            }
            match action {
                crate::toast::Action::RetryCreate { project, request } => {
                    let retried = self
                        .explorer_search_state
                        .trees
                        .get(&project)
                        .and_then(|page| {
                            self.explorer_search_state
                                .explorers
                                .get_mut(&project)?
                                .retry_create(&request, &page.rows, &self.locale)
                        });
                    if let Some(request) = retried {
                        self.create_entry(project, request);
                    }
                }
                crate::toast::Action::RetryRename { project, request } => {
                    let retried = self
                        .explorer_search_state
                        .trees
                        .get(&project)
                        .and_then(|page| {
                            self.explorer_search_state
                                .explorers
                                .get_mut(&project)?
                                .retry_rename(&request, &page.rows, &self.locale)
                        });
                    if let Some(request) = retried {
                        self.rename_entry(project, request);
                    }
                }
            }
        }
    }

    fn submit(&mut self, command: HostCommand) -> bool {
        let command = match self.intercept_editor_command(command) {
            ControlFlow::Continue(command) => command,
            ControlFlow::Break(submitted) => return submitted,
        };
        let command = match self.intercept_settings_command(command) {
            ControlFlow::Continue(command) => command,
            ControlFlow::Break(submitted) => return submitted,
        };
        let is_settings = domains::settings::is_settings_command(&command);
        let (command, tracked_save) = match self.prepare_editor_submission(command) {
            ControlFlow::Continue(prepared) => prepared,
            ControlFlow::Break(submitted) => return submitted,
        };
        let file_resolution = match &command {
            HostCommand::TerminalAgent(
                domains::terminal_agent::Command::ResolveTerminalFileLinks {
                    owner, request, ..
                },
            ) => Some((owner.clone(), request.clone())),
            _ => None,
        };
        let theme_operation = match &command {
            HostCommand::Settings(domains::settings::Command::ThemeEdit(command)) => {
                Some(command.clone())
            }
            _ => None,
        };
        let snippet_operation = match &command {
            HostCommand::Settings(domains::settings::Command::SnippetEdit(request)) => {
                Some(request.clone())
            }
            _ => None,
        };
        let snippet_catalog = match &command {
            HostCommand::Settings(domains::settings::Command::ReadSnippetCatalog(request)) => {
                Some(request.clone())
            }
            _ => None,
        };
        let result = self
            .bridge
            .as_ref()
            .ok_or_else(|| AppError::Forbidden("native host is closing".into()))
            .and_then(|bridge| bridge.submit(command));
        if let Err(error) = result {
            if let Some(request) = snippet_catalog {
                self.settings_state
                    .settings_views
                    .accept_snippet_catalog(request.failed(error), Instant::now());
                return false;
            }
            if let Some(request) = snippet_operation {
                self.settings_state
                    .settings_views
                    .accept_snippet(request.failed(error));
                return false;
            }
            if let Some(command) = theme_operation {
                self.toasts.ipc_error(&self.locale, &error, Instant::now());
                self.settings_state
                    .settings_views
                    .accept_theme(command.failed(error));
                return false;
            }
            if let Some((owner, request)) = file_resolution {
                self.terminal_agent_state
                    .terminal_views
                    .cancel_file_links(&owner, &request);
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
        let view_states = self.capture_view_states();
        self.view_state_flush.finish(view_states.clone());
        self.settings_state.app_file_views.clear();
        self.editor_completion.clear();
        self.editor_highlights.clear();
        self.editor_inlay_hints.clear();
        self.editor_code_lenses.clear();
        self.editor_selection_ranges.clear(&mut self.store);
        self.editor_rename.clear();
        self.editor_code_actions.clear();
        if let Err(error) = self
            .editor_documentation
            .clear(&mut self.store, &mut self.editor_syntax)
        {
            self.report(&editor_error(error));
        }
        let bridge = self.bridge.take().map(HostBridge::disconnect);
        let lsp = self.lsp.take().map(crate::lsp::LspBridge::disconnect);
        let web = self
            .preview_state
            .web_bridge
            .take()
            .map(crate::preview_web_host::Bridge::disconnect);
        self.clear_builtin_json();
        let syntax = self.editor_syntax.disconnect();
        self.preview_state.web_previews.invalidate_all();
        self.preview_state.web_views.clear();
        let drafts = self.drafts();
        let services = self.services.clone();
        let repaint = context.clone();
        let (sender, receiver) = oneshot::channel();
        self.closing = Some(receiver);
        self.runtime.spawn(async move {
            let (result, ()) = tokio::join!(
                shutdown(services, bridge, lsp, web, drafts, view_states),
                crate::editor_syntax::finished(syntax),
            );
            drop(sender.send(result));
            repaint.request_repaint();
        });
    }
}

impl NativeApplication {
    fn toast_interaction_enabled(&self) -> bool {
        self.closing.is_none()
            && !self.shell_state.system_usage.detail_open
            && !self.commands_state.keybindings.is_open()
            && !self.commands_state.palette.is_open()
            && self.pending_tab_close.is_none()
            && self.explorer_search_state.pending_entry_delete.is_none()
    }

    fn background_tick(&mut self, context: &egui::Context) {
        self.tick_shell_menus(context);
        self.toasts
            .set_reduced_motion(self.motion_preference.current().unwrap_or(false));
        let toast_enabled = self.toast_interaction_enabled();
        let toast_position = self.services.state.settings.read().toast_position.clone();
        let toast_now = Instant::now();
        self.toasts
            .main()
            .set_position(context, &toast_position, toast_now, toast_enabled);
        self.toasts.main().tick(context, toast_now, toast_enabled);
        if self.closing.is_some() || self.services.state.is_shutting_down() {
            self.terminal_agent_state.terminal_views.cancel_inputs();
            self.reveals.clear();
        } else if let Err(error) = self.terminal_agent_state.terminal_views.flush_inputs(
            self.terminal_agent_state.terminals.hub(),
            &self.services,
            context,
        ) {
            self.status = Some(error.to_string());
        }
        self.tick_terminal_agent_fonts(context);
        let editor_font_size = self.services.state.settings.read().editor_font_size;
        if presentation::update_editor_font_size(&mut self.editor.appearance, editor_font_size) {
            context.request_repaint();
        }
        self.tick_terminal_agent_cursors(context);
        self.tick_terminal_agent_live_sessions(context);
        self.reconcile_previews();
        self.poll(context);
        self.consume_provider_events(context, Instant::now());
        self.sync_theme_preview(context);
        if self.closing.is_some() || self.is_exit_ready || self.services.state.is_shutting_down() {
            self.settings_state.presentation_refresh.cancel();
            self.settings_state.settings_views.clear();
            self.settings_state.app_file_views.clear();
        } else {
            self.editor_syntax.follow_plugins(
                &self.services.state,
                &self.services.plugin,
                &self.services.tasks,
            );
            let syntax_theme = self
                .settings_state
                .preview_theme
                .as_ref()
                .unwrap_or(&self.settings_state.resolved_theme);
            if let Some(delay) = self
                .editor_syntax
                .tick(&self.store, syntax_theme, Instant::now())
            {
                context.request_repaint_after(delay);
            }
            self.tick_settings_reads(context);
        }
        self.reveals
            .reconcile(&self.services.state.layouts.read(), Instant::now());
        let auxiliary_explorers = self.shell_state.auxiliary_windows.explorer_mounts();
        self.explorer_search_state
            .explorer_clipboards
            .reconcile_windows(
                self.controller.snapshot().shell.tree.as_ref(),
                &auxiliary_explorers.keys().cloned().collect(),
            );
        self.problems
            .reconcile(self.controller.snapshot().shell.tree.as_ref());
        self.explorer_search_state.symbol_sidebars.reconcile(
            self.controller.snapshot().shell.tree.as_ref(),
            &auxiliary_explorers,
        );
        self.explorer_search_state.breadcrumbs.reconcile(
            context,
            &self.services.state.layouts.read(),
            self.controller.snapshot().shell.tree.as_ref(),
            &self.shell.scope,
        );
        self.reconcile_builtin_json(context);
        if self.closing.is_none() && !self.services.state.is_shutting_down() {
            self.editor_selection_ranges
                .tick_native(&self.store, &self.services.tasks, context);
        }
        self.reconcile_lsp();
        self.poll_native_formatting();
        self.dispatch_completion(context);
        if let Some(delay) = self.editor_symbols.next_refresh(Instant::now()) {
            context.request_repaint_after(delay);
        }
        if let Some(delay) = self.editor_folding.next_refresh(Instant::now()) {
            context.request_repaint_after(delay);
        }
        self.poll_lsp();
        if let Some(lsp) = &self.lsp
            && let Some(error) = self.editor_rename.poll(&mut self.store, lsp)
        {
            self.status = Some(crate::toast::describe_error(&self.locale, &error));
            self.toasts.ipc_error(&self.locale, &error, Instant::now());
        }
        if let Some(lsp) = &self.lsp {
            self.editor_code_actions.dispatch(lsp, Instant::now());
            if let Some(delay) = self.editor_code_actions.next_refresh(Instant::now()) {
                context.request_repaint_after(delay);
            }
            let (error, finished) = self.editor_code_actions.poll(&self.store, lsp);
            if let Some(error) = error {
                self.status = Some(crate::toast::describe_error(&self.locale, &error));
                self.toasts.ipc_error(&self.locale, &error, Instant::now());
            }
            if let Some(finished) = finished {
                self.consume_code_action_navigation(finished);
            }
        }
        self.terminal_agent_state
            .terminal_views
            .chord_status(context, Instant::now());
        self.tick_shell_system_usage(context);
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
        self.fulfill_view_states();
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
                    self.view_state_flush.resume();
                    if !self.services.state.is_shutting_down() {
                        self.loading.clear();
                        self.reconnect_explorer_search();
                        self.reconnect_git();
                        self.reconnect_shell();
                        self.reconnect_commands();
                        self.reconnect_preview(context);
                        let repaint_context = context.clone();
                        match HostBridge::connect_with_application_ports(
                            self.services.clone(),
                            Arc::new(move || repaint_context.request_repaint()),
                            crate::host::Terminals {
                                tabs: self.terminal_agent_state.terminals.clone(),
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
        let zoom_factor = context.zoom_factor();
        let is_zoomed = self.shell_state.main_window_zoomed;
        context.input(|input| {
            self.shell_state
                .window_state
                .observe(input.viewport(), zoom_factor, is_zoomed)
        });
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
        self.tick_commands_files(&snapshot);
        if self.closing.is_none() && !self.is_exit_ready && !self.services.state.is_shutting_down()
        {
            let open_projects: HashSet<_> = snapshot.layouts.keys().cloned().collect();
            self.tick_explorer_search_trees(context, &open_projects);
            for project in self.mirror_sweeps.due(
                snapshot.shell.tree.as_ref(),
                &self.shell_state.auxiliary_windows.explorer_mounts(),
                &snapshot.layouts,
            ) {
                if !self.submit(HostCommand::Editor(domains::editor::Command::PruneMirrors(
                    project.clone(),
                ))) {
                    self.mirror_sweeps.retry(&project);
                    context.request_repaint();
                }
            }
            self.tick_git_title_branches(context, &snapshot, &open_projects);
        }
        if !self.workspace_busy() {
            self.flush_observations();
            self.flush_persistence(context);
        }
    }
}

impl eframe::App for NativeApplication {
    fn raw_input_hook(&mut self, context: &egui::Context, input: &mut egui::RawInput) {
        self.shell_state
            .app_menu
            .inject(input.viewport_id, &mut input.events);
        self.terminal_agent_state
            .terminal_views
            .raw_input(context, input);
    }

    fn raw_input_hook_with_replay(
        &mut self,
        context: &egui::Context,
        input: &mut egui::RawInput,
        replayed_events: usize,
    ) {
        self.shell_state
            .app_menu
            .inject(input.viewport_id, &mut input.events);
        self.terminal_agent_state
            .terminal_views
            .raw_input_with_replay(context, input, replayed_events);
    }

    fn logic(&mut self, context: &egui::Context, frame: &mut eframe::Frame) {
        self.shell_state.main_window_zoomed = crate::window_state::is_zoomed(frame);
        self.background_tick(context);
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.frame_ui(ui, frame);
    }

    fn on_exit(&mut self) {
        if let Err(error) = self.shell_state.window_state.save() {
            log::warn!("native window state save failed: {:?}", error.kind());
        }
        self.exit_application();
    }
}

pub(crate) struct WindowPass {
    snapshot: Arc<taide_native_ui::snapshot::ShellSnapshot>,
    document_edits: Vec<(TabId, DocumentEdit)>,
    fold_commands: Vec<(TabId, FoldCommand)>,
    web_placements: Vec<crate::preview_web_view::Placement>,
    root_enabled: bool,
    overlay: egui::ViewportId,
}

impl NativeApplication {
    fn frame_ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        if !self.is_any_window_focused(&context) {
            self.terminal_agent_state
                .terminal_views
                .clear_keymap_chord(&context);
        }
        self.tick_preview_media();
        let snapshot = self.controller.snapshot();
        self.terminal_agent_state
            .editor_ide_selection
            .begin_frame(self.services.ide.is_running());
        self.editor_view_state.begin_frame(&self.store);
        self.editor_markdown
            .reconcile(&context, &snapshot.layouts, &self.store);
        self.tick_explorer_search_clipboards(&snapshot);
        if self.closing.is_none() && !self.services.state.is_shutting_down() {
            self.shell_state.zen_fullscreen_state.reconcile(
                &context,
                snapshot.shell.window_chrome.zen,
                self.services.state.settings.read().zen_fullscreen,
            );
        }
        self.tick_terminal_agent_views(&snapshot);
        let scopes = self.window_scopes();
        let (document_edits, fold_commands) =
            self.reconcile_editor_features(&context, &snapshot, &scopes);
        self.settings_state.settings_views.begin_frame();
        let mut pass = WindowPass {
            snapshot,
            document_edits,
            fold_commands,
            web_placements: Vec::new(),
            root_enabled: false,
            overlay: self.overlay_viewport(&context),
        };
        let main_proceeded = self.show_window(ui, frame, &mut pass);
        let auxiliary_proceeded = self.show_auxiliary_windows(&context, frame, &mut pass);
        self.shell_state.app_menu.finish_child_passes();
        self.finish_window_frames(&context);
        if main_proceeded && auxiliary_proceeded {
            self.finish_frame(&context, frame, pass);
        }
    }

    fn finish_window_frames(&mut self, context: &egui::Context) {
        self.tick_terminal_agent_ide_selection(context);
        self.settings_state.settings_views.finish_frame();
        self.sync_theme_preview(context);
    }

    fn window_command_context(
        &self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) -> (
        Option<PaneId>,
        Option<ProjectId>,
        Option<ViewKey>,
        CommandContext,
    ) {
        let editor_project = match &self.shell.scope {
            WindowScope::Main => snapshot.focused_project(),
            WindowScope::Auxiliary { project, .. } => Some(project),
        };
        let window = editor_project.and_then(|project| {
            let layout = snapshot.layouts.get(project)?;
            crate::symbol_sidebar::window_tree(project, layout, &self.shell.scope)
        });
        let target = window.map(|(_, pane)| pane.clone());
        let focused_view = window.and_then(|(root, pane)| {
            taide_native_ui::snapshot::active_tab(root, pane).map(|tab| ViewKey {
                window: WINDOW_LABEL.into(),
                pane: pane.clone(),
                tab: tab.id.clone(),
            })
        });
        let command_context = crate::command_dispatch::context(
            &snapshot,
            &self.shell.scope,
            crate::command_dispatch::active_editor_actions(&self.store, focused_view.as_ref()).map(
                |mut actions| {
                    let writable_source = focused_view
                        .as_ref()
                        .and_then(|key| {
                            let fallback = self
                                .store
                                .views()
                                .find(key)
                                .and_then(|view| self.store.views().get(view))
                                .map(|view| view.document);
                            self.focused_editor_source(&context, &key.tab, fallback)
                        })
                        .and_then(|(source, _)| self.store.views().get(source))
                        .and_then(|source| self.store.documents().snapshot(source.document).ok())
                        .is_some_and(|document| !document.metadata.read_only);
                    if writable_source {
                        actions.extend([
                            "editor.action.formatDocument".to_owned(),
                            "editor.action.formatSelection".to_owned(),
                            "editor.action.rename".to_owned(),
                            "editor.action.quickFix".to_owned(),
                            "editor.action.refactor".to_owned(),
                            "editor.action.sourceAction".to_owned(),
                            "editor.action.organizeImports".to_owned(),
                            "editor.action.fixAll".to_owned(),
                            "editor.action.autoFix".to_owned(),
                        ]);
                    }
                    actions.retain(|action| {
                        if action == "codelens.showLensesInCurrentLine" {
                            return self.services.state.settings.read().editor_code_lens_enabled
                                && focused_view
                                    .as_ref()
                                    .and_then(|key| {
                                        let fallback = self
                                            .store
                                            .views()
                                            .find(key)
                                            .and_then(|view| self.store.views().get(view))
                                            .map(|view| view.document);
                                        self.focused_editor_source(&context, &key.tab, fallback)
                                    })
                                    .and_then(|(source, _)| self.store.views().get(source))
                                    .and_then(|source| {
                                        self.store.documents().snapshot(source.document).ok()
                                    })
                                    .zip(editor_project)
                                    .is_some_and(|(document, project)| {
                                        self.lsp.as_ref().is_some_and(|lsp| {
                                            !lsp.code_lens_providers(project, &document).is_empty()
                                        })
                                    });
                        }
                        if let Some(command) =
                            taide_native_ui::editor_code_actions::Command::from_action(action)
                        {
                            return writable_source
                                && focused_view
                                    .as_ref()
                                    .and_then(|key| {
                                        let fallback = self
                                            .store
                                            .views()
                                            .find(key)
                                            .and_then(|view| self.store.views().get(view))
                                            .map(|view| view.document);
                                        self.focused_editor_source(&context, &key.tab, fallback)
                                    })
                                    .and_then(|(source, _)| self.store.views().get(source))
                                    .and_then(|source| {
                                        self.store.documents().snapshot(source.document).ok()
                                    })
                                    .zip(editor_project)
                                    .is_some_and(|(document, project)| {
                                        self.lsp.as_ref().is_some_and(|lsp| {
                                            !lsp.code_action_providers(project, &document, command)
                                                .is_empty()
                                        })
                                    });
                        }
                        if action == "editor.action.rename" {
                            return writable_source
                                && focused_view
                                    .as_ref()
                                    .and_then(|key| {
                                        let fallback = self
                                            .store
                                            .views()
                                            .find(key)
                                            .and_then(|view| self.store.views().get(view))
                                            .map(|view| view.document);
                                        self.focused_editor_source(&context, &key.tab, fallback)
                                    })
                                    .and_then(|(source, _)| self.store.views().get(source))
                                    .and_then(|source| {
                                        self.store.documents().snapshot(source.document).ok()
                                    })
                                    .zip(editor_project)
                                    .is_some_and(|(document, project)| {
                                        self.lsp.as_ref().is_some_and(|lsp| {
                                            !lsp.rename_providers(project, &document).is_empty()
                                        })
                                    });
                        }
                        if let Some(command) =
                            taide_native_editor::formatting::Command::from_action(action)
                        {
                            return focused_view
                                .as_ref()
                                .and_then(|key| {
                                    let fallback = self
                                        .store
                                        .views()
                                        .find(key)
                                        .and_then(|view| self.store.views().get(view))
                                        .map(|view| view.document);
                                    self.focused_editor_source(&context, &key.tab, fallback)
                                })
                                .and_then(|(source, _)| self.store.views().get(source))
                                .and_then(|source| {
                                    self.store.documents().snapshot(source.document).ok()
                                })
                                .filter(|document| !document.metadata.read_only)
                                .zip(editor_project)
                                .is_some_and(|(document, project)| {
                                    self.lsp.as_ref().is_some_and(|lsp| {
                                        !lsp.formatting_providers(project, &document, command)
                                            .is_empty()
                                    }) || crate::editor_json_formatting::is_eligible(&document)
                                });
                        }
                        if let Some(command) =
                            crate::command_registry::HighlightCommand::from_action(action)
                        {
                            if command == crate::command_registry::HighlightCommand::Trigger {
                                return true;
                            }
                            return focused_view
                                .as_ref()
                                .and_then(|key| self.store.views().find(key))
                                .is_some_and(|owner| {
                                    let source = self.editor_highlights.source_for_owner(
                                        &self.store,
                                        context.viewport_id(),
                                        owner,
                                    );
                                    self.editor_highlights.has_highlights(
                                        &self.store,
                                        context.viewport_id(),
                                        source,
                                    )
                                });
                        }
                        if taide_native_editor::documentation::Command::from_action(action)
                            == Some(taide_native_editor::documentation::Command::Signature(
                                taide_native_editor::documentation::SignatureCommand::Trigger,
                            ))
                        {
                            return focused_view
                                .as_ref()
                                .and_then(|key| self.store.views().find(key))
                                .and_then(|view| self.store.views().get(view))
                                .and_then(|view| {
                                    self.store.documents().snapshot(view.document).ok()
                                })
                                .zip(editor_project)
                                .is_some_and(|(document, project)| {
                                    self.lsp.as_ref().is_some_and(|lsp| {
                                        !lsp.documentation_providers(
                                            project,
                                            &document,
                                            crate::editor_documentation::Kind::Signature,
                                        )
                                        .is_empty()
                                    })
                                });
                        }
                        let Some(taide_native_editor::symbol_locations::Command::Request {
                            kind,
                            mode,
                        }) = taide_native_editor::symbol_locations::Command::from_action(action)
                        else {
                            return true;
                        };
                        focused_view
                            .as_ref()
                            .and_then(|key| {
                                let fallback = self
                                    .store
                                    .views()
                                    .find(key)
                                    .and_then(|view| self.store.views().get(view))
                                    .map(|view| view.document);
                                self.focused_editor_source(&context, &key.tab, fallback)
                            })
                            .filter(|(source, owner)| {
                                source == owner
                                    || crate::editor_locations::supports_embedded_request(
                                        kind, mode,
                                    )
                            })
                            .and_then(|(source, _)| self.store.views().get(source))
                            .and_then(|view| self.store.documents().snapshot(view.document).ok())
                            .zip(editor_project)
                            .is_some_and(|(document, project)| {
                                self.lsp.as_ref().is_some_and(|lsp| {
                                    !lsp.location_providers(project, &document, kind).is_empty()
                                })
                            })
                    });
                    actions
                },
            ),
        );
        (
            target,
            editor_project.cloned(),
            focused_view,
            command_context,
        )
    }

    fn show_window(
        &mut self,
        ui: &mut Ui,
        frame: &mut eframe::Frame,
        frame_pass: &mut WindowPass,
    ) -> bool {
        let context = ui.ctx().clone();
        let snapshot = frame_pass.snapshot.clone();
        self.tick_explorer_search_breadcrumbs(&context, &snapshot);
        let (target, editor_project, focused_view, command_context) =
            self.window_command_context(&context, &snapshot);
        self.terminal_agent_state
            .terminal_views
            .set_command_context(command_context.clone());
        let mut command_errors = Vec::new();
        let mut commands = Vec::new();
        let mut settings_output = domains::settings::FrameOutput::default();
        let mut keymap_actions = Vec::new();
        let mut keymap_editor_scope = None;
        let mut keymap_documents = HashMap::new();
        let mut explorer_output = domains::explorer_search::FrameOutput::default();
        let mut load_files = Vec::new();
        let mut preview_output = domains::preview::FrameOutput::default();
        let mut web_placements = Vec::new();
        let mut save_missing_drafts = Vec::new();
        let mut load_untitled = Vec::new();
        let mut banner_actions = Vec::new();
        let mut changed_documents = std::mem::take(&mut self.editor_menu_changes);
        let mut new_focus = if context.viewport_id() == egui::ViewportId::ROOT {
            self.focused.clone()
        } else {
            self.shell_state
                .auxiliary_windows
                .focused_editor(context.viewport_id())
        };
        let mut status = self.status.take();
        let is_workspace_busy = self.workspace_busy();
        let is_enabled = self.closing.is_none()
            && !self.shell_state.system_usage.detail_open
            && self.explorer_search_state.pending_entry_delete.is_none()
            && !self.explorer_search_state.deleting_entry
            && !is_workspace_busy
            && !self.services.state.is_shutting_down()
            && self.pending_tab_close.is_none()
            && self.saving_missing_draft.is_none()
            && self.saving_untitled.is_none()
            && self.pending_disk_choice.is_none();
        let viewport = context.viewport_id();
        let pass = context.cumulative_frame_nr();
        let palette_focus = self
            .commands_state
            .palette
            .source_focus()
            .filter(|_| self.commands_state.palette.is_open());
        self.editor_keymap_targets
            .retain(|(owner_viewport, id), (view, previous)| {
                (*owner_viewport != viewport
                    || *previous == pass
                    || previous.checked_add(1) == Some(pass)
                    || palette_focus == Some(*id))
                    && (self.editor_locations.preview_owner(*view).is_some()
                        || self.store.views().get(*view).is_some_and(|view| {
                            snapshot
                                .layouts
                                .values()
                                .flat_map(taide_layout::service::all_roots)
                                .any(|root| {
                                    taide_native_ui::snapshot::active_tab(root, &view.key.pane)
                                        .is_some_and(|tab| tab.id == view.key.tab)
                                })
                        }))
            });
        if is_enabled
            && !self.commands_state.keybindings.is_capturing()
            && context.input(|input| input.raw.focused)
            && let Err(error) = self
                .terminal_agent_state
                .terminal_views
                .capture_window_keymap_with_local(
                    &context,
                    self.services
                        .state
                        .settings
                        .read()
                        .keymap_overrides
                        .as_deref(),
                    &mut keymap_actions,
                    editor_project.is_some(),
                    |id| {
                        let (view, _) = self.editor_keymap_targets.get(&(viewport, id))?;
                        if crate::editor_context_menu::body(&context, *view).is_some_and(|editor| {
                            taide_native_ui::editor_ai_edit::is_control(&context, editor, id)
                        }) {
                            return None;
                        }
                        Some(self.store.views().get(*view)?.composition.is_some())
                    },
                    |id, event| {
                        self.editor_keymap_targets
                            .get(&(viewport, id))
                            .is_some_and(|(view, _)| {
                                crate::editor_context_menu::body(&context, *view).is_some_and(
                                    |editor| {
                                        taide_native_ui::editor_ai_edit::owns_event(
                                            &context, editor, id, event,
                                        )
                                    },
                                ) || self
                                    .editor_code_actions
                                    .local_key(&self.store, *view, event)
                                    || self.editor_rename.local_key(*view, event)
                                    || self.editor_locations.local_key(*view, event)
                            })
                    },
                )
        {
            status = Some(error.to_string());
        }
        self.tooltips.begin_frame(&context);
        let lsp_summary = snapshot
            .focused_project()
            .and_then(|project| self.lsp.as_ref().and_then(|lsp| lsp.summary(project)));
        let welcome_shortcuts = match self.shell_state.welcome.shortcuts(
            self.services
                .state
                .settings
                .read()
                .keymap_overrides
                .as_deref(),
            &self.locale,
            context.os().is_mac(),
        ) {
            Ok(shortcuts) => shortcuts,
            Err(error) => {
                status = Some(error.to_string());
                Arc::new(Vec::new())
            }
        };
        let mut surfaces = AppSurfaces {
            welcome: &self.shell_state.welcome,
            open_projects: &snapshot.projects,
            welcome_shortcuts: &welcome_shortcuts,
            problem_opens: &mut self.problem_opens,
            tooltips: &self.tooltips,
            tooltip_appearance: &self.tooltip_appearance,
            problems: &mut self.problems,
            problems_appearance: &self.problems_appearance,
            explorer_appearance: &self.explorer_search_state.explorer_appearance,
            explorer_icons: &mut self.explorer_search_state.explorer_icons,
            navigation_appearance: &self.explorer_search_state.navigation_appearance,
            navigation_icons: &mut self.explorer_search_state.navigation_icons,
            symbol_sidebars: &mut self.explorer_search_state.symbol_sidebars,
            breadcrumbs: &mut self.explorer_search_state.breadcrumbs,
            diagnostics: &self.lsp_diagnostics,
            editor_symbols: &self.editor_symbols,
            editor_folding: &self.editor_folding,
            focused_slot: snapshot.shell.focused.as_ref(),
            chord_status: self
                .terminal_agent_state
                .terminal_views
                .chord_status(&context, Instant::now()),
            status_chord_appearance: &self.status_chord_appearance,
            system_usage: &mut self.shell_state.system_usage,
            system_usage_appearance: &self.shell_state.system_usage_appearance,
            system_usage_icon: &mut self.shell_state.system_usage_icon,
            status_editor_appearance: &self.status_editor_appearance,
            status_ide_appearance: &self.terminal_agent_state.status_ide_appearance,
            status_ide_icons: &mut self.terminal_agent_state.status_ide_icons,
            status_view: focused_view.clone(),
            command_context: &command_context,
            document_edits: &mut frame_pass.document_edits,
            fold_commands: &mut frame_pass.fold_commands,
            command_errors: &mut command_errors,
            lsp_summary,
            lsp_status_appearance: &self.lsp_status_appearance,
            app_file_views: &mut self.settings_state.app_file_views,
            settings_output: &mut settings_output,
            settings_views: &mut self.settings_state.settings_views,
            settings_appearance: &self.settings_state.settings_appearance,
            terminals: &self.terminal_agent_state.terminals,
            terminal_views: &mut self.terminal_agent_state.terminal_views,
            keymap_actions: &mut keymap_actions,
            keymap_editor_scope: &mut keymap_editor_scope,
            editor_keymap_targets: &mut self.editor_keymap_targets,
            keymap_documents: &mut keymap_documents,
            terminal_appearance: &self.terminal_agent_state.terminal_appearance,
            services: &self.services,
            locale: &self.locale,
            store: &mut self.store,
            editor: &self.editor,
            editor_find: &mut self.editor_find,
            editor_problems: &mut self.editor_problems,
            editor_locations: &mut self.editor_locations,
            editor_rename: &mut self.editor_rename,
            editor_code_actions: &mut self.editor_code_actions,
            editor_documentation: &mut self.editor_documentation,
            editor_completion: &mut self.editor_completion,
            editor_highlights: &mut self.editor_highlights,
            editor_inlay_hints: &mut self.editor_inlay_hints,
            editor_code_lenses: &mut self.editor_code_lenses,
            editor_selection_ranges: &mut self.editor_selection_ranges,
            editor_ide_selection: &mut self.terminal_agent_state.editor_ide_selection,
            editor_view_state: &mut self.editor_view_state,
            editor_markdown: &mut self.editor_markdown,
            peek_models: &self.peek_models,
            lsp: self.lsp.as_ref(),
            find_history: &mut self.find_history,
            find_appearance: &self.find_appearance,
            editor_display_colors: self.editor_display_colors,
            editor_bracket_colors: self.editor_bracket_colors,
            editor_sticky_colors: self.editor_sticky_colors,
            editor_minimap_colors: self.editor_minimap_colors,
            editor_diagnostic_colors: self.editor_diagnostic_colors,
            editor_overview_colors: self.editor_overview_colors,
            editor_problem_colors: self.editor_problem_colors,
            editor_location_colors: self.editor_location_colors,
            editor_documentation_colors: self.editor_documentation_colors,
            editor_markdown_colors: self.editor_markdown_colors,
            editor_completion_colors: self.editor_completion_colors,
            editor_ai_edit_colors: self.editor_ai_edit_colors,
            editor_highlight_colors: self.editor_highlight_colors,
            editor_code_action_colors: self.editor_code_action_colors,
            editor_inlay_hint_colors: self.editor_inlay_hint_colors,
            editor_code_lens_colors: self.editor_code_lens_colors,
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
            open_with: &mut self.preview_state.open_with,
            previews: &mut self.preview_state.previews,
            preview_output: &mut preview_output,
            pdf_previews: &mut self.preview_state.pdf_previews,
            hwp_previews: &mut self.preview_state.hwp_previews,
            pdf_appearance: &self.preview_state.pdf_appearance,
            presentation_previews: &mut self.preview_state.presentation_previews,
            presentation_appearance: &self.preview_state.presentation_appearance,
            spreadsheet_previews: &mut self.preview_state.spreadsheet_previews,
            spreadsheet_appearance: &self.preview_state.spreadsheet_appearance,
            web_previews: &self.preview_state.web_previews,
            web_placements: &mut web_placements,
            untitled: &self.untitled,
            untitled_loading: &self.untitled_loading,
            untitled_failed: &self.untitled_failed,
            load_untitled: &mut load_untitled,
            missing_drafts: &self.missing_drafts,
            save_missing_drafts: &mut save_missing_drafts,
            loading: &self.loading,
            failed: &self.failed,
            refused: &self.refused,
            trees: &self.explorer_search_state.trees,
            tree_loading: &self.explorer_search_state.tree_loading,
            explorers: &mut self.explorer_search_state.explorers,
            explorer_clipboards: &mut self.explorer_search_state.explorer_clipboards,
            explorer_output: &mut explorer_output,
            projects: &snapshot.projects,
            layouts: &snapshot.layouts,
            scope: &self.shell.scope,
            auxiliary_windows: &self.shell_state.auxiliary_windows,
            title_branches: &self.git_state.title_branches,
            commands: &mut commands,
            load_files: &mut load_files,
            target: target.as_ref(),
            focused: &mut new_focus,
            reveals: &mut self.reveals,
            status: &mut status,
            is_reduced_motion: self.motion_preference.current().unwrap_or(false),
            status_ready: StatusReady::default(),
        };
        let mut intents = ui
            .add_enabled_ui(
                is_enabled
                    && !self.commands_state.keybindings.is_open()
                    && !self.commands_state.palette.is_open(),
                |ui| self.shell.show(ui, &snapshot, &mut surfaces),
            )
            .inner;
        self.close_explorer_search_discarded_delete(&explorer_output.discarded_explorers);
        if matches!(self.shell.scope, WindowScope::Main) {
            self.shell_state.zen_hint.show(
                ui,
                ui.max_rect(),
                snapshot.shell.window_chrome.zen,
                Instant::now(),
                &self.shell_state.zen_hint_colors,
                (
                    &presentation::message(&self.locale, "zen.hint", &[]),
                    &presentation::message(&self.locale, "zen.hintExit", &[]),
                ),
            );
            let window = ui.max_rect();
            if crate::file_drop::is_hovering(&context)
                || self.preview_state.web_views.drags().is_hovering()
            {
                if let Err(error) = crate::file_drop::paint_overlay(
                    ui,
                    window,
                    &self.shell_state.file_drop_colors,
                    &presentation::message(&self.locale, "app.dropToOpen", &[]),
                    &mut self.explorer_search_state.navigation_icons,
                ) {
                    status = Some(error.to_string());
                }
                for banner in
                    crate::status_error_banner::layer_ids(self.settings_state.presentation_failures)
                {
                    context.set_sublayer(crate::file_drop::overlay_layer(), banner);
                }
            }
            let mut paths = crate::file_drop::dropped_paths(&context);
            paths.extend(self.preview_state.web_views.drags().take_dropped());
            if !paths.is_empty()
                && self.closing.is_none()
                && !self.is_exit_ready
                && !self.services.state.is_shutting_down()
            {
                self.submit(HostCommand::Shell(
                    domains::shell::Command::OpenDroppedPaths {
                        focused: snapshot.focused_project().cloned(),
                        paths,
                    },
                ));
            }
        }
        match crate::status_error_banner::show(
            &context,
            self.settings_state.presentation_failures,
            self.settings_state
                .preview_theme
                .as_ref()
                .unwrap_or(&self.settings_state.resolved_theme),
            &self.locale,
            &mut self.explorer_search_state.explorer_icons,
            is_enabled
                && !self.commands_state.keybindings.is_open()
                && !self.commands_state.palette.is_open(),
        ) {
            Ok(true) => {
                self.settings_state.presentation_refresh.retry();
                context.request_repaint();
            }
            Ok(false) => {}
            Err(error) => status = Some(error.to_string()),
        }
        self.status = status;
        if context.viewport_id() == egui::ViewportId::ROOT {
            self.focused = new_focus;
            frame_pass.web_placements = web_placements;
            frame_pass.root_enabled = is_enabled;
        } else {
            self.shell_state
                .auxiliary_windows
                .set_focused_editor(context.viewport_id(), new_focus);
        }
        for error in settings_output.settings_errors {
            self.toasts
                .settings_failed(&self.locale, &error, Instant::now());
        }
        for error in settings_output
            .theme_errors
            .into_iter()
            .chain(command_errors)
        {
            self.toasts.ipc_error(&self.locale, &error, Instant::now());
        }
        for notice in settings_output.snippet_notices {
            self.toasts.snippet(&self.locale, notice, Instant::now());
        }
        self.load_settings_requests(
            settings_output.load_settings,
            settings_output.load_settings_resources,
            settings_output.load_app_files,
        );
        self.finish_terminal_agent_frame(&context);
        if is_enabled && !self.shell_state.system_usage.detail_open {
            if !self.commands_state.keybindings.is_capturing()
                && context.input(|input| input.raw.focused)
            {
                let scope = crate::keymap::Context {
                    terminal: self
                        .terminal_agent_state
                        .terminal_views
                        .has_keyboard_focus(&context),
                    editor: keymap_editor_scope.is_some(),
                };
                self.commands_state
                    .keybindings
                    .observe_closed(scope, Instant::now());
                if let Err(error) = self.terminal_agent_state.terminal_views.route_window_keys(
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
                    editor_project.is_some(),
                ) {
                    self.status = Some(error.to_string());
                }
            }
            if context.viewport_id() == frame_pass.overlay {
                keymap_actions.append(&mut self.commands_state.palette_commands);
            }
            intents.extend(keymap_actions.into_iter().filter_map(|action| {
                crate::command_dispatch::intent_with_editor(
                    &action,
                    &command_context,
                    &snapshot,
                    focused_view.as_ref(),
                    &self.shell.scope,
                )
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
            if !self.commands_state.keybindings.is_capturing()
                && !self.commands_state.palette.is_open()
                && context
                    .input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::N))
                && let Some(project) = editor_project.as_ref()
                && let Some(pane) = target.as_ref()
            {
                commands.push(HostCommand::Editor(domains::editor::Command::NewUntitled {
                    project: project.clone(),
                    pane: pane.clone(),
                }));
            }
            for tab in load_untitled {
                if self.untitled_loading.insert(tab.clone())
                    && !self.submit(HostCommand::Editor(domains::editor::Command::OpenUntitled(
                        tab.clone(),
                    )))
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
                    if !self.submit(HostCommand::Editor(domains::editor::Command::OpenDocument(
                        path.clone(),
                    ))) {
                        self.loading.remove(&path);
                        self.loading_file_tabs.remove(&path);
                        context.request_repaint();
                    }
                }
            }
            self.load_preview_requests(&context, preview_output);
            self.load_explorer_search_trees(&context, explorer_output.load_trees);
            let mut actions = Vec::new();
            for command in commands {
                if matches!(
                    command,
                    HostCommand::Editor(domains::editor::Command::SetDirty { .. })
                        | HostCommand::Settings(domains::settings::Command::ThemeEdit(_))
                        | HostCommand::Settings(domains::settings::Command::SnippetEdit(_))
                ) {
                    self.submit(command);
                } else {
                    actions.push(command);
                }
            }
            if !self.flush_dirty() {
                self.terminal_agent_state.terminal_views.submission_failed();
                self.fail_explorer_search_entries(
                    explorer_output.create_entries,
                    explorer_output.rename_entries,
                );
                context.request_repaint();
                return false;
            }
            self.submit_explorer_search_entries(
                explorer_output.create_entries,
                explorer_output.rename_entries,
                explorer_output.paste_entries,
            );
            for command in actions {
                if !self.submit(command) {
                    self.terminal_agent_state.terminal_views.submission_failed();
                    break;
                }
            }
            for intent in intents {
                match intent {
                    ShellIntent::Commands(intent) => self.apply_commands_intent(intent, &context),
                    ShellIntent::Editor(intent) => self.apply_editor_intent(
                        intent,
                        &context,
                        frame,
                        &snapshot,
                        &command_context,
                        &keymap_documents,
                    ),
                    ShellIntent::ExplorerSearch(intent) => {
                        self.apply_explorer_search_intent(intent, &context, &snapshot)
                    }
                    ShellIntent::Git(intent) => self.apply_git_intent(intent),
                    ShellIntent::TerminalAgent(intent) => self.apply_terminal_agent_intent(intent),
                    ShellIntent::Settings(intent) => self.apply_settings_intent(intent, &snapshot),
                    ShellIntent::Preview(intent) => self.apply_preview_intent(intent, &context),
                    intent => self.apply_shell_intent(intent, frame, &snapshot),
                }
            }
            for (tab, action) in banner_actions {
                self.choose_disk(&tab, action);
            }
            self.queue_explorer_search_delete(explorer_output.delete_entries);
            self.flush_observations();
        }
        let is_project_display_open = self.shell_state.project_display_dialog.is_open();
        let is_project_group_open = self.shell_state.project_group_dialog.is_open()
            || self.shell_state.project_group_delete.is_open();
        if context.viewport_id() == frame_pass.overlay {
            self.show_overlays(&context, frame, &snapshot, &command_context, is_enabled);
        } else {
            self.tooltips.finish_frame(&context);
        }
        if matches!(self.shell.scope, WindowScope::Main)
            && crate::zen::escape(
                &context,
                snapshot.shell.window_chrome.zen,
                is_enabled
                    && !self.commands_state.keybindings.is_open()
                    && !self.commands_state.palette.is_open()
                    && !is_project_display_open
                    && !is_project_group_open
                    && self.pending_tab_close.is_none()
                    && self.explorer_search_state.pending_entry_delete.is_none()
                    && !self.explorer_search_state.deleting_entry
                    && self.saving_missing_draft.is_none()
                    && self.saving_untitled.is_none()
                    && self.pending_disk_choice.is_none(),
                keymap_editor_scope == Some(true),
            )
            && let Err(error) = self.controller.submit(ShellMutation::SetWindowChrome(
                taide_model::project::WindowChromePatch {
                    zen: Some(false),
                    ..Default::default()
                },
            ))
        {
            self.report(&error);
        }
        self.finish_terminal_agent_frame(&context);
        true
    }

    fn show_overlays(
        &mut self,
        context: &egui::Context,
        frame: &mut eframe::Frame,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        command_context: &CommandContext,
        is_enabled: bool,
    ) {
        self.show_shell_usage_detail(context);
        self.show_commands_overlays(context, snapshot, command_context, is_enabled);
        self.tooltips.finish_frame(context);
        self.show_explorer_search_delete_dialog(context);
        self.tab_close_dialog(context, frame);
        self.show_shell_dialogs(context, snapshot);
    }

    fn finish_frame(
        &mut self,
        context: &egui::Context,
        frame: &mut eframe::Frame,
        frame_pass: WindowPass,
    ) {
        let WindowPass {
            web_placements,
            root_enabled: is_enabled,
            ..
        } = frame_pass;
        self.flush_persistence(context);
        self.show_preview_web_views(context, frame, web_placements, is_enabled);
        let toast_position = self.services.state.settings.read().toast_position.clone();
        let toast_enabled = self.toast_interaction_enabled();
        if let Err(error) = self.toasts.main().show(
            context,
            self.toast_theme,
            &toast_position,
            Instant::now(),
            toast_enabled,
        ) {
            self.status = Some(error.to_string());
        }
        self.run_toast_actions();
        self.fulfill_view_states();
    }

    fn exit_application(&mut self) {
        if self.is_exit_ready {
            return;
        }
        let pending_close = self.closing.take();
        let view_states = self.capture_view_states();
        self.view_state_flush.finish(view_states.clone());
        self.editor_completion.clear();
        self.editor_inlay_hints.clear();
        self.editor_code_lenses.clear();
        self.editor_selection_ranges.clear(&mut self.store);
        self.editor_formatting.clear();
        self.editor_formatting.clear_native_saves();
        if let Err(error) = self
            .editor_documentation
            .clear(&mut self.store, &mut self.editor_syntax)
        {
            self.report(&editor_error(error));
        }
        let bridge = self.bridge.take().map(HostBridge::disconnect);
        let lsp = self.lsp.take().map(crate::lsp::LspBridge::disconnect);
        let web = self
            .preview_state
            .web_bridge
            .take()
            .map(crate::preview_web_host::Bridge::disconnect);
        self.clear_builtin_json();
        let syntax = self.editor_syntax.disconnect();
        self.preview_state.web_previews.invalidate_all();
        self.preview_state.web_views.clear();
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
                    shutdown(services.clone(), bridge, lsp, web, drafts, view_states),
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

pub(crate) struct Draft {
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
    view_states: crate::editor_view_state_sync::Captures,
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
        if !services.state.projects.read().contains_key(&draft.project) {
            log::warn!(
                "native exit skipped a draft of closed project {}",
                draft.project
            );
            continue;
        }
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
    let view_state_result = match view_states {
        Ok(captured) => crate::editor_view_state::apply(&services, &captured).await,
        Err(error) => Err(error),
    };
    crate::host::flush_layouts(&services).await?;
    view_state_result?;
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

fn paint_rail_icon(
    icons: &mut crate::problems_icons::Icons,
    status: &mut Option<String>,
    ui: &Ui,
    rect: egui::Rect,
    icon: taide_native_ui::project_rail::RailIcon<'_>,
    color: egui::Color32,
) {
    if let Err(error) = icons.paint(ui, rect, rail_glyph(icon), color, 0.0) {
        *status = Some(error.to_string());
    }
}

fn rail_glyph(icon: taide_native_ui::project_rail::RailIcon<'_>) -> crate::problems_icons::Glyph {
    match icon {
        taide_native_ui::project_rail::RailIcon::Project(name) => {
            crate::problems_icons::project(name)
        }
        taide_native_ui::project_rail::RailIcon::RootMissing => {
            crate::problems_icons::Glyph::TriangleAlert
        }
    }
}

#[derive(Default)]
struct StatusReady {
    problems: bool,
    ide: bool,
    usage: Option<egui::TextureId>,
}

#[derive(Default)]
struct StatusBarOutput {
    problem_toggles: usize,
    commands: Vec<HostCommand>,
    is_usage_detail_requested: bool,
    errors: Vec<String>,
}

struct AppSurfaces<'a> {
    welcome: &'a crate::welcome::State,
    open_projects: &'a [taide_model::project::ProjectRef],
    welcome_shortcuts: &'a [taide_native_ui::shell::WelcomeShortcutRow],
    problem_opens: &'a mut Vec<crate::problems::Open>,
    tooltips: &'a crate::tooltips::Provider,
    tooltip_appearance: &'a crate::tooltips::Appearance,
    problems: &'a mut crate::problems::Views,
    problems_appearance: &'a crate::problems::Appearance,
    explorer_appearance: &'a crate::explorer::Appearance,
    explorer_icons: &'a mut crate::problems_icons::Icons,
    navigation_appearance: &'a crate::symbol_outline::Appearance,
    navigation_icons: &'a mut crate::navigation_icons::Icons,
    symbol_sidebars: &'a mut crate::symbol_sidebar::Views,
    breadcrumbs: &'a mut crate::breadcrumbs::Views,
    diagnostics: &'a crate::diagnostics::Store,
    editor_symbols: &'a crate::editor_symbols::State,
    editor_folding: &'a crate::editor_folding::State,
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
    settings_output: &'a mut domains::settings::FrameOutput,
    settings_views: &'a mut crate::settings_view::Views,
    settings_appearance: &'a crate::settings_view::Appearance,
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
    editor_problems: &'a mut crate::editor_problems::State,
    editor_locations: &'a mut crate::editor_locations::State,
    editor_rename: &'a mut crate::editor_rename_controller::Controller,
    editor_code_actions: &'a mut crate::editor_code_actions_controller::Controller,
    editor_documentation: &'a mut crate::editor_documentation::State,
    editor_completion: &'a mut crate::editor_completion::State,
    editor_highlights: &'a mut crate::editor_highlights::State,
    editor_inlay_hints: &'a mut crate::editor_inlay_hints::State,
    editor_code_lenses: &'a mut crate::editor_code_lenses::State,
    editor_selection_ranges: &'a mut crate::editor_selection_ranges::State,
    editor_ide_selection: &'a mut crate::editor_ide_selection::State,
    editor_view_state: &'a mut crate::editor_view_state::State,
    editor_markdown: &'a mut crate::editor_markdown::State,
    peek_models: &'a crate::peek_models::Models,
    lsp: Option<&'a crate::lsp::LspBridge>,
    find_history: &'a mut taide_native_ui::editor_find_widget::FindHistory,
    find_appearance: &'a taide_native_ui::editor_find_widget::FindAppearance,
    editor_display_colors: taide_native_ui::editor_display::EditorDisplayColors,
    editor_bracket_colors: taide_native_ui::editor_brackets::EditorBracketColors,
    editor_sticky_colors: taide_native_ui::editor_sticky_scroll::EditorStickyColors,
    editor_minimap_colors: taide_native_ui::editor_minimap::EditorMinimapColors,
    editor_diagnostic_colors: taide_native_ui::editor_diagnostics::DiagnosticColors,
    editor_overview_colors: taide_native_ui::editor_overview::OverviewColors,
    editor_problem_colors: taide_native_ui::editor_problems::Colors,
    editor_location_colors: taide_native_ui::editor_locations::Colors,
    editor_documentation_colors: taide_native_ui::editor_documentation::Colors,
    editor_markdown_colors: taide_native_ui::editor_markup::Colors,
    editor_completion_colors: taide_native_ui::editor_completion::Colors,
    editor_ai_edit_colors: taide_native_ui::editor_ai_edit::Colors,
    editor_highlight_colors: crate::editor_highlights::Colors,
    editor_code_action_colors: taide_native_ui::editor_code_actions::Colors,
    editor_inlay_hint_colors: taide_native_ui::editor_inlay_hints::Colors,
    editor_code_lens_colors: taide_native_ui::editor_code_lenses::Colors,
    editor_sticky_scroll: bool,
    editor_syntax: &'a mut crate::editor_syntax::EditorSyntax,
    banner_appearance: &'a BannerAppearance,
    restore_notices: &'a HashMap<TabId, BannerVariant>,
    banner_actions: &'a mut Vec<(TabId, BannerAction)>,
    changed_documents: &'a mut HashMap<DocumentId, ViewId>,
    files: &'a HashMap<String, FileDocument>,
    open_with: &'a mut crate::open_with::Registry,
    previews: &'a mut crate::preview::Cache,
    preview_output: &'a mut domains::preview::FrameOutput,
    pdf_previews: &'a mut crate::preview_pdf::Cache,
    hwp_previews: &'a mut crate::preview_hwp::Cache,
    pdf_appearance: &'a crate::preview_pdf_surface::Appearance,
    presentation_previews: &'a mut crate::preview_presentation_cache::Cache,
    presentation_appearance: &'a crate::preview_presentation_surface::Appearance,
    spreadsheet_previews: &'a mut crate::preview_spreadsheet_cache::Cache,
    spreadsheet_appearance: &'a crate::preview_spreadsheet_surface::Appearance,
    web_previews: &'a crate::preview_web_cache::Cache,
    web_placements: &'a mut Vec<crate::preview_web_view::Placement>,
    untitled: &'a HashMap<TabId, UntitledDocument>,
    untitled_loading: &'a HashSet<TabId>,
    untitled_failed: &'a HashMap<TabId, AppError>,
    load_untitled: &'a mut Vec<TabId>,
    missing_drafts: &'a HashMap<String, crate::missing_draft::MissingDraft>,
    save_missing_drafts: &'a mut Vec<TabId>,
    loading: &'a HashSet<String>,
    failed: &'a HashMap<String, AppError>,
    refused: &'a HashSet<String>,
    trees: &'a HashMap<ProjectId, TreeRowPage>,
    tree_loading: &'a HashSet<ProjectId>,
    explorers: &'a mut HashMap<ProjectId, crate::explorer::Explorer>,
    explorer_clipboards: &'a mut crate::explorer_clipboard_owners::ClipboardOwners,
    explorer_output: &'a mut domains::explorer_search::FrameOutput,
    projects: &'a [taide_model::project::ProjectRef],
    layouts: &'a HashMap<ProjectId, taide_model::layout::ProjectLayout>,
    scope: &'a WindowScope,
    auxiliary_windows: &'a crate::auxiliary_windows::Windows,
    title_branches: &'a crate::title_branch::Branches,
    commands: &'a mut Vec<HostCommand>,
    load_files: &'a mut Vec<(String, TabId)>,
    target: Option<&'a PaneId>,
    focused: &'a mut Option<(PaneId, TabId)>,
    reveals: &'a mut crate::editor_reveal::Reveals,
    status: &'a mut Option<String>,
    is_reduced_motion: bool,
    status_ready: StatusReady,
}

impl ShellSurfaces for AppSurfaces<'_> {
    type StatusOutput = StatusBarOutput;
    fn text(&self, key: &str) -> String {
        presentation::message(self.locale, key, &[])
    }
    fn welcome_content(
        &self,
        project: Option<&ProjectId>,
    ) -> taide_native_ui::shell::WelcomeContent {
        self.welcome.content(project, self.welcome_shortcuts)
    }
    fn rail_recent_projects(&self) -> Vec<taide_native_ui::shell::WelcomeRecent> {
        self.welcome.rail(self.open_projects)
    }
    fn is_reduced_motion(&self) -> bool {
        self.is_reduced_motion
    }
    fn rail_icon(
        &mut self,
        painter: &egui::Painter,
        slot: egui::layers::ShapeIdx,
        rect: egui::Rect,
        icon: taide_native_ui::project_rail::RailIcon<'_>,
        color: egui::Color32,
    ) {
        let glyph = rail_glyph(icon);
        match self
            .explorer_icons
            .prepare(painter.ctx(), rect.size(), glyph)
            .and_then(|()| {
                self.explorer_icons
                    .shape(painter.ctx(), rect, glyph, color, 0.0)
            }) {
            Ok(shape) => painter.set(slot, shape),
            Err(error) => *self.status = Some(error.to_string()),
        }
    }
    fn text_with_args(&self, key: &str, arguments: &[(&str, &str)]) -> String {
        presentation::message(self.locale, key, arguments)
    }
    fn tooltip(
        &mut self,
        response: &egui::Response,
        align: egui::RectAlign,
        lines: Vec<taide_native_ui::shell::TooltipLine>,
    ) {
        let size = egui::Vec2::splat(taide_native_ui::shell::TOOLTIP_ICON_SIZE);
        let origin = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        let foreground = self.tooltip_appearance.foreground();
        let lines = lines
            .into_iter()
            .map(|line| {
                let icon = line.icon.and_then(|icon| {
                    let glyph = rail_glyph(icon);
                    match self
                        .explorer_icons
                        .prepare(&response.ctx, size, glyph)
                        .and_then(|()| {
                            self.explorer_icons.shape(
                                &response.ctx,
                                origin,
                                glyph,
                                foreground.gamma_multiply(line.opacity),
                                0.0,
                            )
                        }) {
                        Ok(shape) => Some(crate::tooltips::Icon { size, shape }),
                        Err(error) => {
                            *self.status = Some(error.to_string());
                            None
                        }
                    }
                });
                crate::tooltips::Line {
                    icon,
                    text: line.text,
                    opacity: line.opacity,
                }
            })
            .collect();
        self.tooltips
            .show_lines(response, lines, align, self.tooltip_appearance);
    }
    fn prepare_status_bar(&mut self, context: &egui::Context) {
        self.status_ready = StatusReady::default();
        match self.problems.prepare_status(context) {
            Ok(()) => self.status_ready.problems = true,
            Err(error) => *self.status = Some(error.to_string()),
        }
        match self.status_ide_icons.prepare(context) {
            Ok(()) => self.status_ready.ide = true,
            Err(error) => *self.status = Some(error.to_string()),
        }
        if self.system_usage.summary().is_some() {
            match self.system_usage_icon.prepare(context) {
                Ok(texture) => self.status_ready.usage = Some(texture),
                Err(error) => *self.status = Some(error.to_string()),
            }
        }
    }
    fn apply_status_bar(&mut self, output: StatusBarOutput) {
        if let Some(slot) = self.focused_slot {
            for _ in 0..output.problem_toggles {
                self.problems.toggle(slot);
            }
        }
        self.commands.extend(output.commands);
        if output.is_usage_detail_requested {
            self.system_usage.detail_open = true;
        }
        if let Some(error) = output.errors.into_iter().last() {
            *self.status = Some(error);
        }
    }
    fn discard_status_bar(&mut self) {
        self.system_usage.detail_open = false;
    }
    fn status_bar(&self, ui: &mut Ui, _: Option<&ProjectId>, output: &mut StatusBarOutput) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = crate::lsp::status::ITEM_GAP;
            if self.status_ready.problems {
                match self.problems.status_button(
                    ui,
                    self.focused_slot,
                    self.diagnostics,
                    self.locale,
                    self.problems_appearance,
                ) {
                    Ok((_, activations)) => output.problem_toggles += activations,
                    Err(error) => output.errors.push(error.to_string()),
                }
            }
            crate::lsp::status::show(
                ui,
                self.locale,
                self.lsp_status_appearance,
                self.lsp_summary,
            );
            if self.status_ready.ide
                && let Err(error) = crate::status_ide::show_prepared(
                    ui,
                    self.locale,
                    self.status_ide_appearance,
                    self.status_ide_icons,
                    self.services.ide.status(),
                )
            {
                output.errors.push(error.to_string());
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
                    &mut output.commands,
                    self.tooltips,
                );
                crate::status_editor::show_font(
                    ui,
                    self.locale,
                    self.status_editor_appearance,
                    crate::status_editor::Target::Editor,
                    editor_font_size,
                    &mut output.commands,
                    self.tooltips,
                );
                if let (Some(texture), Some(usage)) =
                    (self.status_ready.usage, self.system_usage.summary())
                    && crate::system_usage_view::show_prepared_status(
                        ui,
                        self.locale,
                        self.system_usage_appearance,
                        texture,
                        usage,
                        self.tooltips,
                    )
                    .clicked()
                {
                    output.is_usage_detail_requested = true;
                    ui.ctx().request_repaint();
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
    fn show_unavailable_tab(&mut self, ui: &mut Ui, tab: &Tab) {
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
    }
}

fn editor_error(error: taide_native_editor::document::EditorError) -> AppError {
    AppError::Internal(format!("native editor: {error:?}"))
}

#[cfg(test)]
#[path = "application-exit-tests.rs"]
mod exit_tests;

#[cfg(all(test, unix))]
#[path = "application-code-actions-tests.rs"]
mod code_actions_tests;
#[cfg(test)]
#[path = "application-code-lenses-tests.rs"]
mod code_lenses_tests;
#[cfg(all(test, unix))]
#[path = "application-inlay-hints-tests.rs"]
mod inlay_hints_tests;
#[cfg(all(test, unix))]
#[path = "application-selection-ranges-tests.rs"]
mod selection_ranges_tests;

#[cfg(all(test, unix))]
#[path = "application-context-menu-tests.rs"]
mod context_menu_tests;
#[cfg(all(test, unix))]
#[path = "application-ide-selection-tests.rs"]
mod ide_selection_tests;
#[cfg(all(test, unix))]
#[path = "application-json-tests.rs"]
mod json_tests;
#[cfg(all(test, unix))]
#[path = "application-startup-tests.rs"]
mod startup_tests;
#[cfg(all(test, unix))]
#[path = "application-view-state-tests.rs"]
mod view_state_tests;

#[cfg(all(test, unix))]
#[path = "application-file-loading-tests.rs"]
mod file_loading_tests;

#[cfg(all(test, unix))]
#[path = "application-missing-draft-tests.rs"]
mod missing_draft_tests;

#[cfg(all(test, unix))]
#[path = "application-markdown-tests.rs"]
mod markdown_tests;
#[cfg(all(test, unix))]
#[path = "application-preview-tests.rs"]
mod preview_tests;

#[cfg(all(test, unix))]
#[path = "application-ai-completion-tests.rs"]
mod ai_completion_tests;
#[cfg(test)]
#[path = "application-ai-edit-tests.rs"]
mod ai_edit_tests;

#[cfg(all(test, unix, feature = "inspection"))]
#[path = "application-emmet-tests.rs"]
mod emmet_tests;

#[cfg(all(test, unix, feature = "inspection"))]
#[path = "application-sticky-definition-tests.rs"]
mod sticky_definition_tests;

#[cfg(all(test, unix))]
#[path = "application-project-rail-tests.rs"]
mod project_rail_tests;

#[cfg(all(test, unix))]
#[path = "application-tab-menu-tests.rs"]
mod tab_menu_tests;

#[cfg(all(test, unix))]
#[path = "application-tab-drag-tests.rs"]
mod tab_drag_tests;

#[cfg(all(test, unix))]
#[path = "application-auxiliary-window-tests.rs"]
mod auxiliary_window_tests;

#[cfg(all(test, unix))]
#[path = "application-window-state-tests.rs"]
mod window_state_tests;

#[cfg(all(test, unix))]
#[path = "application-app-menu-tests.rs"]
mod app_menu_tests;

#[cfg(all(test, unix))]
#[path = "application-file-drop-tests.rs"]
mod file_drop_tests;

#[cfg(all(test, unix))]
#[path = "application-error-boundary-tests.rs"]
mod error_boundary_tests;

#[cfg(all(test, unix))]
#[path = "application-shell-dialogs-tests.rs"]
mod shell_dialogs_tests;

#[cfg(all(test, unix))]
#[path = "application-explorer-views-tests.rs"]
mod explorer_views_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "application-terminal-agent-tests.rs"]
mod terminal_agent_tests;

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

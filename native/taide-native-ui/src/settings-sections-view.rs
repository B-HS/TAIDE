use std::collections::BTreeSet;

use super::dialogs::{self, Alert, Choice, Size};
use super::*;
use crate::settings_code_controls::{TextDraft, trim_text};
use crate::settings_controls::HostedChange;
use crate::settings_sections_data::{
    Action, Busy, Data, HookChange, HookTarget, OwnerEvent, PickPurpose, Progress, Query, Secret,
    Status, VsixEvent, VsixExtraction,
};
use taide_model::agent::HookInstallScope;
use taide_model::ai::AiProviderId;
use taide_model::app::{AppFileTarget, PromptTemplateId};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LspInstallPhase, LspInstallStrategy, LspServerDetection};
use taide_model::plugin::{LoadedPlugin, PluginErrorCode};
use taide_model::project::ProjectRef;
use taide_model::remote::{REMOTE_PASSWORD_MIN_LEN, is_valid_allowed_host};
use taide_model::vsix::VsixThemeImportFailure;

const HOOK_AGENTS: [(&str, &str, HookInstallScope); 5] = [
    (
        "claude",
        "settings.agentHooksAgentClaude",
        HookInstallScope::Project,
    ),
    (
        "codex",
        "settings.agentHooksAgentCodex",
        HookInstallScope::User,
    ),
    (
        "gemini",
        "settings.agentHooksAgentGemini",
        HookInstallScope::User,
    ),
    (
        "opencode",
        "settings.agentHooksAgentOpencode",
        HookInstallScope::User,
    ),
    ("pi", "settings.agentHooksAgentPi", HookInstallScope::User),
];
const HOOKS_TOGGLE_GAP: f32 = 8.0;
const HOOKS_LIST_GAP: f32 = 12.0;
const HOOK_ROW_PADDING_Y: i8 = 6;
const HOOK_ROW_RADIUS: u8 = 4;
const PERF_HEADING_GAP: f32 = 4.0;
const ROW_PADDING_X: i8 = 12;
const ROW_PADDING_Y: i8 = 8;
const LIST_GAP: f32 = 6.0;
const LSP_ROW_GAP: f32 = 4.0;
const TOKEN_ROW_GAP: f32 = 6.0;
const HEADER_GAP: f32 = 8.0;
const INLINE_GAP: f32 = 6.0;
const INDENT: i8 = 22;
const BADGE_TEXT_SIZE: f32 = 10.0;
const BADGE_PADDING_X: i8 = 4;
const BADGE_RADIUS: u8 = 4;
const BADGE_BORDER_OPACITY: f32 = 0.4;
const XS_BUTTON_HEIGHT: f32 = 24.0;
const XS_BUTTON_PADDING_X: f32 = 8.0;
const COPY_ICON_SIZE: f32 = 12.0;
const PROGRESS_HEIGHT: f32 = 4.0;
const PROGRESS_TRACK_OPACITY: f32 = 0.2;
const PROGRESS_GAP: f32 = 4.0;
const SPIN_SECONDS: f64 = 1.0;
const ROTATION_ORIGIN: f32 = 0.5;
const BYTES_PER_KIB: f64 = 1024.0;
const BYTES_PER_MIB: f64 = BYTES_PER_KIB * 1024.0;
const ONE_DECIMAL_SCALE: f64 = 10.0;
const EXPERIMENTAL_SERVERS: [&str; 1] = ["expert"];
const DEFAULT_AI_PROVIDER: AiProviderId = AiProviderId::OllamaCloud;
const AI_PROVIDERS: [(AiProviderId, &str); 3] = [
    (AiProviderId::OllamaCloud, "settings.aiProviderOllamaCloud"),
    (AiProviderId::Codex, "settings.aiProviderCodex"),
    (AiProviderId::Omlx, "settings.aiProviderOmlx"),
];
const PROMPTS: [(PromptTemplateId, &str); 3] = [
    (PromptTemplateId::AutoTabDefault, "prompts.autoTabTitle"),
    (
        PromptTemplateId::InlineEditDefault,
        "prompts.inlineEditTitle",
    ),
    (
        PromptTemplateId::CommitMessageDefault,
        "prompts.commitMessageTitle",
    ),
];
const PROMPT_LIST_TOP: i8 = 12;
const ACTION_ICON_SIZE: f32 = 14.0;
const XS_ICON_PADDING_X: f32 = 6.0;
const XS_ICON_GAP: f32 = 4.0;
const PLUGIN_ROW_GAP: f32 = 2.0;
const PLUGIN_INLINE_GAP: f32 = 8.0;
const PLUGIN_SECTION_GAP: f32 = 12.0;
const EMPTY_PADDING_Y: i8 = 16;
const EMPTY_LINE_GAP: f32 = 4.0;
const DASH_LENGTH: f32 = 3.0;
const VSIX_LIST_HEIGHT: f32 = 256.0;
const VSIX_ROW_PADDING_X: i8 = 8;
const VSIX_ROW_PADDING_Y: i8 = 6;
const VSIX_ROW_RADIUS: u8 = 4;
const VSIX_SECTION_GAP: f32 = 8.0;
const VSIX_DETAIL_GAP: f32 = 6.0;
const VSIX_WARNING_GAP: f32 = 2.0;
const VSIX_WARNING_ICON: f32 = 12.0;
const FAILED_ROW_OPACITY: f32 = 0.5;
const CHECKBOX_SIZE: f32 = 16.0;
const CHECKBOX_RADIUS: u8 = 4;
const DUPLICATE_SEPARATOR: &str = " / ";
const GITHUB_TOKEN_CREATE_URL: &str =
    "https://github.com/settings/tokens/new?scopes=gist&description=TAIDE%20Sync";
const SECTION_GAP: f32 = 8.0;
const HINT_GAP: f32 = 2.0;
const NEWER_BADGE_OPACITY: f32 = 0.15;
const NEWER_BADGE_PADDING_X: i8 = 6;
const NEWER_BADGE_PADDING_Y: i8 = 2;
const PASSWORD_ROW_GAP: f32 = 6.0;
const HOSTS_GAP: f32 = 6.0;
const HOST_LIST_GAP: f32 = 4.0;
const HOST_ROW_PADDING_X: i8 = 8;
const HOST_ROW_PADDING_Y: i8 = 4;
const HOST_ROW_RADIUS: u8 = 4;
const HOST_REMOVE_SIZE: f32 = 20.0;

pub(super) struct Appearance {
    agent_running: Color32,
    badge: Color32,
    warning: Color32,
    pub(super) error: Color32,
    success: Color32,
    pub(super) modal: Color32,
    pub(super) modal_border: Color32,
    pub(super) shadow: Color32,
    pub(super) primary_foreground: Color32,
    separator: Color32,
}

impl Appearance {
    pub(super) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            agent_running: color(theme, "appSidebar.iconAgentRunning")?,
            badge: color(theme, "appSidebar.badge")?,
            warning: color(theme, "statusIndicator.warning")?,
            error: color(theme, "statusIndicator.error")?,
            success: color(theme, "statusIndicator.success")?,
            modal: color(theme, "modal.background")?,
            modal_border: color(theme, "modal.border")?,
            shadow: color(theme, "app.shadow")?,
            primary_foreground: color(theme, "button.primaryForeground")?,
            separator: color(theme, "modal.separator")?,
        })
    }
}

/// Marks a Settings frame drawn inside the native application, which adds the host-backed sections.
#[derive(Clone, Copy, Debug)]
pub struct Host<'a> {
    /// Formats an RFC 3339 sync time for display; `None` keeps the stored text.
    pub format_time: fn(&str) -> Option<String>,
    /// Open projects whose agent hooks Settings lists.
    pub projects: &'a [ProjectRef],
    /// Shows the macOS-only `taide` shell command row.
    pub is_macos: bool,
    /// Shows the development build notification hint.
    pub is_dev_build: bool,
    /// Adds the Performance section while instrumentation is on.
    pub is_perf_enabled: bool,
}

/// Settings owner state that outlives a theme or snippet editor replacing the sections.
#[derive(Default)]
struct Container {
    owner: Option<u64>,
    issued_remote_url: Option<String>,
    is_sync_conflict_open: bool,
}

/// Section-level state of one Settings owner; it is dropped while a theme or snippet editor replaces
/// the sections.
#[derive(Default)]
pub struct OwnerState {
    mounted: bool,
    ollama_cloud_token: String,
    codex_token: String,
    omlx_key: String,
    omlx_url: Option<TextDraft>,
    is_omlx_url_committed: bool,
    provider: code_view::Picker,
    model: code_view::Picker,
    uninstall: Option<(String, String, dialogs::Focus)>,
    vsix_ticket: Option<u64>,
    vsix: Option<VsixDialog>,
    returning_focus: Option<dialogs::Returning>,
    sync_token: String,
    sync_download: Option<Id>,
    sync_conflict: Option<dialogs::Focus>,
    remote_password: String,
    remote_host: String,
    remote_host_composition: crate::modal::Composition,
    hooks_consent: Option<(ProjectId, dialogs::Focus)>,
    is_hooks_list_shown: bool,
    perf_ticket: Option<u64>,
    perf_snapshot: Option<String>,
    is_perf_busy: bool,
    container: Container,
}

enum Grammars {
    Idle,
    Importing,
    Imported,
    Failed(AppError),
}

struct VsixDialog {
    ticket: u64,
    extraction: VsixExtraction,
    selected: BTreeSet<String>,
    focus: dialogs::Focus,
    confirm: Option<dialogs::Focus>,
    returning_focus: Option<dialogs::Returning>,
    is_saving: bool,
    grammars: Grammars,
}

impl VsixDialog {
    fn new(ticket: u64, extraction: VsixExtraction, origin: Id) -> Self {
        let selected = extraction
            .candidates
            .iter()
            .filter(|candidate| candidate.failure.is_none())
            .map(|candidate| candidate.key.clone())
            .collect();
        Self {
            ticket,
            extraction,
            selected,
            focus: dialogs::Focus::open(origin),
            confirm: None,
            returning_focus: None,
            is_saving: false,
            grammars: Grammars::Idle,
        }
    }

    fn selected(&self) -> impl Iterator<Item = &taide_model::vsix::VsixThemeCandidate> {
        self.extraction
            .candidates
            .iter()
            .filter(|candidate| self.selected.contains(&candidate.key) && candidate.theme.is_some())
    }

    fn duplicates(&self, locale: &ResolvedLocale) -> Vec<String> {
        self.selected()
            .filter(|candidate| candidate.id_collides)
            .map(|candidate| {
                message(
                    locale,
                    "settings.themeImportDuplicate",
                    &[("name", &candidate.id)],
                )
            })
            .collect()
    }

    fn persist(&mut self, data: &mut Data, locale: &ResolvedLocale) {
        let mut used = self
            .extraction
            .existing_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let themes = self
            .selected()
            .filter_map(|candidate| {
                let theme = candidate.theme.as_ref()?;
                let (id, name) = if candidate.id_collides {
                    (
                        crate::theme_draft::unique_theme_id(&theme.name, &used),
                        message(
                            locale,
                            "themeEditor.duplicateNameTemplate",
                            &[("name", &theme.name)],
                        ),
                    )
                } else {
                    (theme.id.clone(), theme.name.clone())
                };
                used.insert(id.clone());
                Some(taide_model::theme::Theme {
                    id,
                    name,
                    ..theme.clone()
                })
            })
            .collect();
        self.is_saving = true;
        data.act(Action::VsixSaveThemes {
            themes,
            extension: self.extraction.extension.clone(),
            ticket: self.ticket,
        });
    }
}

impl OwnerState {
    pub(super) fn mount(&mut self) -> bool {
        !std::mem::replace(&mut self.mounted, true)
    }

    pub fn reset_section_state(&mut self) {
        let container = std::mem::take(&mut self.container);
        *self = Self {
            container,
            ..Self::default()
        };
    }

    /// Delivers the host results addressed to this owner.
    pub(super) fn receive(&mut self, data: &mut Data) {
        if self.container.owner.is_none() {
            self.container.owner = data.next_owner();
        }
        let Some(owner) = self.container.owner else {
            return;
        };
        let perf = self
            .perf_ticket
            .map(|ticket| data.take_owner_events(ticket));
        for event in data
            .take_owner_events(owner)
            .into_iter()
            .chain(perf.into_iter().flatten())
        {
            match event {
                OwnerEvent::SyncConflict => self.container.is_sync_conflict_open = true,
                OwnerEvent::RemoteLinkIssued(url) => {
                    self.container.issued_remote_url = Some(url);
                }
                OwnerEvent::PerfRead(snapshot) => {
                    self.perf_snapshot = Some(snapshot);
                    self.is_perf_busy = false;
                }
                OwnerEvent::PerfReset => {
                    self.perf_snapshot = None;
                    self.is_perf_busy = false;
                }
                OwnerEvent::PerfFailed => self.is_perf_busy = false,
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Variant {
    Outline,
    Ghost,
}

fn to_fixed_one(value: f64) -> String {
    format!(
        "{:.1}",
        (value * ONE_DECIMAL_SCALE).round() / ONE_DECIMAL_SCALE
    )
}

fn format_bytes(bytes: f64) -> String {
    if bytes < BYTES_PER_KIB {
        return format!("{bytes}B");
    }
    if bytes < BYTES_PER_MIB {
        return format!("{}KB", to_fixed_one(bytes / BYTES_PER_KIB));
    }
    format!("{}MB", to_fixed_one(bytes / BYTES_PER_MIB))
}

fn row_frame(appearance: &super::Appearance) -> egui::Frame {
    egui::Frame::NONE
        .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
        .corner_radius(ROW_RADIUS)
        .inner_margin(egui::Margin::symmetric(ROW_PADDING_X, ROW_PADDING_Y))
}

fn indented<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::NONE
        .inner_margin(egui::Margin {
            left: INDENT,
            ..egui::Margin::ZERO
        })
        .show(ui, add)
        .inner
}

fn small(ui: &mut Ui, text: impl Into<String>, color: Color32) -> egui::Response {
    ui.add(egui::Label::new(RichText::new(text).size(TEXT_SIZE).color(color)).wrap())
}

fn xs_button(
    ui: &mut Ui,
    label: &str,
    variant: Variant,
    enabled: bool,
    appearance: &super::Appearance,
) -> egui::Response {
    let button = egui::Button::new(
        RichText::new(label)
            .size(TEXT_SIZE)
            .color(appearance.foreground),
    )
    .corner_radius(ROW_RADIUS)
    .min_size(egui::vec2(0.0, XS_BUTTON_HEIGHT));
    let button = match variant {
        Variant::Outline => button.stroke(Stroke::new(BORDER_WIDTH, appearance.border)),
        Variant::Ghost => button.frame_when_inactive(false),
    };
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(XS_BUTTON_PADDING_X, 0.0);
        ui.add_enabled(enabled, button)
    })
    .inner
}

fn header(
    ui: &mut Ui,
    icons: &Icons,
    is_ok: bool,
    name: &str,
    badge: Option<String>,
    status: &str,
    height: f32,
    appearance: &super::Appearance,
    trailing: impl FnOnce(&mut Ui),
) {
    let tone = if is_ok {
        appearance.sections.agent_running
    } else {
        appearance.sections.badge
    };
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::right_to_left(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = HEADER_GAP;
            trailing(ui);
            ui.label(RichText::new(status).size(TEXT_SIZE).color(tone));
            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                if let Some(image) = icons.image(
                    if is_ok {
                        Icon::CircleCheck
                    } else {
                        Icon::CircleX
                    },
                    tone,
                ) {
                    ui.add(image);
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(name)
                            .size(TEXT_SIZE)
                            .color(appearance.foreground),
                    )
                    .truncate(),
                );
                if let Some(badge) = badge {
                    egui::Frame::NONE
                        .stroke(Stroke::new(
                            BORDER_WIDTH,
                            appearance
                                .sections
                                .warning
                                .gamma_multiply(BADGE_BORDER_OPACITY),
                        ))
                        .corner_radius(BADGE_RADIUS)
                        .inner_margin(egui::Margin::symmetric(BADGE_PADDING_X, 0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(badge)
                                    .size(BADGE_TEXT_SIZE)
                                    .color(appearance.sections.warning),
                            );
                        });
                }
            });
        },
    );
}

fn traced(output: &mut Output, _field: &'static str, response: egui::Response) -> egui::Response {
    #[cfg(any(test, feature = "inspection"))]
    output.traces.push(Trace::capture(_field, &response));
    #[cfg(not(any(test, feature = "inspection")))]
    let _ = output;
    response
}

/// Draws the `taide` shell command row, the agent hooks block and the IDE integration switch that
/// follow the agent status badge switch.
pub(super) fn interface(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    mounting: bool,
    host: Host<'_>,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let now = web_time::Instant::now();
    data.observe(Query::CliStatus, mounting, now);
    dialogs::settle(ui, &mut state.returning_focus);
    if host.is_macos {
        cli_row(ui, data, locale, appearance, output);
    }
    let enabled = settings.agent_hooks_enabled;
    let is_hooks_list_mounting = mounting || !state.is_hooks_list_shown;
    state.is_hooks_list_shown = enabled;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = HOOKS_TOGGLE_GAP;
        if switch_control(
            ui,
            "settings.agentHooks",
            Some("settings.agentHooksHint"),
            enabled,
            locale,
            appearance,
            output,
        ) {
            output
                .changes
                .push(Change::Hosted(HostedChange::AgentHooks(!enabled)));
        }
        if enabled {
            hooks_list(
                ui,
                data,
                state,
                is_hooks_list_mounting,
                host,
                locale,
                appearance,
                output,
            );
        }
    });
    if switch_control(
        ui,
        "settings.ideIntegration",
        Some("settings.ideIntegrationHint"),
        settings.ide_integration_enabled,
        locale,
        appearance,
        output,
    ) {
        output
            .changes
            .push(Change::Hosted(HostedChange::IdeIntegration(
                !settings.ide_integration_enabled,
            )));
    }
    if state.hooks_consent.as_ref().is_some_and(|(project, _)| {
        !settings.agent_hooks_enabled || !host.projects.iter().any(|open| open.id == *project)
    }) {
        state.hooks_consent = None;
    }
    let consent = state.hooks_consent.as_mut().and_then(|(project, focus)| {
        let alert = Alert {
            title: message(locale, "agent.hooksConsentTitle", &[]),
            description: message(locale, "agent.hooksConsentDescription", &[]),
            action: message(locale, "agent.hooksInstall", &[]),
            is_destructive: false,
            size: Size::Default,
            alternative: None,
        };
        dialogs::alert(
            ui,
            ui.id().with("agent-hooks-consent"),
            focus,
            &alert,
            locale,
            appearance,
            output,
        )
        .map(|choice| (choice, project.clone()))
    });
    if let Some((choice, project)) = consent {
        if choice == Choice::Confirm {
            data.change_hooks(
                HookTarget {
                    project,
                    agent: HOOK_AGENTS[0].0,
                },
                HookChange::Install,
            );
        }
        if let Some((_, focus)) = state.hooks_consent.take() {
            state.returning_focus = focus.close();
        }
    }
}

fn cli_row(
    ui: &mut Ui,
    data: &mut Data,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let status = match data.cli_status() {
        Status::Ready(status) => Some(status.clone()),
        Status::Pending | Status::Failed(_) => None,
    };
    let detail = status
        .as_ref()
        .filter(|status| status.installed)
        .map(|status| {
            if status.dangling {
                ("settings.cliStatusDangling", appearance.sections.warning)
            } else {
                ("settings.cliStatusInstalled", appearance.muted)
            }
        });
    let text_height = if detail.is_some() {
        LINE_HEIGHT * 2.0 + HINT_GAP
    } else {
        LINE_HEIGHT
    };
    let height = text_height.max(XS_BUTTON_HEIGHT);
    let installed = status.as_ref().is_some_and(|status| status.installed);
    let dangling = status.as_ref().is_some_and(|status| status.dangling);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::right_to_left(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = INLINE_GAP;
            if installed {
                let uninstall = traced(
                    output,
                    "settings.cliUninstallButton",
                    xs_button(
                        ui,
                        &message(locale, "settings.cliUninstallButton", &[]),
                        Variant::Outline,
                        !data.is_busy(Busy::CliUninstall),
                        appearance,
                    ),
                );
                if uninstall.clicked() {
                    data.act(Action::CliUninstall);
                }
            }
            if !installed || dangling {
                let install = traced(
                    output,
                    "settings.cliInstallButton",
                    xs_button(
                        ui,
                        &message(locale, "settings.cliInstallButton", &[]),
                        Variant::Outline,
                        !data.is_busy(Busy::CliInstall),
                        appearance,
                    ),
                );
                if install.clicked() {
                    data.act(Action::CliInstall);
                }
            }
            ui.spacing_mut().item_spacing.x = CONTROL_GAP;
            ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = HINT_GAP;
                ui.add_space((height - text_height) * HALF);
                small(
                    ui,
                    message(locale, "keymap.category.shellCommand", &[]),
                    appearance.foreground,
                );
                if let Some((key, color)) = detail {
                    small(ui, message(locale, key, &[]), color);
                }
            });
        },
    );
}

fn hooks_list(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    mounting: bool,
    host: Host<'_>,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let now = web_time::Instant::now();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = HOOKS_LIST_GAP;
        for (agent, label, scope) in HOOK_AGENTS {
            if scope == HookInstallScope::Project {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = LIST_GAP;
                    small(ui, message(locale, label, &[]), appearance.muted);
                    if host.projects.is_empty() {
                        small(
                            ui,
                            message(locale, "app.noProjectOpen", &[]),
                            appearance.muted,
                        );
                    }
                    for project in host.projects {
                        let target = HookTarget {
                            project: project.id.clone(),
                            agent,
                        };
                        data.observe(Query::HooksStatus(target.clone()), mounting, now);
                        ui.push_id(project.id.as_str(), |ui| {
                            hook_project_row(
                                ui, data, state, &target, project, locale, appearance, output,
                            );
                        });
                    }
                });
                continue;
            }
            let target = HookTarget {
                project: ProjectId(String::new()),
                agent,
            };
            data.observe(Query::HooksStatus(target.clone()), mounting, now);
            hook_user_row(ui, data, &target, label, locale, appearance, output);
        }
    });
}

fn hook_project_row(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    target: &HookTarget,
    project: &ProjectRef,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    egui::Frame::NONE
        .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
        .corner_radius(HOOK_ROW_RADIUS)
        .inner_margin(egui::Margin::symmetric(ROW_PADDING_X, HOOK_ROW_PADDING_Y))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = HEADER_GAP;
                    let installed = match data.hooks_status(target) {
                        Status::Pending => None,
                        Status::Ready(status) => Some(status.installed),
                        Status::Failed(_) => Some(false),
                    };
                    match installed {
                        None => {
                            ui.label(
                                RichText::new(message(locale, "settings.loading", &[]))
                                    .size(TEXT_SIZE)
                                    .color(appearance.muted),
                            );
                        }
                        Some(true) => {
                            let uninstall = traced(
                                output,
                                "agent.hooksUninstall",
                                xs_button(
                                    ui,
                                    &message(locale, "agent.hooksUninstall", &[]),
                                    Variant::Outline,
                                    !data.is_changing_hooks(target, HookChange::Uninstall),
                                    appearance,
                                ),
                            );
                            ui.label(
                                RichText::new(message(locale, "agent.hooksInstalled", &[]))
                                    .size(TEXT_SIZE)
                                    .color(appearance.sections.agent_running),
                            );
                            if uninstall.clicked() {
                                data.change_hooks(target.clone(), HookChange::Uninstall);
                            }
                        }
                        Some(false) => {
                            let install = traced(
                                output,
                                "agent.hooksInstall",
                                xs_button(
                                    ui,
                                    &message(locale, "agent.hooksInstall", &[]),
                                    Variant::Outline,
                                    !data.is_changing_hooks(target, HookChange::Install),
                                    appearance,
                                ),
                            );
                            if install.clicked() && state.hooks_consent.is_none() {
                                state.hooks_consent = Some((
                                    target.project.clone(),
                                    dialogs::Focus::open(install.id),
                                ));
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&project.name)
                                    .size(TEXT_SIZE)
                                    .color(appearance.foreground),
                            )
                            .truncate(),
                        );
                    });
                },
            );
        });
}

fn hook_user_row(
    ui: &mut Ui,
    data: &mut Data,
    target: &HookTarget,
    label: &'static str,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    let status = match data.hooks_status(target) {
        Status::Ready(status) => Some(status.clone()),
        Status::Pending | Status::Failed(_) => None,
    };
    let is_cli_pending = matches!(data.cli_status(), Status::Pending);
    let is_cli_installed = matches!(data.cli_status(), Status::Ready(cli) if cli.installed);
    let requires_cli = status
        .as_ref()
        .is_some_and(|status| status.requires_taide_cli);
    let installed = status.as_ref().is_some_and(|status| status.installed);
    let is_cli_missing = requires_cli && !is_cli_pending && !is_cli_installed;
    let is_cli_blocking = requires_cli && (is_cli_pending || (is_cli_missing && !installed));
    let enabled = status.is_some()
        && !data.is_changing_hooks(target, HookChange::Install)
        && !data.is_changing_hooks(target, HookChange::Uninstall)
        && !is_cli_blocking;
    let mut hints = vec![
        (
            message(locale, "settings.agentHooksUserLevelDescription", &[]),
            appearance.muted,
        ),
        (
            message(locale, "settings.agentHooksUserLevelWarning", &[]),
            appearance.sections.warning,
        ),
    ];
    if is_cli_missing {
        hints.push((
            message(locale, "settings.agentHooksCliMissing", &[]),
            appearance.sections.warning,
        ));
    }
    if switch_lines(
        ui,
        label,
        &message(locale, label, &[]),
        &hints,
        installed,
        enabled,
        appearance,
        output,
    ) {
        data.change_hooks(
            target.clone(),
            if installed {
                HookChange::Uninstall
            } else {
                HookChange::Install
            },
        );
    }
}

/// Draws the notification settings and test notification buttons with the development hint.
pub(super) fn notifications(
    ui: &mut Ui,
    data: &mut Data,
    host: Host<'_>,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = PLUGIN_INLINE_GAP;
        let open = traced(
            output,
            "settings.notificationsOpenSystemSettings",
            xs_icon_button(
                ui,
                Icon::ExternalLink,
                &message(locale, "settings.notificationsOpenSystemSettings", &[]),
                true,
                0.0,
                Variant::Outline,
                appearance,
                icons,
            ),
        );
        let test = traced(
            output,
            "settings.notificationsSendTest",
            xs_icon_button(
                ui,
                Icon::Bell,
                &message(locale, "settings.notificationsSendTest", &[]),
                true,
                0.0,
                Variant::Outline,
                appearance,
                icons,
            ),
        );
        if open.clicked() {
            data.act(Action::OpenNotificationSettings);
        }
        if test.clicked() {
            data.send_test_notification(
                message(locale, "settings.notificationsSendTest", &[]),
                message(locale, "notification.enableHint", &[]),
            );
        }
    });
    if host.is_dev_build {
        small(
            ui,
            message(locale, "settings.notificationsDevHint", &[]),
            appearance.muted,
        );
    }
}

/// Draws the Performance section that reads and resets the native instrumentation registry.
pub(super) fn performance(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = PLUGIN_INLINE_GAP;
        let read = traced(
            output,
            "settings.performanceRead",
            xs_button(
                ui,
                &message(locale, "settings.performanceRead", &[]),
                Variant::Outline,
                !state.is_perf_busy,
                appearance,
            ),
        );
        let reset = traced(
            output,
            "settings.performanceReset",
            xs_button(
                ui,
                &message(locale, "settings.performanceReset", &[]),
                Variant::Outline,
                !state.is_perf_busy,
                appearance,
            ),
        );
        if (read.clicked() || reset.clicked())
            && let Some(ticket) = state.perf_ticket.or_else(|| data.next_owner())
        {
            state.perf_ticket = Some(ticket);
            state.is_perf_busy = true;
            data.act(if read.clicked() {
                Action::PerfRead { ticket }
            } else {
                Action::PerfReset { ticket }
            });
        }
    });
    let Some(snapshot) = &state.perf_snapshot else {
        return;
    };
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = PERF_HEADING_GAP;
        ui.label(
            RichText::new(message(locale, "settings.performanceNative", &[]))
                .size(TEXT_SIZE)
                .family(crate::font_families::medium(ui))
                .color(appearance.foreground),
        );
        egui::ScrollArea::horizontal()
            .id_salt("performance-native")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(snapshot)
                            .monospace()
                            .size(TEXT_SIZE)
                            .color(appearance.foreground),
                    )
                    .extend(),
                );
            });
    });
}

pub(super) fn lsp(
    ui: &mut Ui,
    data: &mut Data,
    mounting: bool,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    data.observe(Query::LspServers, mounting, web_time::Instant::now());
    let servers = match data.lsp_servers() {
        Status::Pending => {
            small(
                ui,
                message(locale, "settings.loading", &[]),
                appearance.muted,
            );
            return;
        }
        Status::Failed(_) => return,
        Status::Ready(servers) => servers.clone(),
    };
    let mut actions = Vec::new();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = LIST_GAP;
        for server in &servers {
            ui.push_id(server.id.as_str(), |ui| {
                lsp_row(
                    ui,
                    server,
                    data.lsp_progress(&server.id),
                    locale,
                    appearance,
                    icons,
                    &mut actions,
                    output,
                );
            });
        }
    });
    for action in actions {
        data.act(action);
    }
}

fn lsp_row(
    ui: &mut Ui,
    server: &LspServerDetection,
    progress: Option<&Progress>,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    actions: &mut Vec<Action>,
    output: &mut Output,
) {
    let is_failed = progress.is_some_and(Progress::is_failed);
    let is_active = progress.is_some_and(Progress::is_active);
    let install = message(locale, "settings.lspInstall", &[]);
    let action = if server.available {
        None
    } else if is_active {
        Some((
            message(locale, "settings.lspCancel", &[]),
            Variant::Ghost,
            true,
            "settings.lspCancel",
        ))
    } else {
        match server.install_strategy {
            LspInstallStrategy::Download => Some((
                install,
                Variant::Outline,
                server.download_available == Some(true),
                "settings.lspInstall",
            )),
            LspInstallStrategy::Toolchain if server.toolchain_available == Some(true) => Some((
                format!(
                    "{} {install}",
                    server.toolchain_tool.as_deref().unwrap_or_default()
                ),
                Variant::Outline,
                true,
                "settings.lspInstall",
            )),
            _ => None,
        }
    };
    let status = if server.available {
        let installed = message(locale, "settings.lspInstalled", &[]);
        match server
            .installed_version
            .as_deref()
            .filter(|version| !version.is_empty())
        {
            Some(version) => format!("{installed} (v{version})"),
            None => installed,
        }
    } else {
        message(locale, "settings.lspNotInstalled", &[])
    };
    let hint = server
        .install_hint
        .as_deref()
        .filter(|hint| !hint.is_empty() && !server.available && !is_active);
    let status_hint = if server.available || is_active || is_failed {
        None
    } else {
        match server.install_strategy {
            LspInstallStrategy::Download if server.download_available != Some(true) => {
                Some("settings.lspChecksumPending")
            }
            LspInstallStrategy::Toolchain if server.toolchain_available != Some(true) => {
                Some("settings.lspToolchainMissing")
            }
            LspInstallStrategy::SdkDetect if server.sdk_available != Some(true) => {
                Some("settings.lspSdkMissing")
            }
            _ => None,
        }
    };
    row_frame(appearance).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.spacing_mut().item_spacing = egui::vec2(HEADER_GAP, LSP_ROW_GAP);
        header(
            ui,
            icons,
            server.available,
            &server.name,
            EXPERIMENTAL_SERVERS
                .contains(&server.id.as_str())
                .then(|| message(locale, "settings.lspExperimental", &[])),
            &status,
            if action.is_some() {
                XS_BUTTON_HEIGHT
            } else {
                LINE_HEIGHT
            },
            appearance,
            |ui| {
                if let Some((label, variant, enabled, _field)) = action {
                    let response = xs_button(ui, &label, variant, enabled, appearance);
                    #[cfg(any(test, feature = "inspection"))]
                    output.traces.push(Trace::capture(_field, &response));
                    if response.clicked() {
                        actions.push(match variant {
                            Variant::Ghost => Action::LspCancel(server.id.clone()),
                            Variant::Outline => Action::LspInstall(server.id.clone()),
                        });
                    }
                }
            },
        );
        if let Some(path) = server
            .resolved_path
            .as_deref()
            .filter(|path| !path.is_empty())
        {
            indented(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(path)
                            .monospace()
                            .size(TEXT_SIZE)
                            .color(appearance.muted),
                    )
                    .wrap(),
                );
            });
        }
        if let Some(progress) = progress.filter(|_| is_active || is_failed) {
            indented(ui, |ui| {
                progress_line(ui, progress, locale, appearance, icons)
            });
        }
        if let Some(key) = status_hint {
            indented(ui, |ui| {
                small(ui, message(locale, key, &[]), appearance.sections.warning)
            });
        }
        if let Some(hint) = hint {
            if server.install_strategy == LspInstallStrategy::SdkDetect {
                indented(ui, |ui| small(ui, hint, appearance.muted));
            } else {
                indented(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = INLINE_GAP;
                        small(ui, hint, appearance.muted);
                        let label = message(locale, "settings.lspCopyCommand", &[]);
                        let (response, trigger) =
                            crate::tooltip_trigger::wrap_button(ui, &label, false, |ui| {
                                ui.add(
                                    egui::Button::opt_image_and_text(
                                        icons.image(Icon::Copy, appearance.foreground).map(
                                            |image| {
                                                image.fit_to_exact_size(egui::Vec2::splat(
                                                    COPY_ICON_SIZE,
                                                ))
                                            },
                                        ),
                                        None,
                                    )
                                    .frame_when_inactive(false)
                                    .corner_radius(ROW_RADIUS)
                                    .min_size(egui::Vec2::splat(XS_BUTTON_HEIGHT)),
                                )
                            });
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                response.enabled(),
                                &label,
                            )
                        });
                        #[cfg(any(test, feature = "inspection"))]
                        output
                            .traces
                            .push(Trace::capture("settings.lspCopyCommand", &response));
                        output.tooltips.push(trigger);
                        if response.clicked() {
                            actions.push(Action::CopyInstallHint(hint.into()));
                        }
                    });
                });
            }
        }
    });
}

fn progress_line(
    ui: &mut Ui,
    progress: &Progress,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
) {
    if progress.is_failed() {
        small(
            ui,
            progress
                .message
                .clone()
                .unwrap_or_else(|| message(locale, "settings.lspInstallFailed", &[])),
            appearance.sections.error,
        );
        return;
    }
    let total = progress
        .total_bytes
        .filter(|_| progress.phase == LspInstallPhase::Downloading);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(INLINE_GAP, PROGRESS_GAP);
        ui.horizontal(|ui| {
            let (spinner, _) =
                ui.allocate_exact_size(egui::Vec2::splat(Icon::Loader.size()), Sense::hover());
            let turn = ui.input(|input| input.time).rem_euclid(SPIN_SECONDS) / SPIN_SECONDS;
            if let Some(image) = icons.image(Icon::Loader, appearance.muted) {
                image
                    .rotate(
                        turn as f32 * std::f32::consts::TAU,
                        egui::Vec2::splat(ROTATION_ORIGIN),
                    )
                    .paint_at(ui, spinner);
            }
            ui.ctx().request_repaint();
            small(
                ui,
                message(locale, "settings.lspInstalling", &[]),
                appearance.muted,
            );
            if let Some(total) = total {
                ui.label(
                    RichText::new(format!(
                        "{} / {}",
                        format_bytes(progress.received_bytes),
                        format_bytes(total)
                    ))
                    .monospace()
                    .size(TEXT_SIZE)
                    .color(appearance.muted),
                );
            }
        });
        if let Some(total) = total {
            let (track, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), PROGRESS_HEIGHT),
                Sense::hover(),
            );
            let ratio = (progress.received_bytes / total).clamp(0.0, 1.0);
            let ratio = if ratio.is_nan() { 0.0 } else { ratio as f32 };
            let radius = PROGRESS_HEIGHT * HALF;
            ui.painter().rect_filled(
                track,
                radius,
                appearance.primary.gamma_multiply(PROGRESS_TRACK_OPACITY),
            );
            ui.painter().rect_filled(
                Rect::from_min_size(
                    track.min,
                    egui::vec2(track.width() * ratio, PROGRESS_HEIGHT),
                ),
                radius,
                appearance.primary,
            );
        }
    });
}

pub(super) fn ai(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    mounting: bool,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    let now = web_time::Instant::now();
    data.observe_settings(settings);
    data.observe(Query::AiTokens, mounting, now);
    let tokens = match data.ai_tokens() {
        Status::Ready(tokens) => Some(tokens.clone()),
        Status::Pending | Status::Failed(_) => None,
    };
    let is_configured = |provider: AiProviderId| {
        tokens.as_ref().is_some_and(|tokens| match provider {
            AiProviderId::OllamaCloud => tokens.ollama_cloud,
            AiProviderId::Codex => tokens.codex,
            AiProviderId::Omlx => tokens.omlx,
        })
    };
    let selected = settings.ai_provider.unwrap_or(DEFAULT_AI_PROVIDER);
    let is_selected_configured = is_configured(selected);
    if is_selected_configured {
        data.observe(Query::AiModels(selected), mounting, now);
    }
    ui.scope(|ui| {
        ui.visuals_mut().extreme_bg_color = appearance.code.input_background;
        ui.visuals_mut().widgets.inactive.bg_stroke =
            Stroke::new(BORDER_WIDTH, appearance.code.input_border);
        ui.spacing_mut().item_spacing.y = LIST_GAP;
        for (provider, label, warning, draft) in [
            (
                AiProviderId::OllamaCloud,
                "settings.aiProviderOllamaCloud",
                None,
                &mut state.ollama_cloud_token,
            ),
            (
                AiProviderId::Codex,
                "settings.aiProviderCodex",
                Some("settings.aiCodexUnofficialWarning"),
                &mut state.codex_token,
            ),
        ] {
            ui.push_id(label, |ui| {
                let is_provider_configured = is_configured(provider);
                row_frame(appearance).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing = egui::vec2(HEADER_GAP, TOKEN_ROW_GAP);
                    header(
                        ui,
                        icons,
                        is_provider_configured,
                        &message(locale, label, &[]),
                        None,
                        &message(
                            locale,
                            if is_provider_configured {
                                "settings.aiTokenSaved"
                            } else {
                                "settings.aiTokenNotSet"
                            },
                            &[],
                        ),
                        LINE_HEIGHT,
                        appearance,
                        |_| {},
                    );
                    if let Some(warning) = warning {
                        indented(ui, |ui| {
                            small(
                                ui,
                                message(locale, warning, &[]),
                                appearance.sections.warning,
                            )
                        });
                    }
                    indented(ui, |ui| {
                        token_input(
                            ui,
                            provider,
                            draft,
                            "settings.aiTokenPlaceholder",
                            is_provider_configured,
                            data,
                            locale,
                            appearance,
                            output,
                        )
                    });
                });
            });
        }
        row_frame(appearance).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing = egui::vec2(INLINE_GAP, TOKEN_ROW_GAP);
            let stored = settings.ai_omlx_base_url.clone().unwrap_or_default();
            let draft = state
                .omlx_url
                .get_or_insert_with(|| TextDraft::new(stored.clone()));
            let response = code_view::labeled_text(
                ui,
                "settings.aiOmlxBaseUrlLabel",
                message(locale, "settings.aiOmlxBaseUrlPlaceholder", &[]),
                stored,
                draft,
                locale,
                appearance,
                output,
            );
            if response.has_focus() {
                state.is_omlx_url_committed = false;
            } else if response.lost_focus() && !state.is_omlx_url_committed {
                state.is_omlx_url_committed = true;
                output
                    .changes
                    .push(Change::Hosted(HostedChange::AiOmlxBaseUrl(
                        trim_text(&draft.text).into(),
                    )));
            }
            token_input(
                ui,
                AiProviderId::Omlx,
                &mut state.omlx_key,
                "settings.aiOmlxApiKeyOptional",
                true,
                data,
                locale,
                appearance,
                output,
            );
        });
    });
    let providers = AI_PROVIDERS
        .iter()
        .map(|(provider, label)| code_view::Choice {
            identity: (*label).into(),
            value: *provider,
            label: message(locale, label, &[]),
            search: String::new(),
            font: None,
        })
        .collect::<Vec<_>>();
    let selected_label = providers
        .iter()
        .find(|choice| choice.value == selected)
        .map(|choice| choice.label.clone())
        .unwrap_or_default();
    if let Some(provider) = option_row(
        ui,
        "settings.aiProviderLabel",
        &selected_label,
        &selected,
        &providers,
        &mut state.provider,
        locale,
        appearance,
        output,
    ) {
        output
            .changes
            .push(Change::Hosted(HostedChange::AiProvider(provider)));
    }
    if is_selected_configured {
        match data.ai_models(selected) {
            Status::Failed(_) => {
                small(
                    ui,
                    message(
                        locale,
                        if selected == AiProviderId::Omlx {
                            "settings.aiOmlxConnectFailed"
                        } else {
                            "settings.aiModelLoadFailed"
                        },
                        &[],
                    ),
                    appearance.sections.error,
                );
            }
            Status::Ready(models) if models.is_empty() => {
                small(
                    ui,
                    message(locale, "settings.aiModelSelectPlaceholder", &[]),
                    appearance.muted,
                );
            }
            Status::Ready(models) => {
                let choices = models
                    .iter()
                    .map(|model| {
                        let label = model
                            .display_name
                            .clone()
                            .unwrap_or_else(|| model.model_id.clone());
                        code_view::Choice {
                            identity: model.model_id.clone(),
                            value: model.model_id.clone(),
                            label,
                            search: String::new(),
                            font: None,
                        }
                    })
                    .collect::<Vec<_>>();
                let active = settings.ai_model.clone().unwrap_or_default();
                let active_label = choices
                    .iter()
                    .find(|choice| choice.value == active)
                    .map(|choice| choice.label.clone());
                state.model.is_placeholder = active_label.is_none();
                if let Some(model) = option_row(
                    ui,
                    "settings.aiModelLabel",
                    &active_label.unwrap_or_else(|| {
                        message(locale, "settings.aiModelSelectPlaceholder", &[])
                    }),
                    &active,
                    &choices,
                    &mut state.model,
                    locale,
                    appearance,
                    output,
                ) {
                    output.changes.push(Change::Hosted(HostedChange::AiModel {
                        provider: selected,
                        model,
                    }));
                }
            }
            Status::Pending => {}
        }
    }
    let is_auto_tab_disabled = !is_selected_configured;
    ui.add_enabled_ui(!is_auto_tab_disabled, |ui| {
        if switch_control(
            ui,
            "settings.aiAutoTabToggle",
            Some(if is_auto_tab_disabled {
                "settings.aiAutoTabProviderRequired"
            } else {
                "settings.aiAutoTabToggleHint"
            }),
            settings.ai_auto_tab_enabled,
            locale,
            appearance,
            output,
        ) {
            output.changes.push(Change::Hosted(HostedChange::AiAutoTab {
                provider: selected,
                enabled: !settings.ai_auto_tab_enabled,
            }));
        }
    });
    let prompts = egui::Frame::NONE
        .inner_margin(egui::Margin {
            top: PROMPT_LIST_TOP,
            ..egui::Margin::ZERO
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = LIST_GAP;
            for (id, label) in PROMPTS {
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), LINE_HEIGHT),
                    egui::Layout::right_to_left(Align::Center),
                    |ui| {
                        let edit = tooltip_icon_button(
                            ui,
                            Icon::Pencil,
                            &message(locale, "prompts.editEntry", &[]),
                            label,
                            appearance,
                            icons,
                            output,
                        );
                        if edit.clicked() {
                            output.open_app_file =
                                Some((AppFileTarget::Prompt { id }, message(locale, label, &[])));
                        }
                        ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                            small(ui, message(locale, label, &[]), appearance.foreground);
                        });
                    },
                );
            }
        });
    let top = prompts.response.rect.top();
    ui.painter().hline(
        prompts.response.rect.x_range(),
        top,
        Stroke::new(BORDER_WIDTH, appearance.border),
    );
}

fn tooltip_icon_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    _field: &'static str,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) -> egui::Response {
    let (response, trigger) = crate::tooltip_trigger::wrap_button(ui, label, false, |ui| {
        ui.add(
            egui::Button::opt_image_and_text(
                icons
                    .image(icon, appearance.foreground)
                    .map(|image| image.fit_to_exact_size(egui::Vec2::splat(ACTION_ICON_SIZE))),
                None,
            )
            .frame(false),
        )
    });
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label)
    });
    #[cfg(any(test, feature = "inspection"))]
    output.traces.push(Trace::capture(_field, &response));
    output.tooltips.push(trigger);
    response
}

fn xs_icon_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    rotation: f32,
    variant: Variant,
    appearance: &super::Appearance,
    icons: &Icons,
) -> egui::Response {
    let image = icons.image(icon, appearance.foreground).map(|image| {
        image
            .fit_to_exact_size(egui::Vec2::splat(ACTION_ICON_SIZE))
            .rotate(rotation, egui::Vec2::splat(ROTATION_ORIGIN))
    });
    let button = egui::Button::opt_image_and_text(
        image,
        Some(
            RichText::new(label)
                .size(TEXT_SIZE)
                .color(appearance.foreground)
                .into(),
        ),
    )
    .gap(XS_ICON_GAP)
    .corner_radius(ROW_RADIUS)
    .min_size(egui::vec2(0.0, XS_BUTTON_HEIGHT));
    let button = match variant {
        Variant::Outline => button.stroke(Stroke::new(BORDER_WIDTH, appearance.border)),
        Variant::Ghost => button.frame_when_inactive(false),
    };
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(XS_ICON_PADDING_X, 0.0);
        ui.add_enabled(enabled, button)
    })
    .inner
}

fn plugin_error_key(code: PluginErrorCode) -> &'static str {
    match code {
        PluginErrorCode::ParseFailed => "settings.pluginError.parseFailed",
        PluginErrorCode::IdMismatch => "settings.pluginError.idMismatch",
        PluginErrorCode::VersionMismatch => "settings.pluginError.versionMismatch",
        PluginErrorCode::PathEscape => "settings.pluginError.pathEscape",
        PluginErrorCode::GrammarMissing => "settings.pluginError.grammarMissing",
        PluginErrorCode::GrammarInvalid => "settings.pluginError.grammarInvalid",
        PluginErrorCode::GrammarConflict => "settings.pluginError.grammarConflict",
    }
}

fn failure_key(failure: VsixThemeImportFailure) -> &'static str {
    match failure {
        VsixThemeImportFailure::Parse => "settings.themeImportThemeParseFailure",
        VsixThemeImportFailure::Incomplete => "settings.themeImportThemeIncomplete",
        VsixThemeImportFailure::Contrast => "settings.themeImportThemeContrastFailure",
    }
}

pub(super) fn plugins(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    mounting: bool,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    data.observe(Query::Plugins, mounting, web_time::Instant::now());
    dialogs::settle(ui, &mut state.returning_focus);
    ui.spacing_mut().item_spacing.y = PLUGIN_SECTION_GAP;
    match data.plugins() {
        Status::Pending => {
            small(
                ui,
                message(locale, "settings.loading", &[]),
                appearance.muted,
            );
        }
        Status::Ready(plugins) if !plugins.is_empty() => {
            let plugins = plugins.clone();
            plugin_list(ui, &plugins, state, locale, appearance, icons, output);
        }
        Status::Ready(_) | Status::Failed(_) => {
            let empty = egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(ROW_PADDING_X, EMPTY_PADDING_Y))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = EMPTY_LINE_GAP;
                    small(
                        ui,
                        message(locale, "settings.pluginsEmpty", &[]),
                        appearance.muted,
                    );
                    small(
                        ui,
                        message(locale, "settings.pluginsManifestHint", &[]),
                        appearance.muted,
                    );
                });
            let rect = empty.response.rect;
            ui.painter().extend(egui::Shape::dashed_line(
                &[
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ],
                Stroke::new(BORDER_WIDTH, appearance.border),
                DASH_LENGTH,
                DASH_LENGTH,
            ));
        }
    }
    let mut vsix_origin = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::splat(PLUGIN_INLINE_GAP);
        let install = xs_icon_button(
            ui,
            Icon::Import,
            &message(locale, "settings.pluginInstallButton", &[]),
            !data.is_installing_plugin(),
            0.0,
            Variant::Outline,
            appearance,
            icons,
        );
        let import = xs_icon_button(
            ui,
            Icon::Import,
            &message(locale, "settings.pluginImportVsixButton", &[]),
            !data.is_extracting_vsix(),
            0.0,
            Variant::Outline,
            appearance,
            icons,
        );
        let is_reloading = data.is_reloading_plugins();
        let rotation = if is_reloading {
            ui.ctx().request_repaint();
            (ui.input(|input| input.time).rem_euclid(SPIN_SECONDS) / SPIN_SECONDS) as f32
                * std::f32::consts::TAU
        } else {
            0.0
        };
        let reload = xs_icon_button(
            ui,
            Icon::RefreshCw,
            &message(locale, "settings.pluginsReload", &[]),
            !is_reloading,
            rotation,
            Variant::Outline,
            appearance,
            icons,
        );
        let folder = xs_icon_button(
            ui,
            Icon::FolderOpen,
            &message(locale, "settings.pluginsOpenFolder", &[]),
            true,
            0.0,
            Variant::Outline,
            appearance,
            icons,
        );
        #[cfg(any(test, feature = "inspection"))]
        for (field, response) in [
            ("settings.pluginInstallButton", &install),
            ("settings.pluginImportVsixButton", &import),
            ("settings.pluginsReload", &reload),
            ("settings.pluginsOpenFolder", &folder),
        ] {
            output.traces.push(Trace::capture(field, response));
        }
        if install.clicked() {
            data.request_pick(PickPurpose::PluginArchive);
        }
        if import.clicked()
            && let Some(ticket) = data.next_vsix_ticket()
        {
            state.vsix_ticket = Some(ticket);
            data.request_pick(PickPurpose::Vsix(ticket));
        }
        if reload.clicked() {
            data.reload_plugins();
        }
        if folder.clicked() {
            output
                .folders
                .push(taide_model::system::AppDataPathKind::Plugins);
        }
        vsix_origin = Some(import.id);
    });
    if let Some(ticket) = state.vsix_ticket {
        for event in data.take_vsix_events(ticket) {
            match event {
                VsixEvent::Extracted(extraction) => {
                    state.vsix =
                        vsix_origin.map(|origin| VsixDialog::new(ticket, extraction, origin));
                }
                VsixEvent::Saved => {
                    if let Some(dialog) = &mut state.vsix {
                        dialog.is_saving = false;
                    }
                }
                VsixEvent::GrammarsImported(result) => {
                    if let Some(dialog) = &mut state.vsix {
                        dialog.grammars = match result {
                            Ok(()) => Grammars::Imported,
                            Err(error) => Grammars::Failed(error),
                        };
                    }
                }
            }
        }
    }
    let uninstall = state.uninstall.as_mut().and_then(|(id, name, focus)| {
        let alert = Alert {
            title: message(
                locale,
                "settings.pluginUninstallConfirmTitle",
                &[("name", name)],
            ),
            description: message(locale, "settings.pluginUninstallConfirmDescription", &[]),
            action: message(locale, "settings.pluginUninstallButton", &[]),
            is_destructive: true,
            size: Size::Small,
            alternative: None,
        };
        dialogs::alert(
            ui,
            ui.id().with("plugin-uninstall"),
            focus,
            &alert,
            locale,
            appearance,
            output,
        )
        .map(|choice| (choice, id.clone()))
    });
    if let Some((choice, id)) = uninstall {
        if choice == Choice::Confirm {
            data.act(Action::PluginUninstall { id });
        }
        if let Some((_, _, focus)) = state.uninstall.take() {
            state.returning_focus = focus.close();
        }
    }
    if state.vsix.is_some() && vsix_dialog(ui, data, state, locale, appearance, icons, output) {
        if let Some(dialog) = state.vsix.take() {
            state.returning_focus = dialog.focus.close();
        }
    }
}

fn plugin_list(
    ui: &mut Ui,
    plugins: &[LoadedPlugin],
    state: &mut OwnerState,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    let uninstall = message(locale, "settings.pluginUninstallButton", &[]);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = LIST_GAP;
        for plugin in plugins {
            ui.push_id(plugin.manifest.id.as_str(), |ui| {
                row_frame(appearance).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing = egui::vec2(PLUGIN_INLINE_GAP, PLUGIN_ROW_GAP);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), LINE_HEIGHT),
                        egui::Layout::right_to_left(Align::Center),
                        |ui| {
                            let trash = tooltip_icon_button(
                                ui,
                                Icon::Trash,
                                &uninstall,
                                "settings.pluginUninstallButton",
                                appearance,
                                icons,
                                output,
                            );
                            if trash.clicked() && state.uninstall.is_none() {
                                state.uninstall = Some((
                                    plugin.manifest.id.clone(),
                                    plugin.manifest.name.clone(),
                                    dialogs::Focus::open(trash.id),
                                ));
                            }
                            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&plugin.manifest.name)
                                            .size(TEXT_SIZE)
                                            .family(crate::font_families::medium(ui))
                                            .color(appearance.foreground),
                                    )
                                    .truncate(),
                                );
                                ui.label(
                                    RichText::new(&plugin.manifest.version)
                                        .monospace()
                                        .size(TEXT_SIZE)
                                        .color(appearance.muted),
                                );
                                ui.label(
                                    RichText::new(message(
                                        locale,
                                        if plugin.enabled {
                                            "settings.pluginEnabled"
                                        } else {
                                            "settings.pluginDisabled"
                                        },
                                        &[],
                                    ))
                                    .size(TEXT_SIZE)
                                    .color(
                                        if plugin.enabled {
                                            appearance.sections.agent_running
                                        } else {
                                            appearance.muted
                                        },
                                    ),
                                );
                            });
                        },
                    );
                    if let Some(code) = plugin.error {
                        small(
                            ui,
                            message(locale, plugin_error_key(code), &[]),
                            appearance.sections.error,
                        );
                    }
                });
            });
        }
    });
}

fn checkbox(
    ui: &mut Ui,
    checked: bool,
    enabled: bool,
    appearance: &super::Appearance,
) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(CHECKBOX_SIZE), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), checked, "")
        });
        if checked {
            ui.painter()
                .rect_filled(rect, CHECKBOX_RADIUS, appearance.primary);
            check(ui, rect, appearance.sections.primary_foreground);
        } else {
            ui.painter().rect_stroke(
                rect,
                CHECKBOX_RADIUS,
                Stroke::new(BORDER_WIDTH, appearance.input),
                egui::StrokeKind::Inside,
            );
        }
        response
    })
    .inner
}

fn vsix_dialog(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) -> bool {
    let Some(dialog) = state.vsix.as_mut() else {
        return false;
    };
    dialogs::settle(ui, &mut dialog.returning_focus);
    let id = ui.id().with("vsix-import");
    let title = message(locale, "settings.pluginImportVsixButton", &[]);
    let description = dialog.extraction.extension.clone();
    let candidates = dialog.extraction.candidates.clone();
    let mut toggled = Vec::new();
    let mut save = None;
    let mut import_grammars = false;
    let selected_count = dialog.selected().count();
    let is_saving = dialog.is_saving;
    let selected = dialog.selected.clone();
    let grammars = match &dialog.grammars {
        Grammars::Idle => None,
        Grammars::Importing => Some(None),
        Grammars::Imported => Some(Some(Ok(()))),
        Grammars::Failed(error) => Some(Some(Err(crate::toast::describe_error(locale, error)))),
    };
    let (_, close) = dialogs::dialog(
        ui,
        id,
        &mut dialog.focus,
        &title,
        &description,
        locale,
        appearance,
        icons,
        output,
        |ui, order, _output| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = VSIX_SECTION_GAP;
                section_heading(
                    ui,
                    "settings.pluginImportVsixThemesSection",
                    locale,
                    appearance,
                );
                if candidates.is_empty() {
                    small(
                        ui,
                        message(locale, "settings.pluginImportVsixNoThemes", &[]),
                        appearance.muted,
                    );
                    return;
                }
                egui::ScrollArea::vertical()
                    .id_salt("vsix-candidates")
                    .max_height(VSIX_LIST_HEIGHT)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = LIST_GAP;
                        for candidate in &candidates {
                            ui.push_id(candidate.key.as_str(), |ui| {
                                ui.scope(|ui| {
                                    if candidate.failure.is_some() {
                                        ui.set_opacity(FAILED_ROW_OPACITY);
                                    }
                                    egui::Frame::NONE
                                        .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
                                        .corner_radius(VSIX_ROW_RADIUS)
                                        .inner_margin(egui::Margin::symmetric(
                                            VSIX_ROW_PADDING_X,
                                            VSIX_ROW_PADDING_Y,
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_min_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = PLUGIN_INLINE_GAP;
                                                let check = checkbox(
                                                    ui,
                                                    selected.contains(&candidate.key),
                                                    candidate.failure.is_none(),
                                                    appearance,
                                                );
                                                if check.enabled() {
                                                    order.push(check.id);
                                                }
                                                if check.clicked() {
                                                    toggled.push(candidate.key.clone());
                                                }
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(Align::Center),
                                                    |ui| {
                                                        if let Some(failure) = candidate.failure {
                                                            small(
                                                                ui,
                                                                message(
                                                                    locale,
                                                                    failure_key(failure),
                                                                    &[],
                                                                ),
                                                                appearance.sections.error,
                                                            );
                                                        }
                                                        ui.with_layout(
                                                            egui::Layout::top_down(Align::Min),
                                                            |ui| {
                                                                ui.spacing_mut().item_spacing.y =
                                                                    0.0;
                                                                candidate_detail(
                                                                    ui, candidate, locale,
                                                                    appearance, icons,
                                                                );
                                                            },
                                                        );
                                                    },
                                                );
                                            });
                                        });
                                });
                            });
                        }
                    });
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
                    egui::Layout::right_to_left(Align::Center),
                    |ui| {
                        let response = ui
                            .scope(|ui| {
                                ui.spacing_mut().button_padding =
                                    egui::vec2(XS_BUTTON_PADDING_X, 0.0);
                                ui.add_enabled(
                                    selected_count > 0 && !is_saving,
                                    egui::Button::new(
                                        RichText::new(message(locale, "common.save", &[]))
                                            .size(TEXT_SIZE)
                                            .color(appearance.sections.primary_foreground),
                                    )
                                    .fill(appearance.primary)
                                    .corner_radius(ROW_RADIUS)
                                    .min_size(egui::vec2(0.0, XS_BUTTON_HEIGHT)),
                                )
                            })
                            .inner;
                        #[cfg(any(test, feature = "inspection"))]
                        _output
                            .traces
                            .push(Trace::capture("common.save", &response));
                        if response.enabled() {
                            order.push(response.id);
                        }
                        if response.clicked() {
                            save = Some(response.id);
                        }
                    },
                );
            });
            let grammars_section = egui::Frame::NONE
                .inner_margin(egui::Margin {
                    top: PROMPT_LIST_TOP,
                    ..egui::Margin::ZERO
                })
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = VSIX_SECTION_GAP;
                    section_heading(
                        ui,
                        "settings.pluginImportVsixGrammarsSection",
                        locale,
                        appearance,
                    );
                    match &grammars {
                        Some(Some(Ok(()))) => {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = VSIX_DETAIL_GAP;
                                if let Some(image) =
                                    icons.image(Icon::CircleCheck, appearance.sections.success)
                                {
                                    ui.add(
                                        image
                                            .fit_to_exact_size(egui::Vec2::splat(ACTION_ICON_SIZE)),
                                    );
                                }
                                small(
                                    ui,
                                    message(locale, "settings.pluginImportVsixSuccess", &[]),
                                    appearance.sections.success,
                                );
                            });
                        }
                        Some(Some(Err(text))) => {
                            small(ui, text.clone(), appearance.sections.error);
                        }
                        _ => {
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
                                egui::Layout::right_to_left(Align::Center),
                                |ui| {
                                    let response = xs_button(
                                        ui,
                                        &message(locale, "settings.pluginImportVsixButton", &[]),
                                        Variant::Outline,
                                        grammars.is_none(),
                                        appearance,
                                    );
                                    #[cfg(any(test, feature = "inspection"))]
                                    _output.traces.push(Trace::capture(
                                        "settings.pluginImportVsixGrammars",
                                        &response,
                                    ));
                                    if response.enabled() {
                                        order.push(response.id);
                                    }
                                    import_grammars = response.clicked();
                                },
                            );
                        }
                    }
                });
            let rect = grammars_section.response.rect;
            ui.painter().hline(
                rect.x_range(),
                rect.top(),
                Stroke::new(BORDER_WIDTH, appearance.sections.separator),
            );
        },
    );
    for key in toggled {
        if !dialog.selected.remove(&key) {
            dialog.selected.insert(key);
        }
    }
    if import_grammars {
        dialog.grammars = Grammars::Importing;
        data.act(Action::VsixImportGrammars {
            path: dialog.extraction.path.clone(),
            ticket: dialog.ticket,
        });
    }
    if let Some(origin) = save {
        if dialog.duplicates(locale).is_empty() {
            dialog.persist(data, locale);
        } else {
            dialog.confirm = Some(dialogs::Focus::open(origin));
        }
    }
    if dialog.confirm.is_some() {
        let description = dialog.duplicates(locale).join(DUPLICATE_SEPARATOR);
        let choice = dialog.confirm.as_mut().and_then(|focus| {
            dialogs::alert(
                ui,
                id.with("overwrite"),
                focus,
                &Alert {
                    title: message(locale, "common.confirm", &[]),
                    description,
                    action: message(locale, "common.confirm", &[]),
                    is_destructive: false,
                    size: Size::Small,
                    alternative: None,
                },
                locale,
                appearance,
                output,
            )
        });
        if let Some(choice) = choice {
            if let Some(focus) = dialog.confirm.take() {
                dialog.returning_focus = focus.close();
            }
            if choice == Choice::Confirm {
                dialog.persist(data, locale);
            }
        }
        return false;
    }
    close
}

fn section_heading(
    ui: &mut Ui,
    key: &str,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
) {
    ui.label(
        RichText::new(message(locale, key, &[]))
            .size(TEXT_SIZE)
            .family(crate::font_families::medium(ui))
            .color(appearance.muted),
    );
}

fn candidate_detail(
    ui: &mut Ui,
    candidate: &taide_model::vsix::VsixThemeCandidate,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
) {
    ui.add(
        egui::Label::new(
            RichText::new(&candidate.label)
                .size(TEXT_SIZE)
                .color(appearance.foreground),
        )
        .truncate(),
    );
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = VSIX_DETAIL_GAP;
        small(
            ui,
            message(
                locale,
                match candidate.theme_type {
                    ThemeType::Dark => "settings.themeDark",
                    ThemeType::Light => "settings.themeLight",
                },
                &[],
            ),
            appearance.muted,
        );
        if candidate.failure.is_some() {
            return;
        }
        if candidate.warning_count > 0 {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = VSIX_WARNING_GAP;
                if let Some(image) = icons.image(Icon::Warning, appearance.sections.warning) {
                    ui.add(image.fit_to_exact_size(egui::Vec2::splat(VSIX_WARNING_ICON)));
                }
                small(
                    ui,
                    candidate.warning_count.to_string(),
                    appearance.sections.warning,
                );
            });
        }
        if candidate.id_collides {
            small(
                ui,
                message(
                    locale,
                    "settings.themeImportDuplicate",
                    &[("name", &candidate.id)],
                ),
                appearance.sections.warning,
            );
        }
    });
}

fn token_input(
    ui: &mut Ui,
    provider: AiProviderId,
    draft: &mut String,
    placeholder: &'static str,
    has_clear: bool,
    data: &mut Data,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    _output: &mut Output,
) {
    let is_saving = data.is_saving_ai_token(provider);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
        egui::Layout::right_to_left(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = INLINE_GAP;
            if has_clear {
                let response = xs_button(
                    ui,
                    &message(locale, "settings.aiTokenClear", &[]),
                    Variant::Ghost,
                    true,
                    appearance,
                );
                #[cfg(any(test, feature = "inspection"))]
                _output
                    .traces
                    .push(Trace::capture("settings.aiTokenClear", &response));
                if response.clicked() {
                    data.act(Action::AiClearToken(provider));
                }
            }
            let save = xs_button(
                ui,
                &message(locale, "settings.aiTokenSave", &[]),
                Variant::Outline,
                !trim_text(draft).is_empty() && !is_saving,
                appearance,
            );
            #[cfg(any(test, feature = "inspection"))]
            _output
                .traces
                .push(Trace::capture("settings.aiTokenSave", &save));
            if save.clicked() {
                let token = trim_text(draft).to_owned();
                if !token.is_empty() {
                    draft.clear();
                    data.save_ai_token(provider, Secret::new(token));
                }
            }
            let _input = ui.add_sized(
                [ui.available_width(), LINE_HEIGHT],
                egui::TextEdit::singleline(draft)
                    .password(true)
                    .return_key(None)
                    .font(FontId::proportional(TEXT_SIZE))
                    .hint_text(message(locale, placeholder, &[]))
                    .margin(egui::Margin::symmetric(
                        code_view::INPUT_PADDING_X,
                        code_view::INPUT_PADDING_Y,
                    )),
            );
            #[cfg(any(test, feature = "inspection"))]
            _output.traces.push(Trace::capture(placeholder, &_input));
        },
    );
}

fn option_row<T: Clone + PartialEq>(
    ui: &mut Ui,
    label_key: &'static str,
    active_text: &str,
    active: &T,
    choices: &[code_view::Choice<T>],
    picker: &mut code_view::Picker,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    output: &mut Output,
) -> Option<T> {
    ui.push_id(label_key, |ui| {
        let width = ui.available_width();
        ui.horizontal(|ui| {
            code_view::left_label(
                ui,
                (width - code_view::OPTION_WIDTH - CONTROL_GAP).max(0.0),
                label_key,
                locale,
                appearance,
            );
            ui.allocate_ui_with_layout(
                egui::vec2(code_view::OPTION_WIDTH, COMBO_TRIGGER_HEIGHT),
                egui::Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(code_view::OPTION_WIDTH);
                    picker.show(
                        ui,
                        label_key,
                        active_text,
                        active,
                        choices,
                        None,
                        None,
                        false,
                        egui::emath::RectAlign::BOTTOM_END,
                        locale,
                        appearance,
                        output,
                    )
                },
            )
            .inner
        })
        .inner
    })
    .inner
}

fn input_visuals(ui: &mut Ui, appearance: &super::Appearance) {
    ui.visuals_mut().extreme_bg_color = appearance.code.input_background;
    ui.visuals_mut().widgets.inactive.bg_stroke =
        Stroke::new(BORDER_WIDTH, appearance.code.input_border);
}

fn password_edit<'a>(
    draft: &'a mut String,
    hint: String,
    salt: &'static str,
) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(draft)
        .id_salt(salt)
        .password(true)
        .return_key(None)
        .font(FontId::proportional(TEXT_SIZE))
        .hint_text(hint)
        .margin(egui::Margin::symmetric(
            code_view::INPUT_PADDING_X,
            code_view::INPUT_PADDING_Y,
        ))
}

fn status_icon(ui: &mut Ui, icons: &Icons, is_ok: bool, tone: Color32) {
    let icon = if is_ok {
        Icon::CircleCheck
    } else {
        Icon::CircleX
    };
    if let Some(image) = icons.image(icon, tone) {
        ui.add(image.fit_to_exact_size(egui::Vec2::splat(ACTION_ICON_SIZE)));
    }
}

fn detail_row(ui: &mut Ui, label: String, value: String, appearance: &super::Appearance) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), LINE_HEIGHT),
        egui::Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = CONTROL_GAP;
            small(ui, label, appearance.muted);
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(value)
                            .size(TEXT_SIZE)
                            .color(appearance.foreground),
                    )
                    .truncate(),
                );
            });
        },
    );
}

fn selectable_line(ui: &mut Ui, text: &str, appearance: &super::Appearance) -> egui::Response {
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(TEXT_SIZE)
                .color(appearance.foreground),
        )
        .wrap()
        .selectable(true),
    )
}

pub(super) fn sync(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    mounting: bool,
    host: Host,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    data.observe(Query::SyncStatus, mounting, web_time::Instant::now());
    dialogs::settle(ui, &mut state.returning_focus);
    let status = match data.sync_status() {
        Status::Ready(status) if status.connected => Some(status.clone()),
        Status::Ready(_) | Status::Pending | Status::Failed(_) => None,
    };
    ui.spacing_mut().item_spacing.y = SECTION_GAP;
    input_visuals(ui, appearance);
    match status {
        None => sync_disconnected(ui, data, state, locale, appearance, output),
        Some(status) => {
            sync_connected(
                ui, data, state, &status, host, settings, locale, appearance, icons, output,
            );
        }
    }
    if !state.container.is_sync_conflict_open {
        return;
    }
    let origin = state.sync_download.unwrap_or_else(|| ui.id());
    let focus = state
        .sync_conflict
        .get_or_insert_with(|| dialogs::Focus::open(origin));
    let alert = Alert {
        title: message(locale, "settings.syncConflictTitle", &[]),
        description: message(locale, "settings.syncConflictDescription", &[]),
        action: message(locale, "settings.syncConflictPullRemote", &[]),
        is_destructive: false,
        size: Size::Small,
        alternative: Some(message(locale, "settings.syncConflictKeepLocal", &[])),
    };
    let Some(choice) = dialogs::alert(
        ui,
        ui.id().with("sync-conflict"),
        focus,
        &alert,
        locale,
        appearance,
        output,
    ) else {
        return;
    };
    state.container.is_sync_conflict_open = false;
    if let Some(focus) = state.sync_conflict.take() {
        state.returning_focus = focus.close();
    }
    match choice {
        Choice::Alternative => data.act(Action::SyncUpload),
        Choice::Confirm => {
            if let Some(owner) = state.container.owner {
                data.act(Action::SyncDownload { force: true, owner });
            }
        }
        Choice::Cancel => {}
    }
}

fn sync_disconnected(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    _output: &mut Output,
) {
    small(
        ui,
        message(locale, "settings.syncDescription", &[]),
        appearance.muted,
    );
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = HINT_GAP;
        small(
            ui,
            message(locale, "settings.syncTokenScopeHint", &[]),
            appearance.muted,
        );
        let _url = selectable_line(ui, GITHUB_TOKEN_CREATE_URL, appearance);
        #[cfg(any(test, feature = "inspection"))]
        _output
            .traces
            .push(Trace::capture("settings.syncTokenUrl", &_url));
    });
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
        egui::Layout::right_to_left(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = INLINE_GAP;
            let connect = xs_button(
                ui,
                &message(locale, "settings.syncConnect", &[]),
                Variant::Outline,
                !trim_text(&state.sync_token).is_empty() && !data.is_busy(Busy::SyncConnect),
                appearance,
            );
            if connect.clicked() {
                let token = trim_text(&state.sync_token).to_owned();
                if !token.is_empty() {
                    state.sync_token.clear();
                    data.act(Action::SyncConnect(Secret::new(token)));
                }
            }
            let _input = ui.add_sized(
                [ui.available_width(), LINE_HEIGHT],
                password_edit(
                    &mut state.sync_token,
                    message(locale, "settings.syncTokenPlaceholder", &[]),
                    "settings-sync-token",
                ),
            );
            #[cfg(any(test, feature = "inspection"))]
            {
                _output
                    .traces
                    .push(Trace::capture("settings.syncConnect", &connect));
                _output
                    .traces
                    .push(Trace::capture("settings.syncTokenPlaceholder", &_input));
            }
        },
    );
}

fn sync_connected(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    status: &taide_model::sync::SyncStatus,
    host: Host,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    _output: &mut Output,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), LINE_HEIGHT),
        egui::Layout::right_to_left(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = HEADER_GAP;
            if status.remote_newer == Some(true) {
                egui::Frame::NONE
                    .fill(
                        appearance
                            .sections
                            .warning
                            .gamma_multiply(NEWER_BADGE_OPACITY),
                    )
                    .corner_radius(BADGE_RADIUS)
                    .inner_margin(egui::Margin::symmetric(
                        NEWER_BADGE_PADDING_X,
                        NEWER_BADGE_PADDING_Y,
                    ))
                    .show(ui, |ui| {
                        small(
                            ui,
                            message(locale, "settings.syncRemoteNewerBadge", &[]),
                            appearance.sections.warning,
                        );
                    });
            }
            ui.with_layout(egui::Layout::left_to_right(Align::Center), |ui| {
                status_icon(ui, icons, true, appearance.sections.agent_running);
                ui.add(
                    egui::Label::new(
                        RichText::new(message(locale, "settings.syncSectionTitle", &[]))
                            .size(TEXT_SIZE)
                            .family(crate::font_families::medium(ui))
                            .color(appearance.foreground),
                    )
                    .truncate(),
                );
            });
        },
    );
    let gist = settings
        .sync_gist_id
        .clone()
        .unwrap_or_else(|| message(locale, "settings.syncGistIdNone", &[]));
    let synced = status.last_synced_at.as_deref().map_or_else(
        || message(locale, "settings.syncLastSyncedNever", &[]),
        |at| (host.format_time)(at).unwrap_or_else(|| at.to_owned()),
    );
    indented(ui, |ui| {
        ui.spacing_mut().item_spacing.y = SECTION_GAP;
        detail_row(
            ui,
            message(locale, "settings.syncGistIdLabel", &[]),
            gist,
            appearance,
        );
        detail_row(
            ui,
            message(locale, "settings.syncLastSyncedLabel", &[]),
            synced,
            appearance,
        );
        small(
            ui,
            message(locale, "settings.syncSecretGistWarning", &[]),
            appearance.sections.warning,
        );
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
            egui::Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = INLINE_GAP;
                let upload = xs_button(
                    ui,
                    &message(locale, "settings.syncUploadNow", &[]),
                    Variant::Outline,
                    !data.is_busy(Busy::SyncUpload),
                    appearance,
                );
                let download = xs_button(
                    ui,
                    &message(locale, "settings.syncDownloadNow", &[]),
                    Variant::Outline,
                    !data.is_busy(Busy::SyncDownload),
                    appearance,
                );
                let disconnect = ui
                    .with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        xs_icon_button(
                            ui,
                            Icon::CircleX,
                            &message(locale, "settings.syncDisconnect", &[]),
                            !data.is_busy(Busy::SyncDisconnect),
                            0.0,
                            Variant::Ghost,
                            appearance,
                            icons,
                        )
                    })
                    .inner;
                #[cfg(any(test, feature = "inspection"))]
                for (field, response) in [
                    ("settings.syncUploadNow", &upload),
                    ("settings.syncDownloadNow", &download),
                    ("settings.syncDisconnect", &disconnect),
                ] {
                    _output.traces.push(Trace::capture(field, response));
                }
                state.sync_download = Some(download.id);
                if upload.clicked() {
                    data.act(Action::SyncUpload);
                }
                if download.clicked()
                    && let Some(owner) = state.container.owner
                {
                    data.act(Action::SyncDownload {
                        force: false,
                        owner,
                    });
                }
                if disconnect.clicked() {
                    data.act(Action::SyncDisconnect);
                }
            },
        );
    });
}

pub(super) fn remote(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    mounting: bool,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    let now = web_time::Instant::now();
    data.observe_settings(settings);
    data.observe(Query::RemoteStatus, mounting, now);
    if let Some(due) = data.remote_refetch_in(now) {
        ui.ctx().request_repaint_after(due);
    }
    let status = match data.remote_status() {
        Status::Ready(status) => Some(*status),
        Status::Pending | Status::Failed(_) => None,
    };
    let is_running = status.is_some_and(|status| status.running);
    let is_password_configured = status.is_some_and(|status| status.password_configured);
    ui.spacing_mut().item_spacing.y = SECTION_GAP;
    input_visuals(ui, appearance);
    if switch_control(
        ui,
        "remote.enableToggle",
        Some("remote.enableToggleHint"),
        settings.remote_access_enabled,
        locale,
        appearance,
        output,
    ) {
        state.container.issued_remote_url = None;
        data.act(Action::UpdateSettings(HostedChange::RemoteAccess(
            !settings.remote_access_enabled,
        )));
    }
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), LINE_HEIGHT),
        egui::Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = HEADER_GAP;
            let tone = if is_running {
                appearance.sections.agent_running
            } else {
                appearance.muted
            };
            status_icon(ui, icons, is_running, tone);
            let port = status.map_or(0, |status| status.port).to_string();
            small(
                ui,
                if is_running {
                    message(locale, "remote.statusRunning", &[("port", &port)])
                } else {
                    message(locale, "remote.statusStopped", &[])
                },
                appearance.foreground,
            );
            if is_running {
                let count = status.map_or(0, |status| status.client_count).to_string();
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    small(
                        ui,
                        message(locale, "remote.clientCountLabel", &[("count", &count)]),
                        appearance.muted,
                    );
                });
            }
        },
    );
    small(
        ui,
        message(locale, "remote.securityWarning", &[]),
        appearance.sections.warning,
    );
    remote_password(
        ui,
        data,
        state,
        is_password_configured,
        locale,
        appearance,
        icons,
        output,
    );
    if is_password_configured
        && switch_control(
            ui,
            "remote.passwordOnlyToggle",
            Some("remote.passwordOnlyToggleDescription"),
            settings.remote_password_only_login,
            locale,
            appearance,
            output,
        )
    {
        data.act(Action::UpdateSettings(HostedChange::RemotePasswordOnly(
            !settings.remote_password_only_login,
        )));
    }
    remote_hosts(ui, data, state, settings, locale, appearance, icons, output);
    if let Some(url) = &state.container.issued_remote_url {
        let _url = selectable_line(ui, url, appearance);
        #[cfg(any(test, feature = "inspection"))]
        output
            .traces
            .push(Trace::capture("remote.issuedUrl", &_url));
    }
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
        egui::Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = INLINE_GAP;
            let issue = xs_button(
                ui,
                &message(locale, "remote.issueLink", &[]),
                Variant::Outline,
                is_running && !data.is_busy(Busy::RemoteIssueLink),
                appearance,
            );
            let revoke = ui
                .with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    xs_icon_button(
                        ui,
                        Icon::CircleX,
                        &message(locale, "remote.revokeSessions", &[]),
                        is_running && !data.is_busy(Busy::RemoteRevoke),
                        0.0,
                        Variant::Ghost,
                        appearance,
                        icons,
                    )
                })
                .inner;
            #[cfg(any(test, feature = "inspection"))]
            for (field, response) in [
                ("remote.issueLink", &issue),
                ("remote.revokeSessions", &revoke),
            ] {
                output.traces.push(Trace::capture(field, response));
            }
            if issue.clicked()
                && let Some(owner) = state.container.owner
            {
                data.act(Action::RemoteIssueLink { owner });
            }
            if revoke.clicked() {
                data.act(Action::RemoteRevoke);
            }
        },
    );
}

fn remote_password(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    is_configured: bool,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    _output: &mut Output,
) {
    row_frame(appearance).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.spacing_mut().item_spacing = egui::vec2(HEADER_GAP, PASSWORD_ROW_GAP);
        header(
            ui,
            icons,
            is_configured,
            &message(locale, "remote.passwordLabel", &[]),
            None,
            &message(
                locale,
                if is_configured {
                    "remote.passwordConfigured"
                } else {
                    "remote.passwordNotConfigured"
                },
                &[],
            ),
            LINE_HEIGHT,
            appearance,
            |_| {},
        );
        if !is_configured {
            indented(ui, |ui| {
                small(
                    ui,
                    message(locale, "remote.passwordHint", &[]),
                    appearance.sections.warning,
                )
            });
        }
        let length = trim_text(&state.remote_password).chars().count();
        let is_too_short = length > 0 && length < REMOTE_PASSWORD_MIN_LEN;
        if is_too_short {
            indented(ui, |ui| {
                small(
                    ui,
                    message(
                        locale,
                        "remote.passwordTooShort",
                        &[("min", &REMOTE_PASSWORD_MIN_LEN.to_string())],
                    ),
                    appearance.sections.warning,
                )
            });
        }
        indented(ui, |ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = INLINE_GAP;
                    if is_configured {
                        let clear = xs_button(
                            ui,
                            &message(locale, "remote.passwordClear", &[]),
                            Variant::Ghost,
                            true,
                            appearance,
                        );
                        #[cfg(any(test, feature = "inspection"))]
                        _output
                            .traces
                            .push(Trace::capture("remote.passwordClear", &clear));
                        if clear.clicked() {
                            data.act(Action::RemoteClearPassword);
                        }
                    }
                    let set = xs_button(
                        ui,
                        &message(locale, "remote.passwordSet", &[]),
                        Variant::Outline,
                        length > 0 && !is_too_short && !data.is_busy(Busy::RemotePassword),
                        appearance,
                    );
                    if set.clicked() && length > 0 && !is_too_short {
                        let password = trim_text(&state.remote_password).to_owned();
                        state.remote_password.clear();
                        data.act(Action::RemoteSetPassword(Secret::new(password)));
                    }
                    let _input = ui.add_sized(
                        [ui.available_width(), LINE_HEIGHT],
                        password_edit(
                            &mut state.remote_password,
                            message(locale, "remote.passwordPlaceholder", &[]),
                            "settings-remote-password",
                        ),
                    );
                    #[cfg(any(test, feature = "inspection"))]
                    {
                        _output
                            .traces
                            .push(Trace::capture("remote.passwordSet", &set));
                        _output
                            .traces
                            .push(Trace::capture("remote.passwordPlaceholder", &_input));
                    }
                },
            );
        });
    });
}

fn remote_hosts(
    ui: &mut Ui,
    data: &mut Data,
    state: &mut OwnerState,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
) {
    let hosts = &settings.remote_allowed_hosts;
    let is_saving = data.is_busy(Busy::Settings);
    let mut removed = None;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = HOSTS_GAP;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = HINT_GAP;
            small(
                ui,
                message(locale, "remote.allowedHostsLabel", &[]),
                appearance.foreground,
            );
            small(
                ui,
                message(locale, "remote.allowedHostsDescription", &[]),
                appearance.muted,
            );
        });
        if !hosts.is_empty() {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = HOST_LIST_GAP;
                for host in hosts {
                    ui.push_id(host, |ui| {
                        egui::Frame::NONE
                            .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
                            .corner_radius(HOST_ROW_RADIUS)
                            .inner_margin(egui::Margin::symmetric(
                                HOST_ROW_PADDING_X,
                                HOST_ROW_PADDING_Y,
                            ))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), HOST_REMOVE_SIZE),
                                    egui::Layout::right_to_left(Align::Center),
                                    |ui| {
                                        let label = message(
                                            locale,
                                            "remote.allowedHostsRemove",
                                            &[("host", host)],
                                        );
                                        let (remove, trigger) = crate::tooltip_trigger::wrap_button(
                                            ui,
                                            &label,
                                            is_saving,
                                            |ui| {
                                                ui.add_enabled(
                                                    !is_saving,
                                                    egui::Button::opt_image_and_text(
                                                        icons
                                                            .image(Icon::Close, appearance.muted)
                                                            .map(|image| {
                                                                image.fit_to_exact_size(
                                                                    egui::Vec2::splat(
                                                                        ACTION_ICON_SIZE,
                                                                    ),
                                                                )
                                                            }),
                                                        None,
                                                    )
                                                    .frame(false)
                                                    .min_size(egui::Vec2::splat(HOST_REMOVE_SIZE)),
                                                )
                                            },
                                        );
                                        remove.widget_info(|| {
                                            egui::WidgetInfo::labeled(
                                                egui::WidgetType::Button,
                                                remove.enabled(),
                                                &label,
                                            )
                                        });
                                        #[cfg(any(test, feature = "inspection"))]
                                        output.traces.push(Trace::capture(
                                            "remote.allowedHostsRemove",
                                            &remove,
                                        ));
                                        output.tooltips.push(trigger);
                                        if remove.clicked() {
                                            removed = Some(host.clone());
                                        }
                                        ui.with_layout(
                                            egui::Layout::left_to_right(Align::Center),
                                            |ui| {
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(host)
                                                            .font(FontId::monospace(TEXT_SIZE))
                                                            .color(appearance.foreground),
                                                    )
                                                    .truncate(),
                                                );
                                            },
                                        );
                                    },
                                );
                            });
                    });
                }
            });
        }
        let candidate = trim_text(&state.remote_host).to_lowercase();
        let is_duplicate =
            !candidate.is_empty() && hosts.iter().any(|host| host.to_lowercase() == candidate);
        let is_invalid = !candidate.is_empty() && !is_valid_allowed_host(&candidate);
        let can_add = !candidate.is_empty() && !is_duplicate && !is_invalid && !is_saving;
        if is_invalid {
            small(
                ui,
                message(locale, "remote.allowedHostsInvalid", &[]),
                appearance.sections.warning,
            );
        }
        let is_composing = ui.input(|input| state.remote_host_composition.observe(&input.events));
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), XS_BUTTON_HEIGHT),
            egui::Layout::right_to_left(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = INLINE_GAP;
                let add = xs_button(
                    ui,
                    &message(locale, "remote.allowedHostsAdd", &[]),
                    Variant::Outline,
                    can_add,
                    appearance,
                );
                let input = ui.add_sized(
                    [ui.available_width(), LINE_HEIGHT],
                    egui::TextEdit::singleline(&mut state.remote_host)
                        .id_salt("settings-remote-allowed-host")
                        .return_key(None)
                        .font(FontId::proportional(TEXT_SIZE))
                        .hint_text(message(locale, "remote.allowedHostsPlaceholder", &[]))
                        .margin(egui::Margin::symmetric(
                            code_view::INPUT_PADDING_X,
                            code_view::INPUT_PADDING_Y,
                        )),
                );
                #[cfg(any(test, feature = "inspection"))]
                {
                    output
                        .traces
                        .push(Trace::capture("remote.allowedHostsAdd", &add));
                    output
                        .traces
                        .push(Trace::capture("remote.allowedHostsPlaceholder", &input));
                }
                let is_submitted = input.has_focus()
                    && !is_composing
                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if (add.clicked() || is_submitted) && can_add {
                    state.remote_host.clear();
                    data.act(Action::UpdateSettings(HostedChange::RemoteAllowedHosts(
                        hosts.iter().cloned().chain([candidate.clone()]).collect(),
                    )));
                }
            },
        );
    });
    if let Some(host) = removed {
        data.act(Action::UpdateSettings(HostedChange::RemoteAllowedHosts(
            hosts
                .iter()
                .filter(|existing| **existing != host)
                .cloned()
                .collect(),
        )));
    }
}

#[cfg(test)]
#[path = "settings-sections-view-tests.rs"]
mod tests;

use taide_model::error::{AppError, AppResult};
use taide_model::settings::{Settings, SettingsPatch};

const RESIZER_MIN: u32 = 0;
const RESIZER_MAX: u32 = 8;
const SEARCH_DEBOUNCE_MIN: u32 = 50;
const SEARCH_DEBOUNCE_MAX: u32 = 2000;
const CODE_FONT_MIN: u32 = 6;
const CODE_FONT_MAX: u32 = 48;
const AUTO_SAVE_MIN: u32 = 0;
const AUTO_SAVE_MAX: u32 = 60_000;
const TAB_SIZE_MIN: u32 = 1;
const TAB_SIZE_MAX: u32 = 8;
const SCROLLBACK_MIN: u32 = 100;
const SCROLLBACK_MAX: u32 = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Appearance,
    Language,
    Interface,
    Notifications,
    Editor,
    Snippets,
    Terminal,
    Keymap,
}

impl Section {
    pub const BASIC: [Self; 8] = [
        Self::Appearance,
        Self::Language,
        Self::Interface,
        Self::Notifications,
        Self::Editor,
        Self::Snippets,
        Self::Terminal,
        Self::Keymap,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Appearance => "settings.appearance",
            Self::Language => "settings.language",
            Self::Interface => "settings.interface",
            Self::Notifications => "settings.notifications",
            Self::Editor => "settings.editor",
            Self::Snippets => "settings.snippetsSectionTitle",
            Self::Terminal => "settings.terminal",
            Self::Keymap => "settings.keymap",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    FollowSystemTheme,
    ShowSystemUsage,
    EditorMinimap,
    AgentStatusBadge,
    IdeAutoOpenDiff,
    EnablePreviewTabs,
    ExplorerAutoReveal,
    WelcomeOnEmptyEditor,
    ZenFullscreen,
    ZenHideStatusBar,
    SearchOnType,
    NotificationsEnabled,
    NotificationsOnlyWhenUnfocused,
    NotifyAgentCompleted,
    NotifyAgentAwaitingInput,
    NotifyTaskCompleted,
    NotifyGitRemote,
    NotifySearchReplace,
    NotifyLspInstall,
    NotifyError,
}

impl Switch {
    pub const ALL: [Self; 20] = [
        Self::FollowSystemTheme,
        Self::ShowSystemUsage,
        Self::EditorMinimap,
        Self::AgentStatusBadge,
        Self::IdeAutoOpenDiff,
        Self::EnablePreviewTabs,
        Self::ExplorerAutoReveal,
        Self::WelcomeOnEmptyEditor,
        Self::ZenFullscreen,
        Self::ZenHideStatusBar,
        Self::SearchOnType,
        Self::NotificationsEnabled,
        Self::NotificationsOnlyWhenUnfocused,
        Self::NotifyAgentCompleted,
        Self::NotifyAgentAwaitingInput,
        Self::NotifyTaskCompleted,
        Self::NotifyGitRemote,
        Self::NotifySearchReplace,
        Self::NotifyLspInstall,
        Self::NotifyError,
    ];

    pub fn section(self) -> Section {
        match self {
            Self::FollowSystemTheme => Section::Appearance,
            Self::ShowSystemUsage => Section::Interface,
            Self::EditorMinimap => Section::Interface,
            Self::AgentStatusBadge => Section::Interface,
            Self::IdeAutoOpenDiff => Section::Interface,
            Self::EnablePreviewTabs => Section::Interface,
            Self::ExplorerAutoReveal => Section::Interface,
            Self::WelcomeOnEmptyEditor => Section::Interface,
            Self::ZenFullscreen => Section::Interface,
            Self::ZenHideStatusBar => Section::Interface,
            Self::SearchOnType => Section::Interface,
            Self::NotificationsEnabled => Section::Notifications,
            Self::NotificationsOnlyWhenUnfocused => Section::Notifications,
            Self::NotifyAgentCompleted => Section::Notifications,
            Self::NotifyAgentAwaitingInput => Section::Notifications,
            Self::NotifyTaskCompleted => Section::Notifications,
            Self::NotifyGitRemote => Section::Notifications,
            Self::NotifySearchReplace => Section::Notifications,
            Self::NotifyLspInstall => Section::Notifications,
            Self::NotifyError => Section::Notifications,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::FollowSystemTheme => "settings.followSystemTheme",
            Self::ShowSystemUsage => "settings.showSystemUsage",
            Self::EditorMinimap => "settings.editorMinimap",
            Self::AgentStatusBadge => "settings.agentStatusBadge",
            Self::IdeAutoOpenDiff => "settings.ideAutoOpenDiff",
            Self::EnablePreviewTabs => "settings.enablePreviewTabs",
            Self::ExplorerAutoReveal => "settings.explorerAutoReveal",
            Self::WelcomeOnEmptyEditor => "settings.welcomeOnEmptyEditor",
            Self::ZenFullscreen => "settings.zenFullscreen",
            Self::ZenHideStatusBar => "settings.zenHideStatusBar",
            Self::SearchOnType => "settings.searchOnType",
            Self::NotificationsEnabled => "settings.notificationsEnabled",
            Self::NotificationsOnlyWhenUnfocused => "settings.notificationsOnlyWhenUnfocused",
            Self::NotifyAgentCompleted => "settings.notificationsAgentCompleted",
            Self::NotifyAgentAwaitingInput => "settings.notificationsAgentAwaitingInput",
            Self::NotifyTaskCompleted => "settings.notificationsTaskCompleted",
            Self::NotifyGitRemote => "settings.notificationsGitRemote",
            Self::NotifySearchReplace => "settings.notificationsSearchReplace",
            Self::NotifyLspInstall => "settings.notificationsLspInstall",
            Self::NotifyError => "settings.notificationsError",
        }
    }

    pub fn description(self) -> Option<&'static str> {
        match self {
            Self::EnablePreviewTabs => Some("settings.enablePreviewTabsHint"),
            Self::ExplorerAutoReveal => Some("settings.explorerAutoRevealDescription"),
            Self::WelcomeOnEmptyEditor => Some("settings.welcomeOnEmptyEditorDescription"),
            Self::ZenFullscreen => Some("settings.zenFullscreenDescription"),
            Self::ZenHideStatusBar => Some("settings.zenHideStatusBarDescription"),
            Self::SearchOnType => Some("settings.searchOnTypeDescription"),
            Self::NotificationsEnabled => Some("settings.notificationsEnabledDescription"),
            Self::NotificationsOnlyWhenUnfocused => {
                Some("settings.notificationsOnlyWhenUnfocusedDescription")
            }
            Self::NotifyAgentAwaitingInput => {
                Some("settings.notificationsAgentAwaitingInputDescription")
            }
            _ => None,
        }
    }

    pub fn value(self, settings: &Settings) -> bool {
        match self {
            Self::FollowSystemTheme => settings.follow_system_theme,
            Self::ShowSystemUsage => settings.show_system_usage,
            Self::EditorMinimap => settings.editor_minimap,
            Self::AgentStatusBadge => settings.agent_status_badge_enabled,
            Self::IdeAutoOpenDiff => settings.ide_auto_open_diff,
            Self::EnablePreviewTabs => settings.enable_preview_tabs,
            Self::ExplorerAutoReveal => settings.explorer_auto_reveal,
            Self::WelcomeOnEmptyEditor => settings.welcome_on_empty_editor,
            Self::ZenFullscreen => settings.zen_fullscreen,
            Self::ZenHideStatusBar => settings.zen_hide_status_bar,
            Self::SearchOnType => settings.search_on_type,
            Self::NotificationsEnabled => settings.notifications_enabled,
            Self::NotificationsOnlyWhenUnfocused => settings.notifications_only_when_unfocused,
            Self::NotifyAgentCompleted => settings.notify_agent_completed,
            Self::NotifyAgentAwaitingInput => settings.notify_agent_awaiting_input,
            Self::NotifyTaskCompleted => settings.notify_task_completed,
            Self::NotifyGitRemote => settings.notify_git_remote,
            Self::NotifySearchReplace => settings.notify_search_replace,
            Self::NotifyLspInstall => settings.notify_lsp_install,
            Self::NotifyError => settings.notify_error,
        }
    }

    fn patch(self, value: bool) -> SettingsPatch {
        match self {
            Self::FollowSystemTheme => SettingsPatch {
                follow_system_theme: Some(value),
                ..Default::default()
            },
            Self::ShowSystemUsage => SettingsPatch {
                show_system_usage: Some(value),
                ..Default::default()
            },
            Self::EditorMinimap => SettingsPatch {
                editor_minimap: Some(value),
                ..Default::default()
            },
            Self::AgentStatusBadge => SettingsPatch {
                agent_status_badge_enabled: Some(value),
                ..Default::default()
            },
            Self::IdeAutoOpenDiff => SettingsPatch {
                ide_auto_open_diff: Some(value),
                ..Default::default()
            },
            Self::EnablePreviewTabs => SettingsPatch {
                enable_preview_tabs: Some(value),
                ..Default::default()
            },
            Self::ExplorerAutoReveal => SettingsPatch {
                explorer_auto_reveal: Some(value),
                ..Default::default()
            },
            Self::WelcomeOnEmptyEditor => SettingsPatch {
                welcome_on_empty_editor: Some(value),
                ..Default::default()
            },
            Self::ZenFullscreen => SettingsPatch {
                zen_fullscreen: Some(value),
                ..Default::default()
            },
            Self::ZenHideStatusBar => SettingsPatch {
                zen_hide_status_bar: Some(value),
                ..Default::default()
            },
            Self::SearchOnType => SettingsPatch {
                search_on_type: Some(value),
                ..Default::default()
            },
            Self::NotificationsEnabled => SettingsPatch {
                notifications_enabled: Some(value),
                ..Default::default()
            },
            Self::NotificationsOnlyWhenUnfocused => SettingsPatch {
                notifications_only_when_unfocused: Some(value),
                ..Default::default()
            },
            Self::NotifyAgentCompleted => SettingsPatch {
                notify_agent_completed: Some(value),
                ..Default::default()
            },
            Self::NotifyAgentAwaitingInput => SettingsPatch {
                notify_agent_awaiting_input: Some(value),
                ..Default::default()
            },
            Self::NotifyTaskCompleted => SettingsPatch {
                notify_task_completed: Some(value),
                ..Default::default()
            },
            Self::NotifyGitRemote => SettingsPatch {
                notify_git_remote: Some(value),
                ..Default::default()
            },
            Self::NotifySearchReplace => SettingsPatch {
                notify_search_replace: Some(value),
                ..Default::default()
            },
            Self::NotifyLspInstall => SettingsPatch {
                notify_lsp_install: Some(value),
                ..Default::default()
            },
            Self::NotifyError => SettingsPatch {
                notify_error: Some(value),
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    value: &'static str,
    label: &'static str,
}

impl Position {
    pub const ALL: [Self; 9] = [
        Self {
            value: "top-left",
            label: "settings.positionTopLeft",
        },
        Self {
            value: "top-center",
            label: "settings.positionTopCenter",
        },
        Self {
            value: "top-right",
            label: "settings.positionTopRight",
        },
        Self {
            value: "middle-left",
            label: "settings.positionMiddleLeft",
        },
        Self {
            value: "middle-center",
            label: "settings.positionMiddleCenter",
        },
        Self {
            value: "middle-right",
            label: "settings.positionMiddleRight",
        },
        Self {
            value: "bottom-left",
            label: "settings.positionBottomLeft",
        },
        Self {
            value: "bottom-center",
            label: "settings.positionBottomCenter",
        },
        Self {
            value: "bottom-right",
            label: "settings.positionBottomRight",
        },
    ];

    pub fn value(self) -> &'static str {
        self.value
    }

    pub fn label(self) -> &'static str {
        self.label
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numeric {
    ResizerThickness,
    SearchOnTypeDebounce,
    EditorFontSize,
    TerminalFontSize,
    AutoSaveDelay,
    EditorTabSize,
    TerminalScrollback,
}

impl Numeric {
    pub fn bounds(self) -> (u32, u32) {
        match self {
            Self::ResizerThickness => (RESIZER_MIN, RESIZER_MAX),
            Self::SearchOnTypeDebounce => (SEARCH_DEBOUNCE_MIN, SEARCH_DEBOUNCE_MAX),
            Self::EditorFontSize | Self::TerminalFontSize => (CODE_FONT_MIN, CODE_FONT_MAX),
            Self::AutoSaveDelay => (AUTO_SAVE_MIN, AUTO_SAVE_MAX),
            Self::EditorTabSize => (TAB_SIZE_MIN, TAB_SIZE_MAX),
            Self::TerminalScrollback => (SCROLLBACK_MIN, SCROLLBACK_MAX),
        }
    }

    pub fn value(self, settings: &Settings) -> u32 {
        match self {
            Self::ResizerThickness => settings.resizer_thickness,
            Self::SearchOnTypeDebounce => settings.search_on_type_debounce_ms,
            Self::EditorFontSize => settings.editor_font_size,
            Self::TerminalFontSize => settings.terminal_font_size,
            Self::AutoSaveDelay => settings.auto_save_delay_ms,
            Self::EditorTabSize => settings.editor_tab_size,
            Self::TerminalScrollback => settings.terminal_scrollback,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ResizerThickness => "settings.resizerThickness",
            Self::SearchOnTypeDebounce => "settings.searchOnTypeDebounceMs",
            Self::EditorFontSize => "settings.editorFontSize",
            Self::TerminalFontSize => "settings.terminalFontSize",
            Self::AutoSaveDelay => "settings.autoSaveDelayMs",
            Self::EditorTabSize => "settings.editorTabSize",
            Self::TerminalScrollback => "settings.terminalScrollback",
        }
    }

    pub fn commit(self, raw: f64, committed: u32) -> AppResult<Option<Change>> {
        if raw.is_nan() {
            return Ok(None);
        }
        let (min, max) = self.bounds();
        let value = raw.clamp(f64::from(min), f64::from(max));
        if value == f64::from(committed) {
            return Ok(None);
        }
        if value.fract() != 0.0 {
            return Err(AppError::InvalidArgument(
                "numeric setting must be an integer".into(),
            ));
        }
        Ok(Some(Change::Numeric(self, value as u32)))
    }

    fn patch(self, value: u32) -> SettingsPatch {
        match self {
            Self::ResizerThickness => SettingsPatch {
                resizer_thickness: Some(value),
                ..Default::default()
            },
            Self::SearchOnTypeDebounce => SettingsPatch {
                search_on_type_debounce_ms: Some(value),
                ..Default::default()
            },
            Self::EditorFontSize => SettingsPatch {
                editor_font_size: Some(value),
                ..Default::default()
            },
            Self::TerminalFontSize => SettingsPatch {
                terminal_font_size: Some(value),
                ..Default::default()
            },
            Self::AutoSaveDelay => SettingsPatch {
                auto_save_delay_ms: Some(value),
                ..Default::default()
            },
            Self::EditorTabSize => SettingsPatch {
                editor_tab_size: Some(value),
                ..Default::default()
            },
            Self::TerminalScrollback => SettingsPatch {
                terminal_scrollback: Some(value),
                ..Default::default()
            },
        }
    }
}

pub struct NumericDraft {
    committed: u32,
    pub text: String,
}

impl NumericDraft {
    pub fn new(value: u32) -> Self {
        Self {
            committed: value,
            text: value.to_string(),
        }
    }

    pub fn sync(&mut self, value: u32) {
        if self.committed != value {
            self.committed = value;
            self.text = value.to_string();
        }
    }

    pub fn commit(&mut self, field: Numeric, value: u32) -> AppResult<Option<Change>> {
        let raw = html_number(&self.text).unwrap_or(f64::NAN);
        self.committed = value;
        self.text = value.to_string();
        field.commit(raw, value)
    }
}

fn html_number(text: &str) -> Option<f64> {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let mantissa = if let Some((mantissa, exponent)) = unsigned.split_once(['e', 'E']) {
        let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        mantissa
    } else {
        unsigned
    };
    let integer = if let Some((integer, fraction)) = mantissa.split_once('.') {
        if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        integer
    } else {
        if mantissa.is_empty() {
            return None;
        }
        mantissa
    };
    if !integer.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Theme(String),
    Language(String),
    Switch(Switch, bool),
    Position(Position),
    Numeric(Numeric, u32),
    Code(crate::settings_code_controls::Change),
    KeymapOverrides(String),
}

impl Change {
    pub fn patch(&self) -> Option<SettingsPatch> {
        match self {
            Self::Theme(_) => None,
            Self::Language(value) => Some(SettingsPatch {
                language: Some(value.clone()),
                ..Default::default()
            }),
            Self::Switch(field, value) => Some(field.patch(*value)),
            Self::Position(value) => Some(SettingsPatch {
                toast_position: Some(value.value().into()),
                ..Default::default()
            }),
            Self::Numeric(field, value) => Some(field.patch(*value)),
            Self::Code(change) => Some(change.patch()),
            Self::KeymapOverrides(value) => Some(SettingsPatch {
                keymap_overrides: Some(value.clone()),
                ..Default::default()
            }),
        }
    }
}

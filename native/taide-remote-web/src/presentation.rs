use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::json;
use taide_model::error::AppResult;
use taide_model::locale::ResolvedLocale;
use taide_model::settings::Settings;
use taide_model::theme::ResolvedTheme;
use taide_native_ui::conflict_banner::BannerAppearance;
use taide_native_ui::editor_surface::EditorAppearance;
use taide_native_ui::shell::ShellColors;

use crate::shell::{Call, Failure};
use crate::{InvokeError, ResponsePayload};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInputs {
    pub theme: String,
    pub language: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Read {
    Theme,
    Locale,
}

impl Read {
    pub fn call(self, inputs: &SystemInputs) -> Call {
        let (command, args) = match self {
            Self::Theme => ("theme_get_current", json!({"systemTheme": inputs.theme})),
            Self::Locale => (
                "locale_get_current",
                json!({"systemLanguage": inputs.language}),
            ),
        };
        Call { command, args }
    }
}

struct Preferences {
    theme_id: String,
    follow_system_theme: bool,
    language: String,
}

#[derive(Deserialize)]
struct SettingsEvent {
    settings: Settings,
}

pub struct PresentationState {
    inputs: SystemInputs,
    preferences: Option<Preferences>,
    theme: Option<ResolvedTheme>,
    locale: Option<ResolvedLocale>,
    pending: BTreeMap<u32, Read>,
    dirty: BTreeSet<Read>,
    failures: BTreeMap<Read, Failure>,
}

impl PresentationState {
    pub fn new(inputs: SystemInputs) -> Self {
        Self {
            inputs,
            preferences: None,
            theme: None,
            locale: None,
            pending: BTreeMap::new(),
            dirty: BTreeSet::new(),
            failures: BTreeMap::new(),
        }
    }

    pub fn theme(&self) -> Option<&ResolvedTheme> {
        self.theme.as_ref()
    }

    pub fn locale(&self) -> Option<&ResolvedLocale> {
        self.locale.as_ref()
    }

    pub fn shell_colors(&self) -> Option<AppResult<ShellColors>> {
        self.theme
            .as_ref()
            .map(taide_native_ui::presentation::shell_colors)
    }

    pub fn editor_appearance(&self, settings: &Settings) -> Option<AppResult<EditorAppearance>> {
        self.theme
            .as_ref()
            .map(|theme| taide_native_ui::presentation::editor_appearance(theme, settings))
    }

    pub fn banner_appearance(&self) -> Option<AppResult<BannerAppearance>> {
        self.theme
            .as_ref()
            .map(taide_native_ui::presentation::banner_appearance)
    }

    pub fn message(&self, key: &str, values: &[(&str, &str)]) -> Option<String> {
        self.locale
            .as_ref()
            .map(|locale| taide_native_ui::presentation::message(locale, key, values))
    }

    pub fn failures(&self) -> &BTreeMap<Read, Failure> {
        &self.failures
    }

    pub fn inputs(&self) -> &SystemInputs {
        &self.inputs
    }

    pub fn is_refreshing(&self) -> bool {
        !self.pending.is_empty() || !self.dirty.is_empty()
    }

    pub fn set_inputs(&mut self, inputs: SystemInputs) {
        if self.inputs.theme != inputs.theme {
            self.dirty.insert(Read::Theme);
        }
        if self.inputs.language != inputs.language {
            self.dirty.insert(Read::Locale);
        }
        self.inputs = inputs;
    }

    pub fn settings(&mut self, settings: &Settings) {
        let theme_changed = self.preferences.as_ref().is_none_or(|previous| {
            previous.theme_id != settings.theme_id
                || previous.follow_system_theme != settings.follow_system_theme
        });
        let locale_changed = self
            .preferences
            .as_ref()
            .is_none_or(|previous| previous.language != settings.language);
        if theme_changed {
            self.dirty.insert(Read::Theme);
        }
        if locale_changed {
            self.dirty.insert(Read::Locale);
        }
        if theme_changed || locale_changed {
            self.preferences = Some(Preferences {
                theme_id: settings.theme_id.clone(),
                follow_system_theme: settings.follow_system_theme,
                language: settings.language.clone(),
            });
        }
    }

    pub fn event(&mut self, name: &str, payload: &str) {
        match name {
            "theme:changed" => {
                self.dirty.insert(Read::Theme);
            }
            "settings:changed" => {
                if let Ok(event) = serde_json::from_str::<SettingsEvent>(payload) {
                    self.settings(&event.settings);
                }
            }
            _ => {}
        }
    }

    pub fn refresh(&mut self) {
        self.dirty.extend([Read::Theme, Read::Locale]);
    }

    pub fn retry(&mut self, read: Read) {
        self.dirty.insert(read);
    }

    pub fn disconnected(&mut self) {
        self.pending.clear();
        self.dirty.clear();
    }

    pub fn next_reads(&self) -> Vec<Read> {
        self.dirty
            .iter()
            .filter(|read| !self.pending.values().any(|pending| pending == *read))
            .copied()
            .collect()
    }

    pub fn sent(&mut self, read: Read, seq: u32) {
        self.dirty.remove(&read);
        self.failures.remove(&read);
        self.pending.insert(seq, read);
    }

    pub fn invocation_failed(&mut self, read: Read, error: InvokeError) {
        self.dirty.remove(&read);
        self.failures.insert(read, Failure::Invocation(error));
    }

    pub fn response(
        &mut self,
        seq: u32,
        result: &Result<ResponsePayload, serde_json::Value>,
    ) -> bool {
        let Some(read) = self.pending.remove(&seq) else {
            return false;
        };
        if self.dirty.contains(&read) {
            return true;
        }
        let applied = match result {
            Ok(ResponsePayload::Json(value)) => match read {
                Read::Theme => serde_json::from_value(value.clone())
                    .map(|theme| self.theme = Some(theme))
                    .map_err(|_| Failure::MalformedResponse),
                Read::Locale => serde_json::from_value(value.clone())
                    .map(|locale| self.locale = Some(locale))
                    .map_err(|_| Failure::MalformedResponse),
            },
            Ok(ResponsePayload::Binary(_)) => Err(Failure::MalformedResponse),
            Err(error) => Err(Failure::Remote(error.clone())),
        };
        match applied {
            Ok(()) => {
                self.failures.remove(&read);
            }
            Err(error) => {
                self.failures.insert(read, error);
            }
        }
        true
    }
}

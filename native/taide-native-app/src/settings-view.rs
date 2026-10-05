pub use taide_native_ui::settings_view::*;

#[cfg(test)]
use crate::{
    presentation::{color, message},
    settings_controls::{Change, Numeric, Position, Section, Switch},
    theme_draft::Mode,
    theme_editor::Editor,
};
#[cfg(test)]
use eframe::egui::{self, Color32, Rect};
#[cfg(test)]
use taide_model::{
    error::AppError,
    ids::{PaneId, ProjectId, TabId},
    layout::{PaneNode, TabKind},
    locale::{LocaleSummary, ResolvedLocale},
    settings::Settings,
    theme::{ThemeSummary, ThemeType},
};
#[cfg(test)]
use taide_runtime::{AppState, locale_actions, theme_actions};

#[cfg(test)]
#[path = "settings-view-tests.rs"]
mod tests;
#[cfg(test)]
#[path = "settings-tooltip-tests.rs"]
mod tooltip_tests;

use taide_model::error::AppResult;
use taide_model::theme::{ResolvedTheme, Theme, ThemeSummary};
use taide_theme::service;

use crate::AppState;

/// Lists themes using the shared builtin and user theme scan.
pub fn theme_list(state: &AppState) -> AppResult<Vec<ThemeSummary>> {
    Ok(service::list_themes(&state.paths))
}

/// Loads a theme using the shared identifier validation and resolution policy.
pub fn theme_get(state: &AppState, theme_id: String) -> AppResult<ResolvedTheme> {
    service::load_theme(&state.paths, &theme_id)
}

/// Saves a user theme using the shared builtin protection and persistence policy.
pub fn theme_save(state: &AppState, theme: Theme) -> AppResult<ThemeSummary> {
    service::save_theme(&state.paths, &theme)
}

/// Deletes a user theme using the shared identifier validation and builtin protection.
pub fn theme_delete(state: &AppState, theme_id: String) -> AppResult<()> {
    service::delete_theme(&state.paths, &theme_id)
}

pub fn theme_get_current(state: &AppState, system_theme: &str) -> AppResult<ResolvedTheme> {
    let theme_id = {
        let settings = state.settings.read();
        if settings.follow_system_theme {
            service::builtin_id_for_system(system_theme).to_string()
        } else {
            settings.theme_id.clone()
        }
    };
    service::load_theme(&state.paths, &theme_id)
}

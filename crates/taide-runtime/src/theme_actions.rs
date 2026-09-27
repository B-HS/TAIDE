use taide_model::error::AppResult;
use taide_model::theme::ResolvedTheme;
use taide_theme::service;

use crate::AppState;

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

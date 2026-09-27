use taide_locale::service;
use taide_model::error::AppResult;
use taide_model::locale::ResolvedLocale;

use crate::AppState;

pub fn locale_get_current(state: &AppState, system_language: &str) -> AppResult<ResolvedLocale> {
    let language = state.settings.read().language.clone();
    let locale_id = service::resolve_language(&state.paths, &language, system_language);
    service::load_locale(&state.paths, &locale_id)
}

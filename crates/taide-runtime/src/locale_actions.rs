use taide_locale::service;
use taide_model::error::AppResult;
use taide_model::locale::{LocaleSummary, ResolvedLocale};

use crate::AppState;

/// Lists locale packs using the shared builtin and user pack scan.
pub fn locale_list(state: &AppState) -> AppResult<Vec<LocaleSummary>> {
    Ok(service::list_locales(&state.paths))
}

/// Loads a locale pack using the existing shared resolution and error policy.
pub fn locale_get(state: &AppState, locale_id: String) -> AppResult<ResolvedLocale> {
    service::load_locale(&state.paths, &locale_id)
}

pub fn locale_get_current(state: &AppState, system_language: &str) -> AppResult<ResolvedLocale> {
    let language = state.settings.read().language.clone();
    locale_get_for_language(state, &language, system_language)
}

pub fn locale_get_for_language(state: &AppState, language: &str, system_language: &str) -> AppResult<ResolvedLocale> {
    let locale_id = service::resolve_language(&state.paths, language, system_language);
    service::load_locale(&state.paths, &locale_id)
}

use futures_util::future::BoxFuture;
use taide_model::settings::Settings;
use tauri::AppHandle;

use crate::error::AppResult;
use crate::state::AppState;

pub type SettingsApply = for<'a> fn(&'a AppHandle, &'a AppState, Settings) -> BoxFuture<'a, AppResult<Settings>>;

pub struct SettingsApplyPort(pub SettingsApply);

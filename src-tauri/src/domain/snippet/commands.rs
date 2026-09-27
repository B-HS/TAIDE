use taide_runtime::snippet_actions;

use super::types::SnippetFile;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn snippet_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<SnippetFile>> {
    snippet_actions::snippet_list(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_save(state: tauri::State<'_, AppState>, file_name: String, content: String) -> AppResult<SnippetFile> {
    snippet_actions::snippet_save(&state, file_name, content)
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_delete(state: tauri::State<'_, AppState>, file_name: String) -> AppResult<()> {
    snippet_actions::snippet_delete(&state, file_name)
}

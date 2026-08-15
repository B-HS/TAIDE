use super::service;
use super::types::SnippetFile;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn snippet_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<SnippetFile>> {
    Ok(service::list_snippet_files(&state.paths))
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_save(state: tauri::State<'_, AppState>, file_name: String, content: String) -> AppResult<SnippetFile> {
    service::save_snippet_file(&state.paths, &file_name, &content)
}

#[tauri::command]
#[specta::specta]
pub async fn snippet_delete(state: tauri::State<'_, AppState>, file_name: String) -> AppResult<()> {
    service::delete_snippet_file(&state.paths, &file_name)
}

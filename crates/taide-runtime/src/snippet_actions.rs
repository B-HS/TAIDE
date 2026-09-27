use taide_model::error::AppResult;
use taide_model::snippet::SnippetFile;
use taide_snippet::service;

use crate::AppState;

/// Lists snippet files using the shared tolerant scan and filename ordering policy.
pub fn snippet_list(state: &AppState) -> AppResult<Vec<SnippetFile>> {
    Ok(service::list_snippet_files(&state.paths))
}

/// Validates and atomically saves snippet content without changing its original formatting.
pub fn snippet_save(state: &AppState, file_name: String, content: String) -> AppResult<SnippetFile> {
    service::save_snippet_file(&state.paths, &file_name, &content)
}

/// Validates the snippet filename and deletes the corresponding shared data file.
pub fn snippet_delete(state: &AppState, file_name: String) -> AppResult<()> {
    service::delete_snippet_file(&state.paths, &file_name)
}

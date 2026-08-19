use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::service;
use super::types::{SearchMatch, SearchQuery, SearchReplaceResult};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::state::AppState;

/// Keyed by `(owner, session_id)` — a caller-supplied session id (one per
/// search surface — the quick search panel or a single Search Editor tab)
/// combined with the calling window's `owner` label
/// (`getCurrentWindow().label` on the frontend — `main`/`editor-<n>`, or the
/// remote client's fixed `domain::remote::types::REMOTE_OWNER_LABEL`), not by
/// [`ProjectId`]. Two surfaces searching the same project run independently;
/// only a second run from the *same* (owner, session_id) pair supersedes its
/// own prior run. See
/// `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4.
///
/// `owner` joined the key in R7#8: `session_id` alone was the frontend's
/// `useId()`, which is only unique *within one React realm* — a second
/// window opens its own realm and can generate the exact same id sequence as
/// the first, so a bare `session_id` key let a second window's search
/// silently cancel the main window's still-running search of the same id.
/// `domain::lsp::commands::lsp_stop`'s `owner` scoping is the precedent this
/// mirrors.
#[derive(Default)]
pub struct SearchStore(Mutex<HashMap<(String, String), Arc<AtomicBool>>>);

impl SearchStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<PathBuf> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

fn begin_search(store: &SearchStore, owner: &str, session_id: &str) -> Arc<AtomicBool> {
    let key = (owner.to_string(), session_id.to_string());
    let mut sessions = store.0.lock();
    if let Some(previous) = sessions.get(&key) {
        previous.store(true, Ordering::SeqCst);
    }

    let cancelled = Arc::new(AtomicBool::new(false));
    sessions.insert(key, cancelled.clone());
    cancelled
}

/// Removes `(owner, session_id)`'s entry once its search finishes — but only if no
/// newer run for that same (owner, session_id) pair has already replaced it (compared by
/// `Arc` identity), so a slow-finishing cancelled run can never clobber a
/// fresh one. Keeps the store bounded to in-flight/most-recent searches
/// instead of growing forever as Search Editor tabs open and close.
fn end_search(store: &SearchStore, owner: &str, session_id: &str, own_flag: &Arc<AtomicBool>) {
    let key = (owner.to_string(), session_id.to_string());
    let mut sessions = store.0.lock();
    if sessions.get(&key).is_some_and(|current| Arc::ptr_eq(current, own_flag)) {
        sessions.remove(&key);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn search_run(
    state: State<'_, AppState>,
    store: State<'_, SearchStore>,
    project_id: ProjectId,
    owner: String,
    session_id: String,
    query: SearchQuery,
    on_match: Channel<SearchMatch>,
) -> AppResult<u32> {
    let root = project_root(&state, &project_id)?;

    let cancelled = {
        let _guard = state.begin_mutation().await;
        begin_search(&store, &owner, &session_id)
    };

    let join_result = tokio::task::spawn_blocking({
        let cancelled = cancelled.clone();
        move || {
            service::search(&root, &query, &cancelled, move |item| {
                let _ = on_match.send(item);
            })
        }
    })
    .await
    .map_err(|error| AppError::Internal(format!("search task failed: {error}")))?;

    end_search(&store, &owner, &session_id, &cancelled);

    join_result
}

/// Reacquires `AppState::begin_mutation`'s single global lock **once per file** instead of holding
/// it for the whole multi-file replace — a project-wide "replace all" can touch hundreds of files,
/// and that one lock is shared by every other mutating command (`file_save`, `git_push`, layout
/// writes, ...), so holding it for the entire walk-and-rewrite would starve all of them for as long
/// as the replace runs. Reacquiring per file keeps each hold short while still serializing every
/// actual write against the rest of the app's mutations. Both the target-file resolution (the tree
/// walk) and each file's guarded read-modify-write run inside `spawn_blocking` — none of it runs on
/// the async worker thread — since `service::replace_one_file` does synchronous filesystem I/O.
#[tauri::command]
#[specta::specta]
pub async fn search_replace(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    query: SearchQuery,
    replacement: String,
    paths: Option<Vec<String>>,
) -> AppResult<SearchReplaceResult> {
    let root = project_root(&state, &project_id)?;

    if query.text.is_empty() {
        return Ok(SearchReplaceResult {
            changed_files: 0,
            replaced_matches: 0,
        });
    }

    let compiled_regex = service::compile_optional_regex(&query)?;
    let target_paths = paths.map(|list| list.into_iter().map(PathBuf::from).collect::<Vec<_>>());

    let target_files = {
        let scan_root = root.clone();
        let scan_query = query.clone();
        tokio::task::spawn_blocking(move || service::resolve_replace_targets(&scan_root, &scan_query, target_paths.as_deref()))
            .await
            .map_err(|error| AppError::Internal(format!("replace scan task failed: {error}")))?
    };

    let mut changed_files = 0u32;
    let mut replaced_matches = 0u32;

    for path in &target_files {
        let app = app.clone();
        let path = path.clone();
        let query = query.clone();
        let replacement = replacement.clone();
        let compiled_regex = compiled_regex.clone();

        let count = tokio::task::spawn_blocking(move || {
            let app_state = app.state::<AppState>();
            let _guard = app_state.begin_mutation_blocking();
            service::replace_one_file(&path, &query, compiled_regex.as_ref(), &replacement)
        })
        .await
        .map_err(|error| AppError::Internal(format!("replace task failed: {error}")))?;

        if let Some(count) = count {
            changed_files += 1;
            replaced_matches += count;
        }
    }

    Ok(SearchReplaceResult {
        changed_files,
        replaced_matches,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn search_cancel(state: State<'_, AppState>, store: State<'_, SearchStore>, owner: String, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    if let Some(cancelled) = store.0.lock().get(&(owner, session_id)) {
        cancelled.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 서로_다른_세션은_서로의_검색을_취소하지_않는다() {
        let store = SearchStore::new();

        let panel_flag = begin_search(&store, "main", "panel");
        let editor_flag = begin_search(&store, "main", "search-editor:tab-1");

        assert!(
            !panel_flag.load(Ordering::SeqCst),
            "다른 세션(search-editor)이 panel 세션을 취소하면 안 된다"
        );
        assert!(!editor_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn 같은_세션의_재검색은_이전_실행을_취소한다() {
        let store = SearchStore::new();

        let first = begin_search(&store, "main", "panel");
        let second = begin_search(&store, "main", "panel");

        assert!(first.load(Ordering::SeqCst), "같은 세션의 재검색은 이전 실행을 취소해야 한다");
        assert!(!second.load(Ordering::SeqCst));
    }

    #[test]
    fn 검색이_끝나면_스토어에서_세션_항목이_제거된다() {
        let store = SearchStore::new();
        let flag = begin_search(&store, "main", "panel");

        end_search(&store, "main", "panel", &flag);

        assert!(store.0.lock().get(&("main".to_string(), "panel".to_string())).is_none());
    }

    #[test]
    fn 이미_새_실행으로_대체된_세션_항목은_종료_시_제거되지_않는다() {
        let store = SearchStore::new();
        let stale = begin_search(&store, "main", "panel");
        let fresh = begin_search(&store, "main", "panel");

        end_search(&store, "main", "panel", &stale);

        let current = store.0.lock().get(&("main".to_string(), "panel".to_string())).cloned();
        assert!(
            current.is_some_and(|entry| Arc::ptr_eq(&entry, &fresh)),
            "새 실행의 항목이 유지되어야 한다"
        );
    }

    #[test]
    fn 검색_취소는_해당_세션의_플래그만_설정한다() {
        let store = SearchStore::new();
        let panel_flag = begin_search(&store, "main", "panel");
        let editor_flag = begin_search(&store, "main", "search-editor:tab-1");

        if let Some(cancelled) = store.0.lock().get(&("main".to_string(), "panel".to_string())) {
            cancelled.store(true, Ordering::SeqCst);
        }

        assert!(panel_flag.load(Ordering::SeqCst));
        assert!(!editor_flag.load(Ordering::SeqCst));
    }

    /// R7#8 회귀: 프론트의 `sessionId`는 `useId()`(React 렐름 로컬)로 만들어지므로, 서로 다른
    /// 창(webview)이 우연히 같은 문자열을 생성할 수 있다. `owner`가 키에 없으면 두 번째 창의
    /// `begin_search`가 첫 번째 창(메인 창)의 진행 중인 검색을 취소해버린다 —
    /// `domain::lsp::commands::lsp_stop`의 owner 스코프 선례를 그대로 따른다.
    #[test]
    fn 서로_다른_창의_같은_session_id는_서로의_검색을_취소하지_않는다() {
        let store = SearchStore::new();

        let main_flag = begin_search(&store, "main", "search-panel-:r0:");
        let second_window_flag = begin_search(&store, "editor-2", "search-panel-:r0:");

        assert!(
            !main_flag.load(Ordering::SeqCst),
            "두 번째 창(editor-2)이 같은 session_id로 검색을 시작해도 메인 창의 검색을 취소하면 안 된다"
        );
        assert!(!second_window_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn 서로_다른_창의_같은_session_id_취소는_해당_창의_플래그만_설정한다() {
        let store = SearchStore::new();
        let main_flag = begin_search(&store, "main", "search-panel-:r0:");
        let second_window_flag = begin_search(&store, "editor-2", "search-panel-:r0:");

        if let Some(cancelled) = store.0.lock().get(&("editor-2".to_string(), "search-panel-:r0:".to_string())) {
            cancelled.store(true, Ordering::SeqCst);
        }

        assert!(second_window_flag.load(Ordering::SeqCst));
        assert!(
            !main_flag.load(Ordering::SeqCst),
            "editor-2 창의 취소가 같은 session_id를 쓰는 main 창의 검색을 취소하면 안 된다"
        );
    }
}

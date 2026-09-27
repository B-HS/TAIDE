use std::path::Path;

use taide_file::service::{self, clear_mirror, MirrorEntry, UntitledMirrorEntry};
use taide_infra::language::LanguageOverlay;
use taide_infra::perf::{self, SpanSlot};
use taide_infra::{persist, root_guard};
use taide_model::error::{AppError, AppResult};
use taide_model::file::OpenedFile;
use taide_model::ids::{ProjectId, TabId};

use super::AppState;

#[derive(Clone)]
pub struct IdeSaveFile(pub fn(&AppState, &Path, &str) -> AppResult<()>);

/// Resolves a save against open projects or a CLI-opened path, writes it atomically while
/// preserving file mode, marks the self-write, and clears the owning project's hot-exit mirror.
pub fn save_file_within_open_projects(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    let projects = state.projects.read().clone();
    let (project_id, resolved) = root_guard::resolve_owning_project_or_cli_opened(&projects, &state.cli_opened_paths.read(), path)?;

    persist::write_atomic_preserving_mode(&resolved, content.as_bytes())?;
    state.self_writes.mark(&resolved);
    match project_id {
        Some(project_id) => clear_mirror(&state.paths, &project_id, &resolved),
        None => Ok(()),
    }
}

/// Validates access before loading language overlays and opens the file on a blocking worker.
pub async fn file_open(
    state: &AppState,
    path: String,
    load_language_overlays: impl FnOnce() -> Vec<LanguageOverlay>,
) -> AppResult<OpenedFile> {
    let _span = perf::span(SpanSlot::FileOpen);
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project_or_cli_opened(&projects, &state.cli_opened_paths.read(), Path::new(&path))?;
    let editor_config_enabled = state.settings.read().editor_config_enabled;
    let language_overlays = load_language_overlays();
    tokio::task::spawn_blocking(move || service::open_file(&resolved, &language_overlays, editor_config_enabled))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Holds the mutation guard across a blocking atomic save using the shared application state.
pub async fn file_save(state: &AppState, path: String, content: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let shared_state = state.clone();
    tokio::task::spawn_blocking(move || save_file_within_open_projects(&shared_state, Path::new(&path), &content))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Creates a project entry under the mutation guard and marks the successful self-write.
pub async fn file_create(state: &AppState, path: String, is_dir: bool) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;
    service::create_entry(&resolved, is_dir)?;
    state.self_writes.mark(&resolved);
    Ok(())
}

/// Validates both entry paths, renames under the mutation guard, and marks both self-writes.
pub async fn file_rename(state: &AppState, from: String, to: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved_from) = root_guard::resolve_entry_owning_project(&projects, Path::new(&from))?;
    let (_, destination) = root_guard::resolve_entry_owning_project(&projects, Path::new(&to))?;
    service::rename_entry(&resolved_from, &destination)?;
    state.self_writes.mark(&resolved_from);
    state.self_writes.mark(&destination);
    Ok(())
}

/// Deletes an authorized project entry under the mutation guard and marks the self-write.
pub async fn file_delete(state: &AppState, path: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_entry_owning_project(&projects, Path::new(&path))?;
    service::delete_entry(&resolved)?;
    state.self_writes.mark(&resolved);
    Ok(())
}

/// Holds the mutation guard across an authorized blocking copy and marks its destination.
pub async fn file_copy(state: &AppState, from: String, to: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved_from) = root_guard::resolve_owning_project(&projects, Path::new(&from))?;
    let (_, resolved_to) = root_guard::resolve_owning_project(&projects, Path::new(&to))?;
    let copy_to = resolved_to.clone();
    tokio::task::spawn_blocking(move || service::copy_entry(&resolved_from, &copy_to))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;
    state.self_writes.mark(&resolved_to);
    Ok(())
}

/// Writes an authorized dirty mirror on a blocking worker without taking the mutation guard.
pub async fn file_mirror_dirty(state: &AppState, project_id: ProjectId, path: String, content: String) -> AppResult<Option<f64>> {
    let projects = state.projects.read().clone();
    let root = root_guard::project_root(&projects, &project_id)?;
    let resolved = root_guard::ensure_within_root(&root, Path::new(&path))?;
    let shared_state = state.clone();
    tokio::task::spawn_blocking(move || service::mirror_dirty(&shared_state.paths, &project_id, &resolved, &path, &content))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Lists mirrors only for an open project.
pub async fn file_list_mirrors(state: &AppState, project_id: ProjectId) -> AppResult<Vec<MirrorEntry>> {
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    service::list_mirrors(&state.paths, &project_id)
}

/// Clears an authorized mirror under the mutation guard.
pub async fn file_clear_mirror(state: &AppState, project_id: ProjectId, path: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let root = root_guard::project_root(&projects, &project_id)?;
    let resolved = root_guard::ensure_within_root(&root, Path::new(&path))?;
    service::clear_mirror(&state.paths, &project_id, &resolved)
}

/// Prunes an open project's mirrors under the mutation guard using their existing display paths.
pub async fn file_prune_mirrors(state: &AppState, project_id: ProjectId, keep_paths: Vec<String>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    service::prune_mirrors(&state.paths, &project_id, &keep_paths)
}

/// Validates the project and tab component before writing an untitled mirror without the mutation guard.
pub async fn file_mirror_untitled(state: &AppState, project_id: ProjectId, tab_id: TabId, content: String) -> AppResult<()> {
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    root_guard::ensure_safe_component(tab_id.as_str())?;
    service::mirror_untitled(&state.paths, &project_id, &tab_id, &content)
}

/// Lists untitled mirrors only for an open project.
pub async fn file_list_untitled_mirrors(state: &AppState, project_id: ProjectId) -> AppResult<Vec<UntitledMirrorEntry>> {
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    service::list_untitled_mirrors(&state.paths, &project_id)
}

/// Validates the project and tab component before clearing an untitled mirror under the mutation guard.
pub async fn file_clear_untitled_mirror(state: &AppState, project_id: ProjectId, tab_id: TabId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    root_guard::ensure_safe_component(tab_id.as_str())?;
    service::clear_untitled_mirror(&state.paths, &project_id, &tab_id)
}

/// Prunes an open project's untitled mirrors under the mutation guard using the supplied keep list.
pub async fn file_prune_untitled_mirrors(state: &AppState, project_id: ProjectId, keep_tab_ids: Vec<TabId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    root_guard::project_root(&projects, &project_id)?;
    service::prune_untitled_mirrors(&state.paths, &project_id, &keep_tab_ids)
}

/// Returns authorized project or CLI-opened file bytes without a toolkit response wrapper.
pub async fn file_read_raw(state: &AppState, path: String) -> AppResult<Vec<u8>> {
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project_or_cli_opened(&projects, &state.cli_opened_paths.read(), Path::new(&path))?;
    service::read_raw(&resolved)
}

#[cfg(test)]
mod tests {
    use taide_infra::self_write::resolve_from_app;
    use taide_model::error::AppErrorKind;
    use taide_model::file::{FsChange, FsChangeKind};
    use taide_model::paths::AppPaths;
    use uuid::Uuid;

    use super::*;

    fn is_self_write(state: &AppState, path: &Path) -> bool {
        resolve_from_app(
            &state.self_writes,
            vec![FsChange {
                kind: FsChangeKind::Modified,
                paths: vec![path.to_string_lossy().into_owned()],
                from_app: false,
            }],
        )[0]
        .from_app
    }

    #[test]
    fn cli_승인_파일만_저장하고_성공_뒤_self_write를_표시한다() {
        let dir = std::env::temp_dir().join(format!("taide-runtime-cli-save-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("테스트 디렉터리 생성");
        let target = dir.join("cli.txt");
        std::fs::write(&target, "old").expect("초기 파일 저장");
        let resolved = std::fs::canonicalize(&target).expect("저장 경로 해석");
        let data_dir = dir.join("data");
        let state = AppState::new(AppPaths::new(data_dir.clone()));

        assert!(save_file_within_open_projects(&state, &target, "new").is_err_and(|error| error.kind() == AppErrorKind::Forbidden));
        assert_eq!(std::fs::read_to_string(&target).expect("거절 뒤 파일 읽기"), "old");
        assert!(!is_self_write(&state, &resolved));

        state.authorize_cli_opened_path(&target);
        save_file_within_open_projects(&state, &target, "new").expect("승인 파일 저장");
        assert_eq!(std::fs::read_to_string(&target).expect("저장 파일 읽기"), "new");
        assert!(is_self_write(&state, &resolved));
        assert!(!data_dir.exists());

        std::fs::remove_dir_all(&dir).expect("테스트 디렉터리 정리");
    }

    #[test]
    fn 원자_저장_실패는_self_write를_표시하지_않고_기존_대상을_보존한다() {
        let dir = std::env::temp_dir().join(format!("taide-runtime-failed-save-{}", Uuid::new_v4()));
        let target = dir.join("directory");
        std::fs::create_dir_all(&target).expect("테스트 디렉터리 생성");
        let sentinel = target.join("sentinel.txt");
        std::fs::write(&sentinel, "old").expect("보존 파일 저장");
        let resolved = std::fs::canonicalize(&target).expect("저장 경로 해석");
        let state = AppState::new(AppPaths::new(dir.join("data")));
        state.authorize_cli_opened_path(&target);

        let error = save_file_within_open_projects(&state, &target, "new").expect_err("디렉터리 덮어쓰기 거절");
        assert_eq!(error.kind(), AppErrorKind::Io);
        assert!(!is_self_write(&state, &resolved));
        assert!(target.is_dir());
        assert_eq!(std::fs::read_to_string(&sentinel).expect("보존 파일 읽기"), "old");
        assert_eq!(std::fs::read_dir(&dir).expect("임시 파일 정리 확인").count(), 1);

        std::fs::remove_dir_all(&dir).expect("테스트 디렉터리 정리");
    }
}

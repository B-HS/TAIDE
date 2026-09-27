use std::path::Path;

use taide_file::service::clear_mirror;
use taide_infra::{persist, root_guard};
use taide_model::error::AppResult;

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

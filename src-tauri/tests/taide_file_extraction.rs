use std::path::Path;

use taide_infra::language::LanguageOverlay;
use taide_lib::domain::file::{editorconfig as legacy_editorconfig, service as legacy_service};
use taide_lib::state::AppState;
use taide_model::error::{AppErrorKind, AppResult};
use taide_model::file::{EditorConfigOptions, OpenedFile};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use uuid::Uuid;

#[test]
fn 파일_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_open: fn(&Path, &[LanguageOverlay], bool) -> AppResult<OpenedFile> = taide_file::service::open_file;
    let legacy_open: fn(&Path, &[LanguageOverlay], bool) -> AppResult<OpenedFile> = legacy_service::open_file;
    assert!(std::ptr::fn_addr_eq(extracted_open, legacy_open));

    let extracted_editorconfig: fn(&Path) -> EditorConfigOptions = taide_file::editorconfig::resolve_for_file;
    let legacy_editorconfig: fn(&Path) -> EditorConfigOptions = legacy_editorconfig::resolve_for_file;
    assert!(std::ptr::fn_addr_eq(extracted_editorconfig, legacy_editorconfig));
}

#[test]
fn guarded_save는_루트_가드와_미러_정리를_함께_수행한다() {
    let dir = std::env::temp_dir().join(format!("taide-file-guarded-save-{}", Uuid::new_v4()));
    let root = dir.join("project");
    std::fs::create_dir_all(&root).expect("프로젝트 디렉터리 생성");
    let target = root.join("a.txt");
    std::fs::write(&target, "old").expect("초기 파일 저장");

    let state = AppState::new(AppPaths::new(dir.join("data")));
    let project_id = ProjectId::new();
    state.projects.write().insert(
        project_id.clone(),
        Project {
            id: project_id.clone(),
            root: root.to_string_lossy().into_owned(),
            name: "project".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );

    let outside = dir.join("outside.txt");
    std::fs::write(&outside, "x").expect("외부 파일 저장");
    assert!(
        legacy_service::save_file_within_open_projects(&state, &outside, "new").is_err_and(|error| error.kind() == AppErrorKind::Forbidden)
    );
    assert_eq!(std::fs::read_to_string(&outside).expect("외부 파일 읽기"), "x");

    let resolved = std::fs::canonicalize(&target).expect("저장 경로 해석");
    legacy_service::mirror_dirty(&state.paths, &project_id, &resolved, &target.to_string_lossy(), "dirty").expect("미러 생성");
    assert_eq!(legacy_service::list_mirrors(&state.paths, &project_id).expect("미러 목록").len(), 1);

    legacy_service::save_file_within_open_projects(&state, &target, "new content").expect("루트 안 저장");
    assert_eq!(std::fs::read_to_string(&target).expect("저장 파일 읽기"), "new content");
    assert!(legacy_service::list_mirrors(&state.paths, &project_id)
        .expect("미러 목록")
        .is_empty());

    std::fs::remove_dir_all(&dir).expect("테스트 디렉터리 정리");
}

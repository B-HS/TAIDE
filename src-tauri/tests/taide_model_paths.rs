use std::path::PathBuf;

use taide_lib::ids::ProjectId;
use taide_lib::paths::AppPaths as FacadeAppPaths;
use taide_model::paths::AppPaths as ModelAppPaths;

fn model_path(paths: FacadeAppPaths) -> ModelAppPaths {
    paths
}

fn facade_path(paths: ModelAppPaths) -> FacadeAppPaths {
    paths
}

#[test]
fn 모델과_기존_공개_경로의_app_paths는_동일한_타입이다() {
    let paths = model_path(FacadeAppPaths::new(PathBuf::from("data")));
    assert_eq!(facade_path(paths).data_dir, PathBuf::from("data"));
}

#[test]
fn 모든_데이터_경로가_기존_하위_위치를_유지한다() {
    let paths = ModelAppPaths::new(PathBuf::from("/data"));
    let id = ProjectId("prj-fixed".to_string());

    assert_eq!(paths.data_dir, PathBuf::from("/data"));
    assert_eq!(paths.settings_file(), PathBuf::from("/data/settings.json"));
    assert_eq!(paths.session_file(), PathBuf::from("/data/session.json"));
    assert_eq!(paths.themes_dir(), PathBuf::from("/data/themes"));
    assert_eq!(paths.snippets_dir(), PathBuf::from("/data/snippets"));
    assert_eq!(paths.locales_dir(), PathBuf::from("/data/locales"));
    assert_eq!(paths.prompts_dir(), PathBuf::from("/data/prompts"));
    assert_eq!(paths.project_dir(&id), PathBuf::from("/data/projects/prj-fixed"));
    assert_eq!(paths.project_file(&id), PathBuf::from("/data/projects/prj-fixed/project.json"));
    assert_eq!(paths.layout_file(&id), PathBuf::from("/data/projects/prj-fixed/layout.json"));
    assert_eq!(paths.buffers_dir(&id), PathBuf::from("/data/projects/prj-fixed/buffers"));
    assert_eq!(paths.plugins_dir(), PathBuf::from("/data/plugins"));
    assert_eq!(paths.lsp_dir(), PathBuf::from("/data/lsp"));
    assert_eq!(paths.lsp_server_dir("rust-analyzer"), PathBuf::from("/data/lsp/rust-analyzer"));
    assert_eq!(
        paths.lsp_server_version_dir("rust-analyzer", "2026.08.01"),
        PathBuf::from("/data/lsp/rust-analyzer/2026.08.01")
    );
}

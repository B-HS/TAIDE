use std::path::PathBuf;

use crate::ids::ProjectId;

pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub fn session_file(&self) -> PathBuf {
        self.data_dir.join("session.json")
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.data_dir.join("themes")
    }

    pub fn snippets_dir(&self) -> PathBuf {
        self.data_dir.join("snippets")
    }

    pub fn locales_dir(&self) -> PathBuf {
        self.data_dir.join("locales")
    }

    pub fn prompts_dir(&self) -> PathBuf {
        self.data_dir.join("prompts")
    }

    pub fn project_dir(&self, id: &ProjectId) -> PathBuf {
        self.data_dir.join("projects").join(id.as_str())
    }

    pub fn project_file(&self, id: &ProjectId) -> PathBuf {
        self.project_dir(id).join("project.json")
    }

    pub fn layout_file(&self, id: &ProjectId) -> PathBuf {
        self.project_dir(id).join("layout.json")
    }

    pub fn buffers_dir(&self, id: &ProjectId) -> PathBuf {
        self.project_dir(id).join("buffers")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.data_dir.join("plugins")
    }

    pub fn lsp_dir(&self) -> PathBuf {
        self.data_dir.join("lsp")
    }

    pub fn lsp_server_dir(&self, server_id: &str) -> PathBuf {
        self.lsp_dir().join(server_id)
    }

    pub fn lsp_server_version_dir(&self, server_id: &str, version: &str) -> PathBuf {
        self.lsp_server_dir(server_id).join(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 프로젝트별_경로가_아이디로_분리된다() {
        let paths = AppPaths::new(PathBuf::from("/data"));
        let id = ProjectId("prj-1".to_string());

        assert_eq!(paths.layout_file(&id), PathBuf::from("/data/projects/prj-1/layout.json"));
        assert_eq!(paths.buffers_dir(&id), PathBuf::from("/data/projects/prj-1/buffers"));
        assert_eq!(paths.session_file(), PathBuf::from("/data/session.json"));
    }

    #[test]
    fn prompts_디렉토리는_data_dir_하위에_위치한다() {
        let paths = AppPaths::new(PathBuf::from("/data"));
        assert_eq!(paths.prompts_dir(), PathBuf::from("/data/prompts"));
    }

    #[test]
    fn snippets_디렉토리는_data_dir_하위에_위치한다() {
        let paths = AppPaths::new(PathBuf::from("/data"));
        assert_eq!(paths.snippets_dir(), PathBuf::from("/data/snippets"));
    }

    #[test]
    fn lsp_경로는_서버_아이디와_버전으로_분리된다() {
        let paths = AppPaths::new(PathBuf::from("/data"));

        assert_eq!(paths.lsp_dir(), PathBuf::from("/data/lsp"));
        assert_eq!(paths.lsp_server_dir("rust-analyzer"), PathBuf::from("/data/lsp/rust-analyzer"));
        assert_eq!(
            paths.lsp_server_version_dir("rust-analyzer", "2026.08.01"),
            PathBuf::from("/data/lsp/rust-analyzer/2026.08.01")
        );
    }
}

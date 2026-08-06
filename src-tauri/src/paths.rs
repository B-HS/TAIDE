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

    pub fn locales_dir(&self) -> PathBuf {
        self.data_dir.join("locales")
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
}

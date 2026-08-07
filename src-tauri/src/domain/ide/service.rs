use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::{IDE_PORT_RANGE_END, IDE_PORT_RANGE_START};
use crate::domain::layout::types::{ProjectLayout, TabKind};
use crate::domain::project::types::Project;
use crate::error::AppResult;
use crate::ids::ProjectId;

const LANGUAGE_ID_BY_EXTENSION: &[(&str, &str)] = &[
    ("ts", "typescript"),
    ("tsx", "typescriptreact"),
    ("js", "javascript"),
    ("jsx", "javascriptreact"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("json", "json"),
    ("rs", "rust"),
    ("py", "python"),
    ("go", "go"),
    ("java", "java"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("hpp", "cpp"),
    ("cs", "csharp"),
    ("rb", "ruby"),
    ("php", "php"),
    ("sh", "shellscript"),
    ("bash", "shellscript"),
    ("zsh", "shellscript"),
    ("html", "html"),
    ("css", "css"),
    ("scss", "scss"),
    ("md", "markdown"),
    ("toml", "toml"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("sql", "sql"),
    ("swift", "swift"),
    ("kt", "kotlin"),
];
const DEFAULT_LANGUAGE_ID: &str = "plaintext";

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub fn generate_auth_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

pub fn port_from_entropy(raw: u32, start: u32, end: u32) -> u32 {
    start + (raw % (end - start + 1))
}

pub fn random_port() -> u32 {
    let uuid = uuid::Uuid::new_v4();
    let bytes = uuid.as_bytes();
    let raw = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    port_from_entropy(raw, IDE_PORT_RANGE_START, IDE_PORT_RANGE_END)
}

pub fn guess_language_id(path: &str) -> &'static str {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_lowercase())
        .and_then(|extension| {
            LANGUAGE_ID_BY_EXTENSION
                .iter()
                .find(|(key, _)| *key == extension)
                .map(|(_, id)| *id)
        })
        .unwrap_or(DEFAULT_LANGUAGE_ID)
}

pub fn ensure_path_within_any_project(projects: &HashMap<ProjectId, Project>, path: &Path) -> AppResult<(ProjectId, PathBuf)> {
    crate::domain::file::service::resolve_owning_project(projects, path)
}

pub fn workspace_folders(projects: &HashMap<ProjectId, Project>) -> Vec<String> {
    let mut roots: Vec<String> = projects
        .values()
        .filter(|project| !project.root_missing)
        .map(|project| project.root.clone())
        .collect();
    roots.sort();
    roots
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenEditorEntry {
    pub path: String,
    pub is_active: bool,
    pub label: String,
    pub language_id: String,
    pub is_dirty: bool,
}

fn file_label(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.to_string())
}

pub fn open_editors_snapshot(layouts: &HashMap<ProjectId, ProjectLayout>) -> Vec<OpenEditorEntry> {
    let mut entries = Vec::new();

    for layout in layouts.values() {
        let active_tab_id = crate::domain::layout::service::find_leaf(&layout.root, &layout.focused_pane).and_then(|leaf| {
            let crate::domain::layout::types::PaneNode::Leaf { active, .. } = leaf else {
                return None;
            };
            active.clone()
        });

        for node in crate::domain::layout::service::collect_leaves(&layout.root) {
            let crate::domain::layout::types::PaneNode::Leaf { tabs, .. } = node else {
                continue;
            };
            for tab in tabs {
                let TabKind::File { path } = &tab.kind else { continue };
                entries.push(OpenEditorEntry {
                    path: path.clone(),
                    is_active: active_tab_id.as_ref() == Some(&tab.id),
                    label: file_label(path),
                    language_id: guess_language_id(path).to_string(),
                    is_dirty: tab.dirty,
                });
            }
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::super::types::IdeDiagnosticSeverity;
    use super::*;
    use crate::domain::project::types::Project;

    #[test]
    fn 같은_바이트열은_상수시간_비교에서_참이다() {
        assert!(constant_time_eq(b"abc123", b"abc123"));
    }

    #[test]
    fn 길이가_다르면_거짓이다() {
        assert!(!constant_time_eq(b"short", b"longer-value"));
    }

    #[test]
    fn 내용이_다르면_거짓이다() {
        assert!(!constant_time_eq(b"abc123", b"abc124"));
    }

    #[test]
    fn 생성된_토큰은_32자_소문자_hex다() {
        let token = generate_auth_token();
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn 포트는_항상_범위_안에_있다() {
        for raw in [0u32, 1, 12345, u32::MAX / 2, u32::MAX] {
            let port = port_from_entropy(raw, IDE_PORT_RANGE_START, IDE_PORT_RANGE_END);
            assert!((IDE_PORT_RANGE_START..=IDE_PORT_RANGE_END).contains(&port));
        }
    }

    #[test]
    fn 알려진_확장자는_언어id로_매핑된다() {
        assert_eq!(guess_language_id("/a/b/main.rs"), "rust");
        assert_eq!(guess_language_id("/a/b/App.tsx"), "typescriptreact");
    }

    #[test]
    fn 모르는_확장자는_plaintext다() {
        assert_eq!(guess_language_id("/a/b/data.unknownext"), "plaintext");
        assert_eq!(guess_language_id("/a/b/noext"), "plaintext");
    }

    fn project(id: &str, root: &str, root_missing: bool) -> (ProjectId, Project) {
        let project_id = ProjectId::from(id.to_string());
        (
            project_id.clone(),
            Project {
                id: project_id,
                root: root.to_string(),
                name: root.to_string(),
                capabilities: Vec::new(),
                root_missing,
            },
        )
    }

    #[test]
    fn 워크스페이스_폴더는_정렬되고_missing_루트는_제외한다() {
        let projects = HashMap::from([project("a", "/z", false), project("b", "/a", false), project("c", "/gone", true)]);
        assert_eq!(workspace_folders(&projects), vec!["/a".to_string(), "/z".to_string()]);
    }

    #[test]
    fn 프로젝트_루트_안의_경로는_통과한다() {
        let tmp = std::env::temp_dir().join(format!("taide-ide-svc-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("src/main.rs"), "fn main() {}").unwrap();

        let (id, proj) = project("p", tmp.to_str().unwrap(), false);
        let projects = HashMap::from([(id.clone(), proj)]);

        let result = ensure_path_within_any_project(&projects, &tmp.join("src/main.rs"));
        assert!(result.is_ok());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 프로젝트_루트_밖의_경로는_거부된다() {
        let tmp = std::env::temp_dir().join(format!("taide-ide-svc-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let (id, proj) = project("p", tmp.to_str().unwrap(), false);
        let projects = HashMap::from([(id.clone(), proj)]);

        let outside = std::env::temp_dir();
        let result = ensure_path_within_any_project(&projects, &outside.join("definitely-outside-file.txt"));
        assert!(result.is_err());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn 진단_severity는_camelcase_문자열로_직렬화된다() {
        let value = serde_json::to_value(IdeDiagnosticSeverity::Warning).unwrap();
        assert_eq!(value, serde_json::json!("warning"));
    }
}

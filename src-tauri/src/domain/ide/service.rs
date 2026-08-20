use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::{IDE_PORT_RANGE_END, IDE_PORT_RANGE_START};
use crate::domain::layout::types::{ProjectLayout, TabKind};
use crate::domain::project::types::Project;
use crate::error::AppResult;
use crate::ids::ProjectId;
use crate::infra::language::LanguageOverlay;
use crate::infra::{language, root_guard};

pub use crate::infra::crypto::constant_time_eq;

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

/// Extensions the MCP `languageId` response (`getOpenEditors`/`openFile`) should still identify
/// precisely even though TAIDE has no bundled grammar for them —
/// `infra::language::LANGUAGE_ID_BY_EXTENSION` deliberately excludes them for that reason (see its
/// doc comment: every entry there doubles as a member of the frontend's `TAIDE_LANGUAGE_IDS`, and
/// these three aren't). An external MCP client (Claude Code) has no notion of "TAIDE doesn't
/// highlight C#" — `languageId` there is just metadata the client uses to judge what kind of file
/// it's looking at, so collapsing it to `"plaintext"` loses real information for no benefit to this
/// consumer. Checked only once [`guess_language_id`]'s shared-table/plugin-overlay lookup has
/// already missed (resolved to `"plaintext"`), so a plugin-contributed or future bundled grammar
/// for the same extension always wins over this fallback.
const MCP_ONLY_LANGUAGE_ID_BY_EXTENSION: &[(&str, &str)] = &[("cs", "csharp"), ("php", "php"), ("sql", "sql")];

pub fn guess_language_id(path: &str, language_overlays: &[LanguageOverlay]) -> String {
    let resolved = language::language_id_for_path(Path::new(path), language_overlays);
    if resolved != "plaintext" {
        return resolved;
    }

    let Some(extension) = Path::new(path).extension().and_then(|value| value.to_str()) else {
        return resolved;
    };
    let extension = extension.to_lowercase();
    MCP_ONLY_LANGUAGE_ID_BY_EXTENSION
        .iter()
        .find(|(key, _)| *key == extension)
        .map(|(_, id)| (*id).to_string())
        .unwrap_or(resolved)
}

pub fn ensure_path_within_any_project(projects: &HashMap<ProjectId, Project>, path: &Path) -> AppResult<(ProjectId, PathBuf)> {
    root_guard::resolve_owning_project(projects, path)
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

/// Snapshots every open `File` tab across a project's main tree *and* its auxiliary windows
/// (`layout::service::all_roots`) — an auxiliary-window tab is still an open editor from the
/// external MCP client's point of view, even though it renders in a different OS window.
/// "Active" is scoped to the main tree's focused pane only: auxiliary windows don't have a
/// project-wide notion of "the" active tab, so their tabs are always reported `is_active: false`.
pub fn open_editors_snapshot(layouts: &HashMap<ProjectId, ProjectLayout>, language_overlays: &[LanguageOverlay]) -> Vec<OpenEditorEntry> {
    let mut entries = Vec::new();

    for layout in layouts.values() {
        let active_tab_id = crate::domain::layout::service::find_leaf(&layout.root, &layout.focused_pane).and_then(|leaf| {
            let crate::domain::layout::types::PaneNode::Leaf { active, .. } = leaf else {
                return None;
            };
            active.clone()
        });

        for root in crate::domain::layout::service::all_roots(layout) {
            for node in crate::domain::layout::service::collect_leaves(root) {
                let crate::domain::layout::types::PaneNode::Leaf { tabs, .. } = node else {
                    continue;
                };
                for tab in tabs {
                    let TabKind::File { path } = &tab.kind else { continue };
                    entries.push(OpenEditorEntry {
                        path: path.clone(),
                        is_active: active_tab_id.as_ref() == Some(&tab.id),
                        label: file_label(path),
                        language_id: guess_language_id(path, language_overlays),
                        is_dirty: tab.dirty,
                    });
                }
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
        assert_eq!(guess_language_id("/a/b/main.rs", &[]), "rust");
        assert_eq!(guess_language_id("/a/b/App.tsx", &[]), "typescriptreact");
    }

    #[test]
    fn 모르는_확장자는_plaintext다() {
        assert_eq!(guess_language_id("/a/b/data.unknownext", &[]), "plaintext");
        assert_eq!(guess_language_id("/a/b/noext", &[]), "plaintext");
    }

    #[test]
    fn 그래머_없는_확장자도_mcp_전용_보강_테이블로_식별된다() {
        assert_eq!(guess_language_id("/a/b/Service.cs", &[]), "csharp");
        assert_eq!(guess_language_id("/a/b/index.php", &[]), "php");
        assert_eq!(guess_language_id("/a/b/schema.SQL", &[]), "sql");
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
                last_opened_at: 0.0,
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

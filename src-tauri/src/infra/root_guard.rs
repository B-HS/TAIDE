use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::project::types::Project;
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;

pub fn project_root(projects: &HashMap<ProjectId, Project>, project_id: &ProjectId) -> AppResult<PathBuf> {
    projects
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

pub fn resolve_owning_project(projects: &HashMap<ProjectId, Project>, path: &Path) -> AppResult<(ProjectId, PathBuf)> {
    for (project_id, project) in projects {
        if let Ok(resolved) = ensure_within_root(Path::new(&project.root), path) {
            return Ok((project_id.clone(), resolved));
        }
    }
    Err(AppError::InvalidArgument(format!(
        "열린 프로젝트 루트 밖의 경로입니다: {}",
        path.display()
    )))
}

pub fn ensure_within_root(root: &Path, path: &Path) -> AppResult<PathBuf> {
    let canonical_root = std::fs::canonicalize(root)?;
    let resolved = canonicalize_lenient(path)?;

    if resolved.starts_with(&canonical_root) {
        Ok(resolved)
    } else {
        Err(AppError::InvalidArgument(format!(
            "경로가 프로젝트 루트 밖에 있습니다: {}",
            path.display()
        )))
    }
}

/// Rejects identifiers that would traverse or escape a directory when used
/// as a single path component (joined verbatim, not resolved via a project
/// root). Untitled-tab mirror files are keyed by `TabId` this way, so the id
/// must not contain path separators or `.`/`..` segments.
pub fn ensure_safe_component(value: &str) -> AppResult<()> {
    let is_traversal = value.is_empty() || value == "." || value == ".." || value.contains('/') || value.contains('\\');
    if is_traversal {
        return Err(AppError::InvalidArgument(format!("유효하지 않은 식별자입니다: {value}")));
    }
    Ok(())
}

fn canonicalize_lenient(path: &Path) -> AppResult<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Ok(canonical);
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::InvalidArgument(format!("유효하지 않은 경로입니다: {}", path.display())))?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("유효하지 않은 경로입니다: {}", path.display())))?;

    Ok(canonicalize_lenient(parent)?.join(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taide-root-guard-test-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn cleanup(dir: &Path) {
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn 루트_밖_경로는_거부된다() {
        let dir = temp_dir("root-guard");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(dir.join("outside")).unwrap();

        let inside = root.join("src").join("main.rs");
        let outside = dir.join("outside").join("secret.txt");

        assert!(ensure_within_root(&root, &inside).is_ok());
        assert!(ensure_within_root(&root, &outside).is_err());

        cleanup(&dir);
    }

    #[test]
    fn 루트_자신은_허용된다() {
        let dir = temp_dir("root-guard-self");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();

        assert!(ensure_within_root(&root, &root).is_ok());

        cleanup(&dir);
    }

    #[test]
    fn 존재하지_않는_새_파일_경로도_루트_하위면_허용된다() {
        let dir = temp_dir("root-guard-new");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();

        let new_file = root.join("new-folder").join("new-file.rs");

        let resolved = ensure_within_root(&root, &new_file).expect("허용되어야 한다");
        assert!(resolved.starts_with(std::fs::canonicalize(&root).unwrap()));

        cleanup(&dir);
    }

    #[test]
    fn 일반_식별자는_안전_컴포넌트로_허용된다() {
        assert!(ensure_safe_component("tab-1234").is_ok());
    }

    #[test]
    fn 경로_탈출_시도는_안전_컴포넌트에서_거부된다() {
        assert!(ensure_safe_component("..").is_err());
        assert!(ensure_safe_component(".").is_err());
        assert!(ensure_safe_component("").is_err());
        assert!(ensure_safe_component("../../etc/passwd").is_err());
        assert!(ensure_safe_component("a/b").is_err());
        assert!(ensure_safe_component("a\\b").is_err());
    }
}

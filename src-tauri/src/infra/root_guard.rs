use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::project::types::Project;
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::ids::ProjectId;

pub fn project_root(projects: &HashMap<ProjectId, Project>, project_id: &ProjectId) -> AppResult<PathBuf> {
    projects
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

/// Picks the *most specific* open project whose root contains `path`, deterministically — not
/// "whichever `HashMap` iteration happens to visit first" (unspecified, and can differ run to
/// run). When two open projects' roots are nested (e.g. a workspace root and a sub-project opened
/// separately inside it) or, degenerately, identical, a path under the inner one used to resolve
/// to either project depending on hash-bucket order. The winner is now chosen by, in order: (1)
/// the longest canonical root (the more specific/nested project), (2) on an exact tie, the
/// lexicographically smallest [`ProjectId`] — both criteria are independent of map iteration
/// order, so the result is stable across runs for the same set of open projects.
pub fn resolve_owning_project(projects: &HashMap<ProjectId, Project>, path: &Path) -> AppResult<(ProjectId, PathBuf)> {
    let mut best: Option<(ProjectId, PathBuf, PathBuf)> = None;

    for (project_id, project) in projects {
        let Ok((canonical_root, resolved)) = canonicalize_root_and_resolve(Path::new(&project.root), path) else {
            continue;
        };

        let is_better = match &best {
            None => true,
            Some((best_id, _, best_root)) => match canonical_root.as_os_str().len().cmp(&best_root.as_os_str().len()) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => project_id < best_id,
            },
        };
        if is_better {
            best = Some((project_id.clone(), resolved, canonical_root));
        }
    }

    // `Forbidden`, not `InvalidArgument` — a boundary rejection is a distinct failure mode from a
    // malformed/duplicate argument (e.g. `file/service.rs`'s "already exists"), and callers that
    // probe path existence via `file_open` (`workspace-edit-applier.ts`'s `pathExists`) rely on the
    // two being distinguishable by error code.
    best.map(|(project_id, resolved, _)| (project_id, resolved)).ok_or_else(|| {
        AppError::localized(
            AppErrorKind::Forbidden,
            "error.path.outsideOpenProjects",
            format!("path is outside every open project's root: {}", path.display()),
        )
        .with_arg("path", path.display())
    })
}

/// Rejects a path that no longer names a file on disk, raising the same localized
/// `error.file.notFound` (arg `path`) that `file::service::open_file` raises, so the message reads
/// identically wherever it surfaces. `resolved` is the canonical path actually probed;
/// `display_path` is the path the caller was handed, and the one the message and the arg carry.
///
/// It lives here, in infra, because *both* ways a file tab can be opened have to run it before the
/// tab exists: `layout::commands::layout_open_tab` (the command every UI and remote-session open
/// goes through) and the IDE MCP `openFile` tool, which cannot call another domain's command
/// function and drives `layout::service::open_tab_and_finish` directly. A gate private to either
/// one would leave the other opening an empty tab for a path that was only remembered — a stale
/// quick-open index entry, or a file Claude Code saw before it was renamed — whose body then showed
/// nothing but a raw `os error 2`. The layout service itself stays unguarded on purpose: it is a
/// pure in-memory tree, and its tests open tabs for paths that never existed.
pub fn ensure_existing_file(resolved: &Path, display_path: &str) -> AppResult<()> {
    if std::fs::metadata(resolved).is_ok_and(|metadata| metadata.is_file()) {
        return Ok(());
    }

    Err(AppError::localized(
        AppErrorKind::NotFound,
        "error.file.notFound",
        format!("file not found: {display_path}"),
    )
    .with_arg("path", display_path))
}

pub fn ensure_within_root(root: &Path, path: &Path) -> AppResult<PathBuf> {
    canonicalize_root_and_resolve(root, path).map(|(_, resolved)| resolved)
}

/// Shared implementation behind [`ensure_within_root`] and the per-candidate check inside
/// [`resolve_owning_project`]'s loop — both need the same "canonicalize root, canonicalize path,
/// check containment" work, and the loop additionally needs the canonical root itself (for its
/// longest-root tie-break) rather than re-deriving it with a second `std::fs::canonicalize` call
/// per candidate per request.
fn canonicalize_root_and_resolve(root: &Path, path: &Path) -> AppResult<(PathBuf, PathBuf)> {
    let canonical_root = std::fs::canonicalize(root)?;
    let resolved = canonicalize_lenient(path)?;

    if resolved.starts_with(&canonical_root) {
        Ok((canonical_root, resolved))
    } else {
        Err(AppError::localized(
            AppErrorKind::Forbidden,
            "error.path.outsideProjectRoot",
            format!("path is outside the project root: {}", path.display()),
        )
        .with_arg("path", path.display()))
    }
}

/// Rejects identifiers that would traverse or escape a directory when used
/// as a single path component (joined verbatim, not resolved via a project
/// root). Untitled-tab mirror files are keyed by `TabId` this way, so the id
/// must not contain path separators or `.`/`..` segments.
pub fn ensure_safe_component(value: &str) -> AppResult<()> {
    let is_traversal = value.is_empty() || value == "." || value == ".." || value.contains('/') || value.contains('\\');
    if is_traversal {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.path.invalidIdentifier",
            format!("invalid identifier: {value}"),
        )
        .with_arg("value", value));
    }
    Ok(())
}

pub(crate) fn canonicalize_lenient(path: &Path) -> AppResult<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Ok(canonical);
    }

    let file_name = path.file_name().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.path.invalid",
            format!("invalid path: {}", path.display()),
        )
        .with_arg("path", path.display())
    })?;
    let parent = path.parent().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.path.invalid",
            format!("invalid path: {}", path.display()),
        )
        .with_arg("path", path.display())
    })?;

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
    fn 루트_밖_경로_거부는_forbidden_이지_invalid_argument_가_아니다() {
        let dir = temp_dir("root-guard-forbidden");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(dir.join("outside")).unwrap();
        let outside = dir.join("outside").join("secret.txt");

        let error = ensure_within_root(&root, &outside).expect_err("root 밖 경로는 실패해야 한다");
        assert_eq!(error.kind(), AppErrorKind::Forbidden);

        let mut projects = HashMap::new();
        projects.insert(
            ProjectId::from("project-1".to_string()),
            Project {
                id: ProjectId::from("project-1".to_string()),
                root: root.to_string_lossy().to_string(),
                name: "project".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
            },
        );
        let error = resolve_owning_project(&projects, &outside).expect_err("어떤 프로젝트 루트에도 속하지 않아야 한다");
        assert_eq!(error.kind(), AppErrorKind::Forbidden);

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

    fn make_project(id: &str, root: &Path) -> Project {
        Project {
            id: ProjectId::from(id.to_string()),
            root: root.to_string_lossy().to_string(),
            name: id.to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
        }
    }

    #[test]
    fn 중첩된_프로젝트_루트에서는_맵_순회_순서와_무관하게_가장_구체적인_루트가_선택된다() {
        let dir = temp_dir("root-guard-nested");
        let outer_root = dir.join("workspace");
        let inner_root = outer_root.join("packages").join("inner");
        std::fs::create_dir_all(&inner_root).unwrap();
        let target = inner_root.join("src").join("main.rs");

        let mut projects = HashMap::new();
        projects.insert(ProjectId::from("outer".to_string()), make_project("outer", &outer_root));
        projects.insert(ProjectId::from("inner".to_string()), make_project("inner", &inner_root));

        for _ in 0..8 {
            let (winner, _) = resolve_owning_project(&projects, &target).expect("어느 한쪽 루트에는 속해야 한다");
            assert_eq!(
                winner,
                ProjectId::from("inner".to_string()),
                "가장 구체적인(가장 긴) 루트가 항상 선택되어야 한다"
            );
        }

        cleanup(&dir);
    }

    #[test]
    fn 동일한_루트를_가진_두_프로젝트는_project_id_오름차순으로_결정적으로_선택된다() {
        let dir = temp_dir("root-guard-tie");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("main.rs");

        let mut projects = HashMap::new();
        projects.insert(ProjectId::from("zzz".to_string()), make_project("zzz", &root));
        projects.insert(ProjectId::from("aaa".to_string()), make_project("aaa", &root));

        let (winner, _) = resolve_owning_project(&projects, &target).expect("루트 하위 경로는 허용되어야 한다");
        assert_eq!(
            winner,
            ProjectId::from("aaa".to_string()),
            "동률이면 사전순으로 가장 작은 ProjectId가 선택되어야 한다"
        );

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

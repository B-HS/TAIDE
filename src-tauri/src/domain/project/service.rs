use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::persist;
use crate::paths::AppPaths;

use super::types::{CapabilityKind, Project, ProjectRef, SessionState};

const GIT_DIR_NAME: &str = ".git";
const BACKUP_SUFFIX: &str = ".bak";
const PROJECTS_DIR_NAME: &str = "projects";

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectOpenResult {
    pub project: Project,
    pub already_open: bool,
}

pub fn list_projects(session: &SessionState) -> Vec<ProjectRef> {
    session.projects.clone()
}

pub fn get_project(projects: &HashMap<ProjectId, Project>, project_id: &ProjectId) -> AppResult<Project> {
    projects
        .get(project_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

pub fn open_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    root: &Path,
) -> AppResult<ProjectOpenResult> {
    let canonical = std::fs::canonicalize(root)?;
    let metadata = std::fs::metadata(&canonical)?;
    if !metadata.is_dir() {
        return Err(AppError::InvalidArgument(format!(
            "경로가 디렉토리가 아닙니다: {}",
            canonical.display()
        )));
    }
    std::fs::read_dir(&canonical)?;

    let root_str = canonical.to_string_lossy().to_string();

    if let Some(existing) = projects.values().find(|project| project.root == root_str) {
        let existing = existing.clone();
        session.active_project = Some(existing.id.clone());
        save_session(paths, session)?;
        return Ok(ProjectOpenResult {
            project: existing,
            already_open: true,
        });
    }

    let id = find_existing_project_id(paths, &root_str)?.unwrap_or_else(ProjectId::new);

    let name = canonical
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| root_str.clone());

    let mut capabilities = Vec::new();
    if canonical.join(GIT_DIR_NAME).is_dir() {
        capabilities.push(CapabilityKind::Git);
    }
    capabilities.push(CapabilityKind::Terminal);

    let project = Project {
        id: id.clone(),
        root: root_str,
        name,
        capabilities,
        root_missing: false,
    };

    projects.insert(id, project.clone());
    upsert_project_ref(session, &project);
    session.active_project = Some(project.id.clone());

    save_project(paths, &project)?;
    save_session(paths, session)?;

    Ok(ProjectOpenResult {
        project,
        already_open: false,
    })
}

pub fn close_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
) -> AppResult<()> {
    projects.remove(project_id);
    session.projects.retain(|reference| &reference.id != project_id);
    if session.active_project.as_ref() == Some(project_id) {
        session.active_project = session.projects.last().map(|reference| reference.id.clone());
    }
    save_session(paths, session)
}

pub fn activate_project(paths: &AppPaths, session: &mut SessionState, project_id: &ProjectId) -> AppResult<()> {
    if !session.projects.iter().any(|reference| &reference.id == project_id) {
        return Err(AppError::NotFound(format!("project not open: {project_id}")));
    }
    session.active_project = Some(project_id.clone());
    save_session(paths, session)
}

pub fn reorder_projects(paths: &AppPaths, session: &mut SessionState, ids: &[ProjectId]) -> AppResult<()> {
    let original = std::mem::take(&mut session.projects);
    let mut used = vec![false; original.len()];
    let mut reordered = Vec::with_capacity(original.len());

    for id in ids {
        if let Some(position) = original.iter().position(|reference| &reference.id == id) {
            if !used[position] {
                used[position] = true;
                reordered.push(original[position].clone());
            }
        }
    }

    for (index, reference) in original.into_iter().enumerate() {
        if !used[index] {
            reordered.push(reference);
        }
    }

    session.projects = reordered;
    save_session(paths, session)
}

pub fn migrate_session(value: serde_json::Value) -> AppResult<SessionState> {
    let session: SessionState = serde_json::from_value(value)?;
    Ok(session)
}

pub fn load_session(paths: &AppPaths) -> AppResult<(SessionState, Vec<String>)> {
    let path = paths.session_file();
    let raw = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((SessionState::default(), Vec::new()));
        }
        Err(error) => return Err(error.into()),
    };

    let parsed = serde_json::from_slice::<serde_json::Value>(&raw)
        .map_err(AppError::from)
        .and_then(migrate_session);

    match parsed {
        Ok(session) => Ok((session, Vec::new())),
        Err(_) => {
            backup_corrupted(&path)?;
            Ok((
                SessionState::default(),
                vec![format!("손상된 세션 파일을 백업하고 기본값으로 초기화했습니다: {}", path.display())],
            ))
        }
    }
}

pub fn save_session(paths: &AppPaths, session: &SessionState) -> AppResult<()> {
    persist::write_json(&paths.session_file(), session)
}

pub fn load_project(paths: &AppPaths, id: &ProjectId) -> AppResult<(Option<Project>, Vec<String>)> {
    let path = paths.project_file(id);
    let raw = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, Vec::new())),
        Err(error) => return Err(error.into()),
    };

    match serde_json::from_slice::<Project>(&raw) {
        Ok(project) => Ok((Some(project), Vec::new())),
        Err(_) => {
            backup_corrupted(&path)?;
            Ok((None, vec![format!("손상된 프로젝트 파일을 백업했습니다: {}", path.display())]))
        }
    }
}

pub fn save_project(paths: &AppPaths, project: &Project) -> AppResult<()> {
    persist::write_json(&paths.project_file(&project.id), project)
}

pub fn restore_session(paths: &AppPaths) -> AppResult<(SessionState, Vec<Project>, Vec<String>)> {
    let (session, mut warnings) = load_session(paths)?;
    let mut projects = Vec::with_capacity(session.projects.len());

    for reference in &session.projects {
        let (loaded, load_warnings) = load_project(paths, &reference.id)?;
        warnings.extend(load_warnings);

        let mut project = loaded.unwrap_or_else(|| Project {
            id: reference.id.clone(),
            root: reference.root.clone(),
            name: reference.name.clone(),
            capabilities: Vec::new(),
            root_missing: false,
        });

        project.root_missing = !Path::new(&project.root).is_dir();
        projects.push(project);
    }

    Ok((session, projects, warnings))
}

fn upsert_project_ref(session: &mut SessionState, project: &Project) {
    if let Some(existing) = session.projects.iter_mut().find(|reference| reference.id == project.id) {
        existing.root = project.root.clone();
        existing.name = project.name.clone();
    } else {
        session.projects.push(ProjectRef {
            id: project.id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
        });
    }
}

fn find_existing_project_id(paths: &AppPaths, root: &str) -> AppResult<Option<ProjectId>> {
    let projects_root = paths.data_dir.join(PROJECTS_DIR_NAME);
    let entries = match std::fs::read_dir(&projects_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = ProjectId(entry.file_name().to_string_lossy().to_string());
        let (loaded, _warnings) = load_project(paths, &id)?;
        if let Some(project) = loaded {
            if project.root == root {
                return Ok(Some(id));
            }
        }
    }

    Ok(None)
}

fn backup_corrupted(path: &Path) -> AppResult<()> {
    let backup_path = PathBuf::from(format!("{}{}", path.display(), BACKUP_SUFFIX));
    std::fs::rename(path, backup_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_paths() -> AppPaths {
        let dir = std::env::temp_dir().join(format!("taide-project-test-{}", uuid::Uuid::new_v4()));
        AppPaths::new(dir)
    }

    fn cleanup(paths: &AppPaths) {
        std::fs::remove_dir_all(&paths.data_dir).ok();
    }

    #[test]
    fn 동일_root_재열기는_기존_프로젝트를_반환한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root).expect("open");
        assert!(!first.already_open);

        let second = open_project(&paths, &mut session, &mut projects, &project_root).expect("open again");
        assert!(second.already_open);
        assert_eq!(first.project.id, second.project.id);
        assert_eq!(projects.len(), 1);
        assert_eq!(session.projects.len(), 1);

        cleanup(&paths);
    }

    #[test]
    fn 이력이_있는_root는_기존_id를_재사용한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();
        let canonical_root = std::fs::canonicalize(&project_root).unwrap().to_string_lossy().to_string();

        let previous_id = ProjectId::new();
        let history = Project {
            id: previous_id.clone(),
            root: canonical_root,
            name: "workspace".to_string(),
            capabilities: vec![CapabilityKind::Terminal],
            root_missing: false,
        };
        save_project(&paths, &history).expect("seed history");

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root).expect("open");
        assert_eq!(opened.project.id, previous_id);
        assert!(!opened.already_open);

        cleanup(&paths);
    }

    #[test]
    fn git_디렉토리가_있으면_git_capability가_부착된다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("repo");
        std::fs::create_dir_all(project_root.join(GIT_DIR_NAME)).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root).expect("open");

        assert!(opened.project.capabilities.contains(&CapabilityKind::Git));
        assert!(opened.project.capabilities.contains(&CapabilityKind::Terminal));

        cleanup(&paths);
    }

    #[test]
    fn reorder는_누락된_id를_뒤에_보존한다() {
        let paths = temp_paths();
        let mut session = SessionState {
            projects: vec![
                ProjectRef {
                    id: ProjectId("prj-a".to_string()),
                    root: "/a".to_string(),
                    name: "a".to_string(),
                },
                ProjectRef {
                    id: ProjectId("prj-b".to_string()),
                    root: "/b".to_string(),
                    name: "b".to_string(),
                },
                ProjectRef {
                    id: ProjectId("prj-c".to_string()),
                    root: "/c".to_string(),
                    name: "c".to_string(),
                },
            ],
            ..SessionState::default()
        };

        reorder_projects(
            &paths,
            &mut session,
            &[ProjectId("prj-c".to_string()), ProjectId("prj-missing".to_string())],
        )
        .expect("reorder");

        let ids: Vec<_> = session.projects.iter().map(|reference| reference.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["prj-c", "prj-a", "prj-b"]);

        cleanup(&paths);
    }

    #[test]
    fn 파손된_session_json은_백업되고_기본값을_반환한다() {
        let paths = temp_paths();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        std::fs::write(paths.session_file(), b"not json").unwrap();

        let (session, warnings) = load_session(&paths).expect("load");

        assert_eq!(session, SessionState::default());
        assert_eq!(warnings.len(), 1);

        let backup_path = PathBuf::from(format!("{}{}", paths.session_file().display(), BACKUP_SUFFIX));
        assert!(backup_path.exists());
        assert!(!paths.session_file().exists());

        cleanup(&paths);
    }

    #[test]
    fn 루트가_없으면_root_missing이_표시된다() {
        let paths = temp_paths();
        let missing_root = paths.data_dir.join("gone");

        let id = ProjectId::new();
        let project = Project {
            id: id.clone(),
            root: missing_root.to_string_lossy().to_string(),
            name: "gone".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
        };
        save_project(&paths, &project).expect("save project");

        let mut session = SessionState::default();
        session.projects.push(ProjectRef {
            id: id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
        });
        save_session(&paths, &session).expect("save session");

        let (_restored_session, projects, warnings) = restore_session(&paths).expect("restore");

        assert_eq!(projects.len(), 1);
        assert!(projects[0].root_missing);
        assert!(warnings.is_empty());

        cleanup(&paths);
    }

    #[test]
    fn close_project은_세션과_활성프로젝트에서_제거한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let opened = open_project(&paths, &mut session, &mut projects, &project_root).expect("open");
        session.active_project = Some(opened.project.id.clone());

        close_project(&paths, &mut session, &mut projects, &opened.project.id).expect("close");

        assert!(projects.is_empty());
        assert!(session.projects.is_empty());
        assert_eq!(session.active_project, None);

        cleanup(&paths);
    }

    #[test]
    fn activate_project은_세션에_없는_id면_실패한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();

        let result = activate_project(&paths, &mut session, &ProjectId::new());

        assert!(result.is_err());

        cleanup(&paths);
    }
}

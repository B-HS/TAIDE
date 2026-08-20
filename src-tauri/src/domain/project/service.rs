use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::persist;
use crate::paths::AppPaths;

use super::types::{CapabilityKind, Project, ProjectRef, SessionState};

const BACKUP_SUFFIX: &str = ".bak";
const PROJECTS_DIR_NAME: &str = "projects";
const MS_PER_SECOND: f64 = 1_000.0;

/// Epoch milliseconds "now", for `Project.last_opened_at` (IPC time-field convention). Falls back
/// to `0.0` on a clock error (pre-1970 system clock) rather than panicking — the same stance
/// `domain::file::service::now_epoch_ms` takes for its own `saved_at_ms`/`modified_ms` fields.
fn now_epoch_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64() * MS_PER_SECOND)
        .unwrap_or(0.0)
}

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

/// Opens (or re-activates) the project at `root`. `detect_capabilities` is called once with the
/// canonicalized root on a fresh open and its result is recorded verbatim as
/// `Project.capabilities` — `project_open` injects the capability registry's `detected_kinds`, so
/// the registry is the single source of that field and `GitWatcherCapability::attach`'s
/// `contains(Git)` gate is the one place the git decision is made on the open path (no second
/// filesystem probe here).
pub fn open_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    root: &Path,
    detect_capabilities: impl FnOnce(&Path) -> Vec<CapabilityKind>,
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
        let mut existing = existing.clone();
        existing.last_opened_at = now_epoch_ms();
        projects.insert(existing.id.clone(), existing.clone());
        session.active_project = Some(existing.id.clone());
        save_project(paths, &existing)?;
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

    let capabilities = detect_capabilities(&canonical);

    let project = Project {
        id: id.clone(),
        root: root_str,
        name,
        capabilities,
        root_missing: false,
        last_opened_at: now_epoch_ms(),
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

/// Activates `project_id` (must already be open in `session`) and, when its `Project` record is
/// present in `projects`, stamps `last_opened_at` and persists it — mirrors `open_project`'s
/// re-open branch so a mere pane-switch back to an already-open project also counts toward the
/// Welcome screen's "recent" ordering (contract §1.1), not only a fresh `project_open` call.
pub fn activate_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
) -> AppResult<()> {
    if !session.projects.iter().any(|reference| &reference.id == project_id) {
        return Err(AppError::NotFound(format!("project not open: {project_id}")));
    }
    session.active_project = Some(project_id.clone());
    if let Some(project) = projects.get_mut(project_id) {
        project.last_opened_at = now_epoch_ms();
        save_project(paths, project)?;
    }
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
            last_opened_at: 0.0,
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

/// Every project id with a persisted record under `paths.data_dir/projects/` — the full on-disk
/// history, independent of which projects (if any) are currently open in `SessionState`. Shared by
/// `find_existing_project_id` (id-reuse lookup by root) and `list_recent_projects` (the Welcome
/// screen's full history listing), so both read the exact same directory-listing rule.
fn iter_project_ids(paths: &AppPaths) -> AppResult<Vec<ProjectId>> {
    let projects_root = paths.data_dir.join(PROJECTS_DIR_NAME);
    let entries = match std::fs::read_dir(&projects_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        ids.push(ProjectId(entry.file_name().to_string_lossy().to_string()));
    }
    Ok(ids)
}

/// Every persisted project record, most-recently-opened first (`Project.last_opened_at`
/// descending) — the Welcome screen's "recent projects" source, distinct from `list_projects`
/// (which only returns the *currently open* session's `ProjectRef`s). `root_missing` is
/// recomputed against the live filesystem the same way `restore_session` does, so a project whose
/// folder moved or was deleted since it was last opened still lists (disabled, per contract §1.2)
/// instead of silently vanishing. A record that fails to parse (corrupted/backed-up) is skipped
/// rather than failing the whole listing — best-effort, matching `restore_session`'s per-entry
/// tolerance.
pub fn list_recent_projects(paths: &AppPaths) -> AppResult<Vec<Project>> {
    let mut projects = Vec::new();
    for id in iter_project_ids(paths)? {
        let (loaded, _warnings) = load_project(paths, &id)?;
        if let Some(mut project) = loaded {
            project.root_missing = !Path::new(&project.root).is_dir();
            projects.push(project);
        }
    }

    projects.sort_by(|a, b| b.last_opened_at.total_cmp(&a.last_opened_at));
    Ok(projects)
}

fn find_existing_project_id(paths: &AppPaths, root: &str) -> AppResult<Option<ProjectId>> {
    for id in iter_project_ids(paths)? {
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

    fn detect_terminal_only(_root: &Path) -> Vec<CapabilityKind> {
        vec![CapabilityKind::Terminal]
    }

    #[test]
    fn 동일_root_재열기는_기존_프로젝트를_반환한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");
        assert!(!first.already_open);

        let second = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open again");
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
            last_opened_at: 1_000.0,
        };
        save_project(&paths, &history).expect("seed history");

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");
        assert_eq!(opened.project.id, previous_id);
        assert!(!opened.already_open);

        cleanup(&paths);
    }

    #[test]
    fn capabilities는_주입된_검출_결과를_그대로_기록한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("repo");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root, |_root| {
            vec![CapabilityKind::Git, CapabilityKind::Terminal]
        })
        .expect("open");

        assert_eq!(opened.project.capabilities, vec![CapabilityKind::Git, CapabilityKind::Terminal]);

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
            last_opened_at: 0.0,
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
        let opened = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");
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
        let mut projects = HashMap::new();

        let result = activate_project(&paths, &mut session, &mut projects, &ProjectId::new());

        assert!(result.is_err());

        cleanup(&paths);
    }

    #[test]
    fn open_project은_재열기_시에도_last_opened_at을_갱신한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");
        let second = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open again");

        assert!(second.already_open);
        assert!(
            second.project.last_opened_at >= first.project.last_opened_at,
            "재열기는 last_opened_at 을 뒤로 미루거나 최소한 유지해야 한다"
        );
        let persisted = projects.get(&second.project.id).expect("in-memory project");
        assert_eq!(
            persisted.last_opened_at, second.project.last_opened_at,
            "재열기 결과가 in-memory 맵에도 반영돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn activate_project은_last_opened_at을_갱신하고_영속화한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let opened = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");

        let before = projects
            .get(&opened.project.id)
            .expect("open 직후 in-memory project")
            .last_opened_at;
        projects.get_mut(&opened.project.id).expect("mutate before activate").last_opened_at = 0.0;

        activate_project(&paths, &mut session, &mut projects, &opened.project.id).expect("activate");

        let after_memory = projects
            .get(&opened.project.id)
            .expect("activate 후 in-memory project")
            .last_opened_at;
        assert!(after_memory > 0.0, "activate 는 last_opened_at 을 다시 채워야 한다");
        assert!(after_memory >= before, "activate 이후 시각이 최초 open 시각보다 과거일 수 없다");

        let (persisted, _warnings) = load_project(&paths, &opened.project.id).expect("load persisted project");
        assert_eq!(
            persisted.expect("project.json 이 존재해야 한다").last_opened_at,
            after_memory,
            "activate 의 갱신은 디스크에도 저장돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn list_recent_projects은_last_opened_at_내림차순으로_정렬한다() {
        let paths = temp_paths();

        let older = Project {
            id: ProjectId("prj-older".to_string()),
            root: "/older".to_string(),
            name: "older".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 1_000.0,
        };
        let newer = Project {
            id: ProjectId("prj-newer".to_string()),
            root: "/newer".to_string(),
            name: "newer".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 2_000.0,
        };
        save_project(&paths, &older).expect("seed older");
        save_project(&paths, &newer).expect("seed newer");

        let recent = list_recent_projects(&paths).expect("list recent");

        let ids: Vec<_> = recent.iter().map(|project| project.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["prj-newer", "prj-older"]);

        cleanup(&paths);
    }

    #[test]
    fn list_recent_projects은_세션에_없는_기록도_포함하고_root_missing을_표시한다() {
        let paths = temp_paths();
        let missing_root = paths.data_dir.join("gone-forever");

        let closed = Project {
            id: ProjectId::new(),
            root: missing_root.to_string_lossy().to_string(),
            name: "gone-forever".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 500.0,
        };
        save_project(&paths, &closed).expect("seed closed history");

        let recent = list_recent_projects(&paths).expect("list recent");

        assert_eq!(recent.len(), 1);
        assert!(
            recent[0].root_missing,
            "세션에 없어도 디스크 기록이면 목록에 포함되고 root_missing 이 표시돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn last_opened_at이_없는_구버전_project_json도_기본값으로_파싱된다() {
        let paths = temp_paths();
        std::fs::create_dir_all(paths.data_dir.join("projects").join("prj-legacy")).unwrap();
        let legacy_json = serde_json::json!({
            "id": "prj-legacy",
            "root": "/legacy",
            "name": "legacy",
        });
        std::fs::write(
            paths.data_dir.join("projects").join("prj-legacy").join("project.json"),
            serde_json::to_vec(&legacy_json).unwrap(),
        )
        .unwrap();

        let (loaded, warnings) = load_project(&paths, &ProjectId("prj-legacy".to_string())).expect("load legacy project");

        let project = loaded.expect("구버전 필드 누락도 파싱에 성공해야 한다");
        assert_eq!(project.last_opened_at, 0.0);
        assert!(warnings.is_empty());

        cleanup(&paths);
    }
}

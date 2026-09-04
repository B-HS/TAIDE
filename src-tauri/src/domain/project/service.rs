use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppErrorKind, AppResult};
use crate::ids::ProjectId;
use crate::infra::clock::now_epoch_ms;
use crate::infra::persist;
use crate::paths::AppPaths;

use super::types::{CapabilityKind, Project, ProjectDisplay, ProjectDisplayPatch, ProjectRef, SessionState};

const BACKUP_SUFFIX: &str = ".bak";
const PROJECTS_DIR_NAME: &str = "projects";
const DISPLAY_ICON_NAME_MAX_BYTES: usize = 64;
const DISPLAY_LABEL_MAX_CODEPOINTS: usize = 4;
/// The color tokens a project display may name, mirroring the theme system's `graph.lane1..lane12`
/// (`src/entities/theme/theme-tokens.ts`, `src/shared/styles/global.css`) — every bundled theme
/// already defines all twelve, so the sidebar can render one as `var(--taide-graph-laneN)` without
/// a new theme token. An explicit allow-list rather than a parsed `lane<N>` range: parsing would
/// also accept `lane01`/`lane+1`, which name no CSS variable.
const DISPLAY_COLOR_TOKENS: &[&str] = &[
    "lane1", "lane2", "lane3", "lane4", "lane5", "lane6", "lane7", "lane8", "lane9", "lane10", "lane11", "lane12",
];

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
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.project.pathNotDirectory",
            format!("path is not a directory: {}", canonical.display()),
        )
        .with_arg("path", canonical.display()));
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

    let history = find_existing_project_record(paths, &root_str)?;
    let id = history.as_ref().map(|project| project.id.clone()).unwrap_or_else(ProjectId::new);
    let display = history.map(|project| project.display).unwrap_or_default();

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
        display,
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

fn display_invalid(field: &str) -> AppError {
    AppError::localized(
        AppErrorKind::InvalidArgument,
        "error.project.displayInvalid",
        format!("invalid project display value for field: {field}"),
    )
}

/// A curated lucide icon name (`src/shared/icons/project-icon-registry.ts`). Only the shape is
/// checked here — an unknown-but-well-formed name is accepted and the frontend registry falls back
/// to the folder icon, so the desktop never has to be rebuilt in lockstep with the registry.
fn sanitize_display_icon(value: &str) -> AppResult<Option<String>> {
    if value.is_empty() {
        return Ok(None);
    }
    let is_icon_name = value.len() <= DISPLAY_ICON_NAME_MAX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    is_icon_name.then(|| Some(value.to_string())).ok_or_else(|| display_invalid("icon"))
}

/// A short text label drawn inside the 40px sidebar button. Control characters are dropped and the
/// result trimmed before the length is measured, so a paste carrying a newline or a stray tab is
/// corrected rather than rejected; a value that is empty once cleaned clears the label, matching
/// the empty-string clear convention instead of persisting an invisible label.
fn sanitize_display_label(value: &str) -> AppResult<Option<String>> {
    let cleaned: String = value.chars().filter(|character| !character.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > DISPLAY_LABEL_MAX_CODEPOINTS {
        return Err(display_invalid("label"));
    }
    Ok(Some(trimmed.to_string()))
}

fn sanitize_display_color(value: &str) -> AppResult<Option<String>> {
    if value.is_empty() {
        return Ok(None);
    }
    DISPLAY_COLOR_TOKENS
        .contains(&value)
        .then(|| Some(value.to_string()))
        .ok_or_else(|| display_invalid("color"))
}

/// Merges one display axis under the settings domain's clearable-string convention (see
/// [`ProjectDisplayPatch`]): an omitted axis keeps `existing` untouched without being re-validated,
/// and a present one goes through `sanitize`, which decides between clearing the axis and replacing
/// it. Rejecting a bad value here (rather than silently dropping it) is what keeps a remote or
/// programmatic caller from persisting a value the sidebar could not render.
fn merge_display_axis(
    patch_value: Option<&String>,
    existing: Option<&String>,
    sanitize: impl FnOnce(&str) -> AppResult<Option<String>>,
) -> AppResult<Option<String>> {
    match patch_value {
        None => Ok(existing.cloned()),
        Some(value) => sanitize(value),
    }
}

fn merge_display(existing: &ProjectDisplay, patch: &ProjectDisplayPatch) -> AppResult<ProjectDisplay> {
    Ok(ProjectDisplay {
        icon: merge_display_axis(patch.icon.as_ref(), existing.icon.as_ref(), sanitize_display_icon)?,
        label: merge_display_axis(patch.label.as_ref(), existing.label.as_ref(), sanitize_display_label)?,
        color: merge_display_axis(patch.color.as_ref(), existing.color.as_ref(), sanitize_display_color)?,
    })
}

/// Applies `patch` to an open project's sidebar presentation and persists the result to both
/// owners: `projects/<id>/project.json` (the source of truth) and `session.json`'s `ProjectRef`
/// mirror, through the same [`upsert_project_ref`] that already mirrors `root`/`name` — writing
/// only one of the two would leave the sidebar (which reads `project_list`) and the project record
/// disagreeing until the next open. The whole patch is validated before anything is assigned, so a
/// rejected axis leaves the project exactly as it was rather than persisting a half-applied change.
pub fn set_project_display(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
    patch: &ProjectDisplayPatch,
) -> AppResult<()> {
    let project = projects
        .get_mut(project_id)
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let merged = merge_display(&project.display, patch)?;
    project.display = merged;
    let updated = project.clone();

    upsert_project_ref(session, &updated);
    save_project(paths, &updated)?;
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
            display: reference.display.clone(),
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
        existing.display = project.display.clone();
    } else {
        session.projects.push(ProjectRef {
            id: project.id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
            display: project.display.clone(),
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

/// Read-only counterpart to `load_project`, for callers that must never touch the filesystem —
/// unlike `load_project` (used by `project_open`, which runs behind `begin_mutation` and may
/// legitimately quarantine a corrupted `project.json` by renaming it to `.bak`), this never
/// writes. A missing file, a read error (e.g. permission denied), or a parse failure are all
/// treated the same way: the record is unavailable, so the caller skips it. Used by
/// `list_recent_projects`, which runs with no mutation guard and must tolerate one damaged or
/// unreadable record without failing the whole listing or racing `project_open`'s own backup
/// rename of the same file.
fn try_load_project_readonly(paths: &AppPaths, id: &ProjectId) -> Option<Project> {
    let raw = std::fs::read(paths.project_file(id)).ok()?;
    serde_json::from_slice::<Project>(&raw).ok()
}

/// Every persisted project record, most-recently-opened first (`Project.last_opened_at`
/// descending) — the Welcome screen's "recent projects" source, distinct from `list_projects`
/// (which only returns the *currently open* session's `ProjectRef`s). `root_missing` is
/// recomputed against the live filesystem the same way `restore_session` does, so a project whose
/// folder moved or was deleted since it was last opened still lists (disabled, per contract §1.2)
/// instead of silently vanishing. A record that fails to read or parse is skipped rather than
/// failing the whole listing (`try_load_project_readonly`'s per-entry tolerance) — and, unlike
/// `load_project`, never backs it up to `.bak`, since this listing has no `begin_mutation` guard.
pub fn list_recent_projects(paths: &AppPaths) -> AppResult<Vec<Project>> {
    let mut projects = Vec::new();
    for id in iter_project_ids(paths)? {
        if let Some(mut project) = try_load_project_readonly(paths, &id) {
            project.root_missing = !Path::new(&project.root).is_dir();
            projects.push(project);
        }
    }

    projects.sort_by(|a, b| b.last_opened_at.total_cmp(&a.last_opened_at));
    Ok(projects)
}

/// The persisted record of a previously-opened project at `root`, if any — `open_project` reuses
/// both its id (so `projects/<id>/` and its layout survive a close/re-open cycle) and the parts of
/// it the user authored rather than the filesystem: currently `display`. Returning the whole
/// record instead of just the id is what keeps a re-open from silently resetting a project's
/// sidebar icon/label/color, since `open_project`'s fresh-open branch builds a brand-new `Project`
/// value even when it is reusing a known id.
fn find_existing_project_record(paths: &AppPaths, root: &str) -> AppResult<Option<Project>> {
    for id in iter_project_ids(paths)? {
        let (loaded, _warnings) = load_project(paths, &id)?;
        if let Some(project) = loaded {
            if project.root == root {
                return Ok(Some(project));
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
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &history).expect("seed history");

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");
        assert_eq!(opened.project.id, previous_id);
        assert!(!opened.already_open);

        cleanup(&paths);
    }

    /// Locks in the precondition `commands::project_open`'s `ProjectActivated` emit relies on
    /// (`docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md` §3, item c): `open_project` sets
    /// `session.active_project` on *every* path, not only a first-time open, so the command must
    /// fan that activation out unconditionally rather than only inside its `!already_open` branch.
    #[test]
    fn open_project은_already_open_여부와_무관하게_session_active_project를_대상_프로젝트로_설정한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("open");
        assert!(!first.already_open);
        assert_eq!(session.active_project, Some(first.project.id.clone()));

        session.active_project = None;

        let second = open_project(&paths, &mut session, &mut projects, &project_root, detect_terminal_only).expect("re-open");
        assert!(second.already_open);
        assert_eq!(session.active_project, Some(second.project.id));

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
                    display: ProjectDisplay::default(),
                },
                ProjectRef {
                    id: ProjectId("prj-b".to_string()),
                    root: "/b".to_string(),
                    name: "b".to_string(),
                    display: ProjectDisplay::default(),
                },
                ProjectRef {
                    id: ProjectId("prj-c".to_string()),
                    root: "/c".to_string(),
                    name: "c".to_string(),
                    display: ProjectDisplay::default(),
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
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &project).expect("save project");

        let mut session = SessionState::default();
        session.projects.push(ProjectRef {
            id: id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
            display: ProjectDisplay::default(),
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
            display: ProjectDisplay::default(),
        };
        let newer = Project {
            id: ProjectId("prj-newer".to_string()),
            root: "/newer".to_string(),
            name: "newer".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 2_000.0,
            display: ProjectDisplay::default(),
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
            display: ProjectDisplay::default(),
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
    fn list_recent_projects은_손상된_기록을_건너뛰고_bak으로_rename하지_않는다() {
        let paths = temp_paths();

        let healthy = Project {
            id: ProjectId("prj-healthy".to_string()),
            root: "/healthy".to_string(),
            name: "healthy".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 1_000.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &healthy).expect("seed healthy");

        let corrupted_dir = paths.data_dir.join("projects").join("prj-corrupted");
        std::fs::create_dir_all(&corrupted_dir).unwrap();
        let corrupted_path = corrupted_dir.join("project.json");
        std::fs::write(&corrupted_path, b"not json").unwrap();

        let recent = list_recent_projects(&paths).expect("list recent");

        assert_eq!(
            recent.iter().map(|project| project.id.as_str()).collect::<Vec<_>>(),
            vec!["prj-healthy"],
            "손상된 기록은 건너뛰고 나머지는 목록에 남아야 한다"
        );
        assert!(
            corrupted_path.exists(),
            "읽기 전용 조회는 손상된 project.json 을 .bak 으로 rename 해서는 안 된다"
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

    fn open_workspace(paths: &AppPaths, session: &mut SessionState, projects: &mut HashMap<ProjectId, Project>) -> ProjectId {
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();
        open_project(paths, session, projects, &project_root, detect_terminal_only)
            .expect("open")
            .project
            .id
    }

    fn patch_of(icon: Option<&str>, label: Option<&str>, color: Option<&str>) -> ProjectDisplayPatch {
        ProjectDisplayPatch {
            icon: icon.map(str::to_string),
            label: label.map(str::to_string),
            color: color.map(str::to_string),
        }
    }

    #[test]
    fn merge_display은_아이콘_이름_규격만_통과시킨다() {
        let empty = ProjectDisplay::default();
        let too_long = "a".repeat(DISPLAY_ICON_NAME_MAX_BYTES + 1);

        assert_eq!(
            merge_display(&empty, &patch_of(Some("square-code-2"), None, None))
                .expect("허용 형식")
                .icon,
            Some("square-code-2".to_string())
        );

        for rejected in ["Rocket", "rocket!", "아이콘", "ro ket", too_long.as_str()] {
            assert_eq!(
                merge_display(&empty, &patch_of(Some(rejected), None, None))
                    .expect_err("규격 밖 아이콘 이름은 거부돼야 한다")
                    .kind(),
                AppErrorKind::InvalidArgument,
                "거부 대상: {rejected}"
            );
        }
    }

    #[test]
    fn merge_display은_아이콘_이름_최대_바이트_경계를_허용한다() {
        let empty = ProjectDisplay::default();
        let at_limit = "a".repeat(DISPLAY_ICON_NAME_MAX_BYTES);

        assert_eq!(
            merge_display(&empty, &patch_of(Some(at_limit.as_str()), None, None))
                .expect("상한과 같은 길이는 허용")
                .icon,
            Some(at_limit)
        );
    }

    #[test]
    fn merge_display은_라벨_길이를_바이트가_아니라_코드포인트로_센다() {
        let empty = ProjectDisplay::default();
        let four_emoji = "🚀🚀🚀🚀";
        assert!(
            four_emoji.len() > DISPLAY_LABEL_MAX_CODEPOINTS,
            "테스트 전제: UTF-8 바이트 수가 상한을 넘어야 한다"
        );

        assert_eq!(
            merge_display(&empty, &patch_of(None, Some(four_emoji), None))
                .expect("코드포인트 4개는 허용")
                .label,
            Some(four_emoji.to_string())
        );
    }

    #[test]
    fn merge_display은_제어문자만_있는_라벨을_해제로_해석한다() {
        let existing = ProjectDisplay {
            icon: None,
            label: Some("TA".to_string()),
            color: None,
        };

        assert_eq!(
            merge_display(&existing, &patch_of(None, Some("\n\t\u{1b}"), None))
                .expect("정리 후 빈 라벨")
                .label,
            None
        );
    }

    #[test]
    fn merge_display은_세_축을_한_번에_해제한다() {
        let existing = ProjectDisplay {
            icon: Some("rocket".to_string()),
            label: Some("TA".to_string()),
            color: Some("lane5".to_string()),
        };

        assert_eq!(
            merge_display(&existing, &patch_of(Some(""), Some(""), Some(""))).expect("전부 해제"),
            ProjectDisplay::default(),
            "빈 문자열 3개는 기본 표시로 되돌려야 한다"
        );
    }

    #[test]
    fn merge_display은_라벨을_정리한_뒤_길이를_판정한다() {
        let empty = ProjectDisplay::default();

        assert_eq!(
            merge_display(&empty, &patch_of(None, Some("  가나\t다라  "), None))
                .expect("정리 후 4자")
                .label,
            Some("가나다라".to_string()),
            "제어문자 제거·trim 이후의 코드포인트 수로 판정해야 한다"
        );
        assert_eq!(
            merge_display(&empty, &patch_of(None, Some("   "), None))
                .expect("공백만 남는 라벨")
                .label,
            None,
            "정리 후 비는 라벨은 거부가 아니라 해제로 해석한다"
        );
        assert_eq!(
            merge_display(&empty, &patch_of(None, Some("가나다라마"), None))
                .expect_err("최대 코드포인트 초과는 거부돼야 한다")
                .kind(),
            AppErrorKind::InvalidArgument
        );
    }

    #[test]
    fn merge_display은_lane_색_토큰_목록만_통과시킨다() {
        let empty = ProjectDisplay::default();

        for accepted in ["lane1", "lane12"] {
            assert_eq!(
                merge_display(&empty, &patch_of(None, None, Some(accepted)))
                    .expect("허용 토큰")
                    .color,
                Some(accepted.to_string())
            );
        }

        for rejected in ["lane0", "lane13", "lane01", "lane+1", "red", "--taide-graph-lane1"] {
            assert_eq!(
                merge_display(&empty, &patch_of(None, None, Some(rejected)))
                    .expect_err("목록 밖 색 토큰은 거부돼야 한다")
                    .kind(),
                AppErrorKind::InvalidArgument,
                "거부 대상: {rejected}"
            );
        }
    }

    #[test]
    fn merge_display의_빈_문자열은_해제하고_미지정_축은_유지한다() {
        let existing = ProjectDisplay {
            icon: Some("rocket".to_string()),
            label: Some("TA".to_string()),
            color: Some("lane5".to_string()),
        };

        let cleared = merge_display(&existing, &patch_of(Some(""), None, None)).expect("아이콘만 해제");
        assert_eq!(cleared.icon, None);
        assert_eq!(cleared.label, existing.label);
        assert_eq!(cleared.color, existing.color);

        assert_eq!(
            merge_display(&existing, &ProjectDisplayPatch::default()).expect("빈 패치"),
            existing,
            "미지정 패치는 기존 값을 그대로 유지해야 한다"
        );
    }

    #[test]
    fn set_project_display은_project_json과_session_json에_같은_값을_남긴다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("database"), Some(" TA "), Some("lane7")),
        )
        .expect("set display");

        let expected = ProjectDisplay {
            icon: Some("database".to_string()),
            label: Some("TA".to_string()),
            color: Some("lane7".to_string()),
        };
        assert_eq!(projects.get(&id).expect("in-memory project").display, expected);

        let (persisted, _warnings) = load_project(&paths, &id).expect("load project.json");
        assert_eq!(persisted.expect("project.json").display, expected);

        let (reloaded, _warnings) = load_session(&paths).expect("load session.json");
        assert_eq!(
            reloaded
                .projects
                .iter()
                .find(|reference| reference.id == id)
                .expect("session ref")
                .display,
            expected,
            "session.json 의 ProjectRef 미러도 재로드 후 같은 값이어야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_거부된_패치를_부분_적용하지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(&paths, &mut session, &mut projects, &id, &patch_of(Some("rocket"), None, None)).expect("set display");

        let error = set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("database"), Some("다섯글자라벨"), None),
        )
        .expect_err("라벨이 규격 밖이면 전체가 거부돼야 한다");

        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(
            projects.get(&id).expect("in-memory project").display,
            ProjectDisplay {
                icon: Some("rocket".to_string()),
                label: None,
                color: None,
            },
            "거부된 패치는 같은 호출의 유효한 축도 적용하지 않아야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_열려_있지_않은_프로젝트면_notfound_이고_아무것도_저장하지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let missing = ProjectId("prj-missing".to_string());

        let error = set_project_display(&paths, &mut session, &mut projects, &missing, &patch_of(Some("rocket"), None, None))
            .expect_err("열려 있지 않은 프로젝트");

        assert_eq!(error.kind(), AppErrorKind::NotFound);
        assert!(session.projects.is_empty(), "세션 미러가 생기면 안 된다");
        assert!(!paths.session_file().exists(), "session.json 을 쓰면 안 된다");

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_해제_패치를_양쪽_파일에_미러한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("rocket"), Some("TA"), Some("lane2")),
        )
        .expect("set display");
        set_project_display(&paths, &mut session, &mut projects, &id, &patch_of(Some(""), Some(""), Some(""))).expect("clear display");

        let (persisted, _warnings) = load_project(&paths, &id).expect("load project.json");
        assert_eq!(persisted.expect("project.json").display, ProjectDisplay::default());

        let (reloaded, _warnings) = load_session(&paths).expect("load session.json");
        assert_eq!(
            reloaded
                .projects
                .iter()
                .find(|reference| reference.id == id)
                .expect("session ref")
                .display,
            ProjectDisplay::default(),
            "해제도 저장과 같은 경로로 session.json 에 미러돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_root와_name_미러를_건드리지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);
        let before = session
            .projects
            .iter()
            .find(|reference| reference.id == id)
            .expect("session ref")
            .clone();

        set_project_display(&paths, &mut session, &mut projects, &id, &patch_of(None, Some("TA"), None)).expect("set display");

        let after = session.projects.iter().find(|reference| reference.id == id).expect("session ref");
        assert_eq!(after.root, before.root);
        assert_eq!(after.name, before.name);
        assert_eq!(session.projects.len(), 1, "미러 갱신이 항목을 늘리면 안 된다");

        cleanup(&paths);
    }

    #[test]
    fn 닫았다_다시_연_프로젝트는_표시_설정을_유지한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("database"), None, Some("lane3")),
        )
        .expect("set display");
        close_project(&paths, &mut session, &mut projects, &id).expect("close");

        let reopened = open_workspace(&paths, &mut session, &mut projects);

        assert_eq!(reopened, id, "같은 루트는 기존 id 를 재사용해야 한다");
        assert_eq!(
            projects.get(&id).expect("재열기한 프로젝트").display,
            ProjectDisplay {
                icon: Some("database".to_string()),
                label: None,
                color: Some("lane3".to_string()),
            },
            "재열기가 사용자 지정 표시를 초기화해서는 안 된다"
        );
        assert_eq!(
            session
                .projects
                .iter()
                .find(|reference| reference.id == id)
                .expect("session ref")
                .display,
            projects.get(&id).expect("재열기한 프로젝트").display,
            "재열기 후에도 session.json 미러가 project.json 과 같아야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn display_필드가_없는_구버전_기록은_기본_표시로_파싱된다() {
        let paths = temp_paths();
        std::fs::create_dir_all(paths.data_dir.join("projects").join("prj-legacy")).unwrap();
        std::fs::write(
            paths.data_dir.join("projects").join("prj-legacy").join("project.json"),
            serde_json::to_vec(&serde_json::json!({ "id": "prj-legacy", "root": "/legacy", "name": "legacy" })).unwrap(),
        )
        .unwrap();
        std::fs::write(
            paths.session_file(),
            serde_json::to_vec(&serde_json::json!({
                "version": crate::domain::project::types::SESSION_SCHEMA_VERSION,
                "projects": [{ "id": "prj-legacy", "root": "/legacy", "name": "legacy" }],
                "activeProject": null,
            }))
            .unwrap(),
        )
        .unwrap();

        let (loaded, project_warnings) = load_project(&paths, &ProjectId("prj-legacy".to_string())).expect("load legacy project");
        assert_eq!(loaded.expect("project.json").display, ProjectDisplay::default());
        assert!(project_warnings.is_empty());

        let (session, session_warnings) = load_session(&paths).expect("load legacy session");
        assert!(session_warnings.is_empty(), "display 가 없다고 세션이 손상 처리되면 안 된다");
        assert_eq!(session.projects[0].display, ProjectDisplay::default());

        cleanup(&paths);
    }
}

use std::path::Path;

use tauri::AppHandle;
use tauri_specta::Event;

use crate::domain::file::types::FsChange;
use crate::events::{GitRefsChanged, GitStatusChanged};
use crate::ids::ProjectId;
use crate::infra::watcher;
use crate::state::AppState;

pub(super) const GIT_DIR_NAME: &str = ".git";
const GIT_INDEX_FILE: &str = "index";
const GIT_HEAD_FILE: &str = "HEAD";
const GIT_REFS_DIR: &str = "refs";
const GIT_OBJECTS_DIR: &str = "objects";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitInvalidation {
    Status,
    Refs,
}

pub fn classify_git_change(path: &Path) -> Option<GitInvalidation> {
    let components: Vec<&str> = path.components().filter_map(|component| component.as_os_str().to_str()).collect();
    let git_dir_index = components.iter().rposition(|&component| component == GIT_DIR_NAME)?;
    let inside_git_dir = &components[git_dir_index + 1..];

    match inside_git_dir {
        [GIT_OBJECTS_DIR, ..] => None,
        [GIT_INDEX_FILE] => Some(GitInvalidation::Status),
        [GIT_HEAD_FILE] => Some(GitInvalidation::Refs),
        [GIT_REFS_DIR, ..] => Some(GitInvalidation::Refs),
        _ => None,
    }
}

/// Boot-restore entry point: probes the live filesystem for a `.git` directory and attaches
/// nothing when it is absent, then delegates to [`register_git_watcher`]. The probe lives here —
/// not in the shared registration body — because `lib.rs`'s restore loop runs before the
/// capability registry is even managed and must not trust a persisted capability list; on the
/// `project_open` path the same predicate is evaluated exactly once instead, by
/// `GitWatcherCapability::detected_kind`, whose result gates `GitWatcherCapability::attach`.
pub fn attach_git_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
    if !Path::new(root).join(GIT_DIR_NAME).is_dir() {
        return;
    }
    register_git_watcher(app, state, project_id, root);
}

/// Starts the `.git`-directory watcher that classifies raw fs changes into status/refs
/// invalidations and fans them out as [`GitStatusChanged`]/[`GitRefsChanged`], registering its
/// handle in `state.git_watchers`. Performs no repo check of its own — the caller owns that
/// decision (`GitWatcherCapability::attach`'s `Project.capabilities` gate on the open path,
/// [`attach_git_watcher`]'s filesystem probe on the boot restore path).
pub(super) fn register_git_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
    let git_dir = Path::new(root).join(GIT_DIR_NAME);
    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(git_dir.clone(), move |changes: Vec<FsChange>| {
        let mut needs_status = false;
        let mut needs_refs = false;

        for change in &changes {
            for path in &change.paths {
                match classify_git_change(Path::new(path)) {
                    Some(GitInvalidation::Status) => needs_status = true,
                    Some(GitInvalidation::Refs) => needs_refs = true,
                    None => {}
                }
            }
        }

        if needs_status {
            let _ = GitStatusChanged {
                project_id: emit_project.clone(),
            }
            .emit(&emit_handle);
        }
        if needs_refs {
            let _ = GitRefsChanged {
                project_id: emit_project.clone(),
            }
            .emit(&emit_handle);
        }
    }) {
        Ok(handle) => {
            state.git_watchers.write().insert(project_id.clone(), handle);
        }
        Err(error) => log::warn!("git 감시를 시작하지 못했습니다 ({}): {error}", git_dir.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_변경은_status_무효화다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/index")), Some(GitInvalidation::Status));
    }

    #[test]
    fn head_변경은_refs_무효화다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/HEAD")), Some(GitInvalidation::Refs));
    }

    #[test]
    fn refs_하위_변경은_refs_무효화다() {
        assert_eq!(
            classify_git_change(Path::new("/repo/.git/refs/heads/main")),
            Some(GitInvalidation::Refs)
        );
        assert_eq!(
            classify_git_change(Path::new("/repo/.git/refs/remotes/origin/main")),
            Some(GitInvalidation::Refs)
        );
    }

    #[test]
    fn objects_하위_변경은_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/objects/ab/cd1234")), None);
        assert_eq!(classify_git_change(Path::new("/repo/.git/objects/pack/pack-abc.idx")), None);
    }

    #[test]
    fn 알수없는_git_내부_경로는_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git/logs/HEAD")), None);
        assert_eq!(classify_git_change(Path::new("/repo/.git/COMMIT_EDITMSG")), None);
    }

    #[test]
    fn git_디렉토리_밖의_경로는_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/src/main.rs")), None);
    }

    #[test]
    fn git_디렉토리_자체는_무시한다() {
        assert_eq!(classify_git_change(Path::new("/repo/.git")), None);
    }

    #[test]
    fn 경로에_git_디렉토리가_여러번_등장하면_가장_안쪽_기준으로_분류한다() {
        assert_eq!(
            classify_git_change(Path::new("/workspace/.git/nested-repo/.git/refs/heads/main")),
            Some(GitInvalidation::Refs)
        );
    }
}

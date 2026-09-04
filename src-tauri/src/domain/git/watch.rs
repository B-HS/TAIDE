use std::path::Path;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::domain::file::types::FsChange;
use crate::events::{GitRefsChanged, GitStatusChanged};
use crate::ids::ProjectId;
use crate::infra::watcher;
use crate::state::AppState;

use super::commands::GitStore;

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

/// The one `AppState` write [`build_git_watcher_handle`]'s callers need. Split out so both attach
/// paths (`project_open`'s `GitWatcherCapability::build_attachment` and the boot restore path
/// `domain::project::commands::restore_project_watchers`) can build the handle — the expensive
/// `FileIdMap` walk — with no `AppState` access at all, and only reach for
/// `AppState::begin_mutation` for this insert.
pub fn register_git_watcher_handle(state: &AppState, project_id: &ProjectId, handle: watcher::WatcherHandle) {
    state.git_watchers.write().insert(project_id.clone(), handle);
}

/// The shared attach entry point for both paths: probes the live filesystem for a `.git` directory
/// and builds (but does not register — see [`register_git_watcher_handle`]) the watcher that
/// classifies raw fs changes into status/refs invalidations, drops `GitStore`'s cached status for
/// the project, and fans the change out as [`GitStatusChanged`]/[`GitRefsChanged`]. Returns `None`
/// for a non-repo root or a failed watcher start, logging the warning in the latter case.
///
/// The cache drop happens **before** either emit, and covers a refs-only change as well as a status
/// one, because `GitStatus` carries `branch`/`ahead`/`behind` — see
/// [`GitStore::invalidate_status`](super::commands::GitStore::invalidate_status) for why this direct
/// call exists alongside the subscription that also sees these events.
///
/// The probe is the *whole* decision on the boot restore path, because a project's
/// `Project.capabilities` was persisted the last time it was open and can't be trusted across
/// whatever happened to `root`'s `.git` directory while the app was closed (it may have been
/// `git init`'d or deleted since). On the `project_open` path `GitWatcherCapability::detected_kind`
/// has already decided fresh, moments earlier in the same command, and gates the call — there the
/// probe is a cheap re-check that avoids watching a directory removed in between.
///
/// Touches no `AppState`, so both callers run it with no `AppState::begin_mutation` held.
pub fn build_git_watcher_handle(app: &AppHandle, project_id: &ProjectId, root: &str) -> Option<watcher::WatcherHandle> {
    if !Path::new(root).join(GIT_DIR_NAME).is_dir() {
        return None;
    }
    build_watcher_handle_inner(app, project_id, root)
}

fn build_watcher_handle_inner(app: &AppHandle, project_id: &ProjectId, root: &str) -> Option<watcher::WatcherHandle> {
    let git_dir = Path::new(root).join(GIT_DIR_NAME);
    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(git_dir.clone(), watcher::WatchScope::GitDir, move |changes: Vec<FsChange>| {
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

        if needs_status || needs_refs {
            emit_handle.state::<GitStore>().invalidate_status(&emit_project);
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
        Ok(handle) => Some(handle),
        Err(error) => {
            log::warn!("git 감시를 시작하지 못했습니다 ({}): {error}", git_dir.display());
            None
        }
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

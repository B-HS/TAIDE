use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use super::service;
use super::types::{DropEdge, OpenTabInSplitRequest, ProjectLayout, ShellViewPatch, Tab, TabKind, TabPathChange, TabPathChangeResult};
use crate::domain::project::types::Project;
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::ids::{PaneId, ProjectId, TabId};
use crate::infra::root_guard;
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn layout_get(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    state
        .layouts
        .read()
        .get(&project_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("layout not found: {project_id}")))
}

/// Rejects a `File` tab before it exists, when its path is outside every open project's root or is
/// no longer a file on disk. The command palette's quick-open hands this command a path out of a
/// file index that can be stale (walked once, then the file was renamed or deleted), and without
/// this gate the tab opened fine and only failed afterwards inside the editor pane's own
/// `file_open` — leaving an unusable tab whose body showed a raw `os error 2`. The check lives at
/// the command boundary rather than in [`service::open_tab`] on purpose: the layout service is a
/// pure in-memory tree, and its tests open tabs for paths that never existed. A boundary rejection
/// stays `root_guard`'s `Forbidden`; a missing file raises `root_guard::ensure_existing_file`'s
/// `error.file.notFound`, the same error `file::service::open_file` raises, so the message reads
/// identically no matter which entry point reported it. That existence half deliberately lives in
/// infra rather than here: the IDE MCP `openFile` tool cannot call this command function across the
/// domain boundary and reaches [`service::open_tab_and_finish`] directly, so it runs the shared
/// gate itself — see `root_guard::ensure_existing_file`. The boundary half is
/// `root_guard::resolve_owning_project_or_cli_opened`'s, not the plain resolver's: a file the user
/// handed to the `taide` CLI (Claude Code's Ctrl+G temp file, outside every root) may become a tab
/// of whichever project the frontend chose for it, while the IDE MCP tool keeps the strict check.
///
/// A refusal is logged at `warn` with which of the two gates rejected it, because this is the one
/// quick-open failure a user reports as "the palette just does nothing" — the frontend answers it
/// with a toast and drops the path, leaving no record of *why* the index and the disk disagreed.
/// Only the path is logged; it carries no secret the app does not already print in tab titles. See
/// `docs/debugging.md` §4.
fn ensure_file_tab_target_exists(
    projects: &HashMap<ProjectId, Project>,
    cli_opened_paths: &HashSet<PathBuf>,
    kind: &TabKind,
) -> AppResult<()> {
    let TabKind::File { path } = kind else {
        return Ok(());
    };

    let result = root_guard::resolve_owning_project_or_cli_opened(projects, cli_opened_paths, Path::new(path))
        .and_then(|(_, resolved)| root_guard::ensure_existing_file(&resolved, path));

    if let Err(error) = &result {
        let reason = match error.kind() {
            AppErrorKind::Forbidden => "프로젝트 경계 밖",
            _ => "파일 부재",
        };
        log::warn!("파일 탭 열기 선검증 실패 (path={path}, 사유={reason}): {error}");
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn layout_open_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    kind: TabKind,
    title: String,
    target: Option<PaneId>,
    preview: bool,
) -> AppResult<ProjectLayout> {
    let projects = state.projects.read().clone();
    ensure_file_tab_target_exists(&projects, &state.cli_opened_paths.read(), &kind)?;

    service::open_tab_and_finish(&app, &state, project_id, kind, title, target, preview).await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_close_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    let (_, _, updated) = service::close_tab_and_finish(&app, &state, &tab_id).await?;
    Ok(updated)
}

/// Where a layout-mutation command finds the `ProjectLayout` it's about to mutate — the three
/// lookup shapes the 13 skeleton-identical commands below used inline before being unified into
/// [`run_layout_mutation`]: resolve the owning project from a tab id, from a pane id, or (when the
/// caller already names the project directly) skip lookup entirely.
enum LayoutLocate {
    WithTab(TabId),
    WithPane(PaneId),
    Direct(ProjectId),
}

async fn run_layout_mutation<F>(app: &AppHandle, state: &State<'_, AppState>, locate: LayoutLocate, mutate: F) -> AppResult<ProjectLayout>
where
    F: FnOnce(&mut ProjectLayout) -> AppResult<()>,
{
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = match locate {
        LayoutLocate::WithTab(tab_id) => service::locate_project_with_tab(&layouts, &tab_id)?,
        LayoutLocate::WithPane(pane_id) => service::locate_project_with_pane(&layouts, &pane_id)?,
        LayoutLocate::Direct(project_id) => project_id,
    };
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    mutate(layout)?;

    let updated = service::finish_mutation(&TauriEventSink(app), state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn layout_activate_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::activate_tab(layout, &tab_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_move_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    pane_id: PaneId,
    index: u32,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::move_tab(layout, &tab_id, &pane_id, index as usize)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_split(
    app: AppHandle,
    state: State<'_, AppState>,
    pane_id: PaneId,
    edge: DropEdge,
    tab_id: TabId,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::split(layout, &pane_id, edge, &tab_id)
    })
    .await
}

/// Opens a *new* tab in a *new* pane beside `target_pane` — what the terminal's context menu calls
/// "split", and what `layout_split` is not: that one moves a tab that already exists. Doing this in
/// one command rather than `layout_open_tab` + `layout_split` is a correctness requirement, not a
/// round-trip saving: `layout_open_tab` activates the new tab in the *source* pane first, which
/// unmounts the terminal the user is looking at (replaying its whole ring buffer) and then, once
/// the split remounts that tab elsewhere, spawns its shell a second time. One mutation guard and
/// one `layout:changed` make that sequence unrepresentable. See
/// `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §F.2 and the `open_tab_in_split`
/// service function.
///
/// `request.edge` must be directional: `DropEdge::Center` means "into the target pane", which is
/// [`layout_open_tab`]'s job, so it is rejected as `InvalidArgument` rather than silently treated
/// as one. `File` kinds run the same pre-flight [`ensure_file_tab_target_exists`] gate
/// `layout_open_tab` runs, for the same reason: a tab must never outlive the path it was opened for.
#[tauri::command]
#[specta::specta]
pub async fn layout_open_tab_in_split(
    app: AppHandle,
    state: State<'_, AppState>,
    request: OpenTabInSplitRequest,
) -> AppResult<ProjectLayout> {
    let OpenTabInSplitRequest {
        project_id,
        target_pane,
        edge,
        kind,
        title,
        preview,
    } = request;

    let projects = state.projects.read().clone();
    ensure_file_tab_target_exists(&projects, &state.cli_opened_paths.read(), &kind)?;

    let preview = preview && state.settings.read().enable_preview_tabs;
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
        let tab = Tab {
            id: TabId::new(),
            kind,
            title,
            pinned: false,
            preview,
            dirty: false,
            view_state: None,
        };
        service::open_tab_in_split(layout, &target_pane, edge, tab)?;
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_resize(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId, sizes: Vec<f32>) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::resize(layout, &pane_id, sizes)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_focus_pane(app: AppHandle, state: State<'_, AppState>, pane_id: PaneId) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::focus_pane(layout, &pane_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_pin_tab(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, pinned: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::pin_tab(layout, &tab_id, pinned)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_preview(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, preview: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_preview(layout, &tab_id, preview)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_reopen_closed(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
        service::reopen_closed(layout);
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_view_state(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    view_state: Option<String>,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_view_state(layout, &tab_id, view_state)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_dirty(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, dirty: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_dirty(layout, &tab_id, dirty)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_terminal_session(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: TabId,
    session_id: String,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_terminal_session(layout, &tab_id, session_id)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_open_untitled(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    target: Option<PaneId>,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
        let pane_id = target.unwrap_or_else(|| layout.focused_pane.clone());
        let index = service::next_untitled_index(layout);
        let tab = Tab {
            id: TabId::new(),
            kind: TabKind::Untitled { index },
            title: format!("Untitled-{index}"),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        };
        service::open_tab(layout, &pane_id, tab, false)?;
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn layout_convert_untitled(app: AppHandle, state: State<'_, AppState>, tab_id: TabId, path: String) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(&path))?;
    let title = resolved
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| AppError::InvalidArgument(format!("invalid path: {path}")))?;

    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    service::convert_untitled_to_file(layout, &tab_id, resolved.to_string_lossy().into_owned(), title)?;

    let updated = service::finish_mutation(&TauriEventSink(&app), &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

/// Rejects a [`TabPathChange`] naming anything outside `root`, by path components — the same
/// containment rule `service::retargeted_path`/`is_same_or_under` match tabs with, so a path this
/// accepts is exactly a path the matcher can act on.
///
/// Deliberately a string containment check rather than `root_guard::ensure_within_root`: the file
/// command this follows has already completed, so `from` (and a deleted `path`) is gone from disk
/// and cannot be canonicalized against anything. The project root itself is canonical (recorded that
/// way when the project is opened — `domain::project::service`), and tab paths are built by walking
/// it, so both sides of this comparison come from the same canonical prefix.
fn ensure_change_within_root(root: &Path, change: &TabPathChange) -> AppResult<()> {
    let paths: &[&String] = match change {
        TabPathChange::Renamed { from, to } => &[from, to],
        TabPathChange::Deleted { path } => &[path],
    };
    for path in paths {
        if !Path::new(path.as_str()).starts_with(root) {
            return Err(AppError::localized(
                AppErrorKind::Forbidden,
                "error.path.outsideProjectRoot",
                format!("path is outside the project root: {path}"),
            )
            .with_arg("path", path.as_str()));
        }
    }
    Ok(())
}

/// Makes open tabs follow a path change the file domain has already committed to disk — a rename
/// repoints every `TabKind::File` tab under the old path (a directory rename moves the whole
/// subtree), a delete closes them. Audit §4-B A3: nothing used to update a tab's stored path, so a
/// renamed file's tab kept pointing at a path that no longer exists (`⌘S` then recreated the old
/// directory through `write_atomic`'s `create_dir_all` and forked the file), and reopening the file
/// under its new name produced a second tab for the same document.
///
/// Called by the frontend *after* `file_rename`/`file_delete` succeeds rather than from inside those
/// commands: the file domain owns paths on disk, not the tab bar, and wiring it here would make
/// `domain::file` depend on `domain::layout` (the same domain-coupling boundary
/// `docs/architecture.md` §2 keeps between `file` and `tree`, where the watcher event — not the file
/// command — drives the tree's own cleanup).
///
/// Reports what changed (`moved`/`closed_paths`) so the frontend can move the per-path state only it
/// owns — monaco model, hot-exit mirror, `FILE.CONTENT` cache, "reopen with" override. When no *open*
/// tab addressed the changed path there is nothing to report, and no revision bump or
/// `layout:changed` event is produced: nothing on screen moved, so making every window refetch the
/// layout would be pure waste.
///
/// A rename with nothing to report can still have changed the layout, though — the closed-tab stack
/// follows a rename silently so "Reopen Closed Tab" cannot resurrect a path that no longer exists —
/// which is why the store write-back is gated on `TabPathChangeOutcome::layout_changed` rather than
/// on there being something to report. Skipping it there (the original shape of this early return)
/// discarded that rewrite along with the whole working clone.
///
/// `from`/`to`/`path` are checked against the project's own root before anything is matched. They are
/// *not* canonicalized (the rename is already done, so `from` no longer exists on disk — see contract
/// §3 S8), but they must still be inside the project: `Deleted { path: "/" }` is "at or under" every
/// absolute path, which would close every file tab in the project and let the frontend's
/// `releaseClosedFileTabPath` discard each one's hot-exit mirror — i.e. every unsaved draft.
#[tauri::command]
#[specta::specta]
pub async fn layout_apply_path_change(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    change: TabPathChange,
) -> AppResult<TabPathChangeResult> {
    let projects = state.projects.read().clone();
    let root = root_guard::project_root(&projects, &project_id)?;
    ensure_change_within_root(&root, &change)?;

    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    let outcome = service::apply_tab_path_change(layout, &change);
    if outcome.is_empty() {
        let snapshot = layout.clone();
        if outcome.layout_changed {
            state.dirty_layouts.write().insert(project_id.clone());
            *state.layouts.write() = layouts;
        }
        return Ok(TabPathChangeResult {
            layout: snapshot,
            moved: outcome.moved,
            closed_paths: outcome.closed_paths,
        });
    }

    let updated = service::finish_mutation(&TauriEventSink(&app), &state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(TabPathChangeResult {
        layout: updated,
        moved: outcome.moved,
        closed_paths: outcome.closed_paths,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn layout_set_shell_view(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    patch: ShellViewPatch,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(&app, &state, LayoutLocate::Direct(project_id), move |layout| {
        service::apply_shell_view_patch(layout, &patch);
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 개명(from: &str, to: &str) -> TabPathChange {
        TabPathChange::Renamed {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn 프로젝트_루트_안의_경로_변경은_통과한다() {
        let root = Path::new("/repo");

        assert!(ensure_change_within_root(root, &개명("/repo/a.txt", "/repo/b.txt")).is_ok());
        assert!(ensure_change_within_root(
            root,
            &TabPathChange::Deleted {
                path: "/repo/src/nested".to_string()
            }
        )
        .is_ok());
        assert!(
            ensure_change_within_root(root, &개명("/repo", "/repo")).is_ok(),
            "루트 자신은 루트 하위다"
        );
    }

    #[test]
    fn 루트_밖_경로와_슬래시는_거절된다() {
        let root = Path::new("/repo");

        assert!(
            ensure_change_within_root(root, &TabPathChange::Deleted { path: "/".to_string() }).is_err(),
            "'/' 는 모든 절대 경로의 조상이라 프로젝트의 모든 파일 탭을 닫고 미저장 미러를 지운다"
        );
        assert!(ensure_change_within_root(root, &개명("/repo/a.txt", "/elsewhere/b.txt")).is_err());
        assert!(ensure_change_within_root(root, &개명("/elsewhere/a.txt", "/repo/b.txt")).is_err());
        assert!(
            ensure_change_within_root(root, &개명("/repo-old/a.txt", "/repo-old/b.txt")).is_err(),
            "성분 단위 비교라 이름이 접두사인 형제 디렉토리는 루트 안이 아니다"
        );
    }

    fn 열린_프로젝트(root: &Path) -> HashMap<ProjectId, Project> {
        let project_id = ProjectId::new();
        HashMap::from([(
            project_id.clone(),
            Project {
                id: project_id,
                root: root.to_string_lossy().into_owned(),
                name: "layout-cmd-test".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        )])
    }

    fn 임시_루트(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("taide-layout-cmd-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn 파일_탭_종류(path: &Path) -> TabKind {
        TabKind::File {
            path: path.to_string_lossy().into_owned(),
        }
    }

    #[test]
    fn 존재하는_파일_탭은_열기_선검증을_통과한다() {
        let root = 임시_루트("existing");
        let file = root.join("a.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();

        assert!(ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &파일_탭_종류(&file)).is_ok());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 사라진_파일_탭은_로케일_not_found_로_거절된다() {
        let root = 임시_루트("missing");
        let stale = root.join("deleted.rs");

        let error = ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &파일_탭_종류(&stale))
            .expect_err("스테일 인덱스가 넘긴 사라진 경로는 탭을 열기 전에 거절되어야 한다");

        assert_eq!(error.kind(), AppErrorKind::NotFound);
        let AppError::Localized(localized) = error else {
            panic!("원문 io 메시지가 아니라 로케일 키가 실려야 한다");
        };
        assert_eq!(localized.key, "error.file.notFound");
        assert_eq!(
            localized.args.get("path").map(String::as_str),
            Some(stale.to_string_lossy().as_ref())
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 디렉토리_경로의_파일_탭도_거절된다() {
        let root = 임시_루트("directory");
        let dir = root.join("src");
        std::fs::create_dir_all(&dir).unwrap();

        let error = ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &파일_탭_종류(&dir))
            .expect_err("디렉토리는 파일 탭이 될 수 없다");
        assert_eq!(error.kind(), AppErrorKind::NotFound);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 루트_밖_파일_탭은_forbidden_으로_거절된다() {
        let dir = 임시_루트("outside");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let outside = dir.join("secret.txt");
        std::fs::write(&outside, "x").unwrap();

        let error = ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &파일_탭_종류(&outside))
            .expect_err("루트 밖 경로는 거절되어야 한다");

        assert_eq!(
            error.kind(),
            AppErrorKind::Forbidden,
            "경계 위반은 root_guard 의 Forbidden 을 그대로 전파한다"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cli_로_연_루트_밖_파일_탭은_통과하고_없는_파일이면_not_found_다() {
        let dir = 임시_루트("cli-opened");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let prompt = dir.join("claude-prompt.md");
        std::fs::write(&prompt, "# prompt\n").unwrap();
        let cli_opened: HashSet<PathBuf> = [std::fs::canonicalize(&prompt).unwrap()].into_iter().collect();

        assert!(ensure_file_tab_target_exists(&열린_프로젝트(&root), &cli_opened, &파일_탭_종류(&prompt)).is_ok());

        std::fs::remove_file(&prompt).unwrap();
        let error = ensure_file_tab_target_exists(&열린_프로젝트(&root), &cli_opened, &파일_탭_종류(&prompt))
            .expect_err("허용 목록에 있어도 디스크에 없으면 거절된다");
        assert_eq!(error.kind(), AppErrorKind::NotFound);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 파일이_아닌_탭_종류는_디스크를_보지_않고_통과한다() {
        let root = 임시_루트("non-file");

        assert!(ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &TabKind::Settings).is_ok());
        assert!(ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &TabKind::Welcome).is_ok());
        assert!(ensure_file_tab_target_exists(&열린_프로젝트(&root), &HashSet::new(), &TabKind::Untitled { index: 1 }).is_ok());

        std::fs::remove_dir_all(&root).ok();
    }
}

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use taide_infra::root_guard;
use taide_layout::service;
use taide_layout::service::{
    close_tab, ensure_focused_pane_valid, get_layout_mut, locate_project_with_tab, open_tab, resolve_default_open_pane, save_layout,
};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{
    ClosedTab, DropEdge, OpenTabInSplitRequest, ProjectLayout, ShellViewPatch, Tab, TabKind, TabPathChange, TabPathChangeResult,
    TabWindowTarget,
};
use taide_model::project::Project;

use crate::{AppState, EventSink, TaskSupervisor, WindowRegistry};

pub fn flush_dirty_layouts(state: &AppState) {
    let dirty: Vec<_> = state.dirty_layouts.write().drain().collect();
    if dirty.is_empty() {
        return;
    }

    let snapshots: Vec<_> = {
        let layouts = state.layouts.read();
        dirty
            .into_iter()
            .filter_map(|project_id| match layouts.get(&project_id) {
                Some(layout) => Some((project_id, layout.clone())),
                None => {
                    log::warn!("dirty_layouts 에 등록된 프로젝트의 레이아웃을 찾을 수 없어 저장을 건너뜁니다: {project_id}");
                    None
                }
            })
            .collect()
    };

    for (project_id, layout) in snapshots {
        if let Err(error) = save_layout(&state.paths, &project_id, &layout) {
            log::warn!("레이아웃 저장 실패 ({project_id}): {error}");
        }
    }
}

/// Flushes shared layouts serially on tracked blocking workers using the host's existing interval.
pub async fn flush_layouts_periodically(state: AppState, tasks: TaskSupervisor, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let state = state.clone();
        let Some(worker) = tasks.spawn_blocking_transient_handle("layout-flush-worker", move || {
            flush_dirty_layouts(&state);
        }) else {
            return;
        };
        let _ = worker.await;
    }
}

pub fn finish_mutation(sink: &dyn EventSink, state: &AppState, project_id: &ProjectId, layout: &mut ProjectLayout) -> ProjectLayout {
    ensure_focused_pane_valid(layout);
    let snapshot = layout.clone();
    state.dirty_layouts.write().insert(project_id.clone());

    sink.publish(AppEvent::LayoutChanged {
        project_id: project_id.clone(),
        revision: snapshot.revision,
    });

    snapshot
}

pub async fn open_tab_and_finish(
    sink: &dyn EventSink,
    state: &AppState,
    project_id: ProjectId,
    kind: TabKind,
    title: String,
    target: Option<PaneId>,
    preview: bool,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let preview = preview && state.settings.read().enable_preview_tabs;
    let pane_id = match target {
        Some(target) => target,
        None => resolve_default_open_pane(layout),
    };
    let tab = Tab {
        id: TabId::new(),
        kind,
        title,
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    open_tab(layout, &pane_id, tab, preview)?;

    let updated = finish_mutation(sink, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

pub async fn close_tab_and_finish<F>(
    sink: &dyn EventSink,
    state: &AppState,
    tab_id: &TabId,
    notify_tab_closed: F,
) -> AppResult<(ProjectId, ClosedTab, ProjectLayout)>
where
    F: FnOnce(&Tab),
{
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let closed = close_tab(layout, tab_id)?;

    let updated = finish_mutation(sink, state, &project_id, layout);
    *state.layouts.write() = layouts;

    notify_tab_closed(&closed.tab);

    Ok((project_id, closed, updated))
}

fn cleanup_emptied_auxiliary_windows(
    windows: &WindowRegistry,
    project_id: &ProjectId,
    layout: &mut ProjectLayout,
    close_window: &impl Fn(&str),
) {
    let emptied_slots: Vec<u32> = layout
        .auxiliary_windows
        .iter()
        .filter(|window| service::is_layout_tree_empty(&window.root))
        .map(|window| window.slot)
        .collect();

    for slot in emptied_slots {
        layout.auxiliary_windows.retain(|window| window.slot != slot);
        if let Some(label) = windows.label_for(project_id, slot) {
            close_window(&label);
        }
    }
}

pub async fn layout_move_tab_to_window<F, OpenFuture, C>(
    sink: &dyn EventSink,
    state: &AppState,
    windows: &WindowRegistry,
    tab_id: TabId,
    target: TabWindowTarget,
    open_auxiliary_window: F,
    close_window: C,
) -> AppResult<ProjectLayout>
where
    F: FnOnce(ProjectId, u32) -> OpenFuture,
    OpenFuture: Future<Output = AppResult<String>>,
    C: Fn(&str),
{
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = service::locate_project_with_tab(&layouts, &tab_id)?;
    let layout = service::get_layout_mut(&mut layouts, &project_id)?;

    match target {
        TabWindowTarget::Main => {
            service::move_tab_to_main(layout, &tab_id)?;
        }
        TabWindowTarget::Existing { slot } => {
            service::move_tab_to_existing_window(layout, &tab_id, slot)?;
        }
        TabWindowTarget::NewAuxiliary => {
            let slot = service::next_window_slot(layout);
            let label = open_auxiliary_window(project_id.clone(), slot).await?;
            if let Err(error) = service::move_tab_to_new_window(layout, &tab_id, slot) {
                close_window(&label);
                return Err(error);
            }
        }
    }

    cleanup_emptied_auxiliary_windows(windows, &project_id, layout, &close_window);

    let updated = finish_mutation(sink, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

pub async fn return_auxiliary_window_tabs(sink: &dyn EventSink, state: &AppState, project_id: ProjectId, window_slot: u32) {
    let _guard = state.begin_mutation().await;

    let mirrored_paths: HashSet<String> = taide_file::service::list_mirrors(&state.paths, &project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|mirror| mirror.path)
        .collect();

    let mut layouts = state.layouts.read().clone();
    let Some(layout) = layouts.get_mut(&project_id) else {
        log::debug!("보조 창 탭 복귀 생략: 프로젝트가 이미 닫혔습니다 (projectId={project_id})");
        return;
    };

    service::clear_auxiliary_window_phantom_dirty(layout, window_slot, &|path| mirrored_paths.contains(path));

    if !service::return_auxiliary_window_tabs(layout, window_slot) {
        log::debug!("보조 창 탭 복귀 생략: 슬롯을 찾을 수 없습니다 (projectId={project_id}, windowSlot={window_slot})");
        return;
    }

    let revision = layout.revision;
    *state.layouts.write() = layouts;

    state.dirty_layouts.write().insert(project_id.clone());
    sink.publish(AppEvent::LayoutChanged { project_id, revision });
}

pub async fn layout_get(state: &AppState, project_id: ProjectId) -> AppResult<ProjectLayout> {
    state
        .layouts
        .read()
        .get(&project_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("layout not found: {project_id}")))
}

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

pub async fn layout_open_tab(
    sink: &dyn EventSink,
    state: &AppState,
    project_id: ProjectId,
    kind: TabKind,
    title: String,
    target: Option<PaneId>,
    preview: bool,
) -> AppResult<ProjectLayout> {
    let projects = state.projects.read().clone();
    ensure_file_tab_target_exists(&projects, &state.cli_opened_paths.read(), &kind)?;

    open_tab_and_finish(sink, state, project_id, kind, title, target, preview).await
}

enum LayoutLocate {
    WithTab(TabId),
    WithPane(PaneId),
    Direct(ProjectId),
}

async fn run_layout_mutation<F>(sink: &dyn EventSink, state: &AppState, locate: LayoutLocate, mutate: F) -> AppResult<ProjectLayout>
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

    let updated = finish_mutation(sink, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

pub async fn layout_activate_tab(sink: &dyn EventSink, state: &AppState, tab_id: TabId) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::activate_tab(layout, &tab_id)
    })
    .await
}

pub async fn layout_move_tab(
    sink: &dyn EventSink,
    state: &AppState,
    tab_id: TabId,
    pane_id: PaneId,
    index: u32,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::move_tab(layout, &tab_id, &pane_id, index as usize)
    })
    .await
}

pub async fn layout_split(
    sink: &dyn EventSink,
    state: &AppState,
    pane_id: PaneId,
    edge: DropEdge,
    tab_id: TabId,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::split(layout, &pane_id, edge, &tab_id)
    })
    .await
}

pub async fn layout_open_tab_in_split(sink: &dyn EventSink, state: &AppState, request: OpenTabInSplitRequest) -> AppResult<ProjectLayout> {
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
    run_layout_mutation(sink, state, LayoutLocate::Direct(project_id), move |layout| {
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

pub async fn layout_resize(sink: &dyn EventSink, state: &AppState, pane_id: PaneId, sizes: Vec<f32>) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::resize(layout, &pane_id, sizes)
    })
    .await
}

pub async fn layout_focus_pane(sink: &dyn EventSink, state: &AppState, pane_id: PaneId) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithPane(pane_id.clone()), move |layout| {
        service::focus_pane(layout, &pane_id)
    })
    .await
}

pub async fn layout_pin_tab(sink: &dyn EventSink, state: &AppState, tab_id: TabId, pinned: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::pin_tab(layout, &tab_id, pinned)
    })
    .await
}

pub async fn layout_set_preview(sink: &dyn EventSink, state: &AppState, tab_id: TabId, preview: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_preview(layout, &tab_id, preview)
    })
    .await
}

pub async fn layout_reopen_closed(sink: &dyn EventSink, state: &AppState, project_id: ProjectId) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::Direct(project_id), move |layout| {
        service::reopen_closed(layout);
        Ok(())
    })
    .await
}

pub async fn layout_set_view_state(
    sink: &dyn EventSink,
    state: &AppState,
    tab_id: TabId,
    view_state: Option<String>,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_view_state(layout, &tab_id, view_state)
    })
    .await
}

pub async fn layout_set_dirty(sink: &dyn EventSink, state: &AppState, tab_id: TabId, dirty: bool) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_dirty(layout, &tab_id, dirty)
    })
    .await
}

pub async fn layout_set_terminal_session(
    sink: &dyn EventSink,
    state: &AppState,
    tab_id: TabId,
    session_id: String,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::WithTab(tab_id.clone()), move |layout| {
        service::set_terminal_session(layout, &tab_id, session_id)
    })
    .await
}

pub async fn layout_open_untitled(
    sink: &dyn EventSink,
    state: &AppState,
    project_id: ProjectId,
    target: Option<PaneId>,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::Direct(project_id), move |layout| {
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

pub async fn layout_convert_untitled(sink: &dyn EventSink, state: &AppState, tab_id: TabId, path: String) -> AppResult<ProjectLayout> {
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

    let updated = finish_mutation(sink, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

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

pub async fn layout_apply_path_change(
    sink: &dyn EventSink,
    state: &AppState,
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

    let updated = finish_mutation(sink, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(TabPathChangeResult {
        layout: updated,
        moved: outcome.moved,
        closed_paths: outcome.closed_paths,
    })
}

pub async fn layout_set_shell_view(
    sink: &dyn EventSink,
    state: &AppState,
    project_id: ProjectId,
    patch: ShellViewPatch,
) -> AppResult<ProjectLayout> {
    run_layout_mutation(sink, state, LayoutLocate::Direct(project_id), move |layout| {
        service::apply_shell_view_patch(layout, &patch);
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_layout::service::{default_layout, load_layout};
    use taide_model::layout::PaneNode;
    use taide_model::paths::AppPaths;

    fn 파일_탭(path: &str) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::File { path: path.to_string() },
            title: path.to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-layout-{name}-{}", uuid::Uuid::new_v4()))
    }

    fn 임시_상태(name: &str) -> AppState {
        AppState::new(AppPaths::new(temp_data_dir(name)))
    }

    fn 저장된_탭_제목(paths: &AppPaths, project_id: &ProjectId) -> Vec<String> {
        let PaneNode::Leaf { tabs, .. } = load_layout(paths, project_id).root else {
            panic!("expected leaf")
        };
        tabs.into_iter().map(|tab| tab.title).collect()
    }

    #[test]
    fn flush_dirty_layouts_는_더러운_프로젝트만_저장하고_집합을_비운다() {
        let state = 임시_상태("flush-dirty");
        let dirty_id = ProjectId::new();
        let clean_id = ProjectId::new();

        let mut dirty_layout = default_layout();
        let leaf_id = dirty_layout.focused_pane.clone();
        open_tab(&mut dirty_layout, &leaf_id, 파일_탭("dirty.rs"), false).expect("open");

        {
            let mut layouts = state.layouts.write();
            layouts.insert(dirty_id.clone(), dirty_layout);
            layouts.insert(clean_id.clone(), default_layout());
        }
        state.dirty_layouts.write().insert(dirty_id.clone());

        flush_dirty_layouts(&state);

        assert!(state.dirty_layouts.read().is_empty(), "flush 후 dirty 집합은 비어야 한다");
        assert!(state.paths.layout_file(&dirty_id).exists());
        assert!(
            !state.paths.layout_file(&clean_id).exists(),
            "더럽지 않은 프로젝트는 저장되지 않아야 한다"
        );
        assert!(저장된_탭_제목(&state.paths, &dirty_id).contains(&"dirty.rs".to_string()));

        std::fs::remove_dir_all(&state.paths.data_dir).ok();
    }

    #[test]
    fn flush_이후_다시_더러워진_레이아웃은_다음_flush_에서_저장된다() {
        let state = 임시_상태("flush-redirty");
        let project_id = ProjectId::new();

        state.layouts.write().insert(project_id.clone(), default_layout());
        state.dirty_layouts.write().insert(project_id.clone());
        flush_dirty_layouts(&state);

        let first = 저장된_탭_제목(&state.paths, &project_id);
        assert!(!first.contains(&"later.rs".to_string()));

        {
            let mut layouts = state.layouts.write();
            let layout = layouts.get_mut(&project_id).expect("layout");
            let leaf_id = layout.focused_pane.clone();
            open_tab(layout, &leaf_id, 파일_탭("later.rs"), false).expect("open");
        }
        state.dirty_layouts.write().insert(project_id.clone());
        flush_dirty_layouts(&state);

        let second = 저장된_탭_제목(&state.paths, &project_id);
        assert_eq!(second.len(), first.len() + 1);
        assert!(
            second.contains(&"later.rs".to_string()),
            "flush 사이에 생긴 변경은 다음 flush 에서 저장돼야 한다"
        );

        std::fs::remove_dir_all(&state.paths.data_dir).ok();
    }

    #[test]
    fn 연속_flush_는_마지막_상태를_남기고_중간_flush_를_덮어쓴다() {
        let state = 임시_상태("flush-order");
        let project_id = ProjectId::new();
        state.layouts.write().insert(project_id.clone(), default_layout());

        for index in 0..3 {
            {
                let mut layouts = state.layouts.write();
                let layout = layouts.get_mut(&project_id).expect("layout");
                let leaf_id = layout.focused_pane.clone();
                open_tab(layout, &leaf_id, 파일_탭(&format!("step-{index}.rs")), false).expect("open");
            }
            state.dirty_layouts.write().insert(project_id.clone());
            flush_dirty_layouts(&state);
        }

        let titles = 저장된_탭_제목(&state.paths, &project_id);
        for index in 0..3 {
            assert!(
                titles.contains(&format!("step-{index}.rs")),
                "매 flush 는 그 시점의 전체 스냅샷을 남겨야 한다"
            );
        }

        std::fs::remove_dir_all(&state.paths.data_dir).ok();
    }

    #[test]
    fn 레이아웃이_없는_더러운_프로젝트는_건너뛰고_집합에서_제거된다() {
        let state = 임시_상태("flush-missing");
        let missing_id = ProjectId::new();
        state.dirty_layouts.write().insert(missing_id.clone());

        flush_dirty_layouts(&state);

        assert!(state.dirty_layouts.read().is_empty());
        assert!(!state.paths.layout_file(&missing_id).exists());

        std::fs::remove_dir_all(&state.paths.data_dir).ok();
    }
}

#[cfg(test)]
mod command_tests {
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

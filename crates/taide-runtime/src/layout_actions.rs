use taide_layout::service::{
    close_tab, ensure_focused_pane_valid, get_layout_mut, locate_project_with_tab, open_tab, resolve_default_open_pane, save_layout,
};
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{ClosedTab, ProjectLayout, Tab, TabKind};

use crate::{AppState, EventSink};

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

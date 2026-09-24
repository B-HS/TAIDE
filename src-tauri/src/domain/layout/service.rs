use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use super::types::{ClosedTab, ProjectLayout, Tab, TabKind};
use crate::error::AppResult;
use crate::events::LayoutChanged;
use crate::ids::{PaneId, ProjectId, TabId};
use crate::state::AppState;

pub use taide_layout::service::*;

pub(crate) const LAYOUT_FLUSH_INTERVAL_MS: u64 = 2_000;

/// Persists every layout marked dirty since the last flush, then clears the dirty set.
///
/// **Blocking.** Each project costs one [`save_layout`] → `persist::write_json` → `write_atomic`,
/// and `write_atomic` ends in `file.sync_all()`, so N dirty projects mean N serialized fsyncs. The
/// `LAYOUT_FLUSH_INTERVAL_MS` ticker in `lib.rs` therefore calls this from
/// `tauri::async_runtime::spawn_blocking` and awaits the handle: off the async worker (architecture
/// §2.1 — blocking IO never runs on it, research 3b §2-D), and awaited so two flushes never overlap,
/// which is what keeps the writes for one project in tick order. The two shutdown callers
/// (`WindowEvent::Destroyed` and `RunEvent::Exit`) stay synchronous on purpose: they are the last
/// chance to persist before the window or the process goes away, and there is nothing left to await
/// on.
///
/// Draining first and writing from the drained snapshots is what makes a background flush lossless:
/// a project re-marked dirty while its snapshot is being written stays in the set and is written
/// again on the next tick, rather than having its newer state dropped.
pub(crate) fn flush_dirty_layouts(state: &AppState) {
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

/// Completes a layout mutation: seals the pane-focus invariant, marks the project dirty for the
/// next flush, announces the new revision, and returns the snapshot the command replies with.
///
/// [`ensure_focused_pane_valid`] runs *before* the snapshot is taken so that the one layout value
/// three consumers share — the in-memory `state.layouts` entry, the persisted copy the flush writes,
/// and the object the frontend caches — can never carry a `focused_pane` (main tree or any
/// auxiliary window) naming a pane that does not exist. Individual mutations still choose the
/// *right* successor themselves ([`close_tab`]); this is the backstop that keeps a future mutation
/// that forgets to from handing the frontend an id it will send straight back as an explicit
/// `target` (contract §1 R1 2). No `revision` bump: the repair rides along with the mutation that
/// caused it, so the snapshot `LayoutChanged` announces is already the repaired one.
pub fn finish_mutation(app: &AppHandle, state: &AppState, project_id: &ProjectId, layout: &mut ProjectLayout) -> ProjectLayout {
    ensure_focused_pane_valid(layout);
    let snapshot = layout.clone();
    state.dirty_layouts.write().insert(project_id.clone());

    let _ = LayoutChanged {
        project_id: project_id.clone(),
        revision: snapshot.revision,
    }
    .emit(app);

    snapshot
}

/// Opens a tab and completes the post-processing (dirty marking + layout-changed event emission).
/// Shared so the Tauri command (`layout_open_tab`) and the IDE domain's `openFile` tool handler
/// run the same path — the service-level entry point that keeps the IDE from reusing the command
/// surface as a second entry point (R6#3). Takes the mutation guard itself to prevent layout
/// read-clone-write races.
pub async fn open_tab_and_finish(
    app: &AppHandle,
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

    let updated = finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

/// 탭을 닫고 후처리(레이아웃 갱신 이벤트 발신 + IDE 도메인의 pending diff 해소 + 터미널 탭이면
/// pty 세션 회수)까지 마친다. Tauri 커맨드(`layout_close_tab`)와 IDE 도메인의
/// `close_tab`/`closeAllDiffTabs` 도구 핸들러가 동일한 경로를 타도록 공유한다 — ClaudeDiff 탭이
/// 어떤 경로로 닫히든 pending 요청이 반드시 해소되고, 터미널 탭이 어떤 경로로 닫히든 그 pty 가
/// 반드시 죽는다. 레이아웃 read-clone-write 경합을 막기 위해 뮤테이션 가드는 이 함수가 직접 잡는다.
///
/// The pty reap is the "탭 닫기 시 pty_kill" half of the T0 #21 fix (`docs/acknowledge/
/// 2026-08-18-audit-t0-fix-contract.md` §2.3) — `project_close`'s `TerminalStore::kill_project`
/// only reaps sessions when the *project* closes, and before this fix nothing called `pty_kill` when
/// an individual terminal tab closed; the session lingered, attached to nothing, until the owning
/// project or the whole app closed.
pub async fn close_tab_and_finish(app: &AppHandle, state: &AppState, tab_id: &TabId) -> AppResult<(ProjectId, ClosedTab, ProjectLayout)> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let closed = close_tab(layout, tab_id)?;

    let updated = finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;

    crate::domain::ide::store::reconcile_closed_tab(app, &closed.tab);
    if let TabKind::Terminal { session_id, .. } = &closed.tab.kind {
        app.state::<crate::domain::terminal::commands::TerminalStore>()
            .kill_session(session_id);
    }

    Ok((project_id, closed, updated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::layout::types::PaneNode;
    use crate::paths::AppPaths;

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

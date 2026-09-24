use std::collections::HashSet;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::domain::file::service as file_service;
use crate::domain::layout::service as layout_service;
use crate::events::LayoutChanged;
use crate::ids::ProjectId;
use crate::state::AppState;

pub use taide_window::service::*;

/// Every path this project currently holds an unsaved hot-exit mirror for — the disk-side half of
/// a tab's `dirty` flag, read once per auxiliary-window close and handed to
/// `layout_service::clear_auxiliary_window_phantom_dirty` as a plain membership test.
///
/// An unreadable mirror directory answers "no mirrors", which is the safe direction here: the
/// worst it does is leave a `dirty` flag alone.
fn mirrored_paths(state: &AppState, project_id: &ProjectId) -> HashSet<String> {
    file_service::list_mirrors(&state.paths, project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|mirror| mirror.path)
        .collect()
}

/// Runs on an auxiliary editor window's `CloseRequested`/`Destroyed` — merges that window's tabs
/// back into the main window's layout tail and drops its `AuxWindowLayout` entry
/// (`layout::service::return_auxiliary_window_tabs`), TAIDE's 0-loss philosophy on aux-window close
/// (contract §3.1/§3.2), unlike VS Code discarding an auxiliary window's content when it closes.
///
/// Spawns the actual mutation as an async task guarded by `AppState::begin_mutation` — this is
/// called from two synchronous `tauri::WindowEvent` handlers that can't `.await` the async mutation
/// guard themselves: `lib.rs`'s `on_window_event` `Destroyed` arm directly, and
/// `domain::window::commands::handle_auxiliary_close_requested` (reached from that same closure's
/// `CloseRequested` arm via `handle_close_requested`). But every `layout::commands` mutation
/// (close/activate/move/pin/...) reads `state.layouts`, clones it, mutates the clone, and writes it
/// back *without* holding the read lock
/// across that gap — so a synchronous read-modify-write straight into `state.layouts` here could
/// still race a concurrent command's write and silently lose either mutation. Routing through the
/// same `begin_mutation` guard and the same clone→mutate→write-back shape those commands use closes
/// that gap. The window itself doesn't wait on this task — by the time either arm reaches here the
/// OS close is already proceeding (`handle_auxiliary_close_requested` intercepts only the *first*
/// close request, to run that window's scoped flush; this runs from the follow-up close it
/// re-issues, and from `Destroyed`) — worst case if the app quits before this task gets scheduled, the just-closed
/// window's `AuxWindowLayout` entry (and its tabs) simply stays in the persisted layout and comes
/// back as a re-opened window on next launch (`restore_auxiliary_windows`) instead of merging into
/// main immediately; nothing is lost. A no-op (with a debug log, not a warning — this is the
/// expected idempotent second call) when the project closed independently or `window_slot` names a
/// window already processed.
pub fn plan_return_of_auxiliary_window_tabs(app: &AppHandle, project_id: &ProjectId, window_slot: u32) {
    let app = app.clone();
    let project_id = project_id.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let _guard = state.begin_mutation().await;

        let mirrored_paths = mirrored_paths(&state, &project_id);

        let mut layouts = state.layouts.read().clone();
        let Some(layout) = layouts.get_mut(&project_id) else {
            log::debug!("보조 창 탭 복귀 생략: 프로젝트가 이미 닫혔습니다 (projectId={project_id})");
            return;
        };

        layout_service::clear_auxiliary_window_phantom_dirty(layout, window_slot, &|path| mirrored_paths.contains(path));

        if !layout_service::return_auxiliary_window_tabs(layout, window_slot) {
            log::debug!("보조 창 탭 복귀 생략: 슬롯을 찾을 수 없습니다 (projectId={project_id}, windowSlot={window_slot})");
            return;
        }

        let revision = layout.revision;
        *state.layouts.write() = layouts;

        state.dirty_layouts.write().insert(project_id.clone());
        let _ = LayoutChanged { project_id, revision }.emit(&app);
    });
}

use std::collections::HashSet;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use super::types::{AUXILIARY_WINDOW_LABEL_PREFIX, AUXILIARY_WINDOW_STATE_KEY};
use crate::domain::layout::service as layout_service;
use crate::events::LayoutChanged;
use crate::ids::ProjectId;
use crate::state::AppState;

pub fn is_auxiliary_label(label: &str) -> bool {
    label.starts_with(AUXILIARY_WINDOW_LABEL_PREFIX)
}

/// Picks the lowest unused `editor-<n>` label given the labels of every window currently open.
/// Rust — not the caller — owns label issuance (contract §3.1): the frontend only supplies which
/// project/layout slot the new window represents, never the OS-level label itself, so two
/// concurrent open requests can never coin the same window label.
pub fn next_auxiliary_label<'a>(existing_labels: impl Iterator<Item = &'a str>) -> String {
    let used: HashSet<u32> = existing_labels
        .filter_map(|label| label.strip_prefix(AUXILIARY_WINDOW_LABEL_PREFIX))
        .filter_map(|suffix| suffix.parse::<u32>().ok())
        .collect();

    let mut candidate = 1u32;
    while used.contains(&candidate) {
        candidate += 1;
    }
    format!("{AUXILIARY_WINDOW_LABEL_PREFIX}{candidate}")
}

/// Collapses every auxiliary editor window label onto one shared `tauri-plugin-window-state`
/// cache key, passed to the plugin's `Builder::map_label` hook (`lib.rs`). Without this, each
/// ever-incrementing `editor-<n>` label would grow `.window-state.json` by one entry every time a
/// window opens and never shrink, since the plugin never learns a label is retired for good. The
/// main window's own label passes through untouched.
pub fn normalize_window_state_label(label: &str) -> &str {
    if is_auxiliary_label(label) {
        AUXILIARY_WINDOW_STATE_KEY
    } else {
        label
    }
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
/// that gap. The window itself doesn't wait on this task (both `CloseRequested` and `Destroyed`
/// already let the OS proceed independently — `handle_auxiliary_close_requested` never calls
/// `prevent_close`) — worst case if the app quits before this task gets scheduled, the just-closed
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

        let mut layouts = state.layouts.read().clone();
        let Some(layout) = layouts.get_mut(&project_id) else {
            log::debug!("보조 창 탭 복귀 생략: 프로젝트가 이미 닫혔습니다 (projectId={project_id})");
            return;
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_접두사가_붙은_라벨만_보조_창으로_인식한다() {
        assert!(is_auxiliary_label("editor-1"));
        assert!(is_auxiliary_label("editor-42"));
        assert!(!is_auxiliary_label("main"));
        assert!(!is_auxiliary_label("editor"));
    }

    #[test]
    fn 기존_라벨이_없으면_editor_1_을_할당한다() {
        assert_eq!(next_auxiliary_label(std::iter::empty()), "editor-1");
    }

    #[test]
    fn 메인_창_라벨은_번호_할당에_영향을_주지_않는다() {
        assert_eq!(next_auxiliary_label(["main"].into_iter()), "editor-1");
    }

    #[test]
    fn 사용중인_가장_작은_번호_다음을_할당한다() {
        assert_eq!(next_auxiliary_label(["main", "editor-1", "editor-2"].into_iter()), "editor-3");
    }

    #[test]
    fn 중간_번호가_닫혀_비면_그_번호를_재사용한다() {
        assert_eq!(next_auxiliary_label(["main", "editor-1", "editor-3"].into_iter()), "editor-2");
    }

    #[test]
    fn 보조_창_라벨은_공유_상태_키로_정규화된다() {
        assert_eq!(normalize_window_state_label("editor-1"), "editor");
        assert_eq!(normalize_window_state_label("editor-7"), "editor");
    }

    #[test]
    fn 메인_창_라벨은_정규화되지_않고_그대로다() {
        assert_eq!(normalize_window_state_label("main"), "main");
    }
}

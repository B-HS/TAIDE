use std::collections::HashMap;
use std::path::PathBuf;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};

use super::service;
use super::types::{
    AuxiliaryWindowInfo, AUXILIARY_WINDOW_DEFAULT_HEIGHT, AUXILIARY_WINDOW_DEFAULT_WIDTH, AUXILIARY_WINDOW_MIN_HEIGHT,
    AUXILIARY_WINDOW_MIN_WIDTH,
};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::state::AppState;

struct AuxiliaryWindowRecord {
    project_id: ProjectId,
    window_slot: u32,
}

/// Runtime registry from an auxiliary editor window's Tauri label back to which project/slot it
/// renders — the label alone (`editor-<n>`) carries no semantic meaning, so `lib.rs`'s
/// `CloseRequested`/`Destroyed` handlers need this to know what to hand
/// `service::plan_return_of_auxiliary_window_tabs`. The main window is never registered here.
#[derive(Default)]
pub struct WindowStore(Mutex<HashMap<String, AuxiliaryWindowRecord>>);

impl WindowStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn register(&self, label: String, project_id: ProjectId, window_slot: u32) {
        self.0.lock().insert(label, AuxiliaryWindowRecord { project_id, window_slot });
    }

    /// Removes and returns `label`'s recorded (project, slot), if any. Called from both
    /// `CloseRequested` (so the tab-return hook fires before the OS destroys the window) and
    /// `Destroyed` (so a window that goes away through any other path — a crash, a programmatic
    /// `.close()` — doesn't leave a stale entry behind). Idempotent: whichever event fires second
    /// finds nothing left to remove.
    pub fn forget(&self, label: &str) -> Option<(ProjectId, u32)> {
        self.0.lock().remove(label).map(|record| (record.project_id, record.window_slot))
    }

    /// Reverse lookup — the OS window label currently rendering `(project_id, window_slot)`, if any
    /// is open. Used by `layout::commands::layout_move_tab_to_window` to close an auxiliary
    /// window's OS window once moving its last tab elsewhere leaves it empty.
    pub fn label_for(&self, project_id: &ProjectId, window_slot: u32) -> Option<String> {
        self.0
            .lock()
            .iter()
            .find(|(_, record)| &record.project_id == project_id && record.window_slot == window_slot)
            .map(|(label, _)| label.clone())
    }
}

/// Opens a new auxiliary editor window (`editor-<n>`) rendering `project_id` pinned to
/// `window_slot`. Rust owns label issuance (`service::next_auxiliary_label`) so two concurrent
/// callers can never coin the same OS window label; `window_slot` is only a semantic key the
/// caller supplies for which layout slot the window represents
/// (`domain::layout::types::AuxWindowLayout::slot`) and is otherwise opaque to this function.
///
/// The window is created `visible(false)`, matching the main window's own boot sequence
/// (`tauri.conf.json`) — the frontend's existing `useRevealWindow` hook (mounted by every window
/// through `ThemeProvider`) shows it once that window's own bootstrap finishes, avoiding a flash
/// of unstyled content. A window that never mounts that provider tree would stay invisible
/// forever; Phase F's auxiliary-window bootstrap is expected to reuse it like every other window.
///
/// Deliberately does **not** take `AppState::begin_mutation` itself — every caller (boot-time
/// restoration in `lib.rs` and `layout::commands::layout_move_tab_to_window`'s `newAuxiliary` path —
/// the standalone `window_open_auxiliary` command that used to be a third caller was removed as a
/// duplicate IPC surface, X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2)
/// already holds it (or must acquire it) for the duration of the whole operation it's part of; a
/// second `.await` on the same non-reentrant `tokio::sync::Mutex` here would deadlock. Boot-time
/// restoration specifically relies on each caller serializing through that single guard — without
/// it, two windows recreated concurrently could both read the same "existing labels" snapshot before
/// either finishes `.build()` and coin the same `editor-<n>` label.
pub async fn open_auxiliary_window(
    app: &AppHandle,
    state: &AppState,
    windows: &WindowStore,
    project_id: ProjectId,
    window_slot: u32,
) -> AppResult<AuxiliaryWindowInfo> {
    let project_name = state
        .projects
        .read()
        .get(&project_id)
        .map(|project| project.name.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let label = service::next_auxiliary_label(app.webview_windows().keys().map(String::as_str));

    // `ProjectId` is always `prj-<uuid-v4>` (see `ids.rs`) — alphanumeric plus hyphens only, so
    // it never needs percent-encoding to be embedded in a URL query string.
    let url = WebviewUrl::App(PathBuf::from(format!(
        "index.html?projectId={}&windowSlot={window_slot}",
        project_id.as_str()
    )));

    WebviewWindowBuilder::new(app, &label, url)
        .title(format!("{project_name} — TAIDE"))
        .inner_size(AUXILIARY_WINDOW_DEFAULT_WIDTH, AUXILIARY_WINDOW_DEFAULT_HEIGHT)
        .min_inner_size(AUXILIARY_WINDOW_MIN_WIDTH, AUXILIARY_WINDOW_MIN_HEIGHT)
        .visible(false)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
        .build()
        .map_err(|error| AppError::Internal(format!("보조 창을 여는 데 실패했습니다: {error}")))?;

    windows.register(label.clone(), project_id.clone(), window_slot);

    Ok(AuxiliaryWindowInfo {
        label,
        project_id,
        window_slot,
    })
}

/// Toggles Zen mode's optional fullscreen (`Settings::zen_fullscreen`) for the calling window —
/// `window: tauri::Window` is Tauri-injected as whichever window's webview made this IPC call
/// (same pattern as `file::commands::file_flush_complete`), so the frontend never needs to name its
/// own label and can't fullscreen a window that isn't itself. A plain wrapper over
/// `tauri::Window::set_fullscreen` — a custom app command, so (per the S1 capability audit) it
/// needs no `core:window:allow-set-fullscreen` ACL permission; that permission only gates the
/// window plugin's own JS-exposed `setFullscreen` call, not a Rust-side call made from inside a
/// command handler.
#[tauri::command]
#[specta::specta]
pub async fn window_set_fullscreen(window: tauri::Window<tauri::Wry>, fullscreen: bool) -> AppResult<()> {
    window
        .set_fullscreen(fullscreen)
        .map_err(|error| AppError::Internal(format!("전체화면 전환에 실패했습니다: {error}")))
}

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

use super::service;
use super::types::{
    AuxiliaryWindowInfo, AUXILIARY_WINDOW_DEFAULT_HEIGHT, AUXILIARY_WINDOW_DEFAULT_WIDTH, AUXILIARY_WINDOW_MIN_HEIGHT,
    AUXILIARY_WINDOW_MIN_WIDTH,
};
use crate::constants;
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::events::HotExitFlushRequested;
use crate::ids::ProjectId;
use crate::infra::navigation_guard;
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
/// restoration in [`restore_auxiliary_windows`] below and
/// `layout::commands::layout_move_tab_to_window`'s `newAuxiliary` path —
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

    navigation_guard::apply_navigation_guard(
        WebviewWindowBuilder::new(app, &label, url)
            .title(format!("{project_name} — TAIDE"))
            .inner_size(AUXILIARY_WINDOW_DEFAULT_WIDTH, AUXILIARY_WINDOW_DEFAULT_HEIGHT)
            .min_inner_size(AUXILIARY_WINDOW_MIN_WIDTH, AUXILIARY_WINDOW_MIN_HEIGHT)
            .visible(false)
            .title_bar_style(TitleBarStyle::Overlay)
            .hidden_title(true),
        app.config(),
    )
    .build()
    .map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.window.auxiliaryOpenFailed",
            format!("failed to open the auxiliary window: {error}"),
        )
        .with_arg("detail", &error)
    })?;

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
    window.set_fullscreen(fullscreen).map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.window.fullscreenToggleFailed",
            format!("failed to toggle fullscreen: {error}"),
        )
        .with_arg("detail", &error)
    })
}

/// A custom (non-predefined) menu item id for the app menu's Quit entry —
/// see [`handle_menu_event`] for why this can't be `PredefinedMenuItem::quit`.
const MENU_ID_QUIT: &str = "taide-quit";

pub(crate) fn build_app_menu(handle: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};

    let quit_item = MenuItemBuilder::with_id(MENU_ID_QUIT, "Quit")
        .accelerator("CmdOrCtrl+Q")
        .build(handle)?;

    let app_menu = SubmenuBuilder::new(handle, "TAIDE")
        .item(&PredefinedMenuItem::about(handle, None, None)?)
        .separator()
        .item(&PredefinedMenuItem::services(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::hide(handle, None)?)
        .item(&PredefinedMenuItem::hide_others(handle, None)?)
        .item(&PredefinedMenuItem::show_all(handle, None)?)
        .separator()
        .item(&quit_item)
        .build()?;

    let edit_menu = SubmenuBuilder::new(handle, "Edit")
        .item(&PredefinedMenuItem::undo(handle, None)?)
        .item(&PredefinedMenuItem::redo(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::cut(handle, None)?)
        .item(&PredefinedMenuItem::copy(handle, None)?)
        .item(&PredefinedMenuItem::paste(handle, None)?)
        .item(&PredefinedMenuItem::select_all(handle, None)?)
        .build()?;

    let window_menu = SubmenuBuilder::new(handle, "Window")
        .item(&PredefinedMenuItem::minimize(handle, None)?)
        .item(&PredefinedMenuItem::maximize(handle, None)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(handle, None)?)
        .build()?;

    MenuBuilder::new(handle).items(&[&app_menu, &edit_menu, &window_menu]).build()
}

/// Handles `CloseRequested` for an auxiliary editor window (`editor-<n>`).
/// Before Wave I every window funneled into the same hot-exit flush/exit
/// sequence `handle_close_requested` runs for the main window below, so
/// closing a second window silently terminated the whole app
/// (`docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §2.1).
/// The close is intentionally left un-prevented here so the OS's default
/// close proceeds and only this one window goes away. Both registry cleanups
/// are idempotent with `lib.rs`'s `WindowEvent::Destroyed` handler, which runs
/// this same cleanup again as a backstop for closes that don't go through
/// `CloseRequested` at all (e.g. a crash).
fn handle_auxiliary_close_requested(window: &tauri::Window<tauri::Wry>) {
    if window.state::<AppState>().forget_hot_exit_flush_window(window.label()) {
        window.app_handle().exit(0);
    }

    if let Some((project_id, window_slot)) = window.state::<WindowStore>().forget(window.label()) {
        service::plan_return_of_auxiliary_window_tabs(&window.app_handle().clone(), &project_id, window_slot);
    }
}

/// Intercepts the window close request to give every open window a chance to
/// flush its own dirty editor models to the hot-exit mirror before the app
/// actually exits. Only the main window runs this path — see
/// [`handle_auxiliary_close_requested`] for `editor-*` windows, which close
/// immediately instead. Always defers the main window's close
/// (`prevent_close`) and lets either every expected window's own
/// `file_flush_complete` call or the timeout fallback below perform the real
/// `AppHandle::exit`, so the window is never destroyed by the OS's default
/// close path. `AppState::begin_hot_exit_flush` guards re-entrant close
/// attempts (e.g. mashing Cmd+Q) from emitting the flush event twice.
pub(crate) fn handle_close_requested(window: &tauri::Window<tauri::Wry>, api: &tauri::CloseRequestApi) {
    if service::is_auxiliary_label(window.label()) {
        handle_auxiliary_close_requested(window);
        return;
    }

    api.prevent_close();

    let state = window.state::<AppState>();
    state.begin_shutdown();
    // `windows()` needs the `unstable` Tauri feature this project doesn't enable;
    // `webview_windows()` is the stable equivalent and every window here is a webview window.
    let expected_windows: HashSet<String> = window.webview_windows().into_keys().collect();
    if !state.begin_hot_exit_flush(expected_windows) {
        return;
    }

    // `Event::emit` broadcasts to every window/webview app-wide regardless of which handle it's
    // called through, so every currently-open auxiliary window already receives this alongside
    // the main window — no separate per-window fanout loop is needed here.
    let _ = HotExitFlushRequested {
        timeout_ms: constants::HOT_EXIT_FLUSH_TIMEOUT_MS as f64,
    }
    .emit(window);

    let app_handle = window.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(constants::HOT_EXIT_FLUSH_TIMEOUT_MS)).await;
        if app_handle.state::<AppState>().force_complete_hot_exit_flush() {
            log::warn!("hot exit flush timed out; exiting without every window's confirmation");
            app_handle.exit(0);
        }
    });
}

/// Routes the app menu's Quit item through the same `WindowEvent::CloseRequested`
/// flush handshake as clicking the window's close button, instead of the native
/// `NSApplication terminate:` binding `PredefinedMenuItem::quit` uses on macOS.
/// `terminate:` runs straight to `applicationWillTerminate` and `RunEvent::Exit`
/// with no `CloseRequested` (or preventable `ExitRequested`) event in between —
/// tao's macOS app delegate has no `applicationShouldTerminate:` hook to catch it
/// — so Cmd+Q would otherwise skip `handle_close_requested` entirely and drop
/// any hot-exit mirror writes still pending in the last debounce window.
pub(crate) fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    if event.id() != MENU_ID_QUIT {
        return;
    }
    if let Some(window) = app.get_webview_window(constants::MAIN_WINDOW_LABEL) {
        let _ = window.close();
    }
}

/// Recreates every `AuxWindowLayout` recorded across every restored (non-`root_missing`) project as
/// a real OS window — the boot-time half of "보조 창 레이아웃 영속·재시작 시 복원(창 재생성)"
/// (contract §3.2). Spawns one task per window rather than awaiting them serially so restoration
/// doesn't block the rest of `setup()`; each task still takes `AppState::begin_mutation` for the
/// duration of its own `window::commands::open_auxiliary_window` call, so concurrent restorations
/// serialize through that single guard instead of racing to coin the same `editor-<n>` label (see
/// that function's doc comment).
pub(crate) fn restore_auxiliary_windows(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let project_ids = service::restorable_project_ids(&state.projects.read());

    let layouts = state.layouts.read();
    let restorations = service::plan_auxiliary_window_restorations(&project_ids, &layouts);
    drop(layouts);

    for (project_id, window_slot) in restorations {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_handle.state::<AppState>();
            let _guard = state.begin_mutation().await;
            let windows = app_handle.state::<WindowStore>();
            if let Err(error) = open_auxiliary_window(&app_handle, &state, &windows, project_id.clone(), window_slot).await {
                log::warn!("보조 창 복원 실패 (projectId={project_id}, windowSlot={window_slot}): {error}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_id(name: &str) -> ProjectId {
        ProjectId::from(format!("prj-{name}"))
    }

    #[test]
    fn 등록한_라벨로_프로젝트와_슬롯을_되찾는다() {
        let store = WindowStore::default();
        store.register("editor-1".to_string(), project_id("a"), 3);

        assert_eq!(store.label_for(&project_id("a"), 3), Some("editor-1".to_string()));
        assert_eq!(store.forget("editor-1"), Some((project_id("a"), 3)));
    }

    /// `CloseRequested` and `Destroyed` both run the same cleanup, so whichever fires second must
    /// find nothing left to remove instead of re-running the tab-return hook for a window that is
    /// already gone.
    #[test]
    fn forget_은_멱등이라_두_번째_호출은_아무것도_돌려주지_않는다() {
        let store = WindowStore::default();
        store.register("editor-1".to_string(), project_id("a"), 1);

        assert!(store.forget("editor-1").is_some());
        assert_eq!(store.forget("editor-1"), None);
    }

    #[test]
    fn 등록되지_않은_라벨은_조회도_해제도_비어_있다() {
        let store = WindowStore::default();

        assert_eq!(store.forget("editor-9"), None);
        assert_eq!(store.label_for(&project_id("a"), 1), None);
    }

    #[test]
    fn 같은_프로젝트의_다른_슬롯은_서로_다른_창으로_구분된다() {
        let store = WindowStore::default();
        store.register("editor-1".to_string(), project_id("a"), 1);
        store.register("editor-2".to_string(), project_id("a"), 2);

        assert_eq!(store.label_for(&project_id("a"), 1), Some("editor-1".to_string()));
        assert_eq!(store.label_for(&project_id("a"), 2), Some("editor-2".to_string()));
        assert_eq!(store.label_for(&project_id("a"), 3), None);
    }

    #[test]
    fn 슬롯_번호가_같아도_프로젝트가_다르면_다른_창이다() {
        let store = WindowStore::default();
        store.register("editor-1".to_string(), project_id("a"), 1);
        store.register("editor-2".to_string(), project_id("b"), 1);

        assert_eq!(store.label_for(&project_id("b"), 1), Some("editor-2".to_string()));
    }

    /// The reverse lookup must stop finding a window the moment it is forgotten — otherwise
    /// `layout_move_tab_to_window` would try to close an OS window that no longer exists.
    #[test]
    fn 해제된_창은_역방향_조회에서도_사라진다() {
        let store = WindowStore::default();
        store.register("editor-1".to_string(), project_id("a"), 1);
        store.forget("editor-1");

        assert_eq!(store.label_for(&project_id("a"), 1), None);
    }
}

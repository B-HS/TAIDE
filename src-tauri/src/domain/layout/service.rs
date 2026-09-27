use taide_runtime::layout_actions;
use tauri::{AppHandle, Manager};

use super::types::{ClosedTab, ProjectLayout, Tab, TabKind};
use crate::error::AppResult;
use crate::ids::{PaneId, ProjectId, TabId};
use crate::platform::event_sink::TauriEventSink;
use crate::state::AppState;

pub use taide_layout::service::*;
pub use taide_runtime::layout_actions::finish_mutation;
pub(crate) use taide_runtime::layout_actions::flush_dirty_layouts;

pub(crate) const LAYOUT_FLUSH_INTERVAL_MS: u64 = 2_000;

pub type LayoutTabClosedObserver = Box<dyn Fn(&AppHandle, &Tab) + Send + Sync>;

/// Runs assembly-registered cleanup reactions after a closed tab's layout snapshot is committed.
pub struct LayoutTabClosedObservers(Vec<LayoutTabClosedObserver>);

impl LayoutTabClosedObservers {
    pub fn new(observers: Vec<LayoutTabClosedObserver>) -> Self {
        Self(observers)
    }

    fn notify(&self, app: &AppHandle, tab: &Tab) {
        for observer in &self.0 {
            observer(app, tab);
        }
    }
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
    layout_actions::open_tab_and_finish(&TauriEventSink(app), state, project_id, kind, title, target, preview).await
}

/// Closes a tab under the mutation guard and runs registered cleanup after committing the layout.
pub async fn close_tab_and_finish(app: &AppHandle, state: &AppState, tab_id: &TabId) -> AppResult<(ProjectId, ClosedTab, ProjectLayout)> {
    layout_actions::close_tab_and_finish(&TauriEventSink(app), state, tab_id, |tab| {
        app.state::<LayoutTabClosedObservers>().notify(app, tab)
    })
    .await
}

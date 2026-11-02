use egui::{Color32, Rect, Ui};
use taide_model::ids::{PaneId, ProjectId, ShellSlotId};
use taide_model::layout::Tab;

use crate::commands::ShellIntent;

pub trait EditorSurfaces {
    fn problems_open(&self, slot: &ShellSlotId) -> bool;
    fn problems_panel(&mut self, ui: &mut Ui, project: &ProjectId, slot: &ShellSlotId);
    fn tab_content(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        intents: &mut Vec<ShellIntent>,
    );
    /// Paints the tab's kind icon; `agent_color` is the activity color of the agent detected in a
    /// terminal tab, which then shows the agent glyph instead (`pane-tab-bar.tsx` `getTabIcon`).
    fn tab_icon(
        &mut self,
        ui: &Ui,
        rect: Rect,
        tab: &Tab,
        agent_color: Option<Color32>,
        title_color: Color32,
    );
}

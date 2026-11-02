use egui::Ui;
use taide_model::ids::ProjectId;

use crate::commands::ShellIntent;
use crate::shell::ExplorerMount;

pub trait ExplorerSearchSurfaces {
    /// State a sidebar panel render may change. It is taken out before the panel's error boundary
    /// (`project-shell.tsx` 150, `auxiliary-window-shell.tsx` 132) so the render only reads the
    /// host and returns its requests inside this value.
    type Explorer;
    fn begin_explorer(&mut self, project: &ProjectId, mount: ExplorerMount<'_>) -> Self::Explorer;
    fn explorer(
        &self,
        ui: &mut Ui,
        project: &ProjectId,
        mount: ExplorerMount<'_>,
        explorer: &mut Self::Explorer,
        intents: &mut Vec<ShellIntent>,
    );
    /// Puts the state back and applies the requests of a render that finished (or did not run).
    fn finish_explorer(
        &mut self,
        context: &egui::Context,
        project: &ProjectId,
        mount: ExplorerMount<'_>,
        explorer: Self::Explorer,
    );
    /// A render panicked: `explorer` may be inconsistent and its requests are dropped; the panel
    /// starts over like the remount after the original boundary's retry.
    fn discard_explorer(
        &mut self,
        project: &ProjectId,
        mount: ExplorerMount<'_>,
        explorer: Self::Explorer,
    );
    fn is_auxiliary_explorer_open(&self, _project: &ProjectId, _slot: u32) -> bool {
        false
    }
}

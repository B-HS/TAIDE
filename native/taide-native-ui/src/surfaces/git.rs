use egui::{Color32, Rect, Ui};
use taide_model::ids::ProjectId;

pub trait GitSurfaces {
    fn branch(&self, project: &ProjectId) -> Option<&str>;
    /// Paints the title bar's branch glyph (`title-bar.tsx` `GitBranch`).
    fn title_branch_icon(&mut self, _ui: &Ui, _rect: Rect, _color: Color32) {}
}

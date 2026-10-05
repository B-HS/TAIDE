use egui::{Id, Response};

pub struct Trigger {
    pub response: Response,
    pub label: String,
    pub align: egui::RectAlign,
    pub focus_target: Option<Id>,
}

pub fn wrap_button(
    ui: &mut egui::Ui,
    label: &str,
    disabled: bool,
    contents: impl FnOnce(&mut egui::Ui) -> Response,
) -> (Response, Trigger) {
    let sense = if disabled {
        egui::Sense::focusable_noninteractive()
    } else {
        egui::Sense::hover()
    };
    let wrapped = ui.scope_builder(egui::UiBuilder::new().sense(sense), |ui| {
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::GenericContainer);
        });
        contents(ui)
    });
    wrapped
        .response
        .ctx
        .accesskit_node_builder(wrapped.response.id, |node| {
            node.set_bounds(egui::accesskit::Rect {
                x0: wrapped.response.rect.left().into(),
                y0: wrapped.response.rect.top().into(),
                x1: wrapped.response.rect.right().into(),
                y1: wrapped.response.rect.bottom().into(),
            });
            if disabled && wrapped.response.enabled() {
                node.add_action(egui::accesskit::Action::Focus);
            }
        });
    let trigger = Trigger {
        response: wrapped.response,
        label: label.into(),
        align: egui::RectAlign::BOTTOM,
        focus_target: wrapped.inner.enabled().then_some(wrapped.inner.id),
    };
    (wrapped.inner, trigger)
}

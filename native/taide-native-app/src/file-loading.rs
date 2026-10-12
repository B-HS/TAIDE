use eframe::egui::{Align, Layout, Response, Ui};
use taide_model::locale::ResolvedLocale;
use taide_native_ui::presentation;

use crate::host::HostCommand;

pub(crate) fn show_refused(
    ui: &mut Ui,
    locale: &ResolvedLocale,
    path: &str,
) -> (Response, Option<HostCommand>) {
    ui.allocate_ui_with_layout(
        ui.available_size(),
        Layout::top_down(Align::Center).with_main_align(Align::Center),
        |ui| {
            ui.label(presentation::message(locale, "editor.cannotOpen", &[]));
            ui.label(presentation::message(
                locale,
                "editor.binaryOrTooLarge",
                &[],
            ));
            let button = ui.button(presentation::message(locale, "editor.openExternally", &[]));
            let command = button.clicked().then(|| HostCommand::OpenPath(path.into()));
            (button, command)
        },
    )
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    use std::collections::BTreeMap;

    const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);

    fn render(
        context: &egui::Context,
        locale: &ResolvedLocale,
        events: Vec<egui::Event>,
    ) -> (Response, Option<HostCommand>) {
        let mut result = None;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
                events,
                ..Default::default()
            },
            |ui| result = Some(show_refused(ui, locale, "/synthetic/refused.rs")),
        );
        output.textures_delta.clear();
        result.unwrap()
    }

    #[test]
    fn 실제_외부_열기_버튼은_한번의_typed_host_명령만_만든다() {
        let context = egui::Context::default();
        let locale = ResolvedLocale {
            id: "synthetic".into(),
            name: "synthetic".into(),
            messages: BTreeMap::from([
                ("editor.cannotOpen".into(), "Cannot open".into()),
                (
                    "editor.binaryOrTooLarge".into(),
                    "Binary or too large".into(),
                ),
                ("editor.openExternally".into(), "Open externally".into()),
            ]),
            warnings: Vec::new(),
        };
        let (button, command) = render(&context, &locale, Vec::new());
        assert!(command.is_none());
        let position = button.rect.center();
        let (_, command) = render(
            &context,
            &locale,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(command.is_none());
        let (_, command) = render(
            &context,
            &locale,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(
            matches!(command, Some(HostCommand::OpenPath(path)) if path == "/synthetic/refused.rs")
        );
        assert!(render(&context, &locale, Vec::new()).1.is_none());
    }
}

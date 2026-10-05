pub(crate) use taide_native_ui::status_chord::{Appearance, show};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::ChordStatus;
    use eframe::egui::{self, Color32};
    use taide_model::{locale::ResolvedLocale, theme::ResolvedTheme};

    const EXPECTED_TEXT_SIZE: f32 = 11.0;
    const EXPECTED_ICON_DOTS: usize = 9;

    #[test]
    fn chord_status는_원본문구와_불일치우선_글꼴_색상_아이콘을_표시한다() {
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap(),
        };
        let theme: ResolvedTheme = serde_json::from_value(serde_json::json!({
            "id": "synthetic-chord",
            "name": "Synthetic",
            "type": "dark",
            "colors": {"statusIndicator.warning": "#ffff00", "statusIndicator.error": "#ff0000"},
            "syntax": {},
            "terminal": {}
        }))
        .unwrap();
        let appearance = Appearance::new(&theme).unwrap();
        let context = egui::Context::default();

        for (status, expected, color) in [
            (
                ChordStatus {
                    shortcut: Some("Ctrl+K".into()),
                    ..Default::default()
                },
                "( Ctrl+K ) waiting for next key",
                Color32::YELLOW,
            ),
            (
                ChordStatus {
                    shortcut: Some("Ctrl+K".into()),
                    no_match: true,
                },
                "No matching shortcut",
                Color32::RED,
            ),
        ] {
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                assert!(show(ui, &locale, &appearance, &ChordStatus::default()).is_none());
                assert!(show(ui, &locale, &appearance, &status).is_some());
            });
            let text = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == expected => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(text.galley.job.sections[0].format.color, color);
            assert_eq!(
                text.galley.job.sections[0].format.font_id.size,
                EXPECTED_TEXT_SIZE
            );
            assert_eq!(output.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Circle(circle) if circle.fill == color)).count(), EXPECTED_ICON_DOTS);
            output.textures_delta.clear();
        }
    }
}

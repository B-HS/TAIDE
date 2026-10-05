use super::*;
use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_runtime::AppState;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 700.0);
const TOP_SPACE: f32 = 100.0;
const FOCUSED: f64 = 1.0;
const SETTLED: f64 = 2.0;
const CLOSED: f64 = 3.0;
const ANSI_START: f64 = 4.0;
const ANSI_INTERVAL: f64 = 5.0;
const ANSI_COUNT: usize = 16;
const FONT: f32 = 12.0;

#[test]
fn theme_tooltip은_picker와_ansi16의_공용_theme_ax와_bottom_top을_보존한다() {
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-theme-tooltip-{}", ProjectId::new())),
    ));
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = Appearance::new(&theme).unwrap();
        let picker_appearance = crate::theme_color_picker::Appearance::new(&theme).unwrap();
        let context = Context::default();
        context.enable_accesskit();
        let provider = Provider::default();
        let mut picker = crate::theme_color_picker::Picker::new("#408080".into());
        let render = |picker: &mut crate::theme_color_picker::Picker, time, events| {
            let mut triggers = Vec::new();
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    ..Default::default()
                },
                |ui| {
                    provider.begin_frame(&context);
                    ui.add_space(TOP_SPACE);
                    assert!(
                        picker
                            .show(
                                ui,
                                Id::new("picker"),
                                "#408080",
                                &locale,
                                &picker_appearance
                            )
                            .is_none()
                    );
                    triggers.extend(picker.take_tooltip());
                    triggers.extend(crate::theme_live_preview::show(ui, &theme, &locale).unwrap());
                    provider.show_triggers(&triggers, &appearance);
                    provider.finish_frame(&context);
                },
            );
            output.textures_delta.clear();
            (output, triggers)
        };
        render(&mut picker, 0.0, Vec::new());
        let (id, rect) = picker.input_traces()[0].unwrap();
        render(
            &mut picker,
            FOCUSED,
            vec![Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    target_node: id.accesskit_id(),
                    target_tree: egui::accesskit::TreeId::ROOT,
                    action: egui::accesskit::Action::Focus,
                    data: None,
                },
            )],
        );
        render(&mut picker, FOCUSED, Vec::new());
        let (opened, triggers) = render(&mut picker, SETTLED, Vec::new());
        let nodes = &opened
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes;
        let (tooltip_id, tooltip) = nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            .expect("actual color picker needs Tooltip role");
        assert_eq!(
            tooltip.label(),
            Some(crate::presentation::message(&locale, "themeEditor.pickColor", &[]).as_str())
        );
        let button = &nodes
            .iter()
            .find(|(node, _)| *node == id.accesskit_id())
            .unwrap()
            .1;
        assert_eq!(button.role(), egui::accesskit::Role::Button);
        assert_eq!(button.described_by(), [*tooltip_id]);
        assert!(tooltip.bounds().unwrap().y0 >= f64::from(rect.bottom()));
        assert_eq!(triggers.len(), ANSI_COUNT + 1);
        assert_eq!(triggers[0].align, egui::RectAlign::BOTTOM);
        let escape = || Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let (closed, _) = render(&mut picker, CLOSED, vec![escape()]);
        assert!(
            closed
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
        );
        for (index, trigger) in triggers[1..].iter().enumerate() {
            assert_eq!(trigger.label, crate::theme_editor_tokens::TERMINAL[index]);
            assert_eq!(trigger.align, egui::RectAlign::TOP);
            let start = ANSI_START + index as f64 * ANSI_INTERVAL;
            let events = vec![Event::PointerMoved(trigger.response.rect.center())];
            render(
                &mut picker,
                start,
                vec![Event::PointerMoved(SCREEN.to_pos2())],
            );
            render(&mut picker, start + FOCUSED, events.clone());
            render(&mut picker, start + SETTLED, events);
            let (opened, current) = render(&mut picker, start + CLOSED, Vec::new());
            let nodes = &opened
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            let tooltips = nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
                .collect::<Vec<_>>();
            assert_eq!(tooltips.len(), 1);
            let (tooltip_id, tooltip) = tooltips[0];
            assert_eq!(
                tooltip.label(),
                Some(trigger.label.as_str()),
                "index={index} target={:?} current={:?} hovered={} pointer={:?}",
                trigger.response.rect,
                current[index + 1].response.rect,
                current[index + 1].response.contains_pointer(),
                context.input(|input| input.pointer.hover_pos()),
            );
            let span = &nodes
                .iter()
                .find(|(node, _)| *node == trigger.response.id.accesskit_id())
                .unwrap()
                .1;
            assert_eq!(span.role(), egui::accesskit::Role::GenericContainer);
            assert_eq!(span.described_by(), [*tooltip_id]);
            assert!(tooltip.bounds().unwrap().y1 <= f64::from(trigger.response.rect.top()));
            assert!(opened.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == crate::presentation::color(&theme, "tooltip.background").unwrap())));
            let text = opened
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == trigger.label => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                text.galley.job.sections[0].format.font_id,
                egui::FontId::proportional(FONT)
            );
        }
    }
}

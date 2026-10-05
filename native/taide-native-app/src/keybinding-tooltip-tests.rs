use super::*;
use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_runtime::AppState;

const SCREEN: egui::Vec2 = egui::vec2(1000.0, 800.0);
const LAYOUT: f64 = 1.0;
const FOCUSED: f64 = 2.0;
const SETTLED: f64 = 3.0;
const CLOSED: f64 = 4.0;
const EXIT_MIDPOINT: f64 = 0.075;
const UNMOUNTED: f64 = 5.0;
const TOOLTIP_FONT: f32 = 12.0;
const OVERRIDES: &str = r#"[{"actionId":"font-size-up","key":"x","mods":["mod"]}]"#;

#[test]
fn keybinding_tooltip은_reset_unbind의_bottom_ax와_modal_escape를_보존한다() {
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
    };
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-keybinding-tooltip-{}", ProjectId::new())),
    ));
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = crate::tooltips::Appearance::new(&theme).unwrap();
        for key in ["settings.keymapResetOne", "settings.keymapUnbind"] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let provider = crate::tooltips::Provider::default();
            let mut editor = Editor::new(&theme, "en", true).unwrap();
            editor.open(&context);
            editor.query = "font-size-up".into();
            let render = |editor: &mut Editor, time, events| {
                let mut painted = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        provider.begin_frame(&context);
                        let output = editor
                            .show(ui.ctx(), &locale, Some(OVERRIDES), true)
                            .unwrap();
                        assert!(output.saves.is_empty() && output.warnings.is_empty());
                        output.show_tooltips(&locale, &provider, &appearance);
                        provider.finish_frame(&context);
                    },
                );
                painted.textures_delta.clear();
                painted
            };
            render(&mut editor, 0.0, Vec::new());
            let layout = render(&mut editor, LAYOUT, Vec::new());
            let label = message(&locale, key, &[]);
            let nodes = &layout
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            let (trigger_id, trigger) = nodes
                .iter()
                .find(|(_, node)| {
                    node.role() == egui::accesskit::Role::Button && node.label() == Some(&label)
                })
                .unwrap();
            let trigger_id = *trigger_id;
            let bounds = trigger.bounds().unwrap();
            render(
                &mut editor,
                FOCUSED,
                vec![Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        target_node: trigger_id,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        action: egui::accesskit::Action::Focus,
                        data: None,
                    },
                )],
            );
            render(&mut editor, FOCUSED, Vec::new());
            let opened = render(&mut editor, SETTLED, Vec::new());
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
            assert_eq!(
                tooltips.len(),
                1,
                "actual modal trigger needs Tooltip role: {key}"
            );
            let (tooltip_id, tooltip) = tooltips[0];
            assert_eq!(tooltip.label(), Some(label.as_str()));
            let trigger = &nodes.iter().find(|(id, _)| *id == trigger_id).unwrap().1;
            assert_eq!(trigger.role(), egui::accesskit::Role::Button);
            assert_eq!(trigger.described_by(), [*tooltip_id]);
            assert!(opened.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == color(&theme, "tooltip.background").unwrap())));
            let text = opened
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == label => Some(text),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                text.galley.job.sections[0].format.font_id,
                egui::FontId::proportional(TOOLTIP_FONT)
            );
            assert!(f64::from(text.pos.y) >= bounds.y1);
            let escape = || Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let closed = render(&mut editor, CLOSED, vec![escape()]);
            assert!(editor.is_open(), "tooltip Escape must not close its modal");
            let nodes = &closed
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes;
            assert!(
                nodes
                    .iter()
                    .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
            );
            assert!(
                nodes
                    .iter()
                    .find(|(id, _)| *id == trigger_id)
                    .unwrap()
                    .1
                    .described_by()
                    .is_empty()
            );
            render(&mut editor, CLOSED, Vec::new());
            render(&mut editor, CLOSED + EXIT_MIDPOINT, vec![escape()]);
            assert!(
                editor.is_open(),
                "exiting tooltip must retain Escape ownership"
            );
            render(&mut editor, CLOSED + EXIT_MIDPOINT, Vec::new());
            render(&mut editor, UNMOUNTED, vec![escape()]);
            assert!(!editor.is_open());
        }
    }
}

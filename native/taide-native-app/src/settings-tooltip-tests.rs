use super::*;

const SCREEN: egui::Vec2 = egui::vec2(1000.0, 900.0);
const LAYOUT: f64 = 1.0;
const FOCUSED: f64 = 2.0;
const SETTLED_OFFSET: f64 = 1.0;
const CLOSED_OFFSET: f64 = 2.0;
const CASE_INTERVAL: f64 = 4.0;
const FONT: f32 = 12.0;

#[test]
fn settings_tooltip은_실제_위치9버튼의_top_theme_ax를_보존한다() {
    let state = AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-settings-tooltip-{}", ProjectId::new())),
    ));
    let locale = taide_runtime::locale_actions::locale_get(&state, "en".into()).unwrap();
    let settings = Settings::default();
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = Appearance::new(&theme).unwrap();
        let tooltip_appearance = crate::tooltips::Appearance::new(&theme).unwrap();
        let context = egui::Context::default();
        context.enable_accesskit();
        let provider = crate::tooltips::Provider::default();
        let owner = Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let mut views = Views::default();
        let render = |views: &mut Views, time, events| {
            views.begin_frame();
            let mut output = None;
            let mut drawing = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                    ..Default::default()
                },
                |ui| {
                    provider.begin_frame(&context);
                    let shown = views.show(ui, owner.clone(), &settings, &locale, &appearance);
                    assert!(shown.changes.is_empty() && shown.themes.is_empty());
                    provider.show_triggers(&shown.tooltips, &tooltip_appearance);
                    output = Some(shown);
                    provider.finish_frame(&context);
                },
            );
            views.finish_frame();
            drawing.textures_delta.clear();
            (output.unwrap(), drawing)
        };
        render(&mut views, 0.0, Vec::new());
        let view = views.inspection_mut().get_mut(&owner).unwrap();
        view.active = Section::Interface;
        view.reset_scroll = true;
        let (_, layout) = render(&mut views, LAYOUT, Vec::new());
        for (index, position) in Position::ALL.iter().enumerate() {
            let label = message(&locale, position.label(), &[]);
            let (trigger_id, trigger) = layout
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(label.as_str()))
                .unwrap();
            let trigger_id = *trigger_id;
            let bounds = trigger.bounds().unwrap();
            let focused = FOCUSED + index as f64 * CASE_INTERVAL;
            render(
                &mut views,
                focused,
                vec![egui::Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        target_node: trigger_id,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        action: egui::accesskit::Action::Focus,
                        data: None,
                    },
                )],
            );
            render(&mut views, focused, Vec::new());
            let (output, opened) = render(&mut views, focused + SETTLED_OFFSET, Vec::new());
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
                "actual position needs Tooltip role: {}",
                position.value()
            );
            let (tooltip_id, tooltip) = tooltips[0];
            assert_eq!(tooltip.label(), Some(label.as_str()));
            let trigger = &nodes.iter().find(|(id, _)| *id == trigger_id).unwrap().1;
            assert_eq!(trigger.role(), egui::accesskit::Role::Button);
            assert_eq!(trigger.described_by(), [*tooltip_id]);
            assert!(tooltip.bounds().unwrap().y1 <= bounds.y0);
            assert_eq!(output.tooltips.len(), Position::ALL.len());
            assert!(
                output
                    .tooltips
                    .iter()
                    .all(|trigger| trigger.align == egui::RectAlign::TOP)
            );
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
                egui::FontId::proportional(FONT)
            );
            let (_, closed) = render(
                &mut views,
                focused + CLOSED_OFFSET,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
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
        }
    }
}

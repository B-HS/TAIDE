use super::*;
use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_runtime::AppState;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const FOCUSED: f64 = 1.0;
const SETTLED: f64 = 2.0;
const CLOSED: f64 = 3.0;
const CLICKED: f64 = 4.0;
const OUTSIDE: f64 = 5.0;
const HOVER: f64 = 6.0;
const HOVER_SETTLED: f64 = 8.0;
const FONT: f32 = 12.0;

#[test]
fn disabled_icon_span의_enter_space는_click_없이_tooltip을_닫지_않는다() {
    let mut failures = Vec::new();
    for disabled in [true, false] {
        for key in [egui::Key::Enter, egui::Key::Space] {
            let context = Context::default();
            context.enable_accesskit();
            let provider = Provider::default();
            let appearance = Appearance {
                background: Color32::BLACK,
                border: Color32::GRAY,
                foreground: Color32::WHITE,
            };
            let render = |time, events| {
                let mut response = None;
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        provider.begin_frame(ui.ctx());
                        let (button, trigger) = wrap_button(ui, "Wrapped action", disabled, |ui| {
                            ui.add_enabled(!disabled, egui::Button::new("Action"))
                        });
                        provider.show_triggers(std::slice::from_ref(&trigger), &appearance);
                        provider.finish_frame(ui.ctx());
                        response = Some((button, trigger.response));
                    },
                );
                output.textures_delta.clear();
                (output, response.unwrap())
            };
            let (_, (button, wrapper)) = render(0.0, Vec::new());
            let target = if disabled { wrapper.id } else { button.id };
            render(
                FOCUSED,
                vec![Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::Focus,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        target_node: target.accesskit_id(),
                        data: None,
                    },
                )],
            );
            render(SETTLED, Vec::new());
            let event = |pressed| Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let events = if key == egui::Key::Space {
                vec![event(true), event(false)]
            } else {
                vec![event(true)]
            };
            let (output, (button, wrapper)) = render(CLOSED, events);
            let owners = provider.0.lock().unwrap();
            let open = owners.get(&ViewportId::ROOT).unwrap().open == Some(wrapper.id);
            if open != disabled || (disabled && button.clicked()) {
                failures.push(format!(
                    "disabled={disabled} key={key:?} open={open} clicked={}",
                    button.clicked()
                ));
            }
            let node = output
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(id, _)| *id == wrapper.id.accesskit_id())
                .unwrap();
            if !node.1.described_by().is_empty() != disabled {
                failures.push(format!(
                    "disabled={disabled} key={key:?} stale wrapper description"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn icon_wrapper_tooltip은_theme_버튼자식과_disabled_span_focus를_보존한다() {
    use crate::ui_icons::{Icon, Icons};

    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-icon-tooltip-{}", ProjectId::new())),
    ));
    let locale = taide_runtime::locale_actions::locale_get(&state, "en".into()).unwrap();
    for name in ["vscode-dark-modern", "vscode-light-modern"] {
        let theme = taide_runtime::theme_actions::theme_get(&state, name.into()).unwrap();
        let appearance = Appearance::new(&theme).unwrap();
        for (icon, key) in [
            (Icon::ThemeReset, "themeEditor.resetToken"),
            (Icon::Copy, "themeEditor.duplicateTheme"),
            (Icon::Pencil, "themeEditor.editTheme"),
        ] {
            for disabled in [false, true] {
                let context = Context::default();
                context.enable_accesskit();
                let provider = Provider::default();
                let mut icons = Icons::new().unwrap();
                let label = crate::presentation::message(&locale, key, &[]);
                let render = |icons: &mut Icons, time, events| {
                    let mut shown = None;
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(time),
                            events,
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                            ..Default::default()
                        },
                        |ui| {
                            provider.begin_frame(&context);
                            icons.prepare(&context).unwrap();
                            let mut triggers = Vec::new();
                            let button = crate::theme_editor::icon_button(
                                ui,
                                icons,
                                icon,
                                Color32::WHITE,
                                &label,
                                disabled,
                                &mut triggers,
                            );
                            provider.show_triggers(&triggers, &appearance);
                            provider.finish_frame(&context);
                            shown = Some((button, triggers));
                        },
                    );
                    output.textures_delta.clear();
                    (output, shown.unwrap())
                };
                let (_, (button, triggers)) = render(&mut icons, 0.0, Vec::new());
                let button_id = button.id.accesskit_id();
                let target = if disabled {
                    triggers[0].response.id.accesskit_id()
                } else {
                    button_id
                };
                render(
                    &mut icons,
                    FOCUSED,
                    vec![Event::AccessKitActionRequest(
                        egui::accesskit::ActionRequest {
                            target_node: target,
                            target_tree: egui::accesskit::TreeId::ROOT,
                            action: egui::accesskit::Action::Focus,
                            data: None,
                        },
                    )],
                );
                render(&mut icons, FOCUSED, Vec::new());
                let (opened, (button, triggers)) = render(&mut icons, SETTLED, Vec::new());
                let nodes = &opened
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes;
                let (tooltip_id, tooltip) = nodes
                    .iter()
                    .find(|(_, node)| node.role() == egui::accesskit::Role::Tooltip)
                    .expect("actual IconButton needs Tooltip role");
                assert_eq!(tooltip.label(), Some(label.as_str()));
                assert_eq!(triggers.len(), 1);
                let wrapper = &triggers[0].response;
                assert_ne!(wrapper.id, button.id);
                assert_eq!(wrapper.rect, button.rect);
                assert_eq!(button.enabled(), !disabled);
                assert!(wrapper.enabled());
                let span = &nodes
                    .iter()
                    .find(|(id, _)| *id == wrapper.id.accesskit_id())
                    .unwrap()
                    .1;
                assert_eq!(span.role(), egui::accesskit::Role::GenericContainer);
                assert_eq!(span.described_by(), [*tooltip_id]);
                assert_eq!(
                    span.supports_action(egui::accesskit::Action::Focus),
                    disabled
                );
                assert!(span.children().contains(&button_id));
                let child = &nodes.iter().find(|(id, _)| *id == button_id).unwrap().1;
                assert_eq!(child.role(), egui::accesskit::Role::Button);
                assert_eq!(child.label(), Some(label.as_str()));
                assert_eq!(child.is_disabled(), disabled);
                assert!(child.described_by().is_empty());
                assert!(tooltip.bounds().unwrap().y0 >= f64::from(wrapper.rect.bottom()));
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
                let (closed, (_, triggers)) = render(
                    &mut icons,
                    CLOSED,
                    vec![Event::Key {
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
                        .find(|(id, _)| *id == triggers[0].response.id.accesskit_id())
                        .unwrap()
                        .1
                        .described_by()
                        .is_empty()
                );
                let (_, (button, triggers)) = render(
                    &mut icons,
                    CLICKED,
                    vec![Event::AccessKitActionRequest(
                        egui::accesskit::ActionRequest {
                            target_node: button_id,
                            target_tree: egui::accesskit::TreeId::ROOT,
                            action: egui::accesskit::Action::Click,
                            data: None,
                        },
                    )],
                );
                assert_eq!(button.clicked(), !disabled);
                render(
                    &mut icons,
                    OUTSIDE,
                    vec![Event::PointerMoved(SCREEN.to_pos2())],
                );
                let point = triggers[0].response.rect.center();
                render(&mut icons, HOVER, vec![Event::PointerMoved(point)]);
                render(
                    &mut icons,
                    HOVER + FOCUSED,
                    vec![Event::PointerMoved(point)],
                );
                let (hovered, (button, _)) = render(&mut icons, HOVER_SETTLED, Vec::new());
                assert!(!button.clicked());
                assert!(
                    hovered
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .any(|(_, node)| node.role() == egui::accesskit::Role::Tooltip
                            && node.label() == Some(label.as_str()))
                );
            }
        }
    }
}

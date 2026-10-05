use egui::{Event, Key, Modifiers, RawInput, os::OperatingSystem};
use taide_native_ui::keymap::{Context, Decision, Route, Windows};

#[test]
fn 공용_route는_컴파일_target이_아닌_실제_host의_command_modifier를_사용한다() {
    for (os, modifiers, opposite) in [
        (
            OperatingSystem::Mac,
            Modifiers::MAC_CMD | Modifiers::COMMAND,
            Modifiers::CTRL | Modifiers::COMMAND,
        ),
        (
            OperatingSystem::Windows,
            Modifiers::CTRL | Modifiers::COMMAND,
            Modifiers::MAC_CMD | Modifiers::COMMAND,
        ),
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let mut windows = Windows::default();
        for (modifiers, expected) in [
            (modifiers, Decision::Dispatch("quick-open".into())),
            (opposite, Decision::None),
        ] {
            let event = Event::Key {
                key: Key::P,
                physical_key: Some(Key::P),
                pressed: true,
                repeat: false,
                modifiers,
            };
            let mut output = context.run_ui(
                RawInput {
                    events: vec![event.clone()],
                    ..Default::default()
                },
                |ui| {
                    let mut observed = None;
                    let handled = windows
                        .route(
                            Route {
                                context: ui.ctx(),
                                event: &event,
                                index: 0,
                                scope: Context::default(),
                                composing: false,
                                overrides: None,
                            },
                            |decision| {
                                observed = Some(decision.clone());
                                matches!(decision, Decision::Dispatch(_))
                            },
                        )
                        .unwrap();
                    assert_eq!(observed, Some(expected.clone()));
                    assert_eq!(handled, matches!(expected, Decision::Dispatch(_)));
                },
            );
            output.textures_delta.clear();
        }
    }
}

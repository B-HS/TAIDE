use eframe::egui::{self, Event, Key};

use super::*;

const BEFORE_DEADLINE: Duration = Duration::from_millis(1);

fn frame(
    windows: &mut Windows,
    context: &egui::Context,
    key: Key,
    editor: bool,
    expected: Decision,
) {
    let modifiers = if cfg!(target_os = "macos") {
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
    } else {
        egui::Modifiers::CTRL | egui::Modifiers::COMMAND
    };
    let event = Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    };
    let mut output = context.run_ui(
        egui::RawInput {
            events: vec![event],
            ..Default::default()
        },
        |ui| {
            let event = ui.input(|input| input.events[0].clone());
            let mut observed_deadline = None;
            for _ in 0..2 {
                windows
                    .route(
                        Route {
                            context: ui.ctx(),
                            event: &event,
                            index: 0,
                            scope: Context {
                                terminal: false,
                                editor,
                            },
                            composing: false,
                            overrides: None,
                        },
                        |decision| {
                            if editor && matches!(decision, Decision::EnterChord) {
                                return true;
                            }
                            assert_eq!(*decision, expected);
                            matches!(
                                decision,
                                Decision::EnterChord
                                    | Decision::NoMatch
                                    | Decision::ResolveChord(_)
                            )
                        },
                    )
                    .unwrap();
                let deadline = windows.windows[&ui.ctx().viewport_id()].no_match_until;
                if let Some(previous) = observed_deadline {
                    assert_eq!(deadline, previous);
                }
                observed_deadline = Some(deadline);
            }
        },
    );
    output.textures_delta.clear();
}

#[test]
fn chord_status는_실제viewport_route의_pending_유예_불일치_단일타이머와_경계를_보존한다() {
    let context = egui::Context::default();
    let mut windows = Windows::default();
    assert_eq!(
        windows.chord_status(&context, Instant::now(), true),
        ChordStatus::default()
    );
    frame(&mut windows, &context, Key::K, false, Decision::EnterChord);
    let started = windows.windows[&context.viewport_id()]
        .map
        .pending
        .as_ref()
        .unwrap()
        .started;
    assert_eq!(
        windows
            .chord_status(&context, started, true)
            .shortcut
            .as_deref(),
        Some("⌘K")
    );
    assert_eq!(
        windows
            .chord_status(&context, started, false)
            .shortcut
            .as_deref(),
        Some("Ctrl+K")
    );
    assert!(
        windows
            .chord_status(&context, started + CHORD_TIMEOUT - BEFORE_DEADLINE, true)
            .shortcut
            .is_some()
    );
    assert_eq!(
        windows.chord_status(&context, started + CHORD_TIMEOUT, true),
        ChordStatus::default()
    );
    frame(&mut windows, &context, Key::G, false, Decision::NoMatch);
    let deadline = windows.windows[&context.viewport_id()]
        .no_match_until
        .unwrap();
    assert!(
        windows
            .chord_status(&context, deadline - BEFORE_DEADLINE, true)
            .no_match
    );
    assert_eq!(
        windows.chord_status(&context, deadline, true),
        ChordStatus::default()
    );
    frame(&mut windows, &context, Key::K, false, Decision::EnterChord);
    let status = windows.chord_status(&context, Instant::now(), true);
    assert!(status.no_match);
    assert_eq!(status.shortcut.as_deref(), Some("⌘K"));
    windows.clear_chord(context.viewport_id());
    let status = windows.chord_status(&context, Instant::now(), true);
    assert!(status.no_match);
    assert!(status.shortcut.is_none());
    windows.clear();
    frame(
        &mut windows,
        &context,
        Key::K,
        true,
        Decision::ObserveEditorPrefix,
    );
    let started = windows.windows[&context.viewport_id()]
        .map
        .editor_deferral
        .unwrap();
    assert_eq!(
        windows
            .chord_status(&context, started, true)
            .shortcut
            .as_deref(),
        Some("⌘K")
    );
    assert_eq!(
        windows.chord_status(&context, started + CHORD_TIMEOUT, true),
        ChordStatus::default()
    );
    windows.retain(|_| false);
    assert_eq!(
        windows.chord_status(&context, Instant::now(), true),
        ChordStatus::default()
    );
}

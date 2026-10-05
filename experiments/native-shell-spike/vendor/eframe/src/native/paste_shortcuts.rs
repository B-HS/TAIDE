use std::collections::HashMap;

use crate::egui::{Event, Key, Modifiers, RawInput};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key as WinitKey, KeyCode, NamedKey, PhysicalKey};
use winit::window::WindowId;

#[derive(Default)]
pub(super) struct PasteShortcuts {
    modifiers: HashMap<WindowId, Modifiers>,
}

impl PasteShortcuts {
    pub(super) fn retain_windows(&mut self, mut is_live: impl FnMut(&WindowId) -> bool) {
        self.modifiers.retain(|window, _| is_live(window));
    }

    pub(super) fn on_window_event(
        &mut self,
        window: WindowId,
        event: &WindowEvent,
        first_event: usize,
        input: &mut RawInput,
    ) -> bool {
        self.observe(window, first_event, input);
        match event {
            WindowEvent::Focused(false) | WindowEvent::Destroyed => {
                self.modifiers.remove(&window);
                false
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => self.preserve_key(
                window,
                &event.logical_key,
                event.physical_key,
                event.state,
                first_event,
                input,
            ),
            _ => false,
        }
    }

    fn observe(&mut self, window: WindowId, first_event: usize, input: &RawInput) {
        for event in input.events.iter().skip(first_event) {
            if let Event::ModifiersChanged(modifiers) = event {
                self.modifiers.insert(window, *modifiers);
            }
        }
    }

    pub(super) fn preserve_key(
        &self,
        window: WindowId,
        logical: &WinitKey,
        physical: PhysicalKey,
        state: ElementState,
        first_event: usize,
        input: &mut RawInput,
    ) -> bool {
        if state != ElementState::Pressed || input.events.len() != first_event {
            return false;
        }
        let modifiers = self.modifiers.get(&window).copied().unwrap_or_default();
        let physical = match physical {
            PhysicalKey::Code(KeyCode::KeyV) => Some(Key::V),
            PhysicalKey::Code(KeyCode::Insert) => Some(Key::Insert),
            _ => None,
        };
        let logical = match logical {
            WinitKey::Character(value) => Key::from_name(value),
            WinitKey::Named(NamedKey::Paste) => Some(Key::Paste),
            WinitKey::Named(NamedKey::Insert) => Some(Key::Insert),
            WinitKey::Named(value) => Key::from_name(&format!("{value:?}")),
            WinitKey::Unidentified(_) | WinitKey::Dead(_) => None,
        };
        let Some(key) = logical.or(physical) else {
            return false;
        };
        let paste = key == Key::Paste
            || (modifiers.command && key == Key::V)
            || (cfg!(target_os = "windows") && modifiers.shift && key == Key::Insert);
        if !paste {
            return false;
        }
        for pressed in [true, false] {
            input.events.push(Event::Key {
                key: Key::Paste,
                physical_key: physical,
                pressed,
                repeat: false,
                modifiers,
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OTHER_WINDOW: u64 = 2;

    #[test]
    fn 빈_clipboard의_paste_key는_창별_modifier와_실제_key를_보존한다() {
        let window = WindowId::dummy();
        let other = WindowId::from(OTHER_WINDOW);
        let command = if cfg!(target_os = "macos") {
            Modifiers::COMMAND | Modifiers::MAC_CMD
        } else {
            Modifiers::COMMAND | Modifiers::CTRL
        };
        let mut adapter = PasteShortcuts::default();
        let mut input = RawInput::default();
        input.events.push(Event::ModifiersChanged(command));
        adapter.observe(window, 0, &input);
        let first_event = input.events.len();
        let physical = PhysicalKey::Code(KeyCode::KeyV);
        let logical = WinitKey::Character("v".into());
        assert!(!adapter.preserve_key(
            other,
            &logical,
            physical,
            ElementState::Pressed,
            first_event,
            &mut input
        ));
        assert!(adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Pressed,
            first_event,
            &mut input
        ));
        assert_eq!(
            input.events.get(first_event),
            Some(&Event::Key {
                key: Key::Paste,
                physical_key: Some(Key::V),
                pressed: true,
                repeat: false,
                modifiers: command,
            })
        );
        assert!(matches!(
            input.events.last(),
            Some(Event::Key {
                key: Key::Paste,
                pressed: false,
                ..
            })
        ));
        assert!(!adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Pressed,
            first_event,
            &mut input
        ));
        input.events.clear();
        assert!(!adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Released,
            0,
            &mut input
        ));
        input.events.push(Event::Paste("actual text".into()));
        assert!(!adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        input.events.clear();
        assert!(!adapter.preserve_key(
            window,
            &WinitKey::Character("z".into()),
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        assert!(adapter.preserve_key(
            window,
            &WinitKey::Character("한".into()),
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        input.events.clear();
        adapter.on_window_event(window, &WindowEvent::Focused(false), 0, &mut input);
        assert!(!adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        input
            .events
            .push(Event::ModifiersChanged(command | Modifiers::ALT));
        adapter.observe(window, 0, &input);
        input.events.clear();
        assert!(adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        assert!(matches!(input.events.last(), Some(Event::Key { modifiers, .. }) if modifiers.alt));
        input.events.clear();
        input.events.push(Event::ModifiersChanged(command));
        adapter.observe(other, 0, &input);
        adapter.retain_windows(|id| *id == other);
        input.events.clear();
        assert!(!adapter.preserve_key(
            window,
            &logical,
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        assert!(adapter.preserve_key(
            other,
            &logical,
            physical,
            ElementState::Pressed,
            0,
            &mut input
        ));
        input.events.clear();
        adapter.retain_windows(|_| false);
        assert!(adapter.modifiers.is_empty());
        adapter.on_window_event(window, &WindowEvent::Destroyed, 0, &mut input);
        assert!(adapter.modifiers.is_empty());
    }
}

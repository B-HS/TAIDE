use serde_json::Value;

use super::super::{KeyEvent, MODIFIER_ONLY, Stage, capture_key, editor_key};
use super::Binding;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Row { id: String, first: Option<Binding> },
    Search,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Warning(&'static str),
    Assign { id: String, binding: Binding },
}

#[derive(Default)]
pub struct Capture {
    pub target: Option<Target>,
    pub searched_key: Option<Binding>,
}

impl Capture {
    pub fn egui_key(&mut self, event: &super::super::egui::Event, is_mac: bool) -> Effect {
        let super::super::egui::Event::Key {
            key,
            physical_key,
            pressed: true,
            repeat,
            modifiers,
        } = event
        else {
            return Effect::None;
        };
        let code = physical_key.map(super::super::dom_code);
        self.key(
            &KeyEvent {
                key: super::super::dom_key(*key),
                code: code.as_deref(),
                modifiers: super::super::Modifiers {
                    meta: modifiers.mac_cmd,
                    control: modifiers.ctrl,
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                },
                repeat: *repeat,
                composing: false,
            },
            is_mac,
        )
    }

    pub fn start_row(&mut self, id: String) {
        self.target = Some(Target::Row { id, first: None });
    }

    pub fn toggle_search(&mut self) -> bool {
        let entering = !matches!(self.target, Some(Target::Search));
        self.target = entering.then_some(Target::Search);
        self.searched_key = None;
        entering
    }

    pub fn blur(&mut self) {
        self.target = None;
    }

    pub fn cancel(&mut self) {
        if matches!(self.target, Some(Target::Search)) {
            self.searched_key = None;
        }
        self.target = None;
    }

    pub fn confirm_single(&mut self) -> Effect {
        let Some(Target::Row {
            id,
            first: Some(first),
        }) = &self.target
        else {
            return Effect::None;
        };
        self.assign(id.clone(), first.clone())
    }

    pub fn key(&mut self, event: &KeyEvent<'_>, is_mac: bool) -> Effect {
        let Some(target) = self.target.clone() else {
            return Effect::None;
        };
        if event.key == "Escape" {
            self.cancel();
            return Effect::None;
        }
        if MODIFIER_ONLY.contains(&event.key) {
            return Effect::None;
        }
        let mut mods = Vec::new();
        if (is_mac && event.modifiers.meta) || (!is_mac && event.modifiers.control) {
            mods.push(Value::String("mod".into()));
        }
        if is_mac && event.modifiers.control {
            mods.push(Value::String("ctrl".into()));
        }
        if event.modifiers.shift {
            mods.push(Value::String("shift".into()));
        }
        if event.modifiers.alt {
            mods.push(Value::String("alt".into()));
        }
        let first = Stage {
            key: capture_key(event),
            command: mods.iter().any(|value| value.as_str() == Some("mod")),
            control: mods.iter().any(|value| value.as_str() == Some("ctrl")),
            shift: event.modifiers.shift,
            alt: event.modifiers.alt,
            mods,
        };
        if matches!(target, Target::Search) {
            self.searched_key = Some(Binding {
                first,
                second: None,
            });
            return Effect::None;
        }
        let Target::Row { id, first: pending } = target else {
            unreachable!();
        };
        if let Some(pending) = pending {
            if event.key == "Enter" && first.mods.is_empty() {
                return self.assign(id, pending);
            }
            return self.assign(
                id,
                Binding {
                    first: pending.first,
                    second: Some(first),
                },
            );
        }
        if first.mods.is_empty() {
            return Effect::Warning("settings.keymapModifierRequired");
        }
        if id.starts_with("monaco.") && !editor_key(&first.key) {
            return Effect::Warning("settings.keymapKeyNotBindable");
        }
        self.target = Some(Target::Row {
            id,
            first: Some(Binding {
                first,
                second: None,
            }),
        });
        Effect::None
    }

    fn assign(&mut self, id: String, binding: Binding) -> Effect {
        if binding.first.mods.is_empty() {
            return Effect::None;
        }
        if id.starts_with("monaco.")
            && (!editor_key(&binding.first.key)
                || binding
                    .second
                    .as_ref()
                    .is_some_and(|stage| !editor_key(&stage.key)))
        {
            return Effect::Warning("settings.keymapKeyNotBindable");
        }
        self.target = None;
        Effect::Assign { id, binding }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::Modifiers;
    use super::*;
    use serde_json::json;

    #[test]
    fn keybinding_capture는_single_chord_검색_blur와_원본_bindable경계를_보존한다() {
        let event = |key| KeyEvent {
            key,
            code: None,
            modifiers: Modifiers::default(),
            repeat: false,
            composing: false,
        };
        let modified = |key| KeyEvent {
            modifiers: Modifiers {
                meta: true,
                ..Default::default()
            },
            ..event(key)
        };
        let mut capture = Capture::default();
        capture.start_row("save".into());
        assert_eq!(capture.key(&event("Shift"), true), Effect::None);
        assert_eq!(
            capture.key(&event("x"), true),
            Effect::Warning("settings.keymapModifierRequired")
        );
        assert_eq!(capture.key(&modified("k"), true), Effect::None);
        let Effect::Assign { id, binding } = capture.key(&event("z"), true) else {
            panic!("expected chord")
        };
        assert_eq!(id, "save");
        assert_eq!(
            binding.json(),
            json!({"key":"k","mods":["mod"],"chord":{"key":"z","mods":[]}})
        );
        assert_eq!(binding.label(true), "⌘K Z");
        assert_eq!(binding.label(false), "Ctrl+K Z");
        assert!(capture.target.is_none());
        capture.start_row("save".into());
        capture.key(&modified("k"), true);
        let Effect::Assign { binding, .. } = capture.key(&event("Enter"), true) else {
            panic!("expected single")
        };
        assert_eq!(binding.json(), json!({"key":"k","mods":["mod"]}));
        capture.start_row("save".into());
        capture.key(&modified("k"), true);
        let Effect::Assign { binding, .. } = capture.key(&modified("Enter"), true) else {
            panic!("expected modified Enter chord")
        };
        assert_eq!(
            binding.json(),
            json!({"key":"k","mods":["mod"],"chord":{"key":"Enter","mods":["mod"]}})
        );
        capture.start_row("monaco.actions.find".into());
        assert_eq!(
            capture.key(&modified("F13"), true),
            Effect::Warning("settings.keymapKeyNotBindable")
        );
        capture.key(&modified("F12"), true);
        assert_eq!(
            capture.key(&event("F13"), true),
            Effect::Warning("settings.keymapKeyNotBindable")
        );
        assert!(matches!(
            capture.target,
            Some(Target::Row { first: Some(_), .. })
        ));
        assert!(matches!(capture.confirm_single(), Effect::Assign { .. }));
        capture.start_row("save".into());
        capture.key(&modified("k"), true);
        capture.key(&event("Escape"), true);
        assert!(capture.target.is_none());
        assert!(capture.toggle_search());
        capture.key(&event("x"), true);
        assert_eq!(
            capture.searched_key.as_ref().unwrap().json(),
            json!({"key":"x","mods":[]})
        );
        capture.blur();
        assert!(capture.target.is_none());
        assert!(capture.searched_key.is_some());
        capture.toggle_search();
        capture.key(&event("ArrowUp"), true);
        assert_eq!(
            capture.searched_key.as_ref().unwrap().json(),
            json!({"key":"ArrowUp","mods":[]})
        );
        capture.key(&event("Escape"), true);
        assert!(capture.target.is_none() && capture.searched_key.is_none());
        assert!(capture.toggle_search());
        let option = KeyEvent {
            key: "˚",
            code: Some("KeyK"),
            modifiers: Modifiers {
                alt: true,
                ..Default::default()
            },
            ..event("˚")
        };
        capture.key(&option, true);
        assert_eq!(
            capture.searched_key.as_ref().unwrap().json(),
            json!({"key":"k","mods":["alt"]})
        );
        assert!(!capture.toggle_search());
        assert!(capture.target.is_none() && capture.searched_key.is_none());
        capture.start_row("save".into());
        let control = KeyEvent {
            modifiers: Modifiers {
                control: true,
                shift: true,
                alt: true,
                ..Default::default()
            },
            ..event("ArrowUp")
        };
        capture.key(&control, false);
        let Effect::Assign { binding, .. } = capture.confirm_single() else {
            panic!("expected nonmac single")
        };
        assert_eq!(
            binding.json(),
            json!({"key":"ArrowUp","mods":["mod","shift","alt"]})
        );
        assert_eq!(binding.label(false), "Ctrl+Alt+Shift+↑");
        let binding =
            Binding::parse(&json!({"key":"ß","mods":["mod","ctrl","alt","shift"]})).unwrap();
        assert_eq!(binding.label(true), "⌃⌥⇧⌘SS");
        assert_eq!(binding.label(false), "Ctrl+Alt+Shift+SS");
    }
}

use std::collections::HashMap;
use std::time::Duration;

use super::Instant;
use super::command_registry;
use super::egui;

use serde_json::Value;
use taide_model::error::{AppError, AppResult};

const DEFAULTS: &str = include_str!("keymap-defaults.json");
const EDITOR_DEFAULTS: &str = include_str!("editor-keymap-defaults.json");
#[path = "keybinding-catalog.rs"]
pub mod catalog;
const CHORD_TIMEOUT: Duration = Duration::from_secs(5);
const NO_MATCH_DURATION: Duration = Duration::from_millis(1500);
const EDITOR_GROUPS: [&str; 7] = [
    "focus-group-left",
    "focus-group-right",
    "focus-group-up",
    "focus-group-down",
    "move-tab-to-group-left",
    "move-tab-to-group-right",
    "close-all-tabs",
];
const MODIFIER_ONLY: [&str; 10] = [
    "Shift",
    "Control",
    "Alt",
    "Meta",
    "AltGraph",
    "CapsLock",
    "NumLock",
    "ScrollLock",
    "OS",
    "Fn",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub meta: bool,
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
}

pub struct KeyEvent<'event> {
    pub key: &'event str,
    pub code: Option<&'event str>,
    pub modifiers: Modifiers,
    pub repeat: bool,
    pub composing: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Context {
    pub terminal: bool,
    pub editor: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Stage {
    key: String,
    mods: Vec<Value>,
    command: bool,
    control: bool,
    shift: bool,
    alt: bool,
}

impl Stage {
    fn parse(value: &Value) -> Option<Self> {
        let key = value.get("key")?.as_str()?.to_owned();
        let mods = value.get("mods")?.as_array()?;
        let contains = |name| mods.iter().any(|value| value.as_str() == Some(name));
        Some(Self {
            key,
            mods: mods.clone(),
            command: contains("mod"),
            control: contains("ctrl"),
            shift: contains("shift"),
            alt: contains("alt"),
        })
    }

    fn matches(&self, event: &KeyEvent<'_>, is_mac: bool) -> bool {
        if self.key.is_empty() {
            return false;
        }
        let expected = canonical(&self.key);
        if expected != canonical(event.key) && expected != event_key(event) {
            return false;
        }
        event.modifiers
            == Modifiers {
                meta: self.command && is_mac,
                control: self.control || (self.command && !is_mac),
                shift: self.shift,
                alt: self.alt,
            }
    }
}

#[derive(Clone)]
struct Entry {
    id: String,
    first: Stage,
    second: Option<Stage>,
    when: Option<String>,
}

impl Entry {
    fn apply_override(&mut self, value: &Value) {
        let Some(first) = Stage::parse(value) else {
            return;
        };
        if self.second.is_some()
            && (self.first.key.to_lowercase() != first.key.to_lowercase()
                || self.first.mods.len() != first.mods.len()
                || !self
                    .first
                    .mods
                    .iter()
                    .all(|modifier| first.mods.contains(modifier)))
        {
            self.when = None;
        }
        self.first = first;
        self.second = value.get("chord").and_then(Stage::parse);
    }

    fn enabled(&self, context: Context) -> bool {
        match self.when.as_deref() {
            None => true,
            Some("terminalFocus") => context.terminal,
            Some("!terminalFocus") => !context.terminal,
            Some("editorTextFocus") => context.editor,
            Some("!editorTextFocus") => !context.editor,
            Some(_) => false,
        }
    }
}

struct CommandBinding {
    id: String,
    first: Stage,
    second: Option<Stage>,
    is_mac_only: bool,
}

#[derive(Clone)]
struct EditorBinding {
    entry: Entry,
    platform: Option<String>,
}

impl CommandBinding {
    fn matches(&self, event: &KeyEvent<'_>, is_mac: bool) -> bool {
        self.second.is_none() && (is_mac || !self.is_mac_only) && self.first.matches(event, is_mac)
    }
}

struct Pending {
    candidates: Vec<usize>,
    started: Instant,
}

struct EditorPending {
    prefix: Stage,
    started: Instant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    None,
    Ignore,
    EnterChord,
    NoMatch,
    Dispatch(String),
    ResolveChord(String),
    ObserveEditorPrefix,
    DeferToEditor,
}

pub struct Keymap {
    base: Vec<Entry>,
    entries: Vec<Entry>,
    overrides: Option<String>,
    pending: Option<Pending>,
    editor_deferral: Option<Instant>,
    editor_prefixes: Vec<Stage>,
    editor_pending: Option<EditorPending>,
    commands: &'static command_registry::Registry,
    command_bindings: Vec<CommandBinding>,
    editor_base: Vec<EditorBinding>,
    editor_bindings: Vec<EditorBinding>,
}

struct Window {
    map: Keymap,
    frame: u64,
    decisions: HashMap<usize, Decision>,
    editor_decisions: HashMap<usize, (Decision, bool)>,
    claimed_text: Option<(usize, egui::Key)>,
    no_match_until: Option<Instant>,
}

pub use super::status_chord::ChordStatus;

fn note_chord(window: &mut Window, context: &egui::Context, decision: &Decision, now: Instant) {
    if matches!(decision, Decision::NoMatch) {
        window.no_match_until = Some(now + NO_MATCH_DURATION);
    }
    if !matches!(decision, Decision::None | Decision::Ignore) {
        context.request_repaint();
    }
}

#[derive(Default)]
pub struct Windows {
    windows: HashMap<egui::ViewportId, Window>,
}

pub struct Route<'event> {
    pub context: &'event egui::Context,
    pub event: &'event egui::Event,
    pub index: usize,
    pub scope: Context,
    pub composing: bool,
    pub overrides: Option<&'event str>,
}

impl Windows {
    pub fn route(
        &mut self,
        request: Route<'_>,
        mut handler: impl FnMut(&Decision) -> bool,
    ) -> AppResult<bool> {
        use egui::Event;
        let Route {
            context,
            event,
            index,
            scope,
            composing,
            overrides,
        } = request;
        let frame = context.cumulative_frame_nr();
        let window = match self.windows.entry(context.viewport_id()) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => entry.insert(Window {
                map: Keymap::new()?,
                frame,
                decisions: HashMap::new(),
                editor_decisions: HashMap::new(),
                claimed_text: None,
                no_match_until: None,
            }),
        };
        if window.frame != frame {
            window.frame = frame;
            window.decisions.clear();
            window.editor_decisions.clear();
            window.claimed_text = None;
        }
        if let Some((previous, key)) = window.claimed_text
            && let Event::Text(text) = event
            && previous.checked_add(1) == Some(index)
            && egui::Key::from_name(text) == Some(key)
        {
            return Ok(true);
        }
        if !matches!(event, Event::Key { pressed: true, .. }) {
            return Ok(false);
        }
        window.map.update(overrides);
        let decision = match window.decisions.get(&index) {
            Some(decision) if index != usize::MAX => decision.clone(),
            _ => {
                let decision = window.map.decide_egui_for_platform(
                    event,
                    scope,
                    composing,
                    context.os().is_mac(),
                    Instant::now(),
                );
                note_chord(window, context, &decision, Instant::now());
                if index != usize::MAX {
                    window.decisions.insert(index, decision.clone());
                }
                decision
            }
        };
        let mut handled = handler(&decision);
        if !handled
            && scope.editor
            && window.map.pending.is_none()
            && matches!(
                decision,
                Decision::None
                    | Decision::Ignore
                    | Decision::ObserveEditorPrefix
                    | Decision::DeferToEditor
                    | Decision::Dispatch(_)
            )
        {
            let (editor, previously_handled) = match window.editor_decisions.get(&index) {
                Some(cached) if index != usize::MAX => cached.clone(),
                _ => {
                    let decision = window.map.decide_editor_egui(
                        event,
                        composing,
                        context.os(),
                        Instant::now(),
                    );
                    note_chord(window, context, &decision, Instant::now());
                    (decision, false)
                }
            };
            handled = previously_handled
                || (!matches!(editor, Decision::None | Decision::Ignore) && handler(&editor));
            if index != usize::MAX {
                window.editor_decisions.insert(index, (editor, handled));
            }
        }
        if handled
            && let Event::Key { key, modifiers, .. } = event
            && !modifiers.ctrl
            && !modifiers.mac_cmd
            && !modifiers.command
            && index != usize::MAX
        {
            window.claimed_text = Some((index, *key));
        }
        Ok(handled)
    }

    pub fn retain(&mut self, mut live: impl FnMut(egui::ViewportId) -> bool) {
        self.windows.retain(|viewport, _| live(*viewport));
    }

    pub fn chord_status(&self, context: &egui::Context, now: Instant, is_mac: bool) -> ChordStatus {
        let Some(window) = self.windows.get(&context.viewport_id()) else {
            return ChordStatus::default();
        };
        let pending = window
            .map
            .pending
            .as_ref()
            .filter(|pending| now < pending.started + CHORD_TIMEOUT)
            .and_then(|pending| {
                Some((
                    catalog::stage_label(
                        &window.map.entries.get(*pending.candidates.first()?)?.first,
                        is_mac,
                    ),
                    pending.started + CHORD_TIMEOUT,
                ))
            });
        let deferral = window
            .map
            .editor_deferral
            .filter(|started| now < *started + CHORD_TIMEOUT)
            .map(|started| {
                let stage = Stage {
                    key: "k".into(),
                    mods: vec![Value::String("mod".into())],
                    command: true,
                    control: false,
                    shift: false,
                    alt: false,
                };
                (
                    catalog::stage_label(&stage, is_mac),
                    started + CHORD_TIMEOUT,
                )
            });
        let waiting = pending.or(deferral);
        let flash = window.no_match_until.filter(|deadline| now < *deadline);
        let deadline = waiting
            .as_ref()
            .map(|(_, deadline)| *deadline)
            .into_iter()
            .chain(flash)
            .min();
        if let Some(deadline) = deadline {
            context.request_repaint_after(deadline.saturating_duration_since(now));
        }
        ChordStatus {
            shortcut: waiting.map(|(label, _)| label),
            no_match: flash.is_some(),
        }
    }

    pub fn clear(&mut self) {
        self.windows.clear();
    }

    pub fn clear_chord(&mut self, viewport: egui::ViewportId) {
        if let Some(window) = self.windows.get_mut(&viewport) {
            window.map.pending = None;
            window.map.editor_pending = None;
            window.map.editor_deferral = None;
            window.decisions.clear();
            window.editor_decisions.clear();
            window.claimed_text = None;
        }
    }
}

pub fn event_index(context: &egui::Context, event: &egui::Event, next: &mut usize) -> usize {
    context.input(|input| raw_event_index(&input.raw.events, event, next))
}

pub fn raw_event_index(raw: &[egui::Event], event: &egui::Event, next: &mut usize) -> usize {
    egui::Context::raw_event_index(raw, event, next)
}

impl Keymap {
    pub fn new() -> AppResult<Self> {
        let parsed: Value = serde_json::from_str(DEFAULTS)
            .map_err(|_| AppError::Internal("native keymap catalog is invalid".into()))?;
        let base = parsed
            .as_array()
            .ok_or_else(|| AppError::Internal("native keymap catalog is not an array".into()))?
            .iter()
            .map(|value| {
                value
                    .get("id")
                    .and_then(Value::as_str)
                    .zip(Stage::parse(value))
                    .map(|(id, first)| Entry {
                        id: id.to_owned(),
                        first,
                        second: value.get("chord").and_then(Stage::parse),
                        when: value.get("when").and_then(Value::as_str).map(str::to_owned),
                    })
                    .ok_or_else(|| AppError::Internal("native keymap entry is invalid".into()))
            })
            .collect::<AppResult<Vec<_>>>()?;
        let editor_parsed: Value = serde_json::from_str(EDITOR_DEFAULTS)
            .map_err(|_| AppError::Internal("native editor keymap is invalid".into()))?;
        let editor_base = editor_parsed
            .as_array()
            .ok_or_else(|| AppError::Internal("native editor keymap is not an array".into()))?
            .iter()
            .map(|value| {
                Ok(EditorBinding {
                    entry: Entry {
                        id: value
                            .get("id")
                            .and_then(Value::as_str)
                            .ok_or_else(|| {
                                AppError::Internal("native editor keymap id is invalid".into())
                            })?
                            .into(),
                        first: Stage::parse(value).ok_or_else(|| {
                            AppError::Internal("native editor keymap stage is invalid".into())
                        })?,
                        second: value.get("chord").and_then(Stage::parse),
                        when: Some("editorTextFocus".into()),
                    },
                    platform: value
                        .get("platform")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(Self {
            entries: base.clone(),
            base,
            overrides: None,
            pending: None,
            editor_deferral: None,
            editor_prefixes: Vec::new(),
            editor_pending: None,
            commands: command_registry::registry()?,
            command_bindings: Vec::new(),
            editor_bindings: editor_base.clone(),
            editor_base,
        })
    }

    pub fn update(&mut self, json: Option<&str>) {
        if self.overrides.as_deref() == json {
            return;
        }
        self.overrides = json.map(str::to_owned);
        self.entries = self.base.clone();
        self.editor_prefixes.clear();
        self.command_bindings.clear();
        self.editor_bindings = self.editor_base.clone();
        let Some(Value::Array(overrides)) = json.and_then(|json| serde_json::from_str(json).ok())
        else {
            return;
        };
        for entry in &mut self.entries {
            let Some(value) = overrides.iter().find(|value| {
                let id = value.get("actionId").and_then(Value::as_str);
                let id = match id {
                    Some("keybindings.open") => Some("open-keybindings-editor"),
                    id => id,
                };
                id == Some(entry.id.as_str()) && Stage::parse(value).is_some()
            }) else {
                continue;
            };
            entry.apply_override(value);
        }
        for value in &overrides {
            if let Some(id) = value
                .get("actionId")
                .and_then(Value::as_str)
                .filter(|id| id.starts_with("monaco."))
                && let Some(first) = Stage::parse(value)
            {
                self.editor_bindings
                    .retain(|binding| binding.entry.id != id);
                self.editor_bindings.push(EditorBinding {
                    entry: Entry {
                        id: id.into(),
                        first,
                        second: value.get("chord").and_then(Stage::parse),
                        when: Some("editorTextFocus".into()),
                    },
                    platform: None,
                });
            }
            if value
                .get("actionId")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with("monaco."))
                && value.get("chord").and_then(Stage::parse).is_some()
                && let Some(stage) = Stage::parse(value)
            {
                self.editor_prefixes.push(stage);
            }
        }
        self.command_bindings = self
            .commands
            .commands()
            .iter()
            .filter(|command| command.runs_via_command())
            .filter_map(|command| {
                overrides.iter().find_map(|value| {
                    if value.get("actionId").and_then(Value::as_str) != Some(command.id.as_str()) {
                        return None;
                    }
                    Some(CommandBinding {
                        id: command.id.clone(),
                        first: Stage::parse(value)?,
                        second: value.get("chord").and_then(Stage::parse),
                        is_mac_only: command.is_mac_only,
                    })
                })
            })
            .collect();
    }

    pub fn decide(
        &mut self,
        event: &KeyEvent<'_>,
        context: Context,
        is_mac: bool,
        now: Instant,
    ) -> Decision {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| now.saturating_duration_since(pending.started) >= CHORD_TIMEOUT)
        {
            self.pending = None;
        }
        if self
            .editor_deferral
            .is_some_and(|started| now.saturating_duration_since(started) >= CHORD_TIMEOUT)
        {
            self.editor_deferral = None;
        }
        if event.composing && !event.modifiers.meta && !event.modifiers.control {
            return Decision::Ignore;
        }
        let ignorable = event.repeat || MODIFIER_ONLY.contains(&event.key);
        if self.editor_deferral.is_some() {
            if ignorable {
                return Decision::Ignore;
            }
            self.editor_deferral = None;
            return Decision::DeferToEditor;
        }
        if self.pending.is_some() && ignorable {
            return Decision::Ignore;
        }
        if let Some(pending) = self.pending.take() {
            return pending
                .candidates
                .into_iter()
                .find_map(|index| {
                    let entry = self.entries.get(index)?;
                    entry
                        .second
                        .as_ref()?
                        .matches(event, is_mac)
                        .then(|| Decision::ResolveChord(entry.id.clone()))
                })
                .unwrap_or(Decision::NoMatch);
        }
        let candidates = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                !context.editor
                    && entry.second.is_some()
                    && entry.enabled(context)
                    && entry.first.matches(event, is_mac)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if !candidates.is_empty() {
            self.pending = Some(Pending {
                candidates,
                started: now,
            });
            return Decision::EnterChord;
        }
        let editor_default = self
            .base
            .iter()
            .find(|entry| entry.id == "open-keybindings-editor");
        if context.editor
            && (editor_default.is_some_and(|entry| entry.first.matches(event, is_mac))
                || self
                    .editor_prefixes
                    .iter()
                    .any(|stage| stage.matches(event, is_mac)))
        {
            self.editor_deferral = Some(now);
            return Decision::ObserveEditorPrefix;
        }
        self.entries
            .iter()
            .find(|entry| {
                entry.second.is_none()
                    && entry.enabled(context)
                    && entry.first.matches(event, is_mac)
            })
            .map(|entry| entry.id.as_str())
            .or_else(|| {
                self.command_bindings
                    .iter()
                    .find(|binding| binding.matches(event, is_mac))
                    .map(|binding| binding.id.as_str())
            })
            .map(|id| Decision::Dispatch(id.to_owned()))
            .unwrap_or(Decision::None)
    }

    pub fn decide_egui(
        &mut self,
        event: &egui::Event,
        context: Context,
        composing: bool,
        now: Instant,
    ) -> Decision {
        self.decide_egui_for_platform(event, context, composing, cfg!(target_os = "macos"), now)
    }

    pub fn decide_egui_for_platform(
        &mut self,
        event: &egui::Event,
        context: Context,
        composing: bool,
        is_mac: bool,
        now: Instant,
    ) -> Decision {
        self.adapt_egui(event, composing, |map, event| {
            map.decide(event, context, is_mac, now)
        })
    }

    fn decide_editor_egui(
        &mut self,
        event: &egui::Event,
        composing: bool,
        os: egui::os::OperatingSystem,
        now: Instant,
    ) -> Decision {
        self.adapt_egui(event, composing, |map, event| {
            map.decide_editor_for_platform(
                event,
                os.is_mac(),
                os == egui::os::OperatingSystem::Nix,
                now,
            )
        })
    }

    fn adapt_egui(
        &mut self,
        event: &egui::Event,
        composing: bool,
        action: impl FnOnce(&mut Self, &KeyEvent<'_>) -> Decision,
    ) -> Decision {
        let egui::Event::Key {
            key,
            physical_key,
            pressed: true,
            repeat,
            modifiers,
        } = event
        else {
            return Decision::None;
        };
        let code = physical_key.map(dom_code);
        action(
            self,
            &KeyEvent {
                key: dom_key(*key),
                code: code.as_deref(),
                modifiers: Modifiers {
                    meta: modifiers.mac_cmd,
                    control: modifiers.ctrl,
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                },
                repeat: *repeat,
                composing,
            },
        )
    }

    pub fn decide_editor(&mut self, event: &KeyEvent<'_>, is_mac: bool, now: Instant) -> Decision {
        self.decide_editor_for_platform(event, is_mac, cfg!(target_os = "linux") && !is_mac, now)
    }

    fn decide_editor_for_platform(
        &mut self,
        event: &KeyEvent<'_>,
        is_mac: bool,
        is_linux: bool,
        now: Instant,
    ) -> Decision {
        if self
            .editor_pending
            .as_ref()
            .is_some_and(|pending| now.saturating_duration_since(pending.started) > CHORD_TIMEOUT)
        {
            self.editor_pending = None;
        }
        if MODIFIER_ONLY.contains(&event.key)
            || (event.composing && !event.modifiers.meta && !event.modifiers.control)
        {
            return Decision::Ignore;
        }
        let supported_group = |entry: &Entry| {
            EDITOR_GROUPS.contains(&entry.id.as_str())
                && entry
                    .second
                    .as_ref()
                    .is_some_and(|second| editor_key(&entry.first.key) && editor_key(&second.key))
        };
        let platform = if is_mac {
            "mac"
        } else if is_linux {
            "linux"
        } else {
            "win"
        };
        let editor_entries = self
            .editor_bindings
            .iter()
            .filter(|binding| {
                binding
                    .platform
                    .as_deref()
                    .is_none_or(|target| target == platform)
            })
            .filter(|binding| {
                self.commands
                    .command(&binding.entry.id)
                    .is_some_and(|command| {
                        command.is_registered(is_mac)
                            && matches!(command.execution, command_registry::Execution::Native(_))
                    })
            })
            .map(|binding| &binding.entry);
        let entries = editor_entries
            .chain(self.entries.iter().filter(|entry| supported_group(entry)))
            .collect::<Vec<_>>();
        let matches = |stage: &Stage| {
            let mut event = KeyEvent {
                key: event.key,
                code: event.code,
                modifiers: event.modifiers,
                repeat: event.repeat,
                composing: event.composing,
            };
            if !is_mac {
                std::mem::swap(&mut event.modifiers.meta, &mut event.modifiers.control);
            }
            stage.matches(&event, true)
        };
        if let Some(pending) = self.editor_pending.take() {
            return entries
                .iter()
                .rev()
                .copied()
                .find(|entry| {
                    canonical(&entry.first.key) == canonical(&pending.prefix.key)
                        && entry.first.command == pending.prefix.command
                        && entry.first.control == pending.prefix.control
                        && entry.first.shift == pending.prefix.shift
                        && entry.first.alt == pending.prefix.alt
                        && entry.second.as_ref().is_some_and(&matches)
                })
                .map(|entry| Decision::ResolveChord(entry.id.clone()))
                .unwrap_or(Decision::NoMatch);
        }
        if let Some(entry) = entries.iter().rev().find(|entry| matches(&entry.first)) {
            if entry.second.is_none() {
                return Decision::Dispatch(entry.id.clone());
            }
            self.editor_pending = Some(EditorPending {
                prefix: entry.first.clone(),
                started: now,
            });
            return Decision::EnterChord;
        }
        Decision::None
    }
}

fn editor_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    if key.len() == 1 {
        return key.as_bytes()[0].is_ascii_alphanumeric() || " ;=,-./`[\\]'".contains(&key);
    }
    matches!(
        key.as_str(),
        "backspace"
            | "tab"
            | "enter"
            | "escape"
            | "space"
            | "pageup"
            | "pagedown"
            | "end"
            | "home"
            | "arrowleft"
            | "arrowup"
            | "arrowright"
            | "arrowdown"
            | "delete"
            | "f1"
            | "f2"
            | "f3"
            | "f4"
            | "f5"
            | "f6"
            | "f7"
            | "f8"
            | "f9"
            | "f10"
            | "f11"
            | "f12"
    )
}

fn dom_key(key: egui::Key) -> &'static str {
    use egui::Key;
    match key {
        Key::ArrowUp => "ArrowUp",
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::Space => " ",
        Key::Minus => "-",
        Key::Quote => "'",
        Key::ShiftLeft | Key::ShiftRight => "Shift",
        Key::ControlLeft | Key::ControlRight => "Control",
        Key::AltLeft | Key::AltRight => "Alt",
        Key::SuperLeft | Key::SuperRight => "Meta",
        _ => key.symbol_or_name(),
    }
}

fn dom_code(key: egui::Key) -> String {
    use egui::Key;
    let name = key.name();
    if name.len() == 1 && name.as_bytes()[0].is_ascii_uppercase() {
        return format!("Key{name}");
    }
    if name.len() == 1 && name.as_bytes()[0].is_ascii_digit() {
        return format!("Digit{name}");
    }
    match key {
        Key::Space => "Space",
        Key::Minus => "Minus",
        Key::Equals => "Equal",
        Key::OpenBracket => "BracketLeft",
        Key::CloseBracket => "BracketRight",
        Key::Semicolon => "Semicolon",
        Key::Quote => "Quote",
        Key::Backtick => "Backquote",
        Key::Comma => "Comma",
        Key::Period => "Period",
        Key::Slash => "Slash",
        Key::Backslash | Key::IntlBackslash => "Backslash",
        _ => dom_key(key),
    }
    .into()
}

fn canonical(key: &str) -> String {
    if key == " " {
        return "space".into();
    }
    key.to_lowercase()
}

fn event_key(event: &KeyEvent<'_>) -> String {
    canonical(&capture_key(event))
}

fn capture_key(event: &KeyEvent<'_>) -> String {
    let bytes = event.key.as_bytes();
    if bytes.len() == 1
        && (bytes[0].is_ascii_alphanumeric() || b"-=[];'`,./\\ ".contains(&bytes[0]))
    {
        return canonical(event.key);
    }
    let code = event.code.unwrap_or_default();
    if let Some(letter) = code.strip_prefix("Key")
        && letter.len() == 1
        && letter.as_bytes()[0].is_ascii_uppercase()
    {
        return letter.to_ascii_lowercase();
    }
    if let Some(digit) = code.strip_prefix("Digit")
        && digit.len() == 1
        && digit.as_bytes()[0].is_ascii_digit()
    {
        return digit.into();
    }
    match code {
        "Space" => "space".into(),
        "Minus" => "-".into(),
        "Equal" => "=".into(),
        "BracketLeft" => "[".into(),
        "BracketRight" => "]".into(),
        "Semicolon" => ";".into(),
        "Quote" => "'".into(),
        "Backquote" => "`".into(),
        "Comma" => ",".into(),
        "Period" => ".".into(),
        "Slash" => "/".into(),
        "Backslash" => "\\".into(),
        _ if event.key.encode_utf16().count() == 1 => event.key.to_lowercase(),
        _ => event.key.to_owned(),
    }
}

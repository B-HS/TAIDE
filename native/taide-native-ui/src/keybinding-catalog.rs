use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};
use taide_model::error::{AppError, AppResult};

use super::{
    Context, DEFAULTS, Entry, KeyEvent, Keymap, Modifiers, Stage, canonical, command_registry,
};

#[path = "keybinding-capture.rs"]
pub mod capture;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    first: Stage,
    second: Option<Stage>,
}

impl Binding {
    pub fn parse(value: &Value) -> Option<Self> {
        Some(Self {
            first: Stage::parse(value)?,
            second: value.get("chord").and_then(Stage::parse),
        })
    }

    pub fn key(&self) -> &str {
        &self.first.key
    }

    pub fn has_chord(&self) -> bool {
        self.second.is_some()
    }

    pub fn json(&self) -> Value {
        let mut value = json!({"key": self.first.key, "mods": self.first.mods});
        if let Some(second) = &self.second {
            value["chord"] = json!({"key": second.key, "mods": second.mods});
        }
        value
    }

    pub fn matches(&self, event: &KeyEvent<'_>, is_mac: bool) -> bool {
        self.first.matches(event, is_mac)
    }

    pub fn label(&self, is_mac: bool) -> String {
        let first = stage_label(&self.first, is_mac);
        match &self.second {
            Some(second) => format!("{first} {}", stage_label(second, is_mac)),
            None => first,
        }
    }

    pub(crate) fn event(&self, is_mac: bool) -> KeyEvent<'_> {
        KeyEvent {
            key: &self.first.key,
            code: None,
            modifiers: Modifiers {
                meta: self.first.command && is_mac,
                control: self.first.control || (self.first.command && !is_mac),
                shift: self.first.shift,
                alt: self.first.alt,
            },
            repeat: false,
            composing: false,
        }
    }
}

pub(super) fn stage_label(stage: &Stage, is_mac: bool) -> String {
    let order = if is_mac {
        ["ctrl", "alt", "shift", "mod"]
    } else {
        ["mod", "ctrl", "alt", "shift"]
    };
    let mut labels = Vec::new();
    for name in order {
        if !stage.mods.iter().any(|value| value.as_str() == Some(name)) {
            continue;
        }
        let label = match (is_mac, name) {
            (true, "mod") => "⌘",
            (true, "ctrl") => "⌃",
            (true, "alt") => "⌥",
            (true, "shift") => "⇧",
            (false, "mod" | "ctrl") => "Ctrl",
            (false, "alt") => "Alt",
            (false, "shift") => "Shift",
            _ => unreachable!(),
        };
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    let key = match stage.key.to_lowercase().as_str() {
        "arrowleft" => "←".into(),
        "arrowright" => "→".into(),
        "arrowup" => "↑".into(),
        "arrowdown" => "↓".into(),
        _ => stage.key.to_uppercase(),
    };
    let separator = if is_mac { "" } else { "+" };
    let modifiers = labels.join(separator);
    if modifiers.is_empty() {
        return key;
    }
    format!("{modifiers}{separator}{key}")
}

#[derive(Clone)]
pub struct Row {
    pub id: String,
    pub title_key: String,
    pub title_default_value: Option<String>,
    pub category_key: Option<String>,
    pub command_id: Option<String>,
    pub keymap_id: Option<String>,
    pub binding: Binding,
    pub when: Option<String>,
    pub is_overridden: bool,
    pub runs_via_command: bool,
    pub is_monaco: bool,
    pub default_binding_label: Option<String>,
    default_binding: Option<Binding>,
}

impl Row {
    pub fn effective_binding(&self) -> &Binding {
        if !self.binding.key().is_empty() || self.is_overridden {
            return &self.binding;
        }
        self.default_binding.as_ref().unwrap_or(&self.binding)
    }

    pub fn is_unassigned(&self) -> bool {
        self.binding.key().is_empty()
            && (self.is_overridden
                || self
                    .default_binding_label
                    .as_deref()
                    .is_none_or(str::is_empty))
    }

    pub fn enabled(&self, context: Context) -> bool {
        Entry {
            id: self.id.clone(),
            first: self.binding.first.clone(),
            second: self.binding.second.clone(),
            when: self.when.clone(),
        }
        .enabled(context)
    }

    pub fn with_binding(&self, binding: Binding) -> Self {
        let mut entry = Entry {
            id: self.id.clone(),
            first: self.binding.first.clone(),
            second: self.binding.second.clone(),
            when: self.when.clone(),
        };
        entry.apply_override(&binding.json());
        Self {
            binding,
            when: entry.when,
            is_overridden: true,
            ..self.clone()
        }
    }
}

#[derive(Clone, Default)]
pub struct Overrides {
    entries: Vec<Value>,
}

impl Overrides {
    pub fn parse(json: Option<&str>) -> Self {
        let Some(Value::Array(entries)) = json.and_then(|json| serde_json::from_str(json).ok())
        else {
            return Self::default();
        };
        Self {
            entries: entries
                .into_iter()
                .filter_map(|mut value| {
                    value.get("actionId")?.as_str()?;
                    Stage::parse(&value)?;
                    if value.get("actionId").and_then(Value::as_str) == Some("keybindings.open") {
                        value["actionId"] = Value::String("open-keybindings-editor".into());
                    }
                    if value
                        .get("chord")
                        .is_some_and(|chord| Stage::parse(chord).is_none())
                    {
                        value.as_object_mut()?.remove("chord");
                    }
                    Some(value)
                })
                .collect(),
        }
    }

    pub fn json(&self) -> AppResult<String> {
        serde_json::to_string(&self.entries).map_err(|error| AppError::Internal(error.to_string()))
    }

    pub fn reset(&self, id: &str) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .filter(|entry| entry.get("actionId").and_then(Value::as_str) != Some(id))
                .cloned()
                .collect(),
        }
    }

    pub fn assign(&self, id: &str, binding: &Binding) -> Self {
        let mut result = self.reset(id);
        let mut value = binding.json();
        value["actionId"] = Value::String(id.to_owned());
        result.entries.push(value);
        result
    }

    pub fn unbind(&self, id: &str) -> Self {
        let mut result = self.reset(id);
        result
            .entries
            .push(json!({"actionId": id, "key": "", "mods": []}));
        result
    }
}

pub fn rows(overrides: &Overrides, is_mac: bool) -> AppResult<Vec<Row>> {
    let map = Keymap::new()?;
    let commands = command_registry::registry()?.commands();
    let defaults: Vec<Value> =
        serde_json::from_str(DEFAULTS).map_err(|error| AppError::Internal(error.to_string()))?;
    let mut represented = HashSet::new();
    let mut rows = Vec::with_capacity(commands.len() + defaults.len());
    for command in commands {
        if !command.is_registered(is_mac) {
            continue;
        }
        let entry =
            match command.keymap_id.as_deref() {
                Some(id) => Some(map.base.iter().find(|entry| entry.id == id).ok_or_else(
                    || AppError::Internal(format!("native command has unknown keymap: {id}")),
                )?),
                None => None,
            };
        if let Some(id) = &command.keymap_id {
            represented.insert(id.clone());
        }
        rows.push(Row {
            id: command
                .keymap_id
                .clone()
                .unwrap_or_else(|| command.id.clone()),
            title_key: command.title_key.clone(),
            title_default_value: command.title_default_value.clone(),
            category_key: command.category_key.clone(),
            command_id: Some(command.id.clone()),
            keymap_id: command.keymap_id.clone(),
            binding: binding(entry),
            when: entry.and_then(|entry| entry.when.clone()),
            is_overridden: false,
            runs_via_command: command.runs_via_command(),
            is_monaco: command.editor_action_id().is_some(),
            default_binding: command
                .default_binding_label
                .as_deref()
                .and_then(parse_default_label),
            default_binding_label: command.default_binding_label.clone(),
        });
    }
    for (entry, metadata) in map.base.iter().zip(defaults) {
        if represented.contains(&entry.id) {
            continue;
        }
        let category = match entry.id.as_str() {
            "command-palette" => Some("keymap.category.app"),
            "font-size-up" | "font-size-down" => Some("keymap.category.editor"),
            "terminal-jump-to-previous-command" | "terminal-jump-to-next-command" => {
                Some("keymap.category.terminal")
            }
            _ => None,
        };
        rows.push(Row {
            id: entry.id.clone(),
            title_key: required(&metadata, "descriptionKey")?,
            title_default_value: None,
            category_key: category.map(str::to_owned),
            command_id: None,
            keymap_id: Some(entry.id.clone()),
            binding: binding(Some(entry)),
            when: entry.when.clone(),
            is_overridden: false,
            runs_via_command: false,
            is_monaco: false,
            default_binding_label: None,
            default_binding: None,
        });
    }
    for row in &mut rows {
        if let Some(value) = overrides
            .entries
            .iter()
            .find(|value| value.get("actionId").and_then(Value::as_str) == Some(row.id.as_str()))
            && let Some(binding) = Binding::parse(value)
        {
            *row = row.with_binding(binding);
        }
    }
    Ok(rows)
}

fn binding(entry: Option<&Entry>) -> Binding {
    Binding {
        first: entry
            .map(|entry| entry.first.clone())
            .unwrap_or_else(|| Stage {
                key: String::new(),
                mods: Vec::new(),
                command: false,
                control: false,
                shift: false,
                alt: false,
            }),
        second: entry.and_then(|entry| entry.second.clone()),
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn required(value: &Value, key: &str) -> AppResult<String> {
    text(value, key)
        .ok_or_else(|| AppError::Internal(format!("native command metadata missing {key}")))
}

pub fn parse_default_label(label: &str) -> Option<Binding> {
    let mut stages = label.split(' ');
    let first = parse_label_stage(stages.next()?)?;
    let second = match stages.next() {
        Some(stage) => Some(parse_label_stage(stage)?),
        None => None,
    };
    if stages.next().is_some() {
        return None;
    }
    Some(Binding { first, second })
}

fn parse_label_stage(raw: &str) -> Option<Stage> {
    let mut rest = raw;
    let mut mods = Vec::new();
    while let Some(glyph) = rest.chars().next() {
        let name = match glyph {
            '⌘' => "mod",
            '⌥' => "alt",
            '⇧' => "shift",
            '⌃' => "ctrl",
            _ => break,
        };
        mods.push(Value::String(name.into()));
        rest = &rest[glyph.len_utf8()..];
    }
    if rest.is_empty() {
        return None;
    }
    let key = match rest {
        "⌫" => "Backspace".into(),
        "⌦" => "Delete".into(),
        "⏎" => "Enter".into(),
        "⇥" => "Tab".into(),
        "Space" => "space".into(),
        key if key.encode_utf16().count() == 1 => key.to_lowercase(),
        key => key.to_owned(),
    };
    Stage::parse(&json!({"key": key, "mods": mods}))
}

pub struct ConflictIndex<'rows> {
    rows: &'rows [Row],
    buckets: HashMap<(String, [bool; 4]), Vec<usize>>,
    is_mac: bool,
}

impl<'rows> ConflictIndex<'rows> {
    pub fn new(rows: &'rows [Row], is_mac: bool) -> Self {
        let mut buckets = HashMap::<_, Vec<_>>::new();
        for (index, row) in rows.iter().enumerate() {
            let binding = row.effective_binding();
            if !binding.key().is_empty() {
                buckets
                    .entry(signature(binding, is_mac))
                    .or_default()
                    .push(index);
            }
        }
        Self {
            rows,
            buckets,
            is_mac,
        }
    }

    pub fn find(&self, candidate: &Row) -> Option<&'rows Row> {
        let binding = candidate.effective_binding();
        if binding.key().is_empty() {
            return None;
        }
        let bucket = self.buckets.get(&signature(binding, self.is_mac))?;
        bucket
            .iter()
            .filter_map(|index| self.rows.get(*index))
            .find(|row| {
                if row.id == candidate.id
                    || (row.when.is_some()
                        && candidate.when.is_some()
                        && row.when != candidate.when)
                {
                    return false;
                }
                let other = row.effective_binding();
                if let (Some(left), Some(right)) = (&other.second, &binding.second)
                    && (left.key.to_lowercase() != right.key.to_lowercase()
                        || left.mods.len() != right.mods.len()
                        || !left
                            .mods
                            .iter()
                            .all(|modifier| right.mods.contains(modifier)))
                {
                    return false;
                }
                other.matches(&binding.event(self.is_mac), self.is_mac)
            })
    }
}

fn signature(binding: &Binding, is_mac: bool) -> (String, [bool; 4]) {
    let modifiers = binding.event(is_mac).modifiers;
    (
        canonical(binding.key()),
        [
            modifiers.meta,
            modifiers.control,
            modifiers.shift,
            modifiers.alt,
        ],
    )
}

pub fn filter_by_key<'rows>(
    rows: &'rows [Row],
    event: &KeyEvent<'_>,
    is_mac: bool,
) -> Vec<&'rows Row> {
    rows.iter()
        .filter(|row| !row.binding.key().is_empty() && row.binding.matches(event, is_mac))
        .collect()
}

pub fn runnable_command<'rows>(
    rows: &'rows [Row],
    event: &KeyEvent<'_>,
    is_mac: bool,
) -> Option<&'rows Row> {
    rows.iter().find(|row| {
        row.runs_via_command
            && row.command_id.is_some()
            && !row.binding.has_chord()
            && row.binding.matches(event, is_mac)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../taide-native-app/tests/fixtures/keybinding-catalog.json");

    fn row_json(row: &Row) -> Value {
        let mut value = row.binding.json();
        let object = value.as_object_mut().unwrap();
        object.extend([
            ("id".into(), json!(row.id)),
            ("titleKey".into(), json!(row.title_key)),
            ("titleDefaultValue".into(), json!(row.title_default_value)),
            ("categoryKey".into(), json!(row.category_key)),
            ("commandId".into(), json!(row.command_id)),
            ("keymapId".into(), json!(row.keymap_id)),
            ("isOverridden".into(), json!(row.is_overridden)),
            ("runsViaCommand".into(), json!(row.runs_via_command)),
            (
                "source".into(),
                json!(if row.is_monaco { "monaco" } else { "app" }),
            ),
            (
                "defaultBindingLabel".into(),
                json!(row.default_binding_label),
            ),
        ]);
        if let Some(when) = &row.when {
            object.insert("when".into(), json!(when));
        }
        value
    }

    #[test]
    fn keybinding_catalog은_전체원본행_등록순서_override_충돌과_필터를_보존한다() {
        let fixtures: Vec<Value> = serde_json::from_str(FIXTURE).unwrap();
        for fixture in fixtures {
            let name = fixture["name"].as_str().unwrap();
            let is_mac = fixture["mac"].as_bool().unwrap();
            let raw = fixture["json"].as_str();
            let overrides = Overrides::parse(raw);
            let rows = rows(&overrides, is_mac).unwrap();
            assert_eq!(
                json!(rows.iter().map(row_json).collect::<Vec<_>>()),
                fixture["rows"],
                "{name}: rows"
            );
            assert_eq!(
                serde_json::from_str::<Value>(&overrides.json().unwrap()).unwrap(),
                fixture["overrides"],
                "{name}: overrides"
            );
            let index = ConflictIndex::new(&rows, is_mac);
            assert_eq!(
                json!(
                    rows.iter()
                        .map(|row| index.find(row).map(|other| other.id.as_str()))
                        .collect::<Vec<_>>()
                ),
                fixture["conflicts"],
                "{name}: conflicts"
            );
            assert_eq!(
                json!(
                    rows.iter()
                        .filter(|row| row.is_unassigned())
                        .map(|row| row.id.as_str())
                        .collect::<Vec<_>>()
                ),
                fixture["unassigned"],
                "{name}: unassigned"
            );
            for filter in fixture["filters"].as_array().unwrap() {
                let binding = Binding::parse(filter).unwrap();
                assert_eq!(
                    json!(
                        filter_by_key(&rows, &binding.event(is_mac), is_mac)
                            .iter()
                            .map(|row| row.id.as_str())
                            .collect::<Vec<_>>()
                    ),
                    filter["ids"],
                    "{name}: filter"
                );
            }
            let mut map = Keymap::new().unwrap();
            map.update(raw);
            for entry in &map.entries {
                let row = rows.iter().find(|row| row.id == entry.id).unwrap();
                assert_eq!(
                    row.binding,
                    binding(Some(entry)),
                    "{name}: shared parser {}",
                    entry.id
                );
                assert_eq!(row.when, entry.when, "{name}: shared scope {}", entry.id);
            }
        }

        let original = Overrides::parse(Some(
            r#"[{"actionId":"save","key":"r","mods":["mod"],"future":{"keep":true}},{"actionId":"unknown.future","key":"z","mods":[],"future":42},{"actionId":"save","key":"t","mods":["mod"]}]"#,
        ));
        let replacement = Binding::parse(
            &json!({"key": "x", "mods": ["mod"], "chord": {"key": "y", "mods": []}}),
        )
        .unwrap();
        let assigned = original.assign("save", &replacement);
        assert_eq!(
            assigned.entries,
            vec![
                json!({"actionId":"unknown.future","key":"z","mods":[],"future":42}),
                json!({"actionId":"save","key":"x","mods":["mod"],"chord":{"key":"y","mods":[]}})
            ]
        );
        let reset = assigned.reset("save");
        assert_eq!(reset.entries, vec![original.entries[1].clone()]);
        let unbound = assigned.unbind("save");
        let unbound_rows = rows(&unbound, true).unwrap();
        let save = unbound_rows.iter().find(|row| row.id == "save").unwrap();
        assert!(save.is_overridden && save.is_unassigned());
        assert!(!save.binding.has_chord());
        assert_eq!(Overrides::default().json().unwrap(), "[]");
        assert!(
            Overrides::parse(Some(
                "[null,42,{}, {\"actionId\":7,\"key\":\"x\",\"mods\":[]}]"
            ))
            .entries
            .is_empty()
        );
        assert!(Overrides::parse(Some("broken json")).entries.is_empty());

        let command_overrides = Overrides::default().assign("sync.uploadNow", &replacement);
        let command_rows = rows(&command_overrides, true).unwrap();
        assert!(runnable_command(&command_rows, &replacement.event(true), true).is_none());
        let single = Binding::parse(&json!({"key":"u","mods":["mod"]})).unwrap();
        let command_rows =
            rows(&command_overrides.assign("sync.uploadNow", &single), true).unwrap();
        assert_eq!(
            runnable_command(&command_rows, &single.event(true), true)
                .unwrap()
                .command_id
                .as_deref(),
            Some("sync.uploadNow")
        );
        for label in ["", "⌘", "⌘K ", "⌘K ⌘S X", " ⌘S"] {
            assert!(parse_default_label(label).is_none(), "{label}");
        }
    }

    #[test]
    fn keymap의_명령행_dispatch는_catalog의_runnable_command와_같은_행을_앱_keymap_뒤에_고른다() {
        use super::super::{Decision, Instant};

        let fixtures: Vec<Value> = serde_json::from_str(FIXTURE).unwrap();
        let now = Instant::now();
        let mut command_dispatches = Vec::new();
        for fixture in fixtures {
            let name = fixture["name"].as_str().unwrap();
            let is_mac = fixture["mac"].as_bool().unwrap();
            let raw = fixture["json"].as_str();
            let rows = rows(&Overrides::parse(raw), is_mac).unwrap();
            let mut map = Keymap::new().unwrap();
            map.update(raw);
            for row in rows
                .iter()
                .filter(|row| row.runs_via_command && !row.binding.key().is_empty())
            {
                let event = row.binding.event(is_mac);
                let expected = runnable_command(&rows, &event, is_mac)
                    .and_then(|row| row.command_id.as_deref());
                let decision = map.decide(&event, Context::default(), is_mac, now);
                map.pending = None;
                match &decision {
                    Decision::Dispatch(id) if map.base.iter().any(|entry| &entry.id == id) => (),
                    Decision::EnterChord => (),
                    Decision::Dispatch(id) => {
                        assert_eq!(Some(id.as_str()), expected, "{name}: {}", row.id);
                        command_dispatches.push(format!("{name}:{id}"));
                    }
                    Decision::None => assert_eq!(None, expected, "{name}: {}", row.id),
                    other => panic!("{name}: {} decided {other:?}", row.id),
                }
            }
        }
        assert_eq!(
            command_dispatches,
            [
                "non-mac-control-space-and-chord-gate-removal:sync.uploadNow",
                "non-mac-control-space-and-chord-gate-removal:sync.uploadNow"
            ]
        );
    }
}

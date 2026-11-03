use std::collections::BTreeSet;

use serde_json::json;

use super::catalog::{Binding, Overrides, Row, rows};
use super::*;

const MONACO_KEYBINDING_TS: &str =
    include_str!("../../../src/shared/lib/monaco/monaco-keybinding.ts");
const MONACO_KEY_CODE_TABLE: &str = "const MONACO_KEY_CODE";
const TABLE_END: &str = "\n}";
const PRINTABLE_ASCII: std::ops::Range<u8> = 0x20..0x7f;
const PLATFORMS: [bool; 2] = [true, false];
const SCOPES: [Context; 3] = [
    Context {
        terminal: false,
        editor: false,
    },
    Context {
        terminal: true,
        editor: false,
    },
    Context {
        terminal: false,
        editor: true,
    },
];

fn monaco_key_codes() -> BTreeSet<String> {
    let table = MONACO_KEYBINDING_TS
        .split_once(MONACO_KEY_CODE_TABLE)
        .and_then(|(_, rest)| rest.split_once('{'))
        .and_then(|(_, rest)| rest.split_once(TABLE_END))
        .map(|(body, _)| body)
        .expect("monaco-keybinding.ts keeps the MONACO_KEY_CODE table");
    table
        .lines()
        .filter_map(|line| {
            let (key, _) = line.trim().rsplit_once(':')?;
            let key = key.trim();
            let unquoted = ['\'', '"']
                .into_iter()
                .find_map(|quote| key.strip_prefix(quote)?.strip_suffix(quote))
                .unwrap_or(key);
            Some(unquoted.replace("\\\\", "\\"))
        })
        .collect()
}

#[test]
fn monaco_바인딩_가능_키_집합은_원본_monaco_key_code_표와_같다() {
    let original = monaco_key_codes();
    assert!(original.contains("f12") && original.contains("\\") && original.contains(" "));
    for key in &original {
        assert!(editor_key(key), "original binds {key:?}");
    }
    let vocabulary = PRINTABLE_ASCII
        .map(|byte| char::from(byte).to_string())
        .chain(egui::Key::ALL.iter().map(|key| canonical(dom_key(*key))))
        .chain(original.iter().cloned())
        .collect::<BTreeSet<_>>();
    for key in &vocabulary {
        assert_eq!(
            editor_key(key),
            original.contains(&key.to_lowercase()),
            "{key:?}"
        );
    }
}

fn scope(row: &Row) -> Context {
    Context {
        terminal: row.when.as_deref() == Some("terminalFocus"),
        editor: row.when.as_deref() == Some("editorTextFocus"),
    }
}

fn decide(map: &mut Keymap, row: &Row, binding: &Binding, is_mac: bool) -> Decision {
    let event = binding.event(is_mac);
    if row.id.starts_with("monaco.") {
        map.decide_editor(&event, is_mac, Instant::now())
    } else {
        map.decide(&event, scope(row), is_mac, Instant::now())
    }
}

#[test]
fn 모든_catalog_행의_override는_새_키로_같은_id를_내고_기본_키는_그_id를_더이상_내지_않는다() {
    let replacement =
        Binding::parse(&json!({"key": "F8", "mods": ["mod", "alt", "shift"]})).unwrap();
    for is_mac in PLATFORMS {
        let mut defaults = Keymap::new().unwrap();
        for context in SCOPES {
            assert_eq!(
                defaults.decide(&replacement.event(is_mac), context, is_mac, Instant::now()),
                Decision::None,
                "the replacement key is free by default"
            );
        }
        assert_eq!(
            defaults.decide_editor(&replacement.event(is_mac), is_mac, Instant::now()),
            Decision::None
        );
        let mut routed = 0;
        let mut unrouted = Vec::new();
        for row in rows(&Overrides::default(), is_mac).unwrap() {
            let overrides = Overrides::default().assign(&row.id, &replacement);
            let overridden = rows(&overrides, is_mac)
                .unwrap()
                .into_iter()
                .find(|candidate| candidate.id == row.id)
                .unwrap();
            let mut map = Keymap::new().unwrap();
            map.update(Some(&overrides.json().unwrap()));
            let decision = decide(&mut map, &overridden, &replacement, is_mac);
            if decision != Decision::Dispatch(row.id.clone()) {
                assert!(
                    row.keymap_id.is_none(),
                    "keymap row {} must follow its override: {decision:?}",
                    row.id
                );
                assert!(
                    row.id.starts_with("monaco.") || !row.runs_via_command,
                    "command row {} must follow its override: {decision:?}",
                    row.id
                );
                unrouted.push(row.id.clone());
                continue;
            }
            routed += 1;
            let old = row.effective_binding().clone();
            if old.key().is_empty() {
                continue;
            }
            let first = decide(&mut map, &row, &old, is_mac);
            assert_ne!(first, Decision::Dispatch(row.id.clone()), "{}", row.id);
            if let Some(second) = old.json().get("chord").and_then(Binding::parse)
                && matches!(first, Decision::EnterChord)
            {
                assert_ne!(
                    decide(&mut map, &row, &second, is_mac),
                    Decision::ResolveChord(row.id.clone()),
                    "{}",
                    row.id
                );
            }
        }
        assert!(routed > 0);
        println!(
            "settings-67 platform={} routed={routed} unrouted={}: {unrouted:?}",
            if is_mac { "mac" } else { "other" },
            unrouted.len()
        );
    }
}

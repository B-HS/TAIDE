use std::collections::HashSet;

use serde::{Serialize, ser::SerializeMap};
use taide_model::{
    error::{AppError, AppResult},
    snippet::{SnippetEntry, SnippetMap, SnippetStringOrList},
};

const JSON_INDENT: &[u8] = b"    ";
const GLOBAL_EXTENSION: &str = ".code-snippets";
const LANGUAGE_EXTENSION: &str = ".json";
const ARRAY_INDEX_LIMIT: u32 = u32::MAX;

pub const LANGUAGE_IDS: [&str; 31] = [
    "rust",
    "typescript",
    "typescriptreact",
    "javascript",
    "javascriptreact",
    "json",
    "jsonc",
    "markdown",
    "toml",
    "yaml",
    "html",
    "css",
    "scss",
    "python",
    "go",
    "shellscript",
    "java",
    "ruby",
    "erb",
    "dart",
    "swift",
    "scala",
    "elixir",
    "heex",
    "haskell",
    "c",
    "cpp",
    "kotlin",
    "lua",
    "zig",
    "hcl",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub body: String,
    pub description: String,
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Validation {
    Incomplete(usize),
    DuplicateNames,
}

impl Draft {
    pub fn empty(id: String) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    pub fn is_valid(&self) -> bool {
        !trim(&self.name).is_empty()
            && !trim(&self.prefix).is_empty()
            && !trim(&self.body).is_empty()
    }

    pub fn is_blank(&self) -> bool {
        self.fields().iter().all(|field| trim(field).is_empty())
    }

    fn fields(&self) -> [&str; 5] {
        [
            &self.name,
            &self.prefix,
            &self.body,
            &self.description,
            &self.scope,
        ]
    }

    fn entry(&self) -> SnippetEntry {
        let prefixes = self
            .prefix
            .split(',')
            .map(trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let prefix = match prefixes.as_slice() {
            [] => SnippetStringOrList::Single(trim(&self.prefix).into()),
            [single] => SnippetStringOrList::Single((*single).into()),
            _ => SnippetStringOrList::Multiple(prefixes.into_iter().map(String::from).collect()),
        };
        let lines = self.body.split('\n').map(String::from).collect::<Vec<_>>();
        let body = if lines.len() > 1 {
            SnippetStringOrList::Multiple(lines)
        } else {
            SnippetStringOrList::Single(self.body.clone())
        };
        let description = trim(&self.description);
        let scope = trim(&self.scope);
        SnippetEntry {
            prefix,
            body,
            description: (!description.is_empty())
                .then(|| SnippetStringOrList::Single(description.into())),
            scope: (!scope.is_empty()).then(|| scope.into()),
        }
    }
}

pub fn trim(value: &str) -> &str {
    value.trim_matches(crate::keybinding_search::js_whitespace)
}

pub fn is_global_file(file_name: &str) -> bool {
    file_name.ends_with(GLOBAL_EXTENSION)
}

pub fn language_file_name(language_id: &str) -> String {
    format!("{language_id}{LANGUAGE_EXTENSION}")
}

pub fn global_file_name(raw_name: &str) -> String {
    let name = trim(raw_name);
    if is_global_file(name) {
        return name.into();
    }
    format!("{name}{GLOBAL_EXTENSION}")
}

pub fn is_safe_file_name(file_name: &str) -> bool {
    !file_name.contains(['/', '\\', ':']) && !file_name.contains("..")
}

fn array_index(key: &str) -> Option<u32> {
    let index = key.parse::<u32>().ok()?;
    (index < ARRAY_INDEX_LIMIT && index.to_string() == key).then_some(index)
}

pub(crate) fn object_order<T>(entries: &mut [(String, T)]) {
    entries.sort_by_key(|(name, _)| array_index(name).map_or((true, 0), |index| (false, index)));
}

pub(crate) fn lines(value: &SnippetStringOrList) -> String {
    match value {
        SnippetStringOrList::Single(value) => value.clone(),
        SnippetStringOrList::Multiple(values) => values.join("\n"),
    }
}

pub fn from_snippets(snippets: &SnippetMap, mut next_id: impl FnMut() -> String) -> Vec<Draft> {
    let mut entries = snippets
        .iter()
        .map(|(name, entry)| (name.clone(), entry))
        .collect::<Vec<_>>();
    object_order(&mut entries);
    entries
        .into_iter()
        .map(|(name, entry)| Draft {
            id: next_id(),
            name,
            prefix: match &entry.prefix {
                SnippetStringOrList::Single(value) => value.clone(),
                SnippetStringOrList::Multiple(values) => values.join(", "),
            },
            body: lines(&entry.body),
            description: entry.description.as_ref().map(lines).unwrap_or_default(),
            scope: entry.scope.clone().unwrap_or_default(),
        })
        .collect()
}

pub fn has_unsaved_changes(drafts: &[Draft], saved: &SnippetMap) -> bool {
    let saved = from_snippets(saved, String::new);
    drafts.len() != saved.len()
        || drafts
            .iter()
            .zip(&saved)
            .any(|(draft, saved)| draft.fields() != saved.fields())
}

pub fn validate(drafts: &[Draft]) -> Result<(), Validation> {
    let incomplete = drafts
        .iter()
        .filter(|draft| !draft.is_blank() && !draft.is_valid())
        .count();
    if incomplete > 0 {
        return Err(Validation::Incomplete(incomplete));
    }
    let mut names = HashSet::new();
    if drafts
        .iter()
        .filter(|draft| draft.is_valid())
        .any(|draft| !names.insert(trim(&draft.name)))
    {
        return Err(Validation::DuplicateNames);
    }
    Ok(())
}

struct Entries(Vec<(String, SnippetEntry)>);

impl Serialize for Entries {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, entry) in &self.0 {
            map.serialize_entry(name, entry)?;
        }
        map.end()
    }
}

pub fn content(drafts: &[Draft]) -> AppResult<String> {
    let mut entries: Vec<(String, SnippetEntry)> = Vec::new();
    for draft in drafts.iter().filter(|draft| draft.is_valid()) {
        let name = trim(&draft.name);
        if let Some((_, entry)) = entries.iter_mut().find(|(key, _)| key == name) {
            *entry = draft.entry();
        } else {
            entries.push((name.into(), draft.entry()));
        }
    }
    object_order(&mut entries);
    let mut bytes = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(JSON_INDENT);
    let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    Entries(entries)
        .serialize(&mut serializer)
        .map_err(|error| AppError::InvalidArgument(format!("invalid snippet content: {error}")))?;
    String::from_utf8(bytes)
        .map_err(|error| AppError::InvalidArgument(format!("invalid snippet UTF-8: {error}")))
}

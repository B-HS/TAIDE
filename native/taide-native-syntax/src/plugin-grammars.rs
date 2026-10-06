use std::sync::Arc;

use ferriki_textmate::RawGrammar;
use serde::Deserialize;
use serde_json::{Map, Value};

const SCOPE_NAME_KEY: &str = "scopeName";
const NAME_KEY: &str = "name";
const PATTERNS_KEY: &str = "patterns";
const REPOSITORY_KEY: &str = "repository";
const METADATA_KEYS_UNREAD_BY_TOKENIZATION: [&str; 2] = ["fileTypes", "firstLineMatch"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginGrammar {
    language_id: String,
    scope_name: String,
    embedded_languages: Vec<String>,
    registration: Arc<Value>,
}

impl PluginGrammar {
    pub fn from_contribution(
        language_id: &str,
        embedded_languages: &[String],
        grammar_json: &str,
    ) -> Option<Self> {
        let Ok(Value::Object(mut registration)) = serde_json::from_str(grammar_json) else {
            return None;
        };
        let scope_name = registration
            .get(SCOPE_NAME_KEY)?
            .as_str()
            .filter(|scope_name| !scope_name.is_empty())?
            .to_owned();
        let patterns = match registration.remove(PATTERNS_KEY) {
            Some(Value::Array(patterns)) => patterns,
            _ => Vec::new(),
        };
        let repository: Map<String, Value> = match registration.remove(REPOSITORY_KEY) {
            Some(Value::Object(repository)) => repository,
            Some(Value::Array(rules)) => rules
                .into_iter()
                .enumerate()
                .map(|(index, rule)| (index.to_string(), rule))
                .collect(),
            _ => Map::new(),
        };
        for key in METADATA_KEYS_UNREAD_BY_TOKENIZATION {
            registration.remove(key);
        }
        registration.insert(NAME_KEY.to_owned(), Value::String(language_id.to_owned()));
        registration.insert(PATTERNS_KEY.to_owned(), Value::Array(patterns));
        registration.insert(REPOSITORY_KEY.to_owned(), Value::Object(repository));
        Some(Self {
            language_id: language_id.to_owned(),
            scope_name,
            embedded_languages: embedded_languages.to_vec(),
            registration: Arc::new(Value::Object(registration)),
        })
    }

    pub fn language_id(&self) -> &str {
        &self.language_id
    }

    pub fn scope_name(&self) -> &str {
        &self.scope_name
    }

    pub fn embedded_languages(&self) -> &[String] {
        &self.embedded_languages
    }

    pub(crate) fn registration(&self) -> &Value {
        &self.registration
    }

    pub(crate) fn raw_grammar(&self) -> Option<RawGrammar> {
        RawGrammar::deserialize(self.registration()).ok()
    }
}

#[cfg(test)]
#[path = "plugin-grammars-tests.rs"]
mod tests;

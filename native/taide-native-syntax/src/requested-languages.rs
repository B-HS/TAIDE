use crate::bundled_grammars::bundled_language_ids;

pub const CORE_LANGUAGE_IDS: [&str; 3] = ["json", "jsonc", "markdown"];

pub fn is_bundled_language(language_id: &str) -> bool {
    bundled_language_ids().is_ok_and(|ids| ids.contains(&language_id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestedLanguages {
    ids: Vec<String>,
}

impl Default for RequestedLanguages {
    fn default() -> Self {
        Self {
            ids: CORE_LANGUAGE_IDS.map(str::to_owned).to_vec(),
        }
    }
}

impl RequestedLanguages {
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    pub fn contains(&self, language_id: &str) -> bool {
        self.ids.iter().any(|id| id == language_id)
    }

    pub fn request(&mut self, language_id: &str) -> bool {
        if !is_bundled_language(language_id) || self.contains(language_id) {
            return false;
        }
        self.ids.push(language_id.to_owned());
        true
    }
}

#[cfg(test)]
#[path = "requested-languages-tests.rs"]
mod tests;

use taide_model::{error::AppResult, snippet::SnippetFile};

use crate::snippet_draft::{self, Draft, LANGUAGE_IDS, Validation};

const GLOBAL_OPTION: &str = "global";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscardTarget {
    Select(String),
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Navigation {
    Unchanged,
    Selected,
    ConfirmDiscard,
    Closed,
}

pub struct NewFile {
    option: String,
    pub global_name: String,
    pub open: bool,
}

impl Default for NewFile {
    fn default() -> Self {
        Self {
            option: LANGUAGE_IDS[0].into(),
            global_name: String::new(),
            open: false,
        }
    }
}

impl NewFile {
    pub fn option(&self) -> &str {
        &self.option
    }

    pub fn select(&mut self, option: &str) -> bool {
        if option != GLOBAL_OPTION && !LANGUAGE_IDS.contains(&option) {
            return false;
        }
        self.option = option.into();
        true
    }

    pub fn is_global(&self) -> bool {
        self.option == GLOBAL_OPTION
    }

    pub fn file_name(&self) -> String {
        if self.is_global() {
            return snippet_draft::global_file_name(&self.global_name);
        }
        snippet_draft::language_file_name(&self.option)
    }

    pub fn can_create(&self, files: &[SnippetFile]) -> bool {
        let name = self.file_name();
        if self.is_global()
            && (snippet_draft::trim(&self.global_name).is_empty()
                || !snippet_draft::is_safe_file_name(&name))
        {
            return false;
        }
        !files.iter().any(|file| file.file_name == name)
    }
}

#[derive(Default)]
pub struct State {
    files: Vec<SnippetFile>,
    selected: Option<String>,
    drafts: Option<Vec<Draft>>,
    row_sequence: u128,
    pub new_file: NewFile,
    pub delete_file_open: bool,
    pub delete_entry: Option<String>,
    discard: Option<DiscardTarget>,
}

impl State {
    pub fn files(&self) -> &[SnippetFile] {
        &self.files
    }

    pub fn set_files(&mut self, files: Vec<SnippetFile>) {
        self.files = files;
        self.sync_drafts();
    }

    pub fn selected_file_name(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn selected_file(&self) -> Option<&SnippetFile> {
        self.files
            .iter()
            .find(|file| Some(&file.file_name) == self.selected.as_ref())
    }

    pub fn drafts(&self) -> Option<&[Draft]> {
        self.drafts.as_deref()
    }

    pub fn drafts_mut(&mut self) -> Option<&mut Vec<Draft>> {
        self.drafts.as_mut()
    }

    pub fn show_scope(&self) -> bool {
        self.selected
            .as_deref()
            .is_some_and(snippet_draft::is_global_file)
    }

    pub fn has_unsaved_changes(&self) -> bool {
        match (self.drafts(), self.selected_file()) {
            (Some(drafts), Some(file)) => {
                snippet_draft::has_unsaved_changes(drafts, &file.snippets)
            }
            _ => false,
        }
    }

    pub fn request_select(&mut self, file_name: String) -> Navigation {
        if self.selected.as_ref() == Some(&file_name) {
            return Navigation::Unchanged;
        }
        if self.has_unsaved_changes() {
            self.discard = Some(DiscardTarget::Select(file_name));
            return Navigation::ConfirmDiscard;
        }
        self.select(file_name);
        Navigation::Selected
    }

    fn select(&mut self, file_name: String) {
        self.selected = Some(file_name);
        self.drafts = None;
        self.sync_drafts();
    }

    fn sync_drafts(&mut self) {
        if self.drafts.is_some() {
            return;
        }
        let Some(file) = self.selected_file() else {
            return;
        };
        let mut sequence = self.row_sequence;
        let drafts = snippet_draft::from_snippets(&file.snippets, || {
            sequence += 1;
            sequence.to_string()
        });
        self.row_sequence = sequence;
        self.drafts = Some(drafts);
    }

    pub fn request_close(&mut self) -> Navigation {
        if self.has_unsaved_changes() {
            self.discard = Some(DiscardTarget::Close);
            return Navigation::ConfirmDiscard;
        }
        Navigation::Closed
    }

    pub fn pending_discard(&self) -> Option<&DiscardTarget> {
        self.discard.as_ref()
    }

    pub fn cancel_discard(&mut self) {
        self.discard = None;
    }

    pub fn confirm_discard(&mut self) -> Navigation {
        match self.discard.take() {
            Some(DiscardTarget::Close) => Navigation::Closed,
            Some(DiscardTarget::Select(file_name)) => {
                self.select(file_name);
                Navigation::Selected
            }
            None => Navigation::Unchanged,
        }
    }

    pub fn append_entry(&mut self) -> bool {
        let Some(drafts) = self.drafts.as_mut() else {
            return false;
        };
        self.row_sequence += 1;
        drafts.push(Draft::empty(self.row_sequence.to_string()));
        true
    }

    pub fn pending_delete_entry_name(&self) -> &str {
        self.drafts()
            .and_then(|drafts| {
                drafts
                    .iter()
                    .find(|draft| Some(&draft.id) == self.delete_entry.as_ref())
            })
            .map(|draft| draft.name.as_str())
            .unwrap_or_default()
    }

    pub fn confirm_delete_entry(&mut self) {
        let Some(id) = self.delete_entry.take() else {
            return;
        };
        if let Some(drafts) = self.drafts.as_mut() {
            drafts.retain(|draft| draft.id != id);
        }
    }

    pub fn validate_save(&self) -> Result<(), Validation> {
        snippet_draft::validate(self.drafts().unwrap_or_default())
    }

    pub fn save_content(&self) -> AppResult<Option<(String, String)>> {
        match (&self.selected, self.drafts()) {
            (Some(file_name), Some(drafts)) => {
                Ok(Some((file_name.clone(), snippet_draft::content(drafts)?)))
            }
            _ => Ok(None),
        }
    }

    pub fn deleted_file(&mut self) {
        self.delete_file_open = false;
        self.selected = None;
        self.drafts = None;
    }
}

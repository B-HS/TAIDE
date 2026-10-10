use std::path::Path;

use chrono::{DateTime, Datelike, FixedOffset, Local};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::language_configuration::LanguageRules;
use taide_native_editor::snippet_expansion::VariableContext;
use taide_native_editor::snippet_variables::{OvertypedText, SelectionVariables, clipboard_value};
use taide_native_editor::view::SelectionSet;
use uuid::Uuid;

const RANDOM_DECIMAL_MODULUS: u128 = 1_000_000;
const RANDOM_HEX_MODULUS: u128 = 0x100_0000;
const RANDOM_WIDTH: usize = 6;
const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const DAY_NAMES_SHORT: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const MONTH_NAMES_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub(crate) struct Clock {
    pub date: DateTime<FixedOffset>,
    pub timezone_name: Option<String>,
}

impl Clock {
    pub(crate) fn now() -> Self {
        Self {
            date: Local::now().fixed_offset(),
            timezone_name: iana_time_zone::get_timezone().ok(),
        }
    }

    fn resolve(&self, name: &str) -> Option<String> {
        let pattern = match name {
            "CURRENT_YEAR" => Some("%Y"),
            "CURRENT_YEAR_SHORT" => Some("%y"),
            "CURRENT_MONTH" => Some("%m"),
            "CURRENT_DATE" => Some("%d"),
            "CURRENT_HOUR" => Some("%H"),
            "CURRENT_MINUTE" => Some("%M"),
            "CURRENT_SECOND" => Some("%S"),
            "CURRENT_MILLISECOND" => Some("%3f"),
            "CURRENT_TIMEZONE_OFFSET" => Some("%:z"),
            _ => None,
        };
        if let Some(pattern) = pattern {
            return Some(self.date.format(pattern).to_string());
        }
        match name {
            "CURRENT_DAY_NAME" => {
                Some(DAY_NAMES[self.date.weekday().num_days_from_sunday() as usize].into())
            }
            "CURRENT_DAY_NAME_SHORT" => {
                Some(DAY_NAMES_SHORT[self.date.weekday().num_days_from_sunday() as usize].into())
            }
            "CURRENT_MONTH_NAME" => Some(MONTH_NAMES[self.date.month0() as usize].into()),
            "CURRENT_MONTH_NAME_SHORT" => {
                Some(MONTH_NAMES_SHORT[self.date.month0() as usize].into())
            }
            "CURRENT_SECONDS_UNIX" => Some(self.date.timestamp().to_string()),
            "CURRENT_MILLISECONDS_UNIX" => Some(self.date.timestamp_millis().to_string()),
            "CURRENT_TIMEZONE_NAME" => self.timezone_name.clone(),
            _ => None,
        }
    }
}

pub(super) struct Variables<'a> {
    pub document: &'a DocumentSnapshot,
    pub selection: &'a SelectionSet,
    pub model_path: &'a str,
    pub language: &'a dyn LanguageRules,
    pub clipboard: Option<&'a str>,
    pub clipboard_spread: bool,
    pub clock: &'a Clock,
    pub random: &'a mut dyn FnMut() -> Uuid,
    pub max_bytes: usize,
}

impl Variables<'_> {
    pub(super) fn resolve(
        &mut self,
        cursor: usize,
        context: VariableContext<'_>,
        overtyped: Option<OvertypedText<'_>>,
    ) -> Result<Option<String>, EditorError> {
        let name = context.name;
        let selection_variables =
            SelectionVariables::new(self.document, self.selection, cursor, self.max_bytes)?;
        let value = if let Some(value) = model_variable(self.model_path, name) {
            Some(value)
        } else if name == "CLIPBOARD" {
            clipboard_value(
                self.clipboard,
                cursor,
                self.selection.selections.len(),
                self.clipboard_spread,
                self.max_bytes,
            )?
        } else if let Some(value) =
            selection_variables.resolve(context, overtyped, |document, byte| {
                let range = line_content_range(document, document.rope.byte_to_line(byte));
                if range.len() > self.max_bytes {
                    return Err(EditorError::Capacity);
                }
                let line = document.rope.byte_slice(range.clone()).to_string();
                Ok(self
                    .language
                    .word_range(&line, byte - range.start)
                    .map(|word| line[word].to_owned()))
            })?
        {
            Some(value)
        } else if let Some(value) = comment_variable(self.language, name) {
            Some(value)
        } else if let Some(value) = self.clock.resolve(name) {
            Some(value)
        } else {
            match name {
                "WORKSPACE_NAME" => Some(String::new()),
                "WORKSPACE_FOLDER" => Some("/".to_owned()),
                "RANDOM" => Some(format!(
                    "{:0width$}",
                    (self.random)().as_u128() % RANDOM_DECIMAL_MODULUS,
                    width = RANDOM_WIDTH
                )),
                "RANDOM_HEX" => Some(format!(
                    "{:0width$x}",
                    (self.random)().as_u128() % RANDOM_HEX_MODULUS,
                    width = RANDOM_WIDTH
                )),
                "UUID" => Some((self.random)().to_string()),
                _ => None,
            }
        };
        if value
            .as_ref()
            .is_some_and(|value| value.len() > self.max_bytes)
        {
            return Err(EditorError::Capacity);
        }
        Ok(value)
    }
}

fn model_variable(model_path: &str, name: &str) -> Option<String> {
    let path = Path::new(model_path);
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let directory = path.parent().unwrap_or(path);
    match name {
        "TM_FILENAME" => Some(filename.into()),
        "TM_FILENAME_BASE" => Some(match filename.rfind('.').filter(|position| *position > 0) {
            Some(position) => filename[..position].into(),
            None => filename.into(),
        }),
        "TM_DIRECTORY" => Some(directory.to_string_lossy().into_owned()),
        "TM_DIRECTORY_BASE" => Some(
            directory
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .into(),
        ),
        "TM_FILEPATH" | "RELATIVE_FILEPATH" => Some(model_path.into()),
        _ => None,
    }
}

fn comment_variable(language: &dyn LanguageRules, name: &str) -> Option<String> {
    let comments = language.comments()?;
    match name {
        "LINE_COMMENT" => comments.line.clone(),
        "BLOCK_COMMENT_START" => comments.block.as_ref().map(|(start, _)| start.clone()),
        "BLOCK_COMMENT_END" => comments.block.as_ref().map(|(_, end)| end.clone()),
        _ => None,
    }
    .filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "editor-completion-variables-tests.rs"]
mod tests;

use std::collections::BTreeSet;
use std::ops::Range;
use std::sync::Arc;

use crate::diagnostics::{Marker, Message, Severity};
use crate::document::EditorError;
use crate::json_parser::{self, Error};

const COMMENT_ERROR_CODE: u32 = 521;
const COMMENT_ERROR_MESSAGE: &str = "Comments are not permitted in JSON.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
    Ignore,
}

impl Level {
    fn severity(self) -> Option<Severity> {
        match self {
            Self::Error => Some(Severity::Error),
            Self::Warning => Some(Severity::Warning),
            Self::Ignore => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub validate: bool,
    pub allow_comments: bool,
    pub comments: Option<Level>,
    pub trailing_commas: Option<Level>,
    pub schema_allows_comments: bool,
    pub schema_allows_trailing_commas: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub parser: json_parser::Limits,
    pub max_diagnostics: usize,
    pub max_message_bytes: usize,
}

struct Collector {
    markers: Vec<Marker>,
    seen: BTreeSet<(usize, &'static str)>,
    message_bytes: usize,
    limits: Limits,
}

impl Collector {
    fn add(
        &mut self,
        bytes: Range<usize>,
        text: &'static str,
        code: u32,
        severity: Severity,
        source: Option<&str>,
    ) -> Result<(), EditorError> {
        if !self.seen.insert((bytes.start, text)) {
            return Ok(());
        }
        let code = code.to_string();
        let size = text
            .len()
            .checked_add(code.len())
            .and_then(|count| count.checked_add(source.map_or(0, str::len)))
            .ok_or(EditorError::Capacity)?;
        self.message_bytes = self
            .message_bytes
            .checked_add(size)
            .ok_or(EditorError::Capacity)?;
        if self.markers.len() >= self.limits.max_diagnostics
            || self.message_bytes > self.limits.max_message_bytes
        {
            return Err(EditorError::Capacity);
        }
        self.markers.push(Marker {
            bytes,
            message: Arc::new(Message {
                severity,
                text: text.to_owned(),
                source: source.map(str::to_owned),
                code: Some(code),
            }),
        });
        Ok(())
    }
}

pub fn syntax_diagnostics(
    input: &str,
    language: &str,
    settings: Settings,
    limits: Limits,
) -> Result<Vec<Marker>, EditorError> {
    if !settings.validate {
        return Ok(Vec::new());
    }
    let document = json_parser::parse(input, true, limits.parser)?;
    let mut collector = Collector {
        markers: Vec::new(),
        seen: BTreeSet::new(),
        message_bytes: 0,
        limits,
    };
    let trailing = if settings.schema_allows_trailing_commas {
        None
    } else {
        settings.trailing_commas.unwrap_or(Level::Error).severity()
    };
    let comments = if settings.schema_allows_comments {
        None
    } else {
        settings.comments.map(Level::severity).unwrap_or_else(|| {
            if settings.allow_comments {
                None
            } else {
                Some(Severity::Error)
            }
        })
    };
    for diagnostic in document.diagnostics {
        let severity = if diagnostic.error == Error::TrailingComma {
            let Some(severity) = trailing else { continue };
            severity
        } else if diagnostic.is_warning {
            Severity::Warning
        } else {
            Severity::Error
        };
        collector.add(
            diagnostic.bytes,
            diagnostic.error.message(),
            diagnostic.error.code(),
            severity,
            Some(language),
        )?;
    }
    if let Some(severity) = comments {
        for comment in document.comments {
            collector.add(
                comment.bytes,
                COMMENT_ERROR_MESSAGE,
                COMMENT_ERROR_CODE,
                severity,
                None,
            )?;
        }
    }
    Ok(collector.markers)
}

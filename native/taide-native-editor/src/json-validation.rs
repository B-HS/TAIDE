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
    pub(crate) fn severity(self) -> Option<Severity> {
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

pub(crate) struct Collector {
    pub(crate) markers: Vec<Marker>,
    seen: BTreeSet<(usize, Vec<u16>)>,
    message_bytes: usize,
    max_diagnostics: usize,
    max_message_bytes: usize,
}

impl Collector {
    pub(crate) fn new(max_diagnostics: usize, max_message_bytes: usize) -> Self {
        Self {
            markers: Vec::new(),
            seen: BTreeSet::new(),
            message_bytes: 0,
            max_diagnostics,
            max_message_bytes,
        }
    }

    pub(crate) fn add(
        &mut self,
        bytes: Range<usize>,
        message: Vec<u16>,
        code: Option<u32>,
        severity: Severity,
        source: Option<&str>,
    ) -> Result<(), EditorError> {
        let key = (bytes.start, message);
        if self.seen.contains(&key) {
            return Ok(());
        }
        let text = String::from_utf16_lossy(&key.1);
        let code = code.map(|code| code.to_string());
        let size = text
            .len()
            .checked_add(code.as_ref().map_or(0, String::len))
            .and_then(|count| count.checked_add(source.map_or(0, str::len)))
            .ok_or(EditorError::Capacity)?;
        self.message_bytes = self
            .message_bytes
            .checked_add(size)
            .ok_or(EditorError::Capacity)?;
        if self.markers.len() >= self.max_diagnostics || self.message_bytes > self.max_message_bytes
        {
            return Err(EditorError::Capacity);
        }
        self.seen.insert(key);
        self.markers.push(Marker {
            bytes,
            message: Arc::new(Message {
                severity,
                text,
                source: source.map(str::to_owned),
                code,
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
    let mut collector = Collector::new(limits.max_diagnostics, limits.max_message_bytes);
    collect_syntax(&document, language, settings, &mut collector)?;
    Ok(collector.markers)
}

pub(crate) fn collect_syntax(
    document: &json_parser::Document,
    language: &str,
    settings: Settings,
    collector: &mut Collector,
) -> Result<(), EditorError> {
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
    for diagnostic in &document.diagnostics {
        let severity = if diagnostic.error == Error::TrailingComma {
            let Some(severity) = trailing else { continue };
            severity
        } else if diagnostic.is_warning {
            Severity::Warning
        } else {
            Severity::Error
        };
        collector.add(
            diagnostic.bytes.clone(),
            diagnostic.error.message().encode_utf16().collect(),
            Some(diagnostic.error.code()),
            severity,
            Some(language),
        )?;
    }
    if let Some(severity) = comments {
        for comment in &document.comments {
            collector.add(
                comment.bytes.clone(),
                COMMENT_ERROR_MESSAGE.encode_utf16().collect(),
                Some(COMMENT_ERROR_CODE),
                severity,
                None,
            )?;
        }
    }
    Ok(())
}

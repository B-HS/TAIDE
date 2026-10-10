use std::ops::Range;

pub use lsp_types::CompletionItemKind;
use lsp_types::{CompletionItem, CompletionResponse, CompletionTextEdit};

use crate::document::{DocumentId, DocumentSnapshot, EditorError, byte_to_char};
use crate::documentation::Content;
use crate::editing::line_content_range;
use crate::indent::IndentOptions;
use crate::lsp::{LspRange, Position};
use crate::snippet_expansion::{VariableContext, expand};
use crate::snippet_insertion::PreparedSnippet;
use crate::snippet_syntax::{
    FinalTabstopOptions, Marker, ParseLimits, RegexMetadata, Transform, parse_complete,
};
use crate::snippet_whitespace::{WhitespaceContext, adjust_whitespace};
use crate::view::{Selection, SelectionSet};

const SUPPORTED_KINDS: [CompletionItemKind; 25] = [
    CompletionItemKind::TEXT,
    CompletionItemKind::METHOD,
    CompletionItemKind::FUNCTION,
    CompletionItemKind::CONSTRUCTOR,
    CompletionItemKind::FIELD,
    CompletionItemKind::VARIABLE,
    CompletionItemKind::CLASS,
    CompletionItemKind::INTERFACE,
    CompletionItemKind::MODULE,
    CompletionItemKind::PROPERTY,
    CompletionItemKind::UNIT,
    CompletionItemKind::VALUE,
    CompletionItemKind::ENUM,
    CompletionItemKind::KEYWORD,
    CompletionItemKind::SNIPPET,
    CompletionItemKind::COLOR,
    CompletionItemKind::FILE,
    CompletionItemKind::REFERENCE,
    CompletionItemKind::FOLDER,
    CompletionItemKind::ENUM_MEMBER,
    CompletionItemKind::CONSTANT,
    CompletionItemKind::STRUCT,
    CompletionItemKind::EVENT,
    CompletionItemKind::OPERATOR,
    CompletionItemKind::TYPE_PARAMETER,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Trigger,
    Accept { alternate: bool },
    Next,
    Previous,
    NextPage,
    PreviousPage,
    First,
    Last,
    Hide,
    ToggleDetails,
    ToggleDetailsFocus,
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        Some(match action {
            "editor.action.triggerSuggest" => Self::Trigger,
            "acceptSelectedSuggestion" | "acceptSelectedSuggestionOnEnter" => {
                Self::Accept { alternate: false }
            }
            "acceptAlternativeSelectedSuggestion" => Self::Accept { alternate: true },
            "selectNextSuggestion" => Self::Next,
            "selectPrevSuggestion" => Self::Previous,
            "selectNextPageSuggestion" => Self::NextPage,
            "selectPrevPageSuggestion" => Self::PreviousPage,
            "selectFirstSuggestion" => Self::First,
            "selectLastSuggestion" => Self::Last,
            "hideSuggestWidget" => Self::Hide,
            "toggleSuggestionDetails" => Self::ToggleDetails,
            "toggleSuggestionFocus" => Self::ToggleDetailsFocus,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub item: CompletionItem,
    pub document: DocumentId,
    pub revision: u64,
    pub requested_byte: usize,
    pub insert: Range<usize>,
    pub replace: Range<usize>,
}

pub struct PreparationOptions {
    pub alternate: bool,
    pub indent: IndentOptions,
    pub limits: ParseLimits,
}

impl Candidate {
    pub fn new(
        document: &DocumentSnapshot,
        position: Position,
        default_range: LspRange,
        mut item: CompletionItem,
    ) -> Option<Self> {
        if item.label.is_empty() {
            return None;
        }
        let requested_byte = strict_position(document, position)?;
        let (insert, replace) = match &item.text_edit {
            Some(CompletionTextEdit::Edit(edit)) => (edit.range, edit.range),
            Some(CompletionTextEdit::InsertAndReplace(edit)) => (edit.insert, edit.replace),
            None => (default_range, default_range),
        };
        if insert.start != replace.start
            || insert.end > replace.end
            || [insert, replace].into_iter().any(|range| {
                range.start.line != position.line
                    || range.end.line != position.line
                    || range.start > position
                    || range.end < position
            })
        {
            return None;
        }
        let insert =
            strict_position(document, insert.start)?..strict_position(document, insert.end)?;
        let replace =
            strict_position(document, replace.start)?..strict_position(document, replace.end)?;
        item.kind = Some(
            item.kind
                .filter(|kind| SUPPORTED_KINDS.contains(kind))
                .unwrap_or(CompletionItemKind::TEXT),
        );
        Some(Self {
            item,
            document: document.id,
            revision: document.revision,
            requested_byte,
            insert,
            replace,
        })
    }

    pub fn text(&self) -> &str {
        match &self.item.text_edit {
            Some(CompletionTextEdit::Edit(edit)) => &edit.new_text,
            Some(CompletionTextEdit::InsertAndReplace(edit)) => &edit.new_text,
            None => self.item.insert_text.as_deref().unwrap_or(&self.item.label),
        }
    }

    pub fn is_snippet(&self) -> bool {
        self.item.insert_text_format == Some(lsp_types::InsertTextFormat::SNIPPET)
    }

    pub fn is_deprecated(&self) -> bool {
        self.item
            .tags
            .as_ref()
            .is_some_and(|tags| tags.contains(&lsp_types::CompletionItemTag::DEPRECATED))
    }

    pub fn documentation(&self) -> Option<Content> {
        self.item.documentation.clone().map(Content::from)
    }

    pub fn prepare(
        &self,
        document: &DocumentSnapshot,
        selection: &SelectionSet,
        options: PreparationOptions,
        compile: impl FnMut(&str, &str) -> Option<RegexMetadata>,
        mut resolve: impl FnMut(usize, VariableContext<'_>) -> Result<Option<String>, EditorError>,
        mut evaluate: impl FnMut(usize, &Transform, &str) -> Result<String, EditorError>,
    ) -> Result<Vec<PreparedSnippet>, EditorError> {
        let ranges = self.replacement_ranges(document, selection, options.alternate)?;
        if document.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if self.text().len() > options.limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        let markers = if self.is_snippet() {
            parse_complete(
                self.text(),
                options.limits,
                FinalTabstopOptions {
                    insert: true,
                    enforce: false,
                },
                compile,
            )?
        } else {
            vec![Marker::Text(self.text().into())]
        };
        let mut text_bytes = 0usize;
        let mut placeholder_count = 0usize;
        ranges
            .into_iter()
            .enumerate()
            .map(|(cursor_index, replace)| {
                let line = line_content_range(document, document.rope.byte_to_line(replace.start));
                if line.len() > options.limits.max_bytes {
                    return Err(EditorError::Capacity);
                }
                let byte_column = replace.start - line.start;
                let line = document.rope.byte_slice(line).to_string();
                let adjusted = adjust_whitespace(
                    &markers,
                    WhitespaceContext {
                        line: &line,
                        byte_column,
                        indent: options.indent,
                        line_ending: document.metadata.line_ending,
                        adjust_indentation: true,
                    },
                    options.limits,
                )?;
                let expansion = expand(
                    &adjusted.markers,
                    options.limits,
                    |context| resolve(cursor_index, context),
                    |transform, value| evaluate(cursor_index, transform, value),
                )?;
                text_bytes = text_bytes
                    .checked_add(expansion.text.len())
                    .ok_or(EditorError::Capacity)?;
                placeholder_count = placeholder_count
                    .checked_add(expansion.placeholders.len())
                    .ok_or(EditorError::Capacity)?;
                if text_bytes > options.limits.max_bytes
                    || placeholder_count > options.limits.max_markers
                {
                    return Err(EditorError::Capacity);
                }
                Ok(PreparedSnippet { replace, expansion })
            })
            .collect()
    }

    pub fn replacement_ranges(
        &self,
        document: &DocumentSnapshot,
        selection: &SelectionSet,
        alternate: bool,
    ) -> Result<Vec<Range<usize>>, EditorError> {
        if self.document != document.id {
            return Err(EditorError::InvalidIdentity);
        }
        if self.revision != document.revision {
            return Err(EditorError::StaleRevision);
        }
        selection.validate(&document.rope)?;
        let primary = selection.selections[selection.primary];
        if primary.head != self.requested_byte {
            return Err(EditorError::Refused);
        }
        let range = if alternate {
            &self.replace
        } else {
            &self.insert
        };
        if range.start > self.requested_byte || range.end < self.requested_byte {
            return Err(EditorError::InvalidBoundary);
        }
        byte_to_char(&document.rope, range.start)?;
        byte_to_char(&document.rope, range.end)?;
        let before = document
            .rope
            .byte_slice(range.start..self.requested_byte)
            .len_utf16_cu();
        let after = document
            .rope
            .byte_slice(self.requested_byte..range.end)
            .len_utf16_cu();
        let primary_before = extension(document, primary, before, 0);
        let primary_after = extension(document, primary, 0, after);
        Ok(selection
            .selections
            .iter()
            .map(|selection| {
                let own =
                    selection.anchor.min(selection.head)..selection.anchor.max(selection.head);
                let extended_before = extension(document, *selection, before, 0);
                let extended_after = extension(document, *selection, 0, after);
                let start = if document.rope.byte_slice(extended_before.clone())
                    == document.rope.byte_slice(primary_before.clone())
                {
                    extended_before.start
                } else {
                    own.start
                };
                let end = if document.rope.byte_slice(extended_after.clone())
                    == document.rope.byte_slice(primary_after.clone())
                {
                    extended_after.end
                } else {
                    own.end
                };
                start..end
            })
            .collect())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Candidates {
    pub items: Vec<Candidate>,
    pub is_incomplete: bool,
}

impl Candidates {
    pub fn new(
        document: &DocumentSnapshot,
        position: Position,
        default_range: LspRange,
        response: Option<CompletionResponse>,
    ) -> Self {
        let (items, is_incomplete) = match response {
            Some(CompletionResponse::Array(items)) => (items, false),
            Some(CompletionResponse::List(list)) => (list.items, list.is_incomplete),
            None => (Vec::new(), false),
        };
        Self {
            items: items
                .into_iter()
                .filter_map(|item| Candidate::new(document, position, default_range, item))
                .collect(),
            is_incomplete,
        }
    }
}

fn strict_position(document: &DocumentSnapshot, position: Position) -> Option<usize> {
    let line = usize::try_from(position.line).ok()?;
    if line >= document.rope.len_lines() {
        return None;
    }
    let range = line_content_range(document, line);
    let text = document.rope.byte_slice(range.clone());
    let units = usize::try_from(position.character).ok()?;
    if units > text.len_utf16_cu() {
        return None;
    }
    let character = text.utf16_cu_to_char(units);
    (text.char_to_utf16_cu(character) == units).then(|| range.start + text.char_to_byte(character))
}

fn extension(
    document: &DocumentSnapshot,
    selection: Selection,
    before: usize,
    after: usize,
) -> Range<usize> {
    let own = selection.anchor.min(selection.head)..selection.anchor.max(selection.head);
    if before == 0 && after == 0 {
        return own;
    }
    let line = document.rope.byte_to_line(selection.head);
    let content = line_content_range(document, line);
    if selection.head > content.end {
        return own;
    }
    let units = document
        .rope
        .byte_slice(content.start..selection.head)
        .len_utf16_cu();
    let end_units = document.rope.byte_slice(content.clone()).len_utf16_cu();
    let text = document.rope.byte_slice(content.clone());
    let start = units.saturating_sub(before);
    let end = units.saturating_add(after).min(end_units);
    let start_character = text.utf16_cu_to_char(start);
    let end_character = text.utf16_cu_to_char(end);
    if text.char_to_utf16_cu(start_character) != start
        || text.char_to_utf16_cu(end_character) != end
    {
        return own;
    }
    content.start + text.char_to_byte(start_character)
        ..content.start + text.char_to_byte(end_character)
}

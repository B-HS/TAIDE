use std::ops::Range as ByteRange;

use lsp_types::{GotoDefinitionResponse, Location, LocationLink, Position, Range, Uri};

use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::line_content_range;
use crate::lsp::{position_to_byte, range_to_bytes};

const LINE_DISTANCE_WEIGHT: u64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Definition,
    Declaration,
    TypeDefinition,
    Implementation,
    References,
}

impl Kind {
    pub fn method(self) -> &'static str {
        match self {
            Self::Definition => "textDocument/definition",
            Self::Declaration => "textDocument/declaration",
            Self::TypeDefinition => "textDocument/typeDefinition",
            Self::Implementation => "textDocument/implementation",
            Self::References => "textDocument/references",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Definition => "Definitions",
            Self::Declaration => "Declarations",
            Self::TypeDefinition => "Type Definitions",
            Self::Implementation => "Implementations",
            Self::References => "References",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    GoTo,
    Aside,
    Peek,
    Hover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Request { kind: Kind, mode: Mode },
    Close,
    Next,
    Previous,
    ToggleFocus,
    OpenSelected { side: bool },
    GotoSelected,
    Select { index: usize, focus_preview: bool },
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        let (kind, mode) = match action {
            "editor.action.revealDefinition" => (Kind::Definition, Mode::GoTo),
            "editor.action.revealDeclaration" => (Kind::Declaration, Mode::GoTo),
            "editor.action.goToTypeDefinition" => (Kind::TypeDefinition, Mode::GoTo),
            "editor.action.goToImplementation" => (Kind::Implementation, Mode::GoTo),
            "editor.action.goToReferences" => (Kind::References, Mode::GoTo),
            "editor.action.revealDefinitionAside" => (Kind::Definition, Mode::Aside),
            "editor.action.peekDefinition" => (Kind::Definition, Mode::Peek),
            "editor.action.peekDeclaration" => (Kind::Declaration, Mode::Peek),
            "editor.action.peekTypeDefinition" => (Kind::TypeDefinition, Mode::Peek),
            "editor.action.peekImplementation" => (Kind::Implementation, Mode::Peek),
            "editor.action.referenceSearch.trigger" => (Kind::References, Mode::Peek),
            "editor.action.showDefinitionPreviewHover" => (Kind::Definition, Mode::Hover),
            _ => return None,
        };
        Some(Self::Request { kind, mode })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub uri: Uri,
    pub range: Range,
    pub selection: Range,
    pub origin: Option<Range>,
}

impl Target {
    pub fn from_location(location: Location) -> Self {
        Self {
            uri: location.uri,
            range: location.range,
            selection: location.range,
            origin: None,
        }
    }

    pub fn from_link(link: LocationLink) -> Self {
        Self {
            uri: link.target_uri,
            range: link.target_range,
            selection: link.target_selection_range,
            origin: link.origin_selection_range,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.range.start <= self.range.end
            && self.range.start <= self.selection.start
            && self.selection.start <= self.selection.end
            && self.selection.end <= self.range.end
            && self.origin.is_none_or(|origin| origin.start <= origin.end)
    }
}

pub fn normalize(response: Option<GotoDefinitionResponse>) -> Vec<Target> {
    match response {
        Some(GotoDefinitionResponse::Scalar(location)) => vec![Target::from_location(location)],
        Some(GotoDefinitionResponse::Array(locations)) => {
            locations.into_iter().map(Target::from_location).collect()
        }
        Some(GotoDefinitionResponse::Link(links)) => {
            links.into_iter().map(Target::from_link).collect()
        }
        None => Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGroup {
    pub uri: Uri,
    pub references: ByteRange<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Locations {
    targets: Vec<Target>,
    groups: Vec<FileGroup>,
    first: Option<usize>,
}

impl Locations {
    pub fn new(targets: Vec<Target>) -> Self {
        let mut targets = targets
            .into_iter()
            .filter(Target::is_valid)
            .enumerate()
            .collect::<Vec<_>>();
        targets.sort_by(|(_, left), (_, right)| {
            left.uri
                .as_str()
                .encode_utf16()
                .cmp(right.uri.as_str().encode_utf16())
                .then_with(|| left.range.start.cmp(&right.range.start))
                .then_with(|| left.range.end.cmp(&right.range.end))
        });
        targets
            .dedup_by(|(_, right), (_, left)| left.uri == right.uri && left.range == right.range);
        let first = targets.iter().position(|(order, _)| *order == 0);
        let targets = targets
            .into_iter()
            .map(|(_, target)| target)
            .collect::<Vec<_>>();
        let mut groups: Vec<FileGroup> = Vec::new();
        for (index, target) in targets.iter().enumerate() {
            if let Some(group) = groups.last_mut().filter(|group| group.uri == target.uri) {
                group.references.end = index + 1;
            } else {
                groups.push(FileGroup {
                    uri: target.uri.clone(),
                    references: index..index + 1,
                });
            }
        }
        Self {
            targets,
            groups,
            first,
        }
    }

    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    pub fn update_ranges(&mut self, index: usize, range: Range, selection: Range) -> bool {
        let Some(target) = self.targets.get_mut(index) else {
            return false;
        };
        let changed = Target {
            range,
            selection,
            ..target.clone()
        };
        if !changed.is_valid() {
            return false;
        }
        *target = changed;
        true
    }

    pub fn groups(&self) -> &[FileGroup] {
        &self.groups
    }

    pub fn first(&self) -> Option<usize> {
        self.first
            .or_else(|| (!self.targets.is_empty()).then_some(0))
    }

    pub fn at(&self, uri: &Uri, position: Position) -> Option<usize> {
        self.targets.iter().position(|target| {
            target.uri == *uri
                && target.selection.start <= position
                && position <= target.selection.end
        })
    }

    pub fn nearest(&self, uri: &Uri, position: Position) -> Option<usize> {
        self.targets
            .iter()
            .enumerate()
            .min_by_key(|(index, target)| {
                let common = uri
                    .as_str()
                    .encode_utf16()
                    .zip(target.uri.as_str().encode_utf16())
                    .take_while(|(left, right)| left == right)
                    .count();
                let distance = u64::from(position.line.abs_diff(target.selection.start.line))
                    * LINE_DISTANCE_WEIGHT
                    + u64::from(
                        position
                            .character
                            .abs_diff(target.selection.start.character),
                    );
                (std::cmp::Reverse(common), distance, *index)
            })
            .map(|(index, _)| index)
    }

    pub fn next(&self, index: usize, forward: bool) -> Option<usize> {
        let count = self.targets.len();
        if index >= count {
            return None;
        }
        Some(if forward {
            (index + 1) % count
        } else {
            index.checked_sub(1).unwrap_or(count - 1)
        })
    }

    pub fn origin_range(&self, fallback: Range) -> Range {
        self.targets.iter().fold(fallback, |range, target| {
            let Some(origin) = target.origin else {
                return range;
            };
            Range::new(range.start.min(origin.start), range.end.max(origin.end))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub text: String,
    pub highlight: ByteRange<usize>,
}

pub fn preview(
    document: &DocumentSnapshot,
    range: Range,
    context_units: u32,
    word_start: impl FnOnce(usize) -> usize,
) -> Result<Preview, EditorError> {
    let bytes = range_to_bytes(document, range)?;
    let start_line = (range.start.line as usize).min(document.rope.len_lines() - 1);
    let end_line = (range.end.line as usize).min(document.rope.len_lines() - 1);
    let context = Position::new(
        range.start.line,
        range.start.character.saturating_sub(context_units),
    );
    let before_start = position_to_byte(document, context).or_else(|error| {
        if error != EditorError::InvalidBoundary || context.character == 0 {
            return Err(error);
        }
        position_to_byte(document, Position::new(context.line, context.character - 1))
    })?;
    let before_start =
        word_start(before_start).clamp(line_content_range(document, start_line).start, bytes.start);
    crate::document::byte_to_char(&document.rope, before_start)?;
    let before = document
        .rope
        .byte_slice(before_start..bytes.start)
        .to_string();
    let before = before.trim_start();
    let inside = document.rope.byte_slice(bytes.clone()).to_string();
    let after_end = line_content_range(document, end_line).end.max(bytes.end);
    let after = document.rope.byte_slice(bytes.end..after_end).to_string();
    let highlight = before.len()..before.len() + inside.len();
    Ok(Preview {
        text: format!("{before}{inside}{}", after.trim_end()),
        highlight,
    })
}

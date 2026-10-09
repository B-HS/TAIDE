use std::collections::BTreeSet;
use std::ops::Range;

use lsp_types::{
    Documentation, Hover, HoverContents, MarkedString, MarkupContent, MarkupKind,
    ParameterInformation, ParameterLabel, Position, SignatureHelp, SignatureInformation,
};

use crate::lsp::LspRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Hover,
    Signature,
}

impl Kind {
    pub fn method(self) -> &'static str {
        match self {
            Self::Hover => "textDocument/hover",
            Self::Signature => "textDocument/signatureHelp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    ShowHover,
    Signature(SignatureCommand),
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        if action == "editor.action.showHover" {
            return Some(Self::ShowHover);
        }
        SignatureCommand::from_action(action).map(Self::Signature)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichDocument {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        contents: Vec<Inline>,
    },
    Quote(Vec<Block>),
    List {
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Code {
        language: String,
        text: String,
    },
    Table {
        columns: Vec<Alignment>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Default,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(Span),
    Image {
        source: String,
        title: String,
        alt: String,
        link: Option<Link>,
        dimensions: ImageDimensions,
    },
    Break,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageDimensions {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
    pub link: Option<Link>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub target: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Plain(String),
    Markdown(String),
    Code { language: String, text: String },
}

impl Content {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Plain(text) | Self::Markdown(text) | Self::Code { text, .. } => {
                text.trim().is_empty()
            }
        }
    }
}

impl From<MarkupContent> for Content {
    fn from(content: MarkupContent) -> Self {
        if content.kind == MarkupKind::PlainText {
            Self::Plain(content.value)
        } else {
            Self::Markdown(content.value)
        }
    }
}

impl From<Documentation> for Content {
    fn from(content: Documentation) -> Self {
        match content {
            Documentation::String(text) => Self::Plain(text),
            Documentation::MarkupContent(content) => content.into(),
        }
    }
}

impl From<MarkedString> for Content {
    fn from(content: MarkedString) -> Self {
        match content {
            MarkedString::String(text) => Self::Markdown(text),
            MarkedString::LanguageString(content) => Self::Code {
                language: content.language,
                text: content.value,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverPart {
    pub range: LspRange,
    pub contents: Vec<Content>,
}

impl HoverPart {
    pub fn new(hover: Hover, fallback: LspRange, position: Position) -> Option<Self> {
        let range = hover.range.unwrap_or(fallback);
        if range.start > range.end || position < range.start || position > range.end {
            return None;
        }
        let contents = match hover.contents {
            HoverContents::Scalar(content) => vec![content.into()],
            HoverContents::Array(contents) => contents.into_iter().map(Content::from).collect(),
            HoverContents::Markup(content) => vec![content.into()],
        };
        let contents = contents
            .into_iter()
            .filter(|content| !content.is_empty())
            .collect::<Vec<_>>();
        if contents.is_empty() {
            return None;
        }
        Some(Self { range, contents })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signatures {
    help: SignatureHelp,
}

impl Signatures {
    pub fn new(mut help: SignatureHelp) -> Option<Self> {
        if help.signatures.is_empty() {
            return None;
        }
        if help
            .active_signature
            .is_none_or(|active| active as usize >= help.signatures.len())
        {
            help.active_signature = Some(0);
        }
        Some(Self { help })
    }

    pub fn value(&self) -> &SignatureHelp {
        &self.help
    }

    pub fn index(&self) -> usize {
        self.help.active_signature.unwrap_or_default() as usize
    }

    pub fn active(&self) -> &SignatureInformation {
        &self.help.signatures[self.index()]
    }

    pub fn parameter(&self) -> Option<&ParameterInformation> {
        let active = self.active();
        let index = active
            .active_parameter
            .or(self.help.active_parameter)
            .unwrap_or_default() as usize;
        active.parameters.as_ref()?.get(index)
    }

    pub fn parameter_range(&self) -> Option<Range<usize>> {
        parameter_range(&self.active().label, &self.parameter()?.label)
    }

    pub fn next(&mut self, forward: bool, cycle: bool) -> bool {
        let index = self.index();
        let length = self.help.signatures.len();
        let next = if forward {
            if index + 1 < length {
                index + 1
            } else if cycle {
                0
            } else {
                return false;
            }
        } else if index > 0 {
            index - 1
        } else if cycle {
            length - 1
        } else {
            return false;
        };
        self.help.active_signature = Some(next as u32);
        true
    }
}

pub fn parameter_range(label: &str, parameter: &ParameterLabel) -> Option<Range<usize>> {
    match parameter {
        ParameterLabel::LabelOffsets([start, end]) => {
            if start > end {
                return None;
            }
            Some(utf16_byte(label, *start)?..utf16_byte(label, *end)?)
        }
        ParameterLabel::Simple(parameter) => {
            if parameter.is_empty() {
                return None;
            }
            label.match_indices(parameter).find_map(|(start, matched)| {
                let end = start + matched.len();
                let before = label[..start].chars().next_back();
                let after = label[end..].chars().next();
                if before.is_some_and(is_js_word) || after.is_some_and(is_js_word) {
                    return None;
                }
                Some(start..end)
            })
        }
    }
}

fn utf16_byte(text: &str, requested: u32) -> Option<usize> {
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        if units == requested {
            return Some(byte);
        }
        units += character.len_utf16() as u32;
        if units > requested {
            return None;
        }
    }
    (units == requested).then_some(text.len())
}

fn is_js_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureCommand {
    Trigger,
    Close,
    Previous,
    Next,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureTriggers {
    trigger: BTreeSet<char>,
    retrigger: BTreeSet<char>,
}

impl SignatureTriggers {
    pub fn from_options<'a>(
        options: impl IntoIterator<Item = &'a lsp_types::SignatureHelpOptions>,
    ) -> Self {
        let mut triggers = Self::default();
        for options in options {
            triggers.trigger.extend(
                options
                    .trigger_characters
                    .iter()
                    .flatten()
                    .filter_map(|character| character.chars().next()),
            );
            triggers.retrigger.extend(
                options
                    .retrigger_characters
                    .iter()
                    .flatten()
                    .filter_map(|character| character.chars().next()),
            );
        }
        triggers.retrigger.extend(triggers.trigger.iter());
        triggers
    }

    pub fn matches(&self, text: &str, active: bool) -> bool {
        text.chars().next_back().is_some_and(|character| {
            self.trigger.contains(&character) || active && self.retrigger.contains(&character)
        })
    }
}

impl SignatureCommand {
    pub fn from_action(action: &str) -> Option<Self> {
        match action {
            "editor.action.triggerParameterHints" => Some(Self::Trigger),
            "closeParameterHints" => Some(Self::Close),
            "showPrevParameterHint" => Some(Self::Previous),
            "showNextParameterHint" => Some(Self::Next),
            _ => None,
        }
    }
}

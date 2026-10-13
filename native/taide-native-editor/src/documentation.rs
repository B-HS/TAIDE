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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Block {
    Inline(Vec<Inline>),
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        contents: Vec<Inline>,
    },
    Quote(Vec<Block>),
    List {
        start: Option<i64>,
        items: Vec<ListItem>,
        style: Style,
    },
    Code {
        language: String,
        text: String,
        style: Style,
    },
    Table {
        columns: Vec<Alignment>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
        layout: Option<TableLayout>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableLayout {
    pub caption: Vec<Inline>,
    pub caption_blocks: Option<Vec<Block>>,
    pub cells: Vec<Vec<TableCell>>,
    pub column_count: usize,
    pub row_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableCell {
    pub column: usize,
    pub column_span: usize,
    pub row_span: usize,
    pub alignment: Alignment,
    pub blocks: Option<Vec<Block>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub ordinal: Option<i64>,
    pub marker_style: Option<ListMarkerStyle>,
    pub blocks: Vec<Block>,
}

impl ListItem {
    pub fn marker(&self, start: Option<i64>, index: usize) -> Option<String> {
        let style = self.marker_style.unwrap_or_else(|| {
            if start.is_some() {
                return ListMarkerStyle::Decimal;
            }
            ListMarkerStyle::Disc
        });
        style.format(
            self.ordinal
                .unwrap_or_else(|| list_ordinal(start.unwrap_or(1), index)),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListMarkerStyle {
    Decimal,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    Disc,
    Circle,
    Square,
    None,
}

const ALPHABET_SIZE: i64 = 26;
const ROMAN_MAX: i64 = 3999;
const ROMAN_DIGITS: &[(i64, &str)] = &[
    (1000, "M"),
    (900, "CM"),
    (500, "D"),
    (400, "CD"),
    (100, "C"),
    (90, "XC"),
    (50, "L"),
    (40, "XL"),
    (10, "X"),
    (9, "IX"),
    (5, "V"),
    (4, "IV"),
    (1, "I"),
];

impl ListMarkerStyle {
    pub fn format(self, ordinal: i64) -> Option<String> {
        match self {
            Self::None => return None,
            Self::Disc => return Some("\u{2022}".into()),
            Self::Circle => return Some("\u{25e6}".into()),
            Self::Square => return Some("\u{25aa}".into()),
            _ => {}
        }
        let mut number = ordinal;
        let mut output = String::new();
        match self {
            Self::LowerAlpha | Self::UpperAlpha if number > 0 => {
                let mut digits = Vec::new();
                let first = if self == Self::UpperAlpha { b'A' } else { b'a' };
                while number > 0 {
                    number -= 1;
                    let digit =
                        u8::try_from(number % ALPHABET_SIZE).expect("alphabet digit is in range");
                    digits.push(char::from(first + digit));
                    number /= ALPHABET_SIZE;
                }
                output.extend(digits.into_iter().rev());
            }
            Self::LowerRoman | Self::UpperRoman if (1..=ROMAN_MAX).contains(&number) => {
                for &(value, digits) in ROMAN_DIGITS {
                    while number >= value {
                        output.push_str(digits);
                        number -= value;
                    }
                }
                if self == Self::LowerRoman {
                    output.make_ascii_lowercase();
                }
            }
            _ => output = ordinal.to_string(),
        }
        output.push('.');
        Some(output)
    }
}

pub fn list_ordinal(start: i64, index: usize) -> i64 {
    start.saturating_add(i64::try_from(index).unwrap_or(i64::MAX))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    Default,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ImageDimensions {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub text: String,
    pub style: Style,
    pub link: Option<Link>,
    pub code_element: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

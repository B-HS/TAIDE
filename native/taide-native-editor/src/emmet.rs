use std::ops::Range;

#[path = "emmet-extraction.rs"]
mod extraction;

#[path = "emmet-markup-parser.rs"]
mod markup_parser;

#[path = "emmet-markup-conversion.rs"]
mod markup_conversion;

#[path = "emmet-markup-preparation.rs"]
mod markup_preparation;

pub use extraction::extract;
pub use markup_conversion::{
    ConversionLimits, ExpandedMarkupAttribute, ExpandedMarkupNode, MarkupRepeat, MarkupValue,
    MarkupValueType, convert_markup,
};
pub use markup_parser::{MarkupAttribute, MarkupLimits, MarkupNode, Repeat, parse_markup};
pub use markup_preparation::prepare_markup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Html,
    Jsx,
    Css,
}

impl Syntax {
    pub fn for_language(language: &str) -> Option<Self> {
        match language {
            "html" | "heex" => Some(Self::Html),
            "javascriptreact" | "typescriptreact" => Some(Self::Jsx),
            "css" | "scss" => Some(Self::Css),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extraction<'a> {
    pub abbreviation: &'a str,
    pub bytes: Range<usize>,
}

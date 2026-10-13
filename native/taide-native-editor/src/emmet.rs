use std::ops::Range;

#[path = "emmet-extraction.rs"]
mod extraction;

#[path = "emmet-markup-parser.rs"]
mod markup_parser;

#[path = "emmet-markup-conversion.rs"]
mod markup_conversion;

#[path = "emmet-markup-preparation.rs"]
mod markup_preparation;

#[path = "emmet-markup-output.rs"]
mod markup_output;

#[path = "emmet-markup-filters.rs"]
mod markup_filters;

#[path = "emmet-markup-lorem.rs"]
mod markup_lorem;

#[path = "emmet-stylesheet-parser.rs"]
mod stylesheet_parser;

#[path = "emmet-stylesheet-preparation.rs"]
mod stylesheet_preparation;

#[path = "emmet-snippet-output.rs"]
mod snippet_output;

#[path = "emmet-stylesheet-format.rs"]
mod stylesheet_format;

#[path = "emmet-stylesheet-output.rs"]
mod stylesheet_output;

#[path = "emmet-completion.rs"]
mod completion;

pub use completion::{CompletionLimits, Suggestion, complete_document, complete_line};
pub use extraction::extract;
pub use markup_conversion::{
    ConversionLimits, ExpandedMarkupAttribute, ExpandedMarkupNode, MarkupRepeat, MarkupValue,
    MarkupValueType, convert_markup,
};
pub use markup_filters::{MarkupFilters, markup_filters, prepare_markup_with_filters};
pub use markup_output::{
    expand_markup, expand_markup_with_filters, expand_markup_with_random, render_markup,
};
pub use markup_parser::{MarkupAttribute, MarkupLimits, MarkupNode, Repeat, parse_markup};
pub use markup_preparation::{prepare_markup, prepare_markup_with_random};
pub use stylesheet_output::{expand_stylesheet, render_stylesheet};
pub use stylesheet_parser::{
    StylesheetProperty, StylesheetToken, StylesheetValue, parse_stylesheet,
};
pub use stylesheet_preparation::prepare_stylesheet;

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

use std::ops::Range;

#[path = "emmet-extraction.rs"]
mod extraction;

pub use extraction::extract;

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

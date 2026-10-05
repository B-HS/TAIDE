use taide_model::file::{EditorConfigIndentStyle, EditorConfigOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndentOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
}

pub fn resolve(config: &EditorConfigOptions, current: IndentOptions) -> IndentOptions {
    let tab_size = match config.indent_style {
        Some(EditorConfigIndentStyle::Tab) => config.tab_width.or(config.indent_size),
        _ => config.indent_size.or(config.tab_width),
    }
    .unwrap_or(current.tab_size);
    let insert_spaces = config
        .indent_style
        .map(|style| style == EditorConfigIndentStyle::Space)
        .unwrap_or(current.insert_spaces);
    IndentOptions {
        tab_size,
        insert_spaces,
    }
}

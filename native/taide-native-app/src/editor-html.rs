use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use taide_lsp::native::protocol::lsp_types;
use taide_model::file::{FileSizeTier, LARGE_FILE_BYTES};
use taide_native_editor::document::{DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::document_symbols::{DocumentSymbols, Symbol, SymbolKind};
use taide_native_editor::html_folding;
use taide_native_editor::html_format;
use taide_native_editor::html_parser;
use taide_native_editor::html_selection;
use taide_native_editor::html_symbols;
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::lsp::TextEdit;
use taide_native_editor::syntax_folding::SyntaxFolds;

const LANGUAGE_ID: &str = "html";
const MAX_TOKENS: usize = 1024 * 1024;
const MAX_NODES: usize = 256 * 1024;
const MAX_DEPTH: usize = 8192;
const MAX_NESTED_DOCUMENTS: usize = 64;
const MAX_INDENT_WIDTH: usize = 1024;
const MAX_SELECTION_POSITIONS: usize = 1024;
const TEXT_MULTIPLIER: usize = 4;
const PARSER_LIMITS: html_parser::Limits = html_parser::Limits {
    max_bytes: LARGE_FILE_BYTES as usize,
    max_tokens: MAX_TOKENS,
    max_nodes: MAX_NODES,
};
const SYMBOL_LIMITS: html_symbols::Limits = html_symbols::Limits {
    parser: PARSER_LIMITS,
    max_text_bytes: LARGE_FILE_BYTES as usize * TEXT_MULTIPLIER,
};
const FOLD_LIMITS: html_folding::Limits = html_folding::Limits {
    max_bytes: LARGE_FILE_BYTES as usize,
    max_tokens: MAX_TOKENS,
    max_depth: MAX_DEPTH,
    max_ranges: MAX_NODES,
};
const SELECTION_LIMITS: html_selection::Limits = html_selection::Limits {
    parser: PARSER_LIMITS,
    max_positions: MAX_SELECTION_POSITIONS,
    max_ranges: MAX_NODES,
};
const FORMAT_LIMITS: html_format::Limits = html_format::Limits {
    max_bytes: LARGE_FILE_BYTES as usize,
    max_output_bytes: LARGE_FILE_BYTES as usize * TEXT_MULTIPLIER,
    max_indent_width: MAX_INDENT_WIDTH,
    max_tokens: MAX_TOKENS,
    max_depth: MAX_NESTED_DOCUMENTS,
};

pub(crate) fn is_html(snapshot: &DocumentSnapshot) -> bool {
    snapshot.metadata.tier == FileSizeTier::Normal
        && !matches!(
            snapshot.key,
            DocumentKey::AppFile(_) | DocumentKey::Virtual(_)
        )
        && snapshot.metadata.language_id == LANGUAGE_ID
}

pub(crate) fn models(
    snapshot: &DocumentSnapshot,
    cancelled: &AtomicBool,
) -> Result<crate::editor_json::Models, EditorError> {
    if snapshot.rope.len_bytes() > PARSER_LIMITS.max_bytes {
        return Err(EditorError::Capacity);
    }
    let text = snapshot.rope.to_string();
    let symbols = html_symbols::document_symbols(&text, SYMBOL_LIMITS)?
        .into_iter()
        .map(|symbol| Symbol {
            name: symbol.name,
            detail: String::new(),
            kind: SymbolKind::FIELD,
            tags: Vec::new(),
            parent: None,
            container_label: symbol.container,
            selection: symbol.bytes.clone(),
            bytes: symbol.bytes,
        })
        .collect::<Vec<_>>();
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    let folds = html_folding::folding_ranges(&text, FOLD_LIMITS)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    Ok(crate::editor_json::Models {
        symbols: Some(Arc::new(DocumentSymbols::from_symbols(snapshot, symbols)?)),
        folds: Arc::new(SyntaxFolds::new(snapshot, vec![folds])),
    })
}

pub(crate) fn selection_ranges(
    snapshot: &DocumentSnapshot,
    positions: &[usize],
) -> Result<Vec<Vec<Range<usize>>>, EditorError> {
    if snapshot.rope.len_bytes() > PARSER_LIMITS.max_bytes {
        return Err(EditorError::Capacity);
    }
    html_selection::selection_ranges(&snapshot.rope.to_string(), positions, SELECTION_LIMITS)
}

pub(crate) fn is_format_eligible(snapshot: &DocumentSnapshot) -> bool {
    is_html(snapshot) && !snapshot.metadata.read_only
}

pub(crate) fn format(
    snapshot: &DocumentSnapshot,
    kind: &crate::editor_formatting::Kind,
    options: &lsp_types::FormattingOptions,
) -> Result<Vec<TextEdit>, EditorError> {
    if !is_format_eligible(snapshot) {
        return Err(EditorError::Refused);
    }
    let options = html_format::Options {
        indentation: IndentOptions {
            tab_size: options.tab_size,
            insert_spaces: options.insert_spaces,
        },
    };
    crate::editor_css::format_ranges(snapshot, kind, FORMAT_LIMITS.max_bytes, |text, range| {
        html_format::format(text, range, options, FORMAT_LIMITS)
    })
}

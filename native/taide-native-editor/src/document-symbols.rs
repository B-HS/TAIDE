use std::ops::Range;
use std::sync::Arc;

pub use lsp_types::SymbolKind;
use lsp_types::{DocumentSymbolResponse, SymbolTag, Uri};

use crate::document::{DocumentId, DocumentKey, DocumentSnapshot, EditorError};
use crate::lsp::{byte_to_position, range_to_bytes};
use crate::sticky_model::{StickyModel, StickyScope};

const SYMBOL_KINDS: [SymbolKind; 26] = [
    SymbolKind::FILE,
    SymbolKind::MODULE,
    SymbolKind::NAMESPACE,
    SymbolKind::PACKAGE,
    SymbolKind::CLASS,
    SymbolKind::METHOD,
    SymbolKind::PROPERTY,
    SymbolKind::FIELD,
    SymbolKind::CONSTRUCTOR,
    SymbolKind::ENUM,
    SymbolKind::INTERFACE,
    SymbolKind::FUNCTION,
    SymbolKind::VARIABLE,
    SymbolKind::CONSTANT,
    SymbolKind::STRING,
    SymbolKind::NUMBER,
    SymbolKind::BOOLEAN,
    SymbolKind::ARRAY,
    SymbolKind::OBJECT,
    SymbolKind::KEY,
    SymbolKind::NULL,
    SymbolKind::ENUM_MEMBER,
    SymbolKind::STRUCT,
    SymbolKind::EVENT,
    SymbolKind::OPERATOR,
    SymbolKind::TYPE_PARAMETER,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub detail: String,
    pub kind: SymbolKind,
    pub tags: Vec<SymbolTag>,
    pub parent: Option<usize>,
    pub container_label: String,
    pub bytes: Range<usize>,
    pub selection: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbols {
    document: DocumentId,
    key: DocumentKey,
    revision: u64,
    language: String,
    symbols: Vec<Symbol>,
    sticky: Option<Arc<StickyModel>>,
}

impl DocumentSymbols {
    pub fn new(
        document: &DocumentSnapshot,
        uri: &Uri,
        response: Option<DocumentSymbolResponse>,
    ) -> Result<Self, EditorError> {
        let mut symbols = Vec::new();
        match response {
            Some(DocumentSymbolResponse::Nested(roots)) => {
                let mut pending = roots
                    .into_iter()
                    .rev()
                    .map(|node| (node, None))
                    .collect::<Vec<_>>();
                while let Some((node, parent)) = pending.pop() {
                    let bytes = range_to_bytes(document, node.range)?;
                    let selection = range_to_bytes(document, node.selection_range)?;
                    if selection.start < bytes.start || selection.end > bytes.end {
                        return Err(EditorError::InvalidBoundary);
                    }
                    let container_label = parent.map_or_else(String::new, |index| {
                        let ancestor: &Symbol = &symbols[index];
                        if ancestor.container_label.is_empty() {
                            ancestor.name.clone()
                        } else {
                            format!("{} > {}", ancestor.container_label, ancestor.name)
                        }
                    });
                    let index = symbols.len();
                    symbols.push(Symbol {
                        name: node.name,
                        detail: node.detail.unwrap_or_default(),
                        kind: normalize_kind(node.kind),
                        tags: node.tags.unwrap_or_default(),
                        parent,
                        container_label,
                        bytes,
                        selection,
                    });
                    pending.extend(
                        node.children
                            .unwrap_or_default()
                            .into_iter()
                            .rev()
                            .map(|child| (child, Some(index))),
                    );
                }
            }
            Some(DocumentSymbolResponse::Flat(entries)) => {
                for entry in entries {
                    if entry.location.uri != *uri {
                        continue;
                    }
                    let bytes = range_to_bytes(document, entry.location.range)?;
                    symbols.push(Symbol {
                        name: entry.name,
                        detail: String::new(),
                        kind: normalize_kind(entry.kind),
                        tags: entry.tags.unwrap_or_default(),
                        parent: None,
                        container_label: String::new(),
                        selection: bytes.clone(),
                        bytes,
                    });
                }
            }
            None => {}
        }
        let sticky = if symbols.is_empty() {
            None
        } else {
            let scopes = symbols
                .iter()
                .map(|symbol| {
                    Ok(StickyScope {
                        start_line: usize::try_from(
                            byte_to_position(document, symbol.selection.start)?.line,
                        )
                        .map_err(|_| EditorError::Capacity)?,
                        end_line: usize::try_from(
                            byte_to_position(document, symbol.bytes.end)?.line,
                        )
                        .map_err(|_| EditorError::Capacity)?
                        .saturating_add(1),
                    })
                })
                .collect::<Result<Vec<_>, EditorError>>()?;
            Some(Arc::new(StickyModel::new(document, &scopes)))
        };
        Ok(Self {
            document: document.id,
            key: document.key.clone(),
            revision: document.revision,
            language: document.metadata.language_id.clone(),
            symbols,
            sticky,
        })
    }

    pub fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.document == document.id
            && self.key == document.key
            && self.revision == document.revision
            && self.language == document.metadata.language_id
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn sticky_model(&self) -> Option<&Arc<StickyModel>> {
        self.sticky.as_ref()
    }
}

fn normalize_kind(kind: SymbolKind) -> SymbolKind {
    if SYMBOL_KINDS.contains(&kind) {
        kind
    } else {
        SymbolKind::VARIABLE
    }
}

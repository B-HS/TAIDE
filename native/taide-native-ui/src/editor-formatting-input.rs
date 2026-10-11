use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::lsp::{LspRange, Position, byte_to_position};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Type { character: char, position: Position },
    Paste { range: LspRange },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub document: DocumentId,
    pub revision: u64,
    pub kind: Kind,
}

impl Input {
    pub(crate) fn typed(
        store: &EditorStore,
        view: ViewId,
        text: &str,
    ) -> Result<Option<Self>, EditorError> {
        let Some(character) = text.chars().last() else {
            return Ok(None);
        };
        Self::capture(store, view, |position, empty| {
            empty.then_some(Kind::Type {
                character,
                position,
            })
        })
    }

    pub(crate) fn pasted(
        store: &EditorStore,
        view: ViewId,
        start: Position,
    ) -> Result<Option<Self>, EditorError> {
        Self::capture(store, view, |end, _| {
            Some(Kind::Paste {
                range: LspRange::new(start.min(end), start.max(end)),
            })
        })
    }

    fn capture(
        store: &EditorStore,
        view: ViewId,
        kind: impl FnOnce(Position, bool) -> Option<Kind>,
    ) -> Result<Option<Self>, EditorError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        if current.composition.is_some() || current.selection.selections.len() != 1 {
            return Ok(None);
        }
        let document = store.documents().snapshot(current.document)?;
        if document.metadata.read_only {
            return Ok(None);
        }
        let selection = current.selection.selections[0];
        let position = byte_to_position(&document, selection.anchor.min(selection.head))?;
        Ok(
            kind(position, selection.anchor == selection.head).map(|kind| Self {
                document: current.document,
                revision: document.revision,
                kind,
            }),
        )
    }
}

use std::collections::HashMap;

use taide_lsp::native::protocol::lsp_types::{self, DocumentChanges, OneOf};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::store::EditorStore;

use crate::lsp::{ProtocolDocument, WorkspaceEditEvent};

#[derive(Debug, PartialEq, Eq)]
pub enum EditFailure {
    UnconnectedResource,
    UnconnectedDocument,
    StaleVersion,
    Editor(EditorError),
}

pub struct EditOutcome {
    pub changed: Vec<DocumentId>,
    pub failure: Option<EditFailure>,
}

pub fn apply_open(store: &mut EditorStore, event: &WorkspaceEditEvent) -> EditOutcome {
    let mut outcome = EditOutcome {
        changed: Vec::new(),
        failure: None,
    };
    let mut expected = event
        .documents
        .iter()
        .map(|document| (document.uri.clone(), document.snapshot.clone()))
        .collect::<HashMap<_, _>>();
    let edits = match &event.edit.document_changes {
        Some(DocumentChanges::Edits(edits)) => edits.clone(),
        Some(DocumentChanges::Operations(operations)) => {
            let mut edits = Vec::new();
            for operation in operations {
                let lsp_types::DocumentChangeOperation::Edit(edit) = operation else {
                    outcome.failure = Some(EditFailure::UnconnectedResource);
                    return outcome;
                };
                edits.push(edit.clone());
            }
            edits
        }
        None => event
            .edit
            .changes
            .as_ref()
            .map(|changes| {
                changes
                    .iter()
                    .map(|(uri, edits)| lsp_types::TextDocumentEdit {
                        text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: None,
                        },
                        edits: edits.iter().cloned().map(OneOf::Left).collect(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    };
    for edit in edits {
        let uri = edit.text_document.uri.as_str();
        let Some(ProtocolDocument {
            snapshot, revision, ..
        }) = event.documents.iter().find(|document| document.uri == uri)
        else {
            outcome.failure = Some(EditFailure::UnconnectedDocument);
            break;
        };
        if revision.is_some_and(|revision| {
            edit.text_document
                .version
                .is_some_and(|version| u64::try_from(version).ok() != Some(revision))
        }) {
            outcome.failure = Some(EditFailure::StaleVersion);
            break;
        }
        let Some(requested) = expected.get(uri) else {
            outcome.failure = Some(EditFailure::UnconnectedDocument);
            break;
        };
        let view = store
            .views()
            .for_document(snapshot.id)
            .next()
            .map(|view| view.id);
        let edits = edit
            .edits
            .into_iter()
            .map(|edit| match edit {
                OneOf::Left(edit) => edit,
                OneOf::Right(edit) => edit.text_edit,
            })
            .collect();
        match taide_native_editor::lsp::apply_text_edits(store, requested, view, edits) {
            Ok(changed) => {
                if changed && !outcome.changed.contains(&snapshot.id) {
                    outcome.changed.push(snapshot.id);
                }
                if let Ok(current) = store.documents().snapshot(snapshot.id) {
                    expected.insert(uri.into(), current);
                }
            }
            Err(error) => {
                outcome.failure = Some(EditFailure::Editor(error));
                break;
            }
        }
    }
    outcome
}

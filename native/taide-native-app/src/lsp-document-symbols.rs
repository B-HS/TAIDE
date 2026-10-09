use super::{Document, Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_symbols::{Group, ProviderIdentity, Response};
use std::collections::HashMap;
use std::sync::Arc;
use taide_native_editor::document_symbols::DocumentSymbols;

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &crate::editor_symbols::Request,
) -> Result<Response, Failure> {
    let snapshot = &request.snapshot;
    let taide_native_editor::document::DocumentKey::File(path) = &snapshot.key else {
        return Err(Failure::DocumentNotOpen);
    };
    let uri =
        taide_lsp::service::workspace_folder_uri(path.to_str().ok_or(Failure::MalformedRequest)?)
            .parse()
            .map_err(|_| Failure::MalformedRequest)?;
    let empty = Arc::new(
        DocumentSymbols::new(snapshot, &uri, None).map_err(|_| Failure::MalformedResponse)?,
    );
    let mut result = Response {
        palette_provider: None,
        palette: empty.clone(),
        groups: Vec::new(),
        error: None,
    };
    let mut candidates = sessions
        .iter()
        .filter_map(|(key, session)| {
            if key.project != request.project {
                return None;
            }
            let document = session.documents.get(&snapshot.id)?;
            Some((key, session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.0
            .server
            .cmp(&right.0.server)
            .then_with(|| left.0.root.cmp(&right.0.root))
    });
    for (_, session, document) in candidates {
        if request.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if !describes(document, snapshot) {
            return Err(Failure::StaleRevision);
        }
        let mut client = session.client.clone();
        if client.snapshot().phase != Phase::Running {
            client.wait_for_phase(Phase::Running).await?;
        }
        let before = client.snapshot();
        let uri: lsp_types::Uri = document
            .uri
            .parse()
            .map_err(|_| Failure::MalformedRequest)?;
        let response = client
            .request_typed::<lsp_types::request::DocumentSymbolRequest>(
                lsp_types::DocumentSymbolParams {
                    text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                },
                Some((document.uri.clone(), document.protocol_revision)),
            )
            .await;
        let after = client.snapshot();
        if before.generation != after.generation
            || before.capability_revision != after.capability_revision
            || after.phase != Phase::Running
        {
            return Err(Failure::StaleGeneration);
        }
        match response {
            Ok(response) => {
                let provider = ProviderIdentity {
                    owner: session.owner,
                    generation: after.generation,
                    capability_revision: after.capability_revision,
                };
                let model = DocumentSymbols::new(snapshot, &uri, response.value)
                    .map(Arc::new)
                    .map_err(|_| Failure::MalformedResponse);
                if result.palette_provider.is_none() {
                    result.palette_provider = Some(provider);
                    result.palette = model.clone().unwrap_or_else(|_| empty.clone());
                    result.error = model.as_ref().err().cloned();
                }
                if let Ok(model) = model {
                    result.groups.push(Group { provider, model });
                }
            }
            Err(Failure::UnsupportedCapability) => continue,
            Err(
                error @ (Failure::StaleGeneration
                | Failure::StaleRevision
                | Failure::Restarted
                | Failure::Cancelled),
            ) => return Err(error),
            Err(error) => {
                if result.palette_provider.is_none() {
                    result.palette_provider = Some(ProviderIdentity {
                        owner: session.owner,
                        generation: after.generation,
                        capability_revision: after.capability_revision,
                    });
                    result.error = Some(error);
                }
            }
        }
    }
    Ok(result)
}

fn describes(
    document: &Document,
    snapshot: &taide_native_editor::document::DocumentSnapshot,
) -> bool {
    document.snapshot.id == snapshot.id
        && document.snapshot.key == snapshot.key
        && document.snapshot.revision == snapshot.revision
        && document.snapshot.metadata.language_id == snapshot.metadata.language_id
}

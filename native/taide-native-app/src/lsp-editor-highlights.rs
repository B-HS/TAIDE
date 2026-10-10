use std::collections::HashMap;

use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_highlights::{Request, Response};
use crate::editor_symbols::ProviderIdentity;

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Response, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let mut candidates = sessions
        .iter()
        .filter_map(|(key, session)| {
            if key.project != request.project {
                return None;
            }
            let document = session.documents.get(&request.snapshot.id)?;
            (document.snapshot.key == request.snapshot.key
                && document.snapshot.revision == request.snapshot.revision
                && document.snapshot.metadata.language_id == request.snapshot.metadata.language_id)
                .then_some((session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(session, _)| std::cmp::Reverse(session.order));
    for (session, document) in candidates {
        let before = session.client.snapshot();
        if !before.supports_document(&document.uri, "textDocument/documentHighlight") {
            continue;
        }
        let uri = document
            .uri
            .parse()
            .map_err(|_| Failure::MalformedRequest)?;
        let result = session
            .client
            .request_typed::<lsp_types::request::DocumentHighlightRequest>(
                lsp_types::DocumentHighlightParams {
                    text_document_position_params: lsp_types::TextDocumentPositionParams {
                        text_document: lsp_types::TextDocumentIdentifier { uri },
                        position: lsp_types::Position {
                            line: request.position.line,
                            character: request.position.character,
                        },
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                },
                Some((document.uri.clone(), document.protocol_revision)),
            )
            .await;
        if request.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        let after = session.client.snapshot();
        if after.phase != Phase::Running
            || before.generation != after.generation
            || before.capability_revision != after.capability_revision
        {
            continue;
        }
        if let Ok(reply) = result {
            return Ok(Response {
                provider: Some(ProviderIdentity {
                    owner: session.owner,
                    generation: after.generation,
                    capability_revision: after.capability_revision,
                }),
                highlights: reply.value.unwrap_or_default(),
            });
        }
    }
    Ok(Response {
        provider: None,
        highlights: Vec::new(),
    })
}

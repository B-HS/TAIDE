use std::collections::HashMap;

use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_inlay_hints::{Request, Response};
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
    let groups = futures_util::future::join_all(candidates.into_iter().map(
        |(session, document)| async move {
            let before = session.client.snapshot();
            if !before.supports_document(&document.uri, "textDocument/inlayHint") {
                return None;
            }
            let Ok(uri) = document.uri.parse() else {
                return None;
            };
            let result = session
                .client
                .request_typed::<lsp_types::request::InlayHintRequest>(
                    lsp_types::InlayHintParams {
                        text_document: lsp_types::TextDocumentIdentifier { uri },
                        range: lsp_types::Range {
                            start: lsp_types::Position {
                                line: request.range.start.line,
                                character: request.range.start.character,
                            },
                            end: lsp_types::Position {
                                line: request.range.end.line,
                                character: request.range.end.character,
                            },
                        },
                        work_done_progress_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await;
            if request.is_cancelled() {
                return None;
            }
            let after = session.client.snapshot();
            if after.phase != Phase::Running
                || before.generation != after.generation
                || before.capability_revision != after.capability_revision
            {
                return None;
            }
            if let Ok(reply) = result {
                return Some((
                    ProviderIdentity {
                        owner: session.owner,
                        generation: after.generation,
                        capability_revision: after.capability_revision,
                    },
                    reply.value.unwrap_or_default(),
                ));
            }
            None
        },
    ))
    .await;
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    Ok(Response {
        groups: groups.into_iter().flatten().collect(),
    })
}

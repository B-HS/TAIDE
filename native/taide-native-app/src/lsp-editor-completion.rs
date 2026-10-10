use std::collections::HashMap;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use taide_native_editor::completion::Candidates;

use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_completion::{Group, Request, Response};
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
    let mut pending = candidates
        .into_iter()
        .enumerate()
        .map(|(ordinal, (session, document))| async move {
            let before = session.client.snapshot();
            let provider = ProviderIdentity {
                owner: session.owner,
                generation: before.generation,
                capability_revision: before.capability_revision,
            };
            if request.is_cancelled()
                || !request.query_providers.contains(&provider)
                || !before.supports_document(&document.uri, "textDocument/completion")
            {
                return None;
            }
            let response = session
                .client
                .request_typed::<lsp_types::request::Completion>(
                    lsp_types::CompletionParams {
                        text_document_position: lsp_types::TextDocumentPositionParams {
                            text_document: lsp_types::TextDocumentIdentifier {
                                uri: document.uri.parse().ok()?,
                            },
                            position: request.position,
                        },
                        context: None,
                        work_done_progress_params: Default::default(),
                        partial_result_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await
                .ok()?
                .value;
            let after = session.client.snapshot();
            if request.is_cancelled()
                || after.phase != Phase::Running
                || before.generation != after.generation
                || before.capability_revision != after.capability_revision
            {
                return None;
            }
            Some(Group {
                provider,
                ordinal,
                candidates: Candidates::new(
                    &request.snapshot,
                    request.position,
                    request.word,
                    response,
                ),
            })
        })
        .collect::<FuturesUnordered<_>>();
    let mut groups = Vec::new();
    while let Some(group) = pending.next().await {
        if request.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if let Some(group) = group {
            groups.push(group);
        }
    }
    groups.sort_by_key(|group| group.ordinal);
    Ok(Response { groups })
}

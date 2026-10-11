use std::collections::HashMap;

use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_selection_ranges::{Request, Response};
use crate::editor_symbols::ProviderIdentity;
use taide_native_editor::lsp::byte_to_position;

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Response, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let positions = request
        .selection
        .selections
        .iter()
        .map(|selection| {
            byte_to_position(&request.snapshot, selection.head)
                .map(|position| lsp_types::Position::new(position.line, position.character))
                .map_err(|_| Failure::MalformedRequest)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut candidates = sessions
        .iter()
        .filter_map(|(key, session)| {
            if Some(&key.project) != request.project.as_ref() {
                return None;
            }
            let document = session.documents.get(&request.snapshot.id)?;
            (document.snapshot.key == request.snapshot.key
                && document.snapshot.revision == request.snapshot.revision
                && document.snapshot.metadata.language_id == request.snapshot.metadata.language_id
                && document.snapshot.metadata.tier == request.snapshot.metadata.tier)
                .then_some((session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(session, _)| std::cmp::Reverse(session.order));
    let groups =
        futures_util::future::join_all(candidates.into_iter().map(|(session, document)| {
            let positions = positions.clone();
            async move {
                let before = session.client.snapshot();
                let provider = ProviderIdentity {
                    owner: session.owner,
                    generation: before.generation,
                    capability_revision: before.capability_revision,
                };
                if !request.providers.contains(&provider)
                    || !before.supports_document(&document.uri, "textDocument/selectionRange")
                {
                    return None;
                }
                let Ok(uri) = document.uri.parse() else {
                    return None;
                };
                let result = session
                    .client
                    .request_typed::<lsp_types::request::SelectionRangeRequest>(
                        lsp_types::SelectionRangeParams {
                            text_document: lsp_types::TextDocumentIdentifier { uri },
                            positions,
                            work_done_progress_params: Default::default(),
                            partial_result_params: Default::default(),
                        },
                        Some((document.uri.clone(), document.protocol_revision)),
                    )
                    .await;
                let after = session.client.snapshot();
                if request.is_cancelled()
                    || after.phase != Phase::Running
                    || before.generation != after.generation
                    || before.capability_revision != after.capability_revision
                {
                    return None;
                }
                result.ok().map(|reply| (provider, reply.value))
            }
        }))
        .await;
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    Ok(Response {
        groups: groups.into_iter().flatten().collect(),
    })
}

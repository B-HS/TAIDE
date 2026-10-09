use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_folding::{Group, Request, Response};
use crate::editor_symbols::ProviderIdentity;
use std::collections::HashMap;
use taide_native_editor::folding::FoldRegion;
use taide_native_editor::syntax_folding::SyntaxFoldRange;

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Response, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let mut candidates = sessions
        .values()
        .filter_map(|session| {
            let document = session.documents.get(&request.snapshot.id)?;
            Some((session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(session, _)| std::cmp::Reverse(session.order));
    let groups = futures_util::future::join_all(candidates.into_iter().map(
        |(session, document)| async move {
            if document.snapshot.key != request.snapshot.key
                || document.snapshot.revision != request.snapshot.revision
                || document.snapshot.metadata.language_id != request.snapshot.metadata.language_id
            {
                return None;
            }
            let mut client = session.client.clone();
            if client.snapshot().phase != Phase::Running {
                client.wait_for_phase(Phase::Running).await.ok()?;
            }
            let before = client.snapshot();
            let response = client
                .request_typed::<lsp_types::request::FoldingRangeRequest>(
                    lsp_types::FoldingRangeParams {
                        text_document: lsp_types::TextDocumentIdentifier {
                            uri: document.uri.parse().ok()?,
                        },
                        work_done_progress_params: Default::default(),
                        partial_result_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await
                .ok()?;
            let after = client.snapshot();
            if before.generation != after.generation
                || before.capability_revision != after.capability_revision
                || after.phase != Phase::Running
            {
                return None;
            }
            Some(Group {
                provider: ProviderIdentity {
                    owner: session.owner,
                    generation: after.generation,
                    capability_revision: after.capability_revision,
                },
                ranges: response.value.map(|ranges| {
                    ranges
                        .into_iter()
                        .map(|range| SyntaxFoldRange {
                            region: FoldRegion {
                                start_line: range.range().start_line as usize,
                                end_line: range.range().end_line as usize,
                            },
                            kind: range.kind,
                        })
                        .collect()
                }),
            })
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

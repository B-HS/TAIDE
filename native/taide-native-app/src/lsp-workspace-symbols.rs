use super::{Failure, Phase, Session, SessionClient, SessionKey, lsp_types};
use std::collections::HashMap;
use taide_model::ids::ProjectId;

use crate::editor_symbols::ProviderIdentity;
use crate::workspace_symbols::{Group, Request, Response};

pub(super) struct Candidate {
    client: SessionClient,
    owner: crate::diagnostics::Owner,
    order: usize,
}

pub(super) fn select(
    sessions: &HashMap<SessionKey, Session>,
    project: &ProjectId,
) -> Vec<Candidate> {
    let mut candidates = sessions
        .iter()
        .filter(|(key, session)| {
            key.project == *project
                && !matches!(
                    session.client.snapshot().phase,
                    Phase::Stopping | Phase::Stopped
                )
        })
        .map(|(_, session)| Candidate {
            client: session.client.clone(),
            owner: session.owner,
            order: session.order,
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.order);
    candidates
}

pub(super) async fn request(
    candidates: &[Candidate],
    request: &Request,
) -> Result<Response, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let groups = futures_util::future::join_all(candidates.iter().map(|candidate| async {
        let mut client = candidate.client.clone();
        if client.snapshot().phase != Phase::Running {
            client.wait_for_phase(Phase::Running).await.ok()?;
        }
        let before = client.snapshot();
        let response = client
            .request_typed::<lsp_types::request::WorkspaceSymbolRequest>(
                lsp_types::WorkspaceSymbolParams {
                    query: request.term.clone(),
                    partial_result_params: Default::default(),
                    work_done_progress_params: Default::default(),
                },
                None,
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
                owner: candidate.owner,
                generation: after.generation,
                capability_revision: after.capability_revision,
            },
            symbols: crate::workspace_symbols::normalize(response.value),
        })
    }))
    .await;
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    Ok(Response {
        groups: groups.into_iter().flatten().collect(),
    })
}

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use taide_native_editor::documentation::HoverPart;
use tokio::sync::mpsc;

use super::{Document, Failure, Phase, Reply, Session, SessionKey, lsp_types};
use crate::editor_documentation::{HoverGroup, Kind, Request, Response, SignatureGroup};
use crate::editor_symbols::ProviderIdentity;

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
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
    if request.kind == Kind::Signature {
        for (session, document) in candidates {
            let before = session.client.snapshot();
            if !before.supports_document(&document.uri, request.kind.method()) {
                continue;
            }
            let result = session
                .client
                .request_typed::<lsp_types::request::SignatureHelpRequest>(
                    lsp_types::SignatureHelpParams {
                        context: None,
                        text_document_position_params: position(request, document)?,
                        work_done_progress_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await;
            let after = session.client.snapshot();
            if request.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            if after.phase != Phase::Running
                || before.generation != after.generation
                || before.capability_revision != after.capability_revision
            {
                continue;
            }
            if let Ok(reply) = result
                && let Some(help) = reply.value
            {
                return Ok(Response::Signature(Some(SignatureGroup {
                    provider: ProviderIdentity {
                        owner: session.owner,
                        generation: after.generation,
                        capability_revision: after.capability_revision,
                    },
                    help,
                })));
            }
        }
        return Ok(Response::Signature(None));
    }
    let mut pending = candidates
        .into_iter()
        .enumerate()
        .map(|(ordinal, (session, document))| async move {
            let before = session.client.snapshot();
            if !before.supports_document(&document.uri, request.kind.method()) {
                return None;
            }
            let result = session
                .client
                .request_typed::<lsp_types::request::HoverRequest>(
                    lsp_types::HoverParams {
                        text_document_position_params: position(request, document).ok()?,
                        work_done_progress_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await
                .ok()?
                .value?;
            let after = session.client.snapshot();
            if request.is_cancelled()
                || after.phase != Phase::Running
                || before.generation != after.generation
                || before.capability_revision != after.capability_revision
            {
                return None;
            }
            Some((
                ordinal,
                HoverGroup {
                    provider: ProviderIdentity {
                        owner: session.owner,
                        generation: after.generation,
                        capability_revision: after.capability_revision,
                    },
                    part: HoverPart::new(result, request.fallback, request.position)?,
                },
            ))
        })
        .collect::<FuturesUnordered<_>>();
    while let Some(part) = pending.next().await {
        if request.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if let Some(part) = part {
            replies
                .send(Reply::Documentation {
                    request: request.clone(),
                    result: Ok(Response::Hover {
                        part: Some(part),
                        complete: false,
                    }),
                })
                .await
                .map_err(|_| Failure::TransportClosed)?;
            repaint();
        }
    }
    Ok(Response::Hover {
        part: None,
        complete: true,
    })
}

fn position(
    request: &Request,
    document: &Document,
) -> Result<lsp_types::TextDocumentPositionParams, Failure> {
    Ok(lsp_types::TextDocumentPositionParams {
        text_document: lsp_types::TextDocumentIdentifier {
            uri: document
                .uri
                .parse()
                .map_err(|_| Failure::MalformedRequest)?,
        },
        position: request.position,
    })
}

use std::collections::HashMap;

use super::{Failure, Phase, Session, SessionKey, lsp_types, protocol_documents};
use crate::editor_rename::{Edits, Prepared, Request, Response, Stage};
use crate::editor_symbols::ProviderIdentity;
use taide_native_editor::lsp::range_to_bytes;

const METHOD: &str = "textDocument/rename";
const PREPARE_METHOD: &str = "textDocument/prepareRename";

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Response, Failure> {
    let mut candidates = sessions
        .iter()
        .filter_map(|(key, session)| {
            let document = session.documents.get(&request.snapshot.id)?;
            let state = session.client.snapshot();
            (key.project == request.project
                && document.snapshot.key == request.snapshot.key
                && document.snapshot.revision == request.snapshot.revision
                && document.snapshot.metadata.language_id == request.snapshot.metadata.language_id
                && state.phase == Phase::Running
                && state.supports_document(&document.uri, METHOD))
            .then_some((session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(session, _)| std::cmp::Reverse(session.order));
    if candidates.is_empty() {
        return Err(Failure::UnsupportedCapability);
    }
    match &request.stage {
        Stage::Prepare => {
            for (session, document) in &candidates {
                if request.is_cancelled() {
                    return Err(Failure::Cancelled);
                }
                let before = session.client.snapshot();
                if !before.supports_document(&document.uri, PREPARE_METHOD) {
                    let (session, _) = candidates[0];
                    return Ok(Response::Prepared(Prepared {
                        provider: identity(session),
                        range: request.fallback.clone(),
                        name: request.fallback_name(),
                    }));
                }
                let response = session
                    .client
                    .request_typed::<lsp_types::request::PrepareRenameRequest>(
                        lsp_types::TextDocumentPositionParams {
                            text_document: lsp_types::TextDocumentIdentifier {
                                uri: document
                                    .uri
                                    .parse()
                                    .map_err(|_| Failure::MalformedRequest)?,
                            },
                            position: request.position,
                        },
                        Some((document.uri.clone(), document.protocol_revision)),
                    )
                    .await?;
                validate(session, &before, request)?;
                let (range, name) = match response.value {
                    Some(lsp_types::PrepareRenameResponse::Range(range)) => (
                        range_to_bytes(&request.snapshot, range)
                            .map_err(|_| Failure::MalformedResponse)?,
                        request.fallback_name(),
                    ),
                    Some(lsp_types::PrepareRenameResponse::RangeWithPlaceholder {
                        range,
                        placeholder,
                    }) => (
                        range_to_bytes(&request.snapshot, range)
                            .map_err(|_| Failure::MalformedResponse)?,
                        placeholder,
                    ),
                    Some(lsp_types::PrepareRenameResponse::DefaultBehavior {
                        default_behavior: true,
                    }) => (request.fallback.clone(), request.fallback_name()),
                    Some(lsp_types::PrepareRenameResponse::DefaultBehavior {
                        default_behavior: false,
                    })
                    | None => continue,
                };
                return Ok(Response::Prepared(Prepared {
                    provider: identity(session),
                    range,
                    name,
                }));
            }
            Ok(Response::Unavailable)
        }
        Stage::Rename { name, provider } => {
            let Some((session, document)) = candidates
                .into_iter()
                .find(|(session, _)| identity(session) == *provider)
            else {
                return Err(Failure::StaleGeneration);
            };
            if request.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            let before = session.client.snapshot();
            let response = session
                .client
                .request_typed::<lsp_types::request::Rename>(
                    lsp_types::RenameParams {
                        text_document_position: lsp_types::TextDocumentPositionParams {
                            text_document: lsp_types::TextDocumentIdentifier {
                                uri: document
                                    .uri
                                    .parse()
                                    .map_err(|_| Failure::MalformedRequest)?,
                            },
                            position: request.position,
                        },
                        new_name: name.clone(),
                        work_done_progress_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await?;
            validate(session, &before, request)?;
            Ok(Response::Edits(Edits {
                provider: identity(session),
                edit: response.value.unwrap_or_default(),
                documents: protocol_documents(session),
                roots: session.roots.clone(),
            }))
        }
    }
}

pub(super) fn identity(session: &Session) -> ProviderIdentity {
    let state = session.client.snapshot();
    ProviderIdentity {
        owner: session.owner,
        generation: state.generation,
        capability_revision: state.capability_revision,
    }
}

fn validate(
    session: &Session,
    before: &super::SessionSnapshot,
    request: &Request,
) -> Result<(), Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let after = session.client.snapshot();
    if after.phase != Phase::Running
        || after.generation != before.generation
        || after.capability_revision != before.capability_revision
    {
        return Err(Failure::StaleGeneration);
    }
    Ok(())
}

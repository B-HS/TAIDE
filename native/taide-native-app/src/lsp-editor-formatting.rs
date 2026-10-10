use std::collections::HashMap;

use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_formatting::{Kind, Request, Response};
use crate::editor_symbols::ProviderIdentity;
use taide_native_editor::lsp::byte_to_position;

const DOCUMENT_METHOD: &str = "textDocument/formatting";
const RANGE_METHOD: &str = "textDocument/rangeFormatting";

pub(super) async fn request(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Response, Failure> {
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
    let selected = match &request.kind {
        Kind::Document => candidates
            .iter()
            .find(|(session, document)| {
                session
                    .client
                    .snapshot()
                    .supports_document(&document.uri, DOCUMENT_METHOD)
            })
            .map(|candidate| (*candidate, DOCUMENT_METHOD))
            .or_else(|| {
                candidates
                    .iter()
                    .find(|(session, document)| {
                        session
                            .client
                            .snapshot()
                            .supports_document(&document.uri, RANGE_METHOD)
                    })
                    .map(|candidate| (*candidate, RANGE_METHOD))
            }),
        Kind::Ranges(_) => candidates
            .iter()
            .find(|(session, document)| {
                session
                    .client
                    .snapshot()
                    .supports_document(&document.uri, RANGE_METHOD)
            })
            .map(|candidate| (*candidate, RANGE_METHOD)),
    };
    let Some(((session, document), method)) = selected else {
        return Err(Failure::UnsupportedCapability);
    };
    let before = session.client.snapshot();
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let uri = document
        .uri
        .parse()
        .map_err(|_| Failure::MalformedRequest)?;
    let edits = if method == DOCUMENT_METHOD {
        session
            .client
            .request_typed::<lsp_types::request::Formatting>(
                lsp_types::DocumentFormattingParams {
                    text_document: lsp_types::TextDocumentIdentifier { uri },
                    options: request.options.clone(),
                    work_done_progress_params: Default::default(),
                },
                Some((document.uri.clone(), document.protocol_revision)),
            )
            .await?
            .value
            .unwrap_or_default()
    } else {
        let ranges = match &request.kind {
            Kind::Ranges(ranges) => ranges.clone(),
            Kind::Document => vec![lsp_types::Range::new(
                lsp_types::Position::default(),
                byte_to_position(&request.snapshot, request.snapshot.rope.len_bytes())
                    .map_err(|_| Failure::MalformedRequest)?,
            )],
        };
        let mut groups = Vec::new();
        for range in ranges {
            let edits = range_edits(session, document, request, range).await?;
            groups.push((range, edits));
        }
        loop {
            let intersection = groups.iter().enumerate().find_map(|(left, (_, edits))| {
                groups
                    .iter()
                    .enumerate()
                    .skip(left + 1)
                    .find_map(|(right, (_, other))| {
                        edits
                            .iter()
                            .any(|edit| {
                                other.iter().any(|other| {
                                    edit.range.start <= other.range.end
                                        && other.range.start <= edit.range.end
                                })
                            })
                            .then_some((left, right))
                    })
            });
            let Some((left, right)) = intersection else {
                break;
            };
            let right = groups.remove(right);
            let left = groups.remove(left);
            let range =
                lsp_types::Range::new(left.0.start.min(right.0.start), left.0.end.max(right.0.end));
            let edits = range_edits(session, document, request, range).await?;
            groups.push((range, edits));
        }
        groups.into_iter().flat_map(|(_, edits)| edits).collect()
    };
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let after = session.client.snapshot();
    if after.phase != Phase::Running
        || before.generation != after.generation
        || before.capability_revision != after.capability_revision
    {
        return Err(Failure::StaleRevision);
    }
    Ok(Response {
        provider: ProviderIdentity {
            owner: session.owner,
            generation: after.generation,
            capability_revision: after.capability_revision,
        },
        edits,
    })
}

async fn range_edits(
    session: &Session,
    document: &super::Document,
    request: &Request,
    range: lsp_types::Range,
) -> Result<Vec<lsp_types::TextEdit>, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    Ok(session
        .client
        .request_typed::<lsp_types::request::RangeFormatting>(
            lsp_types::DocumentRangeFormattingParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: document
                        .uri
                        .parse()
                        .map_err(|_| Failure::MalformedRequest)?,
                },
                range,
                options: request.options.clone(),
                work_done_progress_params: Default::default(),
            },
            Some((document.uri.clone(), document.protocol_revision)),
        )
        .await?
        .value
        .unwrap_or_default())
}

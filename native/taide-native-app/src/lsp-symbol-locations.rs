use std::collections::HashMap;

use taide_native_editor::symbol_locations::{Kind, Mode, Target, normalize};

use super::{Failure, Phase, Session, SessionKey, lsp_types};
use crate::editor_locations::{Group, Request, Response};
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
            let mut client = session.client.clone();
            if client.snapshot().phase != Phase::Running {
                client.wait_for_phase(Phase::Running).await.ok()?;
            }
            let before = client.snapshot();
            if !before.supports_document(&document.uri, request.kind.method()) {
                return None;
            }
            let position = lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: document.uri.parse().ok()?,
                },
                position: request.position,
            };
            let revision = Some((document.uri.clone(), document.protocol_revision));
            let targets = if request.kind == Kind::References {
                let parameters = lsp_types::ReferenceParams {
                    text_document_position: position,
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                    context: lsp_types::ReferenceContext {
                        include_declaration: true,
                    },
                };
                let first = client
                    .request_typed::<lsp_types::request::References>(
                        parameters.clone(),
                        revision.clone(),
                    )
                    .await
                    .ok()?
                    .value
                    .unwrap_or_default();
                let locations =
                    if matches!(request.mode, Mode::GoTo | Mode::Aside) && first.len() == 2 {
                        let mut parameters = parameters;
                        parameters.context.include_declaration = false;
                        match client
                            .request_typed::<lsp_types::request::References>(parameters, revision)
                            .await
                        {
                            Ok(response)
                                if response
                                    .value
                                    .as_ref()
                                    .is_some_and(|locations| locations.len() == 1) =>
                            {
                                response.value.unwrap_or_default()
                            }
                            _ => first,
                        }
                    } else {
                        first
                    };
                locations
                    .into_iter()
                    .map(Target::from_location)
                    .collect::<Vec<_>>()
            } else {
                let parameters = lsp_types::GotoDefinitionParams {
                    text_document_position_params: position,
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                };
                let result = match request.kind {
                    Kind::Definition => client
                        .request_typed::<lsp_types::request::GotoDefinition>(parameters, revision)
                        .await
                        .map(|response| response.value),
                    Kind::Declaration => client
                        .request_typed::<lsp_types::request::GotoDeclaration>(parameters, revision)
                        .await
                        .map(|response| response.value),
                    Kind::TypeDefinition => client
                        .request_typed::<lsp_types::request::GotoTypeDefinition>(
                            parameters, revision,
                        )
                        .await
                        .map(|response| response.value),
                    Kind::Implementation => client
                        .request_typed::<lsp_types::request::GotoImplementation>(
                            parameters, revision,
                        )
                        .await
                        .map(|response| response.value),
                    Kind::References => return None,
                };
                normalize(result.ok()?)
            };
            let after = client.snapshot();
            if request.is_cancelled()
                || before.generation != after.generation
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
                targets: targets
                    .into_iter()
                    .filter(|target| {
                        target.is_valid()
                            && crate::editor_locations::file_path(&target.uri).is_some()
                    })
                    .collect(),
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

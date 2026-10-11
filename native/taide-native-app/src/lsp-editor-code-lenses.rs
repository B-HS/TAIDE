use std::collections::HashMap;

use super::{Failure, Phase, Reply, Session, SessionKey, lsp_types};
use crate::editor_code_lenses::{ExecuteRequest, Request, ResolveRequest, Response};
use crate::editor_symbols::ProviderIdentity;

#[derive(Clone)]
pub(super) enum Work {
    Provide(Request),
    Resolve(ResolveRequest),
    Execute(ExecuteRequest),
}

impl Work {
    pub(super) fn request(&self) -> &Request {
        match self {
            Self::Provide(request) => request,
            Self::Resolve(request) => &request.request,
            Self::Execute(request) => &request.request,
        }
    }

    pub(super) fn rejected(self, error: Failure) -> Reply {
        match self {
            Self::Provide(request) => Reply::CodeLenses {
                request,
                result: Err(error),
            },
            Self::Resolve(request) => Reply::CodeLensResolved {
                request,
                result: Err(error),
            },
            Self::Execute(request) => Reply::CodeLensExecuted {
                request,
                result: Err(super::failure(error)),
            },
        }
    }
}

fn identity(session: &Session) -> ProviderIdentity {
    let state = session.client.snapshot();
    ProviderIdentity {
        owner: session.owner,
        generation: state.generation,
        capability_revision: state.capability_revision,
    }
}

fn document<'a>(session: &'a Session, request: &Request) -> Result<&'a super::Document, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let document = session
        .documents
        .get(&request.snapshot.id)
        .ok_or(Failure::DocumentNotOpen)?;
    if document.snapshot.key != request.snapshot.key
        || document.snapshot.revision != request.snapshot.revision
        || document.snapshot.metadata.language_id != request.snapshot.metadata.language_id
    {
        return Err(Failure::StaleRevision);
    }
    Ok(document)
}

fn provider<'a>(
    sessions: &'a HashMap<SessionKey, Session>,
    request: &Request,
    provider: ProviderIdentity,
) -> Result<&'a Session, Failure> {
    sessions
        .iter()
        .find_map(|(key, session)| {
            (key.project == request.project
                && identity(session) == provider
                && session.client.snapshot().phase == Phase::Running)
                .then_some(session)
        })
        .ok_or(Failure::StaleGeneration)
}

async fn provide(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Response, Failure> {
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    let mut candidates = sessions
        .iter()
        .filter(|(key, _)| key.project == request.project)
        .filter_map(|(_, session)| {
            document(session, request)
                .ok()
                .map(|document| (session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(session, _)| std::cmp::Reverse(session.order));
    let groups = futures_util::future::join_all(candidates.into_iter().map(
        |(session, document)| async move {
            let before = identity(session);
            let state = session.client.snapshot();
            if !state.supports_document(&document.uri, "textDocument/codeLens") {
                return None;
            }
            let can_resolve = state.supports_document(&document.uri, "codeLens/resolve");
            let Ok(uri) = document.uri.parse() else {
                return None;
            };
            let result = session
                .client
                .request_typed::<lsp_types::request::CodeLensRequest>(
                    lsp_types::CodeLensParams {
                        text_document: lsp_types::TextDocumentIdentifier { uri },
                        work_done_progress_params: Default::default(),
                        partial_result_params: Default::default(),
                    },
                    Some((document.uri.clone(), document.protocol_revision)),
                )
                .await;
            if request.is_cancelled()
                || identity(session) != before
                || session.client.snapshot().phase != Phase::Running
            {
                return None;
            }
            result
                .ok()
                .map(|reply| (before, can_resolve, reply.value.unwrap_or_default()))
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

async fn resolve(
    sessions: &HashMap<SessionKey, Session>,
    request: &ResolveRequest,
) -> Result<lsp_types::CodeLens, Failure> {
    let session = provider(sessions, &request.request, request.provider)?;
    let document = document(session, &request.request)?;
    let reply = session
        .client
        .request_typed::<lsp_types::request::CodeLensResolve>(
            lsp_types::CodeLens {
                range: request.lens.range,
                command: None,
                data: request.lens.data.clone(),
            },
            Some((document.uri.clone(), document.protocol_revision)),
        )
        .await?;
    if request.request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    if identity(session) != request.provider || session.client.snapshot().phase != Phase::Running {
        return Err(Failure::StaleGeneration);
    }
    Ok(reply.value)
}

async fn execute(
    sessions: &HashMap<SessionKey, Session>,
    request: &ExecuteRequest,
    commands: &tokio::sync::mpsc::Sender<super::Command>,
) -> taide_model::error::AppResult<Option<crate::editor_code_actions::Navigation>> {
    let session = provider(sessions, &request.request, request.provider).map_err(super::failure)?;
    document(session, &request.request).map_err(super::failure)?;
    let current = super::bindings(commands, request.request.snapshot.id)
        .await
        .map_err(super::failure)?;
    let current = provider(&current, &request.request, request.provider).map_err(super::failure)?;
    document(current, &request.request).map_err(super::failure)?;
    let result = super::execute(&current.client, request.command.clone()).await;
    if request.request.is_cancelled() {
        return Err(super::failure(Failure::Cancelled));
    }
    if identity(session) != request.provider || session.client.snapshot().phase != Phase::Running {
        return Err(super::failure(Failure::StaleGeneration));
    }
    match result {
        Ok(()) => Ok(None),
        Err(Failure::UnsupportedCapability) => {
            super::editor_code_actions::client_navigation(request.command.clone())
                .map_err(super::failure)
        }
        Err(error) => Err(super::failure(error)),
    }
}

pub(super) async fn run(
    sessions: &HashMap<SessionKey, Session>,
    work: Work,
    commands: &tokio::sync::mpsc::Sender<super::Command>,
) -> Reply {
    match work {
        Work::Provide(request) => {
            let result = provide(sessions, &request).await;
            Reply::CodeLenses { request, result }
        }
        Work::Resolve(request) => {
            let result = resolve(sessions, &request).await;
            Reply::CodeLensResolved { request, result }
        }
        Work::Execute(request) => {
            let result = execute(sessions, &request, commands).await;
            Reply::CodeLensExecuted { request, result }
        }
    }
}

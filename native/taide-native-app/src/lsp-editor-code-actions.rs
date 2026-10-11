use std::collections::{HashMap, HashSet};

use super::{Failure, Phase, Session, SessionKey, SessionSnapshot, lsp_types, protocol_documents};
use crate::editor_code_actions::{Candidate, Chosen, Navigation, Request, Resolved};
use crate::editor_symbols::ProviderIdentity;
use taide_native_editor::lsp::{byte_to_position, position_to_byte};
use taide_native_ui::editor_code_actions::{Command, kind_contains};
use tokio::task::JoinSet;

const METHOD: &str = "textDocument/codeAction";
const RESOLVE_METHOD: &str = "codeAction/resolve";
const FALLBACK_KINDS: [&str; 3] = ["quickfix", "refactor", "source"];

pub(super) fn supports(command: Command, options: &[lsp_types::CodeActionOptions]) -> bool {
    options.iter().any(|option| {
        option
            .code_action_kinds
            .as_ref()
            .filter(|kinds| !kinds.is_empty())
            .map_or_else(
                || FALLBACK_KINDS.iter().any(|kind| command.intersects(kind)),
                |kinds| kinds.iter().any(|kind| command.intersects(kind.as_str())),
            )
    })
}

pub(super) fn identity(session: &Session) -> ProviderIdentity {
    let state = session.client.snapshot();
    ProviderIdentity {
        owner: session.owner,
        generation: state.generation,
        capability_revision: state.capability_revision,
    }
}

pub(super) fn document<'a>(
    session: &'a Session,
    request: &Request,
) -> Result<&'a super::Document, Failure> {
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
    if request.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    Ok(document)
}

fn client_navigation(command: lsp_types::Command) -> Result<Option<Navigation>, Failure> {
    use taide_native_editor::symbol_locations::{Mode, Target};
    let arguments = command.arguments.unwrap_or_default();
    let target = |value: &serde_json::Value| {
        let target = if value.get("targetUri").is_some() {
            serde_json::from_value::<lsp_types::LocationLink>(value.clone()).map(Target::from_link)
        } else {
            serde_json::from_value::<lsp_types::Location>(value.clone()).map(Target::from_location)
        }
        .map_err(|_| Failure::MalformedRequest)?;
        target
            .is_valid()
            .then_some(target)
            .ok_or(Failure::MalformedRequest)
    };
    match command.command.as_str() {
        "editor.action.showReferences" | "rust-analyzer.showReferences" => {
            let uri = arguments
                .first()
                .and_then(serde_json::Value::as_str)
                .ok_or(Failure::MalformedRequest)?
                .parse()
                .map_err(|_| Failure::MalformedRequest)?;
            let position = serde_json::from_value::<lsp_types::Position>(
                arguments.get(1).cloned().ok_or(Failure::MalformedRequest)?,
            )
            .map_err(|_| Failure::MalformedRequest)?;
            let targets = arguments
                .get(2)
                .and_then(serde_json::Value::as_array)
                .ok_or(Failure::MalformedRequest)?
                .iter()
                .map(target)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(Navigation {
                uri,
                position,
                targets,
                mode: Mode::Peek,
            }))
        }
        "rust-analyzer.gotoLocation" => {
            let target = target(arguments.first().ok_or(Failure::MalformedRequest)?)?;
            Ok(Some(Navigation {
                uri: target.uri.clone(),
                position: target.selection.start,
                targets: vec![target],
                mode: Mode::GoTo,
            }))
        }
        _ => Err(Failure::UnsupportedCapability),
    }
}

pub(super) async fn apply(
    services: &std::sync::Arc<taide_runtime::AppServices>,
    session: &Session,
    request: &Request,
    resolved: Resolved,
    commands: &tokio::sync::mpsc::Sender<super::Command>,
    replies: &tokio::sync::mpsc::Sender<super::Reply>,
    repaint: &std::sync::Arc<dyn Fn() + Send + Sync>,
) -> taide_model::error::AppResult<Option<Navigation>> {
    let before = session.client.snapshot();
    validate(session, &before, request).map_err(super::failure)?;
    if let Some(edit) = resolved.edit {
        crate::lsp_workspace_worker::apply(
            services,
            edit,
            resolved.documents,
            Some(resolved.roots),
            replies,
            repaint,
        )
        .await?;
    }
    validate(session, &before, request).map_err(super::failure)?;
    let Some(command) = resolved.command else {
        return Ok(None);
    };
    let bound = super::bindings(commands, request.snapshot.id)
        .await
        .map_err(super::failure)?;
    let current = bound
        .iter()
        .find_map(|(key, current)| {
            (key.project == request.project && identity(current) == resolved.provider)
                .then_some(current)
        })
        .ok_or_else(|| super::failure(Failure::StaleGeneration))?;
    validate(current, &before, request).map_err(super::failure)?;
    match super::execute(&current.client, command.clone()).await {
        Ok(()) => {
            validate(session, &before, request).map_err(super::failure)?;
            Ok(None)
        }
        Err(Failure::UnsupportedCapability) => client_navigation(command).map_err(super::failure),
        Err(error) => Err(super::failure(error)),
    }
}

fn validate(session: &Session, before: &SessionSnapshot, request: &Request) -> Result<(), Failure> {
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

fn intersects(left: lsp_types::Range, right: lsp_types::Range) -> bool {
    let position = |position: lsp_types::Position| (position.line, position.character);
    position(left.start) <= position(right.end) && position(right.start) <= position(left.end)
}

async fn query_one(
    session: Session,
    request: Request,
    range: lsp_types::Range,
    only: Option<&'static str>,
) -> Result<Vec<Candidate>, Failure> {
    let document = document(&session, &request)?;
    let before = session.client.snapshot();
    if !before.supports_document(&document.uri, METHOD)
        || !before
            .document_code_action_options
            .get(&document.uri)
            .is_some_and(|options| {
                supports(request.invocation.command, options)
                    && only.is_none_or(|requested| {
                        options.iter().any(|option| {
                            option
                                .code_action_kinds
                                .as_ref()
                                .filter(|kinds| !kinds.is_empty())
                                .is_none_or(|kinds| {
                                    kinds.iter().any(|kind| {
                                        kind_contains(requested, kind.as_str())
                                            || kind_contains(kind.as_str(), requested)
                                    })
                                })
                        })
                    })
            })
    {
        return Err(Failure::UnsupportedCapability);
    }
    let provider = identity(&session);
    let response = session
        .client
        .request_typed::<lsp_types::request::CodeActionRequest>(
            lsp_types::CodeActionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: document
                        .uri
                        .parse()
                        .map_err(|_| Failure::MalformedRequest)?,
                },
                range,
                context: lsp_types::CodeActionContext {
                    diagnostics: session
                        .raw_diagnostics
                        .get(&document.uri)
                        .iter()
                        .filter(|diagnostic| intersects(diagnostic.range, range))
                        .cloned()
                        .collect(),
                    only: only.map(|kind| vec![lsp_types::CodeActionKind::new(kind)]),
                    trigger_kind: Some(if request.invocation.automatic {
                        lsp_types::CodeActionTriggerKind::AUTOMATIC
                    } else {
                        lsp_types::CodeActionTriggerKind::INVOKED
                    }),
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            },
            Some((document.uri.clone(), document.protocol_revision)),
        )
        .await?;
    validate(&session, &before, &request)?;
    Ok(response
        .value
        .unwrap_or_default()
        .into_iter()
        .map(|raw| Candidate { provider, raw })
        .collect())
}

async fn query_at(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
    range: lsp_types::Range,
    only: Option<&'static str>,
) -> Result<Vec<Candidate>, Failure> {
    let mut pending = JoinSet::new();
    for (key, session) in sessions {
        if key.project != request.project || document(session, request).is_err() {
            continue;
        }
        let session = session.clone();
        let request = request.clone();
        pending.spawn(async move {
            (
                session.order,
                query_one(session, request, range, only).await,
            )
        });
    }
    let mut responses = Vec::new();
    while let Some(response) = pending.join_next().await {
        if request.is_cancelled() {
            return Err(Failure::Cancelled);
        }
        if let Ok((order, Ok(candidates))) = response {
            responses.push((order, candidates));
        }
    }
    responses.sort_by_key(|(order, _)| std::cmp::Reverse(*order));
    Ok(responses
        .into_iter()
        .flat_map(|(_, candidates)| candidates)
        .collect())
}

pub(super) async fn query(
    sessions: &HashMap<SessionKey, Session>,
    request: &Request,
) -> Result<Vec<Candidate>, Failure> {
    let actions = query_at(
        sessions,
        request,
        request.range,
        request.invocation.command.kind(),
    )
    .await?;
    let valid_fix = |candidate: &Candidate| {
        let action = candidate.presentation();
        action.disabled.is_none()
            && request.invocation.command.accepts(&action)
            && action
                .kind
                .as_deref()
                .is_some_and(|kind| kind_contains("quickfix", kind))
    };
    if request.invocation.automatic
        || !matches!(
            request.invocation.command,
            Command::QuickFix | Command::AutoFix
        )
        || actions.iter().any(valid_fix)
    {
        return Ok(actions);
    }
    let primary = request
        .selection
        .selections
        .get(request.selection.primary)
        .ok_or(Failure::MalformedRequest)?;
    let caret =
        byte_to_position(&request.snapshot, primary.head).map_err(|_| Failure::MalformedRequest)?;
    let mut positions = sessions
        .iter()
        .filter(|(key, _)| key.project == request.project)
        .filter_map(|(_, session)| {
            document(session, request)
                .ok()
                .map(|document| (session, document))
        })
        .flat_map(|(session, document)| session.raw_diagnostics.get(&document.uri))
        .filter(|diagnostic| {
            diagnostic.range.start.line == caret.line || diagnostic.range.end.line == caret.line
        })
        .map(|diagnostic| diagnostic.range.end)
        .filter(|position| {
            position_to_byte(&request.snapshot, *position)
                .and_then(|byte| byte_to_position(&request.snapshot, byte))
                .is_ok_and(|actual| actual == *position)
        })
        .collect::<Vec<_>>();
    positions.sort_by_key(|position| {
        (
            position.line.abs_diff(caret.line),
            position.character.abs_diff(caret.character),
            position.line,
            position.character,
        )
    });
    positions.dedup();
    let mut seen = actions
        .iter()
        .filter(|candidate| candidate.presentation().disabled.is_none())
        .map(|candidate| {
            let action = candidate.presentation();
            (candidate.provider, action.title, action.kind)
        })
        .collect::<HashSet<_>>();
    let mut nearby = Vec::new();
    for position in positions {
        let range = lsp_types::Range::new(position, position);
        if range == request.range {
            continue;
        }
        for candidate in query_at(sessions, request, range, Some("quickfix"))
            .await?
            .into_iter()
            .filter(valid_fix)
        {
            let action = candidate.presentation();
            if seen.insert((candidate.provider, action.title, action.kind)) {
                nearby.push(candidate);
            }
        }
    }
    nearby.extend(actions);
    Ok(nearby)
}

pub(super) async fn resolve(
    sessions: &HashMap<SessionKey, Session>,
    chosen: &Chosen,
) -> Result<Resolved, Failure> {
    let request = &chosen.request;
    let session = sessions
        .iter()
        .find_map(|(key, session)| {
            (key.project == request.project && identity(session) == chosen.candidate.provider)
                .then_some(session)
        })
        .ok_or(Failure::StaleGeneration)?;
    let document = document(session, request)?;
    let before = session.client.snapshot();
    if !before.supports_document(&document.uri, METHOD) {
        return Err(Failure::UnsupportedCapability);
    }
    let (edit, command) = match &chosen.candidate.raw {
        lsp_types::CodeActionOrCommand::Command(command) => (None, Some(command.clone())),
        lsp_types::CodeActionOrCommand::CodeAction(original) => {
            if original.disabled.is_some() {
                return Err(Failure::MalformedRequest);
            }
            let mut effective = original.clone();
            if original.edit.is_none() && before.supports_document(&document.uri, RESOLVE_METHOD) {
                match session
                    .client
                    .request_typed::<lsp_types::request::CodeActionResolveRequest>(
                        original.clone(),
                        Some((document.uri.clone(), document.protocol_revision)),
                    )
                    .await
                {
                    Ok(response) => effective = response.value,
                    Err(
                        Failure::ServerError(_)
                        | Failure::TimedOut
                        | Failure::MalformedResponse
                        | Failure::UnsupportedCapability,
                    ) => (),
                    Err(error) => return Err(error),
                }
            }
            if effective.disabled.is_some() {
                return Err(Failure::MalformedResponse);
            }
            (
                effective.edit,
                effective.command.or_else(|| original.command.clone()),
            )
        }
    };
    validate(session, &before, request)?;
    Ok(Resolved {
        provider: chosen.candidate.provider,
        edit,
        command,
        documents: protocol_documents(session),
        roots: session.roots.clone(),
    })
}

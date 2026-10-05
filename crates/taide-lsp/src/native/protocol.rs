use lsp_types::{
    ApplyWorkspaceEditParams, ApplyWorkspaceEditResponse, CancelParams, ConfigurationParams,
    LogMessageParams, MessageActionItem, NumberOrString, PublishDiagnosticsParams,
    RegistrationParams, ShowMessageParams, ShowMessageRequestParams, UnregistrationParams,
    WorkDoneProgress, WorkDoneProgressCreateParams, WorkspaceFolder,
};
use serde_json::{json, Value};

use super::Failure;

pub use lsp_types;

const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const REQUEST_FAILED: i32 = -32803;
const REQUEST_CANCELLED: i32 = -32800;
const MAX_PERCENTAGE: u64 = 100;
const MAX_SEVERITY: u64 = 4;
const MAX_DIAGNOSTIC_TAG: u64 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshKind {
    SemanticTokens,
    CodeLens,
    InlayHint,
    Diagnostics,
    FoldingRange,
    InlineValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerRequestKind {
    Register(RegistrationParams),
    Unregister(UnregistrationParams),
    Configuration(ConfigurationParams),
    ApplyEdit(ApplyWorkspaceEditParams),
    CreateProgress(WorkDoneProgressCreateParams),
    Refresh(RefreshKind),
    WorkspaceFolders,
    ShowMessage(ShowMessageRequestParams),
}

#[derive(Debug, PartialEq)]
pub enum ServerNotification {
    Diagnostics(PublishDiagnosticsParams),
    Log(LogMessageParams),
    ShowMessage(ShowMessageParams),
    Progress {
        token: NumberOrString,
        value: ProgressValue,
    },
    Cancel(CancelParams),
    Extension,
}

#[derive(Debug, PartialEq)]
pub enum ProgressValue {
    WorkDone(WorkDoneProgress),
    Partial(Value),
}

#[derive(Debug, PartialEq)]
pub enum ServerReply {
    Configuration(Vec<Value>),
    ApplyEdit(ApplyWorkspaceEditResponse),
    Acknowledged,
    WorkspaceFolders(Option<Vec<WorkspaceFolder>>),
    ShowMessage(Option<MessageActionItem>),
    Rejected(Rejection),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    InvalidRequest,
    Unsupported,
    InvalidParams,
    Unavailable,
    Cancelled,
}

impl Rejection {
    pub(crate) fn response(self, id: &NumberOrString) -> Value {
        let (code, message) = match self {
            Self::InvalidRequest => (INVALID_REQUEST, "invalid request"),
            Self::Unsupported => (METHOD_NOT_FOUND, "unsupported request"),
            Self::InvalidParams => (INVALID_PARAMS, "invalid request parameters"),
            Self::Unavailable => (REQUEST_FAILED, "request unavailable"),
            Self::Cancelled => (REQUEST_CANCELLED, "request cancelled"),
        };
        json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
    }
}

impl ServerReply {
    pub(crate) fn response(
        self,
        id: &NumberOrString,
        request: &ServerRequestKind,
    ) -> Result<Value, Failure> {
        let result = match (self, request) {
            (Self::Rejected(rejection), _) => return Ok(rejection.response(id)),
            (Self::Configuration(values), ServerRequestKind::Configuration(params))
                if values.len() == params.items.len() =>
            {
                json!(values)
            }
            (Self::ApplyEdit(mut reply), ServerRequestKind::ApplyEdit(_)) => {
                if reply
                    .failed_change
                    .is_some_and(|index| index > i32::MAX as u32)
                    || (reply.applied
                        && (reply.failed_change.is_some() || reply.failure_reason.is_some()))
                {
                    return Err(Failure::MalformedResponse);
                }
                if !reply.applied {
                    reply.failure_reason = Some("edit rejected".into());
                }
                json!(reply)
            }
            (
                Self::Acknowledged,
                ServerRequestKind::CreateProgress(_) | ServerRequestKind::Refresh(_),
            ) => Value::Null,
            (Self::WorkspaceFolders(folders), ServerRequestKind::WorkspaceFolders) => {
                json!(folders)
            }
            (Self::ShowMessage(item), ServerRequestKind::ShowMessage(params))
                if item.as_ref().is_none_or(|item| {
                    params
                        .actions
                        .as_ref()
                        .is_some_and(|actions| actions.contains(item))
                }) =>
            {
                json!(item)
            }
            _ => return Err(Failure::MalformedResponse),
        };
        Ok(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}

#[derive(Debug, PartialEq)]
pub enum IncomingMessage {
    Request {
        id: NumberOrString,
        request: Result<ServerRequestKind, Rejection>,
    },
    Notification(ServerNotification),
    Response,
}

macro_rules! params {
    ($value:expr, $type:ty, $variant:path) => {
        serde_json::from_value::<$type>($value.clone())
            .map($variant)
            .map_err(|_| Rejection::InvalidParams)
    };
}

fn has_nonnull_fields(value: &Value, fields: &[&str]) -> bool {
    fields
        .iter()
        .all(|field| value.get(field).is_none_or(|value| !value.is_null()))
}

fn is_range(value: &Value) -> bool {
    let position = |name| {
        let value = value.get(name)?;
        let line = value.get("line")?.as_u64()?;
        let character = value.get("character")?.as_u64()?;
        (line <= i32::MAX as u64 && character <= i32::MAX as u64).then_some((line, character))
    };
    matches!((position("start"), position("end")), (Some(start), Some(end)) if start <= end)
}

fn is_text_edits(value: &Value) -> bool {
    value.as_array().is_some_and(|edits| {
        edits
            .iter()
            .all(|edit| edit.get("range").is_some_and(is_range))
    })
}

fn decode_request(method: &str, params: &Value) -> Result<ServerRequestKind, Rejection> {
    if params.is_array() {
        return Err(Rejection::InvalidParams);
    }
    match method {
        "client/registerCapability" => {
            params!(params, RegistrationParams, ServerRequestKind::Register)
        }
        "client/unregisterCapability" => {
            params!(params, UnregistrationParams, ServerRequestKind::Unregister)
        }
        "workspace/configuration" => {
            if !params
                .get("items")
                .and_then(Value::as_array)
                .is_some_and(|items| {
                    items.iter().all(|item| {
                        item.is_object() && has_nonnull_fields(item, &["scopeUri", "section"])
                    })
                })
            {
                return Err(Rejection::InvalidParams);
            }
            params!(
                params,
                ConfigurationParams,
                ServerRequestKind::Configuration
            )
        }
        "workspace/applyEdit" => {
            let edit = params.get("edit").ok_or(Rejection::InvalidParams)?;
            if !has_nonnull_fields(params, &["label"])
                || !has_nonnull_fields(edit, &["changes", "documentChanges", "changeAnnotations"])
                || edit.get("changes").is_some_and(|changes| {
                    !changes
                        .as_object()
                        .is_some_and(|changes| changes.values().all(is_text_edits))
                })
                || edit.get("documentChanges").is_some_and(|changes| {
                    !changes.as_array().is_some_and(|changes| {
                        changes
                            .iter()
                            .all(|change| change.get("edits").is_none_or(is_text_edits))
                    })
                })
            {
                return Err(Rejection::InvalidParams);
            }
            params!(
                params,
                ApplyWorkspaceEditParams,
                ServerRequestKind::ApplyEdit
            )
        }
        "window/workDoneProgress/create" => params!(
            params,
            WorkDoneProgressCreateParams,
            ServerRequestKind::CreateProgress
        ),
        "window/showMessageRequest" => {
            if !is_message(params) || !has_nonnull_fields(params, &["actions"]) {
                return Err(Rejection::InvalidParams);
            }
            params!(
                params,
                ShowMessageRequestParams,
                ServerRequestKind::ShowMessage
            )
        }
        "workspace/workspaceFolders" => {
            if !params.is_null() {
                return Err(Rejection::InvalidParams);
            }
            Ok(ServerRequestKind::WorkspaceFolders)
        }
        "workspace/semanticTokens/refresh"
        | "workspace/codeLens/refresh"
        | "workspace/inlayHint/refresh"
        | "workspace/diagnostic/refresh"
        | "workspace/foldingRange/refresh"
        | "workspace/inlineValue/refresh" => {
            if !params.is_null() {
                return Err(Rejection::InvalidParams);
            }
            let kind = match method {
                "workspace/semanticTokens/refresh" => RefreshKind::SemanticTokens,
                "workspace/codeLens/refresh" => RefreshKind::CodeLens,
                "workspace/inlayHint/refresh" => RefreshKind::InlayHint,
                "workspace/diagnostic/refresh" => RefreshKind::Diagnostics,
                "workspace/foldingRange/refresh" => RefreshKind::FoldingRange,
                _ => RefreshKind::InlineValue,
            };
            Ok(ServerRequestKind::Refresh(kind))
        }
        _ => Err(Rejection::Unsupported),
    }
}

fn decode_notification(method: &str, params: &Value) -> Result<ServerNotification, Rejection> {
    match method {
        "textDocument/publishDiagnostics" => {
            if !has_nonnull_fields(params, &["version"])
                || !params
                    .get("diagnostics")
                    .and_then(Value::as_array)
                    .is_some_and(|diagnostics| {
                        diagnostics.iter().all(|diagnostic| {
                            diagnostic.get("range").is_some_and(is_range)
                                && has_nonnull_fields(
                                    diagnostic,
                                    &[
                                        "severity",
                                        "code",
                                        "codeDescription",
                                        "source",
                                        "tags",
                                        "relatedInformation",
                                    ],
                                )
                                && diagnostic.get("severity").is_none_or(|value| {
                                    value
                                        .as_u64()
                                        .is_some_and(|value| (1..=MAX_SEVERITY).contains(&value))
                                })
                                && diagnostic.get("tags").is_none_or(|value| {
                                    value.as_array().is_some_and(|tags| {
                                        tags.iter().all(|tag| {
                                            tag.as_u64().is_some_and(|tag| {
                                                (1..=MAX_DIAGNOSTIC_TAG).contains(&tag)
                                            })
                                        })
                                    })
                                })
                                && diagnostic.get("relatedInformation").is_none_or(|value| {
                                    value.as_array().is_some_and(|related| {
                                        related.iter().all(|related| {
                                            related
                                                .get("location")
                                                .and_then(|location| location.get("range"))
                                                .is_some_and(is_range)
                                        })
                                    })
                                })
                        })
                    })
            {
                return Err(Rejection::InvalidParams);
            }
            params!(
                params,
                PublishDiagnosticsParams,
                ServerNotification::Diagnostics
            )
        }
        "window/logMessage" => {
            if !is_message(params) {
                return Err(Rejection::InvalidParams);
            }
            params!(params, LogMessageParams, ServerNotification::Log)
        }
        "window/showMessage" => {
            if !is_message(params) {
                return Err(Rejection::InvalidParams);
            }
            params!(params, ShowMessageParams, ServerNotification::ShowMessage)
        }
        "$/cancelRequest" => params!(params, CancelParams, ServerNotification::Cancel),
        "$/progress" => {
            let token = serde_json::from_value::<NumberOrString>(
                params.get("token").ok_or(Rejection::InvalidParams)?.clone(),
            )
            .map_err(|_| Rejection::InvalidParams)?;
            let value = params.get("value").ok_or(Rejection::InvalidParams)?;
            let value = ProgressValue::Partial(value.clone());
            Ok(ServerNotification::Progress { token, value })
        }
        _ => Ok(ServerNotification::Extension),
    }
}

pub fn decode_work_done(value: &Value) -> Result<WorkDoneProgress, Failure> {
    if !value.is_object()
        || !has_nonnull_fields(value, &["message", "cancellable", "percentage"])
        || value
            .get("percentage")
            .is_some_and(|value| !value.as_u64().is_some_and(|value| value <= MAX_PERCENTAGE))
    {
        return Err(Failure::MalformedResponse);
    }
    serde_json::from_value(value.clone()).map_err(|_| Failure::MalformedResponse)
}

fn is_message(params: &Value) -> bool {
    params.is_object()
        && params
            .get("type")
            .and_then(Value::as_u64)
            .is_some_and(|kind| (1..=MAX_SEVERITY).contains(&kind))
}

pub fn decode(message: &Value) -> Result<IncomingMessage, Failure> {
    if !message.is_object() || message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(Failure::MalformedResponse);
    }
    let Some(method) = message.get("method") else {
        let id = message.get("id").ok_or(Failure::MalformedResponse)?;
        if !id.is_null() && serde_json::from_value::<NumberOrString>(id.clone()).is_err() {
            return Err(Failure::MalformedResponse);
        }
        if message.get("params").is_some()
            || message.get("result").is_some() == message.get("error").is_some()
        {
            return Err(Failure::MalformedResponse);
        }
        if let Some(error) = message.get("error") {
            if error
                .get("code")
                .and_then(Value::as_i64)
                .and_then(|code| i32::try_from(code).ok())
                .is_none()
                || error.get("message").and_then(Value::as_str).is_none()
            {
                return Err(Failure::MalformedResponse);
            }
        }
        return Ok(IncomingMessage::Response);
    };
    let method = method
        .as_str()
        .filter(|method| !method.is_empty())
        .ok_or(Failure::MalformedResponse)?;
    let params = message.get("params").unwrap_or(&Value::Null);
    let is_valid_envelope = message.get("result").is_none() && message.get("error").is_none();
    let is_valid_params =
        message.get("params").is_none() || params.is_object() || params.is_array();
    if let Some(id) = message.get("id") {
        let id = serde_json::from_value::<NumberOrString>(id.clone())
            .map_err(|_| Failure::MalformedResponse)?;
        let request = if !is_valid_envelope {
            Err(Rejection::InvalidRequest)
        } else if !is_valid_params {
            Err(Rejection::InvalidParams)
        } else {
            decode_request(method, params)
        };
        return Ok(IncomingMessage::Request { id, request });
    }
    if !is_valid_envelope || !is_valid_params {
        return Err(Failure::MalformedResponse);
    }
    decode_notification(method, params)
        .map(IncomingMessage::Notification)
        .map_err(|_| Failure::MalformedResponse)
}

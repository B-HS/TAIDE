use lsp_types::request::{self, Request};
use lsp_types::Uri;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Failure;

const MAX_SCHEMA_DEPTH: usize = 128;
const SEMANTIC_TOKEN_FIELDS: usize = 5;
const MAX_UINT: u64 = i32::MAX as u64;

#[cfg(test)]
mod folding_range_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn folding_range는_표준과_사용자_종류를_줄_검증과_함께_보존한다() {
        let ranges = json!([
            {"startLine":0,"endLine":3,"kind":"imports"},
            {"startLine":4,"endLine":7,"startCharacter":2,"endCharacter":9,"kind":"custom","collapsedText":"title"},
            {"startLine":8,"endLine":9}
        ]);
        let reply = TypedReply::<request::FoldingRangeRequest>::decode(ranges.clone());
        assert!(reply.is_ok());
        let reply = reply.unwrap();
        assert_eq!(serde_json::to_value(reply.value).unwrap(), ranges);
        assert!(TypedReply::<request::FoldingRangeRequest>::decode(Value::Null).is_ok());
        for invalid in [
            json!([{"startLine":5,"endLine":2,"kind":"custom"}]),
            json!([{"startLine":0,"endLine":-1}]),
            json!([{"startLine":0,"endLine":1,"kind":3}]),
        ] {
            assert!(TypedReply::<request::FoldingRangeRequest>::decode(invalid).is_err());
        }
    }
}

pub trait FeatureRequest: Request {
    type Reply: serde::de::DeserializeOwned + Serialize + Send + Sync + 'static;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldingRangeReply {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(flatten)]
    range: lsp_types::FoldingRange,
}

impl FoldingRangeReply {
    pub fn range(&self) -> &lsp_types::FoldingRange {
        &self.range
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SemanticTokensDeltaResult {
    Tokens(lsp_types::SemanticTokens),
    Delta(SemanticTokensDelta),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokensDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_id: Option<String>,
    pub edits: Vec<SemanticTokensEdit>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticTokensEdit {
    pub start: u32,
    pub delete_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<u32>>,
}

pub struct TypedReply<R: FeatureRequest> {
    pub value: R::Reply,
    pub raw: Value,
}

impl<R: FeatureRequest> TypedReply<R> {
    pub fn decode(raw: Value) -> Result<Self, Failure> {
        let value = serde_json::from_value::<R::Reply>(raw.clone())
            .map_err(|_| Failure::MalformedResponse)?;
        let projection = serde_json::to_value(&value).map_err(|_| Failure::MalformedResponse)?;
        if R::METHOD != "workspace/executeCommand" {
            validate_projection(R::METHOD, &projection).map_err(|_| Failure::MalformedResponse)?;
        }
        if R::METHOD == "textDocument/completion" {
            validate_completion_defaults(&raw)?;
        }
        Ok(Self { value, raw })
    }
}

fn request<R: Request>(params: &Value) -> Result<(), Failure> {
    if !params.is_object() {
        return Err(Failure::MalformedRequest);
    }
    let value = serde_json::from_value::<R::Params>(params.clone())
        .map_err(|_| Failure::MalformedRequest)?;
    let projection = serde_json::to_value(value).map_err(|_| Failure::MalformedRequest)?;
    validate_projection(R::METHOD, &projection).map_err(|_| Failure::MalformedRequest)
}

fn response<R: FeatureRequest>(result: &Value) -> Result<(), Failure> {
    TypedReply::<R>::decode(result.clone()).map(|_| ())
}

macro_rules! feature_reply_type {
    ($request:ty) => {
        <$request as Request>::Result
    };
    ($request:ty, $reply:ty) => {
        $reply
    };
}

macro_rules! feature_contracts {
    ($($method:literal => $request:ty $(=> $reply:ty)?),+ $(,)?) => {
        $(impl FeatureRequest for $request {
            type Reply = feature_reply_type!($request $(, $reply)?);
        })+
        pub const METHODS: &[&str] = &[$($method),+];

        pub fn validate_request(method: &str, params: &Value) -> Result<(), Failure> {
            match method {
                $($method => request::<$request>(params),)+
                _ if params.is_object() || params.is_array() => Ok(()),
                _ => Err(Failure::MalformedRequest),
            }
        }

        pub fn validate_response(method: &str, result: &Value) -> Result<(), Failure> {
            match method {
                $($method => response::<$request>(result),)+
                _ => Ok(()),
            }
        }
    };
}

feature_contracts! {
    "textDocument/hover" => request::HoverRequest,
    "textDocument/definition" => request::GotoDefinition,
    "textDocument/references" => request::References,
    "textDocument/rename" => request::Rename,
    "textDocument/formatting" => request::Formatting,
    "textDocument/rangeFormatting" => request::RangeFormatting,
    "textDocument/inlayHint" => request::InlayHintRequest,
    "textDocument/documentSymbol" => request::DocumentSymbolRequest,
    "textDocument/documentHighlight" => request::DocumentHighlightRequest,
    "textDocument/selectionRange" => request::SelectionRangeRequest,
    "textDocument/codeAction" => request::CodeActionRequest,
    "textDocument/foldingRange" => request::FoldingRangeRequest => Option<Vec<FoldingRangeReply>>,
    "textDocument/implementation" => request::GotoImplementation,
    "textDocument/typeDefinition" => request::GotoTypeDefinition,
    "textDocument/declaration" => request::GotoDeclaration,
    "workspace/symbol" => request::WorkspaceSymbolRequest,
    "textDocument/completion" => request::Completion,
    "textDocument/onTypeFormatting" => request::OnTypeFormatting,
    "textDocument/signatureHelp" => request::SignatureHelpRequest,
    "textDocument/diagnostic" => request::DocumentDiagnosticRequest,
    "textDocument/codeLens" => request::CodeLensRequest,
    "workspace/executeCommand" => request::ExecuteCommand,
    "textDocument/prepareRename" => request::PrepareRenameRequest,
    "codeAction/resolve" => request::CodeActionResolveRequest,
    "codeLens/resolve" => request::CodeLensResolve,
    "textDocument/semanticTokens/full" => request::SemanticTokensFullRequest,
    "textDocument/semanticTokens/full/delta" => request::SemanticTokensFullDeltaRequest => Option<SemanticTokensDeltaResult>,
}

fn position(value: &Value) -> Result<(u64, u64), Failure> {
    let line = value
        .get("line")
        .and_then(Value::as_u64)
        .filter(|line| *line <= MAX_UINT)
        .ok_or(Failure::MalformedResponse)?;
    let character = value
        .get("character")
        .and_then(Value::as_u64)
        .filter(|character| *character <= MAX_UINT)
        .ok_or(Failure::MalformedResponse)?;
    Ok((line, character))
}

fn validate_schema(value: &Value, depth: usize) -> Result<(), Failure> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(Failure::MalformedResponse);
    }
    match value {
        Value::Array(values) => {
            for value in values {
                validate_schema(value, depth + 1)?;
            }
        }
        Value::Object(fields) => {
            if fields.contains_key("tabSize") && fields.contains_key("insertSpaces") {
                if !fields["tabSize"]
                    .as_u64()
                    .is_some_and(|value| value <= MAX_UINT)
                {
                    return Err(Failure::MalformedResponse);
                }
                return Ok(());
            }
            if fields.contains_key("line") && fields.contains_key("character") {
                position(value)?;
            }
            if let (Some(start), Some(end)) = (fields.get("start"), fields.get("end")) {
                if start.is_object() && end.is_object() && position(start)? > position(end)? {
                    return Err(Failure::MalformedResponse);
                }
            }
            if let (Some(start), Some(end)) = (fields.get("startLine"), fields.get("endLine")) {
                let start = start
                    .as_u64()
                    .filter(|value| *value <= MAX_UINT)
                    .ok_or(Failure::MalformedResponse)?;
                let end = end
                    .as_u64()
                    .filter(|value| *value <= MAX_UINT)
                    .ok_or(Failure::MalformedResponse)?;
                if start > end {
                    return Err(Failure::MalformedResponse);
                }
                for key in ["startCharacter", "endCharacter"] {
                    if fields
                        .get(key)
                        .is_some_and(|value| !value.as_u64().is_some_and(|value| value <= MAX_UINT))
                    {
                        return Err(Failure::MalformedResponse);
                    }
                }
            }
            if let Some(changes) = fields.get("changes").and_then(Value::as_object) {
                for uri in changes.keys() {
                    if uri
                        .parse::<Uri>()
                        .map_err(|_| Failure::MalformedResponse)?
                        .scheme()
                        .is_none()
                    {
                        return Err(Failure::MalformedResponse);
                    }
                }
            }
            for (key, value) in fields {
                if matches!(key.as_str(), "data" | "arguments" | "experimental") {
                    continue;
                }
                if matches!(
                    key.as_str(),
                    "uri" | "targetUri" | "oldUri" | "newUri" | "scopeUri" | "href"
                ) {
                    if let Some(value) = value.as_str() {
                        let uri = value
                            .parse::<Uri>()
                            .map_err(|_| Failure::MalformedResponse)?;
                        if uri.scheme().is_none() {
                            return Err(Failure::MalformedResponse);
                        }
                    }
                }
                validate_schema(value, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_projection(method: &str, value: &Value) -> Result<(), Failure> {
    validate_schema(value, 0)?;
    if matches!(
        method,
        "textDocument/semanticTokens/full" | "textDocument/semanticTokens/full/delta"
    ) {
        let validate_data = |value: &Value| {
            value.as_array().is_some_and(|values| {
                values
                    .iter()
                    .all(|value| value.as_u64().is_some_and(|value| value <= MAX_UINT))
            })
        };
        if let Some(data) = value.get("data") {
            if !validate_data(data)
                || !data
                    .as_array()
                    .is_some_and(|data| data.len() % SEMANTIC_TOKEN_FIELDS == 0)
            {
                return Err(Failure::MalformedResponse);
            }
        }
        if let Some(edits) = value.get("edits").and_then(Value::as_array) {
            for edit in edits {
                for field in ["start", "deleteCount"] {
                    if !edit
                        .get(field)
                        .and_then(Value::as_u64)
                        .is_some_and(|value| value <= MAX_UINT)
                    {
                        return Err(Failure::MalformedResponse);
                    }
                }
                if edit.get("data").is_some_and(|data| !validate_data(data)) {
                    return Err(Failure::MalformedResponse);
                }
            }
        }
    }
    Ok(())
}

fn validate_completion_defaults(value: &Value) -> Result<(), Failure> {
    let Some(defaults) = value.get("itemDefaults") else {
        return Ok(());
    };
    let fields = defaults.as_object().ok_or(Failure::MalformedResponse)?;
    if fields.get("commitCharacters").is_some_and(|value| {
        !value
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_string))
    }) {
        return Err(Failure::MalformedResponse);
    }
    for field in ["insertTextFormat", "insertTextMode"] {
        if fields
            .get(field)
            .is_some_and(|value| !matches!(value.as_u64(), Some(1 | 2)))
        {
            return Err(Failure::MalformedResponse);
        }
    }
    if let Some(range) = fields.get("editRange") {
        if let (Some(insert), Some(replace)) = (range.get("insert"), range.get("replace")) {
            for value in [insert, replace] {
                serde_json::from_value::<lsp_types::Range>(value.clone())
                    .map_err(|_| Failure::MalformedResponse)?;
                validate_schema(value, 0)?;
            }
        } else {
            serde_json::from_value::<lsp_types::Range>(range.clone())
                .map_err(|_| Failure::MalformedResponse)?;
            validate_schema(range, 0)?;
        }
    }
    Ok(())
}

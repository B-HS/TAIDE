use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;

use serde_json::{json, Value};

const MAX_HEADER_BYTES: usize = 4 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_METHODS: usize = 32;
const MAX_DOCUMENTS: usize = 8;
const CRASH_EXIT_CODE: u8 = 7;
const FULL_SYNC_KIND: u64 = 1;
const OVERSIZED_BODY_LENGTH: usize = 1025;
const NOTIFICATION_BURST_COUNT: usize = 3;
const SERVER_REQUEST_ID: i32 = -1;
const INITIALIZE_TRANSIENT_FAILURES: usize = 2;
const INITIALIZE_EXHAUSTED_FAILURES: usize = 3;
const INITIALIZE_FAILURE_CODE: i64 = -32002;

#[derive(Default)]
struct MockServer {
    initialize_count: usize,
    initialize_failures: usize,
    is_initialized: bool,
    is_shutdown: bool,
    should_crash_on_hover: bool,
    should_ignore_exit: bool,
    should_ignore_hover: bool,
    should_malformed_hover_once: bool,
    should_ignore_initialize: bool,
    should_send_client_requests: bool,
    should_send_client_progress: bool,
    should_track_saves: bool,
    should_format_documents: bool,
    should_run_save_actions: bool,
    should_publish_raw_diagnostics: bool,
    should_publish_inactive_diagnostics: bool,
    should_track_workspace_roots: bool,
    workspace_folders: Vec<Value>,
    workspace_reply: Option<Value>,
    pending_action_command: Option<Value>,
    held_progress: BTreeMap<u64, Value>,
    has_dynamic_registration: bool,
    unregister_trigger: Option<Value>,
    documents: BTreeMap<String, Value>,
    methods: Vec<String>,
}

fn invalid(detail: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, detail)
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut header_bytes = 0;
    let mut content_length = None;
    loop {
        let mut line = Vec::new();
        let remaining = u64::try_from(MAX_HEADER_BYTES + 1 - header_bytes)
            .map_err(|_| invalid("header size cannot be represented"))?;
        let count = (&mut *reader)
            .take(remaining)
            .read_until(b'\n', &mut line)?;
        if count == 0 && header_bytes == 0 {
            return Ok(None);
        }
        header_bytes += count;
        if count == 0 || header_bytes > MAX_HEADER_BYTES || !line.ends_with(b"\r\n") {
            return Err(invalid("invalid or oversized framing header"));
        }
        if line == b"\r\n" {
            break;
        }
        let header = std::str::from_utf8(&line)
            .map_err(|_| invalid("header must be ASCII-compatible UTF-8"))?;
        let (name, value) = header
            .split_once(':')
            .ok_or_else(|| invalid("header must have a colon"))?;
        if !name.eq_ignore_ascii_case("Content-Length") || content_length.is_some() {
            return Err(invalid("fixture accepts exactly one Content-Length header"));
        }
        let length = value
            .trim()
            .parse::<usize>()
            .map_err(|_| invalid("Content-Length must be a nonnegative byte count"))?;
        if length == 0 || length > MAX_BODY_BYTES {
            return Err(invalid("invalid or oversized framing body"));
        }
        content_length = Some(length);
    }
    let length = content_length.ok_or_else(|| invalid("missing Content-Length"))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|_| invalid("body must be valid JSON"))
}

fn write_response(output: &mut impl Write, id: &Value, result: Value) -> io::Result<()> {
    if id.as_u64().is_none() {
        return Err(invalid("fixture request ID must be an unsigned integer"));
    }
    write_payload(output, &json!({"jsonrpc":"2.0","id":id,"result":result}))
}

fn write_payload(output: &mut impl Write, payload: &Value) -> io::Result<()> {
    let body = payload.to_string();
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(body.as_bytes())?;
    output.flush()
}

fn write_document_diagnostic(
    output: &mut impl Write,
    uri: &str,
    version: &Value,
    message: &str,
) -> io::Result<()> {
    write_payload(
        output,
        &json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
            "uri":uri, "version":version, "diagnostics":[{
                "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
                "message":message, "code":"NATIVE_LIFETIME", "source":"synthetic lifetime"
            }]
        }}),
    )
}

impl MockServer {
    fn handle(&mut self, message: Value, output: &mut impl Write) -> io::Result<Option<ExitCode>> {
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err(invalid("fixture requires JSON-RPC 2.0"));
        }
        if self.should_track_workspace_roots && message.get("method").is_none() {
            if !message["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("roots-"))
                || message["result"] != json!(self.workspace_folders)
            {
                return Err(invalid(
                    "workspace roots reply must reflect every joined root",
                ));
            }
            self.workspace_reply = Some(message["result"].clone());
            return Ok(None);
        }
        if self.should_run_save_actions && message.get("method").is_none() {
            if message["id"] != "action-edit" || message["result"]["applied"] != true {
                return Err(invalid("save action edit must be acknowledged"));
            }
            let id = self
                .pending_action_command
                .take()
                .ok_or_else(|| invalid("unexpected action edit reply"))?;
            write_response(output, &id, Value::Null)?;
            return Ok(None);
        }
        if message.get("method").is_none()
            && (self.should_send_client_requests || self.has_dynamic_registration)
        {
            if message.get("id").is_none()
                || message.get("result").is_some() == message.get("error").is_some()
            {
                return Err(invalid("client reply requires ID and one result or error"));
            }
            write_payload(
                output,
                &json!({"jsonrpc":"2.0","method":"synthetic/clientReply","params":message}),
            )?;
            if self.has_dynamic_registration && message["id"] == "unregister" {
                if message.get("result") != Some(&Value::Null) {
                    return Err(invalid("unregister must be acknowledged"));
                }
                let trigger = self
                    .unregister_trigger
                    .take()
                    .ok_or_else(|| invalid("missing unregister trigger"))?;
                write_response(output, &trigger, Value::Null)?;
            }
            if message["id"] == "progress" && message.get("result") == Some(&Value::Null) {
                for value in [
                    json!({"kind":"begin","title":"합성"}),
                    json!({"kind":"report","percentage":100}),
                    json!({"kind":"end"}),
                ] {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"index","value":value}}),
                    )?;
                }
            }
            return Ok(None);
        }
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("fixture requires a method"))?;
        if self.methods.len() >= MAX_METHODS {
            return Err(invalid("fixture method budget exhausted"));
        }
        self.methods.push(method.into());
        let id = message.get("id");
        if method == "exit" {
            if id.is_some() {
                return Err(invalid("exit must be a notification"));
            }
            if self.should_ignore_exit && self.is_shutdown {
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","method":"synthetic/ignoredExit"}),
                )?;
                return Ok(None);
            }
            return Ok(Some(if self.is_shutdown {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }));
        }
        if self.is_shutdown {
            return Err(invalid("only exit may follow shutdown"));
        }
        if method == "initialize" {
            if self.is_initialized
                || (self.initialize_failures == 0
                    && (self.initialize_count != 0 || self.methods.len() != 1))
            {
                return Err(invalid(
                    "initialize must be the first request and occur once",
                ));
            }
            let params = message
                .get("params")
                .ok_or_else(|| invalid("initialize requires params"))?;
            let has_valid_root = if self.should_format_documents {
                params["rootUri"]
                    .as_str()
                    .is_some_and(|root| root.starts_with("file:///"))
                    && params["workspaceFolders"][0]["uri"] == params["rootUri"]
                    && params["rootPath"].as_str().is_some()
            } else {
                params.get("rootUri") == Some(&Value::Null)
            };
            if params.get("processId") != Some(&Value::Null)
                || !has_valid_root
                || params
                    .get("capabilities")
                    .and_then(Value::as_object)
                    .is_none()
            {
                return Err(invalid(
                    "fixture requires valid roots and client capabilities",
                ));
            }
            self.initialize_count += 1;
            if self.initialize_count <= self.initialize_failures {
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":id.ok_or_else(|| invalid("initialize requires ID"))?,
                    "error":{"code":INITIALIZE_FAILURE_CODE,"message":"synthetic transient initialize failure","data":{"retry":true}}}),
                )?;
                return Ok(None);
            }
            if self.should_track_workspace_roots {
                self.workspace_folders = params["workspaceFolders"]
                    .as_array()
                    .ok_or_else(|| invalid("workspace roots must be an array"))?
                    .clone();
            }
            if self.should_ignore_initialize {
                return Ok(None);
            }
            if self.should_send_client_progress {
                let token = params
                    .get("workDoneToken")
                    .ok_or_else(|| invalid("initialize progress token required"))?;
                for value in [
                    json!({"kind":"begin","title":"합성 초기화"}),
                    json!({"kind":"report","percentage":100}),
                ] {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":token,"value":value}}),
                    )?;
                }
            }
            let mut capabilities = json!({"positionEncoding":"utf-16","textDocumentSync":FULL_SYNC_KIND,"hoverProvider":!self.has_dynamic_registration});
            if self.should_track_saves {
                capabilities["textDocumentSync"] =
                    json!({"openClose":true,"change":FULL_SYNC_KIND,"save":{"includeText":true}});
            }
            if self.should_format_documents {
                capabilities["documentFormattingProvider"] = json!(true);
            }
            if self.should_track_workspace_roots {
                capabilities["workspace"] =
                    json!({"workspaceFolders":{"supported":true,"changeNotifications":true}});
            }
            if self.should_run_save_actions {
                capabilities["codeActionProvider"] = json!({"resolveProvider":true,"codeActionKinds":["source.fixAll","source.organizeImports"]});
                capabilities["executeCommandProvider"] =
                    json!({"commands":["synthetic.verify","synthetic.push"]});
            }
            if self.should_send_client_progress {
                capabilities["definitionProvider"] = json!(true);
            }
            write_response(
                output,
                id.ok_or_else(|| invalid("initialize requires ID"))?,
                json!({"capabilities":capabilities}),
            )?;
            if self.should_send_client_progress {
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":params["workDoneToken"],"value":{"kind":"begin","title":false}}}),
                )?;
            }
            return Ok(None);
        }
        if self.initialize_count == 0
            || (self.initialize_failures == 0 && self.initialize_count != 1)
        {
            return Err(invalid("fixture cannot accept methods before initialize"));
        }
        if method == "initialized" {
            if self.is_initialized
                || self.methods.len() != self.initialize_count + 1
                || id.is_some()
            {
                return Err(invalid("initialized must follow initialize once"));
            }
            self.is_initialized = true;
            if self.has_dynamic_registration {
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":"register","method":"client/registerCapability","params":{"registrations":[{"id":"dynamic-hover","method":"textDocument/hover","registerOptions":{"documentSelector":[{"language":"rust","scheme":"file","pattern":"**/*.rs"}]}}]}}),
                )?;
            }
            if self.should_send_client_requests {
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":SERVER_REQUEST_ID,"method":"workspace/configuration","params":{"items":[{"section":"synthetic"},{}]}}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":"unsupported","method":"synthetic/unknownRequest"}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":"invalid","method":"workspace/configuration","params":{"items":[{"section":null}]}}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":"edit","method":"workspace/applyEdit","params":{"edit":{"changes":{"file:///synthetic/outside.rs":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"합성"}]}}}}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":"progress","method":"window/workDoneProgress/create","params":{"token":"index"}}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":"refresh","method":"workspace/codeLens/refresh"}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///synthetic/native.rs","version":0,"diagnostics":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"message":"합성 진단","code":"SYNTHETIC","data":null,"experimental":{"retained":true}}]}}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"partial","value":[{"synthetic":true}]}}),
                )?;
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","method":"window/logMessage","params":{"type":3,"message":"합성 로그"}}),
                )?;
            }
            return Ok(None);
        }
        if !self.is_initialized {
            return Err(invalid("fixture cannot accept methods before initialized"));
        }
        if self.should_track_workspace_roots && method == "workspace/didChangeWorkspaceFolders" {
            if id.is_some() || message["params"]["event"]["removed"] != json!([]) {
                return Err(invalid("joined roots must be an added-only notification"));
            }
            let added = message["params"]["event"]["added"]
                .as_array()
                .ok_or_else(|| invalid("joined roots require an array"))?;
            for folder in added {
                if self
                    .workspace_folders
                    .iter()
                    .any(|existing| existing["uri"] == folder["uri"])
                {
                    return Err(invalid("joined workspace root must not be duplicated"));
                }
                self.workspace_folders.push(folder.clone());
            }
            write_payload(
                output,
                &json!({"jsonrpc":"2.0","id":format!("roots-{}",self.workspace_folders.len()),"method":"workspace/workspaceFolders"}),
            )?;
            return Ok(None);
        }
        if method == "shutdown" {
            write_response(
                output,
                id.ok_or_else(|| invalid("shutdown requires ID"))?,
                Value::Null,
            )?;
            self.is_shutdown = true;
            return Ok(None);
        }
        if method == "$/cancelRequest" {
            if id.is_some()
                || message
                    .get("params")
                    .and_then(|params| params.get("id"))
                    .and_then(Value::as_u64)
                    .is_none()
            {
                return Err(invalid(
                    "cancelRequest requires a numeric target ID notification",
                ));
            }
            let cancelled_id = message["params"]["id"]
                .as_u64()
                .ok_or_else(|| invalid("cancel must identify a numeric request"))?;
            if let Some(held) = self.held_progress.remove(&cancelled_id) {
                for field in ["workDoneToken", "partialResultToken"] {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":held[field],"value":{"kind":"begin","title":false}}}),
                    )?;
                }
            }
            return Ok(None);
        }
        if method == "synthetic/unregister" && self.has_dynamic_registration {
            self.unregister_trigger = Some(
                id.ok_or_else(|| invalid("unregister trigger needs ID"))?
                    .clone(),
            );
            write_payload(
                output,
                &json!({"jsonrpc":"2.0","id":"unregister","method":"client/unregisterCapability","params":{"unregisterations":[{"id":"dynamic-hover","method":"textDocument/hover"}]}}),
            )?;
            return Ok(None);
        }
        if self.should_run_save_actions && method == "codeAction/resolve" {
            let params = &message["params"];
            let uri = params["data"]["uri"]
                .as_str()
                .ok_or_else(|| invalid("resolve requires document metadata"))?;
            let current = self
                .documents
                .get(uri)
                .ok_or_else(|| invalid("resolve requires open document"))?;
            write_response(
                output,
                id.ok_or_else(|| invalid("resolve requires ID"))?,
                json!({
                    "title":"resolved", "kind":"source.fixAll.synthetic", "edit":{"documentChanges":[{
                        "textDocument":{"uri":uri,"version":current["version"]},
                        "edits":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"fixed:"}]
                    }]}, "command":{"title":"verify","command":"synthetic.verify","arguments":[uri]}
                }),
            )?;
            return Ok(None);
        }
        if self.should_run_save_actions && method == "workspace/executeCommand" {
            let params = &message["params"];
            let uri = params["arguments"][0]
                .as_str()
                .ok_or_else(|| invalid("command requires URI argument"))?;
            let current = self
                .documents
                .get(uri)
                .ok_or_else(|| invalid("command requires open mirror"))?;
            let id = id.ok_or_else(|| invalid("command requires ID"))?;
            match params["command"].as_str() {
                Some("synthetic.verify")
                    if current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("fixed:")) =>
                {
                    write_response(output, id, Value::Null)?
                }
                Some("synthetic.push") => {
                    self.pending_action_command = Some(id.clone());
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":"action-edit","method":"workspace/applyEdit","params":{"edit":{"changes":{uri:[{
                            "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"command:"
                        }]}}}}),
                    )?;
                }
                _ => return Err(invalid("code action command must follow its edit")),
            }
            return Ok(None);
        }
        let params = message
            .get("params")
            .ok_or_else(|| invalid("document method requires params"))?;
        let document = params
            .get("textDocument")
            .ok_or_else(|| invalid("document method requires identifier"))?;
        let uri = document
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("document requires URI"))?;
        match method {
            "textDocument/didOpen" => {
                if id.is_some()
                    || self.documents.contains_key(uri)
                    || self.documents.len() >= MAX_DOCUMENTS
                    || document.get("text").and_then(Value::as_str).is_none()
                    || document.get("languageId").and_then(Value::as_str).is_none()
                    || document.get("version").and_then(Value::as_i64).is_none()
                {
                    return Err(invalid("didOpen requires one balanced, valid mirror"));
                }
                self.documents.insert(uri.into(), document.clone());
                if self.should_publish_raw_diagnostics {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
                            "uri":"file:///synthetic/native-unbound(a),.rs", "diagnostics":[{
                                "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},
                                "message":"synthetic unbound diagnostic", "code":42, "source":"synthetic raw",
                                "data":{"fix":"synthetic retained data"}
                            }]
                        }}),
                    )?;
                }
                if self.should_run_save_actions {
                    let published_uri = if self.should_publish_raw_diagnostics {
                        uri.replace("%28", "(")
                            .replace("%29", ")")
                            .replace("%2C", ",")
                    } else {
                        uri.to_owned()
                    };
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{
                            "uri":published_uri,"version":document["version"],"diagnostics":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"message":"synthetic diagnostic","severity":1}]
                        }}),
                    )?;
                }
            }
            "textDocument/didChange" => {
                let current = self
                    .documents
                    .get_mut(uri)
                    .ok_or_else(|| invalid("didChange requires an open mirror"))?;
                let version = document
                    .get("version")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| invalid("change requires version"))?;
                let old_version = current["version"]
                    .as_i64()
                    .ok_or_else(|| invalid("mirror requires version"))?;
                let changes = params
                    .get("contentChanges")
                    .and_then(Value::as_array)
                    .ok_or_else(|| invalid("change requires contentChanges"))?;
                if id.is_some()
                    || version <= old_version
                    || changes.len() != 1
                    || changes[0].get("range").is_some()
                {
                    return Err(invalid("fixture requires increasing-version full sync"));
                }
                let text = changes[0]
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("full change requires text"))?;
                current["version"] = json!(version);
                current["text"] = json!(text);
                if self.should_publish_inactive_diagnostics {
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic updated diagnostic",
                    )?;
                }
            }
            "textDocument/didClose" => {
                if id.is_some() {
                    return Err(invalid("didClose requires one open mirror"));
                }
                let closed = self
                    .documents
                    .remove(uri)
                    .ok_or_else(|| invalid("didClose requires one open mirror"))?;
                if self.should_publish_inactive_diagnostics {
                    write_document_diagnostic(
                        output,
                        uri,
                        &closed["version"],
                        "synthetic inactive diagnostic",
                    )?;
                }
            }
            "textDocument/didSave" => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("save requires an open mirror"))?;
                if id.is_some() || !self.should_track_saves || params["text"] != current["text"] {
                    return Err(invalid("save requires the latest mirror text"));
                }
                if self.should_publish_inactive_diagnostics {
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic stale diagnostic",
                    )?;
                }
            }
            "textDocument/formatting" => {
                if !self.should_format_documents {
                    return Err(invalid("formatter fixture is disabled"));
                }
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("format requires an open mirror"))?;
                let text = current["text"]
                    .as_str()
                    .ok_or_else(|| invalid("format requires mirror text"))?;
                let mut lines = text.split('\n').collect::<Vec<_>>();
                let line = lines.len() - 1;
                let character = lines
                    .pop()
                    .unwrap_or_default()
                    .trim_end_matches('\r')
                    .encode_utf16()
                    .count();
                write_response(
                    output,
                    id.ok_or_else(|| invalid("format requires ID"))?,
                    json!([{
                        "range":{"start":{"line":0,"character":0},"end":{"line":line,"character":character}},
                        "newText":format!("formatted:{text}")
                    }]),
                )?;
            }
            "textDocument/codeAction" if self.should_run_save_actions => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("code actions require mirror"))?;
                if params["context"]["triggerKind"] != 2
                    || params["context"]["diagnostics"]
                        .as_array()
                        .is_none_or(|diagnostics| diagnostics.len() != 1)
                {
                    return Err(invalid(
                        "save actions require automatic trigger and server diagnostics",
                    ));
                }
                let actions = match params["context"]["only"][0].as_str() {
                    Some("source.fixAll") => json!([
                        {"title":"wrong kind","kind":"quickfix","edit":{"changes":{uri:[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"wrong:"}]}}},
                        {"title":"resolve fix","kind":"source.fixAll.synthetic","data":{"uri":uri}},
                        {"title":"server push","command":"synthetic.push","arguments":[uri]}
                    ]),
                    Some("source.organizeImports")
                        if current["text"]
                            .as_str()
                            .is_some_and(|text| text.starts_with("command:fixed:")) =>
                    {
                        json!([{
                            "title":"organize", "kind":"source.organizeImports", "edit":{"changes":{uri:[{
                                "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"imports:"
                            }]}}
                        }])
                    }
                    _ => {
                        return Err(invalid(
                            "organize imports must follow fix and completed command",
                        ));
                    }
                };
                write_response(
                    output,
                    id.ok_or_else(|| invalid("code actions require ID"))?,
                    actions,
                )?;
            }
            "textDocument/hover" | "textDocument/definition" => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("hover requires an open mirror"))?;
                let id = id.ok_or_else(|| invalid("hover requires ID"))?;
                if method == "textDocument/definition" {
                    if !self.should_send_client_progress {
                        return Err(invalid("definition progress fixture not enabled"));
                    }
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":params["workDoneToken"],"value":{"kind":"begin","title":"합성 정의"}}}),
                    )?;
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":params["partialResultToken"],"value":{"kind":"begin","title":false}}}),
                    )?;
                    if params["syntheticHold"] == true {
                        let request_id = id
                            .as_u64()
                            .ok_or_else(|| invalid("held request requires numeric ID"))?;
                        if self
                            .held_progress
                            .insert(request_id, params.clone())
                            .is_some()
                        {
                            return Err(invalid("held request ID must be unique"));
                        }
                        return Ok(None);
                    }
                    for value in [
                        json!({"kind":"report","percentage":100}),
                        json!({"kind":"end"}),
                    ] {
                        write_payload(
                            output,
                            &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":params["workDoneToken"],"value":value}}),
                        )?;
                    }
                    write_response(output, id, Value::Null)?;
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":params["workDoneToken"],"value":{"kind":"begin","title":false}}}),
                    )?;
                    return Ok(None);
                }
                if self.should_ignore_hover {
                    return Ok(None);
                }
                if self.should_crash_on_hover {
                    io::stderr().write_all(b"synthetic crash\n")?;
                    return Ok(Some(ExitCode::from(CRASH_EXIT_CODE)));
                }
                if self.should_malformed_hover_once {
                    self.should_malformed_hover_once = false;
                    write_response(output, id, json!({"contents":false}))?;
                    return Ok(None);
                }
                let mut result = json!({
                    "contents":{"kind":"plaintext","value":current["text"]},
                    "experimental":{"version":current["version"],"initializeCount":self.initialize_count,"methods":self.methods}
                });
                if self.should_track_workspace_roots {
                    result["experimental"]["workspaceFolders"] = json!(self.workspace_folders);
                    result["experimental"]["workspaceReply"] = json!(self.workspace_reply);
                }
                write_response(output, id, result)?;
            }
            _ => return Err(invalid("unsupported synthetic fixture method")),
        }
        Ok(None)
    }
}

fn main() -> io::Result<ExitCode> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 1 {
        return Err(invalid("unsupported synthetic server argument"));
    }
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut server = MockServer::default();
    match args.first().map(String::as_str) {
        None => {}
        Some("--crash-on-hover") => server.should_crash_on_hover = true,
        Some("--ignore-exit") => server.should_ignore_exit = true,
        Some("--ignore-hover") => server.should_ignore_hover = true,
        Some("--malformed-hover") => server.should_malformed_hover_once = true,
        Some("--ignore-initialize") => server.should_ignore_initialize = true,
        Some("--client-requests") => server.should_send_client_requests = true,
        Some("--client-progress") => server.should_send_client_progress = true,
        Some("--save-lifecycle") => server.should_track_saves = true,
        Some("--native-document") => {
            server.should_track_saves = true;
            server.should_format_documents = true;
        }
        Some("--native-actions") => {
            server.should_track_saves = true;
            server.should_format_documents = true;
            server.should_run_save_actions = true;
        }
        Some(
            mode @ ("--native-raw-diagnostics"
            | "--native-idle-diagnostics"
            | "--native-workspace-roots"
            | "--native-reinitialize"
            | "--native-reinitialize-exhausted"),
        ) => {
            server.should_track_saves = true;
            server.should_format_documents = true;
            server.should_run_save_actions = true;
            server.should_publish_raw_diagnostics = true;
            server.should_publish_inactive_diagnostics = mode == "--native-idle-diagnostics";
            server.should_track_workspace_roots = mode == "--native-workspace-roots";
            if mode == "--native-reinitialize" {
                server.initialize_failures = INITIALIZE_TRANSIENT_FAILURES;
            }
            if mode == "--native-reinitialize-exhausted" {
                server.initialize_failures = INITIALIZE_EXHAUSTED_FAILURES;
            }
        }
        Some("--dynamic-registration") => server.has_dynamic_registration = true,
        Some("--oversized-output") => {
            write!(output, "Content-Length: {OVERSIZED_BODY_LENGTH}\r\n\r\n")?;
            output.flush()?;
            loop {
                std::thread::park();
            }
        }
        Some("--notification-burst") => {
            for _ in 0..NOTIFICATION_BURST_COUNT {
                write_payload(
                    &mut output,
                    &json!({"jsonrpc":"2.0","method":"synthetic/notice"}),
                )?;
            }
            loop {
                std::thread::park();
            }
        }
        Some("--truncated-output") => {
            output.write_all(b"Content-Length: 10\r\n\r\n{")?;
            output.flush()?;
            return Ok(ExitCode::SUCCESS);
        }
        Some(_) => return Err(invalid("unsupported synthetic server argument")),
    }
    while let Some(message) = read_message(&mut input)? {
        if let Some(exit) = server.handle(message, &mut output)? {
            return Ok(exit);
        }
    }
    Err(invalid("unexpected stdin EOF before exit"))
}

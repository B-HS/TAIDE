use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::process::ExitCode;

use serde_json::{Value, json};

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
const RENAME_ERROR_CODE: i64 = -32603;

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
    should_echo_format_options: bool,
    should_format_ranges: bool,
    should_format_on_type: bool,
    should_expand_format_ranges: bool,
    should_wait_formatting: bool,
    held_formatting: BTreeMap<u64, Value>,
    should_run_save_actions: bool,
    should_publish_raw_diagnostics: bool,
    should_publish_inactive_diagnostics: bool,
    should_track_workspace_roots: bool,
    document_symbols: Option<&'static str>,
    held_symbol: Option<Value>,
    folding: Option<&'static str>,
    held_fold: Option<Value>,
    highlights: Option<&'static str>,
    has_registered_highlight_revision: bool,
    locations: Option<&'static str>,
    held_location: Option<Value>,
    documentation: Option<&'static str>,
    held_documentation: BTreeMap<u64, Value>,
    completion: Option<&'static str>,
    rename: Option<&'static str>,
    held_completion: BTreeMap<u64, Value>,
    workspace_symbols: Option<&'static str>,
    held_workspace_symbol: Option<Value>,
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

fn position_byte(text: &str, position: &Value) -> io::Result<usize> {
    let line = position["line"]
        .as_u64()
        .and_then(|line| usize::try_from(line).ok())
        .ok_or_else(|| invalid("format requires a line"))?;
    let character = position["character"]
        .as_u64()
        .and_then(|character| usize::try_from(character).ok())
        .ok_or_else(|| invalid("format requires a character"))?;
    let content = text
        .split('\n')
        .nth(line)
        .ok_or_else(|| invalid("format line is outside the document"))?
        .trim_end_matches('\r');
    let start = text
        .split_inclusive('\n')
        .take(line)
        .map(str::len)
        .sum::<usize>();
    let mut units = 0;
    for (byte, value) in content.char_indices() {
        if units == character {
            return Ok(start + byte);
        }
        units += value.len_utf16();
    }
    if units == character {
        return Ok(start + content.len());
    }
    Err(invalid("format character is not a UTF-16 boundary"))
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
            && (self.should_send_client_requests
                || self.has_dynamic_registration
                || self.highlights == Some("dynamic"))
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
            let has_valid_root = if self.should_format_documents
                || self.should_format_ranges
                || self.workspace_symbols.is_some()
                || self.locations.is_some()
                || self.documentation.is_some()
                || self.completion.is_some()
                || self.highlights.is_some()
                || self.rename.is_some()
            {
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
            if self.should_format_ranges {
                capabilities["documentRangeFormattingProvider"] = json!(true);
            }
            if self.should_format_on_type {
                capabilities["documentOnTypeFormattingProvider"] =
                    json!({"firstTriggerCharacter":";","moreTriggerCharacter":["\n"]});
            }
            if let Some(mode) = self.rename {
                if params["capabilities"]["textDocument"]["rename"]["prepareSupport"] != true {
                    return Err(invalid("native rename requires prepare support"));
                }
                capabilities["renameProvider"] = match mode {
                    "unsupported" => json!(false),
                    "no-prepare" => json!(true),
                    _ => json!({"prepareProvider":true}),
                };
            }
            if self.document_symbols.is_some() {
                if params["capabilities"]["textDocument"]["documentSymbol"]["hierarchicalDocumentSymbolSupport"]
                    != true
                {
                    return Err(invalid(
                        "native document symbols require hierarchical support",
                    ));
                }
                capabilities["documentSymbolProvider"] = json!(true);
            }
            if self.folding.is_some() {
                if params["capabilities"]["textDocument"]["foldingRange"]["lineFoldingOnly"] != true
                {
                    return Err(invalid("native folding requires line support"));
                }
                capabilities["foldingRangeProvider"] = json!(true);
            }
            if self.highlights.is_some_and(|mode| mode != "unsupported") {
                capabilities["documentHighlightProvider"] = json!(true);
            }
            if self.locations.is_some() {
                for feature in [
                    "definition",
                    "declaration",
                    "typeDefinition",
                    "implementation",
                ] {
                    if params["capabilities"]["textDocument"][feature]["linkSupport"] != true {
                        return Err(invalid("native locations require link support"));
                    }
                }
                for provider in [
                    "definitionProvider",
                    "declarationProvider",
                    "typeDefinitionProvider",
                    "implementationProvider",
                    "referencesProvider",
                ] {
                    capabilities[provider] = json!(true);
                }
            }
            if self.workspace_symbols.is_some() {
                if params["capabilities"]["workspace"]["symbol"]["symbolKind"]["valueSet"]
                    .as_array()
                    .is_none()
                {
                    return Err(invalid("native workspace symbols require kind support"));
                }
                capabilities["workspaceSymbolProvider"] = json!(true);
            }
            if self.should_track_workspace_roots {
                capabilities["workspace"] =
                    json!({"workspaceFolders":{"supported":true,"changeNotifications":true}});
            }
            if let Some(mode) = self.documentation {
                if params["capabilities"]["textDocument"]["hover"]["contentFormat"]
                    != json!(["markdown", "plaintext"])
                    || !params["capabilities"]["textDocument"]["signatureHelp"].is_object()
                {
                    return Err(invalid(
                        "native documentation requires typed client capabilities",
                    ));
                }
                capabilities["hoverProvider"] = json!(mode != "unsupported");
                if mode != "unsupported" {
                    capabilities["signatureHelpProvider"] =
                        json!({"triggerCharacters":["(",","],"retriggerCharacters":[")"]});
                }
            }
            if let Some(mode) = self.completion {
                if params["capabilities"]["textDocument"]["completion"]["completionItem"]["snippetSupport"]
                    != true
                    || params["capabilities"]["textDocument"]["completion"]["contextSupport"]
                        != true
                {
                    return Err(invalid(
                        "native completion requires typed snippet capabilities",
                    ));
                }
                if mode != "unsupported" {
                    capabilities["completionProvider"] =
                        json!({"triggerCharacters":[".",":"],"resolveProvider":false});
                }
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
            if let Some(held) = self.held_formatting.remove(&cancelled_id) {
                write_response(output, &held["id"], Value::Null)?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("held format requires URI"))?,
                    &held["version"],
                    "synthetic format cancelled",
                )?;
            }
            if let Some(held) = self.held_documentation.remove(&cancelled_id) {
                write_response(output, &held["id"], Value::Null)?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("held documentation requires URI"))?,
                    &held["version"],
                    "synthetic documentation cancelled",
                )?;
            }
            if let Some(held) = self.held_completion.remove(&cancelled_id) {
                write_response(output, &held["id"], json!([{"label":"stale-cancelled"}]))?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("held completion requires URI"))?,
                    &held["version"],
                    "synthetic completion cancelled",
                )?;
            }
            if self
                .held_location
                .as_ref()
                .is_some_and(|held| held["id"] == cancelled_id)
            {
                let held = self
                    .held_location
                    .take()
                    .ok_or_else(|| invalid("cancel requires held location"))?;
                write_response(output, &held["id"], Value::Null)?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("held location requires URI"))?,
                    &held["version"],
                    "synthetic location cancelled",
                )?;
            }
            if self
                .held_symbol
                .as_ref()
                .is_some_and(|held| held["id"] == cancelled_id)
            {
                let held = self
                    .held_symbol
                    .take()
                    .ok_or_else(|| invalid("cancel requires held symbols"))?;
                write_response(output, &held["id"], Value::Null)?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("held symbols require uri"))?,
                    &held["version"],
                    "synthetic symbol cancelled",
                )?;
            }
            if let Some(held) = self.held_progress.remove(&cancelled_id) {
                for field in ["workDoneToken", "partialResultToken"] {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":held[field],"value":{"kind":"begin","title":false}}}),
                    )?;
                }
            }
            if self
                .held_fold
                .as_ref()
                .is_some_and(|held| held["id"] == cancelled_id)
            {
                let held = self
                    .held_fold
                    .take()
                    .ok_or_else(|| invalid("cancel requires held folding"))?;
                write_response(output, &held["id"], Value::Null)?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("folding hold requires URI"))?,
                    &held["version"],
                    "synthetic folding cancelled",
                )?;
            }
            if self
                .held_workspace_symbol
                .as_ref()
                .is_some_and(|held| held["id"] == cancelled_id)
            {
                let held = self
                    .held_workspace_symbol
                    .take()
                    .ok_or_else(|| invalid("cancel requires held workspace symbols"))?;
                write_response(output, &held["id"], Value::Null)?;
                write_document_diagnostic(
                    output,
                    held["uri"]
                        .as_str()
                        .ok_or_else(|| invalid("workspace hold requires URI"))?,
                    &held["version"],
                    "synthetic workspace cancelled",
                )?;
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
        if method == "workspace/symbol" {
            let id = id.ok_or_else(|| invalid("workspace symbols require ID"))?;
            let query = message["params"]["query"]
                .as_str()
                .filter(|query| !query.is_empty())
                .ok_or_else(|| invalid("workspace symbols require query"))?;
            let mode = self
                .workspace_symbols
                .ok_or_else(|| invalid("workspace symbols are not advertised"))?;
            if mode == "--native-workspace-symbols-crash" && query == "restart" {
                return Ok(Some(ExitCode::FAILURE));
            }
            let (uri, current) = self
                .documents
                .first_key_value()
                .ok_or_else(|| invalid("workspace symbols require a fixture mirror"))?;
            if mode == "--native-workspace-symbols-wait" && query == "hold" {
                self.held_workspace_symbol =
                    Some(json!({"id":id,"uri":uri,"version":current["version"]}));
                write_document_diagnostic(
                    output,
                    uri,
                    &current["version"],
                    "synthetic workspace held",
                )?;
                return Ok(None);
            }
            if mode == "--native-workspace-symbols-error" {
                write_payload(
                    output,
                    &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32001,"message":"synthetic workspace error"}}),
                )?;
                return Ok(None);
            }
            let range = json!({"start":{"line":1,"character":4},"end":{"line":1,"character":10}});
            let mut result = json!([
                {"name":format!("ServerOnly:{uri}"),"kind":12,"containerName":"Container","location":{"uri":uri,"range":range}},
                {"name":format!("Workspace:{query}"),"kind":6,"location":{"uri":uri,"range":range}}
            ]);
            if mode == "--native-workspace-symbols-nested" {
                result
                    .as_array_mut()
                    .ok_or_else(|| invalid("workspace response must be an array"))?
                    .push(json!({"name":"Lazy","kind":12,"location":{"uri":uri},"data":{"id":1}}));
            }
            write_response(output, id, result)?;
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
            "textDocument/completion" if self.completion.is_some() => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("completion requires current mirror"))?;
                let id = id.ok_or_else(|| invalid("completion requires ID"))?;
                let mode = self
                    .completion
                    .ok_or_else(|| invalid("completion mode required"))?;
                if params.get("context").is_some()
                    || !params["position"]["line"].is_u64()
                    || !params["position"]["character"].is_u64()
                {
                    return Err(invalid(
                        "completion requires UTF-16 position without extra context",
                    ));
                }
                if mode == "wait" {
                    self.held_completion.insert(
                        id.as_u64()
                            .ok_or_else(|| invalid("hold requires numeric ID"))?,
                        json!({"id":id,"uri":uri,"version":current["version"]}),
                    );
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic completion held",
                    )?;
                    return Ok(None);
                }
                if mode == "error" {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"synthetic completion error"}}),
                    )?;
                    return Ok(None);
                }
                let items = json!([
                    {"label":"method","kind":3,"detail":"fn method(a)","documentation":{"kind":"markdown","value":"**method**"},
                    "sortText":"first","filterText":"method","insertTextFormat":2,"textEdit":{"range":{"start":{"line":params["position"]["line"],"character":4},"end":params["position"]},"newText":"method(${1|a,b|})$0"},"data":"opaque"},
                    {"label":"modifier","kind":999,"insertText":"modifier","documentation":"plain","sortText":"second"}
                ]);
                let result = match mode {
                    "null" => Value::Null,
                    "empty" => json!([]),
                    "bad" => json!({"isIncomplete":true,"items":false}),
                    "array" => items,
                    _ => json!({"isIncomplete":true,"items":items}),
                };
                write_response(output, id, result)?;
            }
            "textDocument/hover" | "textDocument/signatureHelp" if self.documentation.is_some() => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("documentation requires current mirror"))?;
                let id = id.ok_or_else(|| invalid("documentation requires ID"))?;
                let mode = self
                    .documentation
                    .ok_or_else(|| invalid("documentation mode required"))?;
                if params.get("context").is_some()
                    || !params["position"]["line"].is_u64()
                    || !params["position"]["character"].is_u64()
                {
                    return Err(invalid(
                        "documentation requires UTF-16 position without extra context",
                    ));
                }
                if mode == "wait" {
                    self.held_documentation.insert(
                        id.as_u64()
                            .ok_or_else(|| invalid("hold requires numeric ID"))?,
                        json!({"id":id,"uri":uri,"version":current["version"]}),
                    );
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic documentation held",
                    )?;
                    return Ok(None);
                }
                if mode == "crash"
                    && current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("crash"))
                {
                    return Ok(Some(ExitCode::from(CRASH_EXIT_CODE)));
                }
                if mode == "error" {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"synthetic documentation error"}}),
                    )?;
                    return Ok(None);
                }
                let result = if mode == "null" {
                    Value::Null
                } else if mode == "bad" {
                    if method == "textDocument/hover" {
                        json!({"contents":false})
                    } else {
                        json!({"signatures":true})
                    }
                } else if method == "textDocument/hover" {
                    if mode == "empty" {
                        json!({"contents":[]})
                    } else {
                        let name = if mode == "alternate" {
                            "alternate"
                        } else {
                            "primary"
                        };
                        json!({"contents":[format!("**{name} docs**"),{"language":"rust","value":format!("fn {name}() {{}}")}]})
                    }
                } else if mode == "empty" {
                    json!({"signatures":[]})
                } else {
                    let (label, parameter) = if mode == "alternate" {
                        ("alternate(\u{1f642}, x)", [10, 12])
                    } else {
                        ("f(\u{1f642}, x)", [2, 4])
                    };
                    json!({"activeSignature":0,"activeParameter":1,"signatures":[{
                        "label":label,"documentation":{"kind":"markdown","value":"**signature docs**"},"activeParameter":0,
                        "parameters":[{"label":parameter,"documentation":{"kind":"plaintext","value":"**literal parameter**"}},{"label":"x"}]
                    },{"label":"second(x)","parameters":[{"label":"x"}]}]})
                };
                write_response(output, id, result)?;
            }
            method
                if self.locations.is_some()
                    && matches!(
                        method,
                        "textDocument/definition"
                            | "textDocument/declaration"
                            | "textDocument/typeDefinition"
                            | "textDocument/implementation"
                            | "textDocument/references"
                    ) =>
            {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("locations require latest mirror"))?;
                let id = id.ok_or_else(|| invalid("locations require ID"))?;
                if self.locations == Some("wait")
                    && current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("hold"))
                {
                    self.held_location =
                        Some(json!({"id":id,"uri":uri,"version":current["version"]}));
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic location held",
                    )?;
                    return Ok(None);
                }
                if self.locations == Some("crash")
                    && current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("crash"))
                {
                    return Ok(Some(ExitCode::from(CRASH_EXIT_CODE)));
                }
                if self.locations == Some("error") {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"synthetic locations error"}}),
                    )?;
                    return Ok(None);
                }
                let selection =
                    json!({"start":{"line":1,"character":4},"end":{"line":1,"character":10}});
                let declaration =
                    json!({"start":{"line":0,"character":0},"end":{"line":0,"character":5}});
                let result = match (self.locations, method) {
                    (Some("null"), _) => Value::Null,
                    (Some("empty"), _) => json!([]),
                    (Some("bad"), _) => {
                        json!([{"uri":uri,"range":{"start":{"line":-1,"character":0},"end":{"line":0,"character":1}}}])
                    }
                    (Some("alternate"), "textDocument/definition") => {
                        json!({"uri":uri,"range":declaration})
                    }
                    (_, "textDocument/definition") => json!({"uri":uri,"range":selection}),
                    (_, "textDocument/declaration") => json!([{"uri":uri,"range":declaration}]),
                    (_, "textDocument/typeDefinition") => {
                        json!([{"targetUri":uri,"targetRange":{"start":{"line":0,"character":0},"end":{"line":2,"character":3}},"targetSelectionRange":selection,"originSelectionRange":{"start":params["position"],"end":params["position"]}}])
                    }
                    (_, "textDocument/implementation") => {
                        json!([{"uri":uri,"range":selection},{"uri":uri,"range":declaration},{"uri":uri,"range":selection}])
                    }
                    (_, "textDocument/references")
                        if params["context"]["includeDeclaration"] == true =>
                    {
                        json!([{"uri":uri,"range":declaration},{"uri":uri,"range":selection}])
                    }
                    (_, "textDocument/references")
                        if params["context"]["includeDeclaration"] == false =>
                    {
                        json!([{"uri":uri,"range":selection}])
                    }
                    _ => return Err(invalid("references require includeDeclaration")),
                };
                write_response(output, id, result)?;
            }
            "textDocument/foldingRange" => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("folding requires open mirror"))?;
                let id = id.ok_or_else(|| invalid("folding requires ID"))?;
                if self.folding == Some("wait")
                    && current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("hold"))
                {
                    self.held_fold = Some(json!({"id":id,"uri":uri,"version":current["version"]}));
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic folding held",
                    )?;
                    return Ok(None);
                }
                if self.folding == Some("crash")
                    && current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("crash"))
                {
                    return Ok(Some(ExitCode::from(CRASH_EXIT_CODE)));
                }
                if self.folding == Some("error") {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"synthetic folding error"}}),
                    )?;
                    return Ok(None);
                }
                let result = match self.folding {
                    Some("empty") => json!([]),
                    Some("null") => Value::Null,
                    _ => json!([
                        {"startLine":0,"endLine":2,"kind":"imports"},
                        {"startLine":1,"endLine":2,"kind":"custom","startCharacter":0,"endCharacter":0}
                    ]),
                };
                write_response(output, id, result)?;
            }
            "textDocument/documentHighlight" => {
                let id = id.ok_or_else(|| invalid("highlights require ID"))?;
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("highlights require latest mirror"))?;
                let mode = self
                    .highlights
                    .ok_or_else(|| invalid("highlights are not advertised"))?;
                if mode == "crash"
                    && current["text"]
                        .as_str()
                        .is_some_and(|text| text.starts_with("crash "))
                {
                    return Ok(Some(ExitCode::from(CRASH_EXIT_CODE)));
                }
                if mode == "wait" {
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic highlights held",
                    )?;
                    return Ok(None);
                }
                if mode == "error" {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"synthetic highlight error"}}),
                    )?;
                    return Ok(None);
                }
                if current.get("text").and_then(Value::as_str).is_none() {
                    return Err(invalid("highlights require text"));
                }
                let result = match mode {
                    "empty" => json!([]),
                    "null" => Value::Null,
                    "bad" => json!([{ "range": false }]),
                    "alternate" => {
                        json!([{ "range": {"start":{"line":0,"character":0},"end":{"line":0,"character":3}}, "kind":3 }])
                    }
                    _ => json!([
                        { "range": {"start":{"line":0,"character":0},"end":{"line":0,"character":5}} },
                        { "range": {"start":{"line":1,"character":4},"end":{"line":1,"character":10}}, "kind":2 },
                        { "range": {"start":{"line":2,"character":0},"end":{"line":2,"character":3}}, "kind":3 }
                    ]),
                };
                write_response(output, id, result)?;
                if mode == "dynamic" && !self.has_registered_highlight_revision {
                    self.has_registered_highlight_revision = true;
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":"highlights-register","method":"client/registerCapability","params":{"registrations":[{"id":"dynamic-highlights-revision","method":"textDocument/completion","registerOptions":{"documentSelector":[{"language":"rust","scheme":"file","pattern":"**/*.rs"}]}}]}}),
                    )?;
                }
            }
            "textDocument/documentSymbol" => {
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("document symbols require latest mirror"))?;
                let id = id.ok_or_else(|| invalid("document symbols require ID"))?;
                let mode = self
                    .document_symbols
                    .ok_or_else(|| invalid("document symbols are not advertised"))?;
                if mode == "--native-symbols-wait" && self.held_symbol.is_none() {
                    self.held_symbol =
                        Some(json!({"id":id,"uri":uri,"version":current["version"]}));
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic symbol held",
                    )?;
                    return Ok(None);
                }
                let range =
                    json!({"start":{"line":0,"character":0},"end":{"line":2,"character":3}});
                let selection =
                    json!({"start":{"line":1,"character":4},"end":{"line":1,"character":10}});
                let result = match mode {
                    "--native-symbols-flat" => {
                        json!([{"name":format!("flat:{}", current["version"]),"kind":12,"location":{"uri":uri,"range":selection},"containerName":"ignored"}])
                    }
                    "--native-symbols-bad" => {
                        json!([{"name":42,"kind":5,"range":range,"selectionRange":selection}])
                    }
                    _ => {
                        json!([{"name":format!("Class:{}", current["version"]),"detail":current["text"],"kind":5,"range":range,"selectionRange":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}},"children":[{"name":"method","kind":6,"range":selection,"selectionRange":selection}]}])
                    }
                };
                write_response(output, id, result)?;
            }
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
                if self.highlights == Some("dynamic") && text.starts_with("enable ") {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":"highlights-unregister","method":"client/unregisterCapability","params":{"unregisterations":[{"id":"dynamic-highlights-revision","method":"textDocument/completion"}]}}),
                    )?;
                }
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
                if self.should_wait_formatting {
                    let id = id
                        .and_then(Value::as_u64)
                        .ok_or_else(|| invalid("held format requires ID"))?;
                    self.held_formatting
                        .insert(id, json!({"id":id,"uri":uri,"version":current["version"]}));
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic format held",
                    )?;
                    return Ok(None);
                }
                let mut lines = text.split('\n').collect::<Vec<_>>();
                let line = lines.len() - 1;
                let character = lines
                    .pop()
                    .unwrap_or_default()
                    .trim_end_matches('\r')
                    .encode_utf16()
                    .count();
                let formatted = if self.should_echo_format_options {
                    let tab_size = params["options"]["tabSize"]
                        .as_u64()
                        .ok_or_else(|| invalid("format requires tabSize"))?;
                    let insert_spaces = params["options"]["insertSpaces"]
                        .as_bool()
                        .ok_or_else(|| invalid("format requires insertSpaces"))?;
                    format!("formatted:{tab_size}:{insert_spaces}:{text}")
                } else {
                    format!("formatted:{text}")
                };
                write_response(
                    output,
                    id.ok_or_else(|| invalid("format requires ID"))?,
                    json!([{
                        "range":{"start":{"line":0,"character":0},"end":{"line":line,"character":character}},
                        "newText":formatted
                    }]),
                )?;
            }
            "textDocument/prepareRename" | "textDocument/rename" => {
                let mode = self
                    .rename
                    .ok_or_else(|| invalid("rename fixture is disabled"))?;
                let uri = params["textDocument"]["uri"]
                    .as_str()
                    .ok_or_else(|| invalid("rename requires URI"))?;
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("rename requires mirror"))?;
                let text = current["text"]
                    .as_str()
                    .ok_or_else(|| invalid("rename requires text"))?;
                let byte = position_byte(text, &params["position"])?;
                let word = "method";
                let start = text
                    .find(word)
                    .ok_or_else(|| invalid("rename requires fixture symbol"))?;
                if !(start..=start + word.len()).contains(&byte) {
                    return Err(invalid("rename requires current UTF-16 symbol position"));
                }
                let id = id.ok_or_else(|| invalid("rename requires ID"))?;
                if mode == "wait" {
                    write_document_diagnostic(
                        output,
                        uri,
                        &current["version"],
                        "synthetic rename held",
                    )?;
                    return Ok(None);
                }
                if mode == "error" {
                    write_payload(
                        output,
                        &json!({"jsonrpc":"2.0","id":id,"error":{"code":RENAME_ERROR_CODE,"message":"synthetic rename error"}}),
                    )?;
                    return Ok(None);
                }
                let range =
                    json!({"start":{"line":1,"character":4},"end":{"line":1,"character":10}});
                let result = if method == "textDocument/prepareRename" {
                    match mode {
                        "null" => Value::Null,
                        "range" => range,
                        "default" => json!({"defaultBehavior":true}),
                        "bad" => {
                            json!({"range":{"start":{"line":1,"character":3},"end":{"line":1,"character":10}},"placeholder":word})
                        }
                        _ => json!({"range":range,"placeholder":word}),
                    }
                } else if mode == "empty" {
                    Value::Null
                } else {
                    let name = params["newName"]
                        .as_str()
                        .filter(|name| !name.trim().is_empty())
                        .ok_or_else(|| invalid("rename requires new name"))?;
                    json!({"documentChanges":[{"textDocument":{"uri":uri,"version":current["version"]},"edits":[{"range":range,"newText":name}]}]})
                };
                write_response(output, id, result)?;
            }
            "textDocument/onTypeFormatting" => {
                if !self.should_format_on_type {
                    return Err(invalid("on type formatter fixture is disabled"));
                }
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("on type format requires mirror"))?;
                let text = current["text"]
                    .as_str()
                    .ok_or_else(|| invalid("on type format requires text"))?;
                let position_byte = position_byte(text, &params["position"])?;
                let character = params["ch"]
                    .as_str()
                    .filter(|character| [";", "\n"].contains(character))
                    .ok_or_else(|| invalid("on type format requires trigger"))?;
                if !text[..position_byte].ends_with(character) {
                    return Err(invalid("on type format requires latest trigger position"));
                }
                let size = params["options"]["tabSize"]
                    .as_u64()
                    .ok_or_else(|| invalid("on type format requires tabSize"))?;
                let spaces = params["options"]["insertSpaces"]
                    .as_bool()
                    .ok_or_else(|| invalid("on type format requires insertSpaces"))?;
                let position = &params["position"];
                write_response(
                    output,
                    id.ok_or_else(|| invalid("on type format requires ID"))?,
                    json!([{"range":{"start":position,"end":position},"newText":format!("type:{size}:{spaces}:{character}")}]),
                )?;
            }
            "textDocument/rangeFormatting" => {
                if !self.should_format_ranges {
                    return Err(invalid("range formatter fixture is disabled"));
                }
                let current = self
                    .documents
                    .get(uri)
                    .ok_or_else(|| invalid("range format requires an open mirror"))?;
                let text = current["text"]
                    .as_str()
                    .ok_or_else(|| invalid("range format requires text"))?;
                let mut range = params["range"].clone();
                if self.should_expand_format_ranges {
                    let lines = text.split('\n').collect::<Vec<_>>();
                    let line = lines.len() - 1;
                    let character = lines
                        .last()
                        .unwrap()
                        .trim_end_matches('\r')
                        .encode_utf16()
                        .count();
                    range = json!({"start":{"line":0,"character":0},"end":{"line":line,"character":character}});
                }
                let start = position_byte(text, &range["start"])?;
                let end = position_byte(text, &range["end"])?;
                let selected = text
                    .get(start..end)
                    .ok_or_else(|| invalid("range format requires an ordered range"))?;
                let size = params["options"]["tabSize"]
                    .as_u64()
                    .ok_or_else(|| invalid("range format requires tabSize"))?;
                let spaces = params["options"]["insertSpaces"]
                    .as_bool()
                    .ok_or_else(|| invalid("range format requires insertSpaces"))?;
                write_response(
                    output,
                    id.ok_or_else(|| invalid("range format requires ID"))?,
                    json!([{"range":range,"newText":format!("range:{size}:{spaces}:{selected}") }]),
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
        Some(
            mode @ ("--native-rename"
            | "--native-rename-range"
            | "--native-rename-no-prepare"
            | "--native-rename-null"
            | "--native-rename-empty"
            | "--native-rename-bad"
            | "--native-rename-error"
            | "--native-rename-default"
            | "--native-rename-wait"
            | "--native-rename-unsupported"),
        ) => {
            server.rename = Some(match mode {
                "--native-rename-range" => "range",
                "--native-rename-no-prepare" => "no-prepare",
                "--native-rename-null" => "null",
                "--native-rename-empty" => "empty",
                "--native-rename-bad" => "bad",
                "--native-rename-error" => "error",
                "--native-rename-default" => "default",
                "--native-rename-wait" => "wait",
                "--native-rename-unsupported" => "unsupported",
                _ => "normal",
            });
        }
        Some("--crash-on-hover") => server.should_crash_on_hover = true,
        Some("--ignore-exit") => server.should_ignore_exit = true,
        Some("--ignore-hover") => server.should_ignore_hover = true,
        Some("--malformed-hover") => server.should_malformed_hover_once = true,
        Some("--ignore-initialize") => server.should_ignore_initialize = true,
        Some(
            mode @ ("--native-completion"
            | "--native-completion-array"
            | "--native-completion-empty"
            | "--native-completion-null"
            | "--native-completion-error"
            | "--native-completion-bad"
            | "--native-completion-wait"
            | "--native-completion-unsupported"),
        ) => {
            server.should_track_saves = true;
            server.completion = Some(match mode {
                "--native-completion-array" => "array",
                "--native-completion-empty" => "empty",
                "--native-completion-null" => "null",
                "--native-completion-error" => "error",
                "--native-completion-bad" => "bad",
                "--native-completion-wait" => "wait",
                "--native-completion-unsupported" => "unsupported",
                _ => "normal",
            });
        }
        Some("--client-requests") => server.should_send_client_requests = true,
        Some("--client-progress") => server.should_send_client_progress = true,
        Some("--save-lifecycle") => server.should_track_saves = true,
        Some(
            mode @ ("--native-highlights"
            | "--native-highlights-alternate"
            | "--native-highlights-empty"
            | "--native-highlights-null"
            | "--native-highlights-error"
            | "--native-highlights-bad"
            | "--native-highlights-wait"
            | "--native-highlights-unsupported"
            | "--native-highlights-crash"
            | "--native-highlights-dynamic"),
        ) => {
            server.should_track_saves = true;
            server.highlights = Some(match mode {
                "--native-highlights-alternate" => "alternate",
                "--native-highlights-empty" => "empty",
                "--native-highlights-null" => "null",
                "--native-highlights-error" => "error",
                "--native-highlights-bad" => "bad",
                "--native-highlights-wait" => "wait",
                "--native-highlights-unsupported" => "unsupported",
                "--native-highlights-crash" => "crash",
                "--native-highlights-dynamic" => "dynamic",
                _ => "normal",
            });
        }
        Some("--native-document") => {
            server.should_track_saves = true;
            server.should_format_documents = true;
        }
        Some("--native-format-options") => {
            server.should_track_saves = true;
            server.should_format_documents = true;
            server.should_echo_format_options = true;
            server.should_format_ranges = true;
            server.should_format_on_type = true;
        }
        Some(
            mode @ ("--native-format-range-only"
            | "--native-format-range-overlap"
            | "--native-format-wait"),
        ) => {
            server.should_track_saves = true;
            server.should_echo_format_options = true;
            server.should_format_documents = mode == "--native-format-wait";
            server.should_format_ranges = mode != "--native-format-wait";
            server.should_expand_format_ranges = mode == "--native-format-range-overlap";
            server.should_wait_formatting = mode == "--native-format-wait";
        }
        Some("--native-actions") => {
            server.should_track_saves = true;
            server.should_format_documents = true;
            server.should_run_save_actions = true;
        }
        Some(
            mode @ ("--native-documentation"
            | "--native-documentation-peek"
            | "--native-documentation-peek-highlights"
            | "--native-documentation-empty"
            | "--native-documentation-null"
            | "--native-documentation-error"
            | "--native-documentation-bad"
            | "--native-documentation-wait"
            | "--native-documentation-crash"
            | "--native-documentation-alternate"
            | "--native-documentation-unsupported"),
        ) => {
            server.should_track_saves = true;
            if matches!(
                mode,
                "--native-documentation-peek" | "--native-documentation-peek-highlights"
            ) {
                server.locations = Some("normal");
            }
            if mode == "--native-documentation-peek-highlights" {
                server.highlights = Some("normal");
            }
            server.documentation = Some(match mode {
                "--native-documentation-empty" => "empty",
                "--native-documentation-null" => "null",
                "--native-documentation-error" => "error",
                "--native-documentation-bad" => "bad",
                "--native-documentation-wait" => "wait",
                "--native-documentation-crash" => "crash",
                "--native-documentation-alternate" => "alternate",
                "--native-documentation-unsupported" => "unsupported",
                _ => "normal",
            });
        }
        Some(
            mode @ ("--native-locations"
            | "--native-locations-empty"
            | "--native-locations-null"
            | "--native-locations-error"
            | "--native-locations-bad"
            | "--native-locations-wait"
            | "--native-locations-crash"
            | "--native-locations-alternate"),
        ) => {
            server.should_track_saves = true;
            server.locations = Some(match mode {
                "--native-locations-empty" => "empty",
                "--native-locations-null" => "null",
                "--native-locations-error" => "error",
                "--native-locations-bad" => "bad",
                "--native-locations-wait" => "wait",
                "--native-locations-crash" => "crash",
                "--native-locations-alternate" => "alternate",
                _ => "ranges",
            });
        }
        Some(
            mode @ ("--native-folding"
            | "--native-folding-empty"
            | "--native-folding-null"
            | "--native-folding-error"
            | "--native-folding-wait"
            | "--native-folding-crash"),
        ) => {
            server.should_track_saves = true;
            server.should_format_documents = true;
            server.folding = Some(match mode {
                "--native-folding-empty" => "empty",
                "--native-folding-null" => "null",
                "--native-folding-error" => "error",
                "--native-folding-wait" => "wait",
                "--native-folding-crash" => "crash",
                _ => "ranges",
            });
        }
        Some(
            mode @ ("--native-symbols"
            | "--native-symbols-flat"
            | "--native-symbols-wait"
            | "--native-symbols-bad"),
        ) => {
            server.should_track_saves = true;
            server.should_format_documents = true;
            server.document_symbols = Some(match mode {
                "--native-symbols-flat" => "--native-symbols-flat",
                "--native-symbols-wait" => "--native-symbols-wait",
                "--native-symbols-bad" => "--native-symbols-bad",
                _ => "--native-symbols",
            });
        }
        Some(
            mode @ ("--native-workspace-symbols"
            | "--native-workspace-symbols-folding"
            | "--native-workspace-symbols-folding-highlights"
            | "--native-workspace-symbols-nested"
            | "--native-workspace-symbols-wait"
            | "--native-workspace-symbols-crash"
            | "--native-workspace-symbols-error"),
        ) => {
            if matches!(
                mode,
                "--native-workspace-symbols-folding"
                    | "--native-workspace-symbols-folding-highlights"
            ) {
                server.folding = Some("ranges");
            }
            if mode == "--native-workspace-symbols-folding-highlights" {
                server.highlights = Some("normal");
            }
            server.workspace_symbols = Some(match mode {
                "--native-workspace-symbols-nested" => "--native-workspace-symbols-nested",
                "--native-workspace-symbols-wait" => "--native-workspace-symbols-wait",
                "--native-workspace-symbols-crash" => "--native-workspace-symbols-crash",
                "--native-workspace-symbols-error" => "--native-workspace-symbols-error",
                _ => "--native-workspace-symbols",
            });
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

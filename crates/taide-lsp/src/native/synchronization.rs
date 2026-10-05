use serde_json::{json, Value};

use super::Failure;

const INCREMENTAL_SYNC_KIND: u64 = 2;

#[derive(Clone, Copy, Default)]
enum SyncKind {
    #[default]
    None,
    Full,
    Incremental,
}

#[derive(Default)]
pub(crate) struct SyncPolicy {
    pub(crate) open_close: bool,
    kind: SyncKind,
    save_include_text: Option<bool>,
}

fn parse_kind(value: Option<&Value>) -> Result<SyncKind, Failure> {
    match value {
        None => Ok(SyncKind::None),
        Some(value) => match value.as_u64() {
            Some(0) => Ok(SyncKind::None),
            Some(1) => Ok(SyncKind::Full),
            Some(INCREMENTAL_SYNC_KIND) => Ok(SyncKind::Incremental),
            _ => Err(Failure::MalformedResponse),
        },
    }
}

fn parse_bool(value: Option<&Value>) -> Result<bool, Failure> {
    match value {
        None => Ok(false),
        Some(value) => value.as_bool().ok_or(Failure::MalformedResponse),
    }
}

fn utf16_end_position(text: &str) -> Value {
    let mut line = 0;
    let mut character = 0;
    let mut characters = text.chars().peekable();
    while let Some(value) = characters.next() {
        if value == '\r' || value == '\n' {
            line += 1;
            character = 0;
            if value == '\r' && characters.peek() == Some(&'\n') {
                characters.next();
            }
            continue;
        }
        character += value.len_utf16();
    }
    json!({"line":line,"character":character})
}

impl SyncPolicy {
    pub(crate) fn parse(value: Option<&Value>) -> Result<Self, Failure> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        if value.is_number() {
            let kind = parse_kind(Some(value))?;
            let is_enabled = !matches!(kind, SyncKind::None);
            return Ok(Self {
                open_close: is_enabled,
                kind,
                save_include_text: is_enabled.then_some(false),
            });
        }
        let options = value.as_object().ok_or(Failure::MalformedResponse)?;
        let save_include_text = match options.get("save") {
            None | Some(Value::Bool(false)) => None,
            Some(Value::Bool(true)) => Some(false),
            Some(Value::Object(options)) => Some(parse_bool(options.get("includeText"))?),
            _ => return Err(Failure::MalformedResponse),
        };
        Ok(Self {
            open_close: parse_bool(options.get("openClose"))?,
            kind: parse_kind(options.get("change"))?,
            save_include_text,
        })
    }

    pub(crate) fn changed(
        &self,
        uri: &str,
        previous: &str,
        text: &str,
        version: i32,
    ) -> Option<Value> {
        let change = match self.kind {
            SyncKind::None => return None,
            SyncKind::Full => json!({"text":text}),
            SyncKind::Incremental => json!({
                "range":{"start":{"line":0,"character":0},"end":utf16_end_position(previous)},"text":text
            }),
        };
        Some(
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
                "textDocument":{"uri":uri,"version":version},"contentChanges":[change]
            }}),
        )
    }

    pub(crate) fn saved(&self, uri: &str, text: &str) -> Option<Value> {
        let include_text = self.save_include_text?;
        let mut params = json!({"textDocument":{"uri":uri}});
        if include_text {
            params["text"] = json!(text);
        }
        Some(json!({"jsonrpc":"2.0","method":"textDocument/didSave","params":params}))
    }
}

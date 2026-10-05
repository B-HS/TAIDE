use std::mem::size_of;

use serde_json::Value;

use super::protocol::ServerReply;
use super::Failure;

const MAX_COMMAND_JSON_DEPTH: usize = 128;

pub(super) struct PayloadSize {
    pub bytes: usize,
    limit: usize,
}

impl PayloadSize {
    pub fn new(limit: usize) -> Self {
        Self { bytes: 0, limit }
    }

    pub fn add(&mut self, bytes: usize) -> Result<(), Failure> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .filter(|size| *size <= self.limit)
            .ok_or(Failure::Capacity)?;
        Ok(())
    }

    pub fn slots<T>(&mut self, capacity: usize) -> Result<(), Failure> {
        self.add(
            capacity
                .checked_mul(size_of::<T>())
                .ok_or(Failure::Capacity)?,
        )
    }

    pub fn value(&mut self, value: &Value, depth: usize) -> Result<(), Failure> {
        if depth > MAX_COMMAND_JSON_DEPTH {
            return Err(Failure::Capacity);
        }
        self.add(size_of::<Value>())?;
        match value {
            Value::String(value) => self.add(value.capacity())?,
            Value::Array(values) => {
                self.slots::<Value>(values.capacity())?;
                for value in values {
                    self.value(value, depth + 1)?;
                }
            }
            Value::Object(values) => {
                self.slots::<(String, Value)>(values.len())?;
                for (key, value) in values {
                    self.add(key.capacity())?;
                    self.value(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn reply(&mut self, reply: &ServerReply) -> Result<(), Failure> {
        match reply {
            ServerReply::Configuration(values) => {
                self.slots::<Value>(values.capacity())?;
                for value in values {
                    self.value(value, 0)?;
                }
            }
            ServerReply::ApplyEdit(reply) => {
                if let Some(reason) = &reply.failure_reason {
                    self.add(reason.capacity())?;
                }
            }
            ServerReply::WorkspaceFolders(Some(folders)) => {
                self.slots::<super::protocol::lsp_types::WorkspaceFolder>(folders.capacity())?;
                for folder in folders {
                    self.add(folder.name.capacity())?;
                    self.add(folder.uri.as_str().len())?;
                }
            }
            ServerReply::ShowMessage(Some(action)) => {
                use super::protocol::lsp_types::MessageActionItemProperty;

                self.add(action.title.capacity())?;
                self.slots::<(String, MessageActionItemProperty)>(action.properties.capacity())?;
                for (key, value) in &action.properties {
                    self.add(key.capacity())?;
                    match value {
                        MessageActionItemProperty::String(value) => self.add(value.capacity())?,
                        MessageActionItemProperty::Object(value) => self.value(value, 0)?,
                        MessageActionItemProperty::Boolean(_)
                        | MessageActionItemProperty::Integer(_) => {}
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::protocol::lsp_types::{
        ApplyWorkspaceEditResponse, MessageActionItem, MessageActionItemProperty, WorkspaceFolder,
    };
    use super::*;

    const TEST_BYTES: usize = 16 * 1024;
    const RESERVED_BYTES: usize = 1024;

    #[test]
    fn payload_shape은_보유_용량과_json_깊이_경계를_검사한다() {
        let mut text = String::with_capacity(RESERVED_BYTES);
        text.push('x');
        let capacity = text.capacity();
        let replies = [
            ServerReply::Configuration(vec![Value::String(text.clone())]),
            ServerReply::ApplyEdit(ApplyWorkspaceEditResponse {
                applied: false,
                failure_reason: Some(text.clone()),
                failed_change: None,
            }),
            ServerReply::ShowMessage(Some(MessageActionItem {
                title: text,
                properties: std::collections::HashMap::from([
                    (
                        "reserved".into(),
                        MessageActionItemProperty::String(String::with_capacity(RESERVED_BYTES)),
                    ),
                    (
                        "opaque".into(),
                        MessageActionItemProperty::Object(serde_json::json!({"flag":true})),
                    ),
                ]),
            })),
            ServerReply::WorkspaceFolders(Some(vec![WorkspaceFolder {
                name: "synthetic".into(),
                uri: "file:///synthetic".parse().unwrap(),
            }])),
        ];
        for reply in replies {
            let mut size = PayloadSize::new(TEST_BYTES);
            size.reply(&reply).unwrap();
            assert!(size.bytes > 0);
            let mut exact = PayloadSize::new(size.bytes);
            exact.reply(&reply).unwrap();
            let mut insufficient = PayloadSize::new(size.bytes - 1);
            assert_eq!(insufficient.reply(&reply), Err(Failure::Capacity));
            if matches!(reply, ServerReply::ShowMessage(_)) {
                assert!(size.bytes >= capacity + RESERVED_BYTES);
            }
        }
        let mut empty = PayloadSize::new(0);
        empty.reply(&ServerReply::Acknowledged).unwrap();
        empty.reply(&ServerReply::WorkspaceFolders(None)).unwrap();
        empty.reply(&ServerReply::ShowMessage(None)).unwrap();
        let mut nested = Value::Null;
        for _ in 0..MAX_COMMAND_JSON_DEPTH {
            nested = Value::Array(vec![nested]);
        }
        PayloadSize::new(TEST_BYTES).value(&nested, 0).unwrap();
        nested = Value::Array(vec![nested]);
        assert_eq!(
            PayloadSize::new(TEST_BYTES).value(&nested, 0),
            Err(Failure::Capacity)
        );
    }
}

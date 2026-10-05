use std::collections::BTreeMap;

use serde_json::Value;
use taide_model::error::AppError;
use taide_native_ui::settings_resources::{Kind, Reply, Request};

use crate::{
    ResponsePayload,
    settings_catalog::{decode, json_payload},
    shell::Call,
};

#[derive(Default)]
pub struct ResourceReads {
    pending: BTreeMap<u32, Request>,
    finished: Vec<Reply>,
}

impl ResourceReads {
    pub fn call(request: &Request) -> Call {
        Call {
            command: request.kind().command(),
            args: Value::Null,
        }
    }

    pub fn sent(&mut self, request: Request, seq: u32) {
        self.pending.insert(seq, request);
    }

    pub fn failed(&mut self, request: Request, error: AppError) {
        self.finished.push(request.failed(error));
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        let Some(request) = self.pending.remove(&seq) else {
            return false;
        };
        let result = json_payload(result);
        self.finished.push(match request.kind() {
            Kind::Fonts => Reply::Fonts {
                request,
                result: result.and_then(decode),
            },
            Kind::Shells => Reply::Shells {
                request,
                result: result.and_then(decode),
            },
        });
        true
    }

    pub fn disconnected(&mut self) {
        for (_, request) in std::mem::take(&mut self.pending) {
            self.failed(
                request,
                crate::settings_catalog::invocation_error(crate::InvokeError::Closed),
            );
        }
    }

    pub fn take_finished(&mut self) -> Vec<Reply> {
        std::mem::take(&mut self.finished)
    }
}

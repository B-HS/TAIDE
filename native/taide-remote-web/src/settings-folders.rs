use std::collections::BTreeSet;

use serde_json::{Value, json};
use taide_model::{error::AppError, system::AppDataPathKind};

use crate::{
    InvokeError, ResponsePayload,
    settings_catalog::{decode, invocation_error, json_payload},
    shell::Call,
};

#[derive(Default)]
pub struct FolderActions {
    pending: BTreeSet<u32>,
    errors: Vec<AppError>,
}

impl FolderActions {
    pub fn call(kind: AppDataPathKind) -> Call {
        Call {
            command: "system_open_app_data_path",
            args: json!({"kind":kind}),
        }
    }

    pub fn sent(&mut self, seq: u32) {
        self.pending.insert(seq);
    }

    pub fn failed(&mut self, error: InvokeError) {
        self.errors.push(invocation_error(error));
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        if !self.pending.remove(&seq) {
            return false;
        }
        if let Err(error) = json_payload(result).and_then(decode::<()>) {
            self.errors.push(error);
        }
        true
    }

    pub fn disconnected(&mut self) {
        self.errors.extend(
            std::mem::take(&mut self.pending)
                .into_iter()
                .map(|_| invocation_error(InvokeError::Closed)),
        );
    }

    pub fn take_errors(&mut self) -> Vec<AppError> {
        std::mem::take(&mut self.errors)
    }
}

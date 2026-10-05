use std::collections::BTreeMap;

use serde_json::{Value, json};
use taide_model::{error::AppError, snippet::SnippetFile};
use taide_native_ui::snippet_edit::{Kind, Outcome, Reply, Request};

use crate::{
    InvokeError, ResponsePayload,
    settings_catalog::{decode, invocation_error, json_payload},
    shell::{Call, Failure},
};

#[derive(Default)]
pub struct SnippetOperations {
    pending: BTreeMap<u32, Request>,
    replies: Vec<Reply>,
    failures: Vec<Failure>,
    catalog_pending: BTreeMap<u32, taide_native_ui::snippet_catalog::Request>,
    catalog_replies: Vec<taide_native_ui::snippet_catalog::Reply>,
}

impl SnippetOperations {
    pub fn call(request: &Request) -> Call {
        match request.kind() {
            Kind::List => Call {
                command: "snippet_list",
                args: Value::Null,
            },
            Kind::Save {
                file_name, content, ..
            } => Call {
                command: "snippet_save",
                args: json!({"fileName": file_name, "content": content}),
            },
            Kind::Delete { file_name } => Call {
                command: "snippet_delete",
                args: json!({"fileName": file_name}),
            },
        }
    }

    pub fn is_pending(&self, request: &Request) -> bool {
        self.pending
            .values()
            .any(|pending| pending.same_request(request))
    }

    pub fn sent(&mut self, request: Request, seq: u32) {
        self.pending.insert(seq, request);
    }

    pub fn failed(&mut self, request: Request, error: AppError) {
        self.finish(request, Err(error));
    }

    fn finish(&mut self, request: Request, result: taide_model::error::AppResult<Outcome>) {
        if !matches!(request.kind(), Kind::List)
            && let Err(error) = &result
        {
            self.failures.push(match serde_json::to_value(error) {
                Ok(value) => Failure::Remote(value),
                Err(_) => Failure::MalformedResponse,
            });
        }
        self.replies.push(Reply { request, result });
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        if let Some(request) = self.catalog_pending.remove(&seq) {
            self.catalog_replies
                .push(taide_native_ui::snippet_catalog::Reply {
                    request,
                    result: json_payload(result).and_then(decode),
                });
            return true;
        }
        let Some(request) = self.pending.remove(&seq) else {
            return false;
        };
        let result = json_payload(result).and_then(|value| match request.kind() {
            Kind::List => decode(value).map(Outcome::Listed),
            Kind::Save { file_name, .. } => {
                let file: SnippetFile = decode(value)?;
                if file.file_name != *file_name {
                    return Err(AppError::InvalidArgument(
                        "remote saved snippet does not match its request".into(),
                    ));
                }
                Ok(Outcome::Saved(file))
            }
            Kind::Delete { .. } => decode::<()>(value).map(|()| Outcome::Deleted),
        });
        self.finish(request, result);
        true
    }

    pub fn has_pending_mutations(&self) -> bool {
        self.pending
            .values()
            .any(|request| !matches!(request.kind(), Kind::List))
    }

    pub fn failures(&self) -> &[Failure] {
        &self.failures
    }

    pub fn take_failures(&mut self) -> Vec<Failure> {
        std::mem::take(&mut self.failures)
    }

    pub fn take_replies(&mut self) -> Vec<Reply> {
        std::mem::take(&mut self.replies)
    }

    pub fn disconnected(&mut self) {
        for request in std::mem::take(&mut self.catalog_pending).into_values() {
            self.catalog_failed(request, invocation_error(InvokeError::Closed));
        }
        for request in std::mem::take(&mut self.pending).into_values() {
            self.failed(request, invocation_error(InvokeError::Closed));
        }
    }

    pub fn catalog_sent(&mut self, request: taide_native_ui::snippet_catalog::Request, seq: u32) {
        self.catalog_pending.insert(seq, request);
    }

    pub fn catalog_failed(
        &mut self,
        request: taide_native_ui::snippet_catalog::Request,
        error: AppError,
    ) {
        self.catalog_replies.push(request.failed(error));
    }

    pub fn take_catalog_replies(&mut self) -> Vec<taide_native_ui::snippet_catalog::Reply> {
        std::mem::take(&mut self.catalog_replies)
    }
}

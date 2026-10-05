use std::collections::BTreeMap;

use serde_json::{Value, json};
use taide_model::{
    app::AppFileTarget,
    error::AppError,
    ids::ProjectId,
    layout::{ProjectLayout, TabKind},
};
use taide_native_ui::settings_owner::Owner;

use crate::{
    InvokeError, ResponsePayload,
    settings_catalog::{decode, invocation_error, json_payload},
    shell::Call,
};

pub struct Opened {
    pub project: ProjectId,
    pub layout: ProjectLayout,
}

#[derive(Default)]
pub struct AppFileOpens {
    pending: BTreeMap<u32, ProjectId>,
    errors: Vec<AppError>,
}

impl AppFileOpens {
    pub fn is_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn settings_call(owner: &Owner) -> Call {
        Call {
            command: "layout_open_tab",
            args: json!({"projectId":owner.project,"kind":TabKind::AppFile { target:AppFileTarget::Settings },"title":"settings.json","target":owner.pane,"preview":false}),
        }
    }

    pub fn sent(&mut self, seq: u32, project: ProjectId) {
        self.pending.insert(seq, project);
    }

    pub fn failed(&mut self, error: InvokeError) {
        self.errors.push(invocation_error(error));
    }

    pub fn response(
        &mut self,
        seq: u32,
        result: &Result<ResponsePayload, Value>,
    ) -> Option<Option<Opened>> {
        let project = self.pending.remove(&seq)?;
        match json_payload(result).and_then(decode::<ProjectLayout>) {
            Ok(layout) => Some(Some(Opened { project, layout })),
            Err(error) => {
                self.errors.push(error);
                Some(None)
            }
        }
    }

    pub fn disconnected(&mut self) {
        self.errors.extend(
            std::mem::take(&mut self.pending)
                .into_values()
                .map(|_| invocation_error(InvokeError::Closed)),
        );
    }

    pub fn take_errors(&mut self) -> Vec<AppError> {
        std::mem::take(&mut self.errors)
    }
}

use std::collections::BTreeMap;

use serde_json::{Value, json};
use taide_model::error::AppError;
use taide_model::settings::Settings;
use taide_native_ui::settings_controls::Change;

use crate::shell::{Call, Failure};
use crate::{InvokeError, ResponsePayload};

pub fn change_call(change: &Change) -> Call {
    if let Change::Theme(id) = change {
        return Call {
            command: "settings_set_theme",
            args: json!({"themeId": id}),
        };
    }
    Call {
        command: "settings_update",
        args: json!({"patch": change.patch()}),
    }
}

pub struct Finished {
    pub seq: u32,
    pub change: Change,
    pub result: Result<Settings, Failure>,
    pub settings_generation: u64,
}

struct Pending {
    change: Change,
    settings_generation: u64,
}

#[derive(Default)]
pub struct PreferenceWrites {
    pending: BTreeMap<u32, Pending>,
    errors: Vec<AppError>,
    close_failures: Vec<Failure>,
}

impl PreferenceWrites {
    pub fn is_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn sent(&mut self, seq: u32, change: Change, settings_generation: u64) {
        self.pending.insert(
            seq,
            Pending {
                change,
                settings_generation,
            },
        );
    }

    pub fn response(
        &mut self,
        seq: u32,
        result: &Result<ResponsePayload, Value>,
    ) -> Option<Finished> {
        let pending = self.pending.remove(&seq)?;
        let result = match result {
            Ok(ResponsePayload::Json(value)) => {
                serde_json::from_value(value.clone()).map_err(|_| Failure::MalformedResponse)
            }
            Ok(ResponsePayload::Binary(_)) => Err(Failure::MalformedResponse),
            Err(value) => Err(Failure::Remote(value.clone())),
        };
        if let Err(failure) = &result {
            self.close_failures.push(failure.clone());
            let error = match failure {
                Failure::Invocation(error) => {
                    crate::settings_catalog::invocation_error(error.clone())
                }
                Failure::Remote(value) => {
                    serde_json::from_value(value.clone()).unwrap_or_else(|_| {
                        AppError::Internal("remote Settings response is malformed".into())
                    })
                }
                Failure::MalformedResponse => {
                    AppError::Internal("remote Settings response is malformed".into())
                }
            };
            self.errors.push(error);
        }
        Some(Finished {
            seq,
            change: pending.change,
            result,
            settings_generation: pending.settings_generation,
        })
    }

    pub fn disconnected(&mut self) -> Vec<Finished> {
        self.close_failures.extend(
            self.pending
                .values()
                .map(|_| Failure::Invocation(InvokeError::Closed)),
        );
        self.errors.extend(
            self.pending
                .values()
                .map(|_| crate::settings_catalog::invocation_error(InvokeError::Closed)),
        );
        std::mem::take(&mut self.pending)
            .into_iter()
            .map(|(seq, pending)| Finished {
                seq,
                change: pending.change,
                result: Err(Failure::Invocation(InvokeError::Closed)),
                settings_generation: pending.settings_generation,
            })
            .collect()
    }

    pub fn take_errors(&mut self) -> Vec<AppError> {
        std::mem::take(&mut self.errors)
    }

    pub fn take_close_failures(&mut self) -> Vec<Failure> {
        std::mem::take(&mut self.close_failures)
    }
}

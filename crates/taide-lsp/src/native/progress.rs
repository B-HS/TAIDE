use std::collections::HashMap;

use lsp_types::{NumberOrString, WorkDoneProgress};
use serde_json::Value;

use super::protocol::{self, ProgressValue};
use super::Failure;

enum ProgressKind {
    WorkDone { has_ended: bool },
    Partial,
}

struct Registration {
    request_id: Option<u64>,
    kind: ProgressKind,
}

pub(super) struct RequestProgress {
    work_done: Option<NumberOrString>,
    partial: Option<NumberOrString>,
}

pub(super) struct ProgressRegistry {
    entries: HashMap<NumberOrString, Registration>,
    capacity: usize,
}

impl ProgressRegistry {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(super) fn prepare_request(&self, params: &Value) -> Result<RequestProgress, Failure> {
        let parse = |field| {
            params
                .get(field)
                .map(|value| {
                    serde_json::from_value::<NumberOrString>(value.clone())
                        .map_err(|_| Failure::MalformedRequest)
                })
                .transpose()
        };
        let work_done = parse("workDoneToken")?;
        let partial = parse("partialResultToken")?;
        if work_done.is_some() && work_done == partial {
            return Err(Failure::MalformedRequest);
        }
        let tokens = work_done.iter().chain(partial.iter());
        if tokens.clone().any(|token| self.entries.contains_key(token)) {
            return Err(Failure::MalformedRequest);
        }
        if self
            .entries
            .len()
            .checked_add(tokens.count())
            .is_none_or(|count| count > self.capacity)
        {
            return Err(Failure::Capacity);
        }
        Ok(RequestProgress { work_done, partial })
    }

    pub(super) fn register_request(&mut self, id: u64, progress: RequestProgress) {
        for (token, kind) in [
            (
                progress.work_done,
                ProgressKind::WorkDone { has_ended: false },
            ),
            (progress.partial, ProgressKind::Partial),
        ] {
            if let Some(token) = token {
                self.entries.insert(
                    token,
                    Registration {
                        request_id: Some(id),
                        kind,
                    },
                );
            }
        }
    }

    pub(super) fn finish_request(&mut self, id: u64) {
        self.entries
            .retain(|_, registration| registration.request_id != Some(id));
    }

    pub(super) fn can_register_server(&self, token: &NumberOrString) -> bool {
        !self.entries.contains_key(token) && self.entries.len() < self.capacity
    }

    pub(super) fn register_server(&mut self, token: NumberOrString) {
        self.entries.insert(
            token,
            Registration {
                request_id: None,
                kind: ProgressKind::WorkDone { has_ended: false },
            },
        );
    }

    pub(super) fn classify(
        &mut self,
        token: &NumberOrString,
        value: &mut ProgressValue,
    ) -> Result<(), Failure> {
        let Some(registration) = self.entries.get_mut(token) else {
            return Ok(());
        };
        let ProgressKind::WorkDone { has_ended: false } = registration.kind else {
            return Ok(());
        };
        let ProgressValue::Partial(raw) = value else {
            return Err(Failure::MalformedResponse);
        };
        let work_done = protocol::decode_work_done(raw)?;
        let is_server_end =
            matches!(work_done, WorkDoneProgress::End(_)) && registration.request_id.is_none();
        if matches!(work_done, WorkDoneProgress::End(_)) {
            registration.kind = ProgressKind::WorkDone { has_ended: true };
        }
        *value = ProgressValue::WorkDone(work_done);
        if is_server_end {
            self.entries.remove(token);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TOKEN_CAPACITY: usize = 2;

    #[test]
    fn 진행_토큰은_충돌_상한_end와_응답_수명을_구별하고_partial_원형을_유지한다() {
        let mut registry = ProgressRegistry::new(TOKEN_CAPACITY);
        for params in [
            json!({"workDoneToken":null}),
            json!({"workDoneToken":true}),
            json!({"workDoneToken":i64::MAX}),
            json!({"workDoneToken":"same","partialResultToken":"same"}),
        ] {
            assert!(matches!(
                registry.prepare_request(&params),
                Err(Failure::MalformedRequest)
            ));
            assert_eq!(registry.len(), 0);
        }
        let params = json!({"workDoneToken":0,"partialResultToken":"partial"});
        let progress = registry.prepare_request(&params).unwrap();
        registry.register_request(1, progress);
        assert_eq!(registry.len(), TOKEN_CAPACITY);
        assert!(matches!(
            registry.prepare_request(&params),
            Err(Failure::MalformedRequest)
        ));
        assert!(matches!(
            registry.prepare_request(&json!({"workDoneToken":"third"})),
            Err(Failure::Capacity)
        ));
        assert!(!registry.can_register_server(&NumberOrString::Number(0)));
        let raw = json!({"kind":"begin","title":false});
        let mut partial = ProgressValue::Partial(raw.clone());
        registry
            .classify(&NumberOrString::String("partial".into()), &mut partial)
            .unwrap();
        assert_eq!(partial, ProgressValue::Partial(raw));
        let mut end = ProgressValue::Partial(json!({"kind":"end"}));
        registry
            .classify(&NumberOrString::Number(0), &mut end)
            .unwrap();
        assert!(matches!(
            end,
            ProgressValue::WorkDone(WorkDoneProgress::End(_))
        ));
        assert_eq!(registry.len(), TOKEN_CAPACITY);
        assert!(matches!(
            registry.prepare_request(&params),
            Err(Failure::MalformedRequest)
        ));
        registry.finish_request(0);
        assert_eq!(registry.len(), TOKEN_CAPACITY);
        registry.finish_request(1);
        assert_eq!(registry.len(), 0);
        let token = NumberOrString::String("server".into());
        assert!(registry.can_register_server(&token));
        registry.register_server(token.clone());
        let mut end = ProgressValue::Partial(json!({"kind":"end"}));
        registry.classify(&token, &mut end).unwrap();
        assert_eq!(registry.len(), 0);
        let progress = registry.prepare_request(&params).unwrap();
        registry.register_request(1, progress);
        registry.clear();
        assert_eq!(registry.len(), 0);
    }
}

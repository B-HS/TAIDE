use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;
use taide_model::{
    error::{AppError, AppResult},
    locale::LocaleSummary,
    theme::ThemeSummary,
};
use taide_native_ui::settings_view::{Catalog, Request};

use crate::{InvokeError, ResponsePayload, shell::Call};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Read {
    Themes,
    Locales,
}

impl Read {
    pub fn call(self) -> Call {
        Call {
            command: match self {
                Self::Themes => "theme_list",
                Self::Locales => "locale_list",
            },
            args: Value::Null,
        }
    }
}

pub struct Finished {
    pub request: Request,
    pub catalog: Catalog,
}

struct Loading {
    request: Request,
    themes: Option<AppResult<Vec<ThemeSummary>>>,
    locales: Option<AppResult<Vec<LocaleSummary>>>,
}

#[derive(Default)]
pub struct CatalogReads {
    loading: Vec<Loading>,
    pending: BTreeMap<u32, (Request, Read)>,
    finished: Vec<Finished>,
}

impl CatalogReads {
    pub fn request(&mut self, request: Request) {
        if self.loading.iter().any(|load| load.request == request) {
            return;
        }
        self.loading
            .retain(|load| load.request.owner() != request.owner());
        self.finished
            .retain(|load| load.request.owner() != request.owner());
        self.loading.push(Loading {
            request,
            themes: None,
            locales: None,
        });
    }

    pub fn next_reads(&self) -> Vec<(Request, Read)> {
        self.loading
            .iter()
            .flat_map(|load| {
                [Read::Themes, Read::Locales]
                    .into_iter()
                    .filter_map(|read| {
                        let complete = match read {
                            Read::Themes => load.themes.is_some(),
                            Read::Locales => load.locales.is_some(),
                        };
                        let pending = self
                            .pending
                            .values()
                            .any(|(request, part)| request == &load.request && *part == read);
                        (!complete && !pending).then(|| (load.request.clone(), read))
                    })
            })
            .collect()
    }

    pub fn sent(&mut self, request: Request, read: Read, seq: u32) {
        self.pending.insert(seq, (request, read));
    }

    pub fn failed(&mut self, request: &Request, read: Read, error: AppError) {
        self.apply(request, read, Err(error));
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        let Some((request, read)) = self.pending.remove(&seq) else {
            return false;
        };
        self.apply(&request, read, json_payload(result));
        true
    }

    fn apply(&mut self, request: &Request, read: Read, result: AppResult<Value>) {
        let Some(index) = self
            .loading
            .iter()
            .position(|load| &load.request == request)
        else {
            return;
        };
        let load = &mut self.loading[index];
        match read {
            Read::Themes => load.themes = Some(result.and_then(decode)),
            Read::Locales => load.locales = Some(result.and_then(decode)),
        }
        if load.themes.is_none() || load.locales.is_none() {
            return;
        }
        let load = self.loading.remove(index);
        self.finished.push(Finished {
            request: load.request,
            catalog: Catalog {
                themes: load.themes.expect("completed theme catalog"),
                locales: load.locales.expect("completed locale catalog"),
            },
        });
    }

    pub fn invalidate(&mut self) {
        self.loading.clear();
        self.finished.clear();
    }

    pub fn disconnected(&mut self) {
        self.pending.clear();
        for load in std::mem::take(&mut self.loading) {
            self.finished.push(Finished {
                request: load.request,
                catalog: Catalog {
                    themes: load
                        .themes
                        .unwrap_or_else(|| Err(invocation_error(InvokeError::Closed))),
                    locales: load
                        .locales
                        .unwrap_or_else(|| Err(invocation_error(InvokeError::Closed))),
                },
            });
        }
    }

    pub fn take_finished(&mut self) -> Vec<Finished> {
        std::mem::take(&mut self.finished)
    }
}

pub fn invocation_error(error: InvokeError) -> AppError {
    match error {
        InvokeError::Closed => AppError::Forbidden("remote Settings connection is closed".into()),
        InvokeError::SessionExpired => {
            AppError::Forbidden("remote Settings session has expired".into())
        }
        InvokeError::SequenceExhausted => {
            AppError::Internal("remote Settings sequence exhausted".into())
        }
        InvokeError::Encoding(_) => {
            AppError::Internal("remote Settings request encoding failed".into())
        }
    }
}

pub fn json_payload(result: &Result<ResponsePayload, Value>) -> AppResult<Value> {
    match result {
        Ok(ResponsePayload::Json(value)) => Ok(value.clone()),
        Ok(ResponsePayload::Binary(_)) => Err(malformed()),
        Err(error) => Err(serde_json::from_value(error.clone()).unwrap_or_else(|_| malformed())),
    }
}

pub fn decode<T: DeserializeOwned>(value: Value) -> AppResult<T> {
    serde_json::from_value(value).map_err(|_| malformed())
}

fn malformed() -> AppError {
    AppError::Internal("remote Settings response is malformed".into())
}

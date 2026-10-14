use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use taide_infra::root_guard;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_graph::{self, Fetch, Job, Progress, Start};
use taide_native_editor::json_schema_load::{self, Response};
use taide_native_editor::json_schema_service::Service;
use taide_native_editor::json_schema_uri;
use taide_runtime::{AppServices, AppState, TaskSupervisor};
use tokio::sync::{oneshot, watch};
use url::Url;

const WORKER_NAME: &str = "native-json-schema-request";
const FILE_WORKER_NAME: &str = "native-json-schema-file";
const FAILED: &str = "Schema request failed";
const DENIED: &str = "Schema request is outside its authorized source";
const TOO_LARGE: &str = "Schema response exceeds the byte budget";
const TIMED_OUT: &str = "Schema request timed out";

#[derive(Clone, Copy)]
pub struct Limits {
    pub uri: json_schema_uri::Limits,
    pub max_bytes: usize,
    pub max_active_requests: usize,
    pub timeout: Duration,
}

#[derive(Clone)]
pub struct Scope {
    state: AppState,
    project: ProjectId,
    root: PathBuf,
    resource: Url,
    active: Arc<AtomicUsize>,
}

pub struct Loading {
    scope: Scope,
    fetch: Fetch,
    receiver: Option<oneshot::Receiver<Payload>>,
    cancel: watch::Sender<bool>,
    _slot: Arc<Slot>,
}

struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

enum Payload {
    Unavailable,
    Text(String),
    Failed(&'static str),
}

impl Scope {
    pub fn new(state: AppState, project: ProjectId, resource: &str) -> AppResult<Self> {
        let resource = Url::parse(resource)
            .map_err(|_| AppError::InvalidArgument("schema resource URI is invalid".into()))?;
        let root = root_guard::project_root(&state.projects.read(), &project)?;
        let scope = Self {
            state,
            project,
            root,
            resource,
            active: Arc::new(AtomicUsize::new(0)),
        };
        if !scope.is_current() {
            return Err(AppError::Forbidden("schema project is unavailable".into()));
        }
        Ok(scope)
    }

    fn is_current(&self) -> bool {
        !self.state.is_shutting_down()
            && root_guard::project_root(&self.state.projects.read(), &self.project)
                .is_ok_and(|root| root == self.root)
    }

    fn allows_reference(&self, parent: &Url, target: &Url) -> bool {
        if !target.username().is_empty() || target.password().is_some() {
            return false;
        }
        match target.scheme() {
            "http" | "https" => true,
            "file" => {
                self.resource.scheme() == "file"
                    && matches!(parent.scheme(), "file" | "schemaservice")
            }
            _ => false,
        }
    }

    pub fn begin(
        &self,
        service: &mut Service<'_>,
        uri: &[u16],
        limits: json_schema_graph::Limits,
    ) -> Result<Start, EditorError> {
        if !self.is_current() {
            return Err(EditorError::StaleRevision);
        }
        if uri.len() > limits.uri.max_input_units {
            return Err(EditorError::Capacity);
        }
        let target = String::from_utf16(uri).map_err(|_| EditorError::InvalidBoundary)?;
        let target = Url::parse(&target).map_err(|_| EditorError::InvalidBoundary)?;
        if target.scheme() != "schemaservice" && !self.allows_reference(&self.resource, &target) {
            return Err(EditorError::InvalidBoundary);
        }
        Job::start(service, uri, true, limits)
    }

    pub fn advance(&self, job: Job, service: &mut Service<'_>) -> Result<Progress, EditorError> {
        if !self.is_current() {
            job.abort(service);
            return Err(EditorError::StaleRevision);
        }
        job.advance_with_access(service, |parent, target| {
            let (Ok(parent), Ok(target)) = (String::from_utf16(parent), String::from_utf16(target))
            else {
                return false;
            };
            let (Ok(parent), Ok(target)) = (Url::parse(&parent), Url::parse(&target)) else {
                return false;
            };
            self.allows_reference(&parent, &target)
        })
    }

    pub fn start(
        &self,
        services: &AppServices,
        service: &mut Service<'_>,
        fetch: Fetch,
        enabled: bool,
        limits: Limits,
    ) -> Result<Loading, EditorError> {
        if service.request_uri(&fetch.ticket)? != fetch.uri || !self.is_current() {
            let _ = service.cancel_load(&fetch.ticket);
            return Err(EditorError::StaleRevision);
        }
        if limits.max_bytes == 0
            || limits.max_bytes.checked_add(1).is_none()
            || limits.timeout.is_zero()
            || self.resource.as_str().encode_utf16().count() > limits.uri.max_input_units
            || fetch
                .referrer
                .as_ref()
                .is_some_and(|referrer| referrer.len() > limits.uri.max_input_units)
        {
            let _ = service.cancel_load(&fetch.ticket);
            return Err(EditorError::Capacity);
        }
        if enabled {
            if let Err(error) = json_schema_load::request_url(&fetch.uri, limits.uri) {
                let _ = service.cancel_load(&fetch.ticket);
                return Err(error);
            }
        }
        if self
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                count
                    .checked_add(1)
                    .filter(|count| *count <= limits.max_active_requests)
            })
            .is_err()
        {
            let _ = service.cancel_load(&fetch.ticket);
            return Err(EditorError::Capacity);
        }
        let slot = Arc::new(Slot(self.active.clone()));
        let (sender, receiver) = oneshot::channel();
        let (cancel, mut cancelled) = watch::channel(false);
        let scope = self.clone();
        let request = fetch.clone();
        let tasks = services.tasks.clone();
        let worker_slot = slot.clone();
        let started = services.tasks.spawn_transient(WORKER_NAME, async move {
            let _slot = worker_slot.clone();
            let payload = if !enabled {
                Payload::Unavailable
            } else {
                tokio::select! {
                    biased;
                    _ = cancelled.changed() => return,
                    result = tokio::time::timeout(limits.timeout, scope.load(tasks, request, limits, worker_slot)) => {
                        match result {
                            Ok(Ok(text)) => Payload::Text(text),
                            Ok(Err(error)) => Payload::Failed(error),
                            Err(_) => Payload::Failed(TIMED_OUT),
                        }
                    }
                }
            };
            if !*cancelled.borrow() && !sender.is_closed() {
                drop(sender.send(payload));
            }
        });
        if !started {
            let _ = service.cancel_load(&fetch.ticket);
            return Err(EditorError::StaleRevision);
        }
        Ok(Loading {
            scope: self.clone(),
            fetch,
            receiver: Some(receiver),
            cancel,
            _slot: slot,
        })
    }

    async fn load(
        self,
        tasks: TaskSupervisor,
        fetch: Fetch,
        limits: Limits,
        slot: Arc<Slot>,
    ) -> Result<String, &'static str> {
        if !self.is_current() {
            return Err(DENIED);
        }
        let uri = json_schema_load::request_url(&fetch.uri, limits.uri).map_err(|_| DENIED)?;
        let uri = String::from_utf16(&uri).map_err(|_| DENIED)?;
        let uri = Url::parse(&uri).map_err(|_| DENIED)?;
        if !uri.username().is_empty() || uri.password().is_some() {
            return Err(DENIED);
        }
        if uri.scheme() == "file" {
            let referrer = fetch
                .referrer
                .as_deref()
                .map(String::from_utf16)
                .transpose()
                .map_err(|_| DENIED)?;
            let referrer = referrer
                .as_deref()
                .map(Url::parse)
                .transpose()
                .map_err(|_| DENIED)?
                .unwrap_or_else(|| self.resource.clone());
            let referrer = if referrer.scheme() == "schemaservice" {
                self.resource.clone()
            } else {
                referrer
            };
            if referrer.scheme() != "file" || self.resource.scheme() != "file" {
                return Err(DENIED);
            }
            let path = uri.to_file_path().map_err(|_| DENIED)?;
            let referrer = referrer.to_file_path().map_err(|_| DENIED)?;
            let resource = self.resource.to_file_path().map_err(|_| DENIED)?;
            return tasks
                .run_blocking_result(FILE_WORKER_NAME, move || {
                    let _slot = slot;
                    if !self.is_current() {
                        return Err(AppError::Forbidden(DENIED.into()));
                    }
                    root_guard::ensure_within_root(&self.root, &resource)?;
                    root_guard::ensure_within_root(&self.root, &referrer)?;
                    let path = root_guard::ensure_within_root(&self.root, &path)?;
                    let root = self.root.canonicalize()?;
                    let anchor = crate::preview_web_file::Anchor::open(root)?;
                    let file = anchor.file(&path)?;
                    if file.metadata()?.len() > limits.max_bytes as u64 {
                        return Err(AppError::InvalidArgument(TOO_LARGE.into()));
                    }
                    let mut bytes = Vec::new();
                    file.take((limits.max_bytes + 1) as u64)
                        .read_to_end(&mut bytes)?;
                    if bytes.len() > limits.max_bytes {
                        return Err(AppError::InvalidArgument(TOO_LARGE.into()));
                    }
                    anchor.check()?;
                    if !self.is_current() {
                        return Err(AppError::Forbidden(DENIED.into()));
                    }
                    Ok(String::from_utf8_lossy(&bytes).into_owned())
                })
                .await
                .map_err(|error| match error {
                    AppError::InvalidArgument(message) if message == TOO_LARGE => TOO_LARGE,
                    error if error.kind() == AppErrorKind::Forbidden => DENIED,
                    _ => FAILED,
                });
        }
        if !matches!(uri.scheme(), "http" | "https") {
            return Err(DENIED);
        }
        let mut response =
            taide_infra::http::outbound_http_client(taide_infra::http::HttpClientProfile::Api)
                .get(uri)
                .send()
                .await
                .map_err(|_| FAILED)?;
        if response
            .content_length()
            .is_some_and(|length| length > limits.max_bytes as u64)
        {
            return Err(TOO_LARGE);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| FAILED)? {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|length| length > limits.max_bytes)
            {
                return Err(TOO_LARGE);
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

impl Loading {
    pub fn try_publish(
        &mut self,
        service: &mut Service<'_>,
        limits: json_schema_load::Limits,
    ) -> Option<Result<(), EditorError>> {
        let receiver = self.receiver.as_mut()?;
        if !self.scope.is_current() {
            self.cancel.send_replace(true);
            self.receiver.take();
            let _ = service.cancel_load(&self.fetch.ticket);
            return Some(Err(EditorError::StaleRevision));
        }
        let payload = match receiver.try_recv() {
            Ok(payload) => payload,
            Err(oneshot::error::TryRecvError::Empty) => return None,
            Err(oneshot::error::TryRecvError::Closed) => {
                self.receiver.take();
                let _ = service.cancel_load(&self.fetch.ticket);
                return Some(Err(EditorError::StaleRevision));
            }
        };
        self.receiver.take();
        let message;
        let response = match &payload {
            Payload::Unavailable => Response::Unavailable,
            Payload::Text(text) => Response::Text(text),
            Payload::Failed(error) => {
                message = error.encode_utf16().collect::<Vec<_>>();
                Response::Failed(&message)
            }
        };
        let result = json_schema_load::publish_response(
            service,
            &self.fetch.ticket,
            response,
            cfg!(windows),
            limits,
        );
        if result.is_err() {
            let _ = service.cancel_load(&self.fetch.ticket);
        }
        Some(result)
    }

    pub fn cancel(mut self, service: &mut Service<'_>) -> Result<(), EditorError> {
        self.cancel.send_replace(true);
        self.receiver.take();
        service.cancel_load(&self.fetch.ticket)
    }
}

impl Drop for Loading {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[cfg(test)]
#[path = "editor-schema-transport-tests.rs"]
mod tests;

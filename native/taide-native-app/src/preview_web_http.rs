use std::{
    collections::HashMap,
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};

use bytes::Bytes;
use futures_util::stream;
use http::{Method, Request, Response, StatusCode, header};
use http_body_util::{BodyExt, Empty, StreamBody, combinators::UnsyncBoxBody};
use hyper::{
    body::{Frame, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use taide_model::{
    error::{AppError, AppResult},
    ids::ProjectId,
};
use taide_runtime::{TaskOperationLease, TaskSupervisor};
use tokio::{
    net::TcpListener,
    sync::{Notify, OwnedSemaphorePermit, Semaphore, watch},
    task::JoinHandle,
};
use url::Url;

use crate::{
    preview::invalid,
    preview_web_document::{HOST, RESOURCE_POLICY, SCHEME, validate_source},
    preview_web_io::TimedIo,
    preview_web_published::{Budget, Document, MAX_DOCUMENT_BYTES},
    preview_web_range::Selection,
    preview_web_resource::{CHUNK_BYTES, Scope},
};

const HEADER_BYTES: usize = 32 * 1024;
const MAX_HEADERS: usize = 64;
const MEDIA_DOCUMENT_PATH: &str = "/__taide_media_document";

type HttpBody = UnsyncBoxBody<Bytes, AppError>;

#[derive(Clone, Copy)]
pub struct Limits {
    pub connections: usize,
    pub sources: usize,
    pub header_timeout: Duration,
    pub idle_timeout: Duration,
}

struct State {
    tasks: TaskSupervisor,
    address: SocketAddr,
    limits: Limits,
    endpoints: Mutex<HashMap<String, Weak<Endpoint>>>,
    slots: Arc<Semaphore>,
    drained: Arc<Notify>,
    stop: watch::Sender<bool>,
    documents: Arc<Budget>,
}

struct Endpoint {
    scope: Scope,
    closed: watch::Sender<bool>,
    document: Mutex<Option<Arc<Document>>>,
}

impl Endpoint {
    async fn retired(&self) {
        let mut closed = self.closed.subscribe();
        tokio::select! {
            _ = closed.wait_for(|value| *value) => {},
            _ = self.scope.closed() => {},
        }
    }
}

pub struct Ticket {
    endpoint: Arc<Endpoint>,
    state: Weak<State>,
    id: String,
    address: SocketAddr,
}

impl Drop for Ticket {
    fn drop(&mut self) {
        self.endpoint.closed.send_replace(true);
        if let Some(state) = self.state.upgrade()
            && let Ok(mut endpoints) = state.endpoints.lock()
        {
            endpoints.remove(&self.id);
        }
    }
}

impl Ticket {
    pub(crate) fn publish(&self, html: Arc<String>) -> AppResult<()> {
        let source = self.endpoint.scope.source_url()?;
        self.publish_at(source.path().into(), html)
    }

    pub(crate) fn publish_media(&self, html: Arc<String>) -> AppResult<Url> {
        self.publish_at(MEDIA_DOCUMENT_PATH.into(), html)?;
        let source = Url::parse(&format!("{SCHEME}://{HOST}{MEDIA_DOCUMENT_PATH}"))
            .map_err(|_| invalid("preview media document URL is invalid"))?;
        self.url(&source)
    }

    fn publish_at(&self, path: String, html: Arc<String>) -> AppResult<()> {
        let state = self
            .state
            .upgrade()
            .filter(|state| !*state.stop.borrow())
            .ok_or_else(|| AppError::Forbidden("preview listener is closed".into()))?;
        if *self.endpoint.closed.borrow() || self.endpoint.scope.is_closed() {
            return Err(AppError::Forbidden("preview ticket is closed".into()));
        }
        let mut slot = self
            .endpoint
            .document
            .lock()
            .map_err(|_| invalid("preview document slot is unavailable"))?;
        if slot.is_some() {
            return Err(invalid("preview document is already published"));
        }
        *slot = Some(Arc::new(Document::new(path, html, &state.documents)?));
        Ok(())
    }

    pub fn url(&self, source: &Url) -> AppResult<Url> {
        validate_source(source)?;
        if *self.endpoint.closed.borrow() || self.endpoint.scope.is_closed() {
            return Err(AppError::Forbidden("preview ticket is closed".into()));
        }
        Url::parse(&format!(
            "http://{}/{}{}",
            self.address,
            self.id,
            source.path()
        ))
        .map_err(|_| invalid("preview resource URL is invalid"))
    }
}

pub struct Server {
    state: Arc<State>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Server {
    pub async fn start(tasks: TaskSupervisor, limits: Limits) -> AppResult<Self> {
        if limits.connections == 0
            || limits.connections > Semaphore::MAX_PERMITS
            || limits.sources == 0
        {
            return Err(invalid("preview HTTP limits are invalid"));
        }
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .map_err(|error| {
                AppError::Internal(format!(
                    "preview listener could not bind ({:?})",
                    error.kind()
                ))
            })?;
        let state = Arc::new(State {
            address: listener
                .local_addr()
                .map_err(|_| invalid("preview listener address is unavailable"))?,
            slots: Arc::new(Semaphore::new(limits.connections)),
            drained: Arc::new(Notify::new()),
            stop: watch::channel(false).0,
            tasks: tasks.clone(),
            limits,
            endpoints: Mutex::new(HashMap::new()),
            documents: Budget::new(MAX_DOCUMENT_BYTES),
        });
        let owned = state.clone();
        let worker = tasks
            .spawn_transient_handle("native-web-listener", async move {
                listen(listener, owned).await;
            })
            .ok_or_else(|| AppError::Forbidden("preview listener is shutting down".into()))?;
        Ok(Self {
            state,
            worker: Some(worker),
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.state.address
    }

    pub fn active_connections(&self) -> usize {
        self.state.limits.connections - self.state.slots.available_permits()
    }

    pub fn register(&self, scope: Scope) -> AppResult<Ticket> {
        if *self.state.stop.borrow() || scope.is_closed() {
            return Err(AppError::Forbidden(
                "preview listener or source is closed".into(),
            ));
        }
        let mut endpoints = self
            .state
            .endpoints
            .lock()
            .map_err(|_| invalid("preview registry is unavailable"))?;
        endpoints.retain(|_, endpoint| endpoint.strong_count() > 0);
        if endpoints.len() >= self.state.limits.sources {
            return Err(invalid("preview source registry is full"));
        }
        let id = ProjectId::new().to_string();
        let endpoint = Arc::new(Endpoint {
            scope,
            closed: watch::channel(false).0,
            document: Mutex::new(None),
        });
        endpoints.insert(id.clone(), Arc::downgrade(&endpoint));
        Ok(Ticket {
            endpoint,
            state: Arc::downgrade(&self.state),
            id,
            address: self.state.address,
        })
    }

    fn stop(&self) {
        self.state.stop.send_replace(true);
        if let Ok(mut endpoints) = self.state.endpoints.lock() {
            for endpoint in endpoints.values().filter_map(Weak::upgrade) {
                endpoint.closed.send_replace(true);
            }
            endpoints.clear();
        }
    }

    pub async fn shutdown(mut self) -> AppResult<()> {
        self.stop();
        if let Some(worker) = self.worker.take() {
            worker
                .await
                .map_err(|_| AppError::Internal("preview listener worker did not finish".into()))?;
        }
        Ok(())
    }
}

struct ConnectionGuard {
    permit: Option<OwnedSemaphorePermit>,
    operation: Option<TaskOperationLease>,
    drained: Arc<Notify>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        drop(self.permit.take());
        drop(self.operation.take());
        self.drained.notify_one();
    }
}

struct Connection {
    _guard: ConnectionGuard,
    selected: watch::Sender<Option<Arc<Endpoint>>>,
}

async fn listen(listener: TcpListener, state: Arc<State>) {
    let mut stopped = state.stop.subscribe();
    let mut workers = Vec::<JoinHandle<()>>::new();
    loop {
        let accepted = tokio::select! {
            biased;
            _ = stopped.wait_for(|value| *value) => break,
            accepted = listener.accept() => accepted,
        };
        let Ok((socket, _peer)) = accepted else { break };
        if *state.stop.borrow() {
            break;
        }
        let Ok(permit) = state.slots.clone().try_acquire_owned() else {
            continue;
        };
        let Some(operation) = state.tasks.begin_operation("native-web-connection") else {
            break;
        };
        let context = Arc::new(Connection {
            _guard: ConnectionGuard {
                permit: Some(permit),
                operation: Some(operation),
                drained: state.drained.clone(),
            },
            selected: watch::channel(None).0,
        });
        let owned = state.clone();
        workers.retain(|worker| !worker.is_finished());
        let worker = state.tasks.spawn_transient_handle("native-web-connection", async move {
            let mut selected = context.selected.subscribe();
            let retire = async {
                let endpoint = selected.wait_for(Option::is_some).await.ok().and_then(|value| value.clone());
                if let Some(endpoint) = endpoint { endpoint.retired().await; }
            };
            let service_context = context.clone();
            let service_state = owned.clone();
            let service = service_fn(move |request| {
                let context = service_context.clone();
                let state = service_state.clone();
                async move { Ok::<_, Infallible>(route(request, state, context).await) }
            });
            let mut builder = http1::Builder::new();
            builder.keep_alive(false).max_headers(MAX_HEADERS).max_buf_size(HEADER_BYTES)
                .timer(TokioTimer::new()).header_read_timeout(owned.limits.header_timeout).writev(false);
            tokio::select! {
                biased;
                _ = retire => {},
                _ = builder.serve_connection(TokioIo::new(TimedIo::new(socket, owned.limits.idle_timeout)), service) => {},
            }
        });
        if let Some(worker) = worker {
            workers.push(worker);
        }
    }
    drop(listener);
    for worker in &workers {
        worker.abort();
    }
    for worker in workers {
        let _result = worker.await;
    }
    while state.slots.available_permits() != state.limits.connections {
        state.drained.notified().await;
    }
}

fn empty() -> HttpBody {
    Empty::<Bytes>::new()
        .map_err(|error| match error {})
        .boxed_unsync()
}

fn response(
    status: StatusCode,
    body: HttpBody,
    mime: &str,
    length: u64,
    range: Option<String>,
) -> Response<HttpBody> {
    let mut builder = Response::builder().status(status)
        .header(header::CONTENT_TYPE, mime).header(header::CONTENT_LENGTH, length)
        .header(header::CACHE_CONTROL, "no-store").header("Referrer-Policy", "no-referrer")
        .header("X-Content-Type-Options", "nosniff").header("Cross-Origin-Resource-Policy", "same-origin")
        .header("Content-Security-Policy", format!("{RESOURCE_POLICY}; frame-ancestors 'none'; base-uri 'self'"))
        .header("Permissions-Policy", "camera=(), microphone=(), geolocation=(), display-capture=(), clipboard-read=(), clipboard-write=()")
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(range) = range {
        builder = builder.header(header::CONTENT_RANGE, range);
    }
    builder.body(body).unwrap_or_else(|_| {
        let mut failed = Response::new(empty());
        *failed.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        failed
    })
}

fn unavailable(status: StatusCode) -> Response<HttpBody> {
    response(status, empty(), "text/plain; charset=utf-8", 0, None)
}

async fn route(
    request: Request<Incoming>,
    state: Arc<State>,
    context: Arc<Connection>,
) -> Response<HttpBody> {
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(state.address.to_string().as_str())
        || request
            .uri()
            .authority()
            .is_some_and(|authority| authority.as_str() != state.address.to_string())
        || request
            .uri()
            .scheme_str()
            .is_some_and(|scheme| scheme != "http")
    {
        return unavailable(StatusCode::BAD_REQUEST);
    }
    if request.method() != Method::GET && request.method() != Method::HEAD {
        let mut response = unavailable(StatusCode::METHOD_NOT_ALLOWED);
        response
            .headers_mut()
            .insert(header::ALLOW, http::HeaderValue::from_static("GET, HEAD"));
        return response;
    }
    if request.headers().contains_key(header::TRANSFER_ENCODING)
        || request
            .headers()
            .get(header::CONTENT_LENGTH)
            .is_some_and(|value| value != "0")
    {
        return unavailable(StatusCode::BAD_REQUEST);
    }
    let Some((id, path)) = request
        .uri()
        .path()
        .strip_prefix('/')
        .and_then(|path| path.split_once('/'))
    else {
        return unavailable(StatusCode::NOT_FOUND);
    };
    let endpoint = match state.endpoints.lock() {
        Ok(endpoints) => endpoints.get(id).and_then(Weak::upgrade),
        Err(_) => return unavailable(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let Some(endpoint) =
        endpoint.filter(|endpoint| !*endpoint.closed.borrow() && !endpoint.scope.is_closed())
    else {
        return unavailable(StatusCode::NOT_FOUND);
    };
    context.selected.send_replace(Some(endpoint.clone()));
    let Ok(url) = Url::parse(&format!("{SCHEME}://{HOST}/{path}")) else {
        return unavailable(StatusCode::BAD_REQUEST);
    };
    let range = match request.headers().get(header::RANGE) {
        Some(value)
            if !request.headers().contains_key(header::IF_RANGE)
                && request.method() == Method::GET =>
        {
            value.to_str().ok().map(str::to_owned)
        }
        _ => None,
    };
    let document = match endpoint.document.lock() {
        Ok(slot) => slot
            .as_ref()
            .filter(|document| document.path == url.path())
            .cloned(),
        Err(_) => return unavailable(StatusCode::INTERNAL_SERVER_ERROR),
    };
    if let Some(document) = document {
        return document_response(
            document,
            endpoint,
            state,
            context,
            request.method() == Method::HEAD,
            range,
        )
        .await;
    }
    let scope = endpoint.scope.clone();
    let owned_context = context.clone();
    let resource = state
        .tasks
        .run_blocking_result("native-web-resource-open", move || {
            let _context = owned_context;
            scope.open(&url, range.as_deref())
        })
        .await;
    let Ok(mut resource) = resource else {
        return unavailable(StatusCode::FORBIDDEN);
    };
    let status = match resource.selection {
        Selection::Full => StatusCode::OK,
        Selection::Partial { .. } => StatusCode::PARTIAL_CONTENT,
        Selection::Unsatisfiable => StatusCode::RANGE_NOT_SATISFIABLE,
    };
    let length = resource.content_length();
    let range = resource.content_range();
    if request.method() == Method::HEAD || resource.body.is_none() {
        return response(status, empty(), resource.mime, length, range);
    }
    let stream = stream::unfold(
        resource
            .body
            .take()
            .map(|body| (body, state.tasks.clone(), context)),
        |current| async move {
            let (mut body, tasks, context) = current?;
            let owned_context = context.clone();
            let result = tasks
                .run_blocking_result("native-web-resource-chunk", move || {
                    let _context = owned_context;
                    let chunk = body.next_chunk()?;
                    Ok((body, chunk))
                })
                .await;
            match result {
                Ok((body, Some(chunk))) => Some((
                    Ok(Frame::data(Bytes::from(chunk))),
                    Some((body, tasks, context)),
                )),
                Ok((_, None)) => None,
                Err(_) => Some((Err(transfer_error()), None)),
            }
        },
    );
    response(
        status,
        StreamBody::new(stream).boxed_unsync(),
        resource.mime,
        length,
        range,
    )
}

fn transfer_error() -> AppError {
    AppError::Forbidden("preview resource transfer interrupted".into())
}

async fn document_response(
    document: Arc<Document>,
    endpoint: Arc<Endpoint>,
    state: Arc<State>,
    context: Arc<Connection>,
    is_head: bool,
    range: Option<String>,
) -> Response<HttpBody> {
    let scope = endpoint.scope.clone();
    let owned_context = context.clone();
    if state
        .tasks
        .run_blocking_result("native-web-document-check", move || {
            let _context = owned_context;
            scope.check()
        })
        .await
        .is_err()
    {
        return unavailable(StatusCode::FORBIDDEN);
    }
    let size = document.bytes.len() as u64;
    let selection = crate::preview_web_range::select(range.as_deref(), size);
    let metadata = crate::preview_web_resource::Response {
        mime: "text/html; charset=utf-8",
        size,
        selection,
        body: None,
    };
    let status = match selection {
        Selection::Full => StatusCode::OK,
        Selection::Partial { .. } => StatusCode::PARTIAL_CONTENT,
        Selection::Unsatisfiable => StatusCode::RANGE_NOT_SATISFIABLE,
    };
    let length = metadata.content_length();
    let range = metadata.content_range();
    if is_head || selection == Selection::Unsatisfiable {
        return response(status, empty(), metadata.mime, length, range);
    }
    let bytes = match selection {
        Selection::Partial { start, length } => document
            .bytes
            .slice(start as usize..(start + length) as usize),
        _ => document.bytes.clone(),
    };
    let stream = stream::unfold(
        Some((bytes, state.tasks.clone(), context, endpoint)),
        |current| async move {
            let (bytes, tasks, context, endpoint) = current?;
            if bytes.is_empty() {
                return None;
            }
            let scope = endpoint.scope.clone();
            let owned_context = context.clone();
            if tasks
                .run_blocking_result("native-web-document-chunk", move || {
                    let _context = owned_context;
                    scope.check()
                })
                .await
                .is_err()
            {
                return Some((Err(transfer_error()), None));
            }
            let end = bytes.len().min(CHUNK_BYTES);
            let chunk = bytes.slice(..end);
            let remaining = bytes.slice(end..);
            Some((
                Ok(Frame::data(chunk)),
                Some((remaining, tasks, context, endpoint)),
            ))
        },
    );
    response(
        status,
        StreamBody::new(stream).boxed_unsync(),
        metadata.mime,
        length,
        range,
    )
}

use super::*;

use std::sync::Arc;

use taide_model::{app_event::AppEvent, paths::AppPaths, project::Project};
use taide_native_editor::json_schema_graph::{self, Job, Progress, Start};
use taide_native_editor::json_schema_service::{LoadState, ResolveState};
use taide_native_syntax::JsonSchemaPatternCompiler;
use taide_runtime::EventSink;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

include!("../../taide-native-editor/tests/fixtures/json-schema-service-limits.rs");

const DEADLINE: Duration = Duration::from_secs(5);
const BYTE_LIMIT: usize = 4096;
const STEP_LIMIT: usize = 32;
const REQUEST_TIMEOUT: Duration = Duration::from_millis(20);
const TRANSPORT_LIMITS: Limits = Limits {
    uri: URI_LIMITS,
    max_bytes: BYTE_LIMIT,
    max_active_requests: 4,
    timeout: DEADLINE,
};
const LOAD_LIMITS: json_schema_load::Limits = json_schema_load::Limits {
    scanner: VALUE_LIMITS.parser.scanner,
    uri: URI_LIMITS,
    max_values: VALUE_LIMITS.max_values,
    max_depth: VALUE_LIMITS.parser.max_depth,
    max_text_units: VALUE_LIMITS.max_text_units,
    max_work: 1024 * 1024,
    max_messages: 16,
    max_message_units: 1024 * 1024,
};
const GRAPH_LIMITS: json_schema_graph::Limits = json_schema_graph::Limits {
    refs: REF_LIMITS,
    uri: URI_LIMITS,
    max_documents: 128,
    max_requests: 4096,
    max_uri_units: 1024 * 1024,
};

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    services: Arc<AppServices>,
    project: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("taide-schema-{}", ProjectId::new()));
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "owned schema fixture".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        );
        Self {
            directory,
            root,
            services,
            project,
        }
    }

    fn file(&self, name: &str, content: &[u8]) -> String {
        let path = self.root.join(name);
        std::fs::write(&path, content).unwrap();
        Url::from_file_path(path).unwrap().to_string()
    }

    fn scope(&self, resource: &str) -> Scope {
        Scope::new(self.services.state.clone(), self.project.clone(), resource).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

fn fetch(service: &mut Service<'_>, uri: &str, referrer: Option<&str>) -> Fetch {
    let uri = units(uri);
    let LoadState::Start(ticket) = service.begin_load(&uri).unwrap() else {
        panic!("load must be new");
    };
    Fetch {
        uri,
        referrer: referrer.map(units),
        ticket,
    }
}

async fn publish(loading: &mut Loading, service: &mut Service<'_>) -> Result<(), EditorError> {
    tokio::time::timeout(DEADLINE, async {
        loop {
            if let Some(result) = loading.try_publish(service, LOAD_LIMITS) {
                return result;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

async fn drained(fixture: &Fixture) {
    tokio::time::timeout(DEADLINE, async {
        while fixture.services.tasks.tracked_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn resolve(
    fixture: &Fixture,
    scope: &Scope,
    service: &mut Service<'_>,
    uri: &str,
) -> Arc<taide_native_editor::json_schema_refs::Resolved> {
    let Start::Job(mut job) = scope.begin(service, &units(uri), GRAPH_LIMITS).unwrap() else {
        panic!("graph must be new");
    };
    for _ in 0..STEP_LIMIT {
        match scope.advance(job, service).unwrap() {
            Progress::Ready(ready) => return ready,
            Progress::Waiting {
                job: next,
                requests,
            } => {
                assert!(!requests.is_empty());
                for fetch in requests {
                    if fetch.uri != units(uri) {
                        assert!(fetch.referrer.is_some());
                    }
                    let mut loading = scope
                        .start(&fixture.services, service, fetch, true, TRANSPORT_LIMITS)
                        .unwrap();
                    publish(&mut loading, service).await.unwrap();
                    assert!(loading.try_publish(service, LOAD_LIMITS).is_none());
                }
                job = next;
            }
        }
    }
    panic!("graph did not finish");
}

#[tokio::test]
async fn 프로젝트_파일을_실제_decode와_graph_cache에_게시하고_의존변경을_회수한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let root = fixture.file("schema.json", br#"{"$ref":"child.json#/$defs/value"}"#);
    let child = fixture.file("child.json", br#"{"$defs":{"value":{"type":"string"}}}"#);
    let scope = fixture.scope(&resource);
    let compiler = JsonSchemaPatternCompiler;
    let mut service = Service::new(&compiler, LIMITS);
    let ready = resolve(&fixture, &scope, &mut service, &root).await;
    assert!(ready.errors.is_empty());
    let node = ready.schema.root().unwrap();
    let node = ready.schema.member(node, &units("type")).unwrap();
    assert!(
        matches!(ready.schema.get(node), Some(taide_native_editor::json_value::Value::String(value)) if value == &units("string"))
    );
    let Start::Ready(cached) = Job::start(&mut service, &units(&root), true, GRAPH_LIMITS).unwrap()
    else {
        panic!("resolved graph must be cached");
    };
    assert!(Arc::ptr_eq(&ready, &cached));
    assert!(service.on_resource_change(&units(&child)).unwrap());
    assert!(matches!(
        service.begin_resolve(&units(&root)).unwrap(),
        ResolveState::Start {
            source: LoadState::Start(_),
            ..
        }
    ));
    drained(&fixture).await;
}

#[tokio::test]
async fn 원격_참조의_file_확대와_root_밖_파일_및_지원하지_않는_scheme을_거절한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let secret_free_schema = fixture.file("local.json", br#"{"type":"string"}"#);
    let outside = fixture.directory.join("outside.json");
    std::fs::write(&outside, b"{}").unwrap();
    let outside = Url::from_file_path(outside).unwrap().to_string();
    let compiler = JsonSchemaPatternCompiler;
    for (uri, referrer) in [
        (
            secret_free_schema.as_str(),
            Some("https://example.invalid/schema"),
        ),
        (outside.as_str(), None),
        ("data:application/json,{}", None),
        ("https://user:password@example.invalid/schema", None),
    ] {
        let mut service = Service::new(&compiler, LIMITS);
        let request = fetch(&mut service, uri, referrer);
        let ticket = request.ticket.clone();
        let mut loading = fixture
            .scope(&resource)
            .start(
                &fixture.services,
                &mut service,
                request,
                true,
                TRANSPORT_LIMITS,
            )
            .unwrap();
        publish(&mut loading, &mut service).await.unwrap();
        let loaded = service.read_unresolved(&ticket).unwrap().unwrap();
        assert_eq!(loaded.errors.len(), 1);
        assert!(
            String::from_utf16_lossy(&loaded.errors[0]).contains(DENIED),
            "{uri}: {:?}",
            loaded.errors
        );
    }
    drained(&fixture).await;
}

#[tokio::test]
async fn 요청_비활성은_원본_unavailable을_게시하고_파일_크기상한을_검사한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let uri = fixture.file("schema.json", &vec![b' '; BYTE_LIMIT + 1]);
    let compiler = JsonSchemaPatternCompiler;
    for enabled in [false, true] {
        let mut service = Service::new(&compiler, LIMITS);
        let request = fetch(&mut service, &uri, None);
        let ticket = request.ticket.clone();
        let mut loading = fixture
            .scope(&resource)
            .start(
                &fixture.services,
                &mut service,
                request,
                enabled,
                TRANSPORT_LIMITS,
            )
            .unwrap();
        publish(&mut loading, &mut service).await.unwrap();
        let loaded = service.read_unresolved(&ticket).unwrap().unwrap();
        assert_eq!(loaded.errors.len(), 1);
        let expected = if enabled {
            TOO_LARGE
        } else {
            "No schema request service available"
        };
        assert!(
            String::from_utf16_lossy(&loaded.errors[0]).contains(expected),
            "{:?}",
            loaded.errors
        );
    }
    drained(&fixture).await;
}

async fn server(
    responses: Vec<(&'static str, Vec<u8>)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (headers, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).await.unwrap();
                request.push(byte[0]);
                assert!(request.len() < BYTE_LIMIT);
            }
            requests.push(String::from_utf8(request).unwrap());
            socket.write_all(headers.as_bytes()).await.unwrap();
            socket.write_all(&body).await.unwrap();
            socket.shutdown().await.unwrap();
        }
        requests
    });
    (address, task)
}

#[tokio::test]
async fn 실제_http의_오류_status_text_redirect와_utf8_replacement를_decode한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let (address, server) = server(vec![
        ("HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", vec![]),
        ("HTTP/1.1 404 Not Found\r\nContent-Length: 13\r\nConnection: close\r\n\r\n", b"{\"title\":\"\xff\"}".to_vec()),
    ]).await;
    let compiler = JsonSchemaPatternCompiler;
    let mut service = Service::new(&compiler, LIMITS);
    let uri = format!("{address}/root");
    let ready = resolve(&fixture, &fixture.scope(&resource), &mut service, &uri).await;
    let requests = tokio::time::timeout(DEADLINE, server)
        .await
        .unwrap()
        .unwrap();
    assert!(requests[0].starts_with("GET /root HTTP/1.1"));
    assert!(requests[1].starts_with("GET /final HTTP/1.1"));
    assert!(ready.errors.is_empty(), "{:?}", ready.errors);
    let node = ready
        .schema
        .member(ready.schema.root().unwrap(), &units("title"))
        .unwrap();
    assert!(
        matches!(ready.schema.get(node), Some(taide_native_editor::json_value::Value::String(value)) if value == &units("�"))
    );
    drained(&fixture).await;
}

#[tokio::test]
async fn 실제_http_content_length와_chunked_응답의_전체_byte_상한을_검사한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let mut chunked = format!("{:x}\r\n", BYTE_LIMIT + 1).into_bytes();
    chunked.extend(vec![b' '; BYTE_LIMIT + 1]);
    chunked.extend_from_slice(b"\r\n0\r\n\r\n");
    let (address, server) = server(vec![
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 4097\r\nConnection: close\r\n\r\n",
            vec![],
        ),
        (
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            chunked,
        ),
    ])
    .await;
    let compiler = JsonSchemaPatternCompiler;
    for path in ["length", "chunked"] {
        let mut service = Service::new(&compiler, LIMITS);
        let uri = format!("{address}/{path}");
        let ready = resolve(&fixture, &fixture.scope(&resource), &mut service, &uri).await;
        assert_eq!(ready.errors.len(), 1);
        assert!(String::from_utf16_lossy(&ready.errors[0]).contains(TOO_LARGE));
    }
    assert_eq!(
        tokio::time::timeout(DEADLINE, server)
            .await
            .unwrap()
            .unwrap()
            .len(),
        2
    );
    drained(&fixture).await;
}

#[tokio::test]
async fn 취소와_프로젝트_변경_종료_및_이전_ticket은_게시하지_않는다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let uri = fixture.file("schema.json", b"{}");
    let compiler = JsonSchemaPatternCompiler;
    for mode in ["cancel", "stale", "project", "shutdown"] {
        let mut service = Service::new(&compiler, LIMITS);
        let request = fetch(&mut service, &uri, None);
        let ticket = request.ticket.clone();
        let mut loading = fixture
            .scope(&resource)
            .start(
                &fixture.services,
                &mut service,
                request,
                true,
                TRANSPORT_LIMITS,
            )
            .unwrap();
        match mode {
            "cancel" => {
                loading.cancel(&mut service).unwrap();
                assert!(matches!(
                    service.begin_load(&units(&uri)).unwrap(),
                    LoadState::Start(_)
                ));
                continue;
            }
            "stale" => {
                service.on_resource_change(&units(&uri)).unwrap();
            }
            "project" => {
                fixture
                    .services
                    .state
                    .projects
                    .write()
                    .get_mut(&fixture.project)
                    .unwrap()
                    .root = fixture.directory.to_str().unwrap().into();
            }
            "shutdown" => fixture.services.state.begin_shutdown(),
            _ => unreachable!(),
        }
        assert_eq!(
            publish(&mut loading, &mut service).await,
            Err(EditorError::StaleRevision)
        );
        assert!(service.read_unresolved(&ticket).is_err());
        if mode == "project" {
            fixture
                .services
                .state
                .projects
                .write()
                .get_mut(&fixture.project)
                .unwrap()
                .root = fixture.root.to_str().unwrap().into();
        }
    }
    drained(&fixture).await;
}

#[tokio::test]
async fn 실제_원격_graph는_로컬_file_참조를_읽지_않고_요청실패로_해결한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let child = fixture.file("child.json", br#"{"title":"must not be read"}"#);
    let body = serde_json::to_vec(&serde_json::json!({"$ref": child})).unwrap();
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let uri = format!("http://{}/root", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; BYTE_LIMIT];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket.write_all(headers.as_bytes()).await.unwrap();
        socket.write_all(&body).await.unwrap();
        socket.shutdown().await.unwrap();
    });
    let compiler = JsonSchemaPatternCompiler;
    let mut service = Service::new(&compiler, LIMITS);
    let local = resolve(&fixture, &fixture.scope(&resource), &mut service, &child).await;
    assert!(local.errors.is_empty());
    assert!(matches!(
        fixture
            .scope("https://example.invalid/document.json")
            .begin(&mut service, &units(&child), GRAPH_LIMITS),
        Err(EditorError::InvalidBoundary)
    ));
    let ready = resolve(&fixture, &fixture.scope(&resource), &mut service, &uri).await;
    tokio::time::timeout(DEADLINE, server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ready.errors.len(), 1);
    assert!(String::from_utf16_lossy(&ready.errors[0]).contains("access denied"));
    assert!(
        ready
            .schema
            .member(ready.schema.root().unwrap(), &units("title"))
            .is_none()
    );
    drained(&fixture).await;
}

#[tokio::test]
async fn 지연된_http의_timeout_명시취소_및_supervisor_종료를_회수한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let compiler = JsonSchemaPatternCompiler;
    for mode in ["timeout", "cancel", "stop"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let uri = format!("http://{}/schema", listener.local_addr().unwrap());
        let mut service = Service::new(&compiler, LIMITS);
        let request = fetch(&mut service, &uri, None);
        let ticket = request.ticket.clone();
        let mut loading = fixture
            .scope(&resource)
            .start(
                &fixture.services,
                &mut service,
                request,
                true,
                Limits {
                    timeout: if mode == "timeout" {
                        REQUEST_TIMEOUT
                    } else {
                        DEADLINE
                    },
                    ..TRANSPORT_LIMITS
                },
            )
            .unwrap();
        let (mut socket, _) = tokio::time::timeout(DEADLINE, listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut request = vec![0; BYTE_LIMIT];
        assert!(
            tokio::time::timeout(DEADLINE, socket.read(&mut request))
                .await
                .unwrap()
                .unwrap()
                > 0
        );
        if mode == "cancel" {
            loading.cancel(&mut service).unwrap();
        } else {
            if mode == "stop" {
                fixture.services.tasks.stop_all();
            }
            let result = publish(&mut loading, &mut service).await;
            if mode == "timeout" {
                result.unwrap();
                let loaded = service.read_unresolved(&ticket).unwrap().unwrap();
                assert!(String::from_utf16_lossy(&loaded.errors[0]).contains(TIMED_OUT));
            } else {
                assert_eq!(result, Err(EditorError::StaleRevision));
            }
        }
        drained(&fixture).await;
        let mut byte = [0];
        assert_eq!(
            tokio::time::timeout(DEADLINE, socket.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        if mode != "timeout" {
            assert!(service.read_unresolved(&ticket).is_err());
        }
    }
}

#[tokio::test]
async fn 실제_file_match_합성_schema의_파일_참조를_프로젝트_권한으로_읽는다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let first = fixture.file("first-schema.json", br#"{"type":"string"}"#);
    let second = fixture.file("second-schema.json", br#"{"minLength":2}"#);
    let scope = fixture.scope(&resource);
    let compiler = JsonSchemaPatternCompiler;
    let mut service = Service::new(&compiler, LIMITS);
    for uri in [&first, &second] {
        service
            .register(&units(uri), None, &[units("*.json")], None, false)
            .unwrap();
    }
    let selected = service.select(&units(&resource), None).unwrap().unwrap();
    let selected = String::from_utf16(&selected).unwrap();
    assert!(selected.starts_with("schemaservice://combinedSchema/"));
    let ready = resolve(&fixture, &scope, &mut service, &selected).await;
    assert!(ready.errors.is_empty(), "{:?}", ready.errors);
    let node = ready
        .schema
        .member(ready.schema.root().unwrap(), &units("allOf"))
        .unwrap();
    let Some(taide_native_editor::json_value::Value::Array(items)) = ready.schema.get(node) else {
        panic!("combined schema must contain allOf");
    };
    assert_eq!(items.len(), 2);
    assert!(ready.schema.member(items[0], &units("type")).is_some());
    assert!(ready.schema.member(items[1], &units("minLength")).is_some());
    drained(&fixture).await;
}

#[tokio::test]
async fn 공유_scope의_응답_대기_상한과_decode_실패는_ticket을_회수한다() {
    let fixture = Fixture::new();
    let resource = fixture.file("document.json", b"{}");
    let scope = fixture.scope(&resource);
    let compiler = JsonSchemaPatternCompiler;
    let mut service = Service::new(&compiler, LIMITS);
    let first = fetch(&mut service, "https://example.invalid/first", None);
    let first_ticket = first.ticket.clone();
    let mut loading = scope
        .start(
            &fixture.services,
            &mut service,
            first,
            false,
            Limits {
                max_active_requests: 1,
                ..TRANSPORT_LIMITS
            },
        )
        .unwrap();
    let second = fetch(&mut service, "https://example.invalid/second", None);
    let second_ticket = second.ticket.clone();
    assert!(matches!(
        scope.clone().start(
            &fixture.services,
            &mut service,
            second,
            false,
            Limits {
                max_active_requests: 1,
                ..TRANSPORT_LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(service.read_unresolved(&second_ticket).is_err());
    tokio::time::timeout(DEADLINE, async {
        loop {
            if let Some(result) = loading.try_publish(
                &mut service,
                json_schema_load::Limits {
                    max_messages: 0,
                    ..LOAD_LIMITS
                },
            ) {
                assert_eq!(result, Err(EditorError::Capacity));
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(service.read_unresolved(&first_ticket).is_err());
    drop(loading);
    let second = fetch(&mut service, "https://example.invalid/second", None);
    let mut loading = scope
        .start(
            &fixture.services,
            &mut service,
            second,
            false,
            Limits {
                max_active_requests: 1,
                ..TRANSPORT_LIMITS
            },
        )
        .unwrap();
    publish(&mut loading, &mut service).await.unwrap();
    drop(loading);
    assert_eq!(scope.active.load(Ordering::Acquire), 0);
    drained(&fixture).await;
    let third = fetch(&mut service, "https://example.invalid/third", None);
    let loading = scope
        .start(
            &fixture.services,
            &mut service,
            third,
            false,
            Limits {
                max_active_requests: 1,
                ..TRANSPORT_LIMITS
            },
        )
        .unwrap();
    loading.cancel(&mut service).unwrap();
    let fourth = fetch(&mut service, "https://example.invalid/fourth", None);
    assert!(matches!(
        scope.start(
            &fixture.services,
            &mut service,
            fourth,
            false,
            Limits {
                max_active_requests: 1,
                ..TRANSPORT_LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    drained(&fixture).await;
    assert_eq!(scope.active.load(Ordering::Acquire), 0);
    let fourth = fetch(&mut service, "https://example.invalid/fourth", None);
    let mut loading = scope
        .start(
            &fixture.services,
            &mut service,
            fourth,
            false,
            Limits {
                max_active_requests: 1,
                ..TRANSPORT_LIMITS
            },
        )
        .unwrap();
    publish(&mut loading, &mut service).await.unwrap();
    drop(loading);
    drained(&fixture).await;
}

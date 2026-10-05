#![cfg(unix)]

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use taide_model::{app_event::AppEvent, ids::ProjectId, paths::AppPaths, project::Project};
use taide_native_app::{
    bootstrap::services,
    preview_web_document::source_url,
    preview_web_http::{Limits, Server},
    preview_web_resource::media_owner,
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use url::Url;

const TIMEOUT: Duration = Duration::from_secs(20);
const MAX_RESPONSE_BYTES: u64 = 512 * 1024;
const CONNECTIONS: usize = 2;
const SOURCES: usize = 1;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

async fn request(address: SocketAddr, url: &Url, method: &str, extra: &str) -> (String, Vec<u8>) {
    tokio::time::timeout(TIMEOUT, async {
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket
            .write_all(
                format!(
                    "{method} {} HTTP/1.1\r\nHost: {address}\r\n{extra}\r\n",
                    url.path()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut bytes = Vec::new();
        socket
            .take(MAX_RESPONSE_BYTES)
            .read_to_end(&mut bytes)
            .await
            .unwrap();
        let separator = bytes
            .windows(b"\r\n\r\n".len())
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let body = bytes.split_off(separator + b"\r\n\r\n".len());
        let headers = String::from_utf8(bytes[..separator].to_vec())
            .unwrap()
            .to_ascii_lowercase();
        (headers, body)
    })
    .await
    .unwrap()
}

async fn connections(server: &Server, count: usize) {
    tokio::time::timeout(TIMEOUT, async {
        while server.active_connections() != count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn web_http는_실제_status_header_range_ticket_owner와_server_종료를_검사한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-web-http-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("synthetic.mp4");
    std::fs::write(&source, b"0123456789").unwrap();
    let outside = fixture.0.join("outside.mp4");
    std::fs::write(&outside, b"outside synthetic bytes").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic HTTP".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state, tasks.clone(), Arc::new(Sink));
    let owner = media_owner(&services, source.to_str().unwrap().into())
        .await
        .unwrap();
    let server = Server::start(
        tasks.clone(),
        Limits {
            connections: CONNECTIONS,
            sources: SOURCES,
            header_timeout: TIMEOUT,
            idle_timeout: TIMEOUT,
        },
    )
    .await
    .unwrap();
    assert!(server.address().ip().is_loopback());
    let ticket = server.register(owner.scope()).unwrap();
    assert!(server.register(owner.scope()).is_err());
    let url = ticket.url(&source_url(&source).unwrap()).unwrap();
    let (headers, body) = request(server.address(), &url, "GET", "").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains("content-length: 10"));
    assert!(headers.contains("content-type: video/mp4"));
    assert!(headers.contains("accept-ranges: bytes"));
    assert!(headers.contains("cache-control: no-store"));
    assert!(headers.contains("referrer-policy: no-referrer"));
    assert!(headers.contains("x-content-type-options: nosniff"));
    assert!(headers.contains("cross-origin-resource-policy: same-origin"));
    assert!(headers.contains("content-security-policy:"));
    assert!(headers.contains("script-src 'none'"));
    assert!(headers.contains("connect-src 'none'"));
    assert!(headers.contains("frame-ancestors 'none'"));
    assert!(headers.contains("permissions-policy: camera=()"));
    assert!(!headers.contains("access-control-allow-origin"));
    assert!(!headers.contains("content-range:"));
    assert_eq!(body, b"0123456789");
    let (headers, body) = request(server.address(), &url, "GET", "Range: bytes=2-4\r\n").await;
    assert!(headers.starts_with("http/1.1 206"));
    assert!(headers.contains("content-range: bytes 2-4/10"));
    assert!(headers.contains("content-length: 3"));
    assert_eq!(body, b"234");
    let (headers, body) = request(server.address(), &url, "HEAD", "Range: bytes=2-4\r\n").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains("content-length: 10"));
    assert!(body.is_empty());
    let (headers, body) = request(server.address(), &url, "GET", "Range: bytes=10-\r\n").await;
    assert!(headers.starts_with("http/1.1 416"));
    assert!(headers.contains("content-range: bytes */10"));
    assert!(body.is_empty());
    let (headers, body) = request(
        server.address(),
        &url,
        "GET",
        "Range: bytes=2-4\r\nIf-Range: \"unknown\"\r\n",
    )
    .await;
    assert!(headers.starts_with("http/1.1 200"));
    assert_eq!(body, b"0123456789");
    let (headers, body) = request(server.address(), &url, "POST", "").await;
    assert!(headers.starts_with("http/1.1 405"));
    assert!(headers.contains("allow: get, head"));
    assert!(body.is_empty());
    let (headers, _) = request(server.address(), &url, "GET", "Content-Length: 1\r\n").await;
    assert!(headers.starts_with("http/1.1 400"));
    let (headers, _) = request(
        server.address(),
        &ticket.url(&source_url(&outside).unwrap()).unwrap(),
        "GET",
        "",
    )
    .await;
    assert!(headers.starts_with("http/1.1 403"));
    let address = server.address();
    let mut unknown = url.clone();
    unknown.set_path("/unknown-capability/file.mp4");
    let (headers, _) = request(address, &unknown, "GET", "").await;
    assert!(headers.starts_with("http/1.1 404"));
    drop(ticket);
    let (headers, _) = request(address, &url, "GET", "").await;
    assert!(headers.starts_with("http/1.1 404"));
    let ticket = server.register(owner.scope()).unwrap();
    let url = ticket.url(&source_url(&source).unwrap()).unwrap();
    drop(owner);
    let (headers, _) = request(address, &url, "GET", "").await;
    assert!(headers.starts_with("http/1.1 404"));
    connections(&server, 0).await;
    let parked = TcpStream::connect(address).await.unwrap();
    let second = TcpStream::connect(address).await.unwrap();
    connections(&server, CONNECTIONS).await;
    let mut rejected = TcpStream::connect(address).await.unwrap();
    let mut byte = [0; 1];
    assert_eq!(
        tokio::time::timeout(TIMEOUT, rejected.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    connections(&server, CONNECTIONS).await;
    tokio::time::timeout(TIMEOUT, server.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert!(TcpStream::connect(address).await.is_err());
    drop((parked, second, rejected, ticket));
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

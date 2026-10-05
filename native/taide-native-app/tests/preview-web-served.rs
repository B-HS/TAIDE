#![cfg(unix)]

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use taide_model::{app_event::AppEvent, ids::ProjectId, paths::AppPaths, project::Project};
use taide_native_app::{
    bootstrap::services,
    preview_web::{Prepared, Request},
    preview_web_host::{Bridge, Reply},
    preview_web_http::{Limits, Server},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use url::Url;

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const TIMEOUT: Duration = Duration::from_secs(20);
const RESPONSE_BYTES: u64 = 512 * 1024;
const CONNECTIONS: usize = 2;
const SOURCES: usize = 2;

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
        let target = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        socket
            .write_all(
                format!("{method} {target} HTTP/1.1\r\nHost: {address}\r\n{extra}\r\n").as_bytes(),
            )
            .await
            .unwrap();
        let mut bytes = Vec::new();
        socket
            .take(RESPONSE_BYTES)
            .read_to_end(&mut bytes)
            .await
            .unwrap();
        let separator = bytes
            .windows(b"\r\n\r\n".len())
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let body = bytes.split_off(separator + b"\r\n\r\n".len());
        (
            String::from_utf8(bytes[..separator].to_vec())
                .unwrap()
                .to_ascii_lowercase(),
            body,
        )
    })
    .await
    .unwrap()
}

async fn prepared(bridge: &mut Bridge, expected: &Request) -> Prepared {
    tokio::time::timeout(TIMEOUT, async {
        let mut source_ready = false;
        loop {
            match bridge.poll() {
                Some(Reply::SourceReady(request)) => {
                    assert_eq!(&request, expected);
                    assert!(!source_ready);
                    source_ready = true;
                }
                Some(Reply::Prepared { request, result }) => {
                    assert_eq!(&request, expected);
                    assert!(source_ready);
                    return result.unwrap();
                }
                None => tokio::task::yield_now().await,
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn web_servedは_実際の_host_helper_base_document_assetと_失効を接続する() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-web-served-{}", ProjectId::new())));
    let root = fixture.0.join("synthetic project");
    let pages = root.join("pages");
    let assets = root.join("assets");
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::create_dir_all(&assets).unwrap();
    let source = pages.join("synthetic # ?.html");
    std::fs::write(&source, b"\xef\xbb\xbf<base href='../assets/'><link rel='stylesheet' href='synthetic.css?q=1'><script>notExecuted()</script><p>synthetic served document").unwrap();
    std::fs::write(assets.join("synthetic.css"), b"p { color: green }").unwrap();
    std::fs::write(pages.join("other.html"), b"<p>not published").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic served".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state, tasks.clone(), Arc::new(Sink));
    let server = Arc::new(
        Server::start(
            tasks.clone(),
            Limits {
                connections: CONNECTIONS,
                sources: SOURCES,
                header_timeout: TIMEOUT,
                idle_timeout: TIMEOUT,
            },
        )
        .await
        .unwrap(),
    );
    let mut bridge = Bridge::connect_served(
        services,
        Arc::new(|| {}),
        PathBuf::from(EXECUTABLE),
        TIMEOUT,
        server.clone(),
    )
    .unwrap();
    let command = Request {
        path: source.to_str().unwrap().into(),
        token: 1,
    };
    bridge.submit(command.clone()).unwrap();
    let document = prepared(&mut bridge, &command).await;
    assert_eq!(document.source.scheme(), "http");
    assert_eq!(document.source.host_str(), Some("127.0.0.1"));
    assert!(document.ticket.is_some());
    let (headers, body) = request(server.address(), &document.source, "GET", "").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains("content-type: text/html; charset=utf-8"));
    assert!(headers.contains("content-security-policy: default-src 'none'; script-src 'none'"));
    assert_eq!(body, document.html.as_bytes());
    let parsed = dom_query::Document::from(std::str::from_utf8(&body).unwrap());
    let base = Url::parse(parsed.select_single("base").attr("href").unwrap().as_ref()).unwrap();
    let css = base
        .join(parsed.select_single("link").attr("href").unwrap().as_ref())
        .unwrap();
    let (headers, body) = request(server.address(), &css, "GET", "").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains("content-type: text/css; charset=utf-8"));
    assert_eq!(body, b"p { color: green }");
    let (headers, body) = request(server.address(), &document.source, "HEAD", "").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains(&format!("content-length: {}", document.html.len())));
    assert!(body.is_empty());
    let (headers, body) = request(
        server.address(),
        &document.source,
        "GET",
        "Range: bytes=0-0\r\n",
    )
    .await;
    assert!(headers.starts_with("http/1.1 206"));
    assert_eq!(body, &document.html.as_bytes()[..1]);
    let other = document.source.join("other.html").unwrap();
    assert!(
        request(server.address(), &other, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    std::fs::write(&source, b"<p>changed synthetic source").unwrap();
    assert!(
        request(server.address(), &document.source, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    let retired = document.source.clone();
    drop(document);
    assert!(
        request(server.address(), &retired, "GET", "")
            .await
            .0
            .starts_with("http/1.1 404")
    );
    bridge.disconnect().await.unwrap();
    Arc::try_unwrap(server)
        .ok()
        .unwrap()
        .shutdown()
        .await
        .unwrap();
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
}

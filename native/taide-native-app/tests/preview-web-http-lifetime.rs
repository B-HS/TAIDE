#![cfg(unix)]

use std::{path::PathBuf, sync::Arc, time::Duration};

use taide_model::{app_event::AppEvent, ids::ProjectId, paths::AppPaths, project::Project};
use taide_native_app::{
    bootstrap::services,
    preview_web_document::source_url,
    preview_web_http::{Limits, Server},
    preview_web_resource::{CHUNK_BYTES, media_owner},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const TIMEOUT: Duration = Duration::from_secs(20);
const FILE_BYTES: u64 = 1024 * 1024 * 1024;
const HEADER_BYTES: usize = 32 * 1024;
const LIMITS: Limits = Limits {
    connections: 2,
    sources: 2,
    header_timeout: TIMEOUT,
    idle_timeout: TIMEOUT,
};

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

async fn headers(socket: &mut TcpStream) -> usize {
    let mut bytes = Vec::new();
    let mut chunk = [0; HEADER_BYTES];
    loop {
        let count = socket.read(&mut chunk).await.unwrap();
        assert!(count > 0);
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(separator) = bytes
            .windows(b"\r\n\r\n".len())
            .position(|bytes| bytes == b"\r\n\r\n")
        {
            let headers = String::from_utf8(bytes[..separator].to_vec())
                .unwrap()
                .to_ascii_lowercase();
            assert!(headers.starts_with("http/1.1 200"));
            assert!(headers.contains(&format!("content-length: {FILE_BYTES}")));
            return bytes.len() - separator - b"\r\n\r\n".len();
        }
        assert!(bytes.len() < HEADER_BYTES);
    }
}

async fn truncated(socket: &mut TcpStream, initial: usize) {
    let mut total = initial as u64;
    let mut chunk = [0; CHUNK_BYTES];
    loop {
        match socket.read(&mut chunk).await {
            Ok(0) => break,
            Ok(count) => total += count as u64,
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
            Err(error) => panic!("unexpected owned connection error: {error}"),
        }
        assert!(total < FILE_BYTES);
    }
    assert!(total < FILE_BYTES);
}

#[tokio::test(flavor = "multi_thread")]
async fn web_http_lifetime은_진행중_ticket_owner_취소와_root_shutdown을_회수한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-web-http-life-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("large.mp4");
    std::fs::File::create(&source)
        .unwrap()
        .set_len(FILE_BYTES)
        .unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic lifetime".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
    let owner = media_owner(&services, source.to_str().unwrap().into())
        .await
        .unwrap();
    let server = Server::start(tasks.clone(), LIMITS).await.unwrap();
    let address = server.address();
    let ticket = server.register(owner.scope()).unwrap();
    let url = ticket.url(&source_url(&source).unwrap()).unwrap();
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(format!("GET {} HTTP/1.1\r\nHost: {address}\r\n\r\n", url.path()).as_bytes())
        .await
        .unwrap();
    let initial = tokio::time::timeout(TIMEOUT, headers(&mut socket))
        .await
        .unwrap();
    drop(ticket);
    tokio::time::timeout(TIMEOUT, truncated(&mut socket, initial))
        .await
        .unwrap();
    let ticket = server.register(owner.scope()).unwrap();
    let url = ticket.url(&source_url(&source).unwrap()).unwrap();
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(format!("GET {} HTTP/1.1\r\nHost: {address}\r\n\r\n", url.path()).as_bytes())
        .await
        .unwrap();
    let initial = tokio::time::timeout(TIMEOUT, headers(&mut socket))
        .await
        .unwrap();
    drop(owner);
    tokio::time::timeout(TIMEOUT, truncated(&mut socket, initial))
        .await
        .unwrap();
    tokio::time::timeout(TIMEOUT, server.shutdown())
        .await
        .unwrap()
        .unwrap();

    let idle = Server::start(
        tasks.clone(),
        Limits {
            idle_timeout: Duration::ZERO,
            ..LIMITS
        },
    )
    .await
    .unwrap();
    let mut socket = TcpStream::connect(idle.address()).await.unwrap();
    let mut byte = [0; 1];
    assert_eq!(
        tokio::time::timeout(TIMEOUT, socket.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    tokio::time::timeout(TIMEOUT, idle.shutdown())
        .await
        .unwrap()
        .unwrap();

    let server = Server::start(tasks.clone(), LIMITS).await.unwrap();
    let address = server.address();
    let _parked = TcpStream::connect(address).await.unwrap();
    state.begin_shutdown();
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(TcpStream::connect(address).await.is_err());
    drop((server, ticket, socket));
}

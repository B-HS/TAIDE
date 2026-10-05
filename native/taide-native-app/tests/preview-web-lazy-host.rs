#![cfg(unix)]

use std::{path::PathBuf, sync::Arc, time::Duration};
use taide_model::{
    app_event::AppEvent,
    ids::{ProjectId, TabId},
    paths::AppPaths,
    project::Project,
};
use taide_native_app::{
    bootstrap::services,
    preview_web_cache::Cache,
    preview_web_host::{Bridge, Reply},
    preview_web_http::Limits,
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const TIMEOUT: Duration = Duration::from_secs(20);
const CONNECTIONS: usize = 2;
const SOURCES: usize = 2;
const RESPONSE_BYTES: u64 = 32 * 1024;

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

async fn reply(bridge: &mut Bridge) -> Reply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn web_lazy_hostは_待機中_listener無し_取消queue_実際helper_cacheと_終了を検査する() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-web-lazy-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("synthetic.html");
    std::fs::write(&source, b"<p>synthetic lazy source").unwrap();
    let path = source.to_str().unwrap().to_owned();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic lazy".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state, tasks.clone(), Arc::new(Sink));
    let mut bridge = Bridge::connect_lazy(
        services,
        Arc::new(|| {}),
        PathBuf::from(EXECUTABLE),
        TIMEOUT,
        Limits {
            connections: CONNECTIONS,
            sources: SOURCES,
            header_timeout: TIMEOUT,
            idle_timeout: TIMEOUT,
        },
    )
    .unwrap();
    assert_eq!(tasks.tracked_count(), 1);
    let mut cache = Cache::default();
    let cancelled = cache.begin(&path).unwrap();
    bridge.cancel_through(cancelled.token);
    bridge.submit(cancelled.clone()).unwrap();
    match reply(&mut bridge).await {
        Reply::Prepared { request, result } => {
            assert_eq!(request, cancelled);
            assert!(result.is_err());
        }
        Reply::SourceReady(_) => panic!("cancelled queue must not read or bind"),
    }
    assert_eq!(tasks.tracked_count(), 1);
    cache.invalidate(&path);
    let request = cache.begin(&path).unwrap();
    bridge.cancel_through(cancelled.token);
    bridge.submit(request.clone()).unwrap();
    match reply(&mut bridge).await {
        Reply::SourceReady(progress) => {
            assert_eq!(progress, request);
            cache.source_ready(&progress);
        }
        Reply::Prepared { .. } => panic!("source progress must precede prepared result"),
    }
    match reply(&mut bridge).await {
        Reply::Prepared {
            request: completed,
            result,
        } => {
            assert_eq!(completed, request);
            cache.accept(completed, result, 0);
        }
        Reply::SourceReady(_) => panic!("progress must occur once"),
    }
    let url = cache.source(&path).unwrap().clone();
    let address = std::net::SocketAddr::new(
        url.host_str().unwrap().parse().unwrap(),
        url.port().unwrap(),
    );
    assert!(cache.begin(&path).is_none());
    cache.reconcile(&std::collections::HashMap::from([(
        TabId::new(),
        path.clone(),
    )]));
    cache.renderer_failed(&path);
    assert_eq!(cache.retained_bytes(), 0);
    assert!(cache.source(&path).is_none());
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket
        .write_all(format!("GET {} HTTP/1.1\r\nHost: {address}\r\n\r\n", url.path()).as_bytes())
        .await
        .unwrap();
    let mut response = Vec::new();
    tokio::time::timeout(
        TIMEOUT,
        socket.take(RESPONSE_BYTES).read_to_end(&mut response),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(response.starts_with(b"HTTP/1.1 404"));
    bridge.disconnect().await.unwrap();
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
    assert!(TcpStream::connect(address).await.is_err());
}

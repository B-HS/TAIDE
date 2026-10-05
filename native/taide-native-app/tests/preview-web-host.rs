use std::{path::PathBuf, sync::Arc, time::Duration};

use taide_model::{app_event::AppEvent, ids::ProjectId, paths::AppPaths, project::Project};
use taide_native_app::{
    bootstrap::services,
    preview_web::Request,
    preview_web_host::{Bridge, Reply},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const TIMEOUT: Duration = Duration::from_secs(20);

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

async fn next(host: &mut Bridge, ready: &Notify) -> Reply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = host.poll() {
                return reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn html_host는_승인_progress_result_순서와_disconnect를_회수한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-html-host-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("synthetic.html");
    std::fs::write(&source, b"<p>host document").unwrap();
    let outside = fixture.0.join("outside.html");
    std::fs::write(&outside, b"<p>outside document").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic HTML".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state, tasks.clone(), Arc::new(Sink));
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = Bridge::connect(
        services.clone(),
        Arc::new(move || signal.notify_one()),
        PathBuf::from(EXECUTABLE),
        TIMEOUT,
    )
    .unwrap();
    assert!(
        Bridge::connect(
            services,
            Arc::new(|| {}),
            PathBuf::from("relative-helper"),
            TIMEOUT
        )
        .is_err()
    );
    let request = Request {
        path: source.to_str().unwrap().into(),
        token: 1,
    };
    host.submit(request.clone()).unwrap();
    match next(&mut host, &ready).await {
        Reply::SourceReady(progress) => assert_eq!(progress, request),
        _ => panic!("source readiness must precede result"),
    }
    match next(&mut host, &ready).await {
        Reply::Prepared {
            request: returned,
            result,
        } => {
            assert_eq!(returned, request);
            assert!(result.unwrap().html.contains("<p>host document</p>"));
        }
        _ => panic!("expected document result"),
    }
    let denied = Request {
        path: outside.to_str().unwrap().into(),
        token: 1,
    };
    host.submit(denied.clone()).unwrap();
    match next(&mut host, &ready).await {
        Reply::Prepared {
            request: returned,
            result,
        } => {
            assert_eq!(returned, denied);
            assert!(result.is_err());
        }
        _ => panic!("denied file must not be ready"),
    }
    host.submit(request.clone()).unwrap();
    assert!(
        matches!(next(&mut host, &ready).await, Reply::SourceReady(progress) if progress == request)
    );
    host.submit(request).unwrap();
    tokio::time::timeout(TIMEOUT, host.disconnect())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

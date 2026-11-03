use super::*;
use taide_model::ids::ProjectId;
use taide_runtime::{EventSink, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(10);

struct InboxSink(Arc<crate::provider_events::Inbox>);

impl EventSink for InboxSink {
    fn publish(&self, event: AppEvent) {
        self.0.record(&event);
    }
}

fn unique_socket() -> PathBuf {
    socket_path(&std::env::temp_dir().join(format!("taide-single-instance-{}", ProjectId::new())))
}

fn served() -> (
    tokio::runtime::Runtime,
    Arc<AppServices>,
    Arc<crate::provider_events::Inbox>,
    PathBuf,
    SocketFile,
) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-single-instance-data-{}", ProjectId::new())),
    ));
    let inbox = Arc::new(crate::provider_events::Inbox::default());
    let services = crate::bootstrap::services(state, tasks, Arc::new(InboxSink(inbox.clone())));
    let path = unique_socket();
    let Claim::Primary(listener, file) = acquire(&path, &[]) else {
        panic!("first claim owns the socket");
    };
    {
        let _context = runtime.enter();
        serve(services.clone(), inbox.clone(), listener);
    }
    (runtime, services, inbox, path, file)
}

#[test]
fn 단일_인스턴스는_두번째_실행의_인자를_중계하고_창을_앞으로_요청한다() {
    let (_runtime, services, inbox, path, file) = served();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, SOCKET_MODE);
    let arguments = vec![
        "/synthetic/notes.md".to_owned(),
        "--wait-marker".to_owned(),
        "/tmp/taide-wait-synthetic".to_owned(),
    ];
    assert!(matches!(acquire(&path, &arguments), Claim::Relayed));
    let queued = services.agents.drain_pending_external_opens();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].path, "/synthetic/notes.md");
    assert_eq!(
        queued[0].wait_marker.as_deref(),
        Some("/tmp/taide-wait-synthetic")
    );
    let pending = inbox.take();
    assert!(pending.raise_main_window && pending.external_opens);

    assert!(matches!(acquire(&path, &[]), Claim::Relayed));
    assert!(services.agents.drain_pending_external_opens().is_empty());
    let pending = inbox.take();
    assert!(pending.raise_main_window && !pending.external_opens);
    drop(file);
    assert!(!path.exists());
}

#[test]
fn 단일_인스턴스는_남은_소켓파일을_바꾸고_상한을_넘는_중계를_닫는다() {
    let stale = unique_socket();
    drop(UnixListener::bind(&stale).unwrap());
    assert!(stale.exists());
    let Claim::Primary(_listener, file) = acquire(&stale, &[]) else {
        panic!("a stale socket is replaced");
    };
    drop(file);
    assert!(!stale.exists());

    let (_runtime, services, inbox, path, _file) = served();
    let mut stream = UnixStream::connect(&path).unwrap();
    stream.set_read_timeout(Some(DEADLINE)).unwrap();
    let oversized = vec![b'a'; MAX_RELAY_BYTES + 1];
    let _ = stream.write_all(&oversized);
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    assert!(answer.is_empty());
    assert!(services.agents.drain_pending_external_opens().is_empty());
    assert!(!inbox.take().raise_main_window);
}

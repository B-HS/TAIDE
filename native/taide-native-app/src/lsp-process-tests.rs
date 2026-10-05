use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{Value, json};
use taide_lsp::manifest;
use taide_lsp::session::LspMessageSubscribers;
use taide_model::ids::ProjectId;
use taide_model::lsp::LspCommandSpec;
use taide_model::paths::AppPaths;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::{Notify, mpsc};

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const MESSAGE_CAPACITY: usize = 8;
const SESSION: &str = "synthetic-production-session";
const SERVER: &str = "gopls";
const CRASH_CODE: i32 = 7;
const EXPECTED_GENERATION: u32 = 1;

#[derive(Default)]
struct Sink {
    events: Mutex<Vec<AppEvent>>,
    changed: Notify,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        assert!(matches!(event, AppEvent::LspSessionStatusChanged { .. }));
        self.events.lock().unwrap().push(event);
        self.changed.notify_one();
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-lsp-process-{}", ProjectId::new()));
        std::fs::create_dir_all(&directory).unwrap();
        let directory = directory.canonicalize().unwrap();
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            AppState::new(AppPaths::new(directory.clone())),
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        Self {
            directory,
            services,
            sink,
        }
    }

    fn entry(&self, command: &str, args: Vec<String>) -> Arc<LspSessionEntry> {
        let mut spec = manifest::find_spec(SERVER).unwrap();
        spec.command = LspCommandSpec::Path {
            bin: command.into(),
            args,
        };
        let entry = Arc::new(LspSessionEntry::new(
            ProjectId::new(),
            spec,
            self.directory.to_str().unwrap().into(),
            LspMessageSubscribers::new(),
        ));
        self.services.lsp.insert(SESSION.into(), entry.clone());
        entry
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.lsp.shutdown();
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

async fn response(process: &LspProcHandle, messages: &mut mpsc::Receiver<String>, id: u64) {
    process
        .write_message(
            &json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}).to_string(),
        )
        .await
        .unwrap();
    let raw = tokio::time::timeout(DEADLINE, messages.recv())
        .await
        .unwrap()
        .unwrap();
    let value: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(value["id"], id);
    assert!(value["result"]["capabilities"].is_object());
}

#[cfg(unix)]
#[tokio::test]
async fn production_port는_실제_child_message_자동재시작_epoch_generation과_shutdown을_연결한다() {
    let fixture = Fixture::new();
    let mock = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/native-lsp-mock");
    assert!(mock.is_file(), "existing synthetic LSP example is required");
    let entry = fixture.entry(mock.to_str().unwrap(), Vec::new());
    let (sender, mut messages) = mpsc::channel(MESSAGE_CAPACITY);
    entry
        .subscribers
        .insert("synthetic".into(), move |message| {
            sender.try_send(message.into()).is_ok()
        });
    let epoch = entry.lifecycle.advance_process_epoch();
    let ports = crate::remote_lsp::Ports::new();
    let first = fixture
        .services
        .lsp
        .spawn_process(|| {
            (ports.create_process)(
                fixture.services.clone(),
                SESSION.into(),
                epoch,
                entry.spec.clone(),
                entry.root.clone(),
            )
        })
        .unwrap();
    *entry.proc.lock() = Some(first.clone());
    response(&first, &mut messages, 1).await;
    first.kill();
    tokio::time::timeout(DEADLINE, async {
        while entry.lifecycle.snapshot().generation != EXPECTED_GENERATION {
            fixture.sink.changed.notified().await;
        }
    })
    .await
    .unwrap();
    first.wait_for_completion().await;
    let restarted = entry.proc.lock().clone().unwrap();
    assert!(!Arc::ptr_eq(&first, &restarted));
    assert!(!entry.lifecycle.is_active_process_epoch(epoch));
    let snapshot = entry.lifecycle.snapshot();
    assert_eq!(snapshot.status, LspSessionStatus::Crashed);
    assert!(snapshot.last_error.unwrap().contains("초기화 핸드셰이크"));
    response(&restarted, &mut messages, 2).await;
    taide_runtime::lsp_actions::lsp_confirm_reinitialize(
        fixture.services.events.as_ref(),
        &fixture.services.lsp,
        SESSION.into(),
        EXPECTED_GENERATION,
    )
    .unwrap();
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Running);
    let events = fixture.sink.events.lock().unwrap().len();
    handle_process_exit(
        fixture.services.clone(),
        SESSION.into(),
        epoch,
        Some(CRASH_CODE),
        "old epoch".into(),
    );
    assert_eq!(fixture.sink.events.lock().unwrap().len(), events);
    let owner = Arc::downgrade(&fixture.services);
    fixture.services.lsp.shutdown();
    tokio::time::timeout(DEADLINE, fixture.services.lsp.wait_for_idle())
        .await
        .unwrap();
    fixture.services.tasks.shutdown().await;
    assert!(restarted.is_finished());
    assert!(entry.lifecycle.is_stopping());
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    assert_eq!(fixture.sink.events.lock().unwrap().len(), events);
    drop(fixture);
    assert!(owner.upgrade().is_none());
}

#[tokio::test]
async fn 재시작상한_정지epoch_없는session과_stderr_masking은_원본정책을_유지한다() {
    let fixture = Fixture::new();
    let entry = fixture.entry("synthetic-never-launched", Vec::new());
    let epoch = entry.lifecycle.advance_process_epoch();
    for expected in 1..=taide_lsp::process::RESTART_BACKOFF_LIMIT {
        assert_eq!(entry.lifecycle.begin_exit_recovery(epoch), Some(expected));
    }
    handle_process_exit(
        fixture.services.clone(),
        SESSION.into(),
        epoch,
        Some(CRASH_CODE),
        "synthetic tail".into(),
    );
    let snapshot = entry.lifecycle.snapshot();
    assert_eq!(snapshot.status, LspSessionStatus::Crashed);
    assert!(
        snapshot
            .last_error
            .unwrap()
            .contains("재시작을 중지했습니다")
    );
    assert!(entry.proc.lock().is_none());
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    entry.lifecycle.mark_stopping();
    handle_process_exit(
        fixture.services.clone(),
        SESSION.into(),
        epoch,
        None,
        String::new(),
    );
    handle_process_exit(
        fixture.services.clone(),
        "missing".into(),
        epoch,
        None,
        String::new(),
    );
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    assert_eq!(
        masked_stderr_tail("synthetic token=not-a-real-credential\n\nnext  \n"),
        "synthetic token=[redacted:key_value] / next"
    );
    fixture.services.tasks.shutdown().await;
}

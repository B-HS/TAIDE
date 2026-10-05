use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp::install::LspInstallStore;
use taide_lsp::native::session::SessionOptions;
use taide_lsp::native::{Failure, Phase};
use taide_lsp::store::LspStore;
use taide_runtime::native_lsp_actions::spawn_session;
use taide_runtime::{AiRequestStore, ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT_MS: u64 = 3_000;
const EXIT_GRACE: Duration = Duration::from_millis(20);
const HEADER_BYTES: usize = 4 * 1024;
const FRAME_BYTES: usize = 1024 * 1024;
const QUEUE_FRAMES: usize = 16;
const QUEUE_BYTES: usize = 4 * FRAME_BYTES;
#[cfg(unix)]
const SLEEP_SECONDS: u64 = 30;

fn config() -> LspProcConfig {
    LspProcConfig {
        command: "synthetic-unused-executable".into(),
        args: Vec::new(),
        cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    }
}

fn options() -> SessionOptions {
    SessionOptions {
        frame_limits: FrameLimits::new(HEADER_BYTES, FRAME_BYTES).unwrap(),
        writer_limits: WriterLimits::new(QUEUE_FRAMES, QUEUE_BYTES).unwrap(),
        command_capacity: QUEUE_FRAMES,
        command_bytes: QUEUE_BYTES,
        incoming_capacity: QUEUE_FRAMES,
        incoming_bytes: QUEUE_BYTES,
        outgoing_capacity: QUEUE_FRAMES,
        outgoing_bytes: QUEUE_BYTES,
        request_timeout_ms: REQUEST_TIMEOUT_MS,
        write_timeout: TEST_TIMEOUT,
        exit_grace: EXIT_GRACE,
    }
}

#[tokio::test]
async fn native_host는_잘못된_옵션과_종료된_감독자에_작업을_등록하지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let mut invalid = options();
    invalid.command_capacity = 0;
    assert!(matches!(
        spawn_session(&tasks, LspStore::new(), config(), json!({}), invalid),
        Err(Failure::Capacity)
    ));
    assert_eq!(tasks.tracked_count(), 0);
    tasks.stop_all();
    assert!(matches!(
        spawn_session(&tasks, LspStore::new(), config(), json!({}), options()),
        Err(Failure::TransportClosed)
    ));
    timeout(TEST_TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn native_host의_초기화_중_실제_child는_root_exit에서_회수되고_작업_추적도_비워진다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let processes = LspStore::new();
    let mut process_config = config();
    process_config.command = "/bin/sleep".into();
    process_config.args = vec![SLEEP_SECONDS.to_string()];
    let mut client = spawn_session(
        &tasks,
        processes.clone(),
        process_config,
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let initializing = timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Initializing))
        .await
        .unwrap()
        .unwrap();
    assert!(initializing.pid.is_some());
    assert_eq!(tasks.tracked_count(), 1);
    let drain = ExitDrain::new(AiRequestStore::new());
    timeout(
        TEST_TIMEOUT,
        drain.wait_for_direct_exit(tasks.clone(), LspInstallStore::new(), processes, TerminalStore::new()),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert_eq!(client.snapshot().phase, Phase::Degraded);
    assert_eq!(client.snapshot().failure, Some(Failure::TransportClosed));
    assert!(matches!(
        spawn_session(&tasks, LspStore::new(), config(), json!({}), options()),
        Err(Failure::TransportClosed)
    ));
}

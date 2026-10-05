use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp::install::LspInstallStore;
use taide_lsp::native::protocol::ServerNotification;
use taide_lsp::native::session::{SessionNoticeKind, SessionOptions};
use taide_lsp::native::{DocumentMirror, Failure, Phase};
use taide_lsp::store::LspStore;
use taide_runtime::native_lsp_actions::spawn_session;
use taide_runtime::{AiRequestStore, ExitDrain, TaskSupervisor};
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT_MS: u64 = 3_000;
const EXIT_GRACE: Duration = Duration::from_millis(20);
const HEADER_BYTES: usize = 4 * 1024;
const FRAME_BYTES: usize = 1024 * 1024;
const QUEUE_FRAMES: usize = 16;
const QUEUE_BYTES: usize = 4 * FRAME_BYTES;
const INIT_PROGRESS_MESSAGES: usize = 3;
const REQUEST_PROGRESS_MESSAGES: usize = 2;
const DOCUMENT_URI: &str = "file:///synthetic/native.rs";

fn config() -> LspProcConfig {
    LspProcConfig {
        command: env!("CARGO_BIN_EXE_mock-server").into(),
        args: vec!["--client-progress".into()],
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
async fn runtime_host_running의_명시_stop과_root_exit는_요청과_owned_worker를_남기지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let store = LspStore::new();
    let mut owners = Vec::new();
    for token in ["graceful", "forced"] {
        let mut client = spawn_session(
            &tasks,
            store.clone(),
            config(),
            json!({"processId":null,"rootUri":null,"capabilities":{},"workDoneToken":"initialize"}),
            options(),
        )
        .unwrap();
        let mut notices = client.take_notifications().unwrap();
        timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
            .await
            .unwrap()
            .unwrap();
        for _ in 0..INIT_PROGRESS_MESSAGES {
            let notice = timeout(TEST_TIMEOUT, notices.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                notice.kind,
                SessionNoticeKind::Notification(ServerNotification::Progress { .. })
            ));
        }
        client
            .open(DocumentMirror {
                uri: DOCUMENT_URI.into(),
                language_id: "rust".into(),
                revision: 0,
                version: 0,
                text: "합성 문서".into(),
            })
            .await
            .unwrap();
        let request_client = client.clone();
        let pending = tokio::spawn(async move {
            request_client
                .request(
                    "textDocument/definition".into(),
                    json!({
                        "textDocument":{"uri":DOCUMENT_URI},
                        "position":{"line":0,"character":0},
                        "workDoneToken":token,
                        "partialResultToken":format!("partial-{token}"),
                        "syntheticHold":true
                    }),
                    Some((DOCUMENT_URI.into(), 0)),
                )
                .await
        });
        for _ in 0..REQUEST_PROGRESS_MESSAGES {
            let notice = timeout(TEST_TIMEOUT, notices.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(
                notice.kind,
                SessionNoticeKind::Notification(ServerNotification::Progress { .. })
            ));
        }
        assert_eq!(client.snapshot().pending, 1);
        assert_eq!(client.snapshot().progress_tokens, REQUEST_PROGRESS_MESSAGES);
        owners.push((client, pending, notices));
    }
    assert_eq!(tasks.tracked_count(), owners.len());
    let (graceful, graceful_pending, mut graceful_notices) = owners.remove(0);
    let graceful_pid = graceful.snapshot().pid.unwrap();
    let (forced, forced_pending, _forced_notices) = owners.pop().unwrap();
    assert_ne!(forced.snapshot().pid.unwrap(), graceful_pid);
    timeout(TEST_TIMEOUT, graceful.stop())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        timeout(TEST_TIMEOUT, graceful_pending)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err(),
        Failure::Stopped
    );
    for _ in 0..REQUEST_PROGRESS_MESSAGES {
        let notice = timeout(TEST_TIMEOUT, graceful_notices.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            notice.kind,
            SessionNoticeKind::Notification(ServerNotification::Progress { .. })
        ));
    }
    assert_eq!(graceful.snapshot().phase, Phase::Stopped);
    assert_eq!(graceful.snapshot().pending, 0);
    assert_eq!(graceful.snapshot().progress_tokens, 0);
    assert!(graceful.snapshot().pid.is_none());
    assert_eq!(forced.snapshot().phase, Phase::Running);
    assert_eq!(forced.snapshot().pending, 1);
    let drain = ExitDrain::new(AiRequestStore::new());
    timeout(
        TEST_TIMEOUT,
        drain.wait_for_direct_exit(
            tasks.clone(),
            LspInstallStore::new(),
            store.clone(),
            Default::default(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        timeout(TEST_TIMEOUT, forced_pending)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err(),
        Failure::TransportClosed
    );
    assert_eq!(tasks.tracked_count(), 0);
    assert_eq!(forced.snapshot().phase, Phase::Degraded);
    assert_eq!(forced.snapshot().failure, Some(Failure::TransportClosed));
    assert_eq!(forced.snapshot().pending, 0);
    assert_eq!(forced.snapshot().progress_tokens, 0);
    assert!(matches!(
        spawn_session(&tasks, store.clone(), config(), json!({}), options()),
        Err(Failure::TransportClosed)
    ));
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

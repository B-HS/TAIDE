use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp::native::protocol::{ProgressValue, ServerNotification};
use taide_lsp::native::session::{
    SessionClient, SessionNotice, SessionNoticeKind, SessionOptions, SessionRunner,
};
use taide_lsp::native::{DocumentMirror, Failure, Phase};
use taide_lsp::store::LspStore;
use tokio::sync::mpsc;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const REQUEST_TIMEOUT_MS: u64 = 500;
const EXIT_GRACE: Duration = Duration::from_millis(20);
const QUEUE_FRAMES: usize = 16;
const HEADER_BYTES: usize = 4 * 1024;
const FRAME_BYTES: usize = 1024 * 1024;
const QUEUE_BYTES: usize = 4 * FRAME_BYTES;
const DOCUMENT_URI: &str = "file:///synthetic/native.rs";
const INIT_PROGRESS_MESSAGES: usize = 3;
const FEATURE_PROGRESS_MESSAGES: usize = 5;
const ACTIVE_REQUESTS: usize = 2;
const TOKENS_PER_REQUEST: usize = 2;

fn config() -> LspProcConfig {
    LspProcConfig {
        command: env!("CARGO_BIN_EXE_mock-server").into(),
        args: vec!["--client-progress".into()],
        cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    }
}

fn prepare() -> (SessionClient, SessionRunner, LspStore) {
    let store = LspStore::new();
    let (client, runner) = SessionRunner::prepare(
        store.clone(),
        config(),
        json!({"processId":null,"rootUri":null,"capabilities":{},"workDoneToken":"initialize"}),
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
        },
    )
    .unwrap();
    (client, runner, store)
}

fn params(token: &str, is_held: bool) -> Value {
    json!({
        "textDocument":{"uri":DOCUMENT_URI},
        "position":{"line":0,"character":0},
        "workDoneToken":token,
        "partialResultToken":format!("partial-{token}"),
        "syntheticHold":is_held
    })
}

async fn receive(notices: &mut mpsc::Receiver<SessionNotice>) -> SessionNotice {
    timeout(TEST_TIMEOUT, notices.recv())
        .await
        .unwrap()
        .unwrap()
}

fn assert_progress(notice: SessionNotice, token: &str, is_work_done: bool) {
    let SessionNoticeKind::Notification(ServerNotification::Progress {
        token: received,
        value,
    }) = notice.kind
    else {
        panic!("진행 알림이 필요합니다")
    };
    assert_eq!(
        received,
        taide_lsp::native::protocol::lsp_types::NumberOrString::String(token.into())
    );
    assert_eq!(matches!(value, ProgressValue::WorkDone(_)), is_work_done);
    if !is_work_done {
        assert_eq!(notice.message["params"]["value"]["title"], false);
    }
}

#[tokio::test]
async fn client_진행은_실제_initialize_응답_취소_timeout_재시작_종료에_묶이고_partial과_섞이지_않는다(
) {
    let (mut client, runner, store) = prepare();
    let mut notices = client.take_notifications().unwrap();
    let worker = tokio::spawn(runner.run());
    let running = timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running)).await;
    assert!(running.is_ok(), "초기화 실패 상태: {:?}", client.snapshot());
    running.unwrap().unwrap();
    for index in 0..INIT_PROGRESS_MESSAGES {
        assert_progress(
            receive(&mut notices).await,
            "initialize",
            index < INIT_PROGRESS_MESSAGES - 1,
        );
    }
    assert_eq!(client.snapshot().progress_tokens, 0);
    client
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "합성".into(),
        })
        .await
        .unwrap();
    let reply = client
        .request(
            "textDocument/definition".into(),
            params("normal", false),
            Some((DOCUMENT_URI.into(), 0)),
        )
        .await
        .unwrap();
    assert!(reply.is_null());
    for index in 0..FEATURE_PROGRESS_MESSAGES {
        let is_partial = index == 1;
        let token = if is_partial {
            "partial-normal"
        } else {
            "normal"
        };
        assert_progress(
            receive(&mut notices).await,
            token,
            !is_partial && index < FEATURE_PROGRESS_MESSAGES - 1,
        );
    }
    assert_eq!(client.snapshot().progress_tokens, 0);
    let request_client = client.clone();
    let request = tokio::spawn(async move {
        request_client
            .request(
                "textDocument/definition".into(),
                params("cancel", true),
                Some((DOCUMENT_URI.into(), 0)),
            )
            .await
    });
    assert_progress(receive(&mut notices).await, "cancel", true);
    assert_progress(receive(&mut notices).await, "partial-cancel", false);
    assert_eq!(client.snapshot().progress_tokens, 2);
    let mut duplicate = params("cancel", false);
    duplicate["partialResultToken"] = json!("unrelated");
    assert_eq!(
        client
            .request("textDocument/definition".into(), duplicate, None)
            .await
            .unwrap_err(),
        Failure::MalformedRequest
    );
    assert_eq!(client.snapshot().progress_tokens, 2);
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert_progress(receive(&mut notices).await, "cancel", false);
    assert_progress(receive(&mut notices).await, "partial-cancel", false);
    assert_eq!(client.snapshot().progress_tokens, 0);
    assert_eq!(client.snapshot().phase, Phase::Running);
    assert_eq!(
        client
            .request(
                "textDocument/definition".into(),
                params("timeout", true),
                None
            )
            .await
            .unwrap_err(),
        Failure::TimedOut
    );
    assert_progress(receive(&mut notices).await, "timeout", true);
    assert_progress(receive(&mut notices).await, "partial-timeout", false);
    assert_progress(receive(&mut notices).await, "timeout", false);
    assert_progress(receive(&mut notices).await, "partial-timeout", false);
    assert_eq!(client.snapshot().progress_tokens, 0);
    client.restart(config()).await.unwrap();
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    for index in 0..INIT_PROGRESS_MESSAGES {
        assert_progress(
            receive(&mut notices).await,
            "initialize",
            index < INIT_PROGRESS_MESSAGES - 1,
        );
    }
    assert_eq!(client.snapshot().progress_tokens, 0);
    timeout(TEST_TIMEOUT, client.stop()).await.unwrap().unwrap();
    assert_eq!(client.snapshot().phase, Phase::Stopped);
    assert_eq!(client.snapshot().progress_tokens, 0);
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn 활성_두_요청과_네_진행_token의_실제_session_restart_stop은_owner와_child를_남기지_않는다()
{
    let (mut client, runner, store) = prepare();
    let mut notices = client.take_notifications().unwrap();
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    for index in 0..INIT_PROGRESS_MESSAGES {
        assert_progress(
            receive(&mut notices).await,
            "initialize",
            index < INIT_PROGRESS_MESSAGES - 1,
        );
    }
    client
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "합성".into(),
        })
        .await
        .unwrap();
    let tokens = ["owner-a", "owner-b"];
    assert_eq!(tokens.len(), ACTIVE_REQUESTS);
    let mut pending = Vec::new();
    for token in tokens {
        let request_client = client.clone();
        pending.push(tokio::spawn(async move {
            request_client
                .request(
                    "textDocument/definition".into(),
                    params(token, true),
                    Some((DOCUMENT_URI.into(), 0)),
                )
                .await
        }));
        assert_progress(receive(&mut notices).await, token, true);
        assert_progress(
            receive(&mut notices).await,
            &format!("partial-{token}"),
            false,
        );
    }
    let old_pid = client.snapshot().pid.unwrap();
    let generation = client.snapshot().generation;
    assert_eq!(client.snapshot().pending, ACTIVE_REQUESTS);
    assert_eq!(
        client.snapshot().progress_tokens,
        ACTIVE_REQUESTS * TOKENS_PER_REQUEST
    );
    client.restart(config()).await.unwrap();
    for request in pending {
        assert_eq!(
            timeout(TEST_TIMEOUT, request)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            Failure::Restarted
        );
    }
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(client.snapshot().generation, generation + 1);
    assert_ne!(client.snapshot().pid.unwrap(), old_pid);
    for index in 0..INIT_PROGRESS_MESSAGES {
        let notice = receive(&mut notices).await;
        assert_eq!(notice.generation, generation + 1);
        assert_progress(notice, "initialize", index < INIT_PROGRESS_MESSAGES - 1);
    }
    assert_eq!(client.snapshot().pending, 0);
    assert_eq!(client.snapshot().progress_tokens, 0);
    let mut pending = Vec::new();
    for token in tokens {
        let request_client = client.clone();
        pending.push(tokio::spawn(async move {
            request_client
                .request(
                    "textDocument/definition".into(),
                    params(token, true),
                    Some((DOCUMENT_URI.into(), 0)),
                )
                .await
        }));
        assert_progress(receive(&mut notices).await, token, true);
        assert_progress(
            receive(&mut notices).await,
            &format!("partial-{token}"),
            false,
        );
    }
    assert_eq!(client.snapshot().pending, ACTIVE_REQUESTS);
    assert_eq!(
        client.snapshot().progress_tokens,
        ACTIVE_REQUESTS * TOKENS_PER_REQUEST
    );
    let stop_client = client.clone();
    let stop = tokio::spawn(async move { stop_client.stop().await });
    for request in pending {
        assert_eq!(
            timeout(TEST_TIMEOUT, request)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            Failure::Stopped
        );
    }
    for token in tokens {
        assert_progress(receive(&mut notices).await, token, false);
        assert_progress(
            receive(&mut notices).await,
            &format!("partial-{token}"),
            false,
        );
    }
    timeout(TEST_TIMEOUT, stop).await.unwrap().unwrap().unwrap();
    assert_eq!(client.snapshot().phase, Phase::Stopped);
    assert_eq!(client.snapshot().pending, 0);
    assert_eq!(client.snapshot().progress_tokens, 0);
    assert_eq!(client.snapshot().pid, None);
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

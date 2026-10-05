use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp::native::protocol::lsp_types::request::HoverRequest;
use taide_lsp::native::session::{SessionOptions, SessionRunner};
use taide_lsp::native::{DocumentMirror, Failure, Phase};
use taide_lsp::store::LspStore;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const QUEUE_FRAMES: usize = 16;
const FRAME_BYTES: usize = 1024 * 1024;
const HEADER_BYTES: usize = 4 * 1024;
const QUEUE_BYTES: usize = 4 * FRAME_BYTES;
const REQUEST_TIMEOUT_MS: u64 = 500;
const EXIT_GRACE: Duration = Duration::from_millis(20);
const DOCUMENT_URI: &str = "file:///synthetic/native.rs";
#[cfg(unix)]
const CLOSED_STDOUT_DURATION_SECONDS: u64 = 30;

fn config(mode: Option<&str>) -> LspProcConfig {
    LspProcConfig {
        command: env!("CARGO_BIN_EXE_mock-server").into(),
        args: mode.map(|mode| vec![mode.into()]).unwrap_or_default(),
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
async fn native_actor는_최신_저장_통지와_문서_닫기_재열기를_실제_서버에_전달한다() {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(Some("--save-lifecycle")),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    let document = || DocumentMirror {
        uri: DOCUMENT_URI.into(),
        language_id: "rust".into(),
        revision: 0,
        version: 0,
        text: "original".into(),
    };
    client.open(document()).await.unwrap();
    client
        .change(DOCUMENT_URI.into(), 0, 1, "最新 😀".into())
        .await
        .unwrap();
    client.saved(DOCUMENT_URI.into()).await.unwrap();
    client.close(DOCUMENT_URI.into()).await.unwrap();
    assert_eq!(
        client.saved(DOCUMENT_URI.into()).await,
        Err(Failure::DocumentNotOpen)
    );
    assert_eq!(
        client.close(DOCUMENT_URI.into()).await,
        Err(Failure::DocumentNotOpen)
    );
    client.open(document()).await.unwrap();
    let result = client
        .request_typed::<HoverRequest>(
            serde_json::from_value(
                json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
            )
            .unwrap(),
            Some((DOCUMENT_URI.into(), 0)),
        )
        .await
        .unwrap();
    assert_eq!(result.raw["contents"]["value"], "original");
    let methods = result.raw["experimental"]["methods"].as_array().unwrap();
    let observed = methods
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    assert!(observed.windows(4).any(|methods| methods
        == [
            "textDocument/didChange",
            "textDocument/didSave",
            "textDocument/didClose",
            "textDocument/didOpen",
        ]));
    timeout(TEST_TIMEOUT, client.stop()).await.unwrap().unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
    assert_eq!(client.snapshot().phase, Phase::Stopped);
}

#[tokio::test]
async fn native_owner는_store에_child를_등록하고_handshake_mirror_request와_join된_stop을_소유한다()
{
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(None),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    assert!(client.snapshot().pid.is_some());
    client
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "한글 e\u{301} 𐐀".into(),
        })
        .await
        .unwrap();
    let reply = client
        .request_typed::<HoverRequest>(
            serde_json::from_value(
                json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
            )
            .unwrap(),
            Some((DOCUMENT_URI.into(), 0)),
        )
        .await
        .unwrap();
    assert!(reply.value.is_some());
    assert_eq!(reply.raw["contents"]["value"], "한글 e\u{301} 𐐀");
    assert_eq!(reply.raw["experimental"]["initializeCount"], 1);
    timeout(TEST_TIMEOUT, client.stop()).await.unwrap().unwrap();
    assert_eq!(client.snapshot().phase, Phase::Stopped);
    assert_eq!(client.snapshot().pending, 0);
    assert_eq!(client.snapshot().pid, None);
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn native_owner는_잘못된_feature_params와_reply를_해당_요청만_실패시키고_후속_typed_왕복을_보존한다()
 {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(Some("--malformed-hover")),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
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
    assert_eq!(
        client
            .request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":DOCUMENT_URI}}),
                Some((DOCUMENT_URI.into(), 0))
            )
            .await,
        Err(Failure::MalformedRequest)
    );
    let params = || {
        serde_json::from_value(
            json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
        )
        .unwrap()
    };
    assert!(matches!(
        client
            .request_typed::<HoverRequest>(params(), Some((DOCUMENT_URI.into(), 0)))
            .await,
        Err(Failure::MalformedResponse)
    ));
    assert_eq!(client.snapshot().phase, Phase::Running);
    assert_eq!(client.snapshot().pending, 0);
    assert!(client.snapshot().pid.is_some());
    let reply = client
        .request_typed::<HoverRequest>(params(), Some((DOCUMENT_URI.into(), 0)))
        .await
        .unwrap();
    assert!(reply.value.is_some());
    assert_eq!(reply.raw["contents"]["value"], "합성");
    timeout(TEST_TIMEOUT, client.stop()).await.unwrap().unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn native_owner는_crash_pending을_종결하고_회수_뒤_restart와_최신_mirror_replay를_수행한다() {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(Some("--crash-on-hover")),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    let running = timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running)).await;
    assert!(
        matches!(running, Ok(Ok(_))),
        "initial Running result: {running:?}; snapshot: {:?}",
        client.snapshot()
    );
    client
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "以前".into(),
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
                Some((DOCUMENT_URI.into(), 0))
            )
            .await,
        Err(Failure::TransportClosed)
    );
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Degraded))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(client.snapshot().pid, None);
    client
        .change(
            DOCUMENT_URI.into(),
            0,
            1,
            "최신 日本語 中文 e\u{301}".into(),
        )
        .await
        .unwrap();
    client.restart(config(None)).await.unwrap();
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(client.snapshot().generation, 1);
    let reply = client
        .request(
            "textDocument/hover".into(),
            json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
            Some((DOCUMENT_URI.into(), 1)),
        )
        .await
        .unwrap();
    assert_eq!(reply["contents"]["value"], "최신 日本語 中文 e\u{301}");
    timeout(TEST_TIMEOUT, client.stop()).await.unwrap().unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn native_owner는_exit_무시의_알림을_전달하고_자동_회수로_stop을_완료한다() {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(Some("--ignore-exit")),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let mut notifications = client.take_notifications().unwrap();
    assert!(client.take_notifications().is_none());
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    let stopping_client = client.clone();
    let stopped = tokio::spawn(async move { stopping_client.stop().await });
    let notice = timeout(TEST_TIMEOUT, notifications.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(notice.generation, 0);
    assert_eq!(notice.message["method"], "synthetic/ignoredExit");
    timeout(TEST_TIMEOUT, stopped)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(client.snapshot().phase, Phase::Stopped);
    assert_eq!(client.snapshot().pid, None);
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    assert!(notifications.recv().await.is_none());
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn native_owner_작업_취소도_store의_회수_소유권을_detach하지_않는다() {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(None),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    assert!(client.snapshot().pid.is_some());
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    assert_eq!(client.snapshot().phase, Phase::Degraded);
    assert_eq!(
        client
            .request("textDocument/hover".into(), json!({}), None)
            .await,
        Err(Failure::TransportClosed)
    );
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn native_owner는_request_waiter_취소와_실제_deadline을_처리한_뒤_마지막_client_drop으로_종료한다()
 {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(Some("--ignore-hover")),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    client
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "한글".into(),
        })
        .await
        .unwrap();
    let requester = client.clone();
    let waiting = tokio::spawn(async move {
        requester
            .request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
                Some((DOCUMENT_URI.into(), 0)),
            )
            .await
    });
    let mut states = client.subscribe();
    timeout(TEST_TIMEOUT, async {
        while states.borrow().pending == 0 {
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    waiting.abort();
    assert!(waiting.await.unwrap_err().is_cancelled());
    timeout(TEST_TIMEOUT, async {
        while states.borrow().pending != 0 {
            states.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert_eq!(client.snapshot().phase, Phase::Running);
    assert_eq!(
        timeout(
            TEST_TIMEOUT,
            client.request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}}),
                Some((DOCUMENT_URI.into(), 0))
            )
        )
        .await
        .unwrap(),
        Err(Failure::TimedOut)
    );
    drop(client);
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    assert_eq!(states.borrow().phase, Phase::Stopped);
    assert_eq!(states.borrow().pid, None);
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn native_owner는_initialize_timeout의_원인을_유지하며_child를_회수한다() {
    let store = LspStore::new();
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        config(Some("--ignore-initialize")),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    let degraded = timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Degraded))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(degraded.failure, Some(Failure::TimedOut));
    assert_eq!(degraded.pending, 0);
    assert_eq!(degraded.pid, None);
    client.stop().await.unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn native_owner는_stdout만_닫고_살아_있는_직접_child도_연결_단절로_회수한다() {
    let store = LspStore::new();
    let closed_stdout = LspProcConfig {
        command: "/bin/sh".into(),
        args: vec![
            "-c".into(),
            format!("exec 1>&-; exec /bin/sleep {CLOSED_STDOUT_DURATION_SECONDS}"),
        ],
        cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    };
    let (mut client, runner) = SessionRunner::prepare(
        store.clone(),
        closed_stdout,
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
        options(),
    )
    .unwrap();
    let worker = tokio::spawn(runner.run());
    let degraded = timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Degraded))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(degraded.failure, Some(Failure::TransportClosed));
    assert_eq!(degraded.pending, 0);
    assert_eq!(degraded.pid, None);
    client.stop().await.unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

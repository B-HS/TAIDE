use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};
use taide_infra::lsp_frame::{FrameLimits, TransportFailure};
use taide_infra::lsp_proc::{self, LspProcConfig, LspProcHandle};
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp_coordinator_spike::{CoordinatorProbe, DocumentMirror, Failure, Phase};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

const REQUEST_TIMEOUT_MS: u64 = 500;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(3);
const MESSAGE_CAPACITY: usize = 8;
const HEADER_LIMIT: usize = 4 * 1024;
const BODY_LIMIT: usize = 1024 * 1024;
const WRITER_BYTE_LIMIT: usize = 4 * 1024 * 1024;
const CRASH_EXIT_CODE: i32 = 7;
const DOCUMENT_URI: &str = "file:///synthetic/main.rs";
const INITIAL_TEXT: &str = "fn main() { /* 한글 e\u{301} 𐐀 */ }\r\n";
const CHANGED_TEXT: &str = "fn main() { /* 日本語 中文 한글 */ }\r\n";
const REPLAY_TEXT: &str = "fn main() { /* replay 한글 e\u{301} 𐐀 */ }\r\n";

struct ProcessFixture {
    handle: LspProcHandle,
    messages: mpsc::Receiver<String>,
    exited: oneshot::Receiver<(Option<i32>, String)>,
}

fn spawn_server(should_crash: bool) -> ProcessFixture {
    let (messages_tx, messages) = mpsc::channel(MESSAGE_CAPACITY);
    let (exit_tx, exited) = oneshot::channel();
    let args = if should_crash {
        vec!["--crash-on-hover".into()]
    } else {
        Vec::new()
    };
    let handle = lsp_proc::spawn_owned(
        LspProcConfig {
            command: env!("CARGO_BIN_EXE_mock-server").into(),
            args,
            cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        },
        FrameLimits::new(HEADER_LIMIT, BODY_LIMIT).unwrap(),
        WriterLimits::new(MESSAGE_CAPACITY, WRITER_BYTE_LIMIT).unwrap(),
        move |payload| {
            messages_tx
                .try_send(payload)
                .map_err(|_| TransportFailure::ConsumerUnavailable)
        },
        move |status, stderr| {
            exit_tx.send((status, stderr)).ok();
        },
    )
    .expect("spawn the exact synthetic Rust server");
    assert!(handle.pid().is_some());
    ProcessFixture {
        handle,
        messages,
        exited,
    }
}

async fn write_batch(handle: &LspProcHandle, messages: &[Value]) {
    for message in messages {
        timeout(PROCESS_TIMEOUT, handle.write_message(&message.to_string()))
            .await
            .expect("owned server write must finish")
            .expect("framed stdin write must succeed");
    }
}

async fn receive(messages: &mut mpsc::Receiver<String>) -> Value {
    let payload = timeout(PROCESS_TIMEOUT, messages.recv())
        .await
        .expect("synthetic server must reply")
        .expect("server stdout must remain open until reply");
    serde_json::from_str(&payload).expect("framed reply must be valid JSON")
}

async fn verify_exit(fixture: &mut ProcessFixture, expected_code: i32) -> String {
    let (code, stderr) = timeout(PROCESS_TIMEOUT, &mut fixture.exited)
        .await
        .expect("owned child must be reaped")
        .expect("exit callback must report completion");
    assert_eq!(code, Some(expected_code));
    timeout(PROCESS_TIMEOUT, fixture.handle.wait_for_completion())
        .await
        .expect("owned stdout/stderr/wait workers must join");
    assert!(fixture.handle.is_exited());
    assert!(fixture.handle.is_finished());
    assert!(fixture.messages.recv().await.is_none());
    stderr
}

#[tokio::test]
async fn 실제_framed_process는_crash_회수_뒤_새_generation에_최신_문서를_replay한다() {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    coordinator
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 1,
            text: INITIAL_TEXT.into(),
        })
        .unwrap();
    let initialize_params = json!({
        "processId":null,"rootUri":null,
        "capabilities":{"general":{"positionEncodings":["utf-16"]}}
    });
    let initialize = coordinator.begin(0, initialize_params.clone()).unwrap();
    let mut crashing = spawn_server(true);
    write_batch(&crashing.handle, &[initialize]).await;
    let response = receive(&mut crashing.messages).await;
    let replay = coordinator.receive(0, 1, response).unwrap();
    assert_eq!(coordinator.phase(), Phase::Replaying);
    write_batch(&crashing.handle, &replay.outgoing).await;
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    assert_eq!(coordinator.phase(), Phase::Running);
    let change = coordinator
        .change(DOCUMENT_URI, 0, 1, CHANGED_TEXT.into())
        .unwrap()
        .unwrap();
    write_batch(&crashing.handle, &[change]).await;
    let hover_params =
        json!({"textDocument":{"uri":DOCUMENT_URI},"position":{"line":0,"character":0}});
    let pending = coordinator
        .request(
            10,
            "textDocument/hover",
            hover_params.clone(),
            Some((DOCUMENT_URI, 1)),
        )
        .unwrap();
    write_batch(&crashing.handle, std::slice::from_ref(&pending)).await;
    let stderr = verify_exit(&mut crashing, CRASH_EXIT_CODE).await;
    assert_eq!(stderr, "synthetic crash\n");

    let restart = coordinator.restart(20, initialize_params).unwrap();
    assert_eq!(restart.completed.len(), 1);
    assert_eq!(restart.completed[0].id, pending["id"].as_u64().unwrap());
    assert_eq!(restart.completed[0].generation, 0);
    assert_eq!(restart.completed[0].result, Err(Failure::Restarted));
    assert_eq!(coordinator.generation(), 1);
    let mut healthy = spawn_server(false);
    write_batch(&healthy.handle, &restart.outgoing).await;
    let response = receive(&mut healthy.messages).await;
    let replay = coordinator.receive(1, 21, response).unwrap();
    assert_eq!(
        replay.outgoing[1]["params"]["textDocument"]["text"],
        CHANGED_TEXT
    );
    assert!(coordinator
        .change(DOCUMENT_URI, 1, 2, REPLAY_TEXT.into())
        .unwrap()
        .is_none());
    write_batch(&healthy.handle, &replay.outgoing).await;
    let delta = coordinator.finish_replay(1).unwrap();
    assert_eq!(delta.len(), 1);
    assert_eq!(coordinator.phase(), Phase::Replaying);
    assert_eq!(
        coordinator.request(
            22,
            "textDocument/hover",
            hover_params.clone(),
            Some((DOCUMENT_URI, 2))
        ),
        Err(Failure::InvalidPhase)
    );
    write_batch(&healthy.handle, &delta).await;
    assert!(coordinator.finish_replay(1).unwrap().is_empty());
    assert_eq!(coordinator.phase(), Phase::Running);
    let hover = coordinator
        .request(
            23,
            "textDocument/hover",
            hover_params.clone(),
            Some((DOCUMENT_URI, 2)),
        )
        .unwrap();
    write_batch(&healthy.handle, std::slice::from_ref(&hover)).await;
    let response = receive(&mut healthy.messages).await;
    let outcome = coordinator.receive(1, 24, response).unwrap();
    assert_eq!(outcome.completed.len(), 1);
    assert_eq!(outcome.completed[0].generation, 1);
    assert_eq!(outcome.completed[0].id, hover["id"].as_u64().unwrap());
    let result = outcome.completed[0].result.as_ref().unwrap();
    assert_eq!(result["contents"]["value"], REPLAY_TEXT);
    assert_eq!(result["experimental"]["version"], 3);
    assert_eq!(result["experimental"]["initializeCount"], 1);
    assert_eq!(
        result["experimental"]["methods"],
        json!([
            "initialize",
            "initialized",
            "textDocument/didOpen",
            "textDocument/didChange",
            "textDocument/hover"
        ])
    );

    let closed = coordinator.close(DOCUMENT_URI).unwrap().unwrap();
    let reopened = coordinator
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 1,
            text: INITIAL_TEXT.into(),
        })
        .unwrap()
        .unwrap();
    write_batch(&healthy.handle, &[closed, reopened]).await;
    let hover = coordinator
        .request(
            25,
            "textDocument/hover",
            hover_params,
            Some((DOCUMENT_URI, 0)),
        )
        .unwrap();
    write_batch(&healthy.handle, &[hover]).await;
    let response = receive(&mut healthy.messages).await;
    let outcome = coordinator.receive(1, 26, response).unwrap();
    let result = outcome.completed[0].result.as_ref().unwrap();
    assert_eq!(result["contents"]["value"], INITIAL_TEXT);
    assert_eq!(result["experimental"]["version"], 1);
    assert_eq!(coordinator.pending_count(), 0);

    let stopping = coordinator.stop(27).unwrap();
    assert_eq!(stopping.outgoing.len(), 2);
    assert_eq!(stopping.outgoing[0]["method"], "textDocument/didClose");
    assert_eq!(stopping.outgoing[1]["method"], "shutdown");
    assert_eq!(coordinator.phase(), Phase::Stopping);
    write_batch(&healthy.handle, &stopping.outgoing).await;
    let response = receive(&mut healthy.messages).await;
    let exiting = coordinator.receive(1, 28, response).unwrap();
    assert_eq!(exiting.completed[0].result, Ok(Value::Null));
    assert_eq!(exiting.outgoing, [json!({"jsonrpc":"2.0","method":"exit"})]);
    assert_eq!(coordinator.phase(), Phase::Stopping);
    write_batch(&healthy.handle, &exiting.outgoing).await;
    assert!(verify_exit(&mut healthy, 0).await.is_empty());
    coordinator.finish_stop(1).unwrap();
    assert_eq!(coordinator.phase(), Phase::Stopped);
    assert_eq!(coordinator.pending_count(), 0);
}

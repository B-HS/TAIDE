use std::path::PathBuf;
use std::time::Duration;

use serde_json::{json, Value};
use taide_infra::lsp_frame::{FrameLimits, TransportFailure};
use taide_infra::lsp_proc::{self, LspProcConfig, LspProcHandle};
use taide_lsp_coordinator_spike::{CoordinatorProbe, Phase};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

const HEADER_LIMIT: usize = 128;
const BODY_LIMIT: usize = 1024;
const REQUEST_TIMEOUT_MS: u64 = 500;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(3);
const EXIT_GRACE: Duration = Duration::from_millis(20);
const MESSAGE_CAPACITY: usize = 8;

struct Fixture {
    handle: LspProcHandle,
    messages: mpsc::Receiver<String>,
    exited: oneshot::Receiver<Option<i32>>,
}

fn spawn(mode: Option<&str>, capacity: usize) -> Fixture {
    let (message_tx, messages) = mpsc::channel(capacity);
    let (exit_tx, exited) = oneshot::channel();
    let handle = lsp_proc::spawn_bounded(
        LspProcConfig {
            command: env!("CARGO_BIN_EXE_mock-server").into(),
            args: mode.map(|mode| vec![mode.into()]).unwrap_or_default(),
            cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        },
        FrameLimits::new(HEADER_LIMIT, BODY_LIMIT).unwrap(),
        move |message| {
            message_tx
                .try_send(message)
                .map_err(|_| TransportFailure::ConsumerUnavailable)
        },
        move |code, _| {
            exit_tx.send(code).ok();
        },
    )
    .unwrap();
    Fixture {
        handle,
        messages,
        exited,
    }
}

async fn send(fixture: &Fixture, messages: &[Value]) {
    for message in messages {
        timeout(
            PROCESS_TIMEOUT,
            fixture.handle.write_message(&message.to_string()),
        )
        .await
        .unwrap()
        .unwrap();
    }
}

async fn receive(fixture: &mut Fixture) -> Value {
    let message = timeout(PROCESS_TIMEOUT, fixture.messages.recv())
        .await
        .unwrap()
        .unwrap();
    serde_json::from_str(&message).unwrap()
}

async fn initialize(fixture: &mut Fixture) -> CoordinatorProbe {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    let message = coordinator
        .begin(
            0,
            json!({"processId":null,"rootUri":null,"capabilities":{}}),
        )
        .unwrap();
    send(fixture, &[message]).await;
    let message = receive(fixture).await;
    let outcome = coordinator.receive(0, 1, message).unwrap();
    send(fixture, &outcome.outgoing).await;
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    assert_eq!(coordinator.phase(), Phase::Running);
    coordinator
}

async fn join(fixture: &mut Fixture) -> Option<i32> {
    let code = timeout(PROCESS_TIMEOUT, &mut fixture.exited)
        .await
        .unwrap()
        .unwrap();
    timeout(PROCESS_TIMEOUT, fixture.handle.wait_for_completion())
        .await
        .unwrap();
    assert!(fixture.handle.is_exited());
    assert!(fixture.handle.is_finished());
    code
}

#[tokio::test]
async fn bounded_process는_outgoing_상한_거부_후에도_정상_초기화와_종료를_수행한다() {
    let mut fixture = spawn(None, MESSAGE_CAPACITY);
    assert!(fixture
        .handle
        .write_message(&"x".repeat(BODY_LIMIT + 1))
        .await
        .is_err());
    let mut coordinator = initialize(&mut fixture).await;
    let outcome = coordinator.stop(2).unwrap();
    send(&fixture, &outcome.outgoing).await;
    let message = receive(&mut fixture).await;
    let exiting = coordinator.receive(0, 3, message).unwrap();
    send(&fixture, &exiting.outgoing).await;
    assert_eq!(join(&mut fixture).await, Some(0));
    assert_eq!(fixture.handle.transport_failure(), None);
    assert!(fixture.messages.recv().await.is_none());
    coordinator.finish_stop(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Stopped);
}

#[tokio::test]
async fn 악성_길이_큐_포화와_잘린_eof는_원인별로_종결하고_owned_child를_회수한다() {
    for (mode, capacity, failure) in [
        (
            "--oversized-output",
            MESSAGE_CAPACITY,
            TransportFailure::BodyTooLarge,
        ),
        (
            "--notification-burst",
            1,
            TransportFailure::ConsumerUnavailable,
        ),
        (
            "--truncated-output",
            MESSAGE_CAPACITY,
            TransportFailure::TruncatedFrame,
        ),
    ] {
        let mut fixture = spawn(Some(mode), capacity);
        join(&mut fixture).await;
        assert_eq!(fixture.handle.transport_failure(), Some(failure), "{mode}");
        if failure == TransportFailure::ConsumerUnavailable {
            assert_eq!(receive(&mut fixture).await["method"], "synthetic/notice");
        }
        assert!(fixture.messages.recv().await.is_none());
        assert!(fixture.handle.write_message("{}").await.is_err());
    }
}

#[tokio::test]
async fn exit를_무시하는_서버도_grace_뒤_직접_owned_kill과_join_후에만_stopped가_된다() {
    let mut fixture = spawn(Some("--ignore-exit"), MESSAGE_CAPACITY);
    let mut coordinator = initialize(&mut fixture).await;
    let outcome = coordinator.stop(2).unwrap();
    send(&fixture, &outcome.outgoing).await;
    let response = receive(&mut fixture).await;
    let exiting = coordinator.receive(0, 3, response).unwrap();
    send(&fixture, &exiting.outgoing).await;
    assert_eq!(
        receive(&mut fixture).await["method"],
        "synthetic/ignoredExit"
    );
    assert_eq!(coordinator.phase(), Phase::Stopping);
    assert!(!fixture.handle.is_exited());
    assert!(timeout(EXIT_GRACE, fixture.handle.wait_for_completion())
        .await
        .is_err());
    assert!(!fixture.handle.is_finished());
    fixture.handle.kill();
    join(&mut fixture).await;
    assert_eq!(fixture.handle.transport_failure(), None);
    assert!(fixture.messages.recv().await.is_none());
    coordinator.finish_stop(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Stopped);
}

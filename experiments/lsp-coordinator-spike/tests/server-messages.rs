use std::path::PathBuf;
use std::time::Duration;

use std::collections::BTreeMap;

use serde_json::{json, Value};
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp::native::protocol::{
    lsp_types::ApplyWorkspaceEditResponse, ProgressValue, ServerNotification, ServerReply,
    ServerRequestKind,
};
use taide_lsp::native::session::{
    ServerRequest, SessionClient, SessionNotice, SessionNoticeKind, SessionOptions, SessionRunner,
};
use taide_lsp::native::{Failure, Phase};
use taide_lsp::store::LspStore;
use tokio::sync::mpsc;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const FRAME_BYTES: usize = 1024 * 1024;
const HEADER_BYTES: usize = 4 * 1024;
const QUEUE_FRAMES: usize = 16;
const QUEUE_BYTES: usize = 4 * FRAME_BYTES;
const REQUEST_TIMEOUT_MS: u64 = 500;
const EXIT_GRACE: Duration = Duration::from_millis(20);
const EXPECTED_REPLIES: usize = 6;
const EXPECTED_WORK_DONE_MESSAGES: usize = 3;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const REQUEST_FAILED: i64 = -32803;

fn config() -> LspProcConfig {
    LspProcConfig {
        command: env!("CARGO_BIN_EXE_mock-server").into(),
        args: vec!["--client-requests".into()],
        cwd: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    }
}

fn prepare() -> (SessionClient, SessionRunner, LspStore) {
    let store = LspStore::new();
    let (client, runner) = SessionRunner::prepare(
        store.clone(),
        config(),
        json!({"processId":null,"rootUri":null,"capabilities":{}}),
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

async fn receive(notices: &mut mpsc::Receiver<SessionNotice>) -> SessionNotice {
    timeout(TEST_TIMEOUT, notices.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn next_request(notices: &mut mpsc::Receiver<SessionNotice>) -> ServerRequest {
    loop {
        if let SessionNoticeKind::Request(request) = receive(notices).await.kind {
            return request;
        }
    }
}

#[tokio::test]
async fn server_request를_response로_오인하지_않고_타입별_왕복과_원형_diagnostic을_보존한다() {
    let (client, runner, store) = prepare();
    let mut notices = client.take_notifications().unwrap();
    let worker = tokio::spawn(runner.run());
    let notice = receive(&mut notices).await;
    assert_eq!(notice.message["method"], "workspace/configuration");
    let SessionNoticeKind::Request(request) = notice.kind else {
        panic!("expected typed server request")
    };
    assert!(
        matches!(&*request.kind, ServerRequestKind::Configuration(params) if params.items.len() == 2)
    );
    let ticket = request.ticket;
    assert_eq!(
        client
            .reply(
                ticket.clone(),
                ServerReply::Configuration(vec![Value::Null])
            )
            .await,
        Err(Failure::MalformedResponse)
    );
    client
        .reply(
            ticket.clone(),
            ServerReply::Configuration(vec![json!({"synthetic":true}), Value::Null]),
        )
        .await
        .unwrap();
    assert_eq!(
        client.reply(ticket, ServerReply::Acknowledged).await,
        Err(Failure::StaleRequest)
    );
    let mut replies = BTreeMap::new();
    let mut has_diagnostics = false;
    let mut has_partial = false;
    let mut has_log = false;
    let mut work_done_messages = 0;
    while replies.len() < EXPECTED_REPLIES
        || !has_diagnostics
        || !has_partial
        || !has_log
        || work_done_messages < EXPECTED_WORK_DONE_MESSAGES
    {
        let notice = receive(&mut notices).await;
        match notice.kind {
            SessionNoticeKind::Request(request) => {
                let reply = match &*request.kind {
                    ServerRequestKind::ApplyEdit(_) => {
                        ServerReply::ApplyEdit(ApplyWorkspaceEditResponse {
                            applied: false,
                            failure_reason: Some("private path must not cross wire".into()),
                            failed_change: None,
                        })
                    }
                    ServerRequestKind::CreateProgress(_) | ServerRequestKind::Refresh(_) => {
                        ServerReply::Acknowledged
                    }
                    _ => panic!("unexpected request"),
                };
                client.reply(request.ticket, reply).await.unwrap();
            }
            SessionNoticeKind::Notification(ServerNotification::Diagnostics(params)) => {
                assert_eq!(params.version, Some(0));
                assert_eq!(params.diagnostics.len(), 1);
                assert_eq!(params.diagnostics[0].message, "합성 진단");
                assert_eq!(
                    notice.message["params"]["diagnostics"][0]["data"],
                    Value::Null
                );
                assert_eq!(
                    notice.message["params"]["diagnostics"][0]["experimental"]["retained"],
                    true
                );
                has_diagnostics = true;
            }
            SessionNoticeKind::Notification(ServerNotification::Progress {
                value: ProgressValue::Partial(value),
                ..
            }) => {
                assert_eq!(value, json!([{"synthetic":true}]));
                has_partial = true;
            }
            SessionNoticeKind::Notification(ServerNotification::Log(params)) => {
                assert_eq!(params.message, "합성 로그");
                has_log = true;
            }
            SessionNoticeKind::Notification(ServerNotification::Progress {
                value: ProgressValue::WorkDone(_),
                ..
            }) => {
                assert_eq!(notice.message["params"]["token"], "index");
                work_done_messages += 1;
            }
            SessionNoticeKind::Notification(ServerNotification::Extension) => {
                assert_eq!(notice.message["method"], "synthetic/clientReply");
                let reply = &notice.message["params"];
                replies.insert(reply["id"].to_string(), reply.clone());
            }
            _ => panic!("unexpected notification"),
        }
    }
    assert_eq!(replies["-1"]["result"], json!([{"synthetic":true},null]));
    assert_eq!(
        replies["\"unsupported\""]["error"]["code"],
        METHOD_NOT_FOUND
    );
    assert_eq!(replies["\"invalid\""]["error"]["code"], INVALID_PARAMS);
    assert_eq!(
        replies["\"edit\""]["result"],
        json!({"applied":false,"failureReason":"edit rejected"})
    );
    assert_eq!(replies["\"progress\""]["result"], Value::Null);
    assert_eq!(replies["\"refresh\""]["result"], Value::Null);
    assert_eq!(client.snapshot().phase, Phase::Running);
    assert_eq!(client.snapshot().server_pending, 0);
    client.stop().await.unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

#[tokio::test]
async fn server_reply는_다른_session_만료된_ticket과_restart_이전_generation을_거절한다() {
    let (mut client, runner, store) = prepare();
    let mut notices = client.take_notifications().unwrap();
    let worker = tokio::spawn(runner.run());
    let request = next_request(&mut notices).await;
    let old_ticket = request.ticket;
    let (other_client, other_runner, other_store) = prepare();
    let mut other_notices = other_client.take_notifications().unwrap();
    let other_worker = tokio::spawn(other_runner.run());
    let other_request = next_request(&mut other_notices).await;
    assert_eq!(old_ticket.id(), other_request.ticket.id());
    assert_eq!(
        client.snapshot().generation,
        other_client.snapshot().generation
    );
    assert_eq!(
        other_client
            .reply(
                old_ticket.clone(),
                ServerReply::Configuration(vec![Value::Null, Value::Null])
            )
            .await,
        Err(Failure::StaleRequest)
    );
    loop {
        let notice = receive(&mut notices).await;
        if notice.message["method"] == "synthetic/clientReply"
            && notice.message["params"]["id"] == -1
        {
            assert_eq!(notice.message["params"]["error"]["code"], REQUEST_FAILED);
            break;
        }
    }
    assert_eq!(
        client
            .reply(
                old_ticket.clone(),
                ServerReply::Configuration(vec![Value::Null, Value::Null])
            )
            .await,
        Err(Failure::StaleRequest)
    );
    assert_eq!(client.snapshot().phase, Phase::Running);
    client.restart(config()).await.unwrap();
    timeout(TEST_TIMEOUT, client.wait_for_phase(Phase::Running))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        client
            .reply(
                old_ticket,
                ServerReply::Configuration(vec![Value::Null, Value::Null])
            )
            .await,
        Err(Failure::StaleGeneration)
    );
    client.stop().await.unwrap();
    other_client.stop().await.unwrap();
    assert_eq!(client.snapshot().server_pending, 0);
    assert_eq!(other_client.snapshot().server_pending, 0);
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    timeout(TEST_TIMEOUT, other_worker).await.unwrap().unwrap();
    store.shutdown();
    other_store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
    timeout(TEST_TIMEOUT, other_store.wait_for_idle())
        .await
        .unwrap();
}

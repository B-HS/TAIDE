use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_lsp::native::session::{SessionOptions, SessionRunner};
use taide_lsp::native::{DocumentMirror, Failure};
use taide_lsp::store::LspStore;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const FRAME_BYTES: usize = 1024 * 1024;
const HEADER_BYTES: usize = 4 * 1024;
const QUEUE_FRAMES: usize = 16;
const QUEUE_BYTES: usize = 4 * FRAME_BYTES;
const REQUEST_TIMEOUT_MS: u64 = 500;
const EXIT_GRACE: Duration = Duration::from_millis(20);
const URI: &str = "file:///synthetic/native.rs";
const OTHER_URI: &str = "file:///synthetic/plain.txt";

#[tokio::test]
async fn 실제_register_응답_뒤에만_selector_기능이_열리고_unregister_응답_뒤에는_닫힌다() {
    let store = LspStore::new();
    let (client, runner) = SessionRunner::prepare(store.clone(), LspProcConfig {
        command:env!("CARGO_BIN_EXE_mock-server").into(), args:vec!["--dynamic-registration".into()], cwd:PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    }, json!({"processId":null,"rootUri":null,"capabilities":{"textDocument":{"hover":{"dynamicRegistration":true}}}}),SessionOptions {
        frame_limits:FrameLimits::new(HEADER_BYTES,FRAME_BYTES).unwrap(), writer_limits:WriterLimits::new(QUEUE_FRAMES,QUEUE_BYTES).unwrap(), command_capacity:QUEUE_FRAMES,command_bytes:QUEUE_BYTES,incoming_capacity:QUEUE_FRAMES,incoming_bytes:QUEUE_BYTES,outgoing_capacity:QUEUE_FRAMES,outgoing_bytes:QUEUE_BYTES,request_timeout_ms:REQUEST_TIMEOUT_MS,write_timeout:TEST_TIMEOUT,exit_grace:EXIT_GRACE,
    }).unwrap();
    let mut notices = client.take_notifications().unwrap();
    let worker = tokio::spawn(runner.run());
    let registered = timeout(TEST_TIMEOUT, notices.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(registered.message["params"]["id"], "register");
    assert_eq!(
        registered.message["params"]["result"],
        serde_json::Value::Null
    );
    assert_eq!(client.snapshot().registrations, 1);
    assert_eq!(client.snapshot().capability_revision, 1);
    for (uri, language) in [(URI, "rust"), (OTHER_URI, "plaintext")] {
        client
            .open(DocumentMirror {
                uri: uri.into(),
                language_id: language.into(),
                revision: 0,
                version: 0,
                text: "합성".into(),
            })
            .await
            .unwrap();
    }
    let hover = client
        .request(
            "textDocument/hover".into(),
            json!({"textDocument":{"uri":URI},"position":{"line":0,"character":0}}),
            Some((URI.into(), 0)),
        )
        .await
        .unwrap();
    assert_eq!(hover["contents"]["value"], "합성");
    assert_eq!(
        client
            .request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":OTHER_URI},"position":{"line":0,"character":0}}),
                Some((OTHER_URI.into(), 0))
            )
            .await,
        Err(Failure::UnsupportedCapability)
    );
    client
        .request("synthetic/unregister".into(), json!({}), None)
        .await
        .unwrap();
    assert_eq!(client.snapshot().registrations, 0);
    assert!(client.snapshot().capability_revision > 1);
    assert_eq!(
        client
            .request(
                "textDocument/hover".into(),
                json!({"textDocument":{"uri":URI},"position":{"line":0,"character":0}}),
                Some((URI.into(), 0))
            )
            .await,
        Err(Failure::UnsupportedCapability)
    );
    client.stop().await.unwrap();
    timeout(TEST_TIMEOUT, worker).await.unwrap().unwrap();
    store.shutdown();
    timeout(TEST_TIMEOUT, store.wait_for_idle()).await.unwrap();
}

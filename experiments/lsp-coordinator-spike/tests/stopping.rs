use serde_json::{json, Value};
use taide_lsp_coordinator_spike::{CoordinatorProbe, DocumentMirror, Failure, Phase};

const REQUEST_TIMEOUT_MS: u64 = 500;
const DOCUMENT_URI: &str = "file:///synthetic/stop.rs";
const SERVER_ERROR: i64 = -32001;

fn document() -> DocumentMirror {
    DocumentMirror {
        uri: DOCUMENT_URI.into(),
        language_id: "rust".into(),
        revision: 0,
        version: 1,
        text: "한글".into(),
    }
}

fn running() -> CoordinatorProbe {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    coordinator.open(document()).unwrap();
    let init = coordinator.begin(0, json!({})).unwrap();
    coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1,"hoverProvider":true}}})).unwrap();
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    coordinator
}

#[test]
fn shutdown은_pending을_종결하고_exit_reply_뒤에도_회수_ack_전에는_stopped가_아니다() {
    let mut coordinator = running();
    let pending = coordinator
        .request(2, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 0)))
        .unwrap();
    let stopping = coordinator.stop(3).unwrap();
    assert_eq!(stopping.completed.len(), 1);
    assert_eq!(stopping.completed[0].id, pending["id"].as_u64().unwrap());
    assert_eq!(stopping.completed[0].result, Err(Failure::Stopped));
    assert_eq!(
        stopping
            .outgoing
            .iter()
            .map(|value| value["method"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["$/cancelRequest", "textDocument/didClose", "shutdown"]
    );
    let shutdown_id = stopping.outgoing.last().unwrap()["id"].clone();
    assert_eq!(coordinator.phase(), Phase::Stopping);
    assert_eq!(coordinator.pending_count(), 1);
    assert!(!coordinator.supports("textDocument/hover"));
    assert_eq!(
        coordinator.request(4, "synthetic/customRequest", json!({}), None),
        Err(Failure::InvalidPhase)
    );
    assert_eq!(coordinator.open(document()), Err(Failure::InvalidPhase));
    assert_eq!(
        coordinator.change(DOCUMENT_URI, 0, 1, "blocked".into()),
        Err(Failure::InvalidPhase)
    );
    assert_eq!(coordinator.close(DOCUMENT_URI), Err(Failure::InvalidPhase));
    assert_eq!(coordinator.saved(DOCUMENT_URI), Err(Failure::InvalidPhase));
    assert_eq!(
        coordinator.restart(4, json!({})),
        Err(Failure::InvalidPhase)
    );
    assert_eq!(coordinator.finish_stop(1), Err(Failure::StaleGeneration));
    assert_eq!(coordinator.finish_stop(0), Err(Failure::InvalidPhase));
    assert_eq!(
        coordinator.cancel(shutdown_id.as_u64().unwrap()),
        Err(Failure::InvalidPhase)
    );
    let late = coordinator
        .receive(
            0,
            4,
            json!({"jsonrpc":"2.0","id":pending["id"],"result":"late"}),
        )
        .unwrap();
    assert!(late.completed.is_empty());
    assert!(late.outgoing.is_empty());
    let stopped = coordinator
        .receive(
            0,
            5,
            json!({"jsonrpc":"2.0","id":shutdown_id,"result":null}),
        )
        .unwrap();
    assert_eq!(stopped.completed[0].result, Ok(Value::Null));
    assert_eq!(stopped.outgoing, [json!({"jsonrpc":"2.0","method":"exit"})]);
    assert_eq!(coordinator.phase(), Phase::Stopping);
    assert_eq!(coordinator.pending_count(), 0);
    coordinator.finish_stop(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Stopped);
    assert_eq!(coordinator.stop(6), Err(Failure::InvalidPhase));
    assert_eq!(coordinator.begin(6, json!({})), Err(Failure::InvalidPhase));
}

#[test]
fn initialization_및_replay_중_종료는_일반_요청없이_exit만_보낸다() {
    for is_replaying in [false, true] {
        let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
        coordinator.open(document()).unwrap();
        let init = coordinator.begin(0, json!({})).unwrap();
        if is_replaying {
            coordinator.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
        }
        let stopping = coordinator.stop(2).unwrap();
        assert_eq!(
            stopping.outgoing,
            [json!({"jsonrpc":"2.0","method":"exit"})]
        );
        assert_eq!(stopping.completed.len(), usize::from(!is_replaying));
        assert_eq!(coordinator.phase(), Phase::Stopping);
        let late = coordinator.receive(0, 3, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
        assert!(late.outgoing.is_empty());
        coordinator.finish_stop(0).unwrap();
        assert_eq!(coordinator.phase(), Phase::Stopped);
    }
}

#[test]
fn shutdown_error_및_timeout은_cancel없이_exit하고_pending을_남기지_않는다() {
    for result in [
        json!({"result":"not null"}),
        json!({"error":{"code":SERVER_ERROR,"message":"synthetic shutdown error"}}),
    ] {
        let mut coordinator = running();
        let stopping = coordinator.stop(2).unwrap();
        let mut response = result;
        response["jsonrpc"] = json!("2.0");
        response["id"] = stopping.outgoing.last().unwrap()["id"].clone();
        let stopped = coordinator.receive(0, 3, response).unwrap();
        assert!(stopped.completed[0].result.is_err());
        assert_eq!(stopped.outgoing, [json!({"jsonrpc":"2.0","method":"exit"})]);
        assert_eq!(coordinator.phase(), Phase::Stopping);
        coordinator.finish_stop(0).unwrap();
    }
    let mut coordinator = running();
    coordinator.stop(2).unwrap();
    let expired = coordinator.expire(2 + REQUEST_TIMEOUT_MS).unwrap();
    assert_eq!(expired.completed[0].result, Err(Failure::TimedOut));
    assert_eq!(expired.outgoing, [json!({"jsonrpc":"2.0","method":"exit"})]);
    assert_eq!(coordinator.pending_count(), 0);
    assert_eq!(coordinator.phase(), Phase::Stopping);
    assert_eq!(coordinator.last_failure(), Some(&Failure::TimedOut));
    coordinator.finish_stop(0).unwrap();
}

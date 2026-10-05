use serde_json::json;
use taide_lsp_coordinator_spike::{CoordinatorProbe, DocumentMirror, Failure, Phase};

const REQUEST_TIMEOUT_MS: u64 = 500;
const DOCUMENT_URI: &str = "file:///synthetic/main.rs";

fn document(revision: u64, text: &str) -> DocumentMirror {
    DocumentMirror {
        uri: DOCUMENT_URI.into(),
        language_id: "rust".into(),
        revision,
        version: 1,
        text: text.into(),
    }
}

#[test]
fn generation마다_초기화는_한번이며_문서_replay_write가_끝나기_전에는_running이_아니다() {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    coordinator.open(document(0, "한글 e\u{301}")).unwrap();
    let initialize = coordinator.begin(0, json!({"capabilities": {}})).unwrap();
    assert_eq!(initialize["method"], "initialize");
    assert_eq!(coordinator.begin(0, json!({})), Err(Failure::InvalidPhase));
    assert_eq!(coordinator.phase(), Phase::Initializing);
    assert_eq!(
        coordinator.request(0, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 0))),
        Err(Failure::InvalidPhase)
    );
    let id = initialize["id"].as_u64().unwrap();
    let replay = coordinator.receive(0, 1, json!({"jsonrpc":"2.0", "id":id, "result":{"capabilities":{"positionEncoding":"utf-16","textDocumentSync":1,"hoverProvider":true}}})).unwrap();
    assert_eq!(coordinator.phase(), Phase::Replaying);
    assert_eq!(replay.outgoing[0]["method"], "initialized");
    assert_eq!(replay.outgoing[1]["method"], "textDocument/didOpen");
    assert_eq!(
        replay.outgoing[1]["params"]["textDocument"]["text"],
        "한글 e\u{301}"
    );
    assert_eq!(coordinator.finish_replay(1), Err(Failure::StaleGeneration));
    coordinator.finish_replay(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Running);
    let pending = coordinator
        .request(10, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 0)))
        .unwrap();
    let restarted = coordinator.restart(20, json!({"capabilities":{}})).unwrap();
    assert_eq!(restarted.completed[0].id, pending["id"].as_u64().unwrap());
    assert_eq!(restarted.completed[0].result, Err(Failure::Restarted));
    assert_eq!(coordinator.generation(), 1);
    assert!(coordinator
        .receive(
            0,
            21,
            json!({"jsonrpc":"2.0","id":pending["id"],"result":"old"})
        )
        .unwrap()
        .completed
        .is_empty());
    let next_id = restarted.outgoing[0]["id"].as_u64().unwrap();
    let replay = coordinator
        .receive(
            1,
            22,
            json!({"jsonrpc":"2.0","id":next_id,"result":{"capabilities":{"textDocumentSync":1}}}),
        )
        .unwrap();
    assert_eq!(
        replay.outgoing[1]["params"]["textDocument"]["text"],
        "한글 e\u{301}"
    );
    coordinator.finish_replay(1).unwrap();
    assert_eq!(coordinator.phase(), Phase::Running);
}

#[test]
fn revision_cancel_timeout은_pending을_종결하고_늦은_응답을_채택하지_않는다() {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    coordinator.open(document(0, "before")).unwrap();
    let init = coordinator.begin(0, json!({})).unwrap();
    coordinator
        .receive(
            0,
            1,
            json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"textDocumentSync":1,"hoverProvider":true}}}),
        )
        .unwrap();
    coordinator.finish_replay(0).unwrap();
    let pending = coordinator
        .request(10, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 0)))
        .unwrap();
    let change = coordinator
        .change(DOCUMENT_URI, 0, 1, "after".into())
        .unwrap()
        .unwrap();
    assert_eq!(change["method"], "textDocument/didChange");
    assert_eq!(change["params"]["textDocument"]["version"], 2);
    let stale = coordinator
        .receive(
            0,
            11,
            json!({"jsonrpc":"2.0","id":pending["id"],"result":"stale"}),
        )
        .unwrap();
    assert_eq!(stale.completed[0].result, Err(Failure::StaleRevision));
    let cancellable = coordinator
        .request(20, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 1)))
        .unwrap();
    let id = cancellable["id"].as_u64().unwrap();
    let cancelled = coordinator.cancel(id).unwrap();
    assert_eq!(cancelled.outgoing[0]["method"], "$/cancelRequest");
    assert_eq!(cancelled.completed[0].result, Err(Failure::Cancelled));
    assert!(coordinator
        .receive(0, 21, json!({"jsonrpc":"2.0","id":id,"result":"late"}))
        .unwrap()
        .completed
        .is_empty());
    let expiring = coordinator
        .request(30, "textDocument/hover", json!({}), None)
        .unwrap();
    assert!(coordinator
        .expire(30 + REQUEST_TIMEOUT_MS - 1)
        .unwrap()
        .completed
        .is_empty());
    let expired = coordinator.expire(30 + REQUEST_TIMEOUT_MS).unwrap();
    assert_eq!(expired.completed[0].id, expiring["id"].as_u64().unwrap());
    assert_eq!(expired.completed[0].result, Err(Failure::TimedOut));
    assert_eq!(expired.outgoing[0]["method"], "$/cancelRequest");
    assert_eq!(coordinator.pending_count(), 0);
    let previous_incarnation = coordinator
        .request(
            600,
            "textDocument/hover",
            json!({}),
            Some((DOCUMENT_URI, 1)),
        )
        .unwrap();
    coordinator.close(DOCUMENT_URI).unwrap();
    coordinator
        .open(document(1, "reopened at the same revision"))
        .unwrap();
    let stale = coordinator.receive(0, 601, json!({"jsonrpc":"2.0","id":previous_incarnation["id"],"result":"previous document incarnation"})).unwrap();
    assert_eq!(stale.completed[0].result, Err(Failure::StaleRevision));
}

#[test]
fn 초기화_오류와_만료는_연결중을_무한히_유지하지_않는다() {
    let mut failed = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    let init = failed.begin(0, json!({})).unwrap();
    let result = failed.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"error":{"code":-32002,"message":"fixture rejected"}})).unwrap();
    assert_eq!(
        result.completed[0].result,
        Err(Failure::ServerError(-32002))
    );
    assert_eq!(failed.phase(), Phase::Degraded);
    let mut expired = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    expired.begin(0, json!({})).unwrap();
    let result = expired.expire(REQUEST_TIMEOUT_MS).unwrap();
    assert_eq!(result.completed[0].result, Err(Failure::TimedOut));
    assert!(result.outgoing.is_empty());
    assert_eq!(expired.phase(), Phase::Degraded);
    let mut invalid = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    let init = invalid.begin(0, json!({})).unwrap();
    assert_eq!(invalid.receive(0, 1, json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"positionEncoding":"utf-8"}}})), Err(Failure::UnsupportedEncoding));
    assert_eq!(invalid.phase(), Phase::Degraded);
    assert_eq!(invalid.pending_count(), 0);
}

#[test]
fn replay_write_중_편집과_close_open은_최신_mirror까지_전송한_뒤_running이_된다() {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    coordinator.open(document(0, "before")).unwrap();
    let init = coordinator.begin(0, json!({})).unwrap();
    coordinator.receive(0, 1, json!({"jsonrpc":"2.0", "id":init["id"], "result":{"capabilities":{"textDocumentSync":1}}})).unwrap();
    assert!(coordinator
        .change(DOCUMENT_URI, 0, 1, "during replay".into())
        .unwrap()
        .is_none());
    let delta = coordinator.finish_replay(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Replaying);
    assert_eq!(delta[0]["method"], "textDocument/didChange");
    assert_eq!(
        delta[0]["params"]["contentChanges"][0]["text"],
        "during replay"
    );
    coordinator.close(DOCUMENT_URI).unwrap();
    let second = DocumentMirror {
        uri: "file:///synthetic/second.rs".into(),
        ..document(0, "second")
    };
    coordinator.open(second).unwrap();
    let delta = coordinator.finish_replay(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Replaying);
    assert_eq!(delta[0]["method"], "textDocument/didClose");
    assert_eq!(delta[1]["method"], "textDocument/didOpen");
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    assert_eq!(coordinator.phase(), Phase::Running);
}

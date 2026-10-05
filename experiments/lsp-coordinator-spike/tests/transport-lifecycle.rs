use serde_json::json;
use taide_lsp::native::{DocumentMirror, Failure, LspCoordinator, Phase};

const REQUEST_TIMEOUT_MS: u64 = 100;
const DOCUMENT_URI: &str = "file:///synthetic/native.rs";

fn initialized() -> LspCoordinator {
    let mut coordinator = LspCoordinator::new(REQUEST_TIMEOUT_MS);
    let initialize = coordinator.begin(0, json!({})).unwrap();
    coordinator
        .receive(
            0,
            1,
            json!({"jsonrpc":"2.0","id":initialize["id"],"result":{"capabilities":{"hoverProvider":true,"textDocumentSync":1}}}),
        )
        .unwrap();
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    coordinator
}

#[test]
fn disconnect는_pending과_capability를_폐기하되_최신_mirror를_다음_generation에_보존한다() {
    let mut coordinator = initialized();
    coordinator
        .open(DocumentMirror {
            uri: DOCUMENT_URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: "처음".into(),
        })
        .unwrap();
    let first = coordinator
        .request(1, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 0)))
        .unwrap();
    coordinator
        .request(2, "textDocument/hover", json!({}), Some((DOCUMENT_URI, 0)))
        .unwrap();
    assert_eq!(coordinator.request_deadline(), Some(1 + REQUEST_TIMEOUT_MS));
    assert_eq!(
        coordinator.process_disconnected(1),
        Err(Failure::StaleGeneration)
    );
    assert_eq!(coordinator.phase(), Phase::Running);
    let outcome = coordinator.process_disconnected(0).unwrap();
    assert!(outcome.outgoing.is_empty());
    assert_eq!(outcome.completed.len(), 2);
    assert!(outcome
        .completed
        .iter()
        .all(|completion| completion.generation == 0
            && completion.result == Err(Failure::TransportClosed)));
    assert_eq!(coordinator.phase(), Phase::Degraded);
    assert_eq!(coordinator.last_failure(), Some(&Failure::TransportClosed));
    assert_eq!(coordinator.pending_count(), 0);
    assert_eq!(coordinator.request_deadline(), None);
    assert!(!coordinator.supports("textDocument/hover"));
    assert!(coordinator.expire(u64::MAX).unwrap().outgoing.is_empty());
    coordinator
        .change(DOCUMENT_URI, 0, 1, "최신 e\u{301} 𐐀".into())
        .unwrap();
    let restarting = coordinator.restart(3, json!({})).unwrap();
    assert_eq!(coordinator.generation(), 1);
    assert_eq!(coordinator.request_deadline(), Some(3 + REQUEST_TIMEOUT_MS));
    assert!(coordinator
        .receive(
            0,
            4,
            json!({"jsonrpc":"2.0","id":first["id"],"result":null})
        )
        .unwrap()
        .completed
        .is_empty());
    let replay = coordinator
        .receive(
            1,
            4,
            json!({"jsonrpc":"2.0","id":restarting.outgoing[0]["id"],"result":{"capabilities":{"hoverProvider":true,"textDocumentSync":1}}}),
        )
        .unwrap();
    assert_eq!(
        replay.outgoing[1]["params"]["textDocument"]["text"],
        "최신 e\u{301} 𐐀"
    );
    assert_eq!(replay.outgoing[1]["params"]["textDocument"]["version"], 1);
    assert_eq!(coordinator.request_deadline(), None);
    assert!(coordinator.finish_replay(1).unwrap().is_empty());
}

#[test]
fn 종료중_disconnect는_shutdown_pending을_종결하지만_회수_ack_전_stopped가_되지_않는다() {
    let mut coordinator = initialized();
    let stopping = coordinator.stop(1).unwrap();
    assert_eq!(stopping.outgoing[0]["method"], "shutdown");
    assert!(coordinator.request_deadline().is_some());
    assert_eq!(coordinator.finish_stop(0), Err(Failure::InvalidPhase));
    let disconnected = coordinator.process_disconnected(0).unwrap();
    assert!(disconnected.outgoing.is_empty());
    assert_eq!(disconnected.completed.len(), 1);
    assert_eq!(disconnected.completed[0].result, Err(Failure::Stopped));
    assert_eq!(coordinator.request_deadline(), None);
    assert_eq!(coordinator.phase(), Phase::Stopping);
    coordinator.finish_stop(0).unwrap();
    assert_eq!(coordinator.phase(), Phase::Stopped);
    assert_eq!(
        coordinator.process_disconnected(0),
        Err(Failure::InvalidPhase)
    );
}

use serde_json::{json, Value};
use taide_lsp_coordinator_spike::{CoordinatorProbe, DocumentMirror, Failure, Phase};

const REQUEST_TIMEOUT_MS: u64 = 500;
const DOCUMENT_URI: &str = "file:///synthetic/sync.rs";
const INCREMENTAL_SYNC_KIND: u64 = 2;

fn document(text: &str) -> DocumentMirror {
    DocumentMirror {
        uri: DOCUMENT_URI.into(),
        language_id: "rust".into(),
        revision: 0,
        version: 1,
        text: text.into(),
    }
}

fn initialize(sync: Option<Value>, text: &str) -> (CoordinatorProbe, Vec<Value>) {
    let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
    coordinator.open(document(text)).unwrap();
    let request = coordinator.begin(0, json!({})).unwrap();
    let mut capabilities = json!({});
    if let Some(sync) = sync {
        capabilities["textDocumentSync"] = sync;
    }
    let reply = coordinator
        .receive(
            0,
            1,
            json!({
                "jsonrpc":"2.0","id":request["id"],"result":{"capabilities":capabilities}
            }),
        )
        .unwrap();
    (coordinator, reply.outgoing)
}

#[test]
fn sync_none_full_및_options은_open_change_close_save의_광고값을_따른다() {
    for sync in [
        None,
        Some(json!(0)),
        Some(json!({})),
        Some(json!({"openClose":false,"change":0,"save":false})),
    ] {
        let (mut coordinator, messages) = initialize(sync, "before");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["method"], "initialized");
        assert!(coordinator.finish_replay(0).unwrap().is_empty());
        assert!(coordinator
            .change(DOCUMENT_URI, 0, 1, "after".into())
            .unwrap()
            .is_none());
        assert!(coordinator.saved(DOCUMENT_URI).unwrap().is_none());
        assert!(coordinator.close(DOCUMENT_URI).unwrap().is_none());
    }
    for sync in [
        json!(1),
        json!({"openClose":true,"change":1,"save":true}),
        json!({"openClose":true,"change":1,"save":{"includeText":true}}),
    ] {
        let includes_text = sync["save"]["includeText"] == true;
        let (mut coordinator, messages) = initialize(Some(sync), "before");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1]["method"], "textDocument/didOpen");
        assert!(coordinator.finish_replay(0).unwrap().is_empty());
        let changed = coordinator
            .change(DOCUMENT_URI, 0, 1, "after".into())
            .unwrap()
            .unwrap();
        assert_eq!(
            changed["params"]["contentChanges"],
            json!([{"text":"after"}])
        );
        let saved = coordinator.saved(DOCUMENT_URI).unwrap().unwrap();
        assert_eq!(saved["method"], "textDocument/didSave");
        assert_eq!(saved["params"].get("text").is_some(), includes_text);
        if includes_text {
            assert_eq!(saved["params"]["text"], "after");
        }
        assert_eq!(
            coordinator.close(DOCUMENT_URI).unwrap().unwrap()["method"],
            "textDocument/didClose"
        );
    }
}

#[test]
fn incremental은_utf16_문서_범위를_쓰고_replay_delta도_이전_mirror에_대해_계산한다() {
    let fixtures = [
        ("", json!({"line":0,"character":0})),
        ("한e\u{301}𐐀", json!({"line":0,"character":5})),
        ("a\r\n日本\r한\n𐐀e\u{301}", json!({"line":3,"character":4})),
        ("한\r\n", json!({"line":1,"character":0})),
    ];
    for (before, end) in fixtures {
        let (mut coordinator, _) = initialize(Some(json!(INCREMENTAL_SYNC_KIND)), before);
        assert!(coordinator.finish_replay(0).unwrap().is_empty());
        let changed = coordinator
            .change(DOCUMENT_URI, 0, 1, "new 한글".into())
            .unwrap()
            .unwrap();
        assert_eq!(
            changed["params"]["contentChanges"][0],
            json!({
                "range":{"start":{"line":0,"character":0},"end":end},"text":"new 한글"
            })
        );
    }
    let (mut coordinator, _) = initialize(
        Some(json!({"openClose":true,"change":INCREMENTAL_SYNC_KIND})),
        "한𐐀",
    );
    assert!(coordinator
        .change(DOCUMENT_URI, 0, 1, "first\r\n".into())
        .unwrap()
        .is_none());
    let delta = coordinator.finish_replay(0).unwrap();
    assert_eq!(
        delta[0]["params"]["contentChanges"][0]["range"]["end"],
        json!({"line":0,"character":3})
    );
    assert_eq!(coordinator.phase(), Phase::Replaying);
    assert!(coordinator
        .change(DOCUMENT_URI, 1, 2, "last".into())
        .unwrap()
        .is_none());
    let delta = coordinator.finish_replay(0).unwrap();
    assert_eq!(
        delta[0]["params"]["contentChanges"][0]["range"]["end"],
        json!({"line":1,"character":0})
    );
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    assert_eq!(coordinator.phase(), Phase::Running);
}

#[test]
fn malformed_sync_capability는_pending을_제거하고_degraded로_전환한다() {
    for sync in [
        json!(3),
        json!(-1),
        Value::Null,
        json!("full"),
        json!({"openClose":1}),
        json!({"change":true}),
        json!({"save":null}),
        json!({"save":{"includeText":"yes"}}),
    ] {
        let mut coordinator = CoordinatorProbe::new(REQUEST_TIMEOUT_MS);
        let request = coordinator.begin(0, json!({})).unwrap();
        assert_eq!(coordinator.receive(0, 1, json!({
            "jsonrpc":"2.0","id":request["id"],"result":{"capabilities":{"textDocumentSync":sync}}
        })), Err(Failure::MalformedResponse));
        assert_eq!(coordinator.phase(), Phase::Degraded);
        assert_eq!(coordinator.pending_count(), 0);
    }
}

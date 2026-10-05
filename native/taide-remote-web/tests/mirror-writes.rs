use std::time::Duration;

use serde_json::{Value, json};
use taide_model::ids::{ProjectId, TabId};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_remote_web::mirror_writes::{
    ClearExpected, MIRROR_DEBOUNCE, MirrorError, MirrorFlushStatus, MirrorKey, MirrorOutcome,
    MirrorWrites,
};
use taide_remote_web::shell::Failure;
use taide_remote_web::{InvokeError, ResponsePayload};

const WRITE_SEQ: u32 = 1;
const CLEAR_SEQ: u32 = 2;
const RETRY_SEQ: u32 = 3;
const NEXT_SEQ: u32 = 4;
const LATE_SEQ: u32 = 5;
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 128;

fn limits() -> EditorLimits {
    EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    }
}

fn key() -> MirrorKey {
    MirrorKey {
        project: ProjectId("project".into()),
        path: "/project/file".into(),
    }
}

fn receipt(content: &str) -> Result<ResponsePayload, Value> {
    Ok(ResponsePayload::Json(
        json!({"writeId":"mirrorwrite-synthetic","entry":{
            "path":key().path,"content":content,"savedAtMs":1,"diskModifiedMs":1,"conflict":false,"sourceMissing":false,
        }}),
    ))
}

#[test]
fn 미러_쓰기의_지연_병합_저장_epoch와_정확한_후속_정리를_소유한다() {
    let mut store = EditorStore::new(limits()).unwrap();
    let document = store
        .open_untitled(TabId("tab".into()), "draft", "text".into())
        .unwrap();
    let mut writes = MirrorWrites::default();
    let key = key();
    writes.observe(key.clone(), document, 1, Duration::ZERO);
    assert!(
        writes
            .ready(MIRROR_DEBOUNCE - Duration::from_millis(1))
            .is_empty()
    );
    assert_eq!(
        writes.ready(MIRROR_DEBOUNCE).as_slice(),
        std::slice::from_ref(&key)
    );
    let request = writes.prepare(&key, 1, "draft".into()).unwrap();
    assert_eq!(request.call().command, "file_mirror_dirty");
    assert_eq!(
        request.call().args,
        json!({"projectId":"project","path":key.path,"content":"draft","receipt":true})
    );
    writes.sent(request, WRITE_SEQ);
    writes.observe(key.clone(), document, 2, MIRROR_DEBOUNCE);
    assert!(writes.ready(MIRROR_DEBOUNCE + MIRROR_DEBOUNCE).is_empty());
    assert_eq!(writes.next_deadline(), None);
    writes.settle(&key, true, MIRROR_DEBOUNCE);
    assert!(
        writes.response(WRITE_SEQ, &receipt("draft"), MIRROR_DEBOUNCE, |_| Some(
            "next".into()
        ))
    );
    let clear = writes.next_clears().pop().unwrap();
    assert_eq!(
        clear.call().args["expectedReceipt"]["writeId"],
        "mirrorwrite-synthetic"
    );
    let request = writes.prepare(&key, 2, "next".into()).unwrap();
    writes.sent(request, NEXT_SEQ);
    writes.clear_sent(clear, CLEAR_SEQ);
    assert!(writes.response(
        CLEAR_SEQ,
        &Ok(ResponsePayload::Json(json!(false))),
        MIRROR_DEBOUNCE,
        |_| None
    ));
    assert!(matches!(
        writes.take_outcomes().as_slice(),
        [MirrorOutcome::Invalidated(_)]
    ));
    assert!(writes.response(NEXT_SEQ, &receipt("next"), MIRROR_DEBOUNCE, |_| None));
    assert!(matches!(
        writes.take_outcomes().as_slice(),
        [MirrorOutcome::Committed { .. }]
    ));
    assert_eq!(writes.flush_status(), MirrorFlushStatus::Ready);
    writes.flush(MIRROR_DEBOUNCE);
    assert!(writes.ready(MIRROR_DEBOUNCE).is_empty());
    writes.observe(key.clone(), document, 3, MIRROR_DEBOUNCE);
    let request = writes.prepare(&key, 3, "last".into()).unwrap();
    writes.sent(request, LATE_SEQ);
    writes.settle(&key, false, MIRROR_DEBOUNCE);
    assert!(
        writes.response(LATE_SEQ, &receipt("last"), MIRROR_DEBOUNCE, |_| Some(
            "last".into()
        ))
    );
    assert_eq!(writes.next_clears().len(), 1);
    assert!(writes.take_outcomes().is_empty());
}

#[test]
fn 같은_초안의_늦은_쓰기만_현재_epoch로_다시_쓰고_해제_초안을_즉시_flush한다() {
    let mut store = EditorStore::new(limits()).unwrap();
    let document = store
        .open_untitled(TabId("tab".into()), "draft", "text".into())
        .unwrap();
    let mut writes = MirrorWrites::default();
    let key = key();
    writes.observe(key.clone(), document, 1, Duration::ZERO);
    let request = writes.prepare(&key, 1, "draft".into()).unwrap();
    writes.sent(request, WRITE_SEQ);
    writes.observe(key.clone(), document, 2, Duration::ZERO);
    writes.settle(&key, true, Duration::ZERO);
    writes.response(WRITE_SEQ, &receipt("draft"), Duration::ZERO, |_| {
        Some("draft".into())
    });
    assert!(writes.next_clears().is_empty());
    assert!(writes.take_outcomes().is_empty());
    assert_eq!(
        writes.ready(Duration::ZERO).as_slice(),
        std::slice::from_ref(&key)
    );
    let request = writes.prepare(&key, 2, "draft".into()).unwrap();
    writes.sent(request, RETRY_SEQ);
    writes.response(RETRY_SEQ, &receipt("draft"), Duration::ZERO, |_| None);
    writes.observe(key.clone(), document, 3, Duration::ZERO);
    writes.retain_active(|_| false, Duration::ZERO);
    assert_eq!(
        writes.ready(Duration::ZERO).as_slice(),
        std::slice::from_ref(&key)
    );
    let request = writes.prepare(&key, 3, "draft".into()).unwrap();
    writes.sent(request, NEXT_SEQ);
    writes.response(NEXT_SEQ, &receipt("draft"), Duration::ZERO, |_| None);
    assert_eq!(writes.document(&key), None);
    assert_eq!(writes.flush_status(), MirrorFlushStatus::Ready);
}

#[test]
fn 미러_오류_종료와_해제_재바인딩은_자동_재전송없이_현재_초안과_정리_receipt를_보존한다() {
    let mut store = EditorStore::new(limits()).unwrap();
    let document = store
        .open_untitled(TabId("first".into()), "first", "text".into())
        .unwrap();
    let replacement = store
        .open_untitled(TabId("second".into()), "second", "text".into())
        .unwrap();
    let key = key();
    let mut writes = MirrorWrites::default();
    writes.observe(key.clone(), document, 1, Duration::ZERO);
    let request = writes.prepare(&key, 1, "first".into()).unwrap();
    writes.sent(request, WRITE_SEQ);
    writes.observe(key.clone(), replacement, 1, Duration::ZERO);
    writes.response(WRITE_SEQ, &receipt("first"), Duration::ZERO, |_| {
        Some("second".into())
    });
    let clear = writes.next_clears().pop().unwrap();
    writes.clear_sent(clear, CLEAR_SEQ);
    writes.response(
        CLEAR_SEQ,
        &Err(json!({"synthetic":"clear"})),
        Duration::ZERO,
        |_| None,
    );
    assert!(writes.next_clears().is_empty());
    assert!(matches!(
        writes.flush_status(),
        MirrorFlushStatus::Failed(MirrorError::Rpc(Failure::Remote(_)))
    ));
    writes.disconnected();
    assert!(writes.ready(MIRROR_DEBOUNCE).is_empty());
    assert!(writes.next_clears().is_empty());
    assert_eq!(writes.take_failures().len(), 2);
    writes.retry(Duration::ZERO);
    assert_eq!(
        writes.ready(Duration::ZERO).as_slice(),
        std::slice::from_ref(&key)
    );
    assert_eq!(writes.next_clears().len(), 1);
    let request = writes.prepare(&key, 1, "second".into()).unwrap();
    writes.sent(request, RETRY_SEQ);
    writes.response(
        RETRY_SEQ,
        &Ok(ResponsePayload::Json(Value::Null)),
        Duration::ZERO,
        |_| None,
    );
    assert_eq!(
        writes.take_failures()[0].error,
        MirrorError::Rpc(Failure::MalformedResponse)
    );
    writes.observe(key.clone(), replacement, 2, Duration::ZERO);
    let request = writes.prepare(&key, 2, "fresh".into()).unwrap();
    writes.invocation_failed(request, InvokeError::Closed);
    assert!(writes.ready(MIRROR_DEBOUNCE).is_empty());
    let clear = writes.next_clears().pop().unwrap();
    writes.clear_sent(clear, NEXT_SEQ);
    writes.response(
        NEXT_SEQ,
        &Ok(ResponsePayload::Binary(vec![])),
        Duration::ZERO,
        |_| None,
    );
    assert!(writes.next_clears().is_empty());
    writes.queue_clear(key, ClearExpected::Listed(None));
    assert_eq!(writes.next_clears()[0].call().args["expected"], Value::Null);
    assert!(!writes.response(LATE_SEQ, &receipt("ignored"), Duration::ZERO, |_| None));
}

use serde_json::{Value, json};
use taide_model::ids::TabId;
use taide_remote_web::dirty::{DirtyState, DirtyUpdate, FlushState};
use taide_remote_web::shell::Failure;
use taide_remote_web::{InvokeError, ResponsePayload};

const FIRST_SEQ: u32 = 1;
const SECOND_SEQ: u32 = 2;
const CLEAN_SEQ: u32 = 3;

fn tab(value: &str) -> TabId {
    TabId(value.into())
}

fn ack(state: &mut DirtyState, seq: u32) {
    assert!(state.response(seq, &Ok(ResponsePayload::Json(Value::Null))));
}

#[test]
fn dirty_owner는_전송전_closed도_한번만_공개하고_명시_retry까지_보류한다() {
    let mut state = DirtyState::default();
    state.observe(tab("queued"), true);
    state.disconnected();
    let failures = state.take_failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].update,
        DirtyUpdate {
            tab: tab("queued"),
            dirty: true
        }
    );
    assert_eq!(failures[0].error, Failure::Invocation(InvokeError::Closed));
    state.disconnected();
    assert!(state.take_failures().is_empty());
    assert!(state.next_updates().is_empty());
    state.retry();
    assert_eq!(
        state.next_updates(),
        [DirtyUpdate {
            tab: tab("queued"),
            dirty: true
        }]
    );
}

#[test]
fn dirty_owner는_탭별_전이_병합_진행중_새값과_해제뒤_늦은_응답을_소유한다() {
    let mut state = DirtyState::default();
    state.observe(tab("first"), false);
    assert!(state.next_updates().is_empty());
    assert!(matches!(state.flush_state(), FlushState::Ready));
    state.observe(tab("first"), true);
    state.observe(tab("first"), true);
    state.observe(tab("second"), true);
    let updates = state.next_updates();
    assert_eq!(
        updates,
        [
            DirtyUpdate {
                tab: tab("first"),
                dirty: true
            },
            DirtyUpdate {
                tab: tab("second"),
                dirty: true
            }
        ]
    );
    assert_eq!(updates[0].call().command, "layout_set_dirty");
    assert_eq!(
        updates[0].call().args,
        json!({"tabId":"first","dirty":true})
    );
    state.sent(updates[0].clone(), FIRST_SEQ);
    state.sent(updates[1].clone(), SECOND_SEQ);
    state.observe(tab("first"), false);
    assert!(state.next_updates().is_empty());
    assert!(matches!(state.flush_state(), FlushState::Pending));
    ack(&mut state, FIRST_SEQ);
    let pending = state.next_updates();
    let [clean] = pending.as_slice() else {
        panic!("expected one latest update");
    };
    assert!(!clean.dirty);
    state.sent(clean.clone(), CLEAN_SEQ);
    state.retain(|key| key == &tab("first"));
    ack(&mut state, SECOND_SEQ);
    ack(&mut state, CLEAN_SEQ);
    assert!(!state.response(SECOND_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
    assert!(state.next_updates().is_empty());
    assert!(matches!(state.flush_state(), FlushState::Ready));
    assert!(state.take_failures().is_empty());
}

#[test]
fn dirty_owner는_오류를_공개하고_closed_불확정값을_재전송하지_않다가_명시_retry한다() {
    for result in [
        Ok(ResponsePayload::Binary(vec![])),
        Err(json!({"code":"SYNTHETIC"})),
    ] {
        let mut state = DirtyState::default();
        state.observe(tab("first"), true);
        let update = state.next_updates().remove(0);
        state.sent(update.clone(), FIRST_SEQ);
        assert!(state.response(FIRST_SEQ, &result));
        assert!(matches!(state.flush_state(), FlushState::Failed(_)));
        state.observe(tab("first"), true);
        assert!(state.next_updates().is_empty());
        let failures = state.take_failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].update, update);
        state.retry();
        state.sent(state.next_updates().remove(0), SECOND_SEQ);
        state.observe(tab("first"), false);
        state.disconnected();
        assert!(matches!(
            state.flush_state(),
            FlushState::Failed(Failure::Invocation(InvokeError::Closed))
        ));
        assert!(state.next_updates().is_empty());
        assert!(!state.response(SECOND_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
        state.retry();
        let updates = state.next_updates();
        assert_eq!(
            updates,
            [DirtyUpdate {
                tab: tab("first"),
                dirty: false
            }]
        );
        state.sent(updates[0].clone(), CLEAN_SEQ);
        ack(&mut state, CLEAN_SEQ);
        assert!(matches!(state.flush_state(), FlushState::Ready));
    }
}

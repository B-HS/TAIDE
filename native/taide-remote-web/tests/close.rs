use taide_remote_web::InvokeError;
use taide_remote_web::close::{CloseFailure, CloseState, drain_state};
use taide_remote_web::dirty::FlushState;
use taide_remote_web::mirror_writes::{MirrorError, MirrorFlushStatus};
use taide_remote_web::shell::Failure;

#[test]
fn 종료는_진행_작업_dirty와_미러를_모두_기다리고_실패를_완료로_바꾸지_않는다() {
    assert_eq!(
        drain_state(false, FlushState::Ready, MirrorFlushStatus::Ready),
        CloseState::Ready
    );
    assert_eq!(
        drain_state(true, FlushState::Ready, MirrorFlushStatus::Ready),
        CloseState::Pending
    );
    assert_eq!(
        drain_state(false, FlushState::Pending, MirrorFlushStatus::Ready),
        CloseState::Pending
    );
    assert_eq!(
        drain_state(false, FlushState::Ready, MirrorFlushStatus::Pending),
        CloseState::Pending
    );
    let dirty = Failure::Invocation(InvokeError::Closed);
    assert_eq!(
        drain_state(true, FlushState::Failed(&dirty), MirrorFlushStatus::Pending),
        CloseState::Failed(CloseFailure::Dirty(dirty.clone()))
    );
    let mirror = MirrorError::Rpc(dirty);
    assert_eq!(
        drain_state(
            true,
            FlushState::Ready,
            MirrorFlushStatus::Failed(mirror.clone())
        ),
        CloseState::Failed(CloseFailure::Mirror(mirror))
    );
}

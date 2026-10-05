use crate::dirty::FlushState;
use crate::files::FileFailure;
use crate::mirror_writes::{MirrorError, MirrorFlushStatus};
use crate::shell::Failure;

#[derive(Debug, Clone, PartialEq)]
pub enum CloseFailure {
    File(FileFailure),
    Preference(Failure),
    Theme(Failure),
    Snippet(Failure),
    AppFile(Failure),
    Dirty(Failure),
    Mirror(MirrorError),
    SchedulerUnavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CloseState {
    Open,
    Pending,
    Ready,
    Failed(CloseFailure),
}

pub fn drain_state(
    pending_file_operations: bool,
    dirty: FlushState<'_>,
    mirror: MirrorFlushStatus,
) -> CloseState {
    if let FlushState::Failed(error) = &dirty {
        return CloseState::Failed(CloseFailure::Dirty((*error).clone()));
    }
    if let MirrorFlushStatus::Failed(error) = mirror {
        return CloseState::Failed(CloseFailure::Mirror(error));
    }
    if pending_file_operations
        || matches!(dirty, FlushState::Pending)
        || matches!(mirror, MirrorFlushStatus::Pending)
    {
        return CloseState::Pending;
    }
    CloseState::Ready
}

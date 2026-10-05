use crate::document::{DocumentKey, EditorError};
use crate::save_cleanup::{CleanupFlags, run};
use crate::store::{EditorStore, SaveSnapshot};
use crate::view::ViewId;

pub struct PreparedSave {
    pub snapshot: SaveSnapshot,
    pub changed: bool,
}

pub fn prepare(
    store: &mut EditorStore,
    requested: SaveSnapshot,
    view: Option<ViewId>,
    flags: CleanupFlags,
    auto_save: bool,
) -> Result<Option<PreparedSave>, EditorError> {
    let current = store.documents().snapshot(requested.document())?;
    if &current.key != requested.key() {
        return Err(EditorError::InvalidIdentity);
    }
    if current.revision != requested.revision() {
        return Err(EditorError::StaleRevision);
    }
    if current.metadata.read_only {
        return Err(EditorError::ReadOnly);
    }
    if !matches!(current.key, DocumentKey::File(_)) {
        return Err(EditorError::InvalidIdentity);
    }
    if !current.dirty {
        return Ok(None);
    }
    let output = run(store, current.id, view, flags, auto_save)?;
    Ok(Some(PreparedSave {
        snapshot: store.save_snapshot(current.id)?,
        changed: output.changed,
    }))
}

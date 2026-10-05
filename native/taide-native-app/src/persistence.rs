use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::file::REFUSED_FILE_BYTES;
use taide_model::ids::ProjectId;
use taide_model::layout::TabKind;
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot};
use taide_runtime::AppServices;

pub const MIRROR_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct DraftEpoch(Arc<AtomicBool>);

impl Default for DraftEpoch {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
}

impl DraftEpoch {
    pub fn invalidate(&self) {
        self.0.store(false, Ordering::Release);
    }

    pub fn is_current(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn same_generation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct MirrorJob {
    pub snapshot: DocumentSnapshot,
    pub project: ProjectId,
    pub epoch: DraftEpoch,
}

#[derive(Default)]
struct Entry {
    revision: u64,
    epoch: DraftEpoch,
    mirror_due: Option<Instant>,
    mirror_pending: bool,
    mirror_inflight: Option<(u64, DraftEpoch)>,
    save_due: Option<Instant>,
    saving: bool,
}

pub struct Due {
    pub document: DocumentId,
    pub mirror: bool,
    pub save: bool,
}

#[derive(Default)]
pub struct Persistence {
    entries: HashMap<DocumentId, Entry>,
}

impl Persistence {
    pub fn changed(
        &mut self,
        document: DocumentId,
        revision: u64,
        now: Instant,
        mirror: bool,
        auto_save: Duration,
    ) {
        let entry = self.entries.entry(document).or_default();
        entry.revision = revision;
        if mirror {
            entry.mirror_pending = true;
            entry.mirror_due = now.checked_add(MIRROR_DEBOUNCE);
        }
        entry.save_due = None;
        if !entry.saving && !auto_save.is_zero() {
            entry.save_due = now.checked_add(auto_save);
        }
    }

    pub fn due(&self, now: Instant) -> Vec<Due> {
        self.entries
            .iter()
            .filter_map(|(document, entry)| {
                if entry.saving {
                    return None;
                }
                let mirror = entry.mirror_pending
                    && entry.mirror_inflight.is_none()
                    && entry.mirror_due.is_some_and(|deadline| deadline <= now);
                let save = entry.save_due.is_some_and(|deadline| deadline <= now);
                (mirror || save).then_some(Due {
                    document: *document,
                    mirror,
                    save,
                })
            })
            .collect()
    }

    pub fn next_wake(&self) -> Option<Instant> {
        self.entries
            .values()
            .filter(|entry| !entry.saving)
            .flat_map(|entry| {
                let mirror = if entry.mirror_inflight.is_none() {
                    entry.mirror_due
                } else {
                    None
                };
                [mirror, entry.save_due].into_iter().flatten()
            })
            .min()
    }

    pub fn mirror_job(&self, snapshot: DocumentSnapshot, project: ProjectId) -> Option<MirrorJob> {
        let entry = self.entries.get(&snapshot.id)?;
        if entry.saving || entry.mirror_inflight.is_some() || !entry.mirror_pending {
            return None;
        }
        Some(MirrorJob {
            snapshot,
            project,
            epoch: entry.epoch.clone(),
        })
    }

    pub fn mirror_started(&mut self, job: &MirrorJob) {
        if let Some(entry) = self.entries.get_mut(&job.snapshot.id) {
            entry.mirror_inflight = Some((job.snapshot.revision, job.epoch.clone()));
            entry.mirror_due = None;
        }
    }

    pub fn mirror_finished(&mut self, job: &MirrorJob, applied: bool, now: Instant) {
        let Some(entry) = self.entries.get_mut(&job.snapshot.id) else {
            return;
        };
        if !entry.epoch.same_generation(&job.epoch) {
            return;
        }
        entry.mirror_inflight = None;
        if applied && entry.revision == job.snapshot.revision {
            entry.mirror_pending = false;
            entry.mirror_due = None;
        } else if entry.revision != job.snapshot.revision && entry.mirror_due.is_none() {
            entry.mirror_due = Some(now);
        }
    }

    pub fn begin_save(&mut self, document: DocumentId) -> Option<DraftEpoch> {
        let entry = self.entries.entry(document).or_default();
        if entry.saving {
            return None;
        }
        entry.saving = true;
        Some(entry.epoch.clone())
    }

    pub fn submission_failed(&mut self, document: DocumentId) {
        if let Some(entry) = self.entries.get_mut(&document) {
            entry.saving = false;
        }
    }

    pub fn disable_auto_save(&mut self, document: DocumentId) {
        if let Some(entry) = self.entries.get_mut(&document) {
            entry.save_due = None;
        }
    }

    pub fn save_finished(
        &mut self,
        document: DocumentId,
        now: Instant,
        dirty: bool,
        mirror: bool,
        auto_save: Duration,
    ) {
        if !dirty {
            self.settled(document);
            return;
        }
        let entry = self.entries.entry(document).or_default();
        entry.epoch.invalidate();
        entry.epoch = DraftEpoch::default();
        entry.saving = false;
        entry.mirror_inflight = None;
        entry.mirror_pending = mirror;
        entry.mirror_due = mirror.then_some(now);
        entry.save_due = if auto_save.is_zero() {
            None
        } else {
            now.checked_add(auto_save)
        };
    }

    pub fn settled(&mut self, document: DocumentId) {
        if let Some(entry) = self.entries.remove(&document) {
            entry.epoch.invalidate();
        }
    }

    pub fn cancel_inflight(&mut self) {
        for entry in self.entries.values_mut() {
            entry.epoch.invalidate();
            entry.epoch = DraftEpoch::default();
            entry.saving = false;
            entry.mirror_inflight = None;
            entry.save_due = None;
        }
    }
}

pub async fn write_mirror(services: &Arc<AppServices>, job: MirrorJob) -> AppResult<bool> {
    let operation = services
        .tasks
        .begin_operation("native-draft-mirror")
        .ok_or_else(|| AppError::Forbidden("native mirror write is stopping".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let services = services.clone();
    let tasks = services.tasks.clone();
    tasks.run_blocking_result("native-draft-mirror", move || {
        let _operation = operation;
        let _guard = guard;
        if !job.epoch.is_current() { return Ok(false); }
        if services.state.is_shutting_down() {
            return Err(AppError::Forbidden("native mirror write is stopping".into()));
        }
        if job.snapshot.rope.len_bytes() as u64 >= REFUSED_FILE_BYTES {
            return Err(AppError::Forbidden("native mirror exceeds the file policy".into()));
        }
        let root = root_guard::project_root(&services.state.projects.read(), &job.project)?;
        let layouts = services.state.layouts.read();
        let layout = layouts.get(&job.project).ok_or_else(|| AppError::NotFound("native mirror layout".into()))?;
        let tabs = taide_layout::service::all_roots(layout).flat_map(crate::tabs::tabs_in);
        match &job.snapshot.key {
            DocumentKey::File(path) => {
                if root_guard::ensure_within_root(&root, path)? != *path {
                    return Err(AppError::Forbidden("native mirror canonical identity changed".into()));
                }
                let exists = tabs.into_iter().any(|tab| matches!(&tab.kind, TabKind::File { path: display } if root_guard::ensure_within_root(&root, Path::new(display)).is_ok_and(|canonical| canonical == *path)));
                if !exists { return Ok(false); }
                taide_file::service::mirror_dirty(&services.state.paths, &job.project, path,
                    path.to_str().ok_or_else(|| AppError::InvalidArgument("native mirror path is not UTF-8".into()))?, &job.snapshot.rope.to_string())?;
            }
            DocumentKey::Untitled(tab_id) => {
                root_guard::ensure_safe_component(tab_id.as_str())?;
                if !tabs.into_iter().any(|tab| &tab.id == tab_id && matches!(tab.kind, TabKind::Untitled { .. })) { return Ok(false); }
                taide_file::service::mirror_untitled(&services.state.paths, &job.project, tab_id, &job.snapshot.rope.to_string())?;
            }
            DocumentKey::AppFile(_) => {
                return Err(AppError::Forbidden("native app files cannot be mirrored".into()));
            }
        }
        Ok(true)
    }).await
}

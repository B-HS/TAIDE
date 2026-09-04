use std::path::Path;

use tauri::AppHandle;

use crate::ids::ProjectId;
use crate::state::AppState;

use super::types::{CapabilityKind, Project};

/// The registration half of one capability's attach — the cheap `AppState`/store write that
/// finishes whatever [`ProjectCapability::build_attachment`] produced. Existing as a separate,
/// deferrable value is the whole point of the split: it lets the expensive half run with **no**
/// `AppState::begin_mutation` held (`domain::project::commands::attach_project_capabilities`),
/// which is what keeps a multi-second watcher walk from freezing every other mutation in the app
/// while a project opens.
///
/// It is an opaque boxed closure rather than a shared payload enum on purpose: each capability
/// keeps the handle it carries private to its own domain (`project` must not learn
/// `infra::watcher::WatcherHandle` or `ProjectLayout` just to ferry them across the two phases),
/// and the registry only needs to know *that* a registration is pending and in what order to run
/// it.
///
/// The closure deliberately takes no [`AppHandle`]: a capability that needs one in the register
/// phase clones it into the closure during the build phase instead. That keeps the entire commit
/// path callable — and therefore unit-testable — without a running app, which an `AppHandle` has
/// no constructor outside of.
///
/// **Dropping an attachment instead of committing it is a supported outcome, not a leak.** It
/// releases whatever the build produced (a watcher handle dropped here stops its debouncer), which
/// is exactly what the open path does when the project turns out to have been closed while the
/// build ran unguarded.
type AttachmentRegister = Box<dyn FnOnce(&AppState, &Project) + Send>;

pub struct ProjectAttachment(Option<AttachmentRegister>);

impl ProjectAttachment {
    /// Nothing is left to register — either the capability owns no per-project `AppState` entry at
    /// all, or its whole attach was side-effect work (writing a lockfile, spawning a task) that the
    /// guard-free build phase already performed.
    pub fn none() -> Self {
        Self(None)
    }

    /// Defers `register` to the guarded commit phase. Everything it needs must already be owned by
    /// the closure — it runs while the caller holds the mutation guard, so it must do nothing but
    /// the store write itself.
    pub fn new(register: impl FnOnce(&AppState, &Project) + Send + 'static) -> Self {
        Self(Some(Box::new(register)))
    }

    fn commit(self, state: &AppState, project: &Project) {
        if let Some(register) = self.0 {
            register(state, project);
        }
    }
}

/// The project-lifecycle extension point declared by `architecture.md` §3: each domain owns the
/// attach/detach of its own per-project resources, and `project_open`/`project_close` only walk the
/// registered implementations instead of calling into other domains by hand. Implementations are
/// registered statically by the assembly (`lib.rs`'s `project_capabilities`) — there is no dynamic
/// plugin registry.
///
/// Every method has a no-op default so a capability that only acquires (e.g. agent hooks) or only
/// reaps (e.g. per-project caches) implements exactly the half it owns. `detected_kind` reports the
/// serialized [`CapabilityKind`] this capability contributes to `Project.capabilities` for a given
/// root, or `None` for lifecycle hooks that have no serialized kind; the registry's
/// [`ProjectCapabilities::detected_kinds`] is the **single source** of that field —
/// `project_open` injects it into `project::service::open_project`, which records the result
/// verbatim (the values the registry produces are pinned by the assembly's test in `lib.rs`).
pub trait ProjectCapability: Send + Sync {
    fn detected_kind(&self, _root: &Path) -> Option<CapabilityKind> {
        None
    }

    /// The attach, in two stages: everything expensive happens **here**, with no
    /// `AppState::begin_mutation` held, and whatever must land in `AppState` comes back as a
    /// [`ProjectAttachment`] the caller commits under a re-acquired guard afterwards.
    ///
    /// The contract a build must honor:
    /// - **Read `state`, never write it.** `state` is passed only for guard-free reads
    ///   (`AppState::paths`); taking the mutation guard here would reintroduce exactly the hold
    ///   this split removes.
    /// - **Tolerate never being committed.** The project may be closed while the build runs; the
    ///   returned attachment is then dropped instead of committed (see [`ProjectAttachment`]).
    /// - Work that touches no `AppState` at all (writing a file, spawning a task) belongs here
    ///   directly, with [`ProjectAttachment::none`] returned — it is strictly better off outside
    ///   the guard.
    fn build_attachment(&self, _app: &AppHandle, _state: &AppState, _project: &Project) -> ProjectAttachment {
        ProjectAttachment::none()
    }

    fn detach(&self, _app: &AppHandle, _state: &AppState, _project_id: &ProjectId) {}
}

/// The statically assembled capability list. [`Self::build_attachments`], [`Self::commit_attachments`],
/// and [`Self::detach_all`] all walk it in registration order, so **registration order is part of
/// the correctness contract**, not a style choice: `project_close` used to hand-code its reap
/// sequence precisely because two of the reaps are order-sensitive (the layout capability must
/// flush a dirty layout before anything observes the project as gone, and terminals must be reaped
/// during close rather than at app exit — see each capability's `detach` doc for the full
/// rationale). The canonical order lives in one place, `lib.rs`'s `project_capabilities`, and is
/// pinned there by a source-scan test; the forward direction of the shared walk itself is pinned by
/// the recording-stub test in this file.
///
/// The attach side is **two walks, not one** (build then commit), and both go through the same
/// forward traversal — the commit walk replays the vector the build walk produced, position for
/// position — so splitting the attach did not split the order contract: a capability still attaches
/// after every capability registered before it and before every capability registered after it.
pub struct ProjectCapabilities {
    capabilities: Vec<Box<dyn ProjectCapability>>,
}

impl ProjectCapabilities {
    pub fn new(capabilities: Vec<Box<dyn ProjectCapability>>) -> Self {
        Self { capabilities }
    }

    /// The single traversal every registry walk goes through — forward, in registration order.
    /// [`Self::build_attachments`], [`Self::detach_all`], and [`Self::detected_kinds`] must all
    /// iterate via this method (or via [`Self::collect_in_registration_order`], which is itself
    /// built on it) rather than touching `self.capabilities` directly, so the forward-walk
    /// contract has exactly one implementation and the recording-stub test below pins it for all of
    /// them at once (a `tauri::AppHandle` cannot be constructed outside a running app, so
    /// `build_attachments`/`detach_all` themselves are not directly callable from a unit test —
    /// this codebase deliberately has no mock-app harness).
    fn for_each_in_registration_order(&self, mut visit: impl FnMut(&dyn ProjectCapability)) {
        for capability in &self.capabilities {
            visit(capability.as_ref());
        }
    }

    /// [`Self::for_each_in_registration_order`] with a per-capability result collected into a
    /// vector whose indices are the registration indices — the shape both [`Self::detected_kinds`]
    /// and [`Self::build_attachments`] need, and the one the unit test below can drive without an
    /// `AppHandle`.
    fn collect_in_registration_order<T>(&self, mut produce: impl FnMut(&dyn ProjectCapability) -> T) -> Vec<T> {
        let mut collected = Vec::with_capacity(self.capabilities.len());
        self.for_each_in_registration_order(|capability| collected.push(produce(capability)));
        collected
    }

    /// The [`CapabilityKind`]s the registered capabilities detect for `root`, in registration
    /// order — the source `project_open` injects into `project::service::open_project` as
    /// `Project.capabilities`.
    pub fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind> {
        self.collect_in_registration_order(|capability| capability.detected_kind(root))
            .into_iter()
            .flatten()
            .collect()
    }

    /// Attach phase 1 — every capability's [`ProjectCapability::build_attachment`], in registration
    /// order, with **no mutation guard held**. The returned vector is positionally paired with the
    /// registration list, so [`Self::commit_attachments`] replays it in the same order.
    pub fn build_attachments(&self, app: &AppHandle, state: &AppState, project: &Project) -> Vec<ProjectAttachment> {
        self.collect_in_registration_order(|capability| capability.build_attachment(app, state, project))
    }

    /// Attach phase 2 — commits what [`Self::build_attachments`] produced, in the same registration
    /// order, under the caller's re-acquired guard. Callers that decide not to attach after all
    /// (the project closed during the build) simply drop the vector instead of calling this; see
    /// [`ProjectAttachment`].
    pub fn commit_attachments(&self, state: &AppState, project: &Project, attachments: Vec<ProjectAttachment>) {
        debug_assert_eq!(
            attachments.len(),
            self.capabilities.len(),
            "커밋할 attachment 수가 등록된 capability 수와 다릅니다 — 두 단계의 순서 짝이 깨졌습니다"
        );
        for attachment in attachments {
            attachment.commit(state, project);
        }
    }

    pub fn detach_all(&self, app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        self.for_each_in_registration_order(|capability| capability.detach(app, state, project_id));
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;
    use crate::domain::project::types::ProjectDisplay;
    use crate::paths::AppPaths;

    struct KindOnly(Option<CapabilityKind>);

    impl ProjectCapability for KindOnly {
        fn detected_kind(&self, _root: &Path) -> Option<CapabilityKind> {
            self.0
        }
    }

    struct RecordingCapability {
        index: usize,
        log: Arc<Mutex<Vec<usize>>>,
    }

    impl ProjectCapability for RecordingCapability {
        fn detected_kind(&self, _root: &Path) -> Option<CapabilityKind> {
            self.log.lock().push(self.index);
            None
        }
    }

    fn recording_registry(log: &Arc<Mutex<Vec<usize>>>) -> ProjectCapabilities {
        ProjectCapabilities::new(
            (0..3)
                .map(|index| {
                    Box::new(RecordingCapability {
                        index,
                        log: Arc::clone(log),
                    }) as Box<dyn ProjectCapability>
                })
                .collect(),
        )
    }

    fn stub_state() -> AppState {
        AppState::new(AppPaths::new(PathBuf::from("/unused")))
    }

    fn stub_project() -> Project {
        Project {
            id: ProjectId::from("prj-attach".to_string()),
            root: "/repo".to_string(),
            name: "attach".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        }
    }

    /// Records `index` when the attachment is committed — the register half of the two-phase
    /// attach, standing in for the `AppState` write a real capability performs there.
    fn recording_attachment(index: usize, log: &Arc<Mutex<Vec<usize>>>) -> ProjectAttachment {
        let log = Arc::clone(log);
        ProjectAttachment::new(move |_state, _project| log.lock().push(index))
    }

    /// Pins the shared walk (`for_each_in_registration_order`) to **forward** registration order —
    /// the traversal `build_attachments`/`detach_all`/`detected_kinds` all delegate to.
    /// `detach_all`'s direction is a correctness contract (`project_close`'s historical reap
    /// order), and this is the closest a unit test can get to it: `build_attachment`/`detach` take
    /// a `tauri::AppHandle`, which has no constructor outside a running app (no mock-app harness in
    /// this codebase), so the recording happens through the walk's visit path instead.
    #[test]
    fn 공유_순회는_등록_순서_전방이다() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let registry = recording_registry(&log);

        registry.for_each_in_registration_order(|capability| {
            capability.detected_kind(Path::new("/unused"));
        });

        assert_eq!(*log.lock(), vec![0, 1, 2]);
    }

    /// `build_attachments` collects through this helper, so pinning it pins that the build walk
    /// visits every capability exactly once, in registration order, and that the returned vector's
    /// index is the registration index — the pairing `commit_attachments` replays.
    #[test]
    fn 등록_순서_수집은_인덱스와_방문_순서를_모두_보존한다() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let registry = recording_registry(&log);

        let collected = registry.collect_in_registration_order(|capability| {
            capability.detected_kind(Path::new("/unused"));
            log.lock().len()
        });

        assert_eq!(*log.lock(), vec![0, 1, 2], "방문 순서가 등록 순서여야 합니다");
        assert_eq!(collected, vec![1, 2, 3], "수집 결과의 인덱스가 등록 인덱스와 일치해야 합니다");
    }

    #[test]
    fn detected_kinds는_등록_순서를_보존하고_미검출_capability를_거른다() {
        let registry = ProjectCapabilities::new(vec![
            Box::new(KindOnly(None)),
            Box::new(KindOnly(Some(CapabilityKind::Git))),
            Box::new(KindOnly(None)),
            Box::new(KindOnly(Some(CapabilityKind::Terminal))),
        ]);

        assert_eq!(
            registry.detected_kinds(Path::new("/unused")),
            vec![CapabilityKind::Git, CapabilityKind::Terminal],
        );
    }

    /// The register phase must reproduce the build phase's order exactly — that equality is what
    /// keeps `project_open`'s attach order identical to `project_close`'s reap order after the
    /// build/register split (`architecture.md` §3·§6.3).
    #[test]
    fn 커밋은_build_와_같은_등록_순서로_한_번씩만_실행된다() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let registry = recording_registry(&log);
        let attachments = (0..3).map(|index| recording_attachment(index, &log)).collect();

        registry.commit_attachments(&stub_state(), &stub_project(), attachments);

        assert_eq!(*log.lock(), vec![0, 1, 2]);
    }

    /// The close-during-build outcome: dropping the built attachments must register **nothing**.
    /// A partially committed walk would leave a closed project's watcher handle in `AppState` with
    /// no `detach` left to reap it.
    #[test]
    fn 커밋하지_않고_버린_attachment는_아무것도_등록하지_않는다() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let attachments: Vec<ProjectAttachment> = (0..3).map(|index| recording_attachment(index, &log)).collect();

        drop(attachments);

        assert!(log.lock().is_empty(), "커밋되지 않은 attachment 는 등록 부수효과가 없어야 합니다");
    }

    #[test]
    fn 등록할_것이_없는_capability의_attachment는_커밋해도_부수효과가_없다() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let registry = recording_registry(&log);
        let attachments = vec![ProjectAttachment::none(), recording_attachment(1, &log), ProjectAttachment::none()];

        registry.commit_attachments(&stub_state(), &stub_project(), attachments);

        assert_eq!(*log.lock(), vec![1]);
    }
}

use std::path::Path;

use tauri::AppHandle;

use crate::ids::ProjectId;
use crate::state::AppState;

use super::types::{CapabilityKind, Project};

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

    fn attach(&self, _app: &AppHandle, _state: &AppState, _project: &Project) {}

    fn detach(&self, _app: &AppHandle, _state: &AppState, _project_id: &ProjectId) {}
}

/// The statically assembled capability list. Both [`Self::attach_all`] and [`Self::detach_all`]
/// walk it in registration order, so **registration order is part of the correctness contract**,
/// not a style choice: `project_close` used to hand-code its reap sequence precisely because two of
/// the reaps are order-sensitive (the layout capability must flush a dirty layout before anything
/// observes the project as gone, and terminals must be reaped during close rather than at app
/// exit — see each capability's `detach` doc for the full rationale). The canonical order lives in
/// one place, `lib.rs`'s `project_capabilities`, and is pinned there by a source-scan test; the
/// forward direction of the shared walk itself is pinned by the recording-stub test in this file.
pub struct ProjectCapabilities {
    capabilities: Vec<Box<dyn ProjectCapability>>,
}

impl ProjectCapabilities {
    pub fn new(capabilities: Vec<Box<dyn ProjectCapability>>) -> Self {
        Self { capabilities }
    }

    /// The single traversal every registry walk goes through — forward, in registration order.
    /// [`Self::attach_all`], [`Self::detach_all`], and [`Self::detected_kinds`] must all iterate
    /// via this method rather than touching `self.capabilities` directly, so the forward-walk
    /// contract has exactly one implementation and the recording-stub test below pins it for all
    /// three at once (a `tauri::AppHandle` cannot be constructed outside a running app, so
    /// `attach_all`/`detach_all` themselves are not directly callable from a unit test — this
    /// codebase deliberately has no mock-app harness).
    fn for_each_in_registration_order(&self, mut visit: impl FnMut(&dyn ProjectCapability)) {
        for capability in &self.capabilities {
            visit(capability.as_ref());
        }
    }

    /// The [`CapabilityKind`]s the registered capabilities detect for `root`, in registration
    /// order — the source `project_open` injects into `project::service::open_project` as
    /// `Project.capabilities`.
    pub fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind> {
        let mut kinds = Vec::new();
        self.for_each_in_registration_order(|capability| kinds.extend(capability.detected_kind(root)));
        kinds
    }

    pub fn attach_all(&self, app: &AppHandle, state: &AppState, project: &Project) {
        self.for_each_in_registration_order(|capability| capability.attach(app, state, project));
    }

    pub fn detach_all(&self, app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        self.for_each_in_registration_order(|capability| capability.detach(app, state, project_id));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;

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

    /// Pins the shared walk (`for_each_in_registration_order`) to **forward** registration order —
    /// the traversal `attach_all`/`detach_all`/`detected_kinds` all delegate to. `detach_all`'s
    /// direction is a correctness contract (`project_close`'s historical reap order), and this is
    /// the closest a unit test can get to it: `attach`/`detach` take a `tauri::AppHandle`, which
    /// has no constructor outside a running app (no mock-app harness in this codebase), so the
    /// recording happens through the walk's visit path instead.
    #[test]
    fn 공유_순회는_등록_순서_전방이다() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let registry = ProjectCapabilities::new(
            (0..3)
                .map(|index| {
                    Box::new(RecordingCapability {
                        index,
                        log: Arc::clone(&log),
                    }) as Box<dyn ProjectCapability>
                })
                .collect(),
        );

        registry.for_each_in_registration_order(|capability| {
            capability.detected_kind(Path::new("/unused"));
        });

        assert_eq!(*log.lock(), vec![0, 1, 2]);
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
}

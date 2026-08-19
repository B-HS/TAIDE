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
/// root, or `None` for lifecycle hooks that have no serialized kind; the set it produces across the
/// registry must stay consistent with what `project::service::open_project` records on the project
/// (pinned by the assembly's parity test in `lib.rs`).
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
/// one place, `lib.rs`'s `project_capabilities`, and is pinned there by a source-scan test.
pub struct ProjectCapabilities {
    capabilities: Vec<Box<dyn ProjectCapability>>,
}

impl ProjectCapabilities {
    pub fn new(capabilities: Vec<Box<dyn ProjectCapability>>) -> Self {
        Self { capabilities }
    }

    /// The [`CapabilityKind`]s the registered capabilities detect for `root`, in registration
    /// order — the registry-side counterpart of the kinds `project::service::open_project` records
    /// on a freshly opened project.
    pub fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind> {
        self.capabilities
            .iter()
            .filter_map(|capability| capability.detected_kind(root))
            .collect()
    }

    pub fn attach_all(&self, app: &AppHandle, state: &AppState, project: &Project) {
        for capability in &self.capabilities {
            capability.attach(app, state, project);
        }
    }

    pub fn detach_all(&self, app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        for capability in &self.capabilities {
            capability.detach(app, state, project_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct KindOnly(Option<CapabilityKind>);

    impl ProjectCapability for KindOnly {
        fn detected_kind(&self, _root: &Path) -> Option<CapabilityKind> {
            self.0
        }
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

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_runtime::AppServices;

#[derive(Clone)]
pub struct Activity(Arc<AtomicBool>);

impl Activity {
    pub fn is_active(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

pub struct ActivityOwner {
    pub activity: Activity,
    repaint: Arc<dyn Fn() + Send + Sync>,
}

impl ActivityOwner {
    pub fn new(repaint: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            activity: Activity(Arc::new(AtomicBool::new(true))),
            repaint,
        }
    }
}

impl Drop for ActivityOwner {
    fn drop(&mut self) {
        self.activity.0.store(false, Ordering::Release);
        (self.repaint)();
    }
}

pub fn scoped_entry(
    services: &AppServices,
    path: &Path,
    roots: Option<&[String]>,
) -> AppResult<PathBuf> {
    if services.state.is_shutting_down() {
        return Err(AppError::Forbidden(
            "workspace file operations are stopping".into(),
        ));
    }
    let (_, resolved) =
        root_guard::resolve_entry_owning_project(&services.state.projects.read(), path)?;
    if let Some(roots) = roots
        && !roots.iter().any(|root| {
            root_guard::canonicalize_lenient(Path::new(root))
                .is_ok_and(|root| resolved.starts_with(root))
        })
    {
        return Err(AppError::Forbidden(
            "workspace file operation is outside its session roots".into(),
        ));
    }
    Ok(resolved)
}

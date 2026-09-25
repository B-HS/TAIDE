use std::path::Path;
use std::sync::Arc;

use taide_model::error::AppResult;

pub trait PlatformServices: Send + Sync {
    fn open_path(&self, path: &Path) -> AppResult<()>;
    fn reveal_item_in_dir(&self, path: &Path) -> AppResult<()>;
    fn open_url(&self, url: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct PlatformServicesState(pub Arc<dyn PlatformServices>);

impl PlatformServicesState {
    pub fn new(platform: Arc<dyn PlatformServices>) -> Self {
        Self(platform)
    }
}

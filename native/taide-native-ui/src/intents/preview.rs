#[derive(Debug, Clone)]
pub enum Intent {
    ReopenWith { path: String, preview: bool },
}

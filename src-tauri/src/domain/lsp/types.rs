pub use taide_model::lsp::*;

pub const RESTART_BACKOFF_LIMIT: u32 = 3;
pub const LSP_MANIFEST_SOURCE: &str = include_str!("../../../resources/lsp-servers.json");

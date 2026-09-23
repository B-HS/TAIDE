pub use taide_model::vsix::*;

pub const VSIX_EXTENSION_ROOT: &str = "extension";
pub const VSIX_MANIFEST_ENTRY: &str = "extension/package.json";
pub const VSIX_ENTRY_MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const VSIX_INCLUDE_CHAIN_MAX_DEPTH: usize = 5;
pub const VSIX_MAX_THEME_CONTRIBUTIONS: usize = 64;
pub const VSIX_TOTAL_MAX_EXTRACTED_BYTES: u64 = 64 * 1024 * 1024;

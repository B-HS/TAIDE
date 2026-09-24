use super::types::AppInfo;

pub use taide_app::service::{app_file_path, perf_snapshot, read_app_file, write_prompt_file};

pub const APP_NAME: &str = "TAIDE";

pub fn app_info() -> AppInfo {
    AppInfo {
        name: APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

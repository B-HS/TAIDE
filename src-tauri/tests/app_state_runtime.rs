use taide_lib::state::AppState as TauriAppState;
use taide_model::paths::AppPaths;
use taide_runtime::AppState;

#[test]
fn runtime_상태는_기존_tauri_상태와_같은_타입이다() {
    let state: TauriAppState = AppState::new(AppPaths::new(std::env::temp_dir()));

    assert!(state.projects.read().is_empty());
    assert!(state.layouts.read().is_empty());
    assert!(!state.is_shutting_down());
}

#[test]
fn 기존_상태_경로는_runtime_구현만_재수출한다() {
    let facade = include_str!("../src/state.rs");
    let manifest = include_str!("../../crates/taide-runtime/Cargo.toml");

    assert!(facade.contains("pub use taide_runtime::{AppState, FlushScope, FlushTicket};"));
    assert!(manifest.contains("taide-infra ="));
    assert!(!manifest.contains("tauri ="));
}

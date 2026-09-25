use taide_lib::domain::window::commands::WindowStore;
use taide_lib::ids::ProjectId;
use taide_lib::platform::window_registry::WindowRegistry;

#[test]
fn 기존_window_store와_platform_registry는_같은_타입이다() {
    let registry = WindowRegistry::default();
    let legacy: &WindowStore = &registry;
    let project_id = ProjectId::from("prj-window-registry".to_string());

    registry.register("editor-1".to_string(), project_id.clone(), 1);
    assert_eq!(legacy.label_for(&project_id, 1).as_deref(), Some("editor-1"));
    assert_eq!(legacy.forget("editor-1"), Some((project_id, 1)));
    assert_eq!(registry.forget("editor-1"), None);
}

#[test]
fn 앱_조립과_창_명령은_platform_registry를_직접_사용한다() {
    let app_source = include_str!("../src/lib.rs");
    let window_source = include_str!("../src/domain/window/commands.rs");

    assert!(app_source.contains("use crate::platform::window_registry::WindowRegistry;"));
    assert!(app_source.contains("app.manage(WindowRegistry::default())"));
    assert!(window_source.contains("use crate::platform::window_registry::WindowRegistry;"));
}

#[test]
fn 폰트와_시스템_사용량의_blocking_호출은_같은_감독자를_사용한다() {
    let font = include_str!("../src/domain/font/commands.rs");
    let system = include_str!("../src/domain/system/commands.rs");
    let remote = include_str!("../src/remote_gateway.rs");

    assert!(font.contains(".run_blocking_result(\"font-list\""));
    assert!(system.contains(".run_blocking_result(\"system-usage-get\""));
    assert!(system.contains(".begin_operation(\"system-usage-breakdown\""));
    assert!(system.contains(".run_blocking_result(\"system-usage-breakdown\""));
    assert!(!font.contains("tauri::async_runtime::spawn_blocking"));
    assert!(!system.contains("tauri::async_runtime::spawn_blocking"));
    assert!(remote.contains("font_list(app.state()).await"));
    assert!(remote.contains("system_usage_get(app.state(), app.state()).await"));
    assert!(remote.contains("system_usage_breakdown(app.clone(), app.state(), app.state(), app.state()).await"));
}

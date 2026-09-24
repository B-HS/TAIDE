use taide_lib::domain::agent::service::HookPayload;
use taide_lib::domain::ide::lockfile::IdeLockfileContent;
use taide_model::agent::HookPayload as ModelHookPayload;
use taide_model::ide::IdeLockfileContent as ModelIdeLockfileContent;

#[test]
fn ide_lockfile의_기존_기본값과_저장_wire를_유지한다() {
    let content: ModelIdeLockfileContent = serde_json::from_value(serde_json::json!({
        "pid": 12,
        "workspaceFolders": ["/repo"],
        "ideName": "TAIDE",
        "transport": "ws",
        "authToken": "fixture-token"
    }))
    .expect("기존 IDE lockfile");
    let facade: IdeLockfileContent = content;
    assert!(!facade.running_in_windows);
    let saved = serde_json::to_value(facade).expect("IDE lockfile 저장");
    assert!(saved.get("runningInWindows").is_none());
    assert_eq!(saved["workspaceFolders"], serde_json::json!(["/repo"]));
}

#[test]
fn agent_hook의_기존_기본값과_공개_경로를_유지한다() {
    let payload: ModelHookPayload = serde_json::from_value(serde_json::json!({
        "hook_event_name": "Stop"
    }))
    .expect("기존 agent hook");
    let facade: HookPayload = payload;
    assert_eq!(facade.hook_event_name, "Stop");
    assert!(facade.cwd.is_empty());
}

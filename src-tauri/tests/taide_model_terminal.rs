use taide_lib::domain::terminal::types::{PtyAttachResult, PtySpawnOptions, ShellProfile, TerminalSession, DEFAULT_SCROLLBACK_BYTES};
use taide_model::terminal::{
    PtyAttachResult as ModelPtyAttachResult, PtySpawnOptions as ModelPtySpawnOptions, ShellProfile as ModelShellProfile,
    TerminalSession as ModelTerminalSession,
};

#[test]
fn terminal_spawn과_attach의_기존_wire를_유지한다() {
    let model: ModelPtySpawnOptions = serde_json::from_value(serde_json::json!({
        "projectId": "prj-1",
        "cwd": "/repo",
        "cols": 80,
        "rows": 24
    }))
    .expect("기존 spawn 옵션");
    let facade: PtySpawnOptions = model;
    assert_eq!(facade.scrollback_bytes, None);
    assert_eq!(
        taide_model::terminal::resolve_scrollback_bytes(facade.scrollback_bytes),
        DEFAULT_SCROLLBACK_BYTES
    );

    let attach: ModelPtyAttachResult = serde_json::from_value(serde_json::json!({
        "subscriptionId": 7,
        "replayBytes": 1024
    }))
    .expect("기존 attach 결과");
    let facade: PtyAttachResult = attach;
    assert_eq!(facade.replay_bytes, 1024);
}

#[test]
fn shell_profile과_session의_기존_wire를_유지한다() {
    let profile: ModelShellProfile = serde_json::from_value(serde_json::json!({
        "id": "default",
        "name": "zsh",
        "path": "/bin/zsh"
    }))
    .expect("기존 shell profile");
    let facade: ShellProfile = profile;
    assert!(facade.args.is_empty());

    let session: ModelTerminalSession = serde_json::from_value(serde_json::json!({
        "id": "pty-1",
        "projectId": "prj-1",
        "cwd": "/repo",
        "shell": "/bin/zsh",
        "running": true
    }))
    .expect("기존 terminal session");
    let facade: TerminalSession = session;
    assert!(facade.running);
}

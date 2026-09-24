use taide_lib::domain::remote::types::{RemoteLinkInfo, RemoteRequest, RemoteStatus};
use taide_model::remote::{RemoteLinkInfo as ModelRemoteLinkInfo, RemoteRequest as ModelRemoteRequest, RemoteStatus as ModelRemoteStatus};

#[test]
fn remote_status와_link의_기존_wire를_유지한다() {
    let model: ModelRemoteStatus = serde_json::from_value(serde_json::json!({
        "running": true,
        "port": 41700,
        "clientCount": 2,
        "passwordConfigured": false
    }))
    .expect("기존 remote 상태");
    let facade: RemoteStatus = model;
    assert_eq!(facade.client_count, 2);

    let model: ModelRemoteLinkInfo = serde_json::from_value(serde_json::json!({
        "url": "http://127.0.0.1:41700"
    }))
    .expect("기존 remote 링크");
    let facade: RemoteLinkInfo = model;
    assert_eq!(facade.url, "http://127.0.0.1:41700");
}

#[test]
fn remote_request의_선택_args_기본값을_유지한다() {
    let model: ModelRemoteRequest = serde_json::from_value(serde_json::json!({
        "seq": 1,
        "command": "project_list"
    }))
    .expect("기존 remote 요청");
    let facade: RemoteRequest = model;
    assert_eq!(facade.seq, 1);
    assert!(facade.args.is_null());
}

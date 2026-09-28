use serde_json::{json, Value};
use taide_remote::{protocol, service, store::RemoteStore};

const REMOTE_FIXTURE: &str = include_str!("fixtures/rust-native/remote-wire-session-v1.json");
const TEST_PASSWORD: &str = "fixture-only-password";
const WRONG_PASSWORD: &str = "fixture-only-wrong";

#[test]
fn remote_호스트와_origin_정책은_fixture와_같다() {
    let fixture: Value = serde_json::from_str(REMOTE_FIXTURE).expect("remote fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    let port = u32::try_from(fixture["bindPort"].as_u64().expect("bind port")).expect("u32 port");
    let allowed_hosts = fixture["allowedHosts"]
        .as_array()
        .expect("allowed hosts")
        .iter()
        .map(|host| host.as_str().expect("allowed hostname").to_owned())
        .collect::<Vec<_>>();

    for case in fixture["hostOriginCases"].as_array().expect("host cases") {
        let host = case["host"].as_str();
        let origin = case["origin"].as_str();
        let allowed = service::is_allowed_host(host, &allowed_hosts, port) && service::is_allowed_origin(origin, host);
        assert_eq!(allowed, case["allowed"].as_bool().expect("expected policy"), "{}", case["name"]);
    }
}

#[test]
fn remote_인증과_session_폐기는_fixture와_같다() {
    let fixture: Value = serde_json::from_str(REMOTE_FIXTURE).expect("remote fixture");
    let store = RemoteStore::default();
    let epoch = store.subscribe_session_epoch();
    let link = store.issue_link_token();
    let wrong_link_rejected = store.consume_link_token("fixture-invalid-link").is_none();
    let nonce = store.consume_link_token(&link).expect("issued link");
    let link_replay_rejected = store.consume_link_token(&link).is_none();
    let pending_nonce = store.has_pending_nonce(&nonce);
    let session = store.promote_nonce_to_session(&nonce).expect("issued nonce");
    let nonce_replay_rejected = store.promote_nonce_to_session(&nonce).is_none();
    let active_session = store.has_active_session(&session);
    store.revoke_all_sessions();
    let revoke_epoch = *epoch.borrow();
    let revoked_session_rejected = !store.has_active_session(&session);
    let password_digest = service::hash_password(TEST_PASSWORD);

    let actual = json!({
        "wrongLinkRejected": wrong_link_rejected,
        "linkReplayRejected": link_replay_rejected,
        "pendingNonce": pending_nonce,
        "nonceReplayRejected": nonce_replay_rejected,
        "activeSession": active_session,
        "revokeEpoch": revoke_epoch,
        "revokedSessionRejected": revoked_session_rejected,
        "correctPasswordAccepted": service::verify_password(&password_digest, TEST_PASSWORD),
        "wrongPasswordRejected": !service::verify_password(&password_digest, WRONG_PASSWORD)
    });
    assert_eq!(actual, fixture["session"]);
}

#[test]
fn remote_binary와_json_frame은_fixture와_같다() {
    let fixture: Value = serde_json::from_str(REMOTE_FIXTURE).expect("remote fixture");
    let payload = json!({ "state": "fixture" });
    let actual = json!({
        "channelBinary": protocol::channel_binary_frame(7, 2, &[0, 255]),
        "responseBinary": protocol::response_binary_frame(9, &[1, 2]),
        "responseJson": serde_json::from_str::<Value>(&protocol::response_frame(9, true, payload.clone())).expect("response frame"),
        "channelJson": serde_json::from_str::<Value>(&protocol::channel_json_frame(7, 2, payload)).expect("channel frame"),
        "channelEnd": serde_json::from_str::<Value>(&protocol::channel_end_frame(7, 2)).expect("channel end")
    });
    assert_eq!(actual, fixture["frames"]);
}

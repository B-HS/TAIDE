use serde_json::json;

#[test]
fn 원격_owner_는_중첩_객체와_배열에서도_고정_라벨로_강제된다() {
    let args = json!({
        "owner": "main",
        "request": {
            "owner": "editor-2",
            "items": [{"owner": "main"}, {"nested": {"owner": "editor-3"}}],
        },
        "projectId": "project-1",
    });

    let filtered = taide_remote::policy::enforce_remote_owner_label(args);

    assert_eq!(filtered["owner"], taide_remote::types::REMOTE_OWNER_LABEL);
    assert_eq!(filtered["request"]["owner"], taide_remote::types::REMOTE_OWNER_LABEL);
    assert_eq!(filtered["request"]["items"][0]["owner"], taide_remote::types::REMOTE_OWNER_LABEL);
    assert_eq!(
        filtered["request"]["items"][1]["nested"]["owner"],
        taide_remote::types::REMOTE_OWNER_LABEL
    );
    assert_eq!(filtered["projectId"], "project-1");
}

#[test]
fn 원격_owner_가_없거나_이미_고정_라벨이면_그대로_유지된다() {
    let args = json!({
        "input": {"owner": taide_remote::types::REMOTE_OWNER_LABEL},
        "items": [{"name": "unchanged"}],
    });

    assert_eq!(taide_remote::policy::enforce_remote_owner_label(args.clone()), args);
}

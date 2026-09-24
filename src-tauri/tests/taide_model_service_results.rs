use taide_lib::domain::{file, git, project};
use taide_model::file::{MirrorEntry as ModelMirrorEntry, UntitledMirrorEntry as ModelUntitledMirrorEntry};
use taide_model::git::AheadBehind as ModelAheadBehind;
use taide_model::ids::TabId;
use taide_model::project::ProjectOpenResult as ModelProjectOpenResult;

#[test]
fn 서비스_결과의_기존_공개_타입_경로를_유지한다() {
    let _: Option<project::service::ProjectOpenResult> = None::<ModelProjectOpenResult>;
    let _: Option<git::service::AheadBehind> = None::<ModelAheadBehind>;
    let _: Option<file::service::MirrorEntry> = None::<ModelMirrorEntry>;
    let _: Option<file::service::UntitledMirrorEntry> = None::<ModelUntitledMirrorEntry>;
}

#[test]
fn 서비스_결과의_기존_camel_case_wire를_유지한다() {
    let ahead_behind = ModelAheadBehind { ahead: 2, behind: 1 };
    assert_eq!(
        serde_json::to_value(ahead_behind).unwrap(),
        serde_json::json!({ "ahead": 2, "behind": 1 })
    );

    let mirror = ModelMirrorEntry {
        path: "/repo/file.rs".to_string(),
        content: "draft".to_string(),
        saved_at_ms: 42.0,
        disk_modified_ms: None,
        conflict: false,
        source_missing: true,
    };
    assert_eq!(
        serde_json::to_value(mirror).unwrap(),
        serde_json::json!({
            "path": "/repo/file.rs",
            "content": "draft",
            "savedAtMs": 42.0,
            "diskModifiedMs": null,
            "conflict": false,
            "sourceMissing": true
        })
    );

    let untitled = ModelUntitledMirrorEntry {
        tab_id: TabId("tab-fixed".to_string()),
        content: "draft".to_string(),
        saved_at_ms: 42.0,
    };
    assert_eq!(
        serde_json::to_value(untitled).unwrap(),
        serde_json::json!({ "tabId": "tab-fixed", "content": "draft", "savedAtMs": 42.0 })
    );
}

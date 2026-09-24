use taide_lib::domain::git::types::{CommitFile, CommitOptions, DiffMode, GitChangeKind, GitStatus, StagedDiffText};
use taide_model::git::{
    CommitFile as ModelCommitFile, CommitOptions as ModelCommitOptions, DiffMode as ModelDiffMode, GitStatus as ModelGitStatus,
    StagedDiffText as ModelStagedDiffText,
};

#[test]
fn git_status와_변경_종류의_기존_wire를_유지한다() {
    let wire = serde_json::json!({
        "rows": [{
            "path": "src/main.rs",
            "absPath": "/repo/src/main.rs",
            "staged": "modified",
            "isConflicted": false
        }],
        "branch": "main",
        "ahead": 0,
        "behind": 0,
        "hasRemote": true
    });
    let model: ModelGitStatus = serde_json::from_value(wire).expect("기존 Git 상태");
    let facade: GitStatus = model;
    assert_eq!(facade.rows[0].staged, Some(GitChangeKind::Modified));
    assert_eq!(facade.rows[0].orig_path, None);
    assert_eq!(facade.rows[0].abs_path, "/repo/src/main.rs");

    let mode: ModelDiffMode = serde_json::from_str("\"indexVsHead\"").expect("기존 diff 모드");
    let facade: DiffMode = mode;
    assert_eq!(facade, DiffMode::IndexVsHead);
}

#[test]
fn git_commit과_diff_결과의_기존_wire를_유지한다() {
    let options: ModelCommitOptions = serde_json::from_value(serde_json::json!({})).expect("기본 커밋 옵션");
    let facade: CommitOptions = options;
    assert!(!facade.amend);
    assert!(!facade.stage_all);

    let file: ModelCommitFile = serde_json::from_value(serde_json::json!({
        "path": "old.txt",
        "absPath": "/repo/old.txt",
        "kind": "deleted"
    }))
    .expect("기존 커밋 파일");
    let facade: CommitFile = file;
    assert_eq!(facade.kind, GitChangeKind::Deleted);
    assert_eq!(facade.orig_abs_path, None);

    let model: ModelStagedDiffText = serde_json::from_value(serde_json::json!({
        "diffText": "diff --git a/a b/a",
        "truncated": false,
        "skippedFiles": [],
        "usedFallback": true
    }))
    .expect("기존 staged diff");
    let facade: StagedDiffText = model;
    assert!(facade.used_fallback);
}

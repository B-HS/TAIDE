use taide_lib::domain::ide::types::{IdeDiagnostic, IdeDiagnosticSeverity, IdeDiffOutcome, IdeSelectionInput, IdeStatus};
use taide_model::ide::{
    IdeDiagnostic as ModelIdeDiagnostic, IdeDiffOutcome as ModelIdeDiffOutcome, IdeSelectionInput as ModelIdeSelectionInput,
    IdeStatus as ModelIdeStatus,
};

#[test]
fn ide_status와_diagnostic의_기존_wire를_유지한다() {
    let status: ModelIdeStatus = serde_json::from_value(serde_json::json!({
        "running": true,
        "port": 41000,
        "connected": false,
        "clientCount": 0
    }))
    .expect("IDE 상태");
    let facade: IdeStatus = status;
    assert!(facade.running);
    assert_eq!(facade.client_count, 0);

    let diagnostic: ModelIdeDiagnostic = serde_json::from_value(serde_json::json!({
        "path": "/repo/main.rs",
        "severity": "warning",
        "startLine": 1,
        "startCharacter": 0,
        "endLine": 1,
        "endCharacter": 4,
        "message": "warning"
    }))
    .expect("IDE 진단");
    let facade: IdeDiagnostic = diagnostic;
    assert_eq!(facade.severity, IdeDiagnosticSeverity::Warning);
    assert_eq!(facade.source, None);
}

#[test]
fn ide_selection과_diff_outcome의_기존_wire를_유지한다() {
    let selection: ModelIdeSelectionInput = serde_json::from_value(serde_json::json!({
        "owner": "main",
        "projectId": "prj-1",
        "path": "/repo/main.rs",
        "text": "test",
        "startLine": 1,
        "startCharacter": 0,
        "endLine": 1,
        "endCharacter": 4,
        "isEmpty": false
    }))
    .expect("IDE 선택 영역");
    let facade: IdeSelectionInput = selection;
    assert_eq!(facade.owner, "main");
    assert_eq!(facade.text, "test");

    let outcome: ModelIdeDiffOutcome = serde_json::from_str("\"tabClosed\"").expect("IDE diff 결과");
    let facade: IdeDiffOutcome = outcome;
    assert_eq!(facade, IdeDiffOutcome::TabClosed);
}

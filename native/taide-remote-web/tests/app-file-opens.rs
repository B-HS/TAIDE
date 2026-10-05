use serde_json::json;
use taide_model::{
    error::AppError,
    ids::{PaneId, ProjectId, TabId},
    layout::LAYOUT_SCHEMA_VERSION,
};
use taide_native_ui::settings_owner::Owner;
use taide_remote_web::{InvokeError, ResponsePayload, app_file_opens::AppFileOpens};

const SEQ: u32 = 1;
const FAILED_SEQ: u32 = 2;
const CLOSED_SEQ: u32 = 3;
const MALFORMED_SEQ: u32 = 4;

#[test]
fn 설정파일_열기는_실제_owner의_pane과_typed_대상을_사용하고_응답을_한번_소비한다() {
    let owner = Owner {
        project: ProjectId("project-synthetic".into()),
        pane: PaneId("auxiliary-pane".into()),
        tab: TabId("settings-tab".into()),
    };
    let call = AppFileOpens::settings_call(&owner);
    assert_eq!(call.command, "layout_open_tab");
    assert_eq!(
        call.args,
        json!({"projectId":"project-synthetic","kind":{"kind":"appFile","target":{"kind":"settings"}},"title":"settings.json","target":"auxiliary-pane","preview":false})
    );
    let mut opens = AppFileOpens::default();
    let result = Ok(ResponsePayload::Json(
        json!({"version":LAYOUT_SCHEMA_VERSION,"root":{"node":"leaf","id":"auxiliary-pane","tabs":[],"active":null},"focusedPane":"auxiliary-pane","revision":SEQ}),
    ));
    assert!(opens.response(SEQ, &result).is_none());
    opens.sent(SEQ, owner.project.clone());
    let opened = opens.response(SEQ, &result).unwrap().unwrap();
    assert_eq!(opened.project, owner.project);
    assert_eq!(opened.layout.focused_pane, owner.pane);
    assert!(opens.response(SEQ, &result).is_none());
    opens.sent(FAILED_SEQ, owner.project.clone());
    assert!(
        opens
            .response(
                FAILED_SEQ,
                &Err(json!({"code":"Forbidden","message":"synthetic refusal"}))
            )
            .unwrap()
            .is_none()
    );
    assert!(
        matches!(opens.take_errors().as_slice(),[AppError::Forbidden(message)] if message == "synthetic refusal")
    );
    opens.sent(CLOSED_SEQ, owner.project.clone());
    opens.disconnected();
    assert_eq!(opens.take_errors().len(), 1);
    opens.disconnected();
    assert!(opens.take_errors().is_empty());
    assert!(opens.response(CLOSED_SEQ, &result).is_none());
    opens.sent(MALFORMED_SEQ, owner.project);
    assert!(
        opens
            .response(MALFORMED_SEQ, &Ok(ResponsePayload::Binary(vec![])))
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        opens.take_errors().as_slice(),
        [AppError::Internal(_)]
    ));
    opens.failed(InvokeError::Closed);
    assert_eq!(opens.take_errors().len(), 1);
}

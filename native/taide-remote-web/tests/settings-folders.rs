use serde_json::json;
use taide_model::{error::AppError, system::AppDataPathKind};
use taide_remote_web::{InvokeError, ResponsePayload, settings_folders::FolderActions};

const SEQ: u32 = 1;
const CLOSED_SEQ: u32 = 2;
const MALFORMED_SEQ: u32 = 3;

#[test]
fn 원격_폴더_버튼은_기존_거절을_표시하고_중복_응답과_closed를_한번만_소비한다() {
    for (kind, value) in [
        (AppDataPathKind::Themes, "themes"),
        (AppDataPathKind::Locales, "locales"),
        (AppDataPathKind::Plugins, "plugins"),
        (AppDataPathKind::Snippets, "snippets"),
    ] {
        let call = FolderActions::call(kind);
        assert_eq!(call.command, "system_open_app_data_path");
        assert_eq!(call.args, json!({"kind":value}));
    }
    let mut actions = FolderActions::default();
    let denied = Err(json!({"code":"Forbidden","message":"synthetic unreachable desktop"}));
    assert!(!actions.response(SEQ, &denied));
    actions.sent(SEQ);
    assert!(actions.response(SEQ, &denied));
    assert!(
        matches!(actions.take_errors().as_slice(), [AppError::Forbidden(message)] if message == "synthetic unreachable desktop")
    );
    assert!(!actions.response(SEQ, &denied));
    assert!(actions.take_errors().is_empty());
    actions.sent(CLOSED_SEQ);
    actions.disconnected();
    assert!(matches!(
        actions.take_errors().as_slice(),
        [AppError::Forbidden(_)]
    ));
    actions.disconnected();
    assert!(actions.take_errors().is_empty());
    assert!(!actions.response(CLOSED_SEQ, &Ok(ResponsePayload::Json(json!(null)))));
    actions.sent(MALFORMED_SEQ);
    assert!(actions.response(MALFORMED_SEQ, &Ok(ResponsePayload::Binary(vec![]))));
    assert!(matches!(
        actions.take_errors().as_slice(),
        [AppError::Internal(_)]
    ));
    actions.failed(InvokeError::Closed);
    assert!(matches!(
        actions.take_errors().as_slice(),
        [AppError::Forbidden(_)]
    ));
}

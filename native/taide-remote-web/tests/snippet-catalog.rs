use serde_json::json;
use taide_model::error::AppError;
use taide_native_ui::{settings_view::Views, snippet_completion};
use taide_remote_web::{ResponsePayload, snippet_operations::SnippetOperations};

const FIRST_SEQ: u32 = 21;
const INVALIDATED_SEQ: u32 = 22;
const CURRENT_SEQ: u32 = 23;
const DISCONNECTED_SEQ: u32 = 24;
const RECOVERED_SEQ: u32 = 25;

#[test]
fn 전역_snippet_wire는_settings없이_조회하고_옛세대_잘못된응답_단절과_회수를_보존한다() {
    let mut views = Views::default();
    let mut operations = SnippetOperations::default();
    let payload = |body: &str| {
        Ok(ResponsePayload::Json(json!([{
            "fileName": "rust.json",
            "snippets": {"Synthetic": {"prefix": "s", "body": body}}
        }])))
    };
    operations.catalog_sent(views.next_snippet_read().unwrap(), FIRST_SEQ);
    assert!(!operations.has_pending_mutations());
    assert!(operations.response(FIRST_SEQ, &payload("initial $0")));
    assert!(views.accept_snippet_catalog(
        operations.take_catalog_replies().pop().unwrap(),
        std::time::Instant::now()
    ));
    assert!(!operations.response(FIRST_SEQ, &payload("duplicate")));
    assert_eq!(
        snippet_completion::collect(views.snippet_catalog().files(), "rust")[0].body,
        "initial $0"
    );
    views.begin_frame();
    views.finish_frame();
    assert!(views.next_snippet_read().is_none());
    views.refresh_snippets();
    operations.catalog_sent(views.next_snippet_read().unwrap(), INVALIDATED_SEQ);
    views.refresh_snippets();
    operations.catalog_sent(views.next_snippet_read().unwrap(), CURRENT_SEQ);
    assert!(operations.response(INVALIDATED_SEQ, &payload("stale")));
    assert!(!views.accept_snippet_catalog(
        operations.take_catalog_replies().pop().unwrap(),
        std::time::Instant::now()
    ));
    assert!(operations.response(CURRENT_SEQ, &Ok(ResponsePayload::Json(json!(null)))));
    let malformed = operations.take_catalog_replies().pop().unwrap();
    assert!(malformed.result.is_err());
    assert!(views.accept_snippet_catalog(malformed, std::time::Instant::now()));
    assert_eq!(views.snippet_catalog().files().len(), 1);
    assert!(views.snippet_catalog().error().is_some());
    assert!(views.next_snippet_read().is_none());
    assert!(operations.take_failures().is_empty());
    views.refresh_snippets();
    operations.catalog_sent(views.next_snippet_read().unwrap(), DISCONNECTED_SEQ);
    operations.disconnected();
    operations.disconnected();
    let mut replies = operations.take_catalog_replies();
    assert_eq!(replies.len(), 1);
    assert!(replies[0].result.is_err());
    views.disconnect_snippets();
    assert!(!views.accept_snippet_catalog(replies.pop().unwrap(), std::time::Instant::now()));
    assert!(!operations.response(DISCONNECTED_SEQ, &payload("late disconnect")));
    assert!(views.next_snippet_read().is_none());
    assert!(!operations.has_pending_mutations());
    assert!(operations.take_failures().is_empty());
    views.reconnect_snippets(std::time::Instant::now());
    let request = views.next_snippet_read().unwrap();
    operations.catalog_sent(request.clone(), RECOVERED_SEQ);
    assert!(operations.response(RECOVERED_SEQ, &payload("recovered $0")));
    assert!(views.accept_snippet_catalog(
        operations.take_catalog_replies().pop().unwrap(),
        std::time::Instant::now()
    ));
    assert!(views.snippet_catalog().error().is_none());
    assert_eq!(
        snippet_completion::collect(views.snippet_catalog().files(), "rust")[0].body,
        "recovered $0"
    );
    views.clear();
    assert!(!request.is_active());
    operations.catalog_failed(
        request,
        AppError::Forbidden("synthetic retired read".into()),
    );
    assert!(!views.accept_snippet_catalog(
        operations.take_catalog_replies().pop().unwrap(),
        std::time::Instant::now()
    ));
    assert!(views.snippet_catalog().files().is_empty());
    assert!(operations.take_failures().is_empty());
}

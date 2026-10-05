use serde_json::json;
use taide_model::{
    app::AppFileTarget,
    ids::{PaneId, ProjectId, TabId},
};
use taide_native_editor::{
    editing::replace_selections,
    store::{EditorLimits, EditorStore},
    view::ViewKey,
};
use taide_remote_web::{
    ResponsePayload,
    app_files::{AppFiles, Owner},
    shell::Call,
};

const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 128;
const READ_SEQ: u32 = 1;
const WRITE_SEQ: u32 = 2;
const CANONICAL_SEQ: u32 = 3;
const FAILED_SEQ: u32 = 4;
const STALE_SEQ: u32 = 5;
const REMOUNT_SEQ: u32 = 6;
const CLOSED_OWNER_SEQ: u32 = 7;

fn owner(tab: &str) -> Owner {
    Owner {
        project: ProjectId("synthetic-project".into()),
        key: ViewKey {
            window: "remote".into(),
            pane: PaneId("synthetic-pane".into()),
            tab: TabId(tab.into()),
        },
        target: AppFileTarget::Settings,
    }
}

fn send_next(files: &mut AppFiles, seq: u32) -> Call {
    let requests = files.next_requests();
    assert_eq!(requests.len(), 1);
    let request = requests.into_iter().next().unwrap();
    assert!(files.is_active(&request));
    let call = request.call();
    files.sent(request, seq);
    call
}

#[test]
fn 앱파일_공유문서는_정규화_ack_늦은편집_거절과_remount_수명을_보존한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let mut files = AppFiles::default();
    let first = owner("first");
    assert!(files.bind(first.clone(), &mut store).unwrap().is_none());
    let read = send_next(&mut files, READ_SEQ);
    assert_eq!(read.command, "app_file_read");
    assert_eq!(read.args, json!({"target":{"kind":"settings"}}));
    assert!(files.response(
        READ_SEQ,
        &Ok(ResponsePayload::Json(json!("{}"))),
        &mut store
    ));
    assert!(!files.response(
        READ_SEQ,
        &Ok(ResponsePayload::Json(json!("wrong"))),
        &mut store
    ));
    let view = files.view(&first, &store).unwrap();
    let document = store.views().get(view).unwrap().document;
    let second = owner("second");
    let other = files.bind(second.clone(), &mut store).unwrap().unwrap();
    assert_eq!(store.views().get(other).unwrap().document, document);
    assert!(files.next_requests().is_empty());
    replace_selections(&mut store, view, "draft ", None).unwrap();
    assert!(files.prepare_save(&first, &mut store).unwrap());
    assert!(!files.prepare_save(&second, &mut store).unwrap());
    let write = send_next(&mut files, WRITE_SEQ);
    assert_eq!(write.command, "app_file_write");
    assert_eq!(
        write.args,
        json!({"target":{"kind":"settings"},"content":"draft {}"})
    );
    replace_selections(&mut store, view, "late ", None).unwrap();
    let late = store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string();
    assert!(files.response(
        WRITE_SEQ,
        &Ok(ResponsePayload::Json(json!(null))),
        &mut store
    ));
    assert!(files.has_pending_writes());
    let canonical = send_next(&mut files, CANONICAL_SEQ);
    assert_eq!(canonical.command, "app_file_read");
    assert!(files.response(
        CANONICAL_SEQ,
        &Ok(ResponsePayload::Json(json!("canonical {}"))),
        &mut store
    ));
    assert!(!files.has_pending_writes());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        late
    );
    assert!(store.documents().snapshot(document).unwrap().dirty);
    assert!(files.prepare_save(&second, &mut store).unwrap());
    send_next(&mut files, FAILED_SEQ);
    assert!(files.response(
        FAILED_SEQ,
        &Err(json!({"code":"InvalidArgument","message":"synthetic invalid JSON"})),
        &mut store
    ));
    let errors = files.take_errors();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_write);
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        late
    );
    files.settings_updated(READ_SEQ.into());
    files.bind(first.clone(), &mut store).unwrap();
    send_next(&mut files, STALE_SEQ);
    files.unbind(&first, &mut store).unwrap();
    files.bind(first.clone(), &mut store).unwrap();
    send_next(&mut files, REMOUNT_SEQ);
    assert!(files.response(
        STALE_SEQ,
        &Ok(ResponsePayload::Json(json!("stale"))),
        &mut store
    ));
    assert!(files.view(&first, &store).is_none());
    assert!(files.response(
        REMOUNT_SEQ,
        &Ok(ResponsePayload::Json(json!("fresh"))),
        &mut store
    ));
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        late
    );
    assert!(files.view(&first, &store).is_some());
    assert!(files.prepare_save(&first, &mut store).unwrap());
    files.disconnected();
    assert!(!files.has_pending_writes());
    assert_eq!(files.take_errors().len(), 1);
    files.disconnected();
    assert!(files.take_errors().is_empty());
    assert!(files.prepare_save(&first, &mut store).unwrap());
    send_next(&mut files, CLOSED_OWNER_SEQ);
    files.unbind(&first, &mut store).unwrap();
    files.unbind(&second, &mut store).unwrap();
    assert!(files.has_pending_writes());
    assert!(files.response(
        CLOSED_OWNER_SEQ,
        &Ok(ResponsePayload::Json(json!(null))),
        &mut store
    ));
    assert!(!files.has_pending_writes());
    assert!(files.next_requests().is_empty());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        late
    );
}

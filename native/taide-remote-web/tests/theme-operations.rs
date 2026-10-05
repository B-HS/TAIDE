use std::collections::BTreeMap;

use serde_json::{Value, json};
use taide_model::{
    error::{AppError, AppErrorKind},
    ids::{PaneId, ProjectId, TabId},
    theme::{ResolvedTheme, ThemeEditorContext, ThemeSummary, ThemeType},
};
use taide_native_ui::{
    settings_owner::Owner,
    theme_draft::{Draft, Mode},
    theme_edit::{Command, Reply, Session},
};
use taide_remote_web::{ResponsePayload, theme_operations::ThemeOperations};

const LIST: u32 = 1;
const SOURCE: u32 = 2;
const BASE: u32 = 3;
const SAVE: u32 = 4;
const DELETE: u32 = 5;
const UNKNOWN: u32 = 6;

fn theme(id: &str) -> ResolvedTheme {
    ResolvedTheme {
        id: id.into(),
        name: "Synthetic".into(),
        theme_type: ThemeType::Dark,
        colors: BTreeMap::from([("app.accent".into(), "#123456".into())]),
        syntax: BTreeMap::new(),
        terminal: BTreeMap::new(),
        token_colors: None,
        syntax_overrides: Vec::new(),
        warnings: Vec::new(),
        author: None,
        license: None,
        source: None,
    }
}

fn themes() -> Vec<ThemeSummary> {
    [("taide-dark", true), ("synthetic-custom", false)]
        .into_iter()
        .map(|(id, builtin)| ThemeSummary {
            id: id.into(),
            name: "Synthetic".into(),
            theme_type: ThemeType::Dark,
            builtin,
        })
        .collect()
}

fn create_session(source: &str, mode: Mode) -> Session {
    Session::new(
        Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
        },
        source.into(),
        mode,
    )
    .unwrap()
}

fn response<T: serde::Serialize>(value: T) -> Result<ResponsePayload, Value> {
    Ok(ResponsePayload::Json(serde_json::to_value(value).unwrap()))
}

#[test]
fn 테마_명령은_실제_session_조회출처_중복억제_공용초안과_저장삭제응답을_보존한다() {
    let session = create_session("synthetic-custom", Mode::Edit);
    let request = session.load_request("ignored".into());
    let mut operations = ThemeOperations::default();
    operations.submit(Command::Load(request.clone()));
    operations.submit(Command::Load(request.clone()));
    let initial = operations.next_calls();
    assert_eq!(initial.len(), 2);
    for invocation in initial {
        assert!(operations.is_active(&invocation));
        assert_eq!(invocation.owner, *request.owner());
        let seq = if invocation.call.command == "theme_list" {
            LIST
        } else {
            SOURCE
        };
        if seq == SOURCE {
            assert_eq!(invocation.call.args, json!({"themeId":"synthetic-custom"}));
        }
        operations.sent(invocation, seq);
    }
    assert!(operations.next_calls().is_empty());
    assert!(!operations.response(UNKNOWN, &response(())));
    assert!(operations.response(SOURCE, &response(theme("synthetic-custom"))));
    assert!(operations.next_calls().is_empty());
    assert!(operations.response(LIST, &response(themes())));
    let base = operations.next_calls().pop().unwrap();
    assert_eq!(base.call.args, json!({"themeId":"taide-dark"}));
    operations.sent(base, BASE);
    assert!(operations.take_replies().is_empty());
    assert!(operations.response(BASE, &response(theme("taide-dark"))));
    let Reply::Loaded {
        request: loaded,
        result,
    } = operations.take_replies().pop().unwrap()
    else {
        panic!("loaded reply")
    };
    assert!(request.same_request(&loaded));
    let draft = result.unwrap();
    assert!(!draft.has_unsaved_changes());
    assert!(operations.next_calls().is_empty());
    let save = session.save_request(&draft).unwrap();
    operations.submit(Command::Save(Box::new(save.clone())));
    assert!(operations.has_pending_mutations());
    let saving = operations.next_calls().pop().unwrap();
    assert_eq!(saving.call.command, "theme_save");
    let context: ThemeEditorContext =
        serde_json::from_value(saving.call.args["editor"].clone()).unwrap();
    assert!(!context.is_create);
    assert_eq!(context.source_theme_id, "synthetic-custom");
    assert_eq!(context.project_id, request.owner().project);
    assert_eq!(
        saving.call.args["theme"],
        serde_json::to_value(save.theme()).unwrap()
    );
    operations.sent(saving, SAVE);
    assert!(operations.next_calls().is_empty());
    assert!(operations.response(SAVE, &response(themes().pop().unwrap())));
    assert!(!operations.has_pending_mutations());
    let Reply::Saved {
        request: saved,
        result,
    } = operations.take_replies().pop().unwrap()
    else {
        panic!("saved reply")
    };
    assert!(save.same_request(&saved));
    assert_eq!(result.unwrap().id, "synthetic-custom");
    let delete = session.delete_request().unwrap();
    operations.submit(Command::Delete(delete.clone()));
    let deleting = operations.next_calls().pop().unwrap();
    assert_eq!(deleting.call.command, "theme_delete");
    assert_eq!(deleting.call.args["themeId"], "synthetic-custom");
    operations.sent(deleting, DELETE);
    assert!(operations.has_pending_mutations());
    assert!(operations.response(DELETE, &response(())));
    let Reply::Deleted {
        request: deleted,
        result,
    } = operations.take_replies().pop().unwrap()
    else {
        panic!("deleted reply")
    };
    assert!(delete.same_request(&deleted));
    assert!(result.is_ok());
    assert!(!operations.has_pending_mutations());
    assert!(operations.failures().is_empty());
}

#[test]
fn 테마_소유자는_builtin_동일조회재사용_실패단일완료_tombstone_closed_자동replay없음을_보존한다() {
    let session = create_session("taide-dark", Mode::Create);
    let request = session.load_request("Synthetic copy".into());
    let mut operations = ThemeOperations::default();
    operations.submit(Command::Load(request.clone()));
    for invocation in operations.next_calls() {
        let seq = if invocation.call.command == "theme_list" {
            LIST
        } else {
            SOURCE
        };
        operations.sent(invocation, seq);
    }
    assert!(operations.response(SOURCE, &response(theme("taide-dark"))));
    assert!(operations.next_calls().is_empty());
    assert!(operations.response(LIST, &response(themes())));
    let reply = operations.take_replies().pop().unwrap();
    let Reply::Loaded { result, .. } = reply else {
        panic!("loaded")
    };
    let draft = result.unwrap();
    assert_eq!(draft.build().unwrap().id, "synthetic");
    assert_eq!(draft.current().name, "Synthetic copy");
    let save = session.save_request(&draft).unwrap();
    operations.submit(Command::Save(Box::new(save)));
    let saving = operations.next_calls().pop().unwrap();
    assert_eq!(saving.call.args["editor"]["isCreate"], true);
    operations.sent(saving, SAVE);
    assert!(operations.response(SAVE, &response(themes().pop().unwrap())));
    assert!(!operations.has_pending_mutations());
    let reply = operations.take_replies().pop().unwrap();
    assert_eq!(reply.error().unwrap().kind(), AppErrorKind::Internal);
    assert_eq!(operations.take_failures().len(), 1);
    assert!(operations.take_failures().is_empty());
    assert!(!operations.response(SAVE, &response(())));
    let save = session.save_request(&draft).unwrap();
    operations.submit(Command::Save(Box::new(save)));
    let saving = operations.next_calls().pop().unwrap();
    operations.sent(saving, SAVE);
    operations.disconnected();
    assert!(!operations.has_pending_mutations());
    assert!(operations.next_calls().is_empty());
    assert_eq!(
        operations
            .take_replies()
            .pop()
            .unwrap()
            .error()
            .unwrap()
            .kind(),
        AppErrorKind::Forbidden
    );
    assert_eq!(operations.take_failures().len(), 1);
    operations.disconnected();
    assert!(operations.take_replies().is_empty());

    let draft = Draft::from_resolved(
        "synthetic-custom",
        Mode::Edit,
        "ignored".into(),
        &themes(),
        theme("synthetic-custom"),
        theme("taide-dark"),
    )
    .unwrap();
    let edit = create_session("synthetic-custom", Mode::Edit);
    operations.submit(Command::Load(edit.load_request("ignored".into())));
    for invocation in operations.next_calls() {
        let seq = if invocation.call.command == "theme_list" {
            LIST
        } else {
            SOURCE
        };
        operations.sent(invocation, seq);
    }
    assert!(operations.response(
        LIST,
        &Err(serde_json::to_value(AppError::NotFound("synthetic".into())).unwrap())
    ));
    assert_eq!(operations.take_replies().len(), 1);
    assert!(operations.response(SOURCE, &response(theme("synthetic-custom"))));
    assert!(operations.take_replies().is_empty());
    assert!(operations.failures().is_empty());
    operations.submit(Command::Save(Box::new(edit.save_request(&draft).unwrap())));
    let saving = operations.next_calls().pop().unwrap();
    drop(edit);
    assert!(!operations.is_active(&saving));
    operations.failed(saving, AppError::Forbidden("synthetic owner closed".into()));
    assert_eq!(operations.take_replies().len(), 1);
    assert!(!operations.has_pending_mutations());
}

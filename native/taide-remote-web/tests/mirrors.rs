use serde_json::{Value, json};
use taide_model::ids::ProjectId;
use taide_model::layout::{LAYOUT_SCHEMA_VERSION, ProjectLayout};
use taide_remote_web::mirrors::{MirrorState, is_within_root};
use taide_remote_web::shell::Failure;
use taide_remote_web::{InvokeError, ResponsePayload};

const FIRST_SEQ: u32 = 1;
const FRESH_SEQ: u32 = 2;
const REOPEN_SEQ: u32 = 3;
const RETRY_SEQ: u32 = 4;
const OLD_SLOT: u32 = 10;
const KEPT_SLOT: u32 = 11;
const NEW_SLOT: u32 = 12;

#[test]
fn 미러_쓰기와_저장은_미조회_cache를_만들지_않고_대기중_옛_목록을_무효화한다() {
    let project = ProjectId("project".into());
    let mirror = serde_json::from_value(json!({"path":"/project/file","content":"draft","savedAtMs":1,"diskModifiedMs":1,"conflict":false,"sourceMissing":false})).unwrap();
    let mut state = MirrorState::default();
    state.record_write(&project, mirror);
    state.settle_path(&project, "/project/file");
    assert!(state.mirrors(&project).is_none());
    assert!(state.next_reads().is_empty());
    state.request(project.clone());
    state.sent(project.clone(), FIRST_SEQ);
    let mirror = serde_json::from_value(json!({"path":"/project/file","content":"current","savedAtMs":1,"diskModifiedMs":null,"conflict":false,"sourceMissing":true})).unwrap();
    state.record_write(&project, mirror);
    assert!(state.mirrors(&project).is_none());
    state.response(FIRST_SEQ, &Ok(ResponsePayload::Json(json!([]))));
    assert!(state.mirrors(&project).is_none());
    state.sent(project.clone(), FRESH_SEQ);
    state.response(FRESH_SEQ, &Ok(ResponsePayload::Json(json!([]))));
    let mirror = serde_json::from_value(json!({"path":"/project/file","content":"current","savedAtMs":1,"diskModifiedMs":null,"conflict":false,"sourceMissing":true})).unwrap();
    state.record_write(&project, mirror);
    assert!(state.mirrors(&project).unwrap()[0].source_missing);
    state.settle_path(&project, "/project/file");
    assert!(state.mirrors(&project).unwrap().is_empty());
    state.refresh(&project);
    state.sent(project.clone(), REOPEN_SEQ);
    state.settle_path(&project, "/project/file");
    state.response(REOPEN_SEQ, &Ok(ResponsePayload::Json(json!([]))));
    assert!(state.mirrors(&project).is_none());
    assert_eq!(
        state.next_reads().as_slice(),
        std::slice::from_ref(&project)
    );
}

fn layout(slots: &[u32]) -> ProjectLayout {
    let root = json!({"node":"leaf","id":"pane","tabs":[],"active":null});
    serde_json::from_value(json!({
        "version":LAYOUT_SCHEMA_VERSION, "root":root, "focusedPane":"pane",
        "auxiliaryWindows": slots.iter().map(|slot| json!({"slot":slot,"root":root,"focusedPane":"pane"})).collect::<Vec<_>>()
    })).unwrap()
}

#[test]
fn 미러_조회는_보조창_실제_소실과_rescan만_재조회하고_일반_watcher는_보존한다() {
    let project = ProjectId("project".into());
    let mut state = MirrorState::default();
    state.request(project.clone());
    state.sent(project.clone(), FIRST_SEQ);
    state.response(FIRST_SEQ, &Ok(ResponsePayload::Json(json!([]))));
    let mut layouts =
        std::collections::HashMap::from([(project.clone(), layout(&[OLD_SLOT, KEPT_SLOT]))]);
    state.reconcile_layouts(&layouts);
    assert!(state.next_reads().is_empty());
    layouts.insert(project.clone(), layout(&[KEPT_SLOT, NEW_SLOT]));
    state.reconcile_layouts(&layouts);
    assert_eq!(
        state.next_reads().as_slice(),
        std::slice::from_ref(&project)
    );
    state.sent(project.clone(), FRESH_SEQ);
    layouts.insert(project.clone(), layout(&[NEW_SLOT]));
    state.reconcile_layouts(&layouts);
    assert!(state.response(FRESH_SEQ, &Err(json!({"stale":"ignored"}))));
    assert!(state.failure(&project).is_none());
    state.sent(project.clone(), REOPEN_SEQ);
    state.response(REOPEN_SEQ, &Ok(ResponsePayload::Json(json!([]))));
    state.reconcile_layouts(&layouts);
    state.event("fs:changed");
    assert!(state.next_reads().is_empty());
    assert!(state.mirrors(&project).is_some());
    state.event("fs:rescan-required");
    assert!(state.mirrors(&project).is_none());
    assert_eq!(
        state.next_reads().as_slice(),
        std::slice::from_ref(&project)
    );
}

#[test]
fn 미러_조회는_프로젝트별_무효화_해제_재바인딩과_늦은_응답을_소유한다() {
    let project = ProjectId("project".into());
    let other = ProjectId("other".into());
    let mut state = MirrorState::default();
    state.request(project.clone());
    state.request(project.clone());
    assert_eq!(
        state.next_reads().as_slice(),
        std::slice::from_ref(&project)
    );
    assert_eq!(
        MirrorState::read_call(&project).command,
        "file_list_mirrors"
    );
    assert_eq!(
        MirrorState::read_call(&project).args,
        json!({"projectId":"project"})
    );
    state.sent(project.clone(), FIRST_SEQ);
    state.refresh(&project);
    assert!(state.next_reads().is_empty());
    assert!(state.response(
        FIRST_SEQ,
        &Ok(ResponsePayload::Json(json!([{"invalid":true}])))
    ));
    assert!(state.failure(&project).is_none());
    assert!(state.mirrors(&project).is_none());
    assert_eq!(
        state.next_reads().as_slice(),
        std::slice::from_ref(&project)
    );
    state.sent(project.clone(), FRESH_SEQ);
    state.request(other.clone());
    state.retain(|id| id == &other);
    state.request(project.clone());
    assert!(state.response(FRESH_SEQ, &Ok(ResponsePayload::Json(json!([])))));
    assert!(state.mirrors(&project).is_none());
    state.sent(project.clone(), REOPEN_SEQ);
    let data = json!([{"path":"/project/gone","content":"draft","savedAtMs":1,"diskModifiedMs":null,"conflict":false,"sourceMissing":true}]);
    assert!(state.response(REOPEN_SEQ, &Ok(ResponsePayload::Json(data))));
    let mirror = &state.mirrors(&project).unwrap()[0];
    assert_eq!(mirror.content, "draft");
    assert!(mirror.source_missing);
    assert!(state.mirrors(&other).is_none());
    state.disconnected();
    assert!(state.next_reads().is_empty());
    assert_eq!(
        state.failure(&project),
        Some(&Failure::Invocation(InvokeError::Closed))
    );
    state.refresh_all();
    state.sent(project.clone(), RETRY_SEQ);
    assert!(state.response(RETRY_SEQ, &Ok(ResponsePayload::Json(json!([])))));
    assert_eq!(state.mirrors(&project).unwrap().len(), 0);
    assert!(!state.response(FIRST_SEQ, &Ok(ResponsePayload::Json(Value::Null))));
}

#[test]
fn 미러_조회_오류는_공개하고_명시_retry까지_반복하지_않는다() {
    let project = ProjectId("project".into());
    let mut state = MirrorState::default();
    state.request(project.clone());
    for (seq, payload, expected) in [
        (
            FIRST_SEQ,
            Ok(ResponsePayload::Binary(vec![])),
            Failure::MalformedResponse,
        ),
        (
            FRESH_SEQ,
            Ok(ResponsePayload::Json(json!([{"path":"bad"}]))),
            Failure::MalformedResponse,
        ),
        (
            REOPEN_SEQ,
            Err(json!({"message":"synthetic"})),
            Failure::Remote(json!({"message":"synthetic"})),
        ),
    ] {
        state.refresh(&project);
        state.sent(project.clone(), seq);
        assert!(state.response(seq, &payload));
        assert_eq!(state.failure(&project), Some(&expected));
        assert!(state.next_reads().is_empty());
        assert!(state.mirrors(&project).is_none());
    }
    state.refresh(&project);
    state.invocation_failed(project.clone(), InvokeError::Closed);
    assert!(state.next_reads().is_empty());
    assert_eq!(
        state.failure(&project),
        Some(&Failure::Invocation(InvokeError::Closed))
    );
}

#[test]
fn 원격_미러_root_판정은_원본의_구분자_dot_드라이브_경계와_대소문자를_보존한다() {
    for (path, root, expected) in [
        (r"C:\project\folder\..\file.rs", "C:/project", true),
        ("C:/project/../../outside", "C:/project", false),
        ("c:/project/file", "C:/project", false),
        ("C:/projects/file", "C:/project", false),
        ("/project/./file", "/project", true),
        ("/project/../outside", "/project", false),
        ("relative/folder/../file", "relative", true),
        ("C:relative", "C:", false),
        ("/file", "/", false),
        ("/", "/", true),
    ] {
        assert_eq!(is_within_root(path, root), expected, "{path} {root}");
    }
}

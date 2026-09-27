use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use taide_lsp::install::LspInstallStore;
use taide_lsp::manifest;
use taide_lsp::session::LspMessageSubscribers;
use taide_lsp::store::{LspSessionEntry, LspStore};
use taide_model::app_event::AppEvent;
use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;
use taide_model::lsp::{LspServerId, LspSessionStatus};
use taide_runtime::{lsp_actions, EventSink};
use uuid::Uuid;

struct Fixture(PathBuf);

#[derive(Default)]
struct Events(Mutex<Vec<AppEvent>>);

impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn session() -> (LspStore, Arc<LspSessionEntry>) {
    let store = LspStore::new();
    let entry = Arc::new(LspSessionEntry::new(
        ProjectId::new(),
        manifest::find_spec("gopls").unwrap(),
        "fixture-root".to_string(),
        LspMessageSubscribers::new(),
    ));
    store.insert("fixture-session".to_string(), entry.clone());
    (store, entry)
}

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("taide-lsp-actions-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("자기 UUID LSP fixture만 정리");
    }
}

#[test]
fn 세션_조회는_프로젝트를_분리하고_기존_상태_snapshot을_유지한다() {
    let store = LspStore::new();
    let project = ProjectId::new();
    let other = ProjectId::new();
    let entry = Arc::new(LspSessionEntry::new(
        project.clone(),
        manifest::find_spec("gopls").unwrap(),
        "fixture-root".to_string(),
        LspMessageSubscribers::new(),
    ));
    store.insert("fixture-session".to_string(), entry.clone());
    assert!(lsp_actions::lsp_sessions(&store, other).unwrap().is_empty());
    let result = lsp_actions::lsp_sessions(&store, project.clone()).unwrap();
    assert_eq!(result, store.sessions_for_project(&project));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].session_id, "fixture-session");
    assert_eq!(result[0].status, LspSessionStatus::Starting);
    assert_eq!(result[0].generation, 0);
    assert!(entry.proc.lock().is_none());
}

#[test]
fn 알_수_없는_server는_경로_탐지_전에_기존_오류로_거절된다() {
    let id = LspServerId::from("fixture-unknown");
    let error = lsp_actions::lsp_resolve_root(id, String::new()).unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert_eq!(error.to_string(), "invalid argument: unknown language server: fixture-unknown");
    assert_eq!(lsp_actions::lsp_resolve_root("gopls".into(), String::new()).unwrap(), None);
}

#[test]
fn root_조회는_기존_가장_가까운_marker와_문자열_경로를_유지한다() {
    let fixture = Fixture::new();
    let root = fixture.0.join("한글 root");
    let nested = root.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(root.join("go.mod"), "module fixture\n").unwrap();
    let file = nested.join("main.go");
    let result = lsp_actions::lsp_resolve_root("gopls".into(), file.to_string_lossy().into_owned()).unwrap();
    assert_eq!(result, Some(root.to_string_lossy().into_owned()));
    assert_eq!(
        result,
        taide_lsp::service::find_root(&manifest::find_spec("gopls").unwrap(), &file).map(|path| path.to_string_lossy().into_owned())
    );
}

#[test]
fn 설치_취소는_멱등이며_worker_lease가_끝나기_전에_슬롯을_해제하지_않는다() {
    let store = LspInstallStore::new();
    let id = LspServerId::from("gopls");
    lsp_actions::lsp_install_cancel(&store, id.clone()).unwrap();
    let guard = store.begin(&id).unwrap();
    let lease = guard.lease();
    assert!(lease.ensure_active().is_ok());
    lsp_actions::lsp_install_cancel(&store, id.clone()).unwrap();
    lsp_actions::lsp_install_cancel(&store, id.clone()).unwrap();
    assert!(lease.ensure_active().is_err());
    assert!(store.begin(&id).is_none());
    drop(guard);
    assert!(store.begin(&id).is_none());
    drop(lease);
    let next = store.begin(&id).unwrap();
    assert!(next.lease().ensure_active().is_ok());
}

#[test]
fn tauri는_조회와_취소를_runtime에_위임하고_os_탐지는_adapter에_남긴다() {
    let source = include_str!("../src/domain/lsp/commands.rs");
    assert!(source.contains("lsp_actions::lsp_sessions(&store, project_id)"));
    assert!(source.contains("lsp_actions::lsp_resolve_root(server_id, file_path)"));
    assert!(source.contains("lsp_actions::lsp_install_cancel(&install_store, server_id)"));
    assert!(source.contains("service::detect_servers(&state.paths.lsp_dir(), &path_var)"));
    assert!(source.contains("perf::add(CounterSlot::LspSend, 1);"));
    assert!(source.contains("lsp_actions::lsp_send(&store, session_id, message).await"));
    assert!(source.contains("lsp_actions::lsp_confirm_reinitialize(&TauriEventSink(&app), &store, session_id, generation)"));
    assert!(source.contains("lsp_actions::lsp_report_reinitialize_failure(&TauriEventSink(&app), &store, session_id, generation)"));
}

#[tokio::test]
async fn send는_없는_세션과_준비되지_않은_proc의_기존_오류를_유지한다() {
    let (store, entry) = session();
    let error = lsp_actions::lsp_send(&store, "missing".to_string(), "fixture".to_string())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    let error = lsp_actions::lsp_send(&store, "fixture-session".to_string(), "fixture".to_string())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert_eq!(error.to_string(), "operation failed: language server not ready");
    assert!(entry.proc.lock().is_none());
}

#[test]
fn 재초기화_확인은_현재_crashed_generation만_전이하고_한_번_발행한다() {
    let (store, entry) = session();
    let events = Events::default();
    let crashed = entry.lifecycle.auto_respawned("fixture crash".to_string());
    lsp_actions::lsp_confirm_reinitialize(&events, &store, "fixture-session".to_string(), 0).unwrap();
    assert!(events.0.lock().unwrap().is_empty());
    assert_eq!(entry.lifecycle.snapshot(), crashed);
    lsp_actions::lsp_confirm_reinitialize(&events, &store, "fixture-session".to_string(), crashed.generation).unwrap();
    lsp_actions::lsp_confirm_reinitialize(&events, &store, "fixture-session".to_string(), crashed.generation).unwrap();
    let published = events.0.lock().unwrap();
    assert_eq!(published.len(), 1);
    assert!(matches!(
        &published[0],
        AppEvent::LspSessionStatusChanged { session_id, status: LspSessionStatus::Running, last_error: None, generation }
            if session_id == "fixture-session" && *generation == crashed.generation
    ));
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Running);
}

#[test]
fn 재초기화_실패는_기존_문구와_crashed_상태를_발행하고_이후_running에서는_생략한다() {
    let (store, entry) = session();
    let events = Events::default();
    let crashed = entry.lifecycle.auto_respawned("fixture crash".to_string());
    lsp_actions::lsp_report_reinitialize_failure(&events, &store, "fixture-session".to_string(), 0).unwrap();
    assert!(events.0.lock().unwrap().is_empty());
    assert_eq!(entry.lifecycle.snapshot(), crashed);
    lsp_actions::lsp_report_reinitialize_failure(&events, &store, "fixture-session".to_string(), crashed.generation).unwrap();
    let snapshot = entry.lifecycle.snapshot();
    assert_eq!(snapshot.status, LspSessionStatus::Crashed);
    assert_eq!(
        snapshot.last_error.as_deref(),
        Some("초기화 핸드셰이크 재시도를 모두 소진해 서버를 재연결하지 못했습니다. 수동으로 다시 시작해주세요.")
    );
    {
        let published = events.0.lock().unwrap();
        assert_eq!(published.len(), 1);
        assert!(matches!(
            &published[0],
            AppEvent::LspSessionStatusChanged { session_id, status: LspSessionStatus::Crashed, last_error, generation }
                if session_id == "fixture-session" && last_error == &snapshot.last_error && *generation == crashed.generation
        ));
    }
    lsp_actions::lsp_confirm_reinitialize(&events, &store, "fixture-session".to_string(), crashed.generation).unwrap();
    lsp_actions::lsp_report_reinitialize_failure(&events, &store, "fixture-session".to_string(), crashed.generation).unwrap();
    assert_eq!(events.0.lock().unwrap().len(), 2);
}

#[test]
fn 재초기화의_없는_세션은_이벤트_없이_거절된다() {
    let store = LspStore::new();
    let events = Events::default();
    for result in [
        lsp_actions::lsp_confirm_reinitialize(&events, &store, "missing".to_string(), 0),
        lsp_actions::lsp_report_reinitialize_failure(&events, &store, "missing".to_string(), 0),
    ] {
        assert_eq!(result.unwrap_err().kind(), AppErrorKind::NotFound);
    }
    assert!(events.0.lock().unwrap().is_empty());
}

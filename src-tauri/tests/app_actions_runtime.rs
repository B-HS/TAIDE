use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::task::Poll;
use std::time::Duration;

use taide_model::app::{AppFileTarget, PromptTemplateId};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind};
use taide_model::paths::AppPaths;
use taide_model::settings::Settings;
use taide_runtime::{app_actions, settings_actions, AppState, EventSink};
use uuid::Uuid;

const ACTION_TIMEOUT: Duration = Duration::from_secs(5);

struct Fixture {
    dir: PathBuf,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-app-actions-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("테스트 디렉터리 생성");
        let state = AppState::new(AppPaths::new(dir.clone()));
        Self { dir, state }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("테스트 디렉터리 정리");
    }
}

#[derive(Default)]
struct RecordingEventSink(Mutex<Vec<AppEvent>>);

impl EventSink for RecordingEventSink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn 읽기는_현재_설정_fallback과_디스크_내용을_보존한다() {
    let fixture = Fixture::new();
    fixture.state.settings.write().agent_hooks_enabled = true;
    let content = app_actions::app_file_read(&fixture.state, AppFileTarget::Settings)
        .await
        .expect("메모리 설정 fallback");
    let parsed = taide_settings::service::parse_settings_json(&content).expect("fallback 파싱");
    assert_eq!(parsed, *fixture.state.settings.read());
    let raw = "{\"version\":1,\"agentHooksEnabled\":false}";
    std::fs::write(fixture.state.paths.settings_file(), raw).expect("설정 fixture 저장");
    assert_eq!(
        app_actions::app_file_read(&fixture.state, AppFileTarget::Settings)
            .await
            .expect("파일 읽기"),
        raw,
    );
    let prompt = app_actions::app_file_read(
        &fixture.state,
        AppFileTarget::Prompt {
            id: PromptTemplateId::InlineEditDefault,
        },
    )
    .await
    .expect("번들 프롬프트 fallback");
    assert!(!prompt.is_empty());
}

#[tokio::test]
async fn 잘못된_설정은_포트를_호출하지_않고_파일과_상태를_보존한다() {
    let fixture = Fixture::new();
    let original = fixture.state.settings.read().clone();
    let raw = "{\"version\":1}";
    std::fs::write(fixture.state.paths.settings_file(), raw).expect("기존 설정 저장");
    let called = AtomicBool::new(false);
    let error = app_actions::app_file_write(&fixture.state, AppFileTarget::Settings, "{ not json".to_string(), |next| async {
        called.store(true, Ordering::SeqCst);
        Ok(next)
    })
    .await
    .expect_err("잘못된 설정 거절");
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(*fixture.state.settings.read(), original);
    assert_eq!(std::fs::read_to_string(fixture.state.paths.settings_file()).unwrap(), raw);
}

#[tokio::test]
async fn 설정_쓰기는_잠금_안에서_공통_적용과_이벤트_완료를_기다린다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let next = Settings {
        agent_hooks_enabled: true,
        ..fixture.state.settings.read().clone()
    };
    let content = serde_json::to_string(&next).expect("설정 직렬화");
    let called = AtomicBool::new(false);
    let guard = fixture.state.begin_mutation().await;
    let mut write = Box::pin(app_actions::app_file_write(
        &fixture.state,
        AppFileTarget::Settings,
        content,
        |parsed| async {
            called.store(true, Ordering::SeqCst);
            let mut acquisition = Box::pin(fixture.state.begin_mutation());
            poll_fn(|cx| {
                assert!(acquisition.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            drop(acquisition);
            tokio::task::yield_now().await;
            settings_actions::apply_and_broadcast(&fixture.state, parsed, |_, _| async {}, &sink).await
        },
    ));
    poll_fn(|cx| {
        assert!(write.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!called.load(Ordering::SeqCst));
    assert!(!fixture.state.paths.settings_file().exists());
    drop(guard);
    tokio::time::timeout(ACTION_TIMEOUT, write)
        .await
        .expect("중첩 mutation 재취득 없음")
        .expect("설정 쓰기 완료");
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(*fixture.state.settings.read(), next);
    assert_eq!(*sink.0.lock().unwrap(), [AppEvent::SettingsChanged { settings: Box::new(next) }]);
}

#[tokio::test]
async fn 프롬프트는_설정_포트_없이_잠금과_검증_후_저장한다() {
    let fixture = Fixture::new();
    let target = AppFileTarget::Prompt {
        id: PromptTemplateId::InlineEditDefault,
    };
    let content = app_actions::app_file_read(&fixture.state, target)
        .await
        .expect("유효한 번들 프롬프트");
    let path = taide_app::service::app_file_path(&fixture.state.paths, target);
    let called = AtomicBool::new(false);
    let guard = fixture.state.begin_mutation().await;
    let mut write = Box::pin(app_actions::app_file_write(&fixture.state, target, content.clone(), |next| async {
        called.store(true, Ordering::SeqCst);
        Ok(next)
    }));
    poll_fn(|cx| {
        assert!(write.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!path.exists());
    drop(guard);
    write.await.expect("프롬프트 저장");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    let error = app_actions::app_file_write(&fixture.state, target, "{ not json".to_string(), |next| async {
        called.store(true, Ordering::SeqCst);
        Ok(next)
    })
    .await
    .expect_err("잘못된 프롬프트 거절");
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
    assert!(!called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn parsed_설정_적용은_잠금을_취득하고_포트_오류를_전파한다() {
    let fixture = Fixture::new();
    let original = fixture.state.settings.read().clone();
    let called = AtomicBool::new(false);
    let guard = fixture.state.begin_mutation().await;
    let mut apply = Box::pin(app_actions::apply_settings_file(&fixture.state, original.clone(), |parsed| {
        let original = &original;
        let called = &called;
        async move {
            assert_eq!(parsed, *original);
            called.store(true, Ordering::SeqCst);
            Err(AppError::InvalidArgument("fixture rejection".to_string()))
        }
    }));
    poll_fn(|cx| {
        assert!(apply.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!called.load(Ordering::SeqCst));
    drop(guard);
    assert_eq!(apply.await.expect_err("포트 오류 전파").kind(), AppErrorKind::InvalidArgument);
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(*fixture.state.settings.read(), original);
    assert!(!fixture.state.paths.settings_file().exists());
}

#[test]
fn app_adapter는_파일_action만_위임하고_remote_strip과_metadata_perf_경계를_유지한다() {
    let commands = include_str!("../src/domain/app/commands.rs");
    for action in ["app_file_read", "app_file_write", "apply_settings_file"] {
        assert!(commands.contains(&format!("app_actions::{action}(")));
    }
    assert_eq!(commands.matches("(apply_settings.0)(&app, &state,").count(), 2);
    assert!(!commands.contains("begin_mutation("));
    assert!(!commands.contains("parse_settings_json("));
    assert!(commands.contains("service::app_info()"));
    assert!(commands.contains("perf::global().reset()"));
    let gateway = include_str!("../src/remote_gateway.rs");
    assert!(gateway.contains("strip_remote_gated_settings(parsed, &current)"));
    assert!(gateway.contains("apply_settings_file(app.clone(), app.state(), app.state(), sanitized)"));
}

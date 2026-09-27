use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::task::Poll;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::error::AppErrorKind;
use taide_model::paths::AppPaths;
use taide_model::settings::{Settings, SettingsPatch};
use taide_runtime::{settings_actions, AppState, EventSink};
use taide_settings::service;
use uuid::Uuid;

const ACTION_TIMEOUT: Duration = Duration::from_secs(5);

struct Fixture {
    dir: PathBuf,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-settings-actions-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("테스트 디렉터리 생성");
        let state = AppState::new(AppPaths::new(dir.clone()));
        Self { dir, state }
    }

    fn persisted(&self) -> Settings {
        service::parse_settings_json(&std::fs::read_to_string(self.state.paths.settings_file()).expect("설정 파일 읽기"))
            .expect("설정 파싱")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("테스트 디렉터리 정리");
    }
}

#[derive(Default)]
struct RecordingEventSink {
    events: Mutex<Vec<AppEvent>>,
    order: Mutex<Vec<&'static str>>,
}

impl EventSink for RecordingEventSink {
    fn publish(&self, event: AppEvent) {
        let name = match &event {
            AppEvent::SettingsChanged { .. } => "settings-changed",
            AppEvent::ThemeChanged { .. } => "theme-changed",
            _ => panic!("예상 밖 이벤트"),
        };
        self.order.lock().unwrap().push(name);
        self.events.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn 공통_적용은_이미_잡은_잠금_안에서_저장과_상태와_observer와_이벤트_순서를_보존한다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let current = fixture.state.settings.read().clone();
    let next = Settings {
        editor_font_size: 0,
        agent_hooks_enabled: !current.agent_hooks_enabled,
        ..current.clone()
    };
    let expected = service::sanitize(next.clone());
    let _guard = fixture.state.begin_mutation().await;
    let updated = tokio::time::timeout(
        ACTION_TIMEOUT,
        settings_actions::apply_and_broadcast(
            &fixture.state,
            next,
            |previous, applied| {
                let fixture = &fixture;
                let sink = &sink;
                let current = &current;
                let expected = &expected;
                async move {
                    assert_eq!(previous, *current);
                    assert_eq!(applied, *expected);
                    assert_eq!(fixture.persisted(), applied);
                    assert_eq!(*fixture.state.settings.read(), applied);
                    assert!(sink.events.lock().unwrap().is_empty());
                    sink.order.lock().unwrap().push("observer-start");
                    tokio::task::yield_now().await;
                    assert!(sink.events.lock().unwrap().is_empty());
                    sink.order.lock().unwrap().push("observer-end");
                }
            },
            &sink,
        ),
    )
    .await
    .expect("공통 경로는 mutation 잠금을 재취득하지 않음")
    .expect("설정 적용");
    assert_eq!(updated, expected);
    assert_eq!(settings_actions::settings_get(&fixture.state).await.expect("설정 조회"), updated);
    assert_eq!(*sink.order.lock().unwrap(), ["observer-start", "observer-end", "settings-changed"],);
    assert_eq!(
        *sink.events.lock().unwrap(),
        [AppEvent::SettingsChanged {
            settings: Box::new(updated)
        }],
    );
}

#[tokio::test]
async fn 저장_실패는_기존_상태와_파일을_보존하고_observer와_이벤트를_실행하지_않는다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let current = fixture.state.settings.read().clone();
    let settings_path = fixture.state.paths.settings_file();
    std::fs::create_dir_all(&settings_path).expect("저장 실패 대상 디렉터리");
    let sentinel = settings_path.join("sentinel.txt");
    std::fs::write(&sentinel, "unchanged").expect("보존할 파일");
    let observer_called = AtomicBool::new(false);
    let error = settings_actions::apply_and_broadcast(
        &fixture.state,
        Settings {
            agent_hooks_enabled: !current.agent_hooks_enabled,
            ..current.clone()
        },
        |_, _| async { observer_called.store(true, Ordering::SeqCst) },
        &sink,
    )
    .await
    .expect_err("디렉터리에 설정 저장 거절");
    assert_eq!(error.kind(), AppErrorKind::Io);
    assert_eq!(*fixture.state.settings.read(), current);
    assert_eq!(std::fs::read_to_string(sentinel).expect("기존 파일 확인"), "unchanged");
    assert!(!observer_called.load(Ordering::SeqCst));
    assert!(sink.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn 설정_patch는_잠금_해제_뒤_최신_상태를_읽고_공통_적용을_호출한다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let guard = fixture.state.begin_mutation().await;
    let observer_called = AtomicBool::new(false);
    let mut update = Box::pin(settings_actions::settings_update(
        &fixture.state,
        SettingsPatch {
            editor_font_size: Some(0),
            ..Default::default()
        },
        |_, _| async { observer_called.store(true, Ordering::SeqCst) },
        &sink,
    ));
    poll_fn(|cx| {
        assert!(update.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!fixture.state.paths.settings_file().exists());
    assert!(!observer_called.load(Ordering::SeqCst));
    fixture.state.settings.write().agent_hooks_enabled = true;
    drop(guard);
    let updated = update.await.expect("잠금 해제 뒤 patch 적용");
    assert!(updated.agent_hooks_enabled);
    assert_eq!(
        updated.editor_font_size,
        service::sanitize(Settings {
            editor_font_size: 0,
            ..Default::default()
        })
        .editor_font_size
    );
    assert_eq!(fixture.persisted(), updated);
    assert!(observer_called.load(Ordering::SeqCst));
    assert_eq!(
        *sink.events.lock().unwrap(),
        [AppEvent::SettingsChanged {
            settings: Box::new(updated)
        }]
    );
}

#[tokio::test]
async fn 테마_변경은_잠금과_시스템_테마_해제를_보존하고_설정_이벤트_뒤_테마를_발행한다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    fixture.state.settings.write().follow_system_theme = true;
    let guard = fixture.state.begin_mutation().await;
    let theme_id = taide_theme::service::BUILTIN_LIGHT_ID.to_string();
    let mut update = Box::pin(settings_actions::settings_set_theme(
        &fixture.state,
        theme_id.clone(),
        |previous, applied| async move {
            assert!(previous.follow_system_theme);
            assert!(!applied.follow_system_theme);
            tokio::task::yield_now().await;
        },
        &sink,
    ));
    poll_fn(|cx| {
        assert!(update.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!fixture.state.paths.settings_file().exists());
    drop(guard);
    let updated = update.await.expect("테마 변경");
    assert_eq!(fixture.persisted(), updated);
    assert_eq!(updated.theme_id, theme_id);
    assert!(!updated.follow_system_theme);
    assert_eq!(*sink.order.lock().unwrap(), ["settings-changed", "theme-changed"]);
    assert_eq!(
        *sink.events.lock().unwrap(),
        [
            AppEvent::SettingsChanged {
                settings: Box::new(updated)
            },
            AppEvent::ThemeChanged { theme_id }
        ],
    );
}

#[tokio::test]
async fn 없는_테마는_저장과_상태와_observer와_이벤트를_변경하지_않는다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let current = fixture.state.settings.read().clone();
    let observer_called = AtomicBool::new(false);
    let error = settings_actions::settings_set_theme(
        &fixture.state,
        "missing-theme".to_string(),
        |_, _| async { observer_called.store(true, Ordering::SeqCst) },
        &sink,
    )
    .await
    .expect_err("없는 테마 거절");
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(*fixture.state.settings.read(), current);
    assert!(!fixture.state.paths.settings_file().exists());
    assert!(!observer_called.load(Ordering::SeqCst));
    assert!(sink.events.lock().unwrap().is_empty());
}

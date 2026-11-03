use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use super::*;
use crate::application::project_rail_tests::{
    Fixture, insert_project, isolated_state, project_ref,
};
use taide_model::agent::{AgentActivity, DetectedAgent};
use taide_model::app_event::AppEvent;
use taide_model::ids::PaneId;
use taide_model::layout::{PaneNode, Tab};
use taide_model::lsp::{LspInstallPhase, LspServerId};
use taide_native_ui::toast::Kind;
use taide_runtime::{AppState, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(10);
const SCREEN: [f32; 2] = [1000.0, 700.0];
const LONG_COMMAND_MS: u32 = 12_000;
const SHORT_COMMAND_MS: u32 = 9_999;
const AGENT_WORKED: Duration = Duration::from_secs(11);
const TERMINAL_SESSION: &str = "synthetic-terminal";
const TERMINAL_TITLE: &str = "Build";

#[derive(Default)]
pub(in crate::application) struct FakePlatform {
    pub(super) sent: Mutex<Vec<(String, String)>>,
}

impl FakePlatform {
    pub(in crate::application) fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().unwrap().clone()
    }
}

impl taide_runtime::PlatformServices for FakePlatform {
    fn open_path(&self, _: &std::path::Path) -> AppResult<()> {
        Err(AppError::Forbidden("synthetic platform".into()))
    }
    fn reveal_item_in_dir(&self, _: &std::path::Path) -> AppResult<()> {
        Err(AppError::Forbidden("synthetic platform".into()))
    }
    fn open_url(&self, _: &str) -> AppResult<()> {
        Err(AppError::Forbidden("synthetic platform".into()))
    }
    fn send_notification(&self, title: &str, body: &str) -> AppResult<()> {
        self.sent
            .lock()
            .unwrap()
            .push((title.to_owned(), body.to_owned()));
        Ok(())
    }
}

pub(in crate::application) fn launch_with_platform(
    fixture: &mut Fixture,
    state: AppState,
    platform: Arc<FakePlatform>,
) -> egui::Context {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let context = egui::Context::default();
    fixture.application = Some(
        NativeApplication::new_with_overrides(
            &eframe::CreationContext::_new_kittest(context.clone()),
            runtime,
            state,
            tasks,
            fixture.directory.join("bin/taide"),
            Vec::new(),
            crate::application::Overrides {
                platform: Some(platform),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    context
}

pub(super) fn frame(application: &mut NativeApplication, context: &egui::Context, focused: bool) {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(SCREEN[0], SCREEN[1]),
            )),
            focused,
            ..Default::default()
        },
        |ui| {
            eframe::App::logic(application, ui.ctx(), &mut eframe::Frame::_new_kittest());
            eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest());
        },
    );
    output.textures_delta.clear();
}

pub(super) fn frame_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    focused: bool,
    phase: &str,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let deadline = Instant::now() + DEADLINE;
    while !ready(application) {
        frame(application, context, focused);
        assert!(
            Instant::now() < deadline,
            "phase={phase} status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

fn hint_count(application: &NativeApplication) -> usize {
    let hint = presentation::message(&application.locale, "notification.enableHint", &[]);
    application
        .toasts
        .inspection()
        .iter()
        .filter(|toast| toast.kind == Kind::Info && toast.title == hint)
        .count()
}

fn notifying_fixture(name: &str) -> (Fixture, AppState, ProjectId) {
    let fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-notifications-{name}-{}",
            ProjectId::new()
        )),
        application: None,
    };
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(fixture.directory.join("data")).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    let root: PathBuf = root.canonicalize().unwrap();
    let state = isolated_state(fixture.directory.join("data"));
    let project = ProjectId::new();
    insert_project(&state, &project, &root, "alpha", false);
    if let Some(layout) = state.layouts.write().get_mut(&project) {
        let pane = PaneId::new();
        layout.root = PaneNode::Leaf {
            id: pane.clone(),
            tabs: vec![Tab {
                id: TabId::new(),
                kind: TabKind::Terminal {
                    session_id: TERMINAL_SESSION.into(),
                    cwd: None,
                },
                title: TERMINAL_TITLE.into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: None,
        };
        layout.focused_pane = pane;
    }
    state.session.write().projects = vec![project_ref(&project, &root, "alpha")];
    {
        let mut settings = state.settings.write();
        settings.notifications_enabled = true;
        settings.notifications_only_when_unfocused = true;
        settings.notify_task_completed = false;
        settings.notify_error = true;
        settings.notify_lsp_install = true;
        settings.notify_agent_completed = true;
        settings.notify_agent_awaiting_input = true;
    }
    (fixture, state, project)
}

fn command(exit_code: Option<i32>, duration_ms: u32) -> AppEvent {
    AppEvent::TerminalCommandFinished {
        session_id: TERMINAL_SESSION.into(),
        cwd: Some("/synthetic/cwd".into()),
        exit_code,
        duration_ms,
    }
}

fn lsp(phase: LspInstallPhase) -> AppEvent {
    AppEvent::LspInstallProgress {
        server_id: LspServerId("synthetic-server".into()),
        phase,
        received_bytes: 0.0,
        total_bytes: None,
        message: None,
    }
}

#[test]
fn 실제_app은_명령완료와_lsp설치를_os알림으로_보내고_포커스_범주_첫전달_안내를_지킨다() {
    let (mut fixture, state, project) = notifying_fixture("command");
    let platform = Arc::new(FakePlatform::default());
    let context = launch_with_platform(&mut fixture, state, platform.clone());
    let application = fixture.application.as_mut().unwrap();
    frame_until(
        application,
        &context,
        true,
        "project snapshot",
        |application| {
            application
                .controller
                .snapshot()
                .project(&project)
                .is_some()
        },
    );
    let events = application.services.events.clone();

    events.publish(command(Some(1), LONG_COMMAND_MS));
    frame(application, &context, true);
    events.publish(command(Some(1), SHORT_COMMAND_MS));
    frame(application, &context, false);
    events.publish(command(Some(0), LONG_COMMAND_MS));
    frame(application, &context, false);
    events.publish(lsp(LspInstallPhase::Downloading));
    frame(application, &context, false);
    assert!(platform.sent().is_empty());
    assert_eq!(hint_count(application), 0);

    events.publish(command(Some(1), LONG_COMMAND_MS));
    frame_until(application, &context, false, "failed command", |_| {
        !platform.sent().is_empty()
    });
    assert_eq!(
        platform.sent(),
        vec![("alpha".to_owned(), "Build · Exit code 1 · 12s".to_owned())]
    );
    frame_until(
        application,
        &context,
        false,
        "first delivery hint",
        |application| hint_count(application) == 1,
    );

    events.publish(lsp(LspInstallPhase::Done));
    frame_until(application, &context, false, "lsp installed", |_| {
        platform.sent().len() == 2
    });
    assert_eq!(
        platform.sent()[1],
        (
            "Language server installed".to_owned(),
            "synthetic-server".to_owned()
        )
    );
    for _ in 0..4 {
        frame(application, &context, false);
    }
    assert_eq!(hint_count(application), 1);
}

#[test]
fn 실제_app은_10초_넘게_일한_agent의_완료를_프로젝트_제목과_탭꼬리로_알린다() {
    let (mut fixture, state, project) = notifying_fixture("agent");
    state.settings.write().notifications_only_when_unfocused = false;
    let platform = Arc::new(FakePlatform::default());
    let context = launch_with_platform(&mut fixture, state, platform.clone());
    let application = fixture.application.as_mut().unwrap();
    frame_until(
        application,
        &context,
        false,
        "project snapshot",
        |application| {
            application
                .controller
                .snapshot()
                .project(&project)
                .is_some()
        },
    );
    let agent = |activity| DetectedAgent {
        session_id: TERMINAL_SESSION.into(),
        name: "claude".into(),
        pid: 1,
        activity,
        blocked_reason: None,
    };
    let start = Instant::now();
    application.shell_state.provider_events.take();
    application
        .shell_state
        .provider_events
        .record(&AppEvent::AgentStateChanged {
            project_id: project.clone(),
            agents: vec![agent(AgentActivity::Working)],
        });
    application.consume_provider_events(&context, start);
    application
        .shell_state
        .provider_events
        .record(&AppEvent::AgentStateChanged {
            project_id: project.clone(),
            agents: vec![agent(AgentActivity::Idle)],
        });
    application.consume_provider_events(&context, start + AGENT_WORKED);
    frame_until(application, &context, false, "agent finished", |_| {
        !platform.sent().is_empty()
    });
    assert_eq!(
        platform.sent(),
        vec![(
            "alpha".to_owned(),
            "claude · Finished · 11s · Build".to_owned()
        )]
    );
}

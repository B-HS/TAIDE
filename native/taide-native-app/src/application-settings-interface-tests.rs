use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use taide_agent::service::HookEmitter;
use taide_infra::perf::{PerfRegistry, SpanSlot};
use taide_model::agent::{AgentActivity, CliInstallStatus, DetectedAgent};
use taide_model::lsp::LspInstallPhase;
use taide_native_ui::settings_controls::{Change, Switch};
use taide_native_ui::toast::Kind;
use taide_runtime::agent_host::CliCommandOutcome;

use super::settings_sections_tests::{
    Scene, errors, missing_cli_agents, nodes, painted, successes, texts_of, update,
};
use super::*;
use crate::host::HostBridge;
use crate::settings_ports::{CliPorts, Ports};

const SPAN_NANOS: u64 = 2_000_000;
const REFUSED: &str = "fixture platform refused";
const LONG_COMMAND_MS: u32 = 12_000;
const NOTIFY_AGENT_WORKED: Duration = Duration::from_secs(11);
const NOTIFY_SETTLE_FRAMES: usize = 8;
const NOTIFY_SESSION: &str = "settings-notify-session";

fn cli_status(installed: bool) -> CliInstallStatus {
    CliInstallStatus {
        installed,
        resolved_path: None,
        dangling: false,
        target_path: crate::agent_hooks::TAIDE_CLI_TARGET_PATH.into(),
        editor_env_hint: String::new(),
    }
}

fn outcome(installed: bool, is_cancelled: bool) -> AppResult<CliCommandOutcome> {
    Ok(CliCommandOutcome {
        status: cli_status(installed),
        is_cancelled,
    })
}

fn warnings(application: &NativeApplication) -> Vec<String> {
    application
        .toasts
        .inspection()
        .into_iter()
        .filter(|toast| toast.kind == Kind::Warning)
        .map(|toast| toast.title)
        .collect()
}

fn click_label(scene: &mut Scene, label: &str) {
    let output = scene.step(Vec::new());
    let node = *nodes(&output, label)
        .first()
        .unwrap_or_else(|| panic!("node exists: {label}"));
    scene.click(node.center());
}

fn sections_data(application: &NativeApplication) -> &sections::Data {
    application.settings_state.settings_views.sections_data()
}

struct Queue {
    results: Mutex<VecDeque<AppResult<CliCommandOutcome>>>,
    calls: AtomicUsize,
}

impl Queue {
    fn new(results: Vec<AppResult<CliCommandOutcome>>) -> Arc<Self> {
        Arc::new(Self {
            results: Mutex::new(results.into()),
            calls: AtomicUsize::new(0),
        })
    }

    fn command(self: &Arc<Self>) -> crate::settings_ports::CliCommand {
        let queue = self.clone();
        Arc::new(move || {
            queue.calls.fetch_add(1, Ordering::SeqCst);
            queue
                .results
                .lock()
                .unwrap()
                .pop_front()
                .expect("fixture command result")
        })
    }
}

#[test]
fn 실제_cli_행은_가짜_포트로_취소_무음_설치_실패_해제를_원본_문구로_알리고_버튼을_바꾼다() {
    let mut scene = Scene::launch(None, |_| {});
    let installs = Queue::new(vec![
        outcome(false, true),
        Err(AppError::Internal("fixture install refused".into())),
        outcome(true, false),
    ]);
    let uninstalls = Queue::new(vec![
        Err(AppError::Internal("fixture uninstall refused".into())),
        outcome(false, false),
    ]);
    let mut agents = crate::remote_agents::Ports::new();
    agents.cli_status = Arc::new(|| cli_status(false));
    scene.ports(Ports {
        detect_lsp_servers: Arc::new(|_| Vec::new()),
        agents: Arc::new(agents),
        cli: CliPorts {
            install: installs.command(),
            uninstall: uninstalls.command(),
        },
        ..Default::default()
    });
    let title = scene.text("keymap.category.shellCommand");
    let install = scene.text("settings.cliInstallButton");
    let uninstall = scene.text("settings.cliUninstallButton");
    let installed = scene.text("settings.cliStatusInstalled");
    let install_success = scene.text("settings.cliInstallSuccess");
    let install_failed = scene.text("settings.cliInstallFailed");
    let uninstall_success = scene.text("settings.cliUninstallSuccess");
    let uninstall_failed = scene.text("settings.cliUninstallFailed");
    let output = scene.until("cli row", |application, output| {
        !texts_of(output, &title).is_empty()
            && matches!(
                sections_data(application).cli_status(),
                sections::Status::Ready(_)
            )
    });
    assert_eq!(texts_of(&output, &install).len(), 1);
    assert!(texts_of(&output, &uninstall).is_empty());

    click_label(&mut scene, &install);
    scene.until("cancelled", |application, _| {
        installs.calls.load(Ordering::SeqCst) == 1
            && !sections_data(application).is_busy(sections::Busy::CliInstall)
    });
    assert!(successes(scene.application()).is_empty());
    assert!(errors(scene.application()).is_empty());
    click_label(&mut scene, &install);
    scene.until("install failed", |application, _| {
        errors(application).contains(&install_failed)
    });
    click_label(&mut scene, &install);
    let output = scene.until("installed", |application, output| {
        successes(application).contains(&install_success)
            && !texts_of(output, &uninstall).is_empty()
    });
    assert_eq!(installs.calls.load(Ordering::SeqCst), 3);
    assert!(texts_of(&output, &install).is_empty());
    assert_eq!(texts_of(&output, &installed).len(), 1);

    click_label(&mut scene, &uninstall);
    scene.until("uninstall failed", |application, _| {
        errors(application).contains(&uninstall_failed)
    });
    click_label(&mut scene, &uninstall);
    let output = scene.until("uninstalled", |application, output| {
        successes(application).contains(&uninstall_success)
            && !texts_of(output, &install).is_empty()
    });
    assert!(texts_of(&output, &uninstall).is_empty());
    assert!(texts_of(&output, &installed).is_empty());
    assert_eq!(uninstalls.calls.load(Ordering::SeqCst), 2);
}

struct Toggles {
    hooks: Mutex<Vec<(bool, bool)>>,
    ide: Mutex<Vec<(bool, bool)>>,
}

fn hooks_scene(home: &std::path::Path) -> (Scene, Arc<Toggles>) {
    let mut scene = Scene::launch_with(None, |_| {}, false);
    let toggles = Arc::new(Toggles {
        hooks: Mutex::new(Vec::new()),
        ide: Mutex::new(Vec::new()),
    });
    {
        let recorded = toggles.clone();
        let repaint = scene.context().clone();
        let application = scene.application();
        let bridge = HostBridge::connect_with_settings_ports(
            application.services.clone(),
            Arc::new(move || repaint.request_repaint()),
            Arc::new(|_| Ok(())),
            Arc::new(|| Ok(String::new())),
            Some(crate::host::Terminals {
                tabs: application.terminal_agent_state.terminals.clone(),
                environment: crate::terminal_environment::provider(),
            }),
            Arc::new(move |_, current, updated| {
                if current.agent_hooks_enabled != updated.agent_hooks_enabled {
                    recorded
                        .hooks
                        .lock()
                        .unwrap()
                        .push((current.agent_hooks_enabled, updated.agent_hooks_enabled));
                }
                if current.ide_integration_enabled != updated.ide_integration_enabled {
                    recorded.ide.lock().unwrap().push((
                        current.ide_integration_enabled,
                        updated.ide_integration_enabled,
                    ));
                }
                Box::pin(async {})
            }),
        )
        .unwrap();
        let old = application.bridge.replace(bridge).unwrap();
        application.runtime.block_on(old.disconnect()).unwrap();
    }
    let home = home.to_str().unwrap().to_string();
    let mut agents = crate::remote_agents::Ports::new();
    agents.cli_status = Arc::new(|| cli_status(false));
    agents.resolve_home = Arc::new(move || Some(home.clone()));
    agents.project_emitter = Arc::new(|_| Box::pin(async { HookEmitter::TerminalSequence }));
    agents.cli_target = "/fixture/bin/taide".into();
    scene.ports(Ports {
        detect_lsp_servers: Arc::new(|_| Vec::new()),
        agents: Arc::new(agents),
        ..Default::default()
    });
    (scene, toggles)
}

fn project_root(application: &NativeApplication) -> String {
    application
        .services
        .state
        .projects
        .read()
        .values()
        .next()
        .unwrap()
        .root
        .clone()
}

fn claude_hooks_installed(root: &str) -> bool {
    taide_agent::service::has_taide_agent_hook_entries(
        "claude",
        &taide_agent::hook_files::read_settings_local(root).unwrap(),
    )
}

fn user_target(agent: &'static str) -> sections::HookTarget {
    sections::HookTarget {
        project: ProjectId(String::new()),
        agent,
    }
}

fn hooks_installed(application: &NativeApplication, agent: &'static str) -> Option<bool> {
    match sections_data(application).hooks_status(&user_target(agent)) {
        sections::Status::Ready(status) => Some(status.installed),
        sections::Status::Pending | sections::Status::Failed(_) => None,
    }
}

#[test]
fn 실제_hooks는_토글_뒤_임시_프로젝트와_가짜_home에_설치_해제하고_원본_오류를_알린다() {
    let home = std::env::temp_dir().join(format!("taide-settings-hooks-home-{}", ProjectId::new()));
    std::fs::create_dir_all(&home).unwrap();
    let (mut scene, toggles) = hooks_scene(&home);
    let toggle = scene.text("settings.agentHooks");
    let install = scene.text("agent.hooksInstall");
    let uninstall = scene.text("agent.hooksUninstall");
    let consent = scene.text("agent.hooksConsentTitle");
    let codex = scene.text("settings.agentHooksAgentCodex");
    let missing = scene.text("settings.agentHooksCliMissing");
    let claude = scene.text("settings.agentHooksAgentClaude");
    let output = scene.until("hooks toggle", |_, output| {
        !nodes(output, &toggle).is_empty()
    });
    assert!(
        !scene
            .application()
            .services
            .state
            .settings
            .read()
            .agent_hooks_enabled
    );
    assert!(texts_of(&output, &claude).is_empty());

    click_label(&mut scene, &toggle);
    let output = scene.until("hooks enabled", |application, output| {
        application
            .services
            .state
            .settings
            .read()
            .agent_hooks_enabled
            && !texts_of(output, &claude).is_empty()
            && !nodes(output, &install).is_empty()
            && hooks_installed(application, "codex").is_some()
            && hooks_installed(application, "gemini").is_some()
    });
    let claude_label = texts_of(&output, &claude)[0];
    let project_name = texts_of(&output, "sections")
        .into_iter()
        .find(|rect| rect.top() > claude_label.top())
        .expect("project row below the Claude Code label");
    assert!(project_name.top() < nodes(&output, &install)[0].bottom());
    assert_eq!(*toggles.hooks.lock().unwrap(), vec![(false, true)]);
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(written["agentHooksEnabled"], serde_json::json!(true));
    assert_eq!(texts_of(&output, &missing).len(), 1);

    let root = project_root(scene.application());
    assert!(!claude_hooks_installed(&root));
    let row = nodes(&output, &install)[0];
    scene.click(row.center());
    let output = scene.until("consent", |_, output| {
        !texts_of(output, &consent).is_empty()
    });
    let confirm = *nodes(&output, &install)
        .iter()
        .find(|node| **node != row)
        .expect("dialog confirm");
    scene.click(confirm.center());
    scene.until("claude installed", |_, output| {
        texts_of(output, &consent).is_empty() && !nodes(output, &uninstall).is_empty()
    });
    assert!(claude_hooks_installed(&root));
    click_label(&mut scene, &uninstall);
    scene.until("claude uninstalled", |_, output| {
        !nodes(output, &install).is_empty()
    });
    assert!(!claude_hooks_installed(&root));

    assert_eq!(hooks_installed(scene.application(), "codex"), Some(false));
    click_label(&mut scene, &codex);
    scene.until("codex installed", |application, _| {
        hooks_installed(application, "codex") == Some(true)
    });
    let codex_file = taide_agent::service::user_level_hooks_path("codex", home.to_str()).unwrap();
    assert!(codex_file.starts_with(&home));
    assert!(taide_agent::service::has_taide_marker_anywhere(
        &taide_agent::hook_files::read_user_level_hooks(&codex_file).unwrap()
    ));
    click_label(&mut scene, &codex);
    scene.until("codex uninstalled", |application, _| {
        hooks_installed(application, "codex") == Some(false)
    });
    assert!(!taide_agent::service::has_taide_marker_anywhere(
        &taide_agent::hook_files::read_user_level_hooks(&codex_file).unwrap()
    ));

    let cli_missing = taide_native_ui::toast::describe_error(
        &scene.application().locale,
        &AppError::InvalidArgument("taide CLI is not installed yet".into()),
    );
    scene
        .application()
        .settings_state
        .settings_views
        .sections_data_mut()
        .change_hooks(user_target("gemini"), sections::HookChange::Install);
    scene.until("gemini refused", |application, _| {
        errors(application).contains(&cli_missing)
    });
    assert_eq!(hooks_installed(scene.application(), "gemini"), Some(false));

    click_label(&mut scene, &toggle);
    scene.until("hooks disabled", |application, output| {
        !application
            .services
            .state
            .settings
            .read()
            .agent_hooks_enabled
            && texts_of(output, &claude).is_empty()
    });
    assert_eq!(
        *toggles.hooks.lock().unwrap(),
        vec![(false, true), (true, false)]
    );
    let disabled = taide_native_ui::toast::describe_error(
        &scene.application().locale,
        &AppError::InvalidArgument("agent hooks are disabled in settings".into()),
    );
    let claude_target = sections::HookTarget {
        project: scene
            .application()
            .services
            .state
            .projects
            .read()
            .keys()
            .next()
            .unwrap()
            .clone(),
        agent: "claude",
    };
    scene
        .application()
        .settings_state
        .settings_views
        .sections_data_mut()
        .change_hooks(claude_target, sections::HookChange::Install);
    scene.until("disabled refused", |application, _| {
        errors(application).contains(&disabled)
    });
    assert!(!claude_hooks_installed(&root));
    assert!(toggles.ide.lock().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn 실제_ide_스위치는_settings_json과_재조정_포트에_한번_반영된다() {
    let home = std::env::temp_dir().join(format!("taide-settings-ide-home-{}", ProjectId::new()));
    let (mut scene, toggles) = hooks_scene(&home);
    let ide = scene.text("settings.ideIntegration");
    let hint = scene.text("settings.ideIntegrationHint");
    let output = scene.until("ide switch", |_, output| !nodes(output, &ide).is_empty());
    assert_eq!(texts_of(&output, &hint).len(), 1);
    let before = scene
        .application()
        .services
        .state
        .settings
        .read()
        .ide_integration_enabled;
    click_label(&mut scene, &ide);
    scene.until("ide toggled", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .ide_integration_enabled
            != before
    });
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(written["ideIntegrationEnabled"], serde_json::json!(!before));
    scene.step(Vec::new());
    assert_eq!(*toggles.ide.lock().unwrap(), vec![(before, !before)]);
    assert!(toggles.hooks.lock().unwrap().is_empty());
}

#[test]
fn 실제_알림_버튼은_가짜_설정열기와_시험알림_억제_사유를_원본_문구로_알린다() {
    let mut scene = Scene::launch(None, |_| {});
    let opens = Arc::new(AtomicUsize::new(0));
    let opened = opens.clone();
    scene.ports(Ports {
        open_notification_settings: Arc::new(move || {
            if opened.fetch_add(1, Ordering::SeqCst) == 0 {
                return Ok(());
            }
            Err(AppError::Internal("fixture opener refused".into()))
        }),
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let open = scene.text("settings.notificationsOpenSystemSettings");
    let test = scene.text("settings.notificationsSendTest");
    scene.until("notification buttons", |_, output| {
        !nodes(output, &open).is_empty() && !nodes(output, &test).is_empty()
    });
    click_label(&mut scene, &open);
    scene.until("opened", |_, _| opens.load(Ordering::SeqCst) == 1);
    click_label(&mut scene, &open);
    scene.until("open failed", |application, _| {
        errors(application).contains(&"fixture opener refused".to_string())
    });
    assert_eq!(opens.load(Ordering::SeqCst), 2);

    let context = scene.context().clone();
    assert!(scene.application().is_any_window_focused(&context));
    assert!(
        scene
            .application()
            .services
            .state
            .settings
            .read()
            .notifications_only_when_unfocused,
        "a focused window must suppress the test notification before any platform call"
    );
    let focused = scene.text("settings.notificationsOnlyWhenUnfocused");
    click_label(&mut scene, &test);
    scene.until("focused", |application, _| {
        warnings(application).contains(&focused)
    });

    scene
        .application()
        .services
        .state
        .settings
        .write()
        .notifications_enabled = false;
    let master = scene.text("settings.notificationsEnabled");
    click_label(&mut scene, &test);
    scene.until("master off", |application, _| {
        warnings(application).contains(&master)
    });

    {
        let application = scene.application();
        let mut settings = application.services.state.settings.write();
        settings.notifications_enabled = true;
        settings.notify_error = false;
    }
    let category = scene.text("settings.notificationsError");
    click_label(&mut scene, &test);
    scene.until("category off", |application, _| {
        warnings(application).contains(&category)
    });
    assert!(successes(scene.application()).is_empty());
    assert!(
        errors(scene.application())
            .iter()
            .all(|title| title == "fixture opener refused")
    );
}

#[test]
fn 실제_performance는_주입_레지스트리가_켜지면_마지막_section으로_읽기_초기화를_반영한다() {
    let mut scene = Scene::launch(None, |_| {});
    scene.ports(Ports {
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let title = scene.text("settings.performance");
    let read = scene.text("settings.performanceRead");
    let reset = scene.text("settings.performanceReset");
    let native = scene.text("settings.performanceNative");
    let remote = scene.text("remote.title");
    let output = scene.until("settings", |_, output| texts_of(output, &remote).len() == 2);
    assert!(texts_of(&output, &title).is_empty());
    assert!(nodes(&output, &read).is_empty());
    let registry: &'static PerfRegistry = Box::leak(Box::new(PerfRegistry::new()));
    registry.set_enabled(true);
    let slot = SpanSlot::ALL[0];
    registry.record_span(slot, SPAN_NANOS);
    scene.ports(Ports {
        perf: registry,
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let output = scene.until("performance", |_, output| {
        texts_of(output, &title).len() == 2
    });
    let toc = texts_of(&output, &title)[0];
    assert!(texts_of(&output, &remote)[0].top() < toc.top());
    assert!(texts_of(&output, &native).is_empty());
    click_label(&mut scene, &read);
    let entry = format!("\"name\": \"{}\",\n      \"count\": 1,", slot.name());
    let output = scene.until("read", |_, output| {
        painted(output)
            .iter()
            .any(|(text, _)| text.contains(&entry))
    });
    assert_eq!(texts_of(&output, &native).len(), 1);
    click_label(&mut scene, &reset);
    scene.until("reset", |_, output| {
        texts_of(output, &native).is_empty()
            && !painted(output)
                .iter()
                .any(|(text, _)| text.contains(&entry))
    });
    assert_eq!(registry.span_sample(slot).count, 0);
}

#[derive(Default)]
struct SettingsPlatform {
    sent: Mutex<Vec<(String, String)>>,
    refusal: Mutex<Option<String>>,
}

impl SettingsPlatform {
    fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().unwrap().clone()
    }
}

impl taide_runtime::PlatformServices for SettingsPlatform {
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
        if let Some(reason) = self.refusal.lock().unwrap().clone() {
            return Err(AppError::Internal(reason));
        }
        self.sent
            .lock()
            .unwrap()
            .push((title.to_owned(), body.to_owned()));
        Ok(())
    }
}

fn settle(scene: &mut Scene) {
    for _ in 0..NOTIFY_SETTLE_FRAMES {
        scene.step(Vec::new());
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(unix)]
#[test]
fn 실제_시험_알림은_창_포커스가_없으면_가짜_플랫폼에_한번_보내고_첫전달_안내없이_성공을_알리며_플랫폼_오류를_알린다()
 {
    let platform = Arc::new(SettingsPlatform::default());
    let mut scene = Scene::launch_on_platform(platform.clone(), |_| {});
    scene.ports(Ports {
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let test = scene.text("settings.notificationsSendTest");
    let hint = scene.text("notification.enableHint");
    scene.until("test button", |_, output| !nodes(output, &test).is_empty());
    scene.set_focused(false);
    scene.step(Vec::new());
    scene.step(Vec::new());
    let context = scene.context().clone();
    assert!(!scene.application().is_any_window_focused(&context));
    click_label(&mut scene, &test);
    scene.until("delivered", |application, _| {
        successes(application).contains(&hint)
    });
    assert_eq!(platform.sent(), vec![(test.clone(), hint.clone())]);
    assert!(warnings(scene.application()).is_empty());
    assert_eq!(
        scene
            .application()
            .toasts
            .inspection()
            .into_iter()
            .filter(|toast| toast.kind == Kind::Info && toast.title == hint)
            .count(),
        0,
        "the test notification must not raise the first delivery hint"
    );
    *platform.refusal.lock().unwrap() = Some(REFUSED.into());
    click_label(&mut scene, &test);
    scene.until("refused", |application, _| {
        errors(application).contains(&REFUSED.to_string())
    });
    assert_eq!(platform.sent().len(), 1);
}

fn agent_event(project: &ProjectId, activity: AgentActivity) -> AppEvent {
    AppEvent::AgentStateChanged {
        project_id: project.clone(),
        agents: vec![DetectedAgent {
            session_id: NOTIFY_SESSION.into(),
            name: "claude".into(),
            pid: 1,
            activity,
            blocked_reason: None,
        }],
    }
}

fn agent_cycle(
    scene: &mut Scene,
    context: &egui::Context,
    project: &ProjectId,
    start: Duration,
) -> Duration {
    let next = start + NOTIFY_AGENT_WORKED;
    let application = scene.application();
    application.shell_state.provider_events.take();
    application
        .shell_state
        .provider_events
        .record(&agent_event(project, AgentActivity::Working));
    application.consume_provider_events(context, Instant::now() + start);
    application
        .shell_state
        .provider_events
        .record(&agent_event(project, AgentActivity::AwaitingInput));
    application.consume_provider_events(context, Instant::now() + next);
    next
}

#[cfg(unix)]
#[test]
fn 실제_os_알림은_범주별_설정_스위치를_settings_경로로_끄면_그_범주만_보내지_않는다() {
    let platform = Arc::new(SettingsPlatform::default());
    let mut scene = Scene::launch_on_platform(platform.clone(), |_| {});
    scene.ports(Ports {
        agents: missing_cli_agents(),
        ..Default::default()
    });
    scene.set_focused(false);
    scene.step(Vec::new());
    scene.step(Vec::new());
    let context = scene.context().clone();
    assert!(!scene.application().is_any_window_focused(&context));
    let events = scene.application().services.events.clone();
    let project = scene
        .application()
        .services
        .state
        .session
        .read()
        .active_project
        .clone()
        .unwrap();
    let command = |exit_code: i32| AppEvent::TerminalCommandFinished {
        session_id: NOTIFY_SESSION.into(),
        cwd: Some("/synthetic/cwd".into()),
        exit_code: Some(exit_code),
        duration_ms: LONG_COMMAND_MS,
    };
    let lsp = |phase: LspInstallPhase| AppEvent::LspInstallProgress {
        server_id: taide_model::lsp::LspServerId("synthetic-server".into()),
        phase,
        received_bytes: 0.0,
        total_bytes: None,
        message: None,
    };

    update(
        &mut scene,
        Change::Switch(Switch::NotificationsEnabled, false),
    );
    scene.until("master off", |application, _| {
        !application
            .services
            .state
            .settings
            .read()
            .notifications_enabled
    });
    events.publish(command(0));
    events.publish(command(1));
    events.publish(lsp(LspInstallPhase::Done));
    agent_cycle(&mut scene, &context, &project, Duration::ZERO);
    settle(&mut scene);
    assert!(platform.sent().is_empty(), "master off sends nothing");

    update(
        &mut scene,
        Change::Switch(Switch::NotificationsEnabled, true),
    );
    scene.until("master on", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .notifications_enabled
    });

    events.publish(command(0));
    scene.until("task completed", |_, _| platform.sent().len() == 1);
    update(
        &mut scene,
        Change::Switch(Switch::NotifyTaskCompleted, false),
    );
    scene.until("task completed off", |application, _| {
        !application
            .services
            .state
            .settings
            .read()
            .notify_task_completed
    });
    events.publish(command(0));
    settle(&mut scene);
    assert_eq!(platform.sent().len(), 1);

    events.publish(command(1));
    scene.until("error sent", |_, _| platform.sent().len() == 2);
    update(&mut scene, Change::Switch(Switch::NotifyError, false));
    scene.until("error off", |application, _| {
        !application.services.state.settings.read().notify_error
    });
    events.publish(command(1));
    settle(&mut scene);
    assert_eq!(platform.sent().len(), 2);

    events.publish(lsp(LspInstallPhase::Done));
    scene.until("lsp sent", |_, _| platform.sent().len() == 3);
    update(&mut scene, Change::Switch(Switch::NotifyLspInstall, false));
    scene.until("lsp off", |application, _| {
        !application
            .services
            .state
            .settings
            .read()
            .notify_lsp_install
    });
    events.publish(lsp(LspInstallPhase::Done));
    settle(&mut scene);
    assert_eq!(platform.sent().len(), 3);

    let mut worked = Duration::ZERO;
    worked = agent_cycle(&mut scene, &context, &project, worked);
    scene.until("agent awaiting input sent", |_, _| {
        platform.sent().len() == 4
    });
    update(
        &mut scene,
        Change::Switch(Switch::NotifyAgentAwaitingInput, false),
    );
    scene.until("awaiting input off", |application, _| {
        !application
            .services
            .state
            .settings
            .read()
            .notify_agent_awaiting_input
    });
    worked = agent_cycle(&mut scene, &context, &project, worked);
    settle(&mut scene);
    assert_eq!(platform.sent().len(), 4);

    update(
        &mut scene,
        Change::Switch(Switch::NotifyAgentAwaitingInput, true),
    );
    scene.until("awaiting input on", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .notify_agent_awaiting_input
    });
    update(
        &mut scene,
        Change::Switch(Switch::NotifyAgentCompleted, false),
    );
    scene.until("agent completed off", |application, _| {
        !application
            .services
            .state
            .settings
            .read()
            .notify_agent_completed
    });
    let application = scene.application();
    application.shell_state.provider_events.take();
    application
        .shell_state
        .provider_events
        .record(&agent_event(&project, AgentActivity::Working));
    application.consume_provider_events(&context, Instant::now() + worked);
    application
        .shell_state
        .provider_events
        .record(&agent_event(&project, AgentActivity::Idle));
    application.consume_provider_events(&context, Instant::now() + worked + NOTIFY_AGENT_WORKED);
    settle(&mut scene);
    assert_eq!(platform.sent().len(), 4);
}

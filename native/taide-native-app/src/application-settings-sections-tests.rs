use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::ai::AiProviderId;
use taide_model::error::AppErrorKind;
use taide_model::ids::TabId;
use taide_model::layout::PaneNode;
use taide_model::lsp::{LspInstallPhase, LspInstallStrategy, LspServerDetection};
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::toast::Kind;
use taide_runtime::{AppState, TaskSupervisor};

use super::*;
use crate::application::project_rail_tests::{
    Fixture, insert_project, isolated_state, pointer, project_ref,
};
use crate::host::HostBridge;

const SCREEN: egui::Vec2 = egui::vec2(1400.0, 9000.0);
const BLANK: egui::Pos2 = egui::pos2(1390.0, 8990.0);
const DEADLINE: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(5);
const MODELS: &str = r#"{"data":[{"id":"alpha"},{"id":"beta"}]}"#;

pub(super) struct Scene {
    fixture: Fixture,
    context: egui::Context,
    started: Instant,
    screen: egui::Vec2,
}

impl Scene {
    pub(super) fn launch(
        secrets: Option<SecretStoreState>,
        configure: impl FnOnce(&AppState),
    ) -> Self {
        Self::launch_with(secrets, configure, true)
    }

    /// Launches the Settings scene; an unattached project is skipped by the startup watcher
    /// restore, which also never reconciles agent hooks against the real home directory.
    pub(super) fn launch_with(
        secrets: Option<SecretStoreState>,
        configure: impl FnOnce(&AppState),
        is_attached: bool,
    ) -> Self {
        Self::assemble(secrets, configure, is_attached, true)
    }

    /// Launches the same project with an empty editor pane instead of the Settings tab.
    pub(super) fn launch_empty_pane(configure: impl FnOnce(&AppState)) -> Self {
        Self::assemble(None, configure, true, false)
    }

    fn assemble(
        secrets: Option<SecretStoreState>,
        configure: impl FnOnce(&AppState),
        is_attached: bool,
        has_settings_tab: bool,
    ) -> Self {
        let mut fixture = Fixture {
            directory: std::env::temp_dir()
                .join(format!("taide-settings-sections-{}", ProjectId::new())),
            application: None,
        };
        let data = fixture.directory.join("data");
        let root = fixture.directory.join("project");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        let state = isolated_state(data);
        configure(&state);
        let project = ProjectId::new();
        insert_project(&state, &project, &root, "sections", !is_attached);
        {
            let mut layouts = state.layouts.write();
            let layout = layouts.get_mut(&project).unwrap();
            let PaneNode::Leaf { id, tabs, active } = &mut layout.root else {
                panic!("default layout is one pane");
            };
            layout.focused_pane = id.clone();
            if !has_settings_tab {
                tabs.clear();
                *active = None;
            }
        }
        if has_settings_tab {
            let mut layouts = state.layouts.write();
            let layout = layouts.get_mut(&project).unwrap();
            let PaneNode::Leaf { id, tabs, active } = &mut layout.root else {
                panic!("default layout is one pane");
            };
            let tab = TabId::new();
            tabs.push(Tab {
                id: tab.clone(),
                kind: TabKind::Settings,
                title: "Settings".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            });
            *active = Some(tab);
            layout.focused_pane = id.clone();
        }
        {
            let slot = taide_model::ids::ShellSlotId::new();
            let mut session = state.session.write();
            session.projects = vec![project_ref(&project, &root, "sections")];
            session.active_project = Some(project.clone());
            session.focused_shell_slot = Some(slot.clone());
            session.shell_slots = Some(taide_model::project::ShellSlotTree::Leaf {
                slot_id: slot,
                project_id: project.clone(),
            });
        }
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let tasks = TaskSupervisor::new(runtime.handle().clone());
        let context = egui::Context::default();
        context.enable_accesskit();
        fixture.application = Some(
            NativeApplication::new_with_secret_store(
                &eframe::CreationContext::_new_kittest(context.clone()),
                runtime,
                state,
                tasks,
                fixture.directory.join("bin/taide"),
                Vec::new(),
                Some(secrets.unwrap_or_else(|| {
                    SecretStoreState::with_store(Arc::new(Keychain::default()))
                })),
            )
            .unwrap(),
        );
        Self {
            fixture,
            context,
            started: Instant::now(),
            screen: SCREEN,
        }
    }

    pub(super) fn resize(&mut self, screen: egui::Vec2) {
        self.screen = screen;
    }

    pub(super) fn application(&mut self) -> &mut NativeApplication {
        self.fixture.application.as_mut().unwrap()
    }

    pub(super) fn context(&self) -> &egui::Context {
        &self.context
    }

    pub(super) fn ports(&mut self, ports: crate::settings_ports::Ports) {
        self.application().settings_state.sections_ports = Arc::new(ports);
    }

    pub(super) fn text(&mut self, key: &str) -> String {
        presentation::message(&self.application().locale, key, &[])
    }

    pub(super) fn step(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let time = self.started.elapsed().as_secs_f64();
        let context = self.context.clone();
        let application = self.fixture.application.as_mut().unwrap();
        eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
        );
        output.textures_delta.clear();
        output
    }

    pub(super) fn until(
        &mut self,
        phase: &str,
        ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
    ) -> egui::FullOutput {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let output = self.step(Vec::new());
            if ready(self.fixture.application.as_ref().unwrap(), &output) {
                return output;
            }
            assert!(
                Instant::now() < deadline,
                "phase={phase}, painted={:?}",
                painted(&output)
                    .into_iter()
                    .map(|(text, rect)| (text, rect.top()))
                    .collect::<Vec<_>>()
            );
            std::thread::sleep(POLL);
        }
    }

    pub(super) fn click(&mut self, position: egui::Pos2) {
        self.step(pointer(position, egui::PointerButton::Primary));
        self.step(Vec::new());
    }

    pub(super) fn type_into(&mut self, position: egui::Pos2, text: &str) {
        self.click(position);
        self.step(vec![egui::Event::Text(text.into())]);
        self.step(Vec::new());
    }

    pub(super) fn keys(&mut self, keys: &[egui::Key]) {
        for key in keys {
            self.step(vec![egui::Event::Key {
                key: *key,
                physical_key: Some(*key),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        self.step(Vec::new());
    }
}

pub(super) fn painted(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
    let mut pending = output
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    let mut texts = Vec::new();
    while let Some(shape) = pending.pop() {
        match shape {
            egui::Shape::Vec(shapes) => pending.extend(shapes),
            egui::Shape::Text(text) => texts.push((
                text.galley.job.text.clone(),
                text.galley.rect.translate(text.pos.to_vec2()),
            )),
            _ => (),
        }
    }
    texts.sort_by(|left, right| left.1.top().total_cmp(&right.1.top()));
    texts
}

pub(super) fn texts_of(output: &egui::FullOutput, value: &str) -> Vec<egui::Rect> {
    painted(output)
        .into_iter()
        .filter(|(text, _)| text == value)
        .map(|(_, rect)| rect)
        .collect()
}

pub(super) fn nodes(output: &egui::FullOutput, label: &str) -> Vec<egui::Rect> {
    let mut rects = output
        .platform_output
        .accesskit_update
        .as_ref()
        .map(|update| {
            update
                .nodes
                .iter()
                .filter(|(_, node)| node.label() == Some(label))
                .filter_map(|(_, node)| node.bounds())
                .map(|bounds| {
                    egui::Rect::from_min_max(
                        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rects.sort_by(|left, right| left.top().total_cmp(&right.top()));
    rects
}

pub(super) fn errors(application: &NativeApplication) -> Vec<String> {
    application
        .toasts
        .inspection()
        .into_iter()
        .filter(|toast| toast.kind == Kind::Error)
        .map(|toast| toast.title)
        .collect()
}

pub(super) fn successes(application: &NativeApplication) -> Vec<String> {
    application
        .toasts
        .inspection()
        .into_iter()
        .filter(|toast| toast.kind == Kind::Success)
        .map(|toast| toast.title)
        .collect()
}

/// Agent ports whose `taide` shell command is absent, so the Interface row never reads the real
/// `/usr/local/bin/taide`.
pub(super) fn missing_cli_agents() -> Arc<crate::remote_agents::Ports> {
    let mut agents = crate::remote_agents::Ports::new();
    agents.cli_status = Arc::new(|| {
        taide_runtime::agent_host::cli_install_status(std::path::Path::new(
            "/nonexistent/taide-settings-fixture/taide",
        ))
    });
    Arc::new(agents)
}

fn detection(id: &str, strategy: LspInstallStrategy) -> LspServerDetection {
    LspServerDetection {
        id: id.into(),
        name: format!("{id} fixture"),
        language_ids: Vec::new(),
        resolved_path: None,
        available: false,
        install_hint: None,
        install_strategy: strategy,
        installed_version: None,
        toolchain_available: None,
        sdk_available: None,
        toolchain_tool: None,
        download_available: Some(true),
        initialization_options: None,
    }
}

#[test]
fn 실제_lsp_section은_임시_데이터와_빈_path로_manifest_서버를_감지해_그린다() {
    let mut scene = Scene::launch(None, |_| {});
    scene.ports(crate::settings_ports::Ports {
        detect_lsp_servers: Arc::new(|lsp_dir| {
            taide_lsp::service::detect_servers(lsp_dir, std::ffi::OsStr::new(""))
        }),
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let manifest = taide_lsp::manifest::servers();
    let downloads = manifest
        .iter()
        .filter(|spec| spec.install.strategy == LspInstallStrategy::Download)
        .count();
    let installed = scene.text("settings.lspInstalled");
    let missing = scene.text("settings.lspNotInstalled");
    let output = scene.until("lsp rows", |_, output| {
        painted(output)
            .iter()
            .filter(|(text, _)| *text == missing || text.starts_with(&installed))
            .count()
            == manifest.len()
    });
    assert!(texts_of(&output, &missing).len() >= downloads);
    for spec in &manifest {
        assert_eq!(texts_of(&output, &spec.name).len(), 1, "{}", spec.id);
    }
}

#[test]
fn 실제_lsp_설치는_진행과_재감지와_실패_toast를_원본대로_보이고_명령을_복사한다() {
    let mut scene = Scene::launch(None, |_| {});
    let detections = Arc::new(AtomicUsize::new(0));
    let (release, receiver) = tokio::sync::mpsc::unbounded_channel::<AppResult<()>>();
    let receiver = Arc::new(tokio::sync::Mutex::new(receiver));
    let counted = detections.clone();
    scene.ports(crate::settings_ports::Ports {
        detect_lsp_servers: Arc::new(move |_| {
            let installed = counted.fetch_add(1, Ordering::SeqCst) > 0;
            vec![
                LspServerDetection {
                    available: installed,
                    ..detection("expert", LspInstallStrategy::Download)
                },
                LspServerDetection {
                    install_hint: Some("zig fetch zls".into()),
                    ..detection("zls", LspInstallStrategy::Download)
                },
            ]
        }),
        lsp_install: Arc::new(move |services, server| {
            let receiver = receiver.clone();
            Box::pin(async move {
                services.events.publish(AppEvent::LspInstallProgress {
                    server_id: server.clone(),
                    phase: LspInstallPhase::Downloading,
                    received_bytes: 512.0,
                    total_bytes: Some(2048.0),
                    message: None,
                });
                let result = receiver.lock().await.recv().await.unwrap();
                if result.is_ok() {
                    services.events.publish(AppEvent::LspInstallProgress {
                        server_id: server,
                        phase: LspInstallPhase::Done,
                        received_bytes: 2048.0,
                        total_bytes: Some(2048.0),
                        message: None,
                    });
                }
                result
            })
        }),
        agents: missing_cli_agents(),
        ..Default::default()
    });
    let install = scene.text("settings.lspInstall");
    let cancel = scene.text("settings.lspCancel");
    let installing = scene.text("settings.lspInstalling");
    let output = scene.until("install buttons", |_, output| {
        nodes(output, &install).len() == 2
    });
    scene.click(nodes(&output, &install)[0].center());
    let output = scene.until("download progress", |_, output| {
        !texts_of(output, "512B / 2.0KB").is_empty()
    });
    assert_eq!(nodes(&output, &cancel).len(), 1);
    assert_eq!(texts_of(&output, &installing).len(), 1);
    release.send(Ok(())).unwrap();
    let installed = scene.text("settings.lspInstalled");
    scene.until("redetected", |_, output| {
        !texts_of(output, &installed).is_empty() && texts_of(output, &installing).is_empty()
    });
    assert_eq!(detections.load(Ordering::SeqCst), 2);

    let output = scene.until("zls install", |_, output| {
        nodes(output, &install).len() == 1
    });
    scene.click(nodes(&output, &install)[0].center());
    scene.until("zls progress", |_, output| {
        !texts_of(output, "512B / 2.0KB").is_empty()
    });
    release
        .send(Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.installCancelled",
            "cancelled",
        )))
        .unwrap();
    scene.until("cancelled", |application, _| {
        application
            .settings_state
            .settings_views
            .sections_data()
            .lsp_progress(&"zls".into())
            .is_none()
    });
    assert!(errors(scene.application()).is_empty());

    let output = scene.until("zls retry", |_, output| nodes(output, &install).len() == 1);
    scene.click(nodes(&output, &install)[0].center());
    scene.until("zls retry progress", |_, output| {
        !texts_of(output, "512B / 2.0KB").is_empty()
    });
    release
        .send(Err(AppError::Internal("fixture install failed".into())))
        .unwrap();
    scene.until("failed toast", |application, _| {
        errors(application) == vec!["fixture install failed".to_string()]
            && application
                .settings_state
                .settings_views
                .sections_data()
                .lsp_progress(&"zls".into())
                .is_none()
    });

    let ports = Arc::new(crate::settings_ports::Ports::default());
    assert!(
        scene
            .application()
            .submit(HostCommand::Settings(Command::Sections {
                request: sections::Request::Act(sections::Action::LspInstall("vtsls".into())),
                ports,
            }))
    );
    let locale = scene.application().locale.clone();
    let sdk_only = taide_native_ui::toast::describe_error(
        &locale,
        &AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.sdkDetectOnly",
            "vtsls: SDK-detect-only servers cannot be installed automatically",
        )
        .with_arg("serverId", "vtsls"),
    );
    scene.until("sdk detect only", |application, _| {
        errors(application).contains(&sdk_only)
    });

    let writes = Arc::new(Mutex::new(Vec::<String>::new()));
    let refuse = Arc::new(AtomicBool::new(false));
    {
        let writer = writes.clone();
        let refused = refuse.clone();
        let repaint = scene.context.clone();
        let application = scene.application();
        let bridge = HostBridge::connect_with_settings_ports(
            application.services.clone(),
            Arc::new(move || repaint.request_repaint()),
            Arc::new(move |text| {
                if refused.load(Ordering::SeqCst) {
                    return Err(AppError::Internal("fixture clipboard refused".into()));
                }
                writer.lock().unwrap().push(text.into());
                Ok(())
            }),
            Arc::new(|| Ok(String::new())),
            Some(crate::host::Terminals {
                tabs: application.terminal_agent_state.terminals.clone(),
                environment: crate::terminal_environment::provider(),
            }),
            application.application_ports.reconcile.clone(),
        )
        .unwrap();
        let old = application.bridge.replace(bridge).unwrap();
        application.runtime.block_on(old.disconnect()).unwrap();
    }
    let copy = scene.text("settings.lspCopyCommand");
    let copied = scene.text("settings.lspCommandCopied");
    let copy_failed = scene.text("settings.lspCommandCopyFailed");
    let output = scene.until("copy button", |_, output| nodes(output, &copy).len() == 1);
    scene.click(nodes(&output, &copy)[0].center());
    scene.until("copied", |application, _| {
        successes(application).contains(&copied)
    });
    assert_eq!(*writes.lock().unwrap(), vec!["zig fetch zls".to_string()]);
    refuse.store(true, Ordering::SeqCst);
    let output = scene.step(Vec::new());
    scene.click(nodes(&output, &copy)[0].center());
    scene.until("copy failed", |application, _| {
        errors(application).contains(&copy_failed)
    });
}

#[derive(Default)]
pub(super) struct Keychain {
    pub(super) values: Mutex<HashMap<&'static str, String>>,
    pub(super) refuse: AtomicBool,
}

fn account_key(account: SecretAccount) -> &'static str {
    match account {
        SecretAccount::AiOllamaCloud => "ollamaCloud",
        SecretAccount::AiCodex => "codex",
        SecretAccount::AiOmlx => "omlx",
        SecretAccount::GithubSync => "github",
        SecretAccount::RemoteAccess => "remote",
    }
}

impl SecretStore for Keychain {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
        if self.refuse.load(Ordering::SeqCst) {
            return Err(AppError::Internal("fixture keychain refused".into()));
        }
        self.values
            .lock()
            .unwrap()
            .insert(account_key(account), value.into());
        Ok(())
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        Ok(self
            .values
            .lock()
            .unwrap()
            .get(account_key(account))
            .cloned())
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        self.values.lock().unwrap().remove(account_key(account));
        Ok(())
    }
}

#[test]
fn 실제_ai_토큰은_가짜_keychain에_저장_해제되고_실패는_원본_문구로_알린다() {
    let keychain = Arc::new(Keychain::default());
    let mut scene = Scene::launch(Some(SecretStoreState::with_store(keychain.clone())), |_| {});
    let not_set = scene.text("settings.aiTokenNotSet");
    let saved = scene.text("settings.aiTokenSaved");
    let placeholder = scene.text("settings.aiTokenPlaceholder");
    let save = scene.text("settings.aiTokenSave");
    let clear = scene.text("settings.aiTokenClear");
    let output = scene.until("token rows", |_, output| {
        texts_of(output, &not_set).len() == 2
    });
    let input = texts_of(&output, &placeholder)[0];
    scene.type_into(input.center(), "  fixture-token  ");
    let output = scene.step(Vec::new());
    scene.click(nodes(&output, &save)[0].center());
    let output = scene.until("saved", |_, output| texts_of(output, &saved).len() == 1);
    assert_eq!(
        keychain.values.lock().unwrap().get("ollamaCloud").cloned(),
        Some("fixture-token".into())
    );
    assert_eq!(texts_of(&output, &placeholder).len(), 2);
    let clears = nodes(&output, &clear);
    assert_eq!(clears.len(), 2);
    scene.click(clears[0].center());
    scene.until("cleared", |_, output| texts_of(output, &not_set).len() == 2);
    assert!(keychain.values.lock().unwrap().is_empty());

    keychain.refuse.store(true, Ordering::SeqCst);
    let output = scene.step(Vec::new());
    let input = texts_of(&output, &placeholder)[0];
    scene.type_into(input.center(), "second-token");
    let output = scene.step(Vec::new());
    scene.click(nodes(&output, &save)[0].center());
    scene.until("refused", |application, _| {
        errors(application) == vec!["fixture keychain refused".to_string()]
    });
    assert!(keychain.values.lock().unwrap().is_empty());
}

async fn models(
    axum::extract::State(hits): axum::extract::State<Arc<AtomicUsize>>,
) -> (
    [(axum::http::header::HeaderName, &'static str); 1],
    &'static str,
) {
    hits.fetch_add(1, Ordering::SeqCst);
    (
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        MODELS,
    )
}

#[test]
fn 실제_omlx_base_url과_모델_선택과_auto_tab은_settings_json에_반영된다() {
    let mut scene = Scene::launch(None, |state| {
        state.settings.write().ai_provider = Some(AiProviderId::Omlx);
    });
    let hits = Arc::new(AtomicUsize::new(0));
    let url = {
        let served = hits.clone();
        scene.application().runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let router = axum::Router::new()
                .route("/v1/models", axum::routing::get(models))
                .with_state(served);
            tokio::spawn(async move { axum::serve(listener, router).await });
            url
        })
    };
    let url_placeholder = scene.text("settings.aiOmlxBaseUrlPlaceholder");
    let required = scene.text("settings.aiAutoTabProviderRequired");
    let model_placeholder = scene.text("settings.aiModelSelectPlaceholder");
    let output = scene.until("omlx row", |_, output| {
        !texts_of(output, &url_placeholder).is_empty() && !texts_of(output, &required).is_empty()
    });
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    scene.type_into(
        texts_of(&output, &url_placeholder)[0].center(),
        &format!("  {url}  "),
    );
    scene.click(BLANK);
    let output = scene.until("models", |application, output| {
        application
            .services
            .state
            .settings
            .read()
            .ai_omlx_base_url
            .as_deref()
            == Some(&url)
            && !texts_of(output, &model_placeholder).is_empty()
    });
    let written =
        std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap();
    assert!(written.contains(&url));
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert!(texts_of(&output, &required).is_empty());

    let model_label = scene.text("settings.aiModelLabel");
    let picker = *nodes(&output, &model_label)
        .iter()
        .max_by(|left, right| left.left().total_cmp(&right.left()))
        .unwrap();
    scene.click(picker.center());
    scene.keys(&[egui::Key::ArrowDown, egui::Key::Enter]);
    scene.until("model selected", |application, _| {
        let settings = application.services.state.settings.read();
        settings.ai_model.as_deref() == Some("beta")
            && settings.ai_provider == Some(AiProviderId::Omlx)
    });

    let toggle = scene.text("settings.aiAutoTabToggle");
    let output = scene.step(Vec::new());
    scene.click(nodes(&output, &toggle)[0].center());
    scene.until("auto tab", |application, _| {
        let settings = application.services.state.settings.read();
        settings.ai_auto_tab_enabled && settings.ai_provider == Some(AiProviderId::Omlx)
    });
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(written["aiAutoTabEnabled"], serde_json::json!(true));
    assert_eq!(written["aiProvider"], serde_json::json!("omlx"));
    assert_eq!(written["aiModel"], serde_json::json!("beta"));

    let provider_label = scene.text("settings.aiProviderLabel");
    let output = scene.step(Vec::new());
    let picker = *nodes(&output, &provider_label)
        .iter()
        .max_by(|left, right| left.left().total_cmp(&right.left()))
        .unwrap();
    scene.click(picker.center());
    scene.keys(&[egui::Key::Enter]);
    scene.until("provider changed", |application, _| {
        let settings = application.services.state.settings.read();
        settings.ai_provider == Some(AiProviderId::OllamaCloud) && settings.ai_model.is_none()
    });
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn 매_프레임_실패는_같은_문구를_한_번만_기록하고_회복_뒤_다시_기록한다() {
    let mut last = None;
    let failed = || Some("native theme preview failed: broken".to_owned());
    assert_eq!(unreported_failure(&mut last, failed()), failed());
    assert_eq!(unreported_failure(&mut last, failed()), None);
    assert_eq!(
        unreported_failure(&mut last, Some("other".into())),
        Some("other".into())
    );
    assert_eq!(unreported_failure(&mut last, None), None);
    assert_eq!(last, None);
    assert_eq!(unreported_failure(&mut last, failed()), failed());
}

const AUTO_TAB_FILE: &str = "auto-tab.txt";
const AUTO_TAB_SOURCE: &str = "let value = ";
const AUTO_TAB_MODEL: &str = "fixture";
const REFUSED_OMLX_URL: &str = "http://127.0.0.1:9";
const SETTLE_FRAMES: usize = 4;

fn focus_editor_end(scene: &mut Scene, view: taide_native_editor::view::ViewId) {
    let deadline = Instant::now() + DEADLINE;
    let body = loop {
        scene.step(Vec::new());
        if let Some(body) = crate::editor_context_menu::body(scene.context(), view) {
            break body;
        }
        assert!(Instant::now() < deadline, "editor body is painted");
        std::thread::sleep(POLL);
    };
    scene
        .context()
        .memory_mut(|memory| memory.request_focus(body));
    scene.step(crate::application::inlay_hints_tests::key(
        egui::Key::End,
        egui::Modifiers::COMMAND,
    ));
}

#[test]
fn 실제_auto_tab_토글은_편집기_자동완성_요청_경로를_켠다() {
    let mut scene = Scene::launch(None, |state| {
        let mut settings = state.settings.write();
        settings.ai_provider = Some(AiProviderId::Omlx);
        settings.ai_omlx_base_url = Some(REFUSED_OMLX_URL.into());
        settings.ai_model = Some(AUTO_TAB_MODEL.into());
    });
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    scene.application().editor_completion.inline =
        crate::editor_ai_completion::State::with_complete(Arc::new(move |_, request, _| {
            recorded.lock().unwrap().push(request.clone());
            Box::pin(async move {
                Ok(taide_model::ai::AiTextResponse {
                    request_id: request.request_id,
                    text: None,
                })
            })
        }));
    let project = scene
        .application()
        .services
        .state
        .session
        .read()
        .active_project
        .clone()
        .unwrap();
    let settings_tab = {
        let layouts = scene.application().services.state.layouts.read();
        let PaneNode::Leaf { active, .. } = &layouts[&project].root else {
            panic!("default layout is one pane");
        };
        active.clone().unwrap()
    };
    let path = scene.fixture.directory.join("project").join(AUTO_TAB_FILE);
    std::fs::write(&path, AUTO_TAB_SOURCE).unwrap();
    let path = path.to_str().unwrap().to_owned();
    assert!(scene.application().submit(HostCommand::Editor(
        crate::domains::editor::Command::OpenFileTab {
            project: project.clone(),
            pane: None,
            path: path.clone(),
            preview: false,
        }
    )));
    scene.until("file view", |application, _| {
        application.files.get(&path).is_some_and(|document| {
            application
                .store
                .views()
                .for_document(document.id)
                .next()
                .is_some()
        })
    });
    let (view, document, file_tab) = {
        let application = scene.application();
        let document = application.files[&path].id;
        let view = application
            .store
            .views()
            .for_document(document)
            .next()
            .unwrap();
        (view.id, document, view.key.tab.clone())
    };
    let text = |scene: &mut Scene| {
        scene
            .application()
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
    };
    focus_editor_end(&mut scene, view);
    scene.step(vec![egui::Event::Text(";".into())]);
    for _ in 0..SETTLE_FRAMES {
        scene.step(Vec::new());
    }
    assert_eq!(text(&mut scene), format!("{AUTO_TAB_SOURCE};"));
    assert!(requests.lock().unwrap().is_empty());

    scene
        .application()
        .controller
        .submit(ShellMutation::ActivateTab(settings_tab))
        .unwrap();
    let toggle = scene.text("settings.aiAutoTabToggle");
    let output = scene.until("auto tab toggle enabled", |_, output| {
        output
            .platform_output
            .accesskit_update
            .as_ref()
            .is_some_and(|update| {
                update
                    .nodes
                    .iter()
                    .any(|(_, node)| node.label() == Some(toggle.as_str()) && !node.is_disabled())
            })
    });
    scene.click(nodes(&output, &toggle)[0].center());
    scene.until("auto tab on", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .ai_auto_tab_enabled
    });

    scene
        .application()
        .controller
        .submit(ShellMutation::ActivateTab(file_tab))
        .unwrap();
    focus_editor_end(&mut scene, view);
    scene.step(vec![egui::Event::Text(";".into())]);
    scene.until("inline completion requested", |_, _| {
        !requests.lock().unwrap().is_empty()
    });
    let request = requests.lock().unwrap().last().unwrap().clone();
    assert_eq!(request.prefix, format!("{AUTO_TAB_SOURCE};;"));
    assert_eq!(request.provider, AiProviderId::Omlx);
    assert_eq!(request.model, AUTO_TAB_MODEL);
    assert_eq!(
        std::fs::canonicalize(&request.file_path).unwrap(),
        std::fs::canonicalize(&path).unwrap()
    );
}

use super::super::{Appearance as SettingsAppearance, Output, Owner, Trace, Views};
use super::*;
use crate::settings_sections_data::{Reply, Request};
use taide_model::ai::{AiModelInfo, AiTokenStatus};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::lsp::LspServerId;
use taide_model::paths::AppPaths;
use taide_model::remote::RemoteStatus;
use taide_model::sync::{SyncDownloadResult, SyncStatus};
use taide_runtime::AppState;

const SCREEN: [f32; 2] = [1280.0, 8000.0];
const FRAME_TIME: f64 = 0.1;
const OUTSIDE: egui::Pos2 = egui::pos2(2.0, 2.0);
const RATIO_TOLERANCE: f32 = 0.01;
const DOWNLOAD_RATIO: f32 = 0.25;
const UNFORMATTED_TIME: &str = "unformatted";

fn formatted_time(at: &str) -> Option<String> {
    (at != UNFORMATTED_TIME).then(|| format!("local {at}"))
}

struct Scene {
    directory: std::path::PathBuf,
    context: egui::Context,
    views: Views,
    owner: Owner,
    settings: Settings,
    locale: ResolvedLocale,
    appearance: SettingsAppearance,
    time: f64,
    projects: Vec<ProjectRef>,
    is_macos: bool,
    is_dev_build: bool,
    is_perf_enabled: bool,
    screen: egui::Vec2,
}

impl Scene {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-settings-sections-{}", ProjectId::new()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        let theme = taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
        let locale =
            taide_runtime::locale_actions::locale_get_for_language(&state, "en", "en").unwrap();
        Self {
            directory,
            context: egui::Context::default(),
            views: Views::default(),
            owner: Owner {
                project: ProjectId::new(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            settings: Settings::default(),
            locale,
            appearance: SettingsAppearance::new(&theme).unwrap(),
            time: 0.0,
            projects: Vec::new(),
            is_macos: false,
            is_dev_build: false,
            is_perf_enabled: false,
            screen: egui::vec2(SCREEN[0], SCREEN[1]),
        }
    }

    fn draw(&mut self, events: Vec<egui::Event>, hosted: bool) -> (Output, egui::FullOutput) {
        self.time += FRAME_TIME;
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
            events,
            time: Some(self.time),
            ..Default::default()
        };
        let mut output = Output::default();
        self.views.begin_frame();
        let mut drawing = self.context.run_ui(input, |ui| {
            output = if hosted {
                self.views.show_with_host(
                    ui,
                    self.owner.clone(),
                    &self.settings,
                    &self.locale,
                    &self.appearance,
                    Host {
                        format_time: formatted_time,
                        projects: &self.projects,
                        is_macos: self.is_macos,
                        is_dev_build: self.is_dev_build,
                        is_perf_enabled: self.is_perf_enabled,
                    },
                )
            } else {
                self.views.show(
                    ui,
                    self.owner.clone(),
                    &self.settings,
                    &self.locale,
                    &self.appearance,
                )
            };
        });
        self.views.finish_frame();
        drawing.textures_delta.clear();
        (output, drawing)
    }

    fn frame(&mut self, events: Vec<egui::Event>) -> (Output, egui::FullOutput) {
        self.draw(events, true)
    }

    fn data(&mut self) -> &mut Data {
        self.views.sections_data_mut()
    }

    fn text(&self, key: &str) -> String {
        message(&self.locale, key, &[])
    }

    fn state(&self) -> &OwnerState {
        &self.views.entries[&self.owner].host
    }

    fn answer(&mut self, mut reply: impl FnMut(Query, u64) -> Option<Reply>) -> Vec<Action> {
        let mut actions = Vec::new();
        for request in self.data().take_requests() {
            match request {
                Request::Fetch { query, generation } => {
                    if let Some(reply) = reply(query, generation) {
                        self.data().accept(reply, web_time::Instant::now());
                    }
                }
                Request::Act(action) => actions.push(action),
            }
        }
        actions
    }

    fn servers(&mut self, servers: Vec<LspServerDetection>) {
        self.frame(Vec::new());
        self.answer(|query, generation| {
            (query == Query::LspServers).then(|| Reply::LspServers {
                generation,
                result: Ok(servers.clone()),
            })
        });
    }

    fn tokens(&mut self, status: AiTokenStatus) {
        self.frame(Vec::new());
        self.answer(|query, generation| {
            (query == Query::AiTokens).then(|| Reply::AiTokens {
                generation,
                result: Ok(status.clone()),
            })
        });
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn painted(drawing: &egui::FullOutput) -> Vec<(String, Color32)> {
    let mut pending = drawing
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
                text.galley
                    .job
                    .sections
                    .first()
                    .map(|section| section.format.color)
                    .unwrap_or(text.fallback_color),
            )),
            _ => (),
        }
    }
    texts
}

fn count(drawing: &egui::FullOutput, value: &str) -> usize {
    painted(drawing)
        .iter()
        .filter(|(text, _)| text == value)
        .count()
}

fn color_of(drawing: &egui::FullOutput, value: &str) -> Color32 {
    painted(drawing)
        .into_iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("text not painted: {value}"))
        .1
}

fn traces<'a>(output: &'a Output, field: &str) -> Vec<&'a Trace> {
    output
        .traces
        .iter()
        .filter(|trace| trace.field == field)
        .collect()
}

fn click(position: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn key(key: egui::Key) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

fn server(id: &str, strategy: LspInstallStrategy) -> LspServerDetection {
    LspServerDetection {
        id: id.into(),
        name: format!("{id} server"),
        language_ids: Vec::new(),
        resolved_path: None,
        available: false,
        install_hint: None,
        install_strategy: strategy,
        installed_version: None,
        toolchain_available: None,
        sdk_available: None,
        toolchain_tool: None,
        download_available: None,
        initialization_options: None,
    }
}

fn expert() -> LspServerDetection {
    LspServerDetection {
        download_available: Some(true),
        install_hint: Some("mix archive.install expert".into()),
        ..server("expert", LspInstallStrategy::Download)
    }
}

fn gopls() -> LspServerDetection {
    LspServerDetection {
        toolchain_available: Some(true),
        toolchain_tool: Some("go".into()),
        ..server("gopls", LspInstallStrategy::Toolchain)
    }
}

fn progress(phase: LspInstallPhase, received: f64, total: Option<f64>) -> Progress {
    Progress {
        phase,
        received_bytes: received,
        total_bytes: total,
        message: None,
    }
}

fn tokens(ollama_cloud: bool, codex: bool, omlx: bool) -> AiTokenStatus {
    AiTokenStatus {
        ollama_cloud,
        codex,
        omlx,
    }
}

#[test]
fn 바이트_형식은_원본_경계를_따른다() {
    assert_eq!(format_bytes(0.0), "0B");
    assert_eq!(format_bytes(1023.0), "1023B");
    assert_eq!(format_bytes(1024.0), "1.0KB");
    assert_eq!(format_bytes(1_048_575.0), "1024.0KB");
    assert_eq!(format_bytes(1_048_576.0), "1.0MB");
    assert_eq!(format_bytes(1280.0), "1.3KB");
    assert_eq!(format_bytes(1152.0), "1.1KB");
    assert_eq!(format_bytes(1_310_720.0), "1.3MB");
}

#[test]
fn 미러는_기본_8개_section만_그리고_호스트는_lsp_ai_plugins를_목차와_카드_끝에_붙인다() {
    let mut scene = Scene::new();
    let (mirror, mirror_drawing) = scene.draw(Vec::new(), false);
    assert!(traces(&mirror, Section::Lsp.title()).is_empty());
    assert!(traces(&mirror, Section::Ai.title()).is_empty());
    assert!(traces(&mirror, Section::Plugins.title()).is_empty());
    assert!(traces(&mirror, Section::Sync.title()).is_empty());
    assert!(traces(&mirror, Section::Remote.title()).is_empty());
    assert_eq!(
        count(&mirror_drawing, &scene.text("settings.lspDescription")),
        0
    );
    assert_eq!(count(&mirror_drawing, &scene.text("remote.description")), 0);
    assert!(scene.data().take_requests().is_empty());
    let (hosted, drawing) = scene.frame(Vec::new());
    let keymap = traces(&hosted, Section::Keymap.title())[0].rect;
    let lsp = traces(&hosted, Section::Lsp.title())[0].rect;
    let ai = traces(&hosted, Section::Ai.title())[0].rect;
    let plugins = traces(&hosted, Section::Plugins.title())[0].rect;
    let sync = traces(&hosted, Section::Sync.title())[0].rect;
    let remote = traces(&hosted, Section::Remote.title())[0].rect;
    assert!(keymap.top() < lsp.top() && lsp.top() < ai.top() && ai.top() < plugins.top());
    assert!(plugins.top() < sync.top() && sync.top() < remote.top());
    assert_eq!(count(&drawing, &scene.text("settings.lspDescription")), 1);
    assert_eq!(count(&drawing, &scene.text("remote.description")), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.loading")),
        count(&mirror_drawing, &scene.text("settings.loading")) + 2
    );
    let fetched = scene
        .data()
        .take_requests()
        .into_iter()
        .filter_map(|request| match request {
            Request::Fetch { query, .. } => Some(query),
            Request::Act(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        fetched,
        vec![
            Query::CliStatus,
            Query::LspServers,
            Query::AiTokens,
            Query::Plugins,
            Query::SyncStatus,
            Query::RemoteStatus
        ]
    );
}

#[test]
fn lsp_목록은_설치상태_버전_경로_배지와_힌트_3종을_원본_분기대로_그린다() {
    let mut scene = Scene::new();
    scene.servers(vec![
        LspServerDetection {
            available: true,
            installed_version: Some("1.2.3".into()),
            resolved_path: Some("/opt/bin/rust-analyzer".into()),
            install_hint: Some("hidden when installed".into()),
            ..server("rust-analyzer", LspInstallStrategy::Download)
        },
        expert(),
        server("zls", LspInstallStrategy::Download),
        gopls(),
        LspServerDetection {
            toolchain_available: Some(false),
            ..server("ruby-lsp", LspInstallStrategy::Toolchain)
        },
        LspServerDetection {
            install_hint: Some("xcode-select --install".into()),
            ..server("sourcekit-lsp", LspInstallStrategy::SdkDetect)
        },
    ]);
    let (output, drawing) = scene.frame(Vec::new());
    let installed = format!("{} (v1.2.3)", scene.text("settings.lspInstalled"));
    assert_eq!(
        color_of(&drawing, &installed),
        scene.appearance.sections.agent_running
    );
    assert_eq!(
        color_of(&drawing, "/opt/bin/rust-analyzer"),
        scene.appearance.muted
    );
    assert_eq!(count(&drawing, "hidden when installed"), 0);
    assert_eq!(count(&drawing, &scene.text("settings.lspExperimental")), 1);
    assert_eq!(count(&drawing, &scene.text("settings.lspNotInstalled")), 5);
    assert_eq!(
        color_of(&drawing, &scene.text("settings.lspNotInstalled")),
        scene.appearance.sections.badge
    );
    for key in [
        "settings.lspChecksumPending",
        "settings.lspToolchainMissing",
        "settings.lspSdkMissing",
    ] {
        assert_eq!(
            color_of(&drawing, &scene.text(key)),
            scene.appearance.sections.warning,
            "{key}"
        );
    }
    assert_eq!(count(&drawing, "xcode-select --install"), 1);
    assert_eq!(count(&drawing, "mix archive.install expert"), 1);
    assert_eq!(
        count(
            &drawing,
            &format!("go {}", scene.text("settings.lspInstall"))
        ),
        1
    );
    let installs = traces(&output, "settings.lspInstall");
    assert_eq!(
        installs
            .iter()
            .map(|trace| trace.enabled)
            .collect::<Vec<_>>(),
        vec![true, false, true]
    );
    assert_eq!(traces(&output, "settings.lspCopyCommand").len(), 1);
    assert!(traces(&output, "settings.lspCancel").is_empty());
}

#[test]
fn lsp_진행은_다운로드_막대와_검증_실패_완료를_나누고_취소와_재설치를_요청한다() {
    let mut scene = Scene::new();
    scene.servers(vec![expert(), gopls()]);
    let expert_id = LspServerId::from("expert");
    scene.data().record_progress(
        expert_id.clone(),
        progress(LspInstallPhase::Downloading, 512.0, Some(2048.0)),
    );
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("settings.lspInstalling")), 1);
    assert_eq!(count(&drawing, "512B / 2.0KB"), 1);
    assert_eq!(count(&drawing, "mix archive.install expert"), 0);
    assert_eq!(traces(&output, "settings.lspCancel").len(), 1);
    assert_eq!(traces(&output, "settings.lspInstall").len(), 1);
    let bars = drawing
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.rect.height() == PROGRESS_HEIGHT => Some(rect.rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(bars.len(), 2);
    assert!((bars[1].width() / bars[0].width() - DOWNLOAD_RATIO).abs() < RATIO_TOLERANCE);
    let cancel = traces(&output, "settings.lspCancel")[0].rect;
    scene.frame(click(cancel.center()));
    let actions = scene.answer(|_, _| None);
    assert!(matches!(&actions[..], [Action::LspCancel(server)] if *server == expert_id));

    scene.data().record_progress(
        expert_id.clone(),
        progress(LspInstallPhase::Verifying, 2048.0, Some(2048.0)),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("settings.lspInstalling")), 1);
    assert_eq!(count(&drawing, "2.0KB / 2.0KB"), 0);

    scene.data().record_progress(
        expert_id.clone(),
        Progress {
            message: Some("boom".into()),
            ..progress(LspInstallPhase::Failed, 0.0, None)
        },
    );
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(color_of(&drawing, "boom"), scene.appearance.sections.error);
    assert_eq!(count(&drawing, "mix archive.install expert"), 1);
    let installs = traces(&output, "settings.lspInstall");
    assert_eq!(installs.len(), 2);
    scene.frame(click(installs[0].rect.center()));
    let actions = scene.answer(|_, _| None);
    assert!(matches!(&actions[..], [Action::LspInstall(server)] if *server == expert_id));

    scene.data().record_progress(
        expert_id.clone(),
        progress(LspInstallPhase::Failed, 0.0, None),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("settings.lspInstallFailed")), 1);

    scene.data().record_progress(
        expert_id,
        progress(LspInstallPhase::Done, 2048.0, Some(2048.0)),
    );
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("settings.lspInstalling")), 0);
    assert_eq!(count(&drawing, &scene.text("settings.lspInstallFailed")), 0);
    assert_eq!(traces(&output, "settings.lspInstall").len(), 2);
}

#[test]
fn lsp_설치명령_복사는_미설치_비활성_sdk아님에서만_보이고_아래쪽_tooltip과_복사를_요청한다() {
    let mut scene = Scene::new();
    scene.servers(vec![expert()]);
    let (mut output, _) = scene.frame(Vec::new());
    let copy = traces(&output, "settings.lspCopyCommand")[0].rect;
    let tooltips = output.take_tooltips();
    let label = scene.text("settings.lspCopyCommand");
    let tooltip = tooltips
        .iter()
        .find(|trigger| trigger.label == label)
        .unwrap();
    assert_eq!(tooltip.align, egui::RectAlign::BOTTOM);
    scene.frame(click(copy.center()));
    let actions = scene.answer(|_, _| None);
    assert!(
        matches!(&actions[..], [Action::CopyInstallHint(text)] if text == "mix archive.install expert")
    );
    scene.data().record_progress(
        "expert".into(),
        progress(LspInstallPhase::Extracting, 0.0, None),
    );
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.lspCopyCommand").is_empty());
}

#[test]
fn ai_토큰_행은_상태_경고_저장중_비활성과_입력비움_해제를_원본대로_처리한다() {
    let mut scene = Scene::new();
    scene.tokens(tokens(true, false, false));
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text("settings.aiTokenSaved")),
        scene.appearance.sections.agent_running
    );
    assert_eq!(
        color_of(&drawing, &scene.text("settings.aiTokenNotSet")),
        scene.appearance.sections.badge
    );
    assert_eq!(
        color_of(&drawing, &scene.text("settings.aiCodexUnofficialWarning")),
        scene.appearance.sections.warning
    );
    assert_eq!(traces(&output, "settings.aiTokenClear").len(), 2);
    assert_eq!(
        traces(&output, "settings.aiTokenSave")
            .iter()
            .map(|trace| trace.enabled)
            .collect::<Vec<_>>(),
        vec![false, false, false]
    );
    let input = traces(&output, "settings.aiTokenPlaceholder")[0].rect;
    scene.frame(click(input.center()));
    scene.frame(vec![egui::Event::Text("  secret-token  ".into())]);
    let (output, _) = scene.frame(Vec::new());
    let save = traces(&output, "settings.aiTokenSave")[0];
    assert!(save.enabled);
    scene.frame(click(save.rect.center()));
    assert!(scene.state().ollama_cloud_token.is_empty());
    let actions = scene.answer(|_, _| None);
    assert!(matches!(
        &actions[..],
        [Action::AiSetToken {
            provider: AiProviderId::OllamaCloud,
            sequence: 1,
            ..
        }]
    ));
    assert!(!format!("{actions:?}").contains("secret-token"));
    scene.frame(click(input.center()));
    scene.frame(vec![egui::Event::Text("again".into())]);
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.aiTokenSave")[0].enabled);
    scene.data().accept(
        Reply::AiTokenSaved {
            provider: AiProviderId::OllamaCloud,
            sequence: 1,
            result: Ok(()),
        },
        web_time::Instant::now(),
    );
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.aiTokenSave")[0].enabled);
    let clear = traces(&output, "settings.aiTokenClear")[0].rect;
    scene.frame(click(clear.center()));
    let actions = scene.answer(|_, _| None);
    assert!(matches!(
        &actions[..],
        [Action::AiClearToken(AiProviderId::OllamaCloud)]
    ));
}

#[test]
fn ai_omlx_행은_base_url을_blur에서_trim해_패치하고_키를_저장한다() {
    let mut scene = Scene::new();
    scene.tokens(tokens(false, false, false));
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &scene.text("settings.aiOmlxBaseUrlLabel")),
        1
    );
    let url = traces(&output, "settings.aiOmlxBaseUrlLabel")[0].rect;
    let key_input = traces(&output, "settings.aiOmlxApiKeyOptional")[0].rect;
    scene.frame(click(url.center()));
    scene.frame(vec![egui::Event::Text(" http://127.0.0.1:9 ".into())]);
    let (blurred, _) = scene.frame(click(OUTSIDE));
    let (settled, _) = scene.frame(Vec::new());
    assert_eq!(
        blurred
            .changes
            .into_iter()
            .chain(settled.changes)
            .collect::<Vec<_>>(),
        vec![Change::Hosted(HostedChange::AiOmlxBaseUrl(
            "http://127.0.0.1:9".into()
        ))]
    );
    scene.frame(click(key_input.center()));
    scene.frame(vec![egui::Event::Text("omlx-key".into())]);
    let (output, _) = scene.frame(Vec::new());
    let save = traces(&output, "settings.aiTokenSave")[2];
    assert!(save.enabled);
    scene.frame(click(save.rect.center()));
    assert!(scene.state().omlx_key.is_empty());
    let actions = scene.answer(|_, _| None);
    assert!(matches!(
        &actions[..],
        [Action::AiSetToken {
            provider: AiProviderId::Omlx,
            ..
        }]
    ));
}

#[test]
fn ai_모델_영역은_미설정_조회중_오류_빈목록_목록과_placeholder를_나눠_그린다() {
    let mut scene = Scene::new();
    scene.settings.ai_provider = Some(AiProviderId::Omlx);
    scene.tokens(tokens(false, false, false));
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.aiModelLabel").is_empty());
    assert!(
        scene
            .answer(|query, _| matches!(query, Query::AiModels(_)).then(|| unreachable!()))
            .is_empty()
    );

    let mut scene = Scene::new();
    scene.settings.ai_provider = Some(AiProviderId::Omlx);
    scene.tokens(tokens(false, false, true));
    let (output, drawing) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.aiModelLabel").is_empty());
    assert_eq!(
        count(&drawing, &scene.text("settings.aiOmlxConnectFailed")),
        0
    );
    scene.answer(|query, generation| {
        (query == Query::AiModels(AiProviderId::Omlx)).then(|| Reply::AiModels {
            provider: AiProviderId::Omlx,
            generation,
            result: Err(AppError::Internal("refused".into())),
        })
    });
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text("settings.aiOmlxConnectFailed")),
        scene.appearance.sections.error
    );

    let mut scene = Scene::new();
    scene.settings.ai_provider = Some(AiProviderId::Codex);
    scene.tokens(tokens(false, true, false));
    scene.frame(Vec::new());
    scene.answer(|query, generation| {
        (query == Query::AiModels(AiProviderId::Codex)).then(|| Reply::AiModels {
            provider: AiProviderId::Codex,
            generation,
            result: Err(AppError::Internal("offline".into())),
        })
    });
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &scene.text("settings.aiModelLoadFailed")),
        1
    );

    let mut scene = Scene::new();
    scene.tokens(tokens(true, false, false));
    scene.frame(Vec::new());
    scene.answer(|query, generation| {
        (query == Query::AiModels(AiProviderId::OllamaCloud)).then(|| Reply::AiModels {
            provider: AiProviderId::OllamaCloud,
            generation,
            result: Ok(Vec::new()),
        })
    });
    let (output, drawing) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.aiModelLabel").is_empty());
    assert_eq!(
        color_of(&drawing, &scene.text("settings.aiModelSelectPlaceholder")),
        scene.appearance.muted
    );
}

#[test]
fn ai_provider와_모델_선택은_키보드로_패치를_만들고_빈_모델은_muted_placeholder다() {
    let mut scene = Scene::new();
    scene.tokens(tokens(true, false, false));
    scene.frame(Vec::new());
    scene.answer(|query, generation| {
        (query == Query::AiModels(AiProviderId::OllamaCloud)).then(|| Reply::AiModels {
            provider: AiProviderId::OllamaCloud,
            generation,
            result: Ok(vec![
                AiModelInfo {
                    model_id: "m1".into(),
                    display_name: None,
                },
                AiModelInfo {
                    model_id: "m2".into(),
                    display_name: Some("Model Two".into()),
                },
            ]),
        })
    });
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text("settings.aiModelSelectPlaceholder")),
        scene.appearance.muted
    );
    let model = traces(&output, "settings.aiModelLabel")[0].rect;
    scene.frame(click(model.center()));
    scene.frame(Vec::new());
    scene.frame(key(egui::Key::ArrowDown));
    let (output, _) = scene.frame(key(egui::Key::Enter));
    assert_eq!(
        output.changes,
        vec![Change::Hosted(HostedChange::AiModel {
            provider: AiProviderId::OllamaCloud,
            model: "m2".into(),
        })]
    );
    scene.settings.ai_model = Some("m2".into());
    for _ in 0..3 {
        scene.frame(Vec::new());
    }
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(color_of(&drawing, "Model Two"), scene.appearance.foreground);
    let provider = traces(&output, "settings.aiProviderLabel")[0].rect;
    scene.frame(click(provider.center()));
    scene.frame(Vec::new());
    scene.frame(key(egui::Key::ArrowDown));
    scene.frame(key(egui::Key::ArrowDown));
    let (output, _) = scene.frame(key(egui::Key::Enter));
    assert_eq!(
        output.changes,
        vec![Change::Hosted(HostedChange::AiProvider(AiProviderId::Omlx))]
    );
}

#[test]
fn ai_auto_tab은_선택_provider가_미설정이면_비활성_힌트이고_설정되면_패치한다() {
    let mut scene = Scene::new();
    scene.tokens(tokens(false, true, false));
    let (output, drawing) = scene.frame(Vec::new());
    let toggle = traces(&output, "settings.aiAutoTabToggle")[0];
    assert!(!toggle.enabled);
    assert_eq!(
        count(&drawing, &scene.text("settings.aiAutoTabProviderRequired")),
        1
    );
    let (output, _) = scene.frame(click(toggle.rect.center()));
    assert!(output.changes.is_empty());

    let mut scene = Scene::new();
    scene.tokens(tokens(true, false, false));
    let (output, drawing) = scene.frame(Vec::new());
    let toggle = traces(&output, "settings.aiAutoTabToggle")[0];
    assert!(toggle.enabled);
    assert_eq!(
        count(&drawing, &scene.text("settings.aiAutoTabToggleHint")),
        1
    );
    let (output, _) = scene.frame(click(toggle.rect.center()));
    assert_eq!(
        output.changes,
        vec![Change::Hosted(HostedChange::AiAutoTab {
            provider: AiProviderId::OllamaCloud,
            enabled: true,
        })]
    );
}

#[test]
fn section_상태_초기화는_입력을_비우고_다음_그리기를_새_mount로_만든다() {
    let mut scene = Scene::new();
    scene.tokens(tokens(false, false, false));
    let (output, _) = scene.frame(Vec::new());
    let input = traces(&output, "settings.aiTokenPlaceholder")[1].rect;
    scene.frame(click(input.center()));
    scene.frame(vec![egui::Event::Text("draft".into())]);
    assert_eq!(scene.state().codex_token, "draft");
    let view = scene.views.entries.get_mut(&scene.owner).unwrap();
    view.host.reset_section_state();
    assert!(view.host.codex_token.is_empty());
    assert!(view.host.mount());
    assert!(!view.host.mount());
}

fn plugin(id: &str, enabled: bool, error: Option<PluginErrorCode>) -> LoadedPlugin {
    LoadedPlugin {
        manifest: taide_model::plugin::PluginManifest {
            manifest_version: 1,
            id: id.into(),
            name: format!("{id} plugin"),
            version: "1.2.3".into(),
            contributes: Default::default(),
        },
        root: format!("/plugins/{id}"),
        enabled,
        error,
    }
}

impl Scene {
    fn plugins(&mut self, plugins: Vec<LoadedPlugin>) {
        self.frame(Vec::new());
        self.answer(|query, generation| {
            (query == Query::Plugins).then(|| Reply::Plugins {
                generation,
                result: Ok(plugins.clone()),
            })
        });
    }

    fn click_trace(&mut self, field: &str) -> (Output, egui::FullOutput) {
        let (output, _) = self.frame(Vec::new());
        let rect = traces(&output, field)
            .last()
            .unwrap_or_else(|| panic!("trace exists: {field}"))
            .rect;
        self.frame(click(rect.center()));
        self.frame(Vec::new())
    }
}

fn theme(id: &str, name: &str) -> taide_model::theme::Theme {
    taide_model::theme::Theme {
        version: 1,
        id: id.into(),
        name: name.into(),
        theme_type: ThemeType::Dark,
        extends: None,
        palette: Default::default(),
        colors: Default::default(),
        syntax: Default::default(),
        terminal: Default::default(),
        token_colors: Some(Vec::new()),
        author: None,
        license: None,
        source: None,
    }
}

fn candidate(
    index: usize,
    label: &str,
    id: &str,
    collides: bool,
    warnings: usize,
    failure: Option<VsixThemeImportFailure>,
) -> taide_model::vsix::VsixThemeCandidate {
    taide_model::vsix::VsixThemeCandidate {
        key: format!("{index}-{label}"),
        label: label.into(),
        id: id.into(),
        theme_type: ThemeType::Dark,
        id_collides: collides,
        warning_count: warnings,
        theme: failure.is_none().then(|| theme(id, label)),
        failure,
    }
}

const VSIX_WARNINGS: usize = 37;
const PLUGIN_ERROR_CODES: [PluginErrorCode; 7] = [
    PluginErrorCode::ParseFailed,
    PluginErrorCode::IdMismatch,
    PluginErrorCode::VersionMismatch,
    PluginErrorCode::PathEscape,
    PluginErrorCode::GrammarMissing,
    PluginErrorCode::GrammarInvalid,
    PluginErrorCode::GrammarConflict,
];

#[test]
fn plugins_목록은_조회중_빈상태_행과_오류코드_7종을_원본대로_그린다() {
    let mut scene = Scene::new();
    let (_, drawing) = scene.frame(Vec::new());
    let loading = count(&drawing, &scene.text("settings.loading"));
    scene.plugins(Vec::new());
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &scene.text("settings.loading")) + 1,
        loading
    );
    assert_eq!(count(&drawing, &scene.text("settings.pluginsEmpty")), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.pluginsManifestHint")),
        1
    );
    let mut plugins = vec![plugin("alpha", true, None)];
    plugins.extend(
        PLUGIN_ERROR_CODES
            .iter()
            .enumerate()
            .map(|(index, code)| plugin(&format!("broken{index}"), false, Some(*code))),
    );
    let mut scene = Scene::new();
    scene.plugins(plugins.clone());
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, "alpha plugin"), 1);
    assert_eq!(count(&drawing, "1.2.3"), plugins.len());
    assert_eq!(
        color_of(&drawing, &scene.text("settings.pluginEnabled")),
        scene.appearance.sections.agent_running
    );
    assert_eq!(
        count(&drawing, &scene.text("settings.pluginDisabled")),
        PLUGIN_ERROR_CODES.len()
    );
    for code in PLUGIN_ERROR_CODES {
        let text = scene.text(plugin_error_key(code));
        assert_eq!(count(&drawing, &text), 1, "{code:?}");
        assert_eq!(color_of(&drawing, &text), scene.appearance.sections.error);
    }
    assert_eq!(
        traces(&output, "settings.pluginUninstallButton").len(),
        plugins.len()
    );
    assert_eq!(count(&drawing, &scene.text("settings.pluginsEmpty")), 0);
}

#[test]
fn plugins_버튼은_파일선택_새로고침_회전_폴더를_요청하고_진행중_비활성이다() {
    let mut scene = Scene::new();
    scene.plugins(Vec::new());
    scene.click_trace("settings.pluginInstallButton");
    assert_eq!(scene.data().take_pick(), Some(PickPurpose::PluginArchive));
    scene.data().picked(PickPurpose::PluginArchive, None);
    assert!(scene.data().take_requests().is_empty());
    scene
        .data()
        .picked(PickPurpose::PluginArchive, Some("/tmp/plugin.zip".into()));
    assert!(matches!(
        scene.data().take_requests().as_slice(),
        [Request::Act(Action::PluginInstall(path))] if path == std::path::Path::new("/tmp/plugin.zip")
    ));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.pluginInstallButton")[0].enabled);
    scene
        .data()
        .accept(Reply::PluginInstalled(Ok(())), web_time::Instant::now());
    scene.click_trace("settings.pluginImportVsixButton");
    assert!(matches!(
        scene.data().take_pick(),
        Some(PickPurpose::Vsix(_))
    ));
    let (output, _) = scene.click_trace("settings.pluginsReload");
    assert!(!traces(&output, "settings.pluginsReload")[0].enabled);
    assert!(
        scene
            .data()
            .take_requests()
            .iter()
            .any(|request| matches!(request, Request::Act(Action::PluginReload)))
    );
    scene.data().accept(
        Reply::PluginsReloaded(Ok(vec![plugin("fresh", true, None)])),
        web_time::Instant::now(),
    );
    let (output, drawing) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.pluginsReload")[0].enabled);
    assert_eq!(count(&drawing, "fresh plugin"), 1);
    let (output, _) = scene.frame(Vec::new());
    let folder = traces(&output, "settings.pluginsOpenFolder")[0].rect;
    let (output, _) = scene.frame(click(folder.center()));
    assert_eq!(
        output.folders,
        vec![taide_model::system::AppDataPathKind::Plugins]
    );
}

#[test]
fn plugin_제거_대화상자는_escape와_바깥클릭을_구분하고_확인즉시_닫혀_제거를_요청한다() {
    let mut scene = Scene::new();
    scene.plugins(vec![plugin("alpha", true, None)]);
    let (output, drawing) = scene.click_trace("settings.pluginUninstallButton");
    let title = message(
        &scene.locale,
        "settings.pluginUninstallConfirmTitle",
        &[("name", "alpha plugin")],
    );
    assert_eq!(count(&drawing, &title), 1);
    assert_eq!(
        count(
            &drawing,
            &scene.text("settings.pluginUninstallConfirmDescription")
        ),
        1
    );
    assert!(!traces(&output, "settings.dialog.cancel").is_empty());
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.dialog.cancel")[0].has_focus);
    let confirm = traces(&output, "settings.dialog.confirm")[0].rect;
    let cancel = traces(&output, "settings.dialog.cancel")[0].rect;
    assert!((confirm.width() - cancel.width()).abs() < RATIO_TOLERANCE);
    let (output, _) = scene.frame(key(egui::Key::Tab));
    assert!(traces(&output, "settings.dialog.cancel")[0].has_focus);
    let (output, _) = scene.frame(key(egui::Key::Tab));
    assert!(traces(&output, "settings.dialog.confirm")[0].has_focus);
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.dialog.cancel")[0].has_focus);
    scene.frame(click(OUTSIDE));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 1);
    scene.frame(key(egui::Key::Escape));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 0);
    assert!(scene.data().take_requests().is_empty());
    let (output, _) = scene.frame(Vec::new());
    let trash = traces(&output, "settings.pluginUninstallButton")[0].id;
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(trash),
        "focus returns to the trash button that opened the dialog"
    );
    let (_, _) = scene.click_trace("settings.pluginUninstallButton");
    let (_, drawing) = scene.click_trace("settings.dialog.confirm");
    assert_eq!(count(&drawing, &title), 0);
    assert!(matches!(
        scene.data().take_requests().as_slice(),
        [Request::Act(Action::PluginUninstall { id })] if id == "alpha"
    ));
}

#[test]
fn ai_프롬프트_3행은_tooltip_연필로_지역화된_제목의_app_file을_연다() {
    let mut scene = Scene::new();
    let (output, drawing) = scene.frame(Vec::new());
    for key in [
        "prompts.autoTabTitle",
        "prompts.inlineEditTitle",
        "prompts.commitMessageTitle",
    ] {
        assert_eq!(count(&drawing, &scene.text(key)), 1, "{key}");
        assert_eq!(traces(&output, key).len(), 1, "{key}");
    }
    let edit = scene.text("prompts.editEntry");
    assert_eq!(
        output
            .tooltips
            .iter()
            .filter(|trigger| trigger.label == edit)
            .count(),
        PROMPTS.len()
    );
    let rect = traces(&output, "prompts.inlineEditTitle")[0].rect;
    let (output, _) = scene.frame(click(rect.center()));
    assert_eq!(
        output.open_app_file,
        Some((
            AppFileTarget::Prompt {
                id: PromptTemplateId::InlineEditDefault
            },
            scene.text("prompts.inlineEditTitle")
        ))
    );
}

fn open_vsix(
    scene: &mut Scene,
    candidates: Vec<taide_model::vsix::VsixThemeCandidate>,
    existing: &[&str],
) {
    scene.plugins(Vec::new());
    scene.click_trace("settings.pluginImportVsixButton");
    let purpose = scene.data().take_pick().unwrap();
    scene.data().picked(purpose, Some("/tmp/theme.vsix".into()));
    let ticket = match scene.data().take_requests().pop() {
        Some(Request::Act(Action::VsixExtract { ticket, .. })) => ticket,
        other => panic!("vsix extraction requested: {other:?}"),
    };
    scene.data().accept(
        Reply::VsixExtracted {
            ticket,
            result: Ok(VsixExtraction {
                path: "/tmp/theme.vsix".into(),
                extension: "Fixture Themes".into(),
                existing_ids: existing.iter().map(|id| (*id).to_owned()).collect(),
                candidates,
            }),
        },
        web_time::Instant::now(),
    );
    scene.frame(Vec::new());
}

#[test]
fn vsix_대화상자는_후보_실패_경고_중복과_빈_목록을_원본대로_그린다() {
    let mut scene = Scene::new();
    open_vsix(&mut scene, Vec::new(), &[]);
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, "Fixture Themes"), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.pluginImportVsixNoThemes")),
        1
    );
    let mut scene = Scene::new();
    open_vsix(
        &mut scene,
        vec![
            candidate(0, "Night", "fixture-night", false, VSIX_WARNINGS, None),
            candidate(1, "Day", "fixture-day", true, 0, None),
            candidate(
                2,
                "Broken",
                "fixture-broken",
                false,
                VSIX_WARNINGS,
                Some(VsixThemeImportFailure::Contrast),
            ),
        ],
        &["fixture-day"],
    );
    let (output, drawing) = scene.frame(Vec::new());
    let warnings = VSIX_WARNINGS.to_string();
    assert_eq!(count(&drawing, &warnings), 1);
    assert_eq!(
        color_of(&drawing, &warnings),
        scene.appearance.sections.warning
    );
    let duplicate = message(
        &scene.locale,
        "settings.themeImportDuplicate",
        &[("name", "fixture-day")],
    );
    assert_eq!(count(&drawing, &duplicate), 1);
    assert_eq!(
        color_of(
            &drawing,
            &scene.text("settings.themeImportThemeContrastFailure")
        ),
        scene.appearance.sections.error
    );
    assert!(traces(&output, "common.save")[0].enabled);
    assert!(
        traces(&output, "settings.dialog.close")[0].rect.top()
            > traces(&output, "common.save")[0].rect.top()
    );
    let selected = &scene.views.entries[&scene.owner]
        .host
        .vsix
        .as_ref()
        .unwrap()
        .selected;
    assert_eq!(
        selected.iter().cloned().collect::<Vec<_>>(),
        vec!["0-Night".to_owned(), "1-Day".to_owned()]
    );
}

#[test]
fn vsix_저장은_중복이면_확인_뒤_복사본_id와_이름으로_요청하고_문법은_3상태를_보인다() {
    let mut scene = Scene::new();
    open_vsix(
        &mut scene,
        vec![
            candidate(0, "Night", "fixture-night", false, 0, None),
            candidate(1, "Day", "fixture-day", true, 0, None),
        ],
        &["fixture-day"],
    );
    let (_, drawing) = scene.click_trace("common.save");
    let duplicate = message(
        &scene.locale,
        "settings.themeImportDuplicate",
        &[("name", "fixture-day")],
    );
    assert_eq!(count(&drawing, &duplicate), 2);
    assert!(scene.data().take_requests().is_empty());
    scene.click_trace("settings.dialog.confirm");
    let themes = match scene.data().take_requests().pop() {
        Some(Request::Act(Action::VsixSaveThemes {
            themes, extension, ..
        })) => {
            assert_eq!(extension, "Fixture Themes");
            themes
        }
        other => panic!("theme save requested: {other:?}"),
    };
    assert_eq!(themes[0].id, "fixture-night");
    assert_eq!(themes[0].name, "Night");
    assert_ne!(themes[1].id, "fixture-day");
    assert!(themes[1].id.starts_with("day"));
    assert_eq!(
        themes[1].name,
        message(
            &scene.locale,
            "themeEditor.duplicateNameTemplate",
            &[("name", "Day")]
        )
    );
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "common.save")[0].enabled);
    let grammars = traces(&output, "settings.pluginImportVsixGrammars")[0].rect;
    scene.frame(click(grammars.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.pluginImportVsixGrammars")[0].enabled);
    let ticket = match scene.data().take_requests().pop() {
        Some(Request::Act(Action::VsixImportGrammars { ticket, path })) => {
            assert_eq!(path, std::path::Path::new("/tmp/theme.vsix"));
            ticket
        }
        other => panic!("grammar import requested: {other:?}"),
    };
    let failure = AppError::Internal("already installed".into());
    assert!(
        scene
            .data()
            .accept(
                Reply::VsixGrammarsImported {
                    ticket,
                    result: Err(failure.clone()),
                },
                web_time::Instant::now(),
            )
            .is_none()
    );
    let (_, drawing) = scene.frame(Vec::new());
    let described = crate::toast::describe_error(&scene.locale, &failure);
    assert_eq!(
        color_of(&drawing, &described),
        scene.appearance.sections.error
    );
    scene
        .views
        .entries
        .get_mut(&scene.owner)
        .unwrap()
        .host
        .vsix
        .as_mut()
        .unwrap()
        .grammars = Grammars::Idle;
    scene.data().accept(
        Reply::VsixGrammarsImported {
            ticket,
            result: Ok(()),
        },
        web_time::Instant::now(),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text("settings.pluginImportVsixSuccess")),
        scene.appearance.sections.success
    );
}

#[test]
fn vsix_대화상자는_모서리_x_바깥클릭_escape로_닫힌다() {
    for close in ["corner", "outside", "escape"] {
        let mut scene = Scene::new();
        open_vsix(
            &mut scene,
            vec![candidate(0, "Night", "fixture-night", false, 0, None)],
            &[],
        );
        let (output, _) = scene.frame(Vec::new());
        let events = match close {
            "corner" => click(traces(&output, "settings.dialog.corner")[0].rect.center()),
            "outside" => click(OUTSIDE),
            _ => key(egui::Key::Escape),
        };
        scene.frame(events);
        let (_, drawing) = scene.frame(Vec::new());
        assert_eq!(count(&drawing, "Fixture Themes"), 0, "{close}");
        assert!(
            scene.views.entries[&scene.owner].host.vsix.is_none(),
            "{close}"
        );
    }
}

const SYNCED_AT: &str = "2026-10-28T05:05:09Z";
const PORT: u32 = 4100;
const CLIENTS: u32 = 3;

fn synced(remote_newer: Option<bool>, last_synced_at: Option<&str>) -> SyncStatus {
    SyncStatus {
        connected: true,
        has_gist: true,
        last_synced_at: last_synced_at.map(Into::into),
        remote_newer,
    }
}

fn remote_status(running: bool, password_configured: bool) -> RemoteStatus {
    RemoteStatus {
        running,
        port: PORT,
        client_count: CLIENTS,
        password_configured,
    }
}

impl Scene {
    fn statuses(&mut self, sync: Option<SyncStatus>, remote: Option<RemoteStatus>) {
        self.frame(Vec::new());
        self.answer(|_, _| None);
        let now = web_time::Instant::now();
        if let Some(sync) = sync {
            self.data().record_sync_status(sync, now);
        }
        if let Some(remote) = remote {
            self.data().record_remote_status(remote, now);
        }
    }

    fn acts(&mut self) -> Vec<Action> {
        self.answer(|_, _| None)
    }

    fn host_state(&mut self) -> &mut OwnerState {
        &mut self.views.entries.get_mut(&self.owner).unwrap().host
    }

    fn type_trace(&mut self, field: &str, text: &str) {
        let (output, _) = self.frame(Vec::new());
        let rect = traces(&output, field)
            .last()
            .unwrap_or_else(|| panic!("trace exists: {field}"))
            .rect;
        self.frame(click(rect.center()));
        self.frame(vec![egui::Event::Text(text.into())]);
    }

    fn trace(&mut self, field: &str) -> Seen {
        let (output, _) = self.frame(Vec::new());
        traces(&output, field)
            .last()
            .map(|trace| Seen {
                rect: trace.rect,
                enabled: trace.enabled,
            })
            .unwrap_or_else(|| panic!("trace exists: {field}"))
    }
}

struct Seen {
    rect: Rect,
    enabled: bool,
}

fn drag(from: egui::Pos2, to: egui::Pos2) -> [Vec<egui::Event>; 3] {
    [
        vec![
            egui::Event::PointerMoved(from),
            egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        vec![egui::Event::PointerMoved(to)],
        vec![egui::Event::PointerButton {
            pos: to,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    ]
}

fn copied(drawing: &egui::FullOutput) -> Vec<String> {
    drawing
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn sync_미연결_화면은_설명_선택가능_url과_빈값_연결중_비활성을_원본대로_따른다() {
    let mut scene = Scene::new();
    scene.statuses(None, None);
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("settings.syncDescription")), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.syncTokenScopeHint")),
        1
    );
    assert_eq!(
        color_of(&drawing, GITHUB_TOKEN_CREATE_URL),
        scene.appearance.foreground
    );
    assert!(!traces(&output, "settings.syncConnect")[0].enabled);
    assert!(traces(&output, "settings.syncUploadNow").is_empty());

    let url = traces(&output, "settings.syncTokenUrl")[0].rect;
    for events in drag(
        egui::pos2(url.left() + 1.0, url.center().y),
        egui::pos2(url.right() - 1.0, url.center().y),
    ) {
        scene.frame(events);
    }
    let (_, drawing) = scene.frame(vec![egui::Event::Copy]);
    assert_eq!(copied(&drawing), vec![GITHUB_TOKEN_CREATE_URL.to_string()]);

    scene.type_trace("settings.syncTokenPlaceholder", "   ");
    assert!(!scene.trace("settings.syncConnect").enabled);
    scene.host_state().sync_token = "  ghp_fixture  ".into();
    let connect = scene.trace("settings.syncConnect");
    assert!(connect.enabled);
    scene.frame(click(connect.rect.center()));
    scene.frame(Vec::new());
    assert!(scene.host_state().sync_token.is_empty());
    let actions = scene.acts();
    assert!(matches!(actions.as_slice(), [Action::SyncConnect(_)]));
    let [Action::SyncConnect(token)] = <[Action; 1]>::try_from(actions).unwrap() else {
        unreachable!()
    };
    assert_eq!(token.expose(), "ghp_fixture");
    scene.host_state().sync_token = "second".into();
    assert!(!scene.trace("settings.syncConnect").enabled, "connecting");
    scene.data().accept(
        Reply::SyncConnected(Ok(synced(None, None))),
        web_time::Instant::now(),
    );
    let (output, drawing) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.syncConnect").is_empty());
    assert_eq!(count(&drawing, &scene.text("settings.syncDescription")), 0);
    assert_eq!(count(&drawing, GITHUB_TOKEN_CREATE_URL), 0);
}

#[test]
fn sync_연결_화면은_배지_없음_문구_시간_경고와_업로드_다운로드_해제를_원본대로_그린다() {
    let mut scene = Scene::new();
    scene.statuses(Some(synced(Some(true), None)), None);
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("settings.syncSectionTitle")), 3);
    assert_eq!(
        color_of(&drawing, &scene.text("settings.syncRemoteNewerBadge")),
        scene.appearance.sections.warning
    );
    assert_eq!(count(&drawing, &scene.text("settings.syncGistIdNone")), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.syncLastSyncedNever")),
        1
    );
    assert_eq!(
        color_of(&drawing, &scene.text("settings.syncSecretGistWarning")),
        scene.appearance.sections.warning
    );
    let upload = traces(&output, "settings.syncUploadNow")[0];
    let download = traces(&output, "settings.syncDownloadNow")[0];
    let disconnect = traces(&output, "settings.syncDisconnect")[0];
    assert!(upload.rect.right() < download.rect.left());
    assert!(download.rect.right() < disconnect.rect.left());
    assert!(upload.enabled && download.enabled && disconnect.enabled);

    scene.settings.sync_gist_id = Some("gist-fixture".into());
    scene.data().record_sync_status(
        synced(Some(false), Some(SYNCED_AT)),
        web_time::Instant::now(),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &scene.text("settings.syncRemoteNewerBadge")),
        0
    );
    assert_eq!(count(&drawing, "gist-fixture"), 1);
    assert_eq!(count(&drawing, &format!("local {SYNCED_AT}")), 1);
    scene.data().record_sync_status(
        synced(None, Some(UNFORMATTED_TIME)),
        web_time::Instant::now(),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, UNFORMATTED_TIME), 1);

    scene.click_trace("settings.syncUploadNow");
    assert!(!scene.trace("settings.syncUploadNow").enabled);
    scene.click_trace("settings.syncDownloadNow");
    assert!(!scene.trace("settings.syncDownloadNow").enabled);
    scene.click_trace("settings.syncDisconnect");
    assert!(!scene.trace("settings.syncDisconnect").enabled);
    let owner = scene.state().container.owner.unwrap();
    assert!(matches!(
        scene.acts().as_slice(),
        [
            Action::SyncUpload,
            Action::SyncDownload { force: false, owner: target },
            Action::SyncDisconnect
        ] if *target == owner
    ));
}

#[test]
fn sync_충돌_대화상자는_버튼_순서_escape_트랩과_편집기_전환_뒤에도_열린_상태를_원본대로_따른다() {
    let mut scene = Scene::new();
    scene.statuses(Some(synced(None, Some(SYNCED_AT))), None);
    scene.click_trace("settings.syncDownloadNow");
    let owner = scene.state().container.owner.unwrap();
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::SyncDownload { force: false, owner: target }] if *target == owner
    ));
    let answer = |data: &mut Data| {
        data.accept(
            Reply::SyncDownloaded {
                force: false,
                owner,
                result: Ok(SyncDownloadResult::Conflict {
                    remote_updated_at: "2026-10-29T00:00:00Z".into(),
                }),
            },
            web_time::Instant::now(),
        )
    };
    let conflict = |data: &mut Data| {
        data.act(Action::SyncDownload {
            force: false,
            owner,
        });
        answer(data)
    };
    assert!(answer(scene.data()).is_none());
    assert!(!scene.data().is_busy(Busy::SyncDownload));
    let title = scene.text("settings.syncConflictTitle");
    scene.frame(Vec::new());
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.syncConflictDescription")),
        1
    );
    assert_eq!(count(&drawing, &scene.text("common.cancel")), 0);
    let keep = traces(&output, "settings.dialog.cancel")[0].rect;
    let pull = traces(&output, "settings.dialog.confirm")[0].rect;
    assert!(keep.right() < pull.left());
    let keep_text = scene.text("settings.syncConflictKeepLocal");
    let pull_text = scene.text("settings.syncConflictPullRemote");
    assert_eq!(count(&drawing, &keep_text), 1);
    assert_eq!(count(&drawing, &pull_text), 1);
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.dialog.cancel")[0].has_focus);
    let (output, _) = scene.frame(key(egui::Key::Tab));
    assert!(traces(&output, "settings.dialog.cancel")[0].has_focus);
    let (output, _) = scene.frame(key(egui::Key::Tab));
    assert!(traces(&output, "settings.dialog.confirm")[0].has_focus);
    let (output, _) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.dialog.cancel")[0].has_focus);
    scene.frame(click(OUTSIDE));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 1, "outside click is ignored");

    scene.host_state().reset_section_state();
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &title),
        1,
        "container state survives an editor round trip"
    );
    scene.frame(key(egui::Key::Escape));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 0);
    assert!(scene.acts().is_empty());
    let (output, _) = scene.frame(Vec::new());
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(traces(&output, "settings.syncDownloadNow")[0].id),
        "focus returns to the download button"
    );

    conflict(scene.data());
    scene.data().take_requests();
    let (_, drawing) = scene.click_trace("settings.dialog.cancel");
    assert_eq!(count(&drawing, &title), 0);
    assert!(matches!(scene.acts().as_slice(), [Action::SyncUpload]));

    conflict(scene.data());
    scene.data().take_requests();
    let (_, drawing) = scene.click_trace("settings.dialog.confirm");
    assert_eq!(count(&drawing, &title), 0);
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::SyncDownload { force: true, owner: target }] if *target == owner
    ));
}

#[test]
fn remote_토글_상태줄_경고_발급_url_폐기는_실행상태와_원본_순서를_따른다() {
    let mut scene = Scene::new();
    scene.statuses(None, Some(remote_status(false, false)));
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text("remote.enableToggleHint")), 1);
    assert_eq!(count(&drawing, &scene.text("remote.statusStopped")), 1);
    assert_eq!(
        color_of(&drawing, &scene.text("remote.securityWarning")),
        scene.appearance.sections.warning
    );
    let clients = message(&scene.locale, "remote.clientCountLabel", &[("count", "3")]);
    assert_eq!(count(&drawing, &clients), 0);
    assert!(!traces(&output, "remote.issueLink")[0].enabled);
    assert!(!traces(&output, "remote.revokeSessions")[0].enabled);
    let toggle = traces(&output, "remote.enableToggle")[0].rect;
    let password = traces(&output, "remote.passwordSet")[0].rect;
    let hosts = traces(&output, "remote.allowedHostsAdd")[0].rect;
    let issue = traces(&output, "remote.issueLink")[0].rect;
    assert!(toggle.top() < password.top() && password.top() < hosts.top());
    assert!(hosts.top() < issue.top());

    scene
        .data()
        .record_remote_status(remote_status(true, false), web_time::Instant::now());
    let (output, drawing) = scene.frame(Vec::new());
    let running = message(&scene.locale, "remote.statusRunning", &[("port", "4100")]);
    assert_eq!(count(&drawing, &running), 1);
    assert_eq!(color_of(&drawing, &clients), scene.appearance.muted);
    assert!(traces(&output, "remote.issueLink")[0].enabled);
    scene.click_trace("remote.issueLink");
    assert!(!scene.trace("remote.issueLink").enabled, "issuing");
    let owner = scene.state().container.owner.unwrap();
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::RemoteIssueLink { owner: target }] if *target == owner
    ));
    let url = "http://127.0.0.1:4100/?t=fixture";
    scene.data().accept(
        Reply::RemoteLinkIssued {
            owner,
            result: Ok((
                taide_model::remote::RemoteLinkInfo { url: url.into() },
                Ok(()),
            )),
        },
        web_time::Instant::now(),
    );
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(color_of(&drawing, url), scene.appearance.foreground);
    let line = traces(&output, "remote.issuedUrl")[0].rect;
    assert!(line.top() > traces(&output, "remote.allowedHostsAdd")[0].rect.top());
    assert!(line.top() < traces(&output, "remote.issueLink")[0].rect.top());

    scene.click_trace("remote.revokeSessions");
    assert!(!scene.trace("remote.revokeSessions").enabled, "revoking");
    assert!(matches!(scene.acts().as_slice(), [Action::RemoteRevoke]));

    scene.host_state().reset_section_state();
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, url), 1, "issued link is container state");
    scene.click_trace("remote.enableToggle");
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, url), 0);
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::UpdateSettings(HostedChange::RemoteAccess(true))]
    ));
}

#[test]
fn remote_비밀번호_행은_길이_경계_비활성과_해제와_password_only_노출을_원본대로_따른다() {
    let mut scene = Scene::new();
    scene.statuses(None, Some(remote_status(true, false)));
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text("remote.passwordNotConfigured")),
        scene.appearance.sections.badge
    );
    assert_eq!(
        color_of(&drawing, &scene.text("remote.passwordHint")),
        scene.appearance.sections.warning
    );
    assert!(traces(&output, "remote.passwordClear").is_empty());
    assert!(traces(&output, "remote.passwordOnlyToggle").is_empty());
    assert!(!traces(&output, "remote.passwordSet")[0].enabled);
    let too_short = message(&scene.locale, "remote.passwordTooShort", &[("min", "8")]);

    scene.type_trace("remote.passwordPlaceholder", "1");
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &too_short), 1);
    assert!(traces(&output, "remote.passwordPlaceholder")[0].has_focus);
    scene.frame(vec![egui::Event::Text("234567".into())]);
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &too_short), 1);
    assert!(!traces(&output, "remote.passwordSet")[0].enabled);
    scene.frame(vec![egui::Event::Text("8".into())]);
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(scene.host_state().remote_password, "12345678");
    assert_eq!(count(&drawing, &too_short), 0);
    assert!(traces(&output, "remote.passwordSet")[0].enabled);
    scene.host_state().remote_password = "  1234567  ".into();
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &too_short), 1);
    assert!(!traces(&output, "remote.passwordSet")[0].enabled);
    scene.host_state().remote_password = "가나다라마바사아".into();
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &too_short), 0);
    let set = traces(&output, "remote.passwordSet")[0];
    assert!(set.enabled);
    scene.frame(click(set.rect.center()));
    scene.frame(Vec::new());
    assert!(scene.host_state().remote_password.is_empty());
    let actions = scene.acts();
    let [Action::RemoteSetPassword(password)] = <[Action; 1]>::try_from(actions).unwrap() else {
        panic!("one password request")
    };
    assert_eq!(password.expose(), "가나다라마바사아");
    scene.host_state().remote_password = "12345678".into();
    assert!(!scene.trace("remote.passwordSet").enabled, "saving");
    scene
        .data()
        .accept(Reply::RemotePasswordSet(Ok(())), web_time::Instant::now());
    assert!(scene.trace("remote.passwordSet").enabled);

    scene.settings.remote_password_only_login = false;
    scene
        .data()
        .record_remote_status(remote_status(true, true), web_time::Instant::now());
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text("remote.passwordConfigured")),
        scene.appearance.sections.agent_running
    );
    assert_eq!(count(&drawing, &scene.text("remote.passwordHint")), 0);
    assert_eq!(
        count(
            &drawing,
            &scene.text("remote.passwordOnlyToggleDescription")
        ),
        1
    );
    let clear = traces(&output, "remote.passwordClear")[0].rect;
    assert!(clear.left() > traces(&output, "remote.passwordSet")[0].rect.right());
    scene.frame(click(clear.center()));
    scene.frame(Vec::new());
    scene.click_trace("remote.passwordOnlyToggle");
    assert!(matches!(
        scene.acts().as_slice(),
        [
            Action::RemoteClearPassword,
            Action::UpdateSettings(HostedChange::RemotePasswordOnly(true))
        ]
    ));
}

#[test]
fn remote_허용_호스트는_검증_중복_enter_조합중_enter_삭제와_저장중_비활성을_원본대로_따른다() {
    let mut scene = Scene::new();
    scene.settings.remote_allowed_hosts = vec!["tunnel.example.com".into()];
    scene.statuses(None, Some(remote_status(false, false)));
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &scene.text("remote.allowedHostsDescription")),
        1
    );
    assert_eq!(count(&drawing, "tunnel.example.com"), 1);
    assert!(traces(&output, "remote.allowedHostsRemove")[0].enabled);
    assert!(!traces(&output, "remote.allowedHostsAdd")[0].enabled);
    let invalid = scene.text("remote.allowedHostsInvalid");

    scene.host_state().remote_host = "https://bad.example.com".into();
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &invalid),
        scene.appearance.sections.warning
    );
    assert!(!traces(&output, "remote.allowedHostsAdd")[0].enabled);
    scene.host_state().remote_host = " TUNNEL.example.com ".into();
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &invalid), 0);
    assert!(
        !traces(&output, "remote.allowedHostsAdd")[0].enabled,
        "duplicate"
    );

    scene.host_state().remote_host = " *.TryCloudflare.com ".into();
    let input = scene.trace("remote.allowedHostsPlaceholder");
    scene.frame(click(input.rect.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(
        traces(&output, "remote.allowedHostsPlaceholder")[0].has_focus,
        "clicked input is focused"
    );
    scene.frame(vec![egui::Event::Ime(egui::ImeEvent::Preedit {
        text: "가".into(),
        active_range_chars: None,
    })]);
    let (output, _) = scene.frame(Vec::new());
    assert!(
        traces(&output, "remote.allowedHostsPlaceholder")[0].has_focus,
        "composing keeps focus"
    );
    scene.frame(key(egui::Key::Enter));
    assert!(scene.acts().is_empty(), "Enter while composing is ignored");
    let (output, _) = scene.frame(Vec::new());
    assert!(
        traces(&output, "remote.allowedHostsPlaceholder")[0].has_focus,
        "Enter while composing keeps focus"
    );
    scene.frame(vec![egui::Event::Ime(
        egui::ImeEvent::Commit(String::new()),
    )]);
    scene.host_state().remote_host = " *.TryCloudflare.com ".into();
    let (output, _) = scene.frame(Vec::new());
    let focused = traces(&output, "remote.allowedHostsPlaceholder")[0].has_focus;
    let draft = scene.host_state().remote_host.clone();
    scene.frame(key(egui::Key::Enter));
    assert!(
        scene.host_state().remote_host.is_empty(),
        "focused={focused}, draft={draft:?}, now={:?}",
        scene.host_state().remote_host
    );
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::UpdateSettings(HostedChange::RemoteAllowedHosts(hosts))]
            if hosts == &["tunnel.example.com".to_string(), "*.trycloudflare.com".to_string()]
    ));
    scene.host_state().remote_host = "other.example.com".into();
    let (output, _) = scene.frame(Vec::new());
    assert!(
        !traces(&output, "remote.allowedHostsAdd")[0].enabled,
        "saving"
    );
    assert!(
        !traces(&output, "remote.allowedHostsRemove")[0].enabled,
        "saving"
    );
    scene
        .data()
        .accept(Reply::SettingsUpdated(Ok(())), web_time::Instant::now());
    scene.settings.remote_allowed_hosts =
        vec!["tunnel.example.com".into(), "*.trycloudflare.com".into()];
    scene.click_trace("remote.allowedHostsAdd");
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::UpdateSettings(HostedChange::RemoteAllowedHosts(hosts))] if hosts.len() == 3
    ));
    scene
        .data()
        .accept(Reply::SettingsUpdated(Ok(())), web_time::Instant::now());
    let (output, _) = scene.frame(Vec::new());
    let first = traces(&output, "remote.allowedHostsRemove")[0].rect;
    scene.frame(click(first.center()));
    scene.frame(Vec::new());
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::UpdateSettings(HostedChange::RemoteAllowedHosts(hosts))]
            if hosts == &["*.trycloudflare.com".to_string()]
    ));
}

fn cli(installed: bool, dangling: bool) -> taide_model::agent::CliInstallStatus {
    taide_model::agent::CliInstallStatus {
        installed,
        resolved_path: None,
        dangling,
        target_path: "/usr/local/bin/taide".into(),
        editor_env_hint: String::new(),
    }
}

fn hooks(agent: &str, installed: bool) -> taide_model::agent::AgentHooksStatus {
    taide_model::agent::AgentHooksStatus {
        agent_name: agent.into(),
        scope: if agent == "claude" {
            HookInstallScope::Project
        } else {
            HookInstallScope::User
        },
        installed,
        requires_taide_cli: agent == "codex",
    }
}

fn project_ref(name: &str) -> ProjectRef {
    ProjectRef {
        id: ProjectId::new(),
        root: format!("/tmp/{name}"),
        name: name.into(),
        display: Default::default(),
        root_missing: false,
    }
}

impl Scene {
    fn set_cli(&mut self, status: taide_model::agent::CliInstallStatus) {
        self.data().accept(
            Reply::CliInstalled(Ok(crate::settings_sections_data::CliOutcome {
                status,
                is_cancelled: false,
            })),
            web_time::Instant::now(),
        );
    }

    fn set_hooks(&mut self, target: HookTarget, status: taide_model::agent::AgentHooksStatus) {
        self.data().accept(
            Reply::HooksChanged {
                target,
                change: HookChange::Install,
                result: Ok(status),
            },
            web_time::Instant::now(),
        );
    }

    fn top(&mut self, field: &str) -> f32 {
        let (output, _) = self.frame(Vec::new());
        traces(&output, field)
            .first()
            .unwrap_or_else(|| panic!("trace exists: {field}"))
            .rect
            .top()
    }
}

fn user(agent: &'static str) -> HookTarget {
    HookTarget {
        project: ProjectId(String::new()),
        agent,
    }
}

#[test]
fn interface_행은_배지_다음에_cli_hooks_ide_순서이고_미러와_비_macos는_cli_행이_없다() {
    let mut scene = Scene::new();
    let (mirror, mirror_drawing) = scene.draw(Vec::new(), false);
    for field in [
        "settings.agentHooks",
        "settings.ideIntegration",
        "settings.cliInstallButton",
    ] {
        assert!(traces(&mirror, field).is_empty(), "{field}");
    }
    assert_eq!(
        count(&mirror_drawing, &scene.text("settings.ideIntegrationHint")),
        0
    );
    let (hosted, drawing) = scene.frame(Vec::new());
    assert!(traces(&hosted, "settings.cliInstallButton").is_empty());
    assert_eq!(
        count(&drawing, &scene.text("keymap.category.shellCommand")),
        0
    );
    assert_eq!(count(&drawing, &scene.text("settings.agentHooksHint")), 1);
    assert_eq!(
        count(&drawing, &scene.text("settings.ideIntegrationHint")),
        1
    );
    scene.is_macos = true;
    let badge = scene.top("settings.agentStatusBadge");
    let shell = scene.top("settings.cliInstallButton");
    let hooks = scene.top("settings.agentHooks");
    let ide = scene.top("settings.ideIntegration");
    let diff = scene.top("settings.ideAutoOpenDiff");
    assert!(badge < shell && shell < hooks && hooks < ide && ide < diff);
    let (output, _) = scene.frame(Vec::new());
    let ide = traces(&output, "settings.ideIntegration")[0].rect;
    let (clicked, _) = scene.frame(click(ide.center()));
    assert_eq!(
        clicked.changes,
        vec![Change::Hosted(HostedChange::IdeIntegration(
            !scene.settings.ide_integration_enabled
        ))]
    );
}

#[test]
fn cli_행은_조회중_미설치_설치됨_끊어진링크를_원본_문구와_색과_버튼으로_그린다() {
    let mut scene = Scene::new();
    scene.is_macos = true;
    let installed_text = scene.text("settings.cliStatusInstalled");
    let dangling_text = scene.text("settings.cliStatusDangling");
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(traces(&output, "settings.cliInstallButton").len(), 1);
    assert!(traces(&output, "settings.cliUninstallButton").is_empty());
    assert_eq!(count(&drawing, &installed_text), 0);
    assert_eq!(
        color_of(&drawing, &scene.text("keymap.category.shellCommand")),
        scene.appearance.foreground
    );
    scene.answer(|query, generation| {
        (query == Query::CliStatus).then(|| Reply::CliStatus {
            generation,
            result: Ok(cli(false, false)),
        })
    });
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(traces(&output, "settings.cliInstallButton").len(), 1);
    assert!(traces(&output, "settings.cliUninstallButton").is_empty());
    assert_eq!(count(&drawing, &installed_text), 0);
    scene.set_cli(cli(true, false));
    let (output, drawing) = scene.frame(Vec::new());
    assert!(traces(&output, "settings.cliInstallButton").is_empty());
    assert_eq!(traces(&output, "settings.cliUninstallButton").len(), 1);
    assert_eq!(color_of(&drawing, &installed_text), scene.appearance.muted);
    scene.set_cli(cli(true, true));
    let (output, drawing) = scene.frame(Vec::new());
    let install = traces(&output, "settings.cliInstallButton")[0].rect;
    let uninstall = traces(&output, "settings.cliUninstallButton")[0].rect;
    assert!(install.right() < uninstall.left());
    assert_eq!(
        color_of(&drawing, &dangling_text),
        scene.appearance.sections.warning
    );
    assert_eq!(count(&drawing, &installed_text), 0);
    scene.frame(click(install.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.cliInstallButton")[0].enabled);
    assert!(traces(&output, "settings.cliUninstallButton")[0].enabled);
    assert!(matches!(scene.acts().as_slice(), [Action::CliInstall]));
    scene.frame(click(uninstall.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.cliUninstallButton")[0].enabled);
    assert!(matches!(scene.acts().as_slice(), [Action::CliUninstall]));
}

#[test]
fn hooks_토글은_꺼지면_목록이_없고_켜면_에이전트_5종과_프로젝트_없음_문구를_원본_순서로_그린다() {
    let mut scene = Scene::new();
    let labels = [
        "settings.agentHooksAgentClaude",
        "settings.agentHooksAgentCodex",
        "settings.agentHooksAgentGemini",
        "settings.agentHooksAgentOpencode",
        "settings.agentHooksAgentPi",
    ];
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &scene.text(labels[0])), 0);
    assert_eq!(count(&drawing, &scene.text("app.noProjectOpen")), 0);
    for label in &labels[1..] {
        assert!(traces(&output, label).is_empty(), "{label}");
    }
    let toggle = traces(&output, "settings.agentHooks")[0].rect;
    let (clicked, _) = scene.frame(click(toggle.center()));
    assert_eq!(
        clicked.changes,
        vec![Change::Hosted(HostedChange::AgentHooks(true))]
    );
    scene.settings.agent_hooks_enabled = true;
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(
        color_of(&drawing, &scene.text(labels[0])),
        scene.appearance.muted
    );
    assert_eq!(
        color_of(&drawing, &scene.text("app.noProjectOpen")),
        scene.appearance.muted
    );
    let mut tops = vec![
        painted_top(&drawing, &scene.text(labels[0])),
        painted_top(&drawing, &scene.text("app.noProjectOpen")),
    ];
    tops.extend(
        labels[1..]
            .iter()
            .map(|label| traces(&output, label)[0].rect.top()),
    );
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    let ide = traces(&output, "settings.ideIntegration")[0].rect.top();
    assert!(tops[tops.len() - 1] < ide);
    let fetched = scene
        .data()
        .take_requests()
        .into_iter()
        .filter_map(|request| match request {
            Request::Fetch {
                query: Query::HooksStatus(target),
                ..
            } => Some(target.agent),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(fetched, vec!["codex", "gemini", "opencode", "pi"]);
}

fn painted_top(drawing: &egui::FullOutput, value: &str) -> f32 {
    let mut pending = drawing
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    while let Some(shape) = pending.pop() {
        match shape {
            egui::Shape::Vec(shapes) => pending.extend(shapes),
            egui::Shape::Text(text) if text.galley.job.text == value => {
                return text.pos.y;
            }
            _ => (),
        }
    }
    panic!("text not painted: {value}")
}

#[test]
fn hooks_프로젝트_행은_조회중_설치됨_미설치와_동의_대화상자를_원본대로_처리한다() {
    let mut scene = Scene::new();
    scene.settings.agent_hooks_enabled = true;
    scene.projects = vec![project_ref("alpha"), project_ref("beta")];
    let alpha = HookTarget {
        project: scene.projects[0].id.clone(),
        agent: "claude",
    };
    let beta = HookTarget {
        project: scene.projects[1].id.clone(),
        agent: "claude",
    };
    let loading = scene.text("settings.loading");
    scene.settings.agent_hooks_enabled = false;
    let (_, closed) = scene.frame(Vec::new());
    scene.settings.agent_hooks_enabled = true;
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, "alpha"), 1);
    assert_eq!(count(&drawing, "beta"), 1);
    assert_eq!(count(&drawing, &loading), count(&closed, &loading) + 2);
    scene.set_hooks(alpha.clone(), hooks("claude", true));
    scene.set_hooks(beta.clone(), hooks("claude", false));
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &loading), count(&closed, &loading));
    assert_eq!(
        color_of(&drawing, &scene.text("agent.hooksInstalled")),
        scene.appearance.sections.agent_running
    );
    assert_eq!(traces(&output, "agent.hooksUninstall").len(), 1);
    assert_eq!(traces(&output, "agent.hooksInstall").len(), 1);
    scene.data().take_requests();
    let uninstall = traces(&output, "agent.hooksUninstall")[0].rect;
    scene.frame(click(uninstall.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "agent.hooksUninstall")[0].enabled);
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::Hooks { target, change: HookChange::Uninstall }] if *target == alpha
    ));
    let title = scene.text("agent.hooksConsentTitle");
    let (_, drawing) = scene.click_trace("agent.hooksInstall");
    assert_eq!(count(&drawing, &title), 1);
    assert_eq!(
        count(&drawing, &scene.text("agent.hooksConsentDescription")),
        1
    );
    assert!(scene.acts().is_empty());
    let (focused, _) = scene.frame(Vec::new());
    assert!(traces(&focused, "settings.dialog.cancel")[0].has_focus);
    let confirm = traces(&focused, "settings.dialog.confirm")[0].rect;
    let cancel = traces(&focused, "settings.dialog.cancel")[0].rect;
    assert!(cancel.right() <= confirm.left());
    scene.frame(click(OUTSIDE));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 1);
    scene.frame(key(egui::Key::Escape));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 0);
    assert!(scene.acts().is_empty());
    let (returned, _) = scene.frame(Vec::new());
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(traces(&returned, "agent.hooksInstall")[0].id),
        "focus returns to the install button that opened the dialog"
    );
    scene.click_trace("agent.hooksInstall");
    let (output, drawing) = scene.click_trace("settings.dialog.confirm");
    assert_eq!(count(&drawing, &title), 0);
    assert!(!traces(&output, "agent.hooksInstall")[0].enabled);
    assert!(matches!(
        scene.acts().as_slice(),
        [Action::Hooks { target, change: HookChange::Install }] if *target == beta
    ));
    scene.click_trace("agent.hooksInstall");
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, &title),
        0,
        "install in flight keeps the button disabled"
    );
    scene.data().accept(
        Reply::HooksChanged {
            target: beta.clone(),
            change: HookChange::Install,
            result: Err(AppError::Internal("x".into())),
        },
        web_time::Instant::now(),
    );
    scene.click_trace("agent.hooksInstall");
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 1);
    scene.settings.agent_hooks_enabled = false;
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &title), 0, "the dialog leaves with its row");
}

const HOOKS_STALE_AGE: std::time::Duration = std::time::Duration::from_secs(61);

fn hooks_fetches(scene: &mut Scene, target: &HookTarget) -> Vec<u64> {
    scene
        .data()
        .take_requests()
        .into_iter()
        .filter_map(|request| match request {
            Request::Fetch {
                query: Query::HooksStatus(fetched),
                generation,
            } if fetched == *target => Some(generation),
            _ => None,
        })
        .collect()
}

#[test]
fn hooks_목록은_껐다_켜면_다시_mount되어_오래된_상태를_다시_조회한다() {
    let mut scene = Scene::new();
    scene.settings.agent_hooks_enabled = true;
    scene.projects = vec![project_ref("alpha")];
    let alpha = HookTarget {
        project: scene.projects[0].id.clone(),
        agent: "claude",
    };
    scene.frame(Vec::new());
    let generation = hooks_fetches(&mut scene, &alpha)[0];
    let stale = web_time::Instant::now()
        .checked_sub(HOOKS_STALE_AGE)
        .unwrap();
    scene.data().accept(
        Reply::HooksStatus {
            target: alpha.clone(),
            generation,
            result: Ok(hooks("claude", true)),
        },
        stale,
    );
    scene.frame(Vec::new());
    assert!(hooks_fetches(&mut scene, &alpha).is_empty());
    scene.settings.agent_hooks_enabled = false;
    scene.frame(Vec::new());
    scene.settings.agent_hooks_enabled = true;
    scene.frame(Vec::new());
    assert_eq!(hooks_fetches(&mut scene, &alpha).len(), 1);
    scene.frame(Vec::new());
    assert!(hooks_fetches(&mut scene, &alpha).is_empty());
}

#[test]
fn hooks_사용자_행은_cli_요구와_조회상태로_비활성_4경우와_경고를_원본대로_정한다() {
    let mut scene = Scene::new();
    scene.settings.agent_hooks_enabled = true;
    let missing = scene.text("settings.agentHooksCliMissing");
    let warning = scene.text("settings.agentHooksUserLevelWarning");
    let (output, drawing) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.agentHooksAgentCodex")[0].enabled);
    assert!(!traces(&output, "settings.agentHooksAgentGemini")[0].enabled);
    assert_eq!(count(&drawing, &warning), 4);
    assert_eq!(
        color_of(&drawing, &warning),
        scene.appearance.sections.warning
    );
    assert_eq!(
        color_of(
            &drawing,
            &scene.text("settings.agentHooksUserLevelDescription")
        ),
        scene.appearance.muted
    );
    scene.set_hooks(user("codex"), hooks("codex", false));
    scene.set_hooks(user("gemini"), hooks("gemini", false));
    let (output, drawing) = scene.frame(Vec::new());
    assert!(
        !traces(&output, "settings.agentHooksAgentCodex")[0].enabled,
        "CLI status still loading"
    );
    assert!(traces(&output, "settings.agentHooksAgentGemini")[0].enabled);
    assert_eq!(count(&drawing, &missing), 0);
    scene.set_cli(cli(false, false));
    let (output, drawing) = scene.frame(Vec::new());
    assert!(
        !traces(&output, "settings.agentHooksAgentCodex")[0].enabled,
        "CLI missing blocks install"
    );
    assert_eq!(count(&drawing, &missing), 1);
    assert_eq!(
        color_of(&drawing, &missing),
        scene.appearance.sections.warning
    );
    scene.set_hooks(user("codex"), hooks("codex", true));
    let (output, drawing) = scene.frame(Vec::new());
    assert!(
        traces(&output, "settings.agentHooksAgentCodex")[0].enabled,
        "installed hooks stay removable"
    );
    assert_eq!(count(&drawing, &missing), 1);
    scene.data().take_requests();
    let codex = traces(&output, "settings.agentHooksAgentCodex")[0].rect;
    scene.frame(click(codex.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.agentHooksAgentCodex")[0].enabled);
    let gemini = traces(&output, "settings.agentHooksAgentGemini")[0].rect;
    scene.frame(click(gemini.center()));
    scene.frame(Vec::new());
    assert!(matches!(
        scene.acts().as_slice(),
        [
            Action::Hooks { target: first, change: HookChange::Uninstall },
            Action::Hooks { target: second, change: HookChange::Install },
        ] if *first == user("codex") && *second == user("gemini")
    ));
    scene.set_cli(cli(true, false));
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &missing), 0);
}

#[test]
fn 알림_버튼은_스위치_아래에서_시스템_설정과_시험_알림을_요청하고_dev_안내는_그_아래에만_있다() {
    let mut scene = Scene::new();
    let hint = scene.text("settings.notificationsDevHint");
    let (mirror, _) = scene.draw(Vec::new(), false);
    assert!(traces(&mirror, "settings.notificationsSendTest").is_empty());
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &hint), 0);
    let last = traces(&output, "settings.notificationsError")[0].rect;
    let open = traces(&output, "settings.notificationsOpenSystemSettings")[0].rect;
    let test = traces(&output, "settings.notificationsSendTest")[0].rect;
    assert!(last.bottom() < open.top());
    assert!((open.top() - test.top()).abs() < RATIO_TOLERANCE);
    assert!(open.right() < test.left());
    scene.is_dev_build = true;
    let (output, drawing) = scene.frame(Vec::new());
    assert_eq!(color_of(&drawing, &hint), scene.appearance.muted);
    let hint_top = painted_top(&drawing, &hint);
    assert!(
        traces(&output, "settings.notificationsSendTest")[0]
            .rect
            .bottom()
            < hint_top
    );
    scene.data().take_requests();
    scene.data().observe_window_focus(false);
    scene.frame(click(open.center()));
    scene.frame(click(test.center()));
    scene.data().observe_window_focus(true);
    scene.frame(click(test.center()));
    scene.frame(Vec::new());
    let title = scene.text("settings.notificationsSendTest");
    let body = scene.text("notification.enableHint");
    let acts = scene.acts();
    assert!(matches!(
        acts.as_slice(),
        [
            Action::OpenNotificationSettings,
            Action::SendTestNotification { title: first_title, body: first_body, any_window_focused: false },
            Action::SendTestNotification { any_window_focused: true, .. },
        ] if *first_title == title && *first_body == body
    ));
}

#[test]
fn performance는_게이트가_켜질때만_목차와_카드_끝에_있고_읽기_초기화를_원본대로_보인다() {
    let mut scene = Scene::new();
    let (output, drawing) = scene.frame(Vec::new());
    assert!(traces(&output, Section::Performance.title()).is_empty());
    assert_eq!(
        count(&drawing, &scene.text("settings.performanceDescription")),
        0
    );
    scene.is_perf_enabled = true;
    let (output, drawing) = scene.frame(Vec::new());
    let toc = traces(&output, Section::Performance.title())[0].rect;
    let remote = traces(&output, Section::Remote.title())[0].rect;
    assert!(remote.top() < toc.top());
    assert_eq!(
        count(&drawing, &scene.text("settings.performanceDescription")),
        1
    );
    let read = traces(&output, "settings.performanceRead")[0].rect;
    let reset = traces(&output, "settings.performanceReset")[0].rect;
    assert!(read.right() < reset.left());
    assert!(
        painted_top(&drawing, &scene.text("remote.description"))
            < painted_top(&drawing, &scene.text("settings.performanceDescription"))
    );
    assert!(painted_top(&drawing, &scene.text("settings.performanceDescription")) < read.top());
    scene.data().take_requests();
    scene.frame(click(read.center()));
    let (output, _) = scene.frame(Vec::new());
    assert!(!traces(&output, "settings.performanceRead")[0].enabled);
    assert!(!traces(&output, "settings.performanceReset")[0].enabled);
    let ticket = match scene.acts().as_slice() {
        [Action::PerfRead { ticket }] => *ticket,
        acts => panic!("one perf read: {acts:?}"),
    };
    let snapshot = taide_model::app::PerfSnapshot {
        enabled: true,
        entries: Vec::new(),
        counters: vec![taide_model::app::PerfCounterEntry {
            name: "search.files".into(),
            total: 3.0,
        }],
    };
    scene.data().accept(
        Reply::PerfRead {
            ticket,
            result: Ok(snapshot.clone()),
        },
        web_time::Instant::now(),
    );
    let (output, drawing) = scene.frame(Vec::new());
    let json = crate::settings_sections_data::perf_json(&snapshot).unwrap();
    assert!(json.contains("\"total\": 3"));
    assert_eq!(count(&drawing, &json), 1);
    let heading = painted_top(&drawing, &scene.text("settings.performanceNative"));
    assert!(read.bottom() < heading && heading < painted_top(&drawing, &json));
    assert!(traces(&output, "settings.performanceRead")[0].enabled);
    scene.frame(click(reset.center()));
    let ticket_again = match scene.acts().as_slice() {
        [Action::PerfReset { ticket }] => *ticket,
        acts => panic!("one perf reset: {acts:?}"),
    };
    assert_eq!(ticket_again, ticket);
    scene.data().accept(
        Reply::PerfReset {
            ticket,
            result: Ok(()),
        },
        web_time::Instant::now(),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, &json), 0);
    assert_eq!(
        count(&drawing, &scene.text("settings.performanceNative")),
        0
    );
}

const TOC_SCREEN_HEIGHT: f32 = 900.0;
const SCROLL_FRAMES: usize = 12;
const WHEEL_DELTA: f32 = -400.0;
const ANCHOR_TOLERANCE: f32 = 1.0;
const ORIGINAL_TOC: [Section; 14] = [
    Section::Appearance,
    Section::Language,
    Section::Interface,
    Section::Notifications,
    Section::Editor,
    Section::Snippets,
    Section::Terminal,
    Section::Keymap,
    Section::Lsp,
    Section::Ai,
    Section::Plugins,
    Section::Sync,
    Section::Remote,
    Section::Performance,
];

fn card_title_top(drawing: &egui::FullOutput, value: &str) -> Option<f32> {
    let mut pending = drawing
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    while let Some(shape) = pending.pop() {
        match shape {
            egui::Shape::Vec(shapes) => pending.extend(shapes),
            egui::Shape::Text(text)
                if text.galley.job.text == value && text.pos.x > super::super::TOC_WIDTH =>
            {
                return Some(text.pos.y);
            }
            _ => (),
        }
    }
    None
}

fn card_anchor(viewport: Rect, title_inset: f32) -> f32 {
    viewport.top() + f32::from(super::super::PAGE_Y) + title_inset
}

fn settle(scene: &mut Scene) -> (Output, egui::FullOutput) {
    for _ in 0..SCROLL_FRAMES {
        scene.frame(Vec::new());
    }
    scene.frame(Vec::new())
}

#[test]
fn 목차는_호스트에서_원본_14항목_순서이고_미러는_8항목이며_클릭한_카드를_viewport_위_32px로_옮긴다()
{
    let mut scene = Scene::new();
    scene.screen = egui::vec2(SCREEN[0], TOC_SCREEN_HEIGHT);
    scene.is_perf_enabled = true;
    let (mirror, _) = scene.draw(Vec::new(), false);
    let basic = Section::BASIC.len();
    let mirror_tops = ORIGINAL_TOC
        .iter()
        .map(|section| {
            traces(&mirror, section.title())
                .first()
                .map(|trace| trace.rect.top())
        })
        .collect::<Vec<_>>();
    assert!(mirror_tops[..basic].iter().all(Option::is_some));
    assert!(mirror_tops[basic..].iter().all(Option::is_none));
    assert!(
        mirror_tops[..basic]
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    let (hosted, drawing) = scene.frame(Vec::new());
    let hosted_tops = ORIGINAL_TOC
        .iter()
        .map(|section| traces(&hosted, section.title())[0].rect.top())
        .collect::<Vec<_>>();
    assert!(hosted_tops.windows(2).all(|pair| pair[0] < pair[1]));
    for section in ORIGINAL_TOC {
        assert!(
            count(&drawing, &scene.text(section.title())) >= 1,
            "{section:?}"
        );
    }
    assert_eq!(
        scene.views.entries[&scene.owner].active,
        Section::Appearance
    );
    let title_inset = card_title_top(&drawing, &scene.text(Section::Appearance.title())).unwrap()
        - traces(&hosted, Section::Appearance.title())[0].rect.top();

    let lsp = traces(&hosted, Section::Lsp.title())[0].rect;
    scene.frame(click(lsp.center()));
    let (output, drawing) = settle(&mut scene);
    assert_eq!(scene.views.entries[&scene.owner].active, Section::Lsp);
    let (_, offset, viewport) = output.scroll.unwrap();
    assert!(offset.y > 0.0);
    let title = card_title_top(&drawing, &scene.text(Section::Lsp.title())).unwrap();
    let anchor = card_anchor(viewport, title_inset);
    assert!(
        (title - anchor).abs() <= ANCHOR_TOLERANCE,
        "title={title}, anchor={anchor}"
    );
    assert!(
        (traces(&output, Section::Appearance.title())[0].rect.top()
            - viewport.top()
            - f32::from(super::super::PAGE_Y))
        .abs()
            <= ANCHOR_TOLERANCE,
        "the table of contents sticks 32px below the viewport top"
    );

    let performance = traces(&output, Section::Performance.title())[0].rect;
    scene.frame(click(performance.center()));
    let (output, drawing) = settle(&mut scene);
    assert_eq!(
        scene.views.entries[&scene.owner].active,
        Section::Performance
    );
    let (_, offset, viewport) = output.scroll.unwrap();
    let title = card_title_top(&drawing, &scene.text(Section::Performance.title())).unwrap();
    let anchor = card_anchor(viewport, title_inset);
    if (title - anchor).abs() > ANCHOR_TOLERANCE {
        assert!(title > anchor);
        scene.frame(vec![
            egui::Event::PointerMoved(viewport.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, WHEEL_DELTA),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let (after, _) = settle(&mut scene);
        assert_eq!(
            after.scroll.unwrap().1,
            offset,
            "a short last card leaves the page at its end, as the original scrollTo clamps"
        );
    }
}

fn focused(scene: &Scene) -> Option<egui::Id> {
    scene.context.memory(|memory| memory.focused())
}

fn tab(is_backward: bool) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key: egui::Key::Tab,
        physical_key: Some(egui::Key::Tab),
        pressed: true,
        repeat: false,
        modifiers: if is_backward {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        },
    }]
}

/// Checks the shared Settings dialog contract: focus starts on the first of `size` controls, Tab
/// and Shift+Tab cycle only through them, `dismiss` closes the dialog without a request, and focus
/// returns to the `opener` control.
fn assert_dialog_contract(
    scene: &mut Scene,
    marker: &str,
    size: usize,
    members: &[&str],
    opener: &str,
    dismiss: Vec<egui::Event>,
) {
    scene.frame(Vec::new());
    let (output, drawing) = scene.frame(Vec::new());
    assert!(count(&drawing, marker) > 0, "{marker} is open");
    let first = focused(scene).expect("an open dialog focuses its first control");
    let opener_id = traces(&output, opener).last().unwrap().id;
    let mut forward = vec![first];
    for _ in 0..size {
        scene.frame(tab(false));
        forward.push(focused(scene).expect("Tab keeps a focused control"));
    }
    assert_eq!(forward[size], first, "{marker}: Tab wraps after {size}");
    let cycle = forward[..size]
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(cycle.len(), size, "{marker}: Tab visits {size} controls");
    assert!(!cycle.contains(&opener_id), "{marker}: focus stays inside");
    let mut backward = vec![first];
    for _ in 0..size {
        scene.frame(tab(true));
        backward.push(focused(scene).expect("Shift+Tab keeps a focused control"));
    }
    assert_eq!(
        backward,
        forward.iter().rev().copied().collect::<Vec<_>>(),
        "{marker}: Shift+Tab walks the same cycle backwards"
    );
    let (output, _) = scene.frame(Vec::new());
    for member in members {
        let id = traces(&output, member).last().unwrap().id;
        assert!(cycle.contains(&id), "{marker}: {member} is in the cycle");
    }
    scene.data().take_requests();
    scene.frame(dismiss);
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(count(&drawing, marker), 0, "{marker} is closed");
    assert!(
        scene.acts().is_empty(),
        "{marker}: dismissal requests nothing"
    );
    let (output, _) = scene.frame(Vec::new());
    assert_eq!(
        focused(scene),
        Some(traces(&output, opener).last().unwrap().id),
        "{marker}: focus returns to {opener}"
    );
}

#[test]
fn 설정_대화상자_5종은_같은_tab_순환_escape_닫기와_여는_컨트롤로_포커스_복귀를_따른다() {
    let alert = ["settings.dialog.cancel", "settings.dialog.confirm"];

    let mut scene = Scene::new();
    scene.settings.agent_hooks_enabled = true;
    scene.projects = vec![project_ref("alpha")];
    let target = HookTarget {
        project: scene.projects[0].id.clone(),
        agent: "claude",
    };
    scene.frame(Vec::new());
    scene.set_hooks(target, hooks("claude", false));
    scene.click_trace("agent.hooksInstall");
    let marker = scene.text("agent.hooksConsentTitle");
    assert_dialog_contract(
        &mut scene,
        &marker,
        alert.len(),
        &alert,
        "agent.hooksInstall",
        key(egui::Key::Escape),
    );

    let mut scene = Scene::new();
    scene.plugins(vec![plugin("alpha", true, None)]);
    scene.click_trace("settings.pluginUninstallButton");
    let marker = message(
        &scene.locale,
        "settings.pluginUninstallConfirmTitle",
        &[("name", "alpha plugin")],
    );
    assert_dialog_contract(
        &mut scene,
        &marker,
        alert.len(),
        &alert,
        "settings.pluginUninstallButton",
        key(egui::Key::Escape),
    );

    let mut scene = Scene::new();
    open_vsix(
        &mut scene,
        vec![candidate(0, "Night", "fixture-night", false, 0, None)],
        &[],
    );
    let composing = |text: &str| {
        egui::Event::Ime(egui::ImeEvent::Preedit {
            text: text.into(),
            active_range_chars: None,
        })
    };
    scene.frame(vec![
        composing("가"),
        egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    scene.frame(vec![egui::Event::Ime(egui::ImeEvent::Commit("가".into()))]);
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, "Fixture Themes"),
        1,
        "Escape during an IME composition keeps the VSIX dialog"
    );
    let vsix_controls = 5;
    assert_dialog_contract(
        &mut scene,
        "Fixture Themes",
        vsix_controls,
        &[
            "common.save",
            "settings.pluginImportVsixGrammars",
            "settings.dialog.close",
            "settings.dialog.corner",
        ],
        "settings.pluginImportVsixButton",
        key(egui::Key::Escape),
    );

    let mut scene = Scene::new();
    open_vsix(
        &mut scene,
        vec![candidate(0, "Day", "fixture-day", true, 0, None)],
        &["fixture-day"],
    );
    scene.click_trace("common.save");
    let marker = scene.text("common.confirm");
    assert_dialog_contract(
        &mut scene,
        &marker,
        alert.len(),
        &alert,
        "common.save",
        key(egui::Key::Escape),
    );
    let (_, drawing) = scene.frame(Vec::new());
    assert_eq!(
        count(&drawing, "Fixture Themes"),
        1,
        "closing the overwrite check keeps the VSIX dialog"
    );

    let mut scene = Scene::new();
    scene.statuses(Some(synced(None, Some(SYNCED_AT))), None);
    scene.click_trace("settings.syncDownloadNow");
    let owner = scene.state().container.owner.unwrap();
    scene.acts();
    scene.data().accept(
        Reply::SyncDownloaded {
            force: false,
            owner,
            result: Ok(SyncDownloadResult::Conflict {
                remote_updated_at: SYNCED_AT.into(),
            }),
        },
        web_time::Instant::now(),
    );
    let marker = scene.text("settings.syncConflictTitle");
    assert_dialog_contract(
        &mut scene,
        &marker,
        alert.len(),
        &alert,
        "settings.syncDownloadNow",
        key(egui::Key::Escape),
    );
}

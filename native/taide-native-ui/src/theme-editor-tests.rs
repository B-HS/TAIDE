use super::*;
use std::{sync::Arc, time::Duration};
use taide_model::{
    ids::{PaneId, ProjectId, TabId},
    layout::{PaneNode, Tab, TabKind},
    paths::AppPaths,
};
use taide_runtime::{AppState, TaskSupervisor, theme_actions};

const SCREEN: [f32; 2] = [1280.0, 900.0];
const FRAME_TIME: f64 = 0.1;
const DEADLINE: Duration = Duration::from_secs(5);
const HEX_INPUT_TRACE: usize = 3;

struct NoopEvents;
impl taide_runtime::EventSink for NoopEvents {
    fn publish(&self, _: taide_model::app_event::AppEvent) {}
}

struct Fixture {
    directory: std::path::PathBuf,
    state: AppState,
    runtime: tokio::runtime::Runtime,
    services: Arc<taide_runtime::AppServices>,
    owner: Owner,
    context: egui::Context,
    locale: ResolvedLocale,
    appearance: Appearance,
    icons: Icons,
    time: f64,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-theme-editor-{}", ProjectId::new()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        let owner = Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: owner.pane.clone(),
            tabs: vec![Tab {
                id: owner.tab.clone(),
                kind: TabKind::Settings,
                title: "Settings".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: Some(owner.tab.clone()),
        };
        layout.focused_pane = owner.pane.clone();
        state.layouts.write().insert(owner.project.clone(), layout);
        let theme = theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
        let locale =
            taide_runtime::locale_actions::locale_get_for_language(&state, "en", "en").unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let services = test_services(
            state.clone(),
            TaskSupervisor::new(runtime.handle().clone()),
            Arc::new(NoopEvents),
        );
        Self {
            directory,
            state,
            runtime,
            services,
            owner,
            context: egui::Context::default(),
            locale,
            appearance: Appearance::new(&theme).unwrap(),
            icons: Icons::new().unwrap(),
            time: 0.0,
        }
    }

    fn editor(&self) -> Editor {
        Editor::new(
            self.owner.clone(),
            "taide-dark".into(),
            Mode::Create,
            "Native copy".into(),
        )
        .unwrap()
    }

    fn frame(
        &mut self,
        editor: &mut Editor,
        events: Vec<egui::Event>,
    ) -> (Output, egui::FullOutput) {
        self.time += FRAME_TIME;
        let mut result = None;
        let mut frame = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                self.icons.prepare(ui.ctx()).unwrap();
                result = Some(editor.show(ui, &self.locale, &self.appearance, &self.icons));
            },
        );
        frame.textures_delta.clear();
        (result.unwrap(), frame)
    }

    fn ready(&mut self, editor: &mut Editor) {
        let output = self.frame(editor, Vec::new()).0;
        assert_eq!(output.commands.len(), 1);
        let reply = self.runtime.block_on(
            output
                .commands
                .into_iter()
                .next()
                .unwrap()
                .execute(&self.services),
        );
        let accepted = editor.accept(reply).unwrap();
        assert!(!accepted.close && !accepted.mutated && accepted.error.is_none());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.runtime.block_on(async {
            tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
                .await
                .unwrap();
        });
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn click(rect: Rect) -> Vec<egui::Event> {
    let pos = rect.center();
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn target(output: &Output, key: &str) -> Rect {
    output
        .traces
        .iter()
        .find(|trace| trace.0 == key)
        .unwrap_or_else(|| panic!("missing target {key}: {:?}", output.traces))
        .2
}

#[test]
fn native_theme_editor는_picker_hex_blur와_reset_순서를_보존한다() {
    let mut fixture = Fixture::new();
    let mut editor = fixture.editor();
    fixture.ready(&mut editor);
    editor.search = "accent".into();
    let base = editor.draft.as_ref().unwrap().base().colors["app.accent"].clone();
    editor
        .draft
        .as_mut()
        .unwrap()
        .set_color(ColorDomain::Colors, "app.accent", "#ff0000".into())
        .unwrap();
    fixture.frame(&mut editor, Vec::new());
    let key = PickerKey::Color(ColorDomain::Colors, "app.accent".into());
    let trigger = editor.pickers[&key].input_traces()[0].unwrap().1;
    fixture.frame(&mut editor, click(trigger));
    fixture.frame(&mut editor, Vec::new());
    let input = editor.pickers[&key].input_traces()[HEX_INPUT_TRACE]
        .unwrap()
        .1;
    fixture.frame(&mut editor, click(input));
    let select = egui::Event::Key {
        key: egui::Key::A,
        physical_key: Some(egui::Key::A),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            command: true,
            mac_cmd: true,
            ..Default::default()
        },
    };
    fixture.frame(
        &mut editor,
        vec![select, egui::Event::Text("#00ff00".into())],
    );
    assert_eq!(
        editor.draft.as_ref().unwrap().current().colors["app.accent"],
        "#ff0000"
    );
    let ready = fixture.frame(&mut editor, Vec::new()).0;
    let output = fixture
        .frame(
            &mut editor,
            click(target(&ready, "reset-Colors-app.accent")),
        )
        .0;
    assert!(output.error.is_none());
    assert_eq!(
        editor.draft.as_ref().unwrap().current().colors["app.accent"],
        base
    );
}

#[test]
fn native_theme_editor는_background_검색_팝업의_hex_focus와_미리보기를_보존한다() {
    let mut fixture = Fixture::new();
    let mut editor = fixture.editor();
    fixture.ready(&mut editor);
    editor.search = "background".into();
    fixture.frame(&mut editor, Vec::new());
    let key = PickerKey::Color(ColorDomain::Colors, "app.background".into());
    let trigger = editor.pickers[&key].input_traces()[0].unwrap().1;
    fixture.frame(&mut editor, click(trigger));
    fixture.frame(&mut editor, Vec::new());
    let (id, input) = editor.pickers[&key].input_traces()[HEX_INPUT_TRACE].unwrap();
    fixture.frame(&mut editor, click(input));
    fixture.frame(&mut editor, Vec::new());
    assert_eq!(fixture.context.memory(|memory| memory.focused()), Some(id));
    fixture.frame(
        &mut editor,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    command: true,
                    mac_cmd: true,
                    ..Default::default()
                },
            },
            egui::Event::Text("#244466".into()),
        ],
    );
    let output = fixture.frame(&mut editor, Vec::new()).0;
    fixture.frame(&mut editor, click(target(&output, "name")));
    fixture.frame(&mut editor, Vec::new());
    assert_eq!(
        editor.preview().unwrap().colors["app.background"],
        "#244466"
    );
}

#[test]
fn native_theme_editor는_원본_token_순서를_보존한다() {
    let source = include_str!("../../../src/entities/theme/theme-tokens.ts");
    let quoted = |marker: &str| {
        source
            .split_once(marker)
            .unwrap()
            .1
            .split_once('[')
            .unwrap()
            .1
            .split_once("] as const")
            .unwrap()
            .0
            .split('\'')
            .enumerate()
            .filter(|(index, _)| index % 2 == 1)
            .map(|(_, token)| token)
            .collect::<Vec<_>>()
    };
    let colors = COLORS
        .iter()
        .flat_map(|(namespace, tokens)| std::iter::once(*namespace).chain(tokens.iter().copied()))
        .collect::<Vec<_>>();
    assert_eq!(quoted("export const COLOR_NAMESPACES"), colors);
    assert_eq!(quoted("export const SYNTAX_TOKENS"), SYNTAX);
    assert_eq!(quoted("export const TERMINAL_TOKENS"), TERMINAL);
    let fixture = Fixture::new();
    let theme = theme_actions::theme_get(&fixture.state, "taide-dark".into()).unwrap();
    for (namespace, tokens) in COLORS {
        for token in *tokens {
            assert!(theme.colors.contains_key(&format!("{namespace}.{token}")));
        }
    }
    for token in SYNTAX {
        assert!(theme.syntax.contains_key(*token));
    }
    for token in TERMINAL {
        assert!(theme.terminal.contains_key(*token));
    }
}

#[test]
fn native_theme_editor는_원본_dirty와_요청별_실패재시도_수명을_보존한다() {
    let mut fixture = Fixture::new();
    let mut editor = fixture.editor();
    fixture.ready(&mut editor);
    let mut output = Output::default();
    editor.request_close(&mut output);
    assert!(output.close);
    assert!(editor.dialog.is_none());
    editor.draft.as_mut().unwrap().rename("Edited copy".into());
    output = Output::default();
    editor.request_close(&mut output);
    assert!(!output.close);
    assert!(editor.dialog == Some(Dialog::Discard));
    editor.dialog = None;
    editor.request_save(&mut output);
    let Command::Save(old) = output.commands.pop().unwrap() else {
        panic!("expected save");
    };
    assert!(
        editor
            .accept(Reply::Saved {
                request: old.clone(),
                result: Err(AppError::Internal("synthetic write failure".into()))
            })
            .unwrap()
            .error
            .is_some()
    );
    editor.request_save(&mut output);
    let Command::Save(current) = output.commands.pop().unwrap() else {
        panic!("expected retry");
    };
    assert!(!old.same_request(&current));
    assert!(
        editor
            .accept(Reply::Saved {
                request: old,
                result: Err(AppError::Internal("late prior failure".into()))
            })
            .is_none()
    );
    assert!(editor.save.is_some());
    let reply = fixture
        .runtime
        .block_on(Command::Save(current.clone()).execute(&fixture.services));
    let accepted = editor.accept(reply).unwrap();
    assert!(accepted.close && accepted.mutated && accepted.error.is_none());
    let saved = theme_actions::theme_get(
        &fixture.state,
        editor.draft.as_ref().unwrap().current().id.clone(),
    )
    .unwrap();
    assert_eq!(saved.name, "Edited copy");
    assert!(
        editor
            .accept(Reply::Saved {
                request: current,
                result: Err(AppError::Internal("duplicate reply".into()))
            })
            .is_none()
    );
    let stale = editor.session.delete_request();
    assert!(stale.is_err());
    let mut edit = Editor::new(fixture.owner.clone(), saved.id, Mode::Edit, String::new()).unwrap();
    fixture.ready(&mut edit);
    output = Output::default();
    edit.request_delete(&mut output);
    let command = output.commands.pop().unwrap();
    let old_load = edit.load.clone();
    assert!(old_load.is_none());
    drop(edit);
    let mut remount = fixture.editor();
    let late = fixture.runtime.block_on(command.execute(&fixture.services));
    assert!(remount.accept(late).is_none());
    assert!(
        theme_actions::theme_list(&fixture.state)
            .unwrap()
            .iter()
            .any(|theme| theme.name == "Edited copy")
    );
}

#[test]
fn native_theme_editor는_실제_입력_filter_reset_dialog와_save를_연결한다() {
    let mut fixture = Fixture::new();
    let mut editor = fixture.editor();
    fixture.ready(&mut editor);
    let first = fixture.frame(&mut editor, Vec::new()).0;
    assert!(first.error.is_none());
    assert!(target(&first, "save").max.x <= SCREEN[0]);
    assert!(target(&first, "search").max.x < SCREEN[0] - PREVIEW_WIDTH);
    editor.search = " keyword ".into();
    let filtered = fixture.frame(&mut editor, Vec::new()).0;
    assert!(filtered.error.is_none());
    assert!(
        filtered
            .traces
            .iter()
            .any(|trace| trace.0 == "bold-keyword")
    );
    assert!(
        !filtered
            .traces
            .iter()
            .any(|trace| trace.0 == "bold-storage")
    );
    let old = editor.draft.as_ref().unwrap().current().syntax["keyword"].clone();
    let clicked = fixture
        .frame(&mut editor, click(target(&filtered, "bold-keyword")))
        .0;
    assert!(clicked.error.is_none());
    assert_eq!(
        editor.draft.as_ref().unwrap().current().syntax["keyword"].bold,
        !old.bold
    );
    let changed = fixture.frame(&mut editor, Vec::new()).0;
    let reset = target(&changed, "reset-Syntax-keyword");
    fixture.frame(&mut editor, click(reset));
    assert_eq!(
        editor.draft.as_ref().unwrap().current().syntax["keyword"],
        old
    );
    editor
        .draft
        .as_mut()
        .unwrap()
        .rename("Renamed through draft".into());
    let changed = fixture.frame(&mut editor, Vec::new()).0;
    fixture.frame(&mut editor, click(target(&changed, "back")));
    let dialog = fixture.frame(&mut editor, Vec::new()).0;
    let canceled = fixture
        .frame(&mut editor, click(target(&dialog, "dialog-cancel")))
        .0;
    assert!(!canceled.close && editor.dialog.is_none());
    let normal = fixture.frame(&mut editor, Vec::new()).0;
    let saved = fixture.frame(&mut editor, click(target(&normal, "save"))).0;
    assert_eq!(saved.commands.len(), 1);
    assert!(!saved.close);
    assert!(editor.save.is_some());
    let reply = fixture.runtime.block_on(
        saved
            .commands
            .into_iter()
            .next()
            .unwrap()
            .execute(&fixture.services),
    );
    assert!(editor.accept(reply).unwrap().close);
    editor.search = "editor".into();
    let filtered = fixture.frame(&mut editor, Vec::new()).0;
    assert!(filtered.error.is_none());
    assert!(
        !filtered
            .traces
            .iter()
            .any(|trace| trace.0.starts_with("bold-"))
    );
}

const REMOTE_CONCURRENT: usize = 128;

struct DeniedPlatform;

impl taide_runtime::PlatformServices for DeniedPlatform {
    fn open_path(&self, _: &std::path::Path) -> AppResult<()> {
        Err(AppError::Forbidden(
            "theme fixture cannot open OS paths".into(),
        ))
    }
    fn reveal_item_in_dir(&self, _: &std::path::Path) -> AppResult<()> {
        Err(AppError::Forbidden(
            "theme fixture cannot reveal OS paths".into(),
        ))
    }
    fn open_url(&self, _: &str) -> AppResult<()> {
        Err(AppError::Forbidden(
            "theme fixture cannot open OS URLs".into(),
        ))
    }
    fn send_notification(&self, _: &str, _: &str) -> AppResult<()> {
        Err(AppError::Forbidden(
            "theme fixture cannot notify the OS".into(),
        ))
    }
}

fn test_services(
    state: AppState,
    tasks: TaskSupervisor,
    events: Arc<dyn taide_runtime::EventSink>,
) -> Arc<taide_runtime::AppServices> {
    Arc::new(taide_runtime::AppServices::new(
        state,
        tasks,
        taide_runtime::RemoteDispatchLimiter::new(REMOTE_CONCURRENT),
        taide_runtime::PlatformServicesState::new(Arc::new(DeniedPlatform)),
        taide_infra::secret::SecretStoreState::new("taide-shared-theme-editor-test".into()),
        taide_runtime::IdeSaveFile(taide_runtime::save_file_within_open_projects),
        events,
    ))
}

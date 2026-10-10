use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Color32, Event, FontId, Key, Modifiers, RawInput, Rect};
use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::paths::AppPaths;
use taide_model::snippet::{SnippetEntry, SnippetFile, SnippetStringOrList};
use taide_native_editor::completion::Command;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::language_configuration::{Language, LineSyntax};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::{Token, TokenKind};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_completion::Provider as CompletionProvider;
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorPresentation, EditorRequest, NativeEditor,
};
use taide_runtime::{AppServices, AppState, EventSink, TaskSupervisor, theme_actions};

use super::{Consumer, Provider, State};

const BYTE_LIMIT: usize = 4096;
const OWNER_LIMIT: usize = 4;
const UNDO_LIMIT: usize = 8;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const SCREEN: egui::Vec2 = egui::vec2(800.0, 600.0);
const FRAME_STEP: f64 = 0.1;
const CLIPBOARD_DEADLINE: Duration = Duration::from_secs(5);

#[test]
fn native_completion_clipboard는_값이_준비되기_전_수락가능한_목록을_노출하지_않는다() {
    let mut files = snippets().to_vec();
    files[0].snippets.get_mut("Mirrored").unwrap().body =
        SnippetStringOrList::Single("${CLIPBOARD:empty}$0".into());
    let mut fixture = Fixture::new("con\ncon", 7, Arc::from(files));
    fixture
        .store
        .set_view_state(
            fixture.view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 3, head: 3 },
                    Selection { anchor: 7, head: 7 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    fixture.frame(Vec::new(), &[Command::Trigger]);
    assert!(fixture.state.pending(&fixture.store, fixture.view));
    assert!(
        fixture
            .state
            .model(&fixture.store, fixture.view)
            .unwrap()
            .is_none()
    );
    assert_eq!(fixture.text(fixture.document), "con\ncon");
    let requests = fixture.state.take_clipboard_requests(&fixture.store);
    assert_eq!(requests.len(), 1);
    assert!(
        fixture
            .state
            .take_clipboard_requests(&fixture.store)
            .is_empty()
    );
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let captured = reads.clone();
    let mut bridge = crate::host::HostBridge::connect_with_clipboard_ports(
        fixture.services.clone(),
        Arc::new(|| {}),
        Arc::new(|_| panic!("unexpected clipboard write")),
        Arc::new(move || {
            captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("first\r\n \r\nsecond".into())
        }),
        None,
    )
    .unwrap();
    bridge
        .submit(crate::host::HostCommand::ReadCompletionClipboard(
            requests[0].clone(),
        ))
        .unwrap();
    let crate::host::HostReply::CompletionClipboard { request, result } =
        fixture.receive(&mut bridge)
    else {
        panic!("expected completion clipboard reply");
    };
    assert!(
        fixture
            .state
            .accept_clipboard(&fixture.store, &request, result.unwrap())
    );
    fixture.frame(Vec::new(), &[]);
    assert!(!fixture.state.pending(&fixture.store, fixture.view));
    assert!(fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "first\nsecond");
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .primary,
        1
    );
    assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 1);
    fixture.finish_bridge(bridge);
}

#[test]
fn native_completion_clipboard는_재입력의_늦은응답과_닫힌요청을_폐기하고_빈값을_해결한다() {
    let mut files = snippets().to_vec();
    files[0].snippets.get_mut("Mirrored").unwrap().prefix =
        SnippetStringOrList::Single("console".into());
    files[0].snippets.get_mut("Mirrored").unwrap().body =
        SnippetStringOrList::Single("${CLIPBOARD:empty}$0".into());
    let mut fixture = Fixture::new("con", 3, Arc::from(files));
    fixture.frame(Vec::new(), &[Command::Trigger]);
    let old = fixture
        .state
        .take_clipboard_requests(&fixture.store)
        .pop()
        .unwrap();
    fixture.frame(vec![Event::Text("s".into())], &[]);
    assert!(old.is_cancelled());
    assert!(
        !fixture
            .state
            .accept_clipboard(&fixture.store, &old, "late".into())
    );
    let current = fixture
        .state
        .take_clipboard_requests(&fixture.store)
        .pop()
        .unwrap();
    assert!(
        fixture
            .state
            .accept_clipboard(&fixture.store, &current, String::new())
    );
    fixture.frame(Vec::new(), &[]);
    assert!(fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "empty");
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.state.close(fixture.view);
    assert!(
        !fixture
            .state
            .accept_clipboard(&fixture.store, &current, "late".into())
    );
    assert_eq!(fixture.text(fixture.document), "empty");
}

#[test]
fn native_completion_clipboard_host는_읽는중_취소와_이미취소한_요청을_실제포트에서_차단한다() {
    let mut files = snippets().to_vec();
    files[0].snippets.get_mut("Mirrored").unwrap().body =
        SnippetStringOrList::Single("$CLIPBOARD".into());
    let mut fixture = Fixture::new("con", 3, Arc::from(files));
    fixture.frame(Vec::new(), &[Command::Trigger]);
    let request = fixture
        .state
        .take_clipboard_requests(&fixture.store)
        .pop()
        .unwrap();
    let (started, ready) = std::sync::mpsc::channel();
    let (release, gate) = std::sync::mpsc::channel();
    let gate = std::sync::Mutex::new(gate);
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let captured = reads.clone();
    let mut bridge = crate::host::HostBridge::connect_with_clipboard_ports(
        fixture.services.clone(),
        Arc::new(|| {}),
        Arc::new(|_| panic!("unexpected clipboard write")),
        Arc::new(move || {
            captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            started.send(()).unwrap();
            gate.lock()
                .unwrap()
                .recv_timeout(CLIPBOARD_DEADLINE)
                .unwrap();
            Ok("late".into())
        }),
        None,
    )
    .unwrap();
    bridge
        .submit(crate::host::HostCommand::ReadCompletionClipboard(
            request.clone(),
        ))
        .unwrap();
    ready.recv_timeout(CLIPBOARD_DEADLINE).unwrap();
    fixture.state.close(fixture.view);
    release.send(()).unwrap();
    let crate::host::HostReply::CompletionClipboard { result, .. } = fixture.receive(&mut bridge)
    else {
        panic!("expected completion clipboard cancellation");
    };
    assert!(result.is_err());
    bridge
        .submit(crate::host::HostCommand::ReadCompletionClipboard(request))
        .unwrap();
    let crate::host::HostReply::CompletionClipboard { result, .. } = fixture.receive(&mut bridge)
    else {
        panic!("expected cancelled completion clipboard reply");
    };
    assert!(result.is_err());
    assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(fixture.text(fixture.document), "con");
    fixture.finish_bridge(bridge);
}

struct Events;
impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

struct Syntax(TokenKind);
impl LineSyntax for Syntax {
    fn tokens(&self, _document: &DocumentSnapshot, _line: usize) -> Option<Vec<Token>> {
        Some(vec![Token {
            start_byte: 0,
            kind: self.0,
        }])
    }
    fn kind_if_inserting(
        &self,
        _document: &DocumentSnapshot,
        _line: usize,
        _byte: usize,
        _character: char,
    ) -> TokenKind {
        self.0
    }
}

struct Fixture {
    directory: PathBuf,
    _runtime: tokio::runtime::Runtime,
    services: Arc<AppServices>,
    context: egui::Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    owner: ViewId,
    project: ProjectId,
    state: State,
    editor: NativeEditor,
    presentation: EditorPresentation,
    files: Arc<[SnippetFile]>,
    syntax: Syntax,
    time: f64,
    focus: bool,
    keymaps: crate::terminal_surface::Views,
    overrides: Option<String>,
    actions: Vec<String>,
    painted: Vec<String>,
    geometry: taide_native_ui::editor_completion::Geometry,
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn snippets() -> Arc<[SnippetFile]> {
    Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([(
            "Mirrored".into(),
            SnippetEntry {
                prefix: SnippetStringOrList::Single("con".into()),
                body: SnippetStringOrList::Single("${1:한}$1 ${2:next}$0".into()),
                description: Some(SnippetStringOrList::Single("**실제 후보 문서**".into())),
                scope: None,
            },
        )]),
    }])
}

impl Fixture {
    fn receive(&self, bridge: &mut crate::host::HostBridge) -> crate::host::HostReply {
        self._runtime.block_on(async {
            tokio::time::timeout(CLIPBOARD_DEADLINE, async {
                loop {
                    if let Some(reply) = bridge.poll() {
                        return reply;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap()
        })
    }

    fn finish_bridge(&self, bridge: crate::host::HostBridge) {
        self._runtime.block_on(async {
            tokio::time::timeout(CLIPBOARD_DEADLINE, bridge.disconnect())
                .await
                .unwrap()
                .unwrap();
            tokio::time::timeout(CLIPBOARD_DEADLINE, self.services.tasks.shutdown())
                .await
                .unwrap();
        });
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }

    fn new(text: &str, byte: usize, files: Arc<[SnippetFile]>) -> Self {
        let project = ProjectId::new();
        let directory =
            std::env::temp_dir().join(format!("taide-batch22-completion-consumer-{project}"));
        std::fs::create_dir_all(&directory).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let app = AppState::new(AppPaths::new(directory.join("data")));
        let theme = theme_actions::theme_get(&app, "taide-dark".into()).unwrap();
        let services = crate::bootstrap::services(
            app,
            TaskSupervisor::new(runtime.handle().clone()),
            Arc::new(Events),
        );
        let context = egui::Context::default();
        context.set_os(egui::os::OperatingSystem::Mac);
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(
            egui::FontFamily::Name(taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into()),
            fonts.families[&egui::FontFamily::Monospace].clone(),
        );
        context.set_fonts(fonts);
        let mut store = EditorStore::new(EditorLimits {
            max_documents: OWNER_LIMIT,
            max_views: OWNER_LIMIT,
            max_undo_groups: UNDO_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), text, "rust".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "synthetic".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: byte,
                        head: byte,
                    }],
                },
                ScrollPosition::default(),
                Vec::new(),
            )
            .unwrap();
        let mut presentation = EditorPresentation::default();
        presentation.options.completion_colors =
            Some(taide_native_ui::presentation::editor_completion_colors(&theme).unwrap());
        Self {
            directory,
            _runtime: runtime,
            services,
            context,
            store,
            document,
            view,
            owner: view,
            project,
            state: State::default(),
            editor: NativeEditor {
                appearance: EditorAppearance {
                    font: FontId::monospace(FONT_SIZE),
                    line_height: LINE_HEIGHT,
                    horizontal_padding: 0.0,
                    background: Color32::BLACK,
                    foreground: Color32::WHITE,
                    muted: Color32::GRAY,
                    selection: Color32::BLUE,
                    cursor: Color32::WHITE,
                    current_line: Color32::TRANSPARENT,
                    line_numbers: false,
                    indent: "    ".into(),
                },
            },
            presentation,
            files,
            syntax: Syntax(TokenKind::Other),
            time: 0.0,
            focus: true,
            keymaps: crate::terminal_surface::Views::default(),
            overrides: None,
            actions: Vec::new(),
            painted: Vec::new(),
            geometry: Default::default(),
        }
    }

    fn frame(&mut self, events: Vec<Event>, commands: &[Command]) -> bool {
        self.time += FRAME_STEP;
        self.state.poll_supply(&self.store);
        self.keymaps
            .set_command_context(crate::command_registry::CommandContext {
                active_project: Some(self.project.clone()),
                active_editor_actions: crate::command_dispatch::active_editor_actions(
                    &self.store,
                    Some(&self.store.views().get(self.view).unwrap().key),
                ),
                ..Default::default()
            });
        let mut changed = false;
        let mut drawing = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                let mut host = Vec::new();
                let consumer = Consumer {
                    state: std::rc::Rc::new(std::cell::RefCell::new(&mut self.state)),
                    services: &self.services,
                    files: self.files.clone(),
                    context: self.context.clone(),
                };
                let mut provider = Provider {
                    consumer: consumer.clone(),
                    project: Some(self.project.clone()),
                    owner: self.owner,
                    owner_key: self.store.views().get(self.owner).unwrap().key.clone(),
                    lsp: None,
                    viewport: self.context.viewport_id(),
                    commands: &mut host,
                    editor: &self.editor,
                    syntax: &self.syntax,
                    model_path: "/synthetic/main.rs".into(),
                    language: "rust".into(),
                };
                assert_eq!(provider.is_embedded(self.view), self.view != self.owner);
                let language = Language {
                    rules: taide_native_syntax::monaco_language("rust")
                        .unwrap()
                        .unwrap(),
                    syntax: &self.syntax,
                };
                let mut next = 0;
                let output = self
                    .editor
                    .show_request(
                        ui,
                        &mut self.store,
                        self.view,
                        EditorRequest {
                            request_focus: self.focus,
                            keymap: |ui: &egui::Ui, event: &Event, composing: bool| {
                                let index = crate::keymap::event_index(ui.ctx(), event, &mut next);
                                self.keymaps
                                    .route_editor_keymap(
                                        crate::keymap::Route {
                                            context: ui.ctx(),
                                            event,
                                            index,
                                            scope: crate::keymap::Context {
                                                terminal: false,
                                                editor: true,
                                            },
                                            composing,
                                            overrides: self.overrides.as_deref(),
                                        },
                                        &mut self.actions,
                                        true,
                                        &consumer,
                                        self.view,
                                    )
                                    .unwrap()
                            },
                            route: |response: &egui::Response| {
                                response.ctx.keyboard_input_route(response.id)
                            },
                            presentation: &self.presentation,
                            tokens: |_: &EditorStore| None,
                            language: Some(language),
                            decorations: &[],
                            fold_commands: &[],
                            fold_controls: None,
                            problems: None,
                            locations: None,
                            syntax_folds: None,
                            documentation: None,
                            documentation_commands: &[],
                            completion: Some(&mut provider),
                            completion_commands: commands,
                        },
                    )
                    .unwrap();
                assert!(output.errors.is_empty(), "{:?}", output.errors);
                changed = output.changed;
                self.geometry = output.completion_geometry;
                self.focus = false;
            },
        );
        self.painted = drawing
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
                _ => None,
            })
            .collect();
        drawing.textures_delta.clear();
        changed
    }

    fn text(&self, document: DocumentId) -> String {
        self.store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.clear();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn native_completion_consumer는_실제snippet공급과_같은프레임_수락_기본입력_tabstop을_보존한다() {
    let mut fixture = Fixture::new("con", 3, snippets());
    assert!(!fixture.frame(Vec::new(), &[Command::Trigger]));
    assert_eq!(
        fixture
            .state
            .model(&fixture.store, fixture.view)
            .unwrap()
            .unwrap()
            .borrow()
            .len(),
        1
    );
    let changed = fixture.frame(
        vec![
            key(Key::Enter, Modifiers::NONE),
            Event::Text("\u{1f600}ab".into()),
            key(Key::Tab, Modifiers::NONE),
        ],
        &[],
    );
    assert!(
        changed,
        "text {:?}, focus {:?}, entries {}, sessions {}",
        fixture.text(fixture.document),
        fixture.context.memory(|memory| memory.focused()),
        fixture.state.entries.len(),
        fixture.state.snippet_sessions.len()
    );
    assert_eq!(
        fixture.text(fixture.document),
        "\u{1f600}ab\u{1f600}ab next"
    );
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 13,
            head: 17
        }]
    );
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
    assert!(fixture.frame(
        vec![Event::Text("done".into()), key(Key::Tab, Modifiers::SHIFT)],
        &[]
    ));
    assert_eq!(
        fixture.text(fixture.document),
        "\u{1f600}ab\u{1f600}ab done"
    );
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [
            Selection { anchor: 0, head: 6 },
            Selection {
                anchor: 6,
                head: 12
            }
        ]
    );
    fixture.frame(vec![key(Key::Escape, Modifiers::NONE)], &[]);
    assert!(!fixture.state.snippet_sessions.contains_key(&fixture.view));
}

#[test]
fn native_completion_preview는_실제snippet과_다중커서에서_표시_취소_수락을_원문_변경_없이_소비한다()
{
    let mut files = snippets().to_vec();
    files[0].snippets.get_mut("Mirrored").unwrap().body =
        SnippetStringOrList::Single("console(${1:value})$0".into());
    let mut fixture = Fixture::new("con\ncon", 7, Arc::from(files));
    fixture
        .store
        .set_view_state(
            fixture.view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 3, head: 3 },
                    Selection { anchor: 7, head: 7 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    fixture.presentation.options.suggest_preview = true;
    assert!(!fixture.frame(Vec::new(), &[Command::Trigger]));
    assert_eq!(fixture.text(fixture.document), "con\ncon");
    assert_eq!(
        fixture
            .painted
            .iter()
            .filter(|text| *text == "console(value)")
            .count(),
        2
    );
    assert!(
        fixture
            .state
            .entries
            .get(&fixture.view)
            .unwrap()
            .preview
            .is_some()
    );
    fixture.frame(vec![key(Key::Escape, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "con\ncon");
    assert!(!fixture.painted.iter().any(|text| text == "console(value)"));
    assert!(fixture.state.entries.is_empty());
    fixture.frame(Vec::new(), &[Command::Trigger]);
    assert!(fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]));
    assert_eq!(
        fixture.text(fixture.document),
        "console(value)\nconsole(value)"
    );
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
    assert!(fixture.state.entries.is_empty());
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .primary,
        1
    );
}

#[test]
fn native_completion_consumer의_peek는_실제소스만_편집하고_owner회수에서_세션을_닫는다() {
    let mut fixture = Fixture::new("con", 3, snippets());
    let owner_document = fixture
        .store
        .open_untitled(TabId::new(), "owner", "rust".into())
        .unwrap();
    fixture.owner = fixture
        .store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            owner_document,
        )
        .unwrap();
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "한한 next");
    assert_eq!(fixture.text(owner_document), "owner");
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
    fixture.store.detach_view(fixture.owner).unwrap();
    fixture.state.reconcile(
        &fixture.store,
        &std::collections::HashSet::from([fixture.project.clone()]),
        |_| true,
        |_, _| std::collections::HashSet::new(),
    );
    assert!(fixture.state.snippet_sessions.is_empty());
    assert_eq!(fixture.text(fixture.document), "한한 next");
}

#[test]
fn native_completion_consumer의_자동요청은_실제단어_숫자_토큰_커서조건을_따른다() {
    for (text, byte, kind, expected) in [
        ("console", 7, TokenKind::Other, true),
        ("conTail", 1, TokenKind::Other, true),
        ("conTail", 3, TokenKind::Other, false),
        ("한Tail", 3, TokenKind::Other, true),
        ("\u{1f600}Tail", 4, TokenKind::Other, false),
        ("1e3", 3, TokenKind::Other, false),
        ("\"con\"", 4, TokenKind::String, false),
        ("//con", 5, TokenKind::Comment, false),
    ] {
        let mut fixture = Fixture::new(text, byte, Arc::from([]));
        fixture.syntax = Syntax(kind);
        let mut host = Vec::new();
        let provider = Provider {
            consumer: Consumer {
                state: std::rc::Rc::new(std::cell::RefCell::new(&mut fixture.state)),
                services: &fixture.services,
                files: fixture.files.clone(),
                context: fixture.context.clone(),
            },
            project: Some(fixture.project.clone()),
            owner: fixture.owner,
            owner_key: fixture
                .store
                .views()
                .get(fixture.owner)
                .unwrap()
                .key
                .clone(),
            lsp: None,
            viewport: fixture.context.viewport_id(),
            commands: &mut host,
            editor: &fixture.editor,
            syntax: &fixture.syntax,
            model_path: "/synthetic/main.rs".into(),
            language: "rust".into(),
        };
        assert_eq!(
            provider.should_auto_trigger(&fixture.store, fixture.view),
            expected,
            "{text:?} at {byte}"
        );
    }
}

#[test]
fn native_completion_consumer의_실제키맵은_재정의_우선순위와_같은프레임_수락을_보존한다() {
    let mut fixture = Fixture::new("c", 1, snippets());
    fixture.overrides = Some(r#"[{"actionId":"monaco.acceptSelectedSuggestion","key":"r","mods":["mod"]},{"actionId":"monaco.editor.action.triggerSuggest","key":"q","mods":["mod"]}]"#.into());
    let command = Modifiers::MAC_CMD | Modifiers::COMMAND;
    fixture.frame(Vec::new(), &[]);
    fixture.frame(vec![key(Key::Q, command)], &[]);
    assert!(fixture.state.entries.contains_key(&fixture.view));
    assert!(fixture.frame(
        vec![
            Event::Text("o".into()),
            key(Key::R, command),
            Event::Text("\u{1f600}".into()),
            key(Key::Tab, Modifiers::NONE)
        ],
        &[]
    ));
    assert_eq!(fixture.text(fixture.document), "\u{1f600}\u{1f600} next");
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 9,
            head: 13
        }]
    );
    assert!(fixture.actions.is_empty());

    let mut fixture = Fixture::new("con", 3, snippets());
    fixture.overrides = Some(
        r#"[{"actionId":"monaco.editor.action.copyLinesDownAction","key":"Tab","mods":[]}]"#.into(),
    );
    fixture.frame(Vec::new(), &[Command::Trigger]);
    assert!(!fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "con");
    assert_eq!(
        fixture.actions,
        ["monaco.editor.action.copyLinesDownAction"]
    );
    assert!(fixture.state.entries.contains_key(&fixture.view));
}

#[test]
fn native_completion_consumer는_문서없는_스니펫의_상세와_실제_크기초기화_명령을_소비한다() {
    const RESIZE_DISTANCE: f32 = 100.0;
    let mut files = snippets().to_vec();
    for file in &mut files {
        for snippet in file.snippets.values_mut() {
            snippet.description = None;
        }
    }
    let mut fixture = Fixture::new("con", 3, Arc::from(files));
    fixture.overrides = Some(
        r#"[{"actionId":"monaco.editor.action.resetSuggestSize","key":"r","mods":["mod"]}]"#.into(),
    );
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(Vec::new(), &[]);
    let original = fixture.geometry.list.unwrap();
    fixture.frame(Vec::new(), &[Command::ToggleDetails]);
    fixture.frame(Vec::new(), &[]);
    assert!(fixture.geometry.details.is_some());
    assert!(fixture.painted.iter().any(|text| text == "Mirrored"));
    let start = original.right_center();
    let end = start + egui::vec2(RESIZE_DISTANCE, 0.0);
    fixture.frame(
        vec![
            Event::PointerMoved(start),
            Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &[],
    );
    fixture.frame(vec![Event::PointerMoved(end)], &[]);
    fixture.frame(
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    fixture.frame(Vec::new(), &[]);
    assert!(
        (fixture.geometry.list.unwrap().width() - original.width() - RESIZE_DISTANCE).abs() < 1.0
    );
    assert!(!fixture.frame(
        vec![key(Key::R, Modifiers::MAC_CMD | Modifiers::COMMAND)],
        &[]
    ));
    fixture.frame(Vec::new(), &[]);
    assert!((fixture.geometry.list.unwrap().width() - original.width()).abs() < 1.0);
    assert!(fixture.actions.is_empty());
    assert_eq!(fixture.text(fixture.document), "con");
    fixture.frame(Vec::new(), &[Command::Hide]);
    fixture.frame(
        vec![key(Key::R, Modifiers::MAC_CMD | Modifiers::COMMAND)],
        &[],
    );
    assert!(fixture.state.commands.is_empty());
    assert_eq!(fixture.text(fixture.document), "con");
}

#[test]
fn native_completion_consumer의_choice는_원본순서_목록과_mirror_수락_다음tabstop을_소비한다() {
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([(
            "Choice".into(),
            SnippetEntry {
                prefix: SnippetStringOrList::Single("con".into()),
                body: SnippetStringOrList::Single("${1|red,green,blue|} $1 ${2:next}$0".into()),
                description: None,
                scope: None,
            },
        )]),
    }]);
    let mut fixture = Fixture::new("con", 3, files);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    assert!(fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "red red next");
    let model = fixture
        .state
        .model(&fixture.store, fixture.view)
        .unwrap()
        .expect("choice popup");
    assert_eq!(model.borrow().len(), 3);
    assert!(fixture.frame(
        vec![
            key(Key::ArrowDown, Modifiers::NONE),
            key(Key::Enter, Modifiers::NONE)
        ],
        &[]
    ));
    assert_eq!(fixture.text(fixture.document), "green green next");
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 12,
            head: 16
        }]
    );
    fixture.frame(vec![key(Key::Tab, Modifiers::SHIFT)], &[]);
    assert_eq!(
        fixture
            .state
            .model(&fixture.store, fixture.view)
            .unwrap()
            .unwrap()
            .borrow()
            .len(),
        3
    );
    fixture.frame(Vec::new(), &[Command::Last]);
    assert!(fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "blue blue next");
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 10,
            head: 14
        }]
    );
    fixture.frame(vec![key(Key::Escape, Modifiers::NONE)], &[]);
    assert!(fixture.state.snippet_sessions.is_empty());
}

#[test]
fn native_completion_consumer의_choice는_다중커서의_primary와_입력후_범위대체를_보존한다() {
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([(
            "Choice".into(),
            SnippetEntry {
                prefix: SnippetStringOrList::Single("con".into()),
                body: SnippetStringOrList::Single("${1|red,green,blue|} $1 ${2:next}$0".into()),
                description: None,
                scope: None,
            },
        )]),
    }]);
    let mut fixture = Fixture::new("con\ncon", 7, files);
    fixture
        .store
        .set_view_state(
            fixture.view,
            SelectionSet {
                primary: 1,
                selections: vec![
                    Selection { anchor: 3, head: 3 },
                    Selection { anchor: 7, head: 7 },
                ],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(
        fixture
            .state
            .model(&fixture.store, fixture.view)
            .unwrap()
            .unwrap()
            .borrow()
            .len(),
        3
    );
    assert!(fixture.frame(
        vec![Event::Text("g".into()), key(Key::Enter, Modifiers::NONE)],
        &[]
    ));
    assert_eq!(
        fixture.text(fixture.document),
        "green green next\ngreen green next"
    );
    let selection = &fixture.store.views().get(fixture.view).unwrap().selection;
    assert_eq!(selection.primary, 1);
    assert_eq!(
        selection.selections,
        [
            Selection {
                anchor: 12,
                head: 16
            },
            Selection {
                anchor: 29,
                head: 33
            }
        ]
    );
}

#[test]
fn native_completion_consumer의_choice_용량거절은_문서와_부분입력_선택을_보존한다() {
    let large = "x".repeat(BYTE_LIMIT + 1);
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([(
            "Choice".into(),
            SnippetEntry {
                prefix: SnippetStringOrList::Single("con".into()),
                body: SnippetStringOrList::Single(format!("${{1|a,{large}|}} $1 $0")),
                description: None,
                scope: None,
            },
        )]),
    }]);
    let mut fixture = Fixture::new("con", 3, files);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    fixture.frame(vec![Event::Text("a".into())], &[]);
    let before = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    let selection = fixture
        .store
        .views()
        .get(fixture.view)
        .unwrap()
        .selection
        .clone();
    let token = fixture
        .state
        .display(&fixture.store, fixture.view)
        .unwrap()
        .0
        .to_string();
    let mut host = Vec::new();
    let mut provider = Provider {
        consumer: Consumer {
            state: std::rc::Rc::new(std::cell::RefCell::new(&mut fixture.state)),
            services: &fixture.services,
            files: fixture.files.clone(),
            context: fixture.context.clone(),
        },
        project: Some(fixture.project.clone()),
        owner: fixture.owner,
        owner_key: fixture
            .store
            .views()
            .get(fixture.owner)
            .unwrap()
            .key
            .clone(),
        lsp: None,
        viewport: fixture.context.viewport_id(),
        commands: &mut host,
        editor: &fixture.editor,
        syntax: &fixture.syntax,
        model_path: "/synthetic/main.rs".into(),
        language: "rust".into(),
    };
    assert!(matches!(
        provider.accept(&mut fixture.store, fixture.view, &token, 1, false),
        Err(taide_native_editor::document::EditorError::Capacity)
    ));
    drop(provider);
    let after = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    assert_eq!(after.rope.to_string(), before.rope.to_string());
    assert_eq!(after.revision, before.revision);
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().selection,
        selection
    );
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
}

#[test]
fn native_completion_consumer의_choice는_밖의편집으로_세션이_끝나면_후보도_회수한다() {
    let files: Arc<[SnippetFile]> = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([(
            "Choice".into(),
            SnippetEntry {
                prefix: SnippetStringOrList::Single("con".into()),
                body: SnippetStringOrList::Single("${1|red,green|} $1 ${2:next}$0".into()),
                description: None,
                scope: None,
            },
        )]),
    }]);
    for reconcile in [true, false] {
        let mut fixture = Fixture::new("con", 3, files.clone());
        fixture.frame(Vec::new(), &[Command::Trigger]);
        fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
        assert!(fixture.state.entries.contains_key(&fixture.view));
        let snapshot = fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap();
        let end = snapshot.rope.len_bytes();
        fixture
            .store
            .apply(
                fixture.document,
                taide_native_editor::store::Transaction {
                    revision: snapshot.revision,
                    edits: vec![taide_native_editor::document::Edit {
                        bytes: end..end,
                        text: "!".into(),
                    }],
                    selection_after: None,
                    group: taide_native_editor::document::UndoGroup(0),
                    origin: None,
                },
            )
            .unwrap();
        if reconcile {
            fixture.state.reconcile(
                &fixture.store,
                &std::collections::HashSet::from([fixture.project.clone()]),
                |_| true,
                |_, _| std::collections::HashSet::new(),
            );
        } else {
            fixture.frame(Vec::new(), &[]);
        }
        assert!(
            fixture.state.snippet_sessions.is_empty(),
            "reconcile {reconcile}"
        );
        assert!(fixture.state.entries.is_empty(), "reconcile {reconcile}");
        assert_eq!(fixture.text(fixture.document), "red red next!");
    }
}

#[test]
fn native_completion_nested는_안쪽mirror와_final을_거쳐_바깥tabstop으로_이동한다() {
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([
            (
                "Outer".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("out".into()),
                    body: SnippetStringOrList::Single("${1:con} $1 ${2:tail}$0".into()),
                    description: None,
                    scope: None,
                },
            ),
            (
                "Inner".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("con".into()),
                    body: SnippetStringOrList::Single("(${1:x}$1 ${2:y})$0".into()),
                    description: None,
                    scope: None,
                },
            ),
        ]),
    }]);
    let mut fixture = Fixture::new("out", 3, files);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "con con tail");
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "(xx y) (xx y) tail");
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections
            .len(),
        4
    );
    fixture.frame(
        vec![
            Event::Text("a".into()),
            key(Key::Tab, Modifiers::NONE),
            Event::Text("b".into()),
            key(Key::Tab, Modifiers::NONE),
        ],
        &[],
    );
    assert_eq!(fixture.text(fixture.document), "(aa b) (aa b) tail");
    assert!(
        fixture.state.snippet_sessions.contains_key(&fixture.view),
        "nested final must keep the outer session"
    );
    fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]);
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 14,
            head: 18
        }]
    );
    fixture.frame(
        vec![
            key(Key::Tab, Modifiers::SHIFT),
            key(Key::Tab, Modifiers::SHIFT),
            key(Key::Tab, Modifiers::SHIFT),
        ],
        &[],
    );
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections
            .len(),
        4
    );
    fixture.frame(vec![Event::Text("\u{1f600}".into())], &[]);
    assert_eq!(
        fixture.text(fixture.document),
        "(\u{1f600}\u{1f600} b) (\u{1f600}\u{1f600} b) tail"
    );
}

#[test]
fn native_completion_nested의_일반텍스트_수락은_바깥세션과_다음tabstop을_유지한다() {
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([
            (
                "Outer".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("out".into()),
                    body: SnippetStringOrList::Single("${1:con} $1 ${2:tail}$0".into()),
                    description: None,
                    scope: None,
                },
            ),
            (
                "Plain".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("con".into()),
                    body: SnippetStringOrList::Single("plain".into()),
                    description: None,
                    scope: None,
                },
            ),
        ]),
    }]);
    let mut fixture = Fixture::new("out", 3, files);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "plain plain tail");
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
    fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]);
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 12,
            head: 16
        }]
    );
}

#[test]
fn native_completion_nested의_새변환과_기존변환은_같은세션에서_독립평가한다() {
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([
            (
                "Outer".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("out".into()),
                    body: SnippetStringOrList::Single(
                        "${1:con} ${2:tail}${2/(\\w+)/${1:/upcase}/}$0".into(),
                    ),
                    description: None,
                    scope: None,
                },
            ),
            (
                "Inner".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("con".into()),
                    body: SnippetStringOrList::Single("${1:in}${1/(.*)/${1:/upcase}/}$0".into()),
                    description: None,
                    scope: None,
                },
            ),
        ]),
    }]);
    let mut fixture = Fixture::new("out", 3, files);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "inin tailtail");
    fixture.frame(
        vec![Event::Text("yo".into()), key(Key::Tab, Modifiers::NONE)],
        &[],
    );
    assert_eq!(fixture.text(fixture.document), "yoYO tailtail");
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
    fixture.frame(
        vec![
            key(Key::Tab, Modifiers::NONE),
            Event::Text("end".into()),
            key(Key::Tab, Modifiers::NONE),
        ],
        &[],
    );
    assert_eq!(fixture.text(fixture.document), "yoYO endEND");
    assert!(fixture.state.snippet_sessions.is_empty());
}

#[test]
fn native_completion_nested의_선택지수락은_안쪽final과_바깥다음위치를_유지한다() {
    let files = Arc::from([SnippetFile {
        file_name: "rust.json".into(),
        snippets: BTreeMap::from([
            (
                "Outer".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("out".into()),
                    body: SnippetStringOrList::Single("${1:con} ${2:tail}$0".into()),
                    description: None,
                    scope: None,
                },
            ),
            (
                "Inner".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("con".into()),
                    body: SnippetStringOrList::Single("${1|red,green|} $1 ${2:x}$0".into()),
                    description: None,
                    scope: None,
                },
            ),
        ]),
    }]);
    let mut fixture = Fixture::new("out", 3, files);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(fixture.document), "red red x tail");
    assert!(fixture.state.entries.contains_key(&fixture.view));
    fixture.frame(
        vec![
            key(Key::ArrowDown, Modifiers::NONE),
            key(Key::Enter, Modifiers::NONE),
        ],
        &[],
    );
    assert_eq!(fixture.text(fixture.document), "green green x tail");
    assert!(!fixture.state.entries.contains_key(&fixture.view));
    fixture.frame(
        vec![
            key(Key::Tab, Modifiers::NONE),
            key(Key::Tab, Modifiers::NONE),
        ],
        &[],
    );
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections,
        [Selection {
            anchor: 14,
            head: 18
        }]
    );
    assert!(fixture.state.snippet_sessions.contains_key(&fixture.view));
}

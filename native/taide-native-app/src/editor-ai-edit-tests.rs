use super::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::app_event::AppEvent;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::paths::AppPaths;
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::{Composition, ScrollPosition};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const DOCUMENT_BYTES: usize = 1024 * 1024;
const OWNER_LIMIT: usize = 4;
const DEADLINE: Duration = Duration::from_secs(5);
const SELECTED: &str = "old";

struct Events;
impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

struct Secrets;
impl SecretStore for Secrets {
    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        Ok(())
    }
    fn get(&self, _: SecretAccount) -> AppResult<Option<String>> {
        panic!("fixture must not read credentials")
    }
    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        Ok(())
    }
}

type Reply = (
    AiInlineEditRequest,
    oneshot::Sender<AppResult<AiTextResponse>>,
);

struct Fixture {
    directory: PathBuf,
    runtime: tokio::runtime::Runtime,
    services: AppServices,
    store: EditorStore,
    document: DocumentId,
    source: ViewId,
    owner: ViewId,
    project: ProjectId,
    state: State,
    context: egui::Context,
    receive: std::sync::mpsc::Receiver<Reply>,
}

impl Fixture {
    fn new(text: &str, selection: Selection) -> Self {
        let project = ProjectId::new();
        let directory = std::env::temp_dir().join(format!("taide-ai-edit-{project}"));
        std::fs::create_dir_all(&directory).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let mut services = crate::bootstrap::services(
            AppState::new(AppPaths::new(directory.join("data"))),
            TaskSupervisor::new(runtime.handle().clone()),
            Arc::new(Events),
        );
        Arc::get_mut(&mut services).unwrap().secrets =
            SecretStoreState::with_store(Arc::new(Secrets));
        let services = Arc::try_unwrap(services).ok().unwrap();
        services.state.settings.write().ai_provider = Some(AiProviderId::Omlx);
        services.state.settings.write().ai_model = Some("fixture".into());
        let mut store = EditorStore::new(EditorLimits {
            max_documents: OWNER_LIMIT,
            max_views: OWNER_LIMIT,
            max_undo_groups: OWNER_LIMIT,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let path = PathBuf::from("/synthetic/source.py");
        let document = store
            .open_file(
                path,
                OpenedFile {
                    path: "/synthetic/source.py".into(),
                    content: text.into(),
                    encoding_lossy: false,
                    language_id: "python".into(),
                    tier: FileSizeTier::Normal,
                    byte_size: u32::try_from(text.len()).unwrap(),
                    line_count: u32::try_from(text.lines().count()).unwrap(),
                    modified_ms: 0.0,
                    read_only: false,
                    editor_config: EditorConfigOptions::default(),
                },
            )
            .unwrap();
        let source = store
            .attach_view(
                ViewKey {
                    window: "source-window".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap();
        let owner_document = store
            .open_untitled(TabId::new(), "owner", "rust".into())
            .unwrap();
        let owner = store
            .attach_view(
                ViewKey {
                    window: "owner-window".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                owner_document,
            )
            .unwrap();
        store
            .set_view_state(
                source,
                SelectionSet {
                    primary: 0,
                    selections: vec![selection],
                },
                ScrollPosition::default(),
                Vec::new(),
            )
            .unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        let state = State::with_complete(Arc::new(move |services, request| {
            let send = send.clone();
            let requests = services.ai_requests.clone();
            Box::pin(async move {
                let (_token, _cancelled) =
                    requests.begin(&request.owner, &request.request_id).unwrap();
                let (reply, receive) = oneshot::channel();
                send.send((request, reply)).unwrap();
                receive
                    .await
                    .unwrap_or_else(|_| Err(AppError::Internal("fixture response closed".into())))
            })
        }));
        Self {
            directory,
            runtime,
            services,
            store,
            document,
            source,
            owner,
            project,
            state,
            context: egui::Context::default(),
            receive,
        }
    }

    fn open(&mut self) {
        self.state
            .open(
                &self.store,
                self.project.clone(),
                self.source,
                self.owner,
                "/synthetic/owner.rs".into(),
            )
            .unwrap();
    }

    fn submit(&mut self) -> Reply {
        self.state
            .session(&self.store, &self.services, self.source)
            .unwrap()
            .instruction = "  rename  ".into();
        assert!(
            self.state
                .submit(&self.store, &self.services, self.source, &self.context)
                .unwrap()
        );
        self.receive.recv_timeout(DEADLINE).unwrap()
    }

    fn resolve(&mut self, text: &str) {
        let (request, reply) = self.submit();
        reply
            .send(Ok(AiTextResponse {
                request_id: request.request_id,
                text: Some(text.into()),
            }))
            .unwrap();
        let deadline = Instant::now() + DEADLINE;
        loop {
            if self
                .state
                .poll(&self.store, &self.services, self.source)
                .unwrap()
                == Poll::Preview
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    fn text(&self) -> String {
        self.store
            .documents()
            .snapshot(self.document)
            .unwrap()
            .rope
            .to_string()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.state.clear();
        self.runtime.block_on(self.services.tasks.shutdown());
        assert_eq!(self.services.tasks.tracked_count(), 0);
        self.runtime.block_on(async {
            tokio::time::timeout(DEADLINE, self.services.ai_requests.wait_for_idle())
                .await
                .unwrap()
        });
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn ai_edit의_선택과_현재줄은_source_문맥을_보존하고_요청_전에는_원문을_바꾸지않는다() {
    let text = "before\r\nold\r\nafter";
    let start = text.find(SELECTED).unwrap();
    for selection in [
        Selection {
            anchor: start,
            head: start,
        },
        Selection {
            anchor: start + SELECTED.len(),
            head: start,
        },
    ] {
        let mut fixture = Fixture::new(text, selection);
        fixture.open();
        let (request, _reply) = fixture.submit();
        assert_eq!(request.selection, SELECTED);
        assert_eq!(request.prefix, "before\r\n");
        assert_eq!(request.suffix, "\r\nafter");
        assert_eq!(request.file_path, "/synthetic/source.py");
        assert_eq!(request.language, "python");
        assert_eq!(request.owner, "owner-window");
        assert_eq!(request.instruction, "rename");
        assert_eq!(request.provider, Some(AiProviderId::Omlx));
        assert_eq!(fixture.text(), text);
        assert!(
            fixture
                .state
                .session(&fixture.store, &fixture.services, fixture.source)
                .unwrap()
                .is_loading()
        );
    }
}

#[test]
fn ai_edit의_미리보기는_원문과_owner를_보존하고_수락은_한번에_되돌린다() {
    let mut fixture = Fixture::new(
        "old\r\nkeep",
        Selection {
            anchor: 0,
            head: SELECTED.len(),
        },
    );
    fixture.open();
    fixture.resolve("Here is code:\n```python\nnew\nnext\n```\nDone");
    let session = fixture
        .state
        .session(&fixture.store, &fixture.services, fixture.source)
        .unwrap();
    assert_eq!(session.preview(), Some("new\nnext"));
    assert_eq!(fixture.text(), "old\r\nkeep");
    assert!(
        fixture
            .state
            .accept(&mut fixture.store, &fixture.services, fixture.source)
            .unwrap()
    );
    assert_eq!(fixture.text(), "new\r\nnext\r\nkeep");
    let owner = fixture.store.views().get(fixture.owner).unwrap().document;
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(owner)
            .unwrap()
            .rope
            .to_string(),
        "owner"
    );
    assert!(fixture.store.undo(fixture.document).unwrap());
    assert_eq!(fixture.text(), "old\r\nkeep");
    assert!(!fixture.store.undo(fixture.document).unwrap());
}

#[test]
fn ai_edit의_닫힘은_진행요청을_회수하고_늦은응답과_삽입을_거절한다() {
    let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
    fixture.open();
    let (_request, reply) = fixture.submit();
    fixture.state.close(fixture.source);
    fixture.runtime.block_on(async {
        tokio::time::timeout(DEADLINE, fixture.services.ai_requests.wait_for_idle())
            .await
            .unwrap()
    });
    assert!(reply.is_closed());
    assert!(
        !fixture
            .state
            .accept(&mut fixture.store, &fixture.services, fixture.source)
            .unwrap()
    );
    assert_eq!(fixture.text(), SELECTED);
}

#[test]
fn ai_edit의_반복열기와_선택이동은_원래대상과_지시를_보존한다() {
    let mut fixture = Fixture::new("old\nother", Selection { anchor: 0, head: 0 });
    fixture.open();
    fixture
        .state
        .session(&fixture.store, &fixture.services, fixture.source)
        .unwrap()
        .instruction = "keep".into();
    let end = fixture.text().len();
    fixture
        .store
        .set_view_state(
            fixture.source,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: end,
                    head: end,
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    fixture.open();
    let session = fixture
        .state
        .session(&fixture.store, &fixture.services, fixture.source)
        .unwrap();
    assert_eq!(session.instruction, "keep");
    assert_eq!(session.origin.target, 0..SELECTED.len());
    assert!(
        fixture
            .state
            .submit(
                &fixture.store,
                &fixture.services,
                fixture.source,
                &fixture.context
            )
            .unwrap()
    );
}

#[test]
fn ai_edit의_문서_설정_credential_수명은_미리보기를_수락하지않는다() {
    for mutation in ["edit", "model", "credential", "owner", "ime"] {
        let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
        fixture.open();
        fixture.resolve("new");
        match mutation {
            "edit" => {
                taide_native_editor::editing::type_text(&mut fixture.store, fixture.source, "x")
                    .unwrap();
            }
            "model" => fixture.services.state.settings.write().ai_model = Some("other".into()),
            "credential" => fixture
                .services
                .secrets
                .0
                .delete(SecretAccount::AiOmlx)
                .unwrap(),
            "owner" => {
                fixture.store.detach_view(fixture.owner).unwrap();
            }
            "ime" => fixture
                .store
                .set_composition(
                    fixture.source,
                    Some(Composition {
                        preedit: "中".into(),
                        replace: 0..0,
                        revision: 0,
                    }),
                )
                .unwrap(),
            _ => unreachable!(),
        }
        assert!(
            !fixture
                .state
                .accept(&mut fixture.store, &fixture.services, fixture.source)
                .unwrap(),
            "{mutation}"
        );
        assert!(
            fixture
                .state
                .session(&fixture.store, &fixture.services, fixture.source)
                .is_none()
        );
        assert_ne!(fixture.text(), "new");
    }
}

#[test]
fn ai_edit의_문맥은_utf16_상한과_유효한_문자경계를_지킨다() {
    let text = format!(
        "{}한{}",
        "\u{1f600}".repeat(CONTEXT_UNITS),
        "\u{1f600}".repeat(CONTEXT_UNITS)
    );
    let start = text.find('한').unwrap();
    let fixture = Fixture::new(
        &text,
        Selection {
            anchor: start,
            head: start + '한'.len_utf8(),
        },
    );
    let snapshot = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    let (prefix, suffix) = context_window(&snapshot, &(start..start + '한'.len_utf8())).unwrap();
    assert_eq!(prefix.encode_utf16().count(), CONTEXT_UNITS);
    assert_eq!(suffix.encode_utf16().count(), CONTEXT_UNITS);
    assert!(context_window(&snapshot, &(start + 1..start + '한'.len_utf8())).is_err());
    assert!(context_window(&snapshot, &(text.len() + 1..text.len() + 1)).is_err());
}

#[test]
fn ai_edit의_fence_제거는_원본의_첫열기_마지막닫기와_공백을_보존한다() {
    for (input, expected) in [
        ("  plain  ", "plain"),
        ("```same```", "same"),
        ("before\n```rust\n code \r\n```\nafter", " code "),
        (
            "```rust\nouter\n```inner```\nlast\n```",
            "outer\n```inner```\nlast",
        ),
        ("```unterminated  ", "```unterminated"),
        ("```\na\n\n```", "a\n"),
        ("\u{feff}code\u{feff}", "code"),
        ("\u{0085}code\u{0085}", "\u{0085}code\u{0085}"),
    ] {
        assert_eq!(strip_code_fence(input), expected, "{input}");
    }
}

#[test]
fn ai_edit의_빈지시와_잘못된_응답은_삽입하지않는다() {
    for response in [None, Some(""), Some("```\n```"), Some("new")] {
        let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
        fixture.open();
        assert!(
            !fixture
                .state
                .submit(
                    &fixture.store,
                    &fixture.services,
                    fixture.source,
                    &fixture.context
                )
                .unwrap()
        );
        let (request, reply) = fixture.submit();
        let id = if response == Some("new") {
            "wrong".into()
        } else {
            request.request_id
        };
        reply
            .send(Ok(AiTextResponse {
                request_id: id,
                text: response.map(str::to_owned),
            }))
            .unwrap();
        let deadline = Instant::now() + DEADLINE;
        let result = loop {
            let result = fixture
                .state
                .poll(&fixture.store, &fixture.services, fixture.source);
            if !matches!(result, Ok(Poll::Waiting)) {
                break result;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        if response == Some("new") {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), Poll::Empty);
        }
        assert!(
            !fixture
                .state
                .accept(&mut fixture.store, &fixture.services, fixture.source)
                .unwrap()
        );
        assert_eq!(fixture.text(), SELECTED);
    }
}

#[test]
fn ai_edit의_읽기전용_등급과_선택_지시_상한은_요청을_거절한다() {
    let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
    fixture
        .store
        .observe_file(
            fixture.document,
            std::path::Path::new("/synthetic/source.py"),
            OpenedFile {
                path: "/synthetic/source.py".into(),
                content: SELECTED.into(),
                encoding_lossy: false,
                language_id: "python".into(),
                tier: FileSizeTier::ReadOnly,
                byte_size: u32::try_from(SELECTED.len()).unwrap(),
                line_count: 1,
                modified_ms: 0.0,
                read_only: true,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    assert_eq!(
        fixture.state.open(
            &fixture.store,
            fixture.project.clone(),
            fixture.source,
            fixture.owner,
            String::new()
        ),
        Err(EditorError::ReadOnly)
    );
    assert!(fixture.receive.try_recv().is_err());
    let text = "x".repeat(SELECTION_BYTES + 1);
    let mut fixture = Fixture::new(
        &text,
        Selection {
            anchor: 0,
            head: text.len(),
        },
    );
    assert_eq!(
        fixture.state.open(
            &fixture.store,
            fixture.project.clone(),
            fixture.source,
            fixture.owner,
            String::new()
        ),
        Err(EditorError::Capacity)
    );
    let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
    fixture.open();
    fixture
        .state
        .session(&fixture.store, &fixture.services, fixture.source)
        .unwrap()
        .instruction = "x".repeat(INSTRUCTION_BYTES + 1);
    assert!(
        fixture
            .state
            .submit(
                &fixture.store,
                &fixture.services,
                fixture.source,
                &fixture.context
            )
            .is_err()
    );
    assert!(fixture.receive.try_recv().is_err());
    assert_eq!(fixture.text(), SELECTED);
}

#[test]
fn ai_edit의_진행요청은_중복제출하지않고_비활성_프로젝트에서_회수한다() {
    let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
    fixture.open();
    let (_request, reply) = fixture.submit();
    assert!(
        !fixture
            .state
            .submit(
                &fixture.store,
                &fixture.services,
                fixture.source,
                &fixture.context
            )
            .unwrap()
    );
    let project = fixture.project.clone();
    fixture
        .state
        .reconcile(&fixture.store, &fixture.services, |origin| {
            origin.project == project
        });
    assert!(
        fixture
            .state
            .session(&fixture.store, &fixture.services, fixture.source)
            .is_some()
    );
    fixture
        .state
        .reconcile(&fixture.store, &fixture.services, |_| false);
    fixture.runtime.block_on(async {
        tokio::time::timeout(DEADLINE, fixture.services.ai_requests.wait_for_idle())
            .await
            .unwrap()
    });
    assert!(reply.is_closed());
    assert_eq!(fixture.text(), SELECTED);
}

#[test]
fn ai_edit의_진행요청은_credential과_문서변경에서_수락없이_회수한다() {
    for mutation in ["credential", "edit"] {
        let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
        fixture.open();
        let (_request, reply) = fixture.submit();
        if mutation == "credential" {
            fixture
                .services
                .secrets
                .0
                .delete(SecretAccount::AiOmlx)
                .unwrap();
        } else {
            taide_native_editor::editing::type_text(&mut fixture.store, fixture.source, "x")
                .unwrap();
        }
        fixture
            .state
            .reconcile(&fixture.store, &fixture.services, |_| true);
        fixture.runtime.block_on(async {
            tokio::time::timeout(DEADLINE, fixture.services.ai_requests.wait_for_idle())
                .await
                .unwrap()
        });
        assert!(reply.is_closed());
        assert!(
            fixture
                .state
                .session(&fixture.store, &fixture.services, fixture.source)
                .is_none()
        );
        assert_ne!(fixture.text(), "new");
    }
}

#[test]
fn ai_edit의_실패응답과_종료된_supervisor는_원문을_보존한다() {
    let mut fixture = Fixture::new(SELECTED, Selection { anchor: 0, head: 0 });
    fixture.open();
    let (_request, reply) = fixture.submit();
    reply
        .send(Err(AppError::Internal("synthetic failure".into())))
        .unwrap();
    let deadline = Instant::now() + DEADLINE;
    loop {
        if fixture
            .state
            .poll(&fixture.store, &fixture.services, fixture.source)
            .is_err()
        {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(
        fixture
            .state
            .session(&fixture.store, &fixture.services, fixture.source)
            .is_none()
    );
    fixture.open();
    fixture.services.tasks.stop_all();
    fixture
        .state
        .session(&fixture.store, &fixture.services, fixture.source)
        .unwrap()
        .instruction = "rename".into();
    assert!(
        fixture
            .state
            .submit(
                &fixture.store,
                &fixture.services,
                fixture.source,
                &fixture.context
            )
            .is_err()
    );
    assert_eq!(fixture.text(), SELECTED);
}

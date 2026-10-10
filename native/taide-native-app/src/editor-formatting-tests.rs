use super::*;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::indent::{IndentConfiguration, IndentationChange};
use taide_native_editor::lsp::{LspRange, Position, TextEdit, byte_to_position};
use taide_native_editor::store::{EditorLimits, Transaction};
use taide_native_editor::view::{Composition, ScrollPosition, Selection, SelectionSet};

const BYTE_LIMIT: usize = 4096;
const DOCUMENT_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;
const INDENT_SIZE: u32 = 4;
const DISPLAY_SIZE: u32 = 8;

struct Fixture {
    store: EditorStore,
    source: ViewId,
    owner: ViewId,
    document: taide_native_editor::document::DocumentId,
    project: ProjectId,
    provider: ProviderIdentity,
    state: State,
}

impl Fixture {
    fn new(text: &str, source_read_only: bool, owner_read_only: bool) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: DOCUMENT_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let open = |path: &str, content: &str, read_only| OpenedFile {
            path: path.into(),
            content: content.into(),
            encoding_lossy: false,
            language_id: "rust".into(),
            tier: FileSizeTier::Normal,
            byte_size: content.len().try_into().unwrap(),
            line_count: 1,
            modified_ms: 0.0,
            read_only,
            editor_config: EditorConfigOptions::default(),
        };
        let document = store
            .open_file(
                "/synthetic/source.rs".into(),
                open("/synthetic/source.rs", text, source_read_only),
            )
            .unwrap();
        let owner = store
            .open_file(
                "/synthetic/owner.rs".into(),
                open("/synthetic/owner.rs", "owner", owner_read_only),
            )
            .unwrap();
        let key = || ViewKey {
            window: "synthetic".into(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let source = store.attach_view(key(), document).unwrap();
        let owner = store.attach_view(key(), owner).unwrap();
        let configuration = IndentConfiguration {
            defaults: IndentOptions {
                tab_size: INDENT_SIZE,
                insert_spaces: true,
            },
            detect_indentation: false,
        };
        store
            .set_indentation(
                document,
                configuration,
                IndentationChange::UseSpaces(INDENT_SIZE),
            )
            .unwrap();
        store
            .set_indentation(
                document,
                configuration,
                IndentationChange::DisplaySize(DISPLAY_SIZE),
            )
            .unwrap();
        Self {
            store,
            source,
            owner,
            document,
            project: ProjectId::new(),
            provider: ProviderIdentity {
                owner: crate::diagnostics::Owner::new(),
                generation: 1,
                capability_revision: 0,
            },
            state: State::default(),
        }
    }

    fn providers(&self) -> HashSet<ProviderIdentity> {
        HashSet::from([self.provider])
    }

    fn begin(&mut self) -> Result<Option<Request>, EditorError> {
        self.state.begin(
            &self.store,
            self.project.clone(),
            self.source,
            self.owner,
            Command::Document,
            IndentOptions {
                tab_size: INDENT_SIZE,
                insert_spaces: true,
            },
            self.providers(),
        )
    }

    fn response(&self, request: &Request, text: &str) -> Result<Response, Failure> {
        Ok(Response {
            provider: self.provider,
            edits: vec![TextEdit::new(
                LspRange::new(
                    Position::default(),
                    byte_to_position(&request.snapshot, request.snapshot.rope.len_bytes()).unwrap(),
                ),
                text.into(),
            )],
        })
    }
}

#[test]
fn 포맷_요청은_readonly_owner의_peek도_현재_편집_폭으로_적용하고_owner와_undo를_보존한다() {
    let mut fixture = Fixture::new("let name=한;", false, true);
    let owner_document = fixture.store.views().get(fixture.owner).unwrap().document;
    let owner_before = fixture.store.documents().snapshot(owner_document).unwrap();
    let request = fixture.begin().unwrap().unwrap();
    assert_eq!(
        (request.options.tab_size, request.options.insert_spaces),
        (INDENT_SIZE, true)
    );
    let response = fixture.response(&request, "let name = 한;");
    let providers = fixture.providers();
    assert!(
        fixture
            .state
            .accept(&mut fixture.store, &request, providers, response)
            .unwrap()
    );
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap()
            .rope
            .to_string(),
        "let name = 한;"
    );
    let owner_after = fixture.store.documents().snapshot(owner_document).unwrap();
    assert_eq!(
        (owner_after.rope, owner_after.revision, owner_after.dirty),
        (owner_before.rope, owner_before.revision, owner_before.dirty)
    );
    assert!(fixture.store.undo(fixture.document).unwrap());
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap()
            .rope
            .to_string(),
        "let name=한;"
    );
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap()
            .indent_options
            .unwrap()
            .tab_size,
        DISPLAY_SIZE
    );
    let mut readonly = Fixture::new("let name=한;", true, false);
    assert!(matches!(readonly.begin(), Err(EditorError::ReadOnly)));
}

#[test]
fn 이전_포맷의_응답은_새_요청을_회수하지_않고_현재_요청만_적용한다() {
    let mut fixture = Fixture::new("a=1", false, false);
    let old = fixture.begin().unwrap().unwrap();
    let current = fixture.begin().unwrap().unwrap();
    assert!(old.is_cancelled());
    let providers = fixture.providers();
    let response = fixture.response(&old, "wrong");
    assert!(
        !fixture
            .state
            .accept(&mut fixture.store, &old, providers.clone(), response)
            .unwrap()
    );
    assert!(!current.is_cancelled());
    let response = fixture.response(&current, "a = 1");
    assert!(
        fixture
            .state
            .accept(&mut fixture.store, &current, providers, response)
            .unwrap()
    );
    assert!(current.is_cancelled());
}

#[test]
fn 포맷은_커서_편집_조합_옵션_공급자_뷰_변경과_종료에서_취소된다() {
    for change in [
        "cursor",
        "edit",
        "composition",
        "options",
        "provider",
        "view",
        "shutdown",
    ] {
        let mut fixture = Fixture::new("a=1", false, false);
        let request = fixture.begin().unwrap().unwrap();
        let mut providers = fixture.providers();
        match change {
            "cursor" => fixture
                .store
                .set_view_state(
                    fixture.source,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection { anchor: 1, head: 1 }],
                    },
                    ScrollPosition::default(),
                    Vec::new(),
                )
                .unwrap(),
            "edit" => {
                fixture
                    .store
                    .apply(
                        fixture.document,
                        Transaction {
                            revision: request.snapshot.revision,
                            edits: vec![Edit {
                                bytes: 0..0,
                                text: "x".into(),
                            }],
                            group: UndoGroup(0),
                            origin: Some(fixture.source),
                            selection_after: None,
                        },
                    )
                    .unwrap();
            }
            "composition" => fixture
                .store
                .set_composition(
                    fixture.source,
                    Some(Composition {
                        revision: request.snapshot.revision,
                        replace: 0..0,
                        preedit: "문".into(),
                    }),
                )
                .unwrap(),
            "options" => {
                fixture
                    .store
                    .set_indentation(
                        fixture.document,
                        IndentConfiguration {
                            defaults: IndentOptions {
                                tab_size: INDENT_SIZE,
                                insert_spaces: true,
                            },
                            detect_indentation: false,
                        },
                        IndentationChange::UseTabs(INDENT_SIZE),
                    )
                    .unwrap();
            }
            "provider" => {
                providers = HashSet::from([ProviderIdentity {
                    generation: fixture.provider.generation + 1,
                    ..fixture.provider
                }]);
            }
            "view" => {
                fixture.store.detach_view(fixture.owner).unwrap();
            }
            "shutdown" => fixture.state.clear(),
            _ => unreachable!(),
        }
        fixture
            .state
            .reconcile(&fixture.store, |_| providers.clone());
        assert!(request.is_cancelled(), "{change}");
        let response = fixture.response(&request, "wrong");
        let before = fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap();
        assert!(
            !fixture
                .state
                .accept(&mut fixture.store, &request, providers, response)
                .unwrap()
        );
        let after = fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap();
        assert_eq!(
            (after.rope, after.revision, after.dirty),
            (before.rope, before.revision, before.dirty)
        );
    }
}

#[test]
fn 포맷의_잘못된_utf16과_겹침은_원자적으로_거절하고_동일한_결과는_undo를_만들지_않는다() {
    for edits in [
        vec![TextEdit::new(
            LspRange::new(
                Position {
                    line: 0,
                    character: 1,
                },
                Position {
                    line: 0,
                    character: 2,
                },
            ),
            "wrong".into(),
        )],
        vec![
            TextEdit::new(
                LspRange::new(
                    Position::default(),
                    Position {
                        line: 0,
                        character: 2,
                    },
                ),
                "one".into(),
            ),
            TextEdit::new(
                LspRange::new(
                    Position::default(),
                    Position {
                        line: 0,
                        character: 2,
                    },
                ),
                "two".into(),
            ),
        ],
    ] {
        let mut fixture = Fixture::new("\u{1f600}=1", false, false);
        let request = fixture.begin().unwrap().unwrap();
        let providers = fixture.providers();
        let response = Ok(Response {
            provider: fixture.provider,
            edits,
        });
        assert!(
            fixture
                .state
                .accept(&mut fixture.store, &request, providers, response)
                .is_err()
        );
        let after = fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap();
        assert_eq!(
            (after.rope, after.revision, after.dirty),
            (
                request.snapshot.rope,
                request.snapshot.revision,
                request.snapshot.dirty
            )
        );
    }
    let mut fixture = Fixture::new("same", false, false);
    let request = fixture.begin().unwrap().unwrap();
    let providers = fixture.providers();
    let response = fixture.response(&request, "same");
    assert!(
        !fixture
            .state
            .accept(&mut fixture.store, &request, providers, response)
            .unwrap()
    );
    assert!(!fixture.store.undo(fixture.document).unwrap());
}

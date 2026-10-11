use super::*;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::{Composition, Selection};

const BYTE_LIMIT: usize = 4096;
const DOCUMENT_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;

struct Fixture {
    store: EditorStore,
    source: ViewId,
    owner: ViewId,
    project: ProjectId,
    provider: ProviderIdentity,
    state: State,
}

impl Fixture {
    fn new(source_read_only: bool, owner_read_only: bool) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: DOCUMENT_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let open = |path: &str, text: &str, read_only| OpenedFile {
            path: path.into(),
            content: text.into(),
            encoding_lossy: false,
            language_id: "rust".into(),
            tier: FileSizeTier::Normal,
            byte_size: text.len().try_into().unwrap(),
            line_count: 1,
            modified_ms: 0.0,
            read_only,
            editor_config: EditorConfigOptions::default(),
        };
        let document = store
            .open_file(
                "/synthetic/source.rs".into(),
                open("/synthetic/source.rs", "\u{1f600} method", source_read_only),
            )
            .unwrap();
        let owner_document = store
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
        let owner = store.attach_view(key(), owner_document).unwrap();
        let head = "\u{1f600} me".len();
        store
            .set_view_state(
                source,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor: head, head }],
                },
                Default::default(),
                Vec::new(),
            )
            .unwrap();
        Self {
            store,
            source,
            owner,
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

    fn begin(&mut self) -> Request {
        self.state
            .begin(
                &self.store,
                self.project.clone(),
                self.source,
                self.owner,
                self.providers(),
            )
            .unwrap()
            .unwrap()
    }

    fn ready(&mut self, request: &Request) {
        let prepared = Prepared {
            provider: self.provider,
            range: request.fallback.clone(),
            name: request.fallback_name(),
        };
        let providers = self.providers();
        assert!(
            self.state
                .accept(
                    &self.store,
                    request,
                    providers,
                    Ok(Response::Prepared(prepared))
                )
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn 이름_바꾸기는_utf16과_언어_단어를_준비하고_같은_이름과_빈_입력을_취소한다() {
    for name in ["method", "", " \t\n", "\u{feff}"] {
        let mut fixture = Fixture::new(false, true);
        let request = fixture.begin();
        assert_eq!(request.position, lsp_types::Position::new(0, 5));
        assert_eq!(
            request.fallback,
            "\u{1f600} ".len().."\u{1f600} method".len()
        );
        assert_eq!(request.fallback_name(), "method");
        fixture.ready(&request);
        assert!(fixture.state.rename(&fixture.store, name.into()).is_none());
        assert!(request.is_cancelled());
    }
    let mut fixture = Fixture::new(false, true);
    let request = fixture.begin();
    fixture.ready(&request);
    let rename = fixture
        .state
        .rename(&fixture.store, "새이름\u{1f600}".into())
        .unwrap();
    assert!(
        matches!(&rename.stage, Stage::Rename { name, provider } if name == "새이름\u{1f600}" && *provider == fixture.provider)
    );
    assert_eq!(rename.snapshot.revision, request.snapshot.revision);
    assert_ne!(rename.token, request.token);
    assert!(fixture.state.prepared().is_none());
    assert!(!rename.is_cancelled());
}

#[test]
fn 준비중_선택_입력_조합_소유와_공급자_변경은_응답을_취소한다() {
    for mutation in ["selection", "edit", "composition", "owner", "providers"] {
        let mut fixture = Fixture::new(false, true);
        let request = fixture.begin();
        let mut providers = fixture.providers();
        match mutation {
            "selection" => {
                let mut selection = fixture
                    .store
                    .views()
                    .get(fixture.source)
                    .unwrap()
                    .selection
                    .clone();
                selection.selections[0].anchor = request.fallback.start;
                fixture
                    .store
                    .set_view_state(fixture.source, selection, Default::default(), Vec::new())
                    .unwrap();
            }
            "edit" => {
                taide_native_editor::editing::type_text(&mut fixture.store, fixture.source, "x")
                    .unwrap();
            }
            "composition" => {
                fixture
                    .store
                    .set_composition(
                        fixture.source,
                        Some(Composition {
                            revision: request.snapshot.revision,
                            replace: request.fallback.clone(),
                            preedit: "조합".into(),
                        }),
                    )
                    .unwrap();
            }
            "owner" => {
                fixture.store.detach_view(fixture.owner).unwrap();
            }
            "providers" => {
                providers.clear();
            }
            _ => unreachable!(),
        }
        fixture.state.reconcile(&fixture.store, |_| providers);
        assert!(request.is_cancelled(), "mutation={mutation}");
        fixture.ready(&request);
        assert!(fixture.state.prepared().is_none());
    }
}

#[test]
fn 새_준비와_끝난_준비의_중복_응답은_현재_요청을_교체하지_않는다() {
    let mut fixture = Fixture::new(false, false);
    let previous = fixture.begin();
    let current = fixture.begin();
    assert!(previous.is_cancelled());
    fixture.ready(&previous);
    assert!(fixture.state.prepared().is_none());
    fixture.ready(&current);
    let rename = fixture
        .state
        .rename(&fixture.store, "renamed".into())
        .unwrap();
    fixture.ready(&current);
    let edits = Edits {
        provider: fixture.provider,
        edit: Default::default(),
        documents: Vec::new(),
        roots: vec!["/synthetic".into()],
    };
    let providers = fixture.providers();
    assert!(
        fixture
            .state
            .accept(
                &fixture.store,
                &rename,
                providers.clone(),
                Ok(Response::Edits(edits))
            )
            .unwrap()
            .is_some()
    );
    assert!(
        fixture
            .state
            .accept(
                &fixture.store,
                &rename,
                providers,
                Err(Failure::MalformedResponse)
            )
            .unwrap()
            .is_none()
    );
    fixture.state.finish(&previous);
    assert!(!rename.is_cancelled());
    fixture.state.finish(&rename);
    assert!(rename.is_cancelled());
}

#[test]
fn readonly_원본은_거절하고_잘못된_준비_범위나_세대는_현재_문서를_편집하지_않는다() {
    let mut fixture = Fixture::new(true, false);
    assert!(matches!(
        fixture.state.begin(
            &fixture.store,
            fixture.project.clone(),
            fixture.source,
            fixture.owner,
            fixture.providers()
        ),
        Err(EditorError::ReadOnly)
    ));
    for bad_provider in [false, true] {
        let mut fixture = Fixture::new(false, false);
        let request = fixture.begin();
        let response = Prepared {
            provider: if bad_provider {
                ProviderIdentity {
                    generation: fixture.provider.generation + 1,
                    ..fixture.provider
                }
            } else {
                fixture.provider
            },
            range: 1..request.fallback.end,
            name: "bad".into(),
        };
        let providers = fixture.providers();
        assert!(matches!(
            fixture.state.accept(
                &fixture.store,
                &request,
                providers,
                Ok(Response::Prepared(response))
            ),
            Err(Failure::MalformedResponse)
        ));
        assert!(request.is_cancelled());
        assert_eq!(
            fixture
                .store
                .documents()
                .snapshot(request.snapshot.id)
                .unwrap()
                .rope,
            request.snapshot.rope
        );
    }
}

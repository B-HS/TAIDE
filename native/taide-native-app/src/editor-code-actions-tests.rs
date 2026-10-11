use super::*;
use serde_json::json;
use taide_model::file::{EditorConfigOptions, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::{Composition, Selection};

const TEXT: &str = "\u{1f600} method\nother";
const PREFIX: &str = "\u{1f600} ";
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
    fn new(read_only: bool, tier: FileSizeTier) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: DOCUMENT_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let open = |path: &str, text: &str, read_only, tier| OpenedFile {
            path: path.into(),
            content: text.into(),
            encoding_lossy: false,
            language_id: "rust".into(),
            tier,
            byte_size: text.len().try_into().unwrap(),
            line_count: 2,
            modified_ms: 0.0,
            read_only,
            editor_config: EditorConfigOptions::default(),
        };
        let document = store
            .open_file(
                "/synthetic/source.rs".into(),
                open("/synthetic/source.rs", TEXT, read_only, tier),
            )
            .unwrap();
        let owner_document = store
            .open_file(
                "/synthetic/owner.rs".into(),
                open("/synthetic/owner.rs", "owner", true, FileSizeTier::Normal),
            )
            .unwrap();
        let key = || ViewKey {
            window: "synthetic".into(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let source = store.attach_view(key(), document).unwrap();
        let owner = store.attach_view(key(), owner_document).unwrap();
        store
            .set_view_state(
                source,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: "\u{1f600} method".len(),
                        head: PREFIX.len(),
                    }],
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

    fn begin(&mut self, command: Command, automatic: bool) -> Request {
        self.state
            .begin(
                &self.store,
                self.project.clone(),
                self.source,
                self.owner,
                self.providers(),
                Invocation { command, automatic },
            )
            .unwrap()
            .unwrap()
    }

    fn candidate(&self, value: serde_json::Value) -> Candidate {
        Candidate {
            provider: self.provider,
            raw: serde_json::from_value(value).unwrap(),
        }
    }

    fn accept(&mut self, request: &Request, candidates: Vec<Candidate>) -> Option<Chosen> {
        self.state
            .accept(&self.store, request, self.providers(), Ok(candidates))
            .unwrap()
    }
}

#[test]
fn 수정_목록은_주_선택의_utf16_범위와_원본_data_편집_명령을_보존한다() {
    let mut fixture = Fixture::new(false, FileSizeTier::Normal);
    let request = fixture.begin(Command::QuickFix, false);
    assert_eq!(
        request.range,
        lsp_types::Range::new(
            lsp_types::Position::new(0, 3),
            lsp_types::Position::new(0, 9)
        )
    );
    assert!(request.describes(&fixture.store));
    let value = json!({
        "title": "수정 名",
        "kind": "quickfix",
        "isPreferred": true,
        "data": {"value": ["名", 1]},
        "diagnostics": [{"range": {"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 9}}, "message": "problem"}],
        "edit": {"changes": {"file:///synthetic/source.rs": []}},
        "command": {"title": "after edit", "command": "synthetic.apply", "arguments": ["名"]},
    });
    let candidate = fixture.candidate(value.clone());
    assert!(fixture.accept(&request, vec![candidate]).is_none());
    let (_, listed, menu_open) = fixture
        .state
        .listed(&fixture.store, fixture.source)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "수정 名");
    assert!(listed[0].preferred);
    assert!(menu_open);
    let chosen = fixture
        .state
        .select(
            &fixture.store,
            fixture.source,
            request.token.as_u128(),
            0,
            true,
        )
        .unwrap();
    assert!(chosen.preview);
    assert_eq!(serde_json::to_value(&chosen.candidate.raw).unwrap(), value);
    assert!(
        fixture
            .state
            .listed(&fixture.store, fixture.source)
            .is_none()
    );
    assert!(!fixture.state.is_applying());
    assert!(
        fixture
            .state
            .begin_apply(&fixture.store, &chosen, fixture.providers())
    );
    assert!(fixture.state.is_applying());
}

#[test]
fn 조회_표시_추가조회_중_선택_편집_조합_소유_공급자_변경은_늦은_수정을_거절한다() {
    for stage in ["query", "listed", "chosen"] {
        for mutation in [
            "selection",
            "edit",
            "composition",
            "owner",
            "provider",
            "active",
        ] {
            let mut fixture = Fixture::new(false, FileSizeTier::Normal);
            let request = fixture.begin(Command::QuickFix, false);
            let candidate = fixture.candidate(json!({"title": "fix", "kind": "quickfix"}));
            if stage != "query" {
                assert!(fixture.accept(&request, vec![candidate.clone()]).is_none());
            }
            let chosen = (stage == "chosen").then(|| {
                fixture
                    .state
                    .select(
                        &fixture.store,
                        fixture.source,
                        request.token.as_u128(),
                        0,
                        false,
                    )
                    .unwrap()
            });
            let mut providers = fixture.providers();
            match mutation {
                "selection" => {
                    fixture
                        .store
                        .set_view_state(
                            fixture.source,
                            Default::default(),
                            Default::default(),
                            Vec::new(),
                        )
                        .unwrap();
                }
                "edit" => {
                    taide_native_editor::editing::type_text(
                        &mut fixture.store,
                        fixture.source,
                        "x",
                    )
                    .unwrap();
                }
                "composition" => {
                    fixture
                        .store
                        .set_composition(
                            fixture.source,
                            Some(Composition {
                                revision: request.snapshot.revision,
                                replace: PREFIX.len()..PREFIX.len(),
                                preedit: "조합".into(),
                            }),
                        )
                        .unwrap();
                }
                "owner" => {
                    fixture.store.detach_view(fixture.owner).unwrap();
                }
                "provider" => {
                    providers = HashSet::from([ProviderIdentity {
                        capability_revision: 1,
                        ..fixture.provider
                    }]);
                }
                "active" => (),
                _ => unreachable!(),
            }
            fixture.state.reconcile(
                &fixture.store,
                |_| mutation != "active",
                |_| providers.clone(),
            );
            assert!(request.is_cancelled(), "stage={stage}, mutation={mutation}");
            assert!(
                fixture
                    .state
                    .accept(
                        &fixture.store,
                        &request,
                        providers.clone(),
                        Ok(vec![candidate])
                    )
                    .unwrap()
                    .is_none()
            );
            if let Some(chosen) = chosen {
                assert!(
                    !fixture
                        .state
                        .begin_apply(&fixture.store, &chosen, providers)
                );
            }
            assert!(
                fixture
                    .state
                    .listed(&fixture.store, fixture.source)
                    .is_none()
            );
        }
    }
}

#[test]
fn 여섯_수동_명령의_단일_자동_적용은_종류와_비활성_preferred를_구분한다() {
    for (command, kind, preferred, automatic_apply) in [
        (Command::QuickFix, "quickfix", false, false),
        (Command::Refactor, "refactor.extract", false, false),
        (Command::Source, "source", false, false),
        (
            Command::OrganizeImports,
            "source.organizeImports",
            false,
            true,
        ),
        (Command::FixAll, "source.fixAll", false, true),
        (Command::AutoFix, "quickfix", true, true),
    ] {
        let mut fixture = Fixture::new(false, FileSizeTier::Normal);
        let request = fixture.begin(command, false);
        let valid =
            fixture.candidate(json!({"title": "valid", "kind": kind, "isPreferred": preferred}));
        let disabled = fixture.candidate(json!({"title": "disabled", "kind": kind, "isPreferred": true, "disabled": {"reason": ""}}));
        let outside = fixture.candidate(json!({"title": "outside", "kind": "unrelated.kind"}));
        let mut candidates = vec![valid, disabled];
        if command != Command::QuickFix {
            candidates.push(outside);
        }
        let chosen = fixture.accept(&request, candidates);
        assert_eq!(chosen.is_some(), automatic_apply, "command={command:?}");
        if let Some(chosen) = chosen {
            assert_eq!(chosen.candidate.presentation().title, "valid");
            assert!(!chosen.preview);
        } else {
            let (_, actions, _) = fixture
                .state
                .listed(&fixture.store, fixture.source)
                .unwrap();
            assert_eq!(actions.len(), 2);
            let disabled = actions
                .iter()
                .position(|action| action.disabled.is_some())
                .unwrap();
            assert!(
                fixture
                    .state
                    .select(
                        &fixture.store,
                        fixture.source,
                        request.token.as_u128(),
                        disabled,
                        false
                    )
                    .is_none()
            );
        }
    }
}

#[test]
fn 자동_조회는_source를_제외하고_전구_열기_전후에도_한번만_선택한다() {
    let mut fixture = Fixture::new(false, FileSizeTier::Normal);
    let request = fixture.begin(Command::QuickFix, true);
    let normal = fixture.candidate(json!({"title": "normal", "kind": "refactor.extract"}));
    let preferred =
        fixture.candidate(json!({"title": "preferred", "kind": "quickfix", "isPreferred": true}));
    let source = fixture.candidate(json!({"title": "source", "kind": "source.fixAll"}));
    let bare = fixture
        .candidate(json!({"title": "bare", "command": "synthetic.command", "arguments": ["名"]}));
    assert!(
        fixture
            .accept(&request, vec![normal, preferred, source, bare])
            .is_none()
    );
    let (_, actions, open) = fixture
        .state
        .listed(&fixture.store, fixture.source)
        .unwrap();
    assert!(!open);
    assert_eq!(
        actions
            .iter()
            .map(|action| action.title.as_str())
            .collect::<Vec<_>>(),
        ["preferred", "normal", "bare"]
    );
    assert!(
        !fixture
            .state
            .open(&fixture.store, fixture.owner, request.token.as_u128())
    );
    assert!(
        fixture
            .state
            .open(&fixture.store, fixture.source, request.token.as_u128())
    );
    assert!(
        fixture
            .state
            .listed(&fixture.store, fixture.source)
            .unwrap()
            .2
    );
    assert!(
        fixture
            .state
            .select(
                &fixture.store,
                fixture.source,
                request.token.as_u128(),
                1,
                false
            )
            .is_some()
    );
    assert!(
        fixture
            .state
            .select(
                &fixture.store,
                fixture.source,
                request.token.as_u128(),
                1,
                false
            )
            .is_none()
    );
}

#[test]
fn 자동_전구_목록을_닫아도_같은_수정_후보를_다시_열_수_있다() {
    let mut fixture = Fixture::new(false, FileSizeTier::Normal);
    let request = fixture.begin(Command::QuickFix, true);
    let candidate = fixture.candidate(json!({"title": "fix", "kind": "quickfix"}));
    fixture.accept(&request, vec![candidate]);
    assert!(
        fixture
            .state
            .open(&fixture.store, fixture.source, request.token.as_u128())
    );
    fixture.state.close(fixture.source, request.token.as_u128());
    assert!(!request.is_cancelled());
    let (_, actions, open) = fixture
        .state
        .listed(&fixture.store, fixture.source)
        .unwrap();
    assert_eq!(actions[0].title, "fix");
    assert!(!open);
    assert!(
        fixture
            .state
            .open(&fixture.store, fixture.source, request.token.as_u128())
    );
    fixture.state.finish(&request);
    assert!(request.is_cancelled());
}

#[test]
fn 이전_중복_다른_공급자_응답은_현재_수정_목록을_대신할_수_없다() {
    let mut fixture = Fixture::new(false, FileSizeTier::Normal);
    let previous = fixture.begin(Command::QuickFix, false);
    let request = fixture.begin(Command::QuickFix, false);
    let candidate = fixture.candidate(json!({"title": "current", "kind": "quickfix"}));
    assert!(previous.is_cancelled());
    assert!(fixture.accept(&previous, vec![candidate.clone()]).is_none());
    assert!(
        fixture
            .state
            .listed(&fixture.store, fixture.source)
            .is_none()
    );
    assert!(fixture.accept(&request, vec![candidate.clone()]).is_none());
    assert!(fixture.accept(&request, Vec::new()).is_none());
    assert_eq!(
        fixture
            .state
            .listed(&fixture.store, fixture.source)
            .unwrap()
            .1[0]
            .title,
        "current"
    );
    fixture.state.close(fixture.owner, request.token.as_u128());
    assert!(!request.is_cancelled());
    fixture.state.close(fixture.source, request.token.as_u128());
    assert!(
        !fixture
            .state
            .listed(&fixture.store, fixture.source)
            .unwrap()
            .2
    );
    assert!(!request.is_cancelled());
    fixture.state.finish(&request);
    assert!(request.is_cancelled());
    let request = fixture.begin(Command::QuickFix, false);
    let foreign = Candidate {
        provider: ProviderIdentity {
            generation: 2,
            ..fixture.provider
        },
        ..candidate
    };
    assert!(matches!(
        fixture.state.accept(
            &fixture.store,
            &request,
            fixture.providers(),
            Ok(vec![foreign])
        ),
        Err(Failure::StaleGeneration)
    ));
    assert!(request.is_cancelled());
}

#[test]
fn 시작한_편집의_자체_문서변경은_허용하고_소유_회수는_끝까지_취소한다() {
    for active in [true, false] {
        let mut fixture = Fixture::new(false, FileSizeTier::Normal);
        let request = fixture.begin(Command::AutoFix, false);
        let candidate =
            fixture.candidate(json!({"title": "fix", "kind": "quickfix", "isPreferred": true}));
        let chosen = fixture.accept(&request, vec![candidate]).unwrap();
        assert!(
            fixture
                .state
                .begin_apply(&fixture.store, &chosen, fixture.providers())
        );
        taide_native_editor::editing::type_text(&mut fixture.store, fixture.source, "applied")
            .unwrap();
        fixture
            .state
            .reconcile(&fixture.store, |_| active, |_| HashSet::new());
        assert_eq!(request.is_cancelled(), !active);
        fixture.state.finish(&request);
        assert!(request.is_cancelled());
        assert!(!fixture.state.is_applying());
    }
}

#[test]
fn 읽기전용_source와_큰_tier는_수정을_거절하고_읽기전용_owner는_구분한다() {
    let mut read_only = Fixture::new(true, FileSizeTier::Normal);
    assert!(matches!(
        read_only.state.begin(
            &read_only.store,
            read_only.project.clone(),
            read_only.source,
            read_only.owner,
            read_only.providers(),
            Invocation {
                command: Command::QuickFix,
                automatic: false
            }
        ),
        Err(EditorError::ReadOnly)
    ));
    let mut large = Fixture::new(false, FileSizeTier::Large);
    assert!(
        large
            .state
            .begin(
                &large.store,
                large.project.clone(),
                large.source,
                large.owner,
                large.providers(),
                Invocation {
                    command: Command::QuickFix,
                    automatic: true
                }
            )
            .unwrap()
            .is_none()
    );
    let mut normal = Fixture::new(false, FileSizeTier::Normal);
    assert!(
        normal
            .begin(Command::QuickFix, false)
            .describes(&normal.store)
    );
}

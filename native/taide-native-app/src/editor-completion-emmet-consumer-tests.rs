use super::*;
use taide_lsp::native::protocol::lsp_types::{CompletionItem, CompletionItemKind};
use taide_native_editor::completion::{Candidate, Candidates};

fn emmet_items(fixture: &Fixture) -> Vec<String> {
    fixture
        .state
        .candidates(&fixture.store, fixture.view)
        .unwrap_or_default()
        .into_iter()
        .filter(|candidate| {
            candidate
                .item
                .detail
                .as_deref()
                .is_some_and(|detail| detail.eq_ignore_ascii_case("Emmet abbreviation"))
        })
        .map(|candidate| candidate.item.label.clone())
        .collect()
}

#[test]
fn native_emmet의_실제완성위젯은_여섯언어와_전체축약식_삽입_undo를_연결한다() {
    for (language, text, output) in [
        (
            "html",
            "div.foo>span",
            "<div class=\"foo\"><span></span></div>",
        ),
        ("heex", "div", "<div></div>"),
        (
            "javascriptreact",
            "div.foo",
            "<div className=\"foo\"></div>",
        ),
        (
            "typescriptreact",
            "div.foo",
            "<div className=\"foo\"></div>",
        ),
        ("css", "m10-20", "margin: 10px 20px;"),
        ("scss", "m10", "margin: 10px;"),
    ] {
        let mut fixture = Fixture::with_language(text, text.len(), Arc::from([]), language);
        fixture.frame(Vec::new(), &[Command::Trigger]);
        assert!(!emmet_items(&fixture).is_empty(), "{language}/{text}");
        let entry = fixture.state.entries.get(&fixture.view).unwrap();
        assert!(!entry.emmet.is_empty());
        for candidate in &entry.emmet {
            assert_eq!(candidate.document, fixture.document);
            assert_eq!(candidate.insert, 0..text.len());
            assert_eq!(candidate.replace, candidate.insert);
        }
        fixture.frame(Vec::new(), &[]);
        assert!(!fixture.geometry.rows.is_empty(), "{language}");
        assert!(fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]));
        assert_eq!(fixture.text(fixture.document), output, "{language}");
        assert!(fixture.store.undo(fixture.document).unwrap());
        assert_eq!(fixture.text(fixture.document), text);
    }
}

#[test]
fn native_emmet의_설정과_standard_token은_후보를_즉시회수하고_오래된위젯을_거절한다() {
    let mut fixture = Fixture::with_language("div", 3, Arc::from([]), "html");
    fixture.frame(Vec::new(), &[Command::Trigger]);
    let old = fixture.state.entries[&fixture.view].widget_token;
    assert_eq!(emmet_items(&fixture), ["div"]);
    fixture.services.state.settings.write().emmet_enabled = false;
    fixture.frame(Vec::new(), &[]);
    assert!(emmet_items(&fixture).is_empty());
    assert_ne!(fixture.state.entries[&fixture.view].widget_token, old);
    fixture.services.state.settings.write().emmet_enabled = true;
    for kind in [
        TokenKind::Comment,
        TokenKind::String,
        TokenKind::Regex,
        TokenKind::Other,
    ] {
        fixture.syntax = Syntax(kind);
        fixture.frame(Vec::new(), &[]);
        assert_eq!(!emmet_items(&fixture).is_empty(), kind == TokenKind::Other);
    }
    assert_eq!(fixture.text(fixture.document), "div");
}

#[test]
fn native_emmet의_기존후보는_편집후_새축약식으로_다시생성한다() {
    let mut fixture = Fixture::with_language("div", 3, Arc::from([]), "html");
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(vec![Event::Text(".foo".into())], &[]);
    fixture.frame(Vec::new(), &[]);
    assert_eq!(emmet_items(&fixture), ["div.foo"]);
    assert_eq!(
        fixture.state.entries[&fixture.view].emmet[0].insert,
        0.."div.foo".len()
    );
    assert!(fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "<div class=\"foo\"></div>");
}

#[test]
fn native_emmet의_실제숫자_trigger는_css후보를_공급하고_꺼진설정에서는_호출하지않는다() {
    for enabled in [true, false] {
        let mut fixture = Fixture::with_language("m", 1, Arc::from([]), "css");
        fixture.services.state.settings.write().emmet_enabled = enabled;
        fixture.frame(Vec::new(), &[]);
        fixture.frame(vec![Event::Text("10".into())], &[]);
        fixture.frame(Vec::new(), &[]);
        assert_eq!(!emmet_items(&fixture).is_empty(), enabled);
        assert_eq!(fixture.text(fixture.document), "m10");
    }
}

#[test]
fn native_emmet의_다른문서_peek_수락은_소유문서를_보존한다() {
    let mut fixture = Fixture::with_language("div", 3, Arc::from([]), "html");
    let original = fixture.document;
    let source = fixture
        .store
        .open_untitled(TabId::new(), "m10", "css".into())
        .unwrap();
    let owner = fixture.view;
    let view = fixture
        .store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            source,
        )
        .unwrap();
    fixture
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 3, head: 3 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    fixture.view = view;
    fixture.owner = owner;
    fixture.document = source;
    fixture.frame(Vec::new(), &[Command::Trigger]);
    fixture.frame(Vec::new(), &[]);
    assert!(fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(source), "margin: 10px;");
    assert_eq!(fixture.text(original), "div");
    assert_eq!(fixture.store.views().get(owner).unwrap().document, original);
    assert!(fixture.store.undo(source).unwrap());
    assert_eq!(fixture.text(source), "m10");
}

#[test]
fn native_emmet의_다중커서와_mirror는_실제문서에_한번의_undo로_삽입한다() {
    let mut fixture = Fixture::with_language("div\ndiv", 3, Arc::from([]), "html");
    let mirror = fixture
        .store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            fixture.document,
        )
        .unwrap();
    fixture
        .store
        .set_view_state(
            fixture.view,
            SelectionSet {
                primary: 0,
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
    fixture.frame(Vec::new(), &[]);
    assert!(fixture.frame(vec![key(Key::Tab, Modifiers::NONE)], &[]));
    assert_eq!(fixture.text(fixture.document), "<div></div>\n<div></div>");
    assert_eq!(
        fixture.store.views().get(mirror).unwrap().document,
        fixture.document
    );
    assert!(fixture.store.undo(fixture.document).unwrap());
    assert_eq!(fixture.text(fixture.document), "div\ndiv");
}

#[test]
fn native_emmet의_tier와_readonly_변경은_revision이같아도_이전요청을_회수한다() {
    for (tier, read_only) in [
        (FileSizeTier::Large, false),
        (FileSizeTier::ReadOnly, false),
        (FileSizeTier::Normal, true),
    ] {
        let mut fixture = Fixture::with_language("div", 3, Arc::from([]), "html");
        let path = fixture.directory.join("source.html");
        let file = OpenedFile {
            path: path.to_string_lossy().into_owned(),
            content: "div".into(),
            encoding_lossy: false,
            language_id: "html".into(),
            tier: FileSizeTier::Normal,
            byte_size: 3,
            line_count: 1,
            modified_ms: 0.0,
            read_only: false,
            editor_config: EditorConfigOptions::default(),
        };
        let saved = fixture.store.save_snapshot(fixture.document).unwrap();
        let tab = fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .key
            .tab
            .clone();
        fixture
            .store
            .convert_untitled_save(saved, path.clone(), file.clone(), tab)
            .unwrap();
        fixture.frame(Vec::new(), &[Command::Trigger]);
        let request = fixture
            .state
            .request(&fixture.store, fixture.view)
            .unwrap()
            .clone();
        assert!(!emmet_items(&fixture).is_empty());
        let snapshot = fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap();
        fixture
            .store
            .observe_file(
                fixture.document,
                &path,
                OpenedFile {
                    tier,
                    read_only,
                    ..file
                },
            )
            .unwrap();
        assert_eq!(
            fixture
                .store
                .documents()
                .snapshot(fixture.document)
                .unwrap()
                .revision,
            snapshot.revision
        );
        fixture.frame(Vec::new(), &[]);
        assert!(request.is_cancelled());
        assert!(emmet_items(&fixture).is_empty());
        fixture.frame(Vec::new(), &[Command::Trigger]);
        assert_eq!(
            !emmet_items(&fixture).is_empty(),
            tier == FileSizeTier::Large && !read_only
        );
        assert_eq!(fixture.text(fixture.document), "div");
    }
}

#[test]
fn native_emmet는_실제_lsp_사용자snippet과_합치고_ime시작에서_무수락_회수한다() {
    let files = Arc::from([SnippetFile {
        file_name: "html.json".into(),
        snippets: BTreeMap::from([(
            "Custom".into(),
            SnippetEntry {
                prefix: SnippetStringOrList::Single("div-custom".into()),
                body: SnippetStringOrList::Single("custom".into()),
                description: None,
                scope: None,
            },
        )]),
    }]);
    let mut fixture = Fixture::with_language("div", 3, files, "html");
    let identity = crate::editor_symbols::ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    };
    let providers = std::collections::HashSet::from([identity]);
    let request = fixture
        .state
        .begin(
            &fixture.store,
            crate::editor_completion::Context {
                project: fixture.project.clone(),
                source: fixture.view,
                owner: fixture.owner,
                word: 0..3,
                viewport: egui::ViewportId::ROOT,
                automatic: false,
            },
            providers.clone(),
        )
        .unwrap();
    let item = Candidate::new(
        &request.snapshot,
        request.position,
        request.word,
        CompletionItem {
            label: "div-lsp".into(),
            kind: Some(CompletionItemKind::PROPERTY),
            ..Default::default()
        },
    )
    .unwrap();
    fixture
        .state
        .accept(
            &fixture.store,
            &request,
            providers,
            Ok(crate::editor_completion::Response {
                groups: vec![crate::editor_completion::Group {
                    provider: identity,
                    ordinal: 0,
                    candidates: Candidates {
                        items: vec![item],
                        is_incomplete: false,
                    },
                }],
            }),
        )
        .unwrap();
    fixture.state.supply(
        &fixture.store,
        &request,
        &fixture.files,
        &fixture.services.tasks,
        Arc::new(|| {}),
    );
    fixture.frame(Vec::new(), &[]);
    fixture.frame(Vec::new(), &[]);
    let items = fixture
        .state
        .candidates(&fixture.store, fixture.view)
        .unwrap()
        .into_iter()
        .map(|candidate| candidate.item.label.as_str())
        .collect::<Vec<_>>();
    assert!(items.contains(&"div"));
    assert!(items.contains(&"div-custom"));
    assert!(items.contains(&"div-lsp"));
    fixture.frame(
        vec![Event::Ime(egui::ImeEvent::Preedit {
            text: "한".into(),
            active_range_chars: None,
        })],
        &[],
    );
    assert!(emmet_items(&fixture).is_empty());
    assert!(request.is_cancelled());
    assert_eq!(fixture.text(fixture.document), "div");
}

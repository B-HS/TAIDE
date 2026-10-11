#![cfg(feature = "native-host")]

use egui::{ViewportId, pos2};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_context_menu::{
    Action, Context, Entry, Group, Session, groups, right_click,
};

const TEXT: &str = "ab\u{1f600}cd\nword";
const BYTE_LIMIT: usize = 4096;
const VIEW_LIMIT: usize = 3;
const UNDO_LIMIT: usize = 4;
const MENU_SCROLL: f32 = 20.0;

fn entries(context: Context) -> Vec<Group> {
    groups(context, "Peek".into(), |_, label| (label.into(), None))
}

fn ids(groups: &[Group]) -> Vec<&str> {
    groups
        .iter()
        .flat_map(|group| group.entries.iter())
        .flat_map(|entry| match entry {
            Entry::Item(item) => vec![item.action.id()],
            Entry::Submenu { items, .. } => items.iter().map(|item| item.action.id()).collect(),
        })
        .collect()
}

fn fixture() -> (EditorStore, ViewId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: VIEW_LIMIT,
        max_undo_groups: UNDO_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_untitled(tab.clone(), TEXT, "rust".into())
        .unwrap();
    let source = store
        .attach_view(
            ViewKey {
                window: "context-menu-fixture".into(),
                pane: PaneId::new(),
                tab,
            },
            document,
        )
        .unwrap();
    let owner = store
        .attach_view(
            ViewKey {
                window: "context-menu-fixture".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, source, owner)
}

fn select(store: &mut EditorStore, view: ViewId, selection: SelectionSet) {
    let current = store.views().get(view).unwrap();
    store
        .set_view_state(
            view,
            selection,
            current.scroll.clone(),
            current.folds.clone(),
        )
        .unwrap();
}

#[test]
fn 메뉴는_원본_그룹과_동률_제목_순서_하위_메뉴_및_재지정_표시를_보존한다() {
    let context = Context {
        has_selection: true,
        definition: true,
        declaration: true,
        type_definition: true,
        implementation: true,
        references: true,
        rename: true,
        document_formatting: true,
        range_formatting: true,
        refactor: true,
        source_actions: true,
        ..Context::default()
    };
    let menu = groups(context, "정의 미리보기".into(), |action, _| {
        (format!("번역 {}", action.id()), Some("재지정 키".into()))
    });
    assert_eq!(
        menu.iter().map(|group| group.id).collect::<Vec<_>>(),
        [
            "navigation",
            "1_modification",
            "9_cutcopypaste",
            "9_terminal"
        ]
    );
    let navigation = menu[0]
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Item(item) => Some(item.action.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        navigation,
        [
            "editor.action.revealDefinition",
            "editor.action.revealDeclaration",
            "editor.action.goToTypeDefinition",
            "editor.action.goToImplementation",
            "editor.action.goToReferences",
            "taide.toggleMinimap"
        ]
    );
    let Entry::Submenu { label, items } = menu[0].entries.last().unwrap() else {
        panic!("Peek submenu is required");
    };
    assert_eq!(label, "정의 미리보기");
    assert_eq!(
        items
            .iter()
            .map(|item| item.action.id())
            .collect::<Vec<_>>(),
        [
            "editor.action.peekDefinition",
            "editor.action.peekDeclaration",
            "editor.action.peekTypeDefinition",
            "editor.action.peekImplementation",
            "editor.action.referenceSearch.trigger"
        ]
    );
    assert!(
        items
            .iter()
            .all(|item| item.shortcut.as_deref() == Some("재지정 키"))
    );
    assert!(ids(&menu).iter().all(|id| !id.contains("inlineEdit")
        && !id.contains("quickFix")
        && !id.contains("CopyWith")));
}

#[test]
fn 읽기전용은_편집_그룹과_cut_paste를_숨기고_탐색_copy_터미널을_보존한다() {
    let menu = entries(Context {
        read_only: true,
        definition: true,
        rename: true,
        document_formatting: true,
        range_formatting: true,
        refactor: true,
        source_actions: true,
        has_selection: true,
        ..Context::default()
    });
    assert_eq!(
        menu.iter().map(|group| group.id).collect::<Vec<_>>(),
        ["navigation", "9_cutcopypaste", "9_terminal"]
    );
    assert_eq!(
        ids(&menu),
        [
            "editor.action.revealDefinition",
            "taide.toggleMinimap",
            "editor.action.peekDefinition",
            "editor.action.clipboardCopyAction",
            "taide.runSelectedTextInTerminal"
        ]
    );
}

#[test]
fn 선택과_복합_편집기_및_공급자_조건은_포맷과_수정_항목을_구분한다() {
    let menu = entries(Context {
        composite: true,
        document_formatting: true,
        range_formatting: true,
        ..Context::default()
    });
    assert!(!ids(&menu).contains(&"editor.action.formatDocument"));
    assert!(!ids(&menu).contains(&"editor.action.formatSelection"));
    let menu = entries(Context {
        has_selection: true,
        range_formatting: true,
        ..Context::default()
    });
    assert!(ids(&menu).contains(&"editor.action.formatSelection"));
    assert!(!ids(&menu).contains(&"editor.action.rename"));
    assert!(!ids(&menu).contains(&"editor.action.refactor"));
    assert!(!ids(&menu).contains(&"editor.action.sourceAction"));
    assert!(
        menu.iter()
            .flat_map(|group| &group.entries)
            .all(|entry| matches!(entry, Entry::Item(_)))
    );
}

#[test]
fn 미리보기는_정의_타입_구현_이동을_보존하며_중첩_peek와_선언_참조를_숨긴다() {
    let menu = entries(Context {
        embedded: true,
        definition: true,
        declaration: true,
        type_definition: true,
        implementation: true,
        references: true,
        ..Context::default()
    });
    let all = ids(&menu);
    assert!(all.contains(&"editor.action.revealDefinition"));
    assert!(all.contains(&"editor.action.goToTypeDefinition"));
    assert!(all.contains(&"editor.action.goToImplementation"));
    assert!(!all.contains(&"editor.action.revealDeclaration"));
    assert!(!all.contains(&"editor.action.goToReferences"));
    assert!(!all.contains(&"taide.toggleMinimap"));
    assert!(!all.contains(&"taide.runSelectedTextInTerminal"));
    assert!(
        menu.iter()
            .flat_map(|group| &group.entries)
            .all(|entry| matches!(entry, Entry::Item(_)))
    );
}

#[test]
fn 우클릭은_역방향과_주_다중_선택_끝점을_보존하고_밖에서만_단일_커서로_바꾼다() {
    let (mut store, source, _) = fixture();
    let selection = SelectionSet {
        primary: 1,
        selections: vec![
            Selection { anchor: 2, head: 0 },
            Selection {
                anchor: 13,
                head: 9,
            },
        ],
    };
    select(&mut store, source, selection.clone());
    for head in [0, 2, 9, 11, 13] {
        right_click(&mut store, source, head).unwrap();
        assert_eq!(store.views().get(source).unwrap().selection, selection);
    }
    right_click(&mut store, source, 6).unwrap();
    assert_eq!(
        store.views().get(source).unwrap().selection,
        SelectionSet {
            primary: 0,
            selections: vec![Selection { anchor: 6, head: 6 }]
        }
    );
    assert_eq!(
        store
            .documents()
            .snapshot(store.views().get(source).unwrap().document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
}

#[test]
fn 잘못된_unicode_바이트와_문서_밖_우클릭은_선택을_변경하지_않는다() {
    let (mut store, source, _) = fixture();
    let before = store.views().get(source).unwrap().selection.clone();
    for head in [3, TEXT.len() + 1, usize::MAX] {
        assert!(right_click(&mut store, source, head).is_err());
        assert_eq!(store.views().get(source).unwrap().selection, before);
    }
}

#[test]
fn 메뉴_수명은_선택_스크롤_편집_뷰_소유_및_뷰포트_변화를_거절한다() {
    for mutation in ["selection", "scroll", "edit", "source-close", "owner-close"] {
        let (mut store, source, owner) = fixture();
        let session = Session::open(
            &store,
            source,
            owner,
            ViewportId::ROOT,
            pos2(0.0, MENU_SCROLL),
            entries(Context::default()),
        )
        .unwrap();
        assert!(session.matches(&store, source, owner, ViewportId::ROOT));
        assert!(!session.matches(&store, owner, source, ViewportId::ROOT));
        assert!(!session.matches(&store, source, owner, ViewportId::from_hash_of("other")));
        match mutation {
            "selection" => right_click(&mut store, source, 6).unwrap(),
            "scroll" => {
                let view = store.views().get(source).unwrap();
                store
                    .set_view_state(
                        source,
                        view.selection.clone(),
                        ScrollPosition {
                            x: 0.0,
                            y: MENU_SCROLL,
                        },
                        view.folds.clone(),
                    )
                    .unwrap();
            }
            "edit" => {
                let document = store.views().get(source).unwrap().document;
                store
                    .apply(
                        document,
                        Transaction {
                            revision: 0,
                            edits: vec![Edit {
                                bytes: 0..0,
                                text: "x".into(),
                            }],
                            group: UndoGroup(0),
                            origin: Some(source),
                            selection_after: None,
                        },
                    )
                    .unwrap();
            }
            "source-close" => {
                store.detach_view(source).unwrap();
            }
            "owner-close" => {
                store.detach_view(owner).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            !session.matches(&store, source, owner, ViewportId::ROOT),
            "{mutation}"
        );
    }
}

#[test]
fn 명령_id는_clipboard와_원본_사용자_등록_액션을_그대로_식별한다() {
    assert_eq!(Action::Copy.id(), "editor.action.clipboardCopyAction");
    assert_eq!(Action::Cut.id(), "editor.action.clipboardCutAction");
    assert_eq!(Action::Paste.id(), "editor.action.clipboardPasteAction");
    assert_eq!(Action::ToggleMinimap.id(), "taide.toggleMinimap");
    assert_eq!(
        Action::RunSelectedText.id(),
        "taide.runSelectedTextInTerminal"
    );
}

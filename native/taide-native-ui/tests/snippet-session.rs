use std::ops::Range;
use std::path::PathBuf;
use std::process::Command;

use serde_json::{Value, json};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, Edit, EditorError, LineEnding, UndoGroup};
use taide_native_editor::editing::{replace_selections, select_all};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::snippet_expansion::expand;
use taide_native_editor::snippet_insertion::{Insertion, PreparedSnippet, insert};
use taide_native_editor::snippet_session::Session;
use taide_native_editor::snippet_syntax::{
    FinalTabstopOptions, ParseLimits, RegexMetadata, parse_complete,
};
use taide_native_editor::snippet_tracking::map_utf16_range;
use taide_native_editor::snippet_whitespace::{
    WhitespaceContext, adjust_whitespace, normalize_transform,
};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{
    Composition, ScrollPosition, Selection, SelectionSet, ViewId, ViewKey,
};

const BYTE_LIMIT: usize = 1024 * 1024;
const NESTING_LIMIT: usize = 64;
const MARKER_LIMIT: usize = 4096;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 16;
const MAPPING_CASE_COUNT: usize = 1680;
const SESSION_CASE_COUNT: usize = 14;
const INDENT_SIZE: u32 = 4;
const REFUSED_BYTE_LIMIT: usize = 8;

#[test]
fn snippet_session은_여러cursor의_primary_조합_삭제와_겹친caret회수를_보존한다() {
    let original = "  aa\n\t bb\n";
    let (mut store, document, view) = fixture(original);
    let indent = IndentOptions {
        tab_size: INDENT_SIZE,
        insert_spaces: true,
    };
    let first = "  ".len().."  aa".len();
    let second = "  aa\n\t ".len().."  aa\n\t bb".len();
    let selection = SelectionSet {
        primary: 0,
        selections: vec![
            Selection {
                anchor: second.end,
                head: second.start,
            },
            Selection {
                anchor: first.start,
                head: first.end,
            },
        ],
    };
    store
        .set_view_state(
            view,
            selection.clone(),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let expected = store.views().get(view).unwrap().clone();
    let template = "${1:漢}$1-${2:next}$0";
    let insertion = insert(
        &mut store,
        &expected,
        0,
        vec![
            prepare(
                template,
                second,
                "\t ".len(),
                "\t bb",
                indent,
                LineEnding::Lf,
            ),
            prepare(template, first, "  ".len(), "  aa", indent, LineEnding::Lf),
        ],
        limits(),
    )
    .unwrap();
    let mut session = Session::new(&store, insertion, indent, limits()).unwrap();
    let composition = Composition {
        revision: 1,
        replace: 0..0,
        preedit: "한".into(),
    };
    store
        .set_composition(view, Some(composition.clone()))
        .unwrap();
    assert!(matches!(
        session.replace(&mut store, "x", None),
        Err(EditorError::Refused)
    ));
    assert!(session.is_active());
    assert_eq!(
        store.views().get(view).unwrap().composition,
        Some(composition)
    );
    store.set_composition(view, None).unwrap();
    assert!(session.replace(&mut store, "𐐀", None).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  𐐀𐐀-next\n\t 𐐀𐐀-next\n"
    );
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    let current = &store.views().get(view).unwrap().selection;
    assert_eq!(current.primary, 0);
    assert!(current.selections[0].head > current.selections[1].head);
    session.replace(&mut store, "v", None).unwrap();
    session.replace(&mut store, "", Some(false)).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  𐐀𐐀-\n\t 𐐀𐐀-\n"
    );
    session
        .step(&mut store, false, |_| Err(EditorError::Refused))
        .unwrap();
    assert_eq!(
        store.views().get(view).unwrap().selection.selections.len(),
        VIEW_LIMIT * 2
    );
    assert!(session.replace(&mut store, "", None).unwrap());
    assert!(!session.is_active());
    assert!(session.decorations(&store).unwrap().is_empty());
    assert_eq!(
        store.views().get(view).unwrap().selection.selections.len(),
        VIEW_LIMIT
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  -\n\t -\n"
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  漢漢-next\n\t 漢漢-next\n"
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        original
    );
    assert_eq!(store.views().get(view).unwrap().selection, selection);
    assert!(!store.undo(document).unwrap());
}

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: BYTE_LIMIT,
        max_nesting: NESTING_LIMIT,
        max_markers: MARKER_LIMIT,
    }
}

#[test]
fn snippet_session은_stale_choice를_좌표변환전에_거절한다() {
    let (mut store, document, view) = fixture("TAIL\n");
    let indent = IndentOptions {
        tab_size: INDENT_SIZE,
        insert_spaces: true,
    };
    let insertion = start(
        &mut store,
        document,
        view,
        "${1|alpha,beta|} ${2:end}$0",
        0,
        indent,
    );
    let session = Session::new(&store, insertion, indent, limits()).unwrap();
    select_all(&mut store, view).unwrap();
    replace_selections(&mut store, view, "", None).unwrap();
    assert!(matches!(
        session.active_choice(&store),
        Err(EditorError::StaleRevision)
    ));
}

#[test]
fn snippet_session의_편집후_선택이탈은_다음_동일group과_undo를_분리한다() {
    let (mut store, document, view) = fixture("TAIL\n");
    let indent = IndentOptions {
        tab_size: INDENT_SIZE,
        insert_spaces: true,
    };
    let insertion = start(
        &mut store,
        document,
        view,
        "${1:漢}$1-${2:end}$0",
        0,
        indent,
    );
    let group = UndoGroup(insertion.revision);
    let mut session = Session::new(&store, insertion, indent, limits()).unwrap();
    session.replace(&mut store, "x", None).unwrap();
    session
        .step(&mut store, true, |_| Err(EditorError::Refused))
        .unwrap();
    session
        .step(&mut store, false, |_| Err(EditorError::Refused))
        .unwrap();
    session.replace(&mut store, "", None).unwrap();
    assert!(!session.is_active());
    let after = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: after.revision,
                group,
                origin: Some(view),
                selection_after: None,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "E".into(),
                }],
            },
        )
        .unwrap();
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        after.rope
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "漢漢-endTAIL\n"
    );
}

fn oracle() -> Value {
    let bun = std::env::var("TAIDE_M8_ORACLE_BUN").unwrap_or_else(|_| "bun".into());
    let output = Command::new(bun)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snippet-session-oracle.mjs"
        ))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn fixture(text: &str) -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            PathBuf::from("/synthetic/session.rs"),
            OpenedFile {
                path: "/synthetic/session.rs".into(),
                content: text.into(),
                language_id: "rust".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "synthetic".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, document, view)
}

fn prepare(
    template: &str,
    replace: Range<usize>,
    column: usize,
    line: &str,
    indent: IndentOptions,
    eol: LineEnding,
) -> PreparedSnippet {
    let parsed = parse_complete(
        template,
        limits(),
        FinalTabstopOptions {
            insert: true,
            enforce: false,
        },
        |pattern, options| {
            Some(RegexMetadata {
                source: pattern.into(),
                ignore_case: options.contains('i'),
                global: options.contains('g'),
            })
        },
    )
    .unwrap();
    let adjusted = adjust_whitespace(
        &parsed,
        WhitespaceContext {
            line,
            byte_column: column,
            indent,
            line_ending: eol,
            adjust_indentation: true,
        },
        limits(),
    )
    .unwrap();
    let expansion = expand(
        &adjusted.markers,
        limits(),
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap();
    PreparedSnippet { replace, expansion }
}

fn start(
    store: &mut EditorStore,
    document: DocumentId,
    view: ViewId,
    template: &str,
    offset: usize,
    indent: IndentOptions,
) -> Insertion {
    let snapshot = store.documents().snapshot(document).unwrap();
    let line = snapshot.rope.line(0).to_string();
    let line = line.trim_end_matches(['\r', '\n']);
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: offset,
                    head: offset,
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let expected = store.views().get(view).unwrap().clone();
    insert(
        store,
        &expected,
        snapshot.revision,
        vec![prepare(
            template,
            offset..offset,
            offset,
            line,
            indent,
            snapshot.metadata.line_ending,
        )],
        limits(),
    )
    .unwrap()
}

fn assert_snapshot(
    store: &EditorStore,
    document: DocumentId,
    view: ViewId,
    session: &Session,
    expected: &Value,
) {
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected["text"].as_str().unwrap(),
        "{expected}"
    );
    let selections = store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections
        .iter()
        .map(|selection| json!([selection.anchor, selection.head]))
        .collect::<Vec<_>>();
    assert_eq!(json!(selections), expected["selections"], "{expected}");
    assert_eq!(
        session.is_active(),
        expected["active"].as_bool().unwrap(),
        "{expected}"
    );
    if session.is_active() {
        assert_eq!(
            session
                .active_choice(store)
                .unwrap()
                .map(|choice| json!(choice.options))
                .unwrap_or(Value::Null),
            expected["choice"],
            "{expected}"
        );
    } else {
        assert!(session.decorations(store).unwrap().is_empty());
    }
}

#[test]
fn snippet_session은_원본_범위stickiness와_실제_편집_tab_choice_transform시점을_보존한다() {
    let cases = oracle();
    let mapping = cases["mapping"].as_array().unwrap();
    assert_eq!(mapping.len(), MAPPING_CASE_COUNT);
    for case in mapping {
        let number = |key| usize::try_from(case[key].as_u64().unwrap()).unwrap();
        let result = map_utf16_range(
            number("start")..number("end"),
            number("editStart")..number("editEnd"),
            number("inserted"),
            case["grow"].as_bool().unwrap(),
            case["force"].as_bool().unwrap(),
        )
        .unwrap();
        assert_eq!(
            json!([result.start, result.end]),
            case["expected"],
            "{case}"
        );
    }
    let sessions = cases["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), SESSION_CASE_COUNT);
    for case in sessions {
        let context = &case["context"];
        let indent = IndentOptions {
            tab_size: context["indentSize"].as_u64().unwrap().try_into().unwrap(),
            insert_spaces: context["insertSpaces"].as_bool().unwrap(),
        };
        let (mut store, document, view) = fixture(case["before"].as_str().unwrap());
        let insertion = start(
            &mut store,
            document,
            view,
            case["template"].as_str().unwrap(),
            context["leading"].as_str().unwrap().len(),
            indent,
        );
        let mut session = Session::new(&store, insertion, indent, limits()).unwrap();
        assert_snapshot(&store, document, view, &session, &case["initial"]);
        for step in case["steps"].as_array().unwrap() {
            if let Some(text) = step["action"]["type"].as_str() {
                session.replace(&mut store, text, None).unwrap();
            } else {
                let evaluations = step["evaluations"].as_array().unwrap();
                let mut evaluation = 0;
                session
                    .step(
                        &mut store,
                        step["action"]["next"].as_bool().unwrap(),
                        |request| {
                            let expected = &evaluations[evaluation];
                            evaluation += 1;
                            assert_eq!(request.cursor_index, 0);
                            assert_eq!(
                                request.line_leading_whitespace,
                                context["leading"].as_str().unwrap()
                            );
                            assert_eq!(request.indent, indent);
                            assert_eq!(
                                request.transform.pattern,
                                expected["pattern"].as_str().unwrap()
                            );
                            assert_eq!(request.value, expected["value"].as_str().unwrap());
                            Ok(expected["result"].as_str().unwrap().into())
                        },
                    )
                    .unwrap();
                assert_eq!(evaluation, evaluations.len());
            }
            assert_snapshot(&store, document, view, &session, step);
        }
        assert_eq!(case["retainedDecorations"], json!(0));
    }
}

#[test]
fn snippet_session은_실패_예산_선택이탈_undo와_owner회수에서_초안과_범위를_보호한다() {
    let indent = IndentOptions {
        tab_size: INDENT_SIZE,
        insert_spaces: true,
    };
    let (mut store, document, view) = fixture("  TAIL\n");
    let insertion = start(
        &mut store,
        document,
        view,
        "${1:foo} ${1/(.*)/${1:/upcase}/} ${2:end}$0",
        "  ".len(),
        indent,
    );
    let mut session = Session::new(&store, insertion, indent, limits()).unwrap();
    let before = store.documents().snapshot(document).unwrap();
    let before_view = store.views().get(view).unwrap().clone();
    let decorations = session.decorations(&store).unwrap();
    assert!(matches!(
        session.step(&mut store, true, |_| Err(EditorError::Refused)),
        Err(EditorError::Refused)
    ));
    assert!(session.is_active());
    assert_eq!(session.decorations(&store).unwrap(), decorations);
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        before.rope
    );
    assert_eq!(
        store.views().get(view).unwrap().selection,
        before_view.selection
    );
    assert!(matches!(
        session.replace(&mut store, &"x".repeat(BYTE_LIMIT), None),
        Err(EditorError::Capacity)
    ));
    assert_eq!(session.decorations(&store).unwrap(), decorations);
    assert_eq!(
        store.documents().snapshot(document).unwrap().revision,
        before.revision
    );
    assert!(store.undo(document).unwrap());
    assert!(!session.synchronize(&store));
    assert!(session.decorations(&store).unwrap().is_empty());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  TAIL\n"
    );
    let insertion = start(
        &mut store,
        document,
        view,
        "${1|alpha,beta|} $1 ${2:end}$0",
        0,
        indent,
    );
    let mut session = Session::new(&store, insertion, indent, limits()).unwrap();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 0, head: 0 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    assert!(!session.synchronize(&store));
    assert!(session.decorations(&store).unwrap().is_empty());
    let insertion = start(&mut store, document, view, "${1:x}${2:y}$0", 0, indent);
    let mut session = Session::new(&store, insertion, indent, limits()).unwrap();
    store.detach_view(view).unwrap();
    assert!(!session.synchronize(&store));
    assert!(session.decorations(&store).unwrap().is_empty());
    assert!(matches!(
        map_utf16_range(1..0, 0..0, 0, true, false),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        map_utf16_range(0..0, 1..0, 0, true, false),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        map_utf16_range(0..0, usize::MAX..usize::MAX, 1, true, false),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        normalize_transform(
            "\n\t",
            "  ",
            IndentOptions {
                tab_size: BYTE_LIMIT.try_into().unwrap(),
                insert_spaces: true
            },
            LineEnding::CrLf,
            REFUSED_BYTE_LIMIT
        ),
        Err(EditorError::Capacity)
    ));
}

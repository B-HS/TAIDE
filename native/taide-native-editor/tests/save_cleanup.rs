use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, EditorError, LineEnding};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::save_cleanup::{CleanupFlags, run};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::{SyntaxSnapshot, Token, TokenKind, TokenLine};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};

const PATH: &str = "/synthetic/cleanup.txt";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;

fn fixture(
    content: &str,
    language: &str,
    limit: usize,
) -> (EditorStore, DocumentId, ViewId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: limit,
    })
    .unwrap();
    let document = store
        .open_file(
            PATH.into(),
            OpenedFile {
                path: PATH.into(),
                content: content.into(),
                language_id: language.into(),
                byte_size: content.len().try_into().unwrap(),
                line_count: content.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let views = ["main", "auxiliary"].map(|window| {
        store
            .attach_view(
                ViewKey {
                    window: window.into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap()
    });
    (store, document, views[0], views[1])
}

fn caret(store: &mut EditorStore, view: ViewId, byte: usize) {
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
}

fn flags() -> CleanupFlags {
    CleanupFlags {
        trim_trailing_whitespace: true,
        insert_final_newline: true,
    }
}

#[test]
fn 원본처럼_undo로_disk와_같아져도_명시적_정착까지_dirty를_유지한다() {
    let (mut store, document, view, _) = fixture("disk", "plaintext", BYTE_LIMIT);
    replace_selections(&mut store, view, "draft ", None).unwrap();
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "disk"
    );
    assert!(store.documents().snapshot(document).unwrap().dirty);
    let saved = store.save_snapshot(document).unwrap();
    assert!(store.mark_saved(saved, None).unwrap());
    assert!(!store.documents().snapshot(document).unwrap().dirty);
}

#[test]
fn 명시적_정리는_ascii_공백만_지우고_crlf_커서와_참여자별_undo를_유지한다() {
    let content = "한😀 \t\r\nsecond \t";
    let (mut store, document, main, auxiliary) = fixture(content, "plaintext", BYTE_LIMIT);
    caret(&mut store, main, content.len());
    caret(&mut store, auxiliary, "한😀 \t".len());
    let output = run(&mut store, document, Some(main), flags(), false).unwrap();
    assert!(output.changed);
    assert!(output.errors.is_empty());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "한😀\r\nsecond\r\n"
    );
    assert_eq!(
        store.views().get(main).unwrap().selection.selections[0].head,
        "한😀\r\nsecond".len()
    );
    assert_eq!(
        store.views().get(auxiliary).unwrap().selection.selections[0].head,
        "한😀".len()
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "한😀\r\nsecond"
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        content
    );
    assert_eq!(
        store.views().get(main).unwrap().selection.selections[0].head,
        content.len()
    );
    assert!(!store.undo(document).unwrap());
    let (mut store, document, main, _) = fixture("\u{a0} \t", "plaintext", BYTE_LIMIT);
    run(&mut store, document, Some(main), flags(), false).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "\u{a0}\n"
    );
    assert_eq!(LineEnding::from_content("a\r\nb\nc"), LineEnding::Lf);
    assert_eq!(LineEnding::from_content("a\rb"), LineEnding::CrLf);
}

#[test]
fn 자동_정리는_같은_줄의_가장_오른쪽_커서와_빈_마지막_줄을_보호한다() {
    let content = "code \t  \n  \t  \n尾 \t";
    let (mut store, document, view, _) = fixture(content, "plaintext", BYTE_LIMIT);
    let heads = [
        "code ".len(),
        "code \t".len(),
        "code \t  \n  ".len(),
        content.len(),
    ];
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: heads
                    .into_iter()
                    .map(|head| Selection { anchor: head, head })
                    .collect(),
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let output = run(&mut store, document, Some(view), flags(), true).unwrap();
    assert!(output.errors.is_empty());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "code \t\n  \n尾 \t\n"
    );
    assert_eq!(
        store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections
            .last()
            .unwrap()
            .head,
        "code \t\n  \n尾 \t".len()
    );
    for content in ["", "  \t", "content\n  \t", "content\n"] {
        let (mut store, document, view, _) = fixture(content, "plaintext", BYTE_LIMIT);
        let output = run(
            &mut store,
            document,
            Some(view),
            CleanupFlags {
                trim_trailing_whitespace: false,
                insert_final_newline: true,
            },
            false,
        )
        .unwrap();
        assert!(!output.changed);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            content
        );
    }
}

#[test]
fn 정리는_분류되지_않은_줄과_문자열_regex를_지우지_않고_토큰_경계를_검증한다() {
    let content = "code \t\n\"literal   \n/regex   \n//comment \t";
    let (mut store, document, view, _) = fixture(content, "rust", BYTE_LIMIT);
    let only_trim = CleanupFlags {
        trim_trailing_whitespace: true,
        insert_final_newline: false,
    };
    assert!(
        !run(&mut store, document, Some(view), only_trim, false)
            .unwrap()
            .changed
    );
    let syntax = SyntaxSnapshot {
        revision: 0,
        language_id: "rust".into(),
        lines: [
            TokenKind::Other,
            TokenKind::String,
            TokenKind::Regex,
            TokenKind::Comment,
        ]
        .into_iter()
        .enumerate()
        .map(|(line, kind)| TokenLine {
            line,
            tokens: vec![Token {
                start_byte: 0,
                kind,
            }],
        })
        .collect(),
    };
    store.install_syntax(document, syntax.clone()).unwrap();
    let mut invalid = syntax.clone();
    invalid.lines[0].tokens[0].start_byte = 1;
    assert_eq!(
        store.install_syntax(document, invalid),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(
        store.syntax(document).unwrap().unwrap().lines.len(),
        syntax.lines.len()
    );
    run(&mut store, document, Some(view), only_trim, false).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "code\n\"literal   \n/regex   \n//comment"
    );
    assert!(store.syntax(document).unwrap().is_none());
    assert_eq!(
        store.install_syntax(document, syntax),
        Err(EditorError::StaleRevision)
    );
    let (mut store, document, _, _) = fixture("한글", "rust", BYTE_LIMIT);
    assert_eq!(
        store.install_syntax(
            document,
            SyntaxSnapshot {
                revision: 0,
                language_id: "rust".into(),
                lines: vec![TokenLine {
                    line: 0,
                    tokens: vec![
                        Token {
                            start_byte: 0,
                            kind: TokenKind::Other
                        },
                        Token {
                            start_byte: 1,
                            kind: TokenKind::String
                        }
                    ]
                }]
            }
        ),
        Err(EditorError::InvalidBoundary)
    );
}

#[test]
fn 참여자_실패와_view_부재는_저장을_취소하지_않고_editorconfig_false가_우선한다() {
    let (mut store, document, view, _) = fixture("abc", "plaintext", "abc".len());
    let output = run(&mut store, document, Some(view), flags(), false).unwrap();
    assert_eq!(output.errors, vec![EditorError::Capacity]);
    assert_eq!(
        store.save_snapshot(document).unwrap().rope().to_string(),
        "abc"
    );
    let (mut store, document, view, _) = fixture("abc \t", "plaintext", BYTE_LIMIT);
    assert!(
        !run(&mut store, document, None, flags(), false)
            .unwrap()
            .changed
    );
    let opened = OpenedFile {
        path: PATH.into(),
        content: "abc \t".into(),
        language_id: "plaintext".into(),
        byte_size: "abc \t".len().try_into().unwrap(),
        line_count: 1,
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 1.0,
        editor_config: EditorConfigOptions {
            trim_trailing_whitespace: Some(false),
            insert_final_newline: Some(false),
            ..Default::default()
        },
    };
    store
        .observe_file(document, std::path::Path::new(PATH), opened)
        .unwrap();
    let output = run(&mut store, document, Some(view), flags(), false).unwrap();
    assert!(!output.changed);
    assert!(output.errors.is_empty());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "abc \t"
    );
}

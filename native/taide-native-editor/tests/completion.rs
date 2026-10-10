use std::path::PathBuf;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, CompletionTextEdit,
    Documentation, InsertReplaceEdit, InsertTextFormat, MarkupContent, MarkupKind,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::completion::{Candidate, Candidates, Command};
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::documentation::Content;
use taide_native_editor::lsp::{LspRange, Position, TextEdit};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet};

const BYTE_LIMIT: usize = 4096;

fn snapshot(text: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            PathBuf::from("/synthetic/completion.rs"),
            OpenedFile {
                path: "/synthetic/completion.rs".into(),
                content: text.into(),
                encoding_lossy: false,
                language_id: "rust".into(),
                tier: FileSizeTier::Normal,
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                modified_ms: 0.0,
                read_only: false,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    store.documents().snapshot(document).unwrap()
}

fn range(start: u32, end: u32) -> LspRange {
    LspRange::new(Position::new(0, start), Position::new(0, end))
}

#[test]
fn 후보는_list_array_null과_문서종류_정렬_필터_불완전_메타데이터를_보존한다() {
    let document = snapshot("한😀con");
    let item = CompletionItem {
        label: "console".into(),
        detail: Some("detail".into()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: "**문서**".into(),
        })),
        sort_text: Some("first".into()),
        filter_text: Some("customFilter".into()),
        data: Some("opaque".into()),
        ..Default::default()
    };
    for response in [
        CompletionResponse::Array(vec![item.clone()]),
        CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: vec![item.clone()],
        }),
    ] {
        let incomplete = matches!(response, CompletionResponse::List(_));
        let result = Candidates::new(&document, Position::new(0, 6), range(3, 6), Some(response));
        assert_eq!(result.is_incomplete, incomplete);
        assert_eq!(result.items.len(), 1);
        let candidate = &result.items[0];
        assert_eq!(candidate.insert, 7..10);
        assert_eq!(candidate.replace, candidate.insert);
        assert_eq!(candidate.text(), "console");
        assert_eq!(candidate.item.kind, Some(CompletionItemKind::TEXT));
        assert_eq!(candidate.item.detail, item.detail);
        assert_eq!(candidate.item.sort_text, item.sort_text);
        assert_eq!(candidate.item.filter_text, item.filter_text);
        assert_eq!(candidate.item.data, item.data);
        assert_eq!(
            candidate.documentation(),
            Some(Content::Markdown("**문서**".into()))
        );
    }
    assert_eq!(
        Candidates::new(&document, Position::new(0, 6), range(3, 6), None),
        Candidates::default()
    );
}

#[test]
fn text_edit가_insert_text와_label보다_우선하고_빈_삽입과_plain_달러를_보존한다() {
    let document = snapshot("con");
    let mut item = CompletionItem {
        label: "label".into(),
        insert_text: Some("$1plain".into()),
        kind: Some(CompletionItemKind::SNIPPET),
        ..Default::default()
    };
    let candidate =
        Candidate::new(&document, Position::new(0, 3), range(0, 3), item.clone()).unwrap();
    assert_eq!(candidate.text(), "$1plain");
    assert!(!candidate.is_snippet());
    assert_eq!(candidate.item.kind, Some(CompletionItemKind::SNIPPET));
    item.insert_text_format = Some(InsertTextFormat::SNIPPET);
    item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
        range: range(0, 3),
        new_text: String::new(),
    }));
    let candidate = Candidate::new(&document, Position::new(0, 3), range(0, 3), item).unwrap();
    assert_eq!(candidate.text(), "");
    assert!(candidate.is_snippet());
}

#[test]
fn insert_replace는_단일줄_같은시작과_접두범위_utf16_경계를_검증한다() {
    let document = snapshot("한😀console\nnext");
    let make = |insert, replace| CompletionItem {
        label: "console".into(),
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: "completed".into(),
            insert,
            replace,
        })),
        ..Default::default()
    };
    let candidate = Candidate::new(
        &document,
        Position::new(0, 6),
        range(3, 6),
        make(range(3, 6), range(3, 10)),
    )
    .unwrap();
    assert_eq!(candidate.insert, 7..10);
    assert_eq!(candidate.replace, 7..14);
    for (insert, replace) in [
        (range(2, 6), range(2, 10)),
        (range(3, 6), range(4, 10)),
        (range(3, 10), range(3, 6)),
        (range(3, 5), range(3, 10)),
        (range(7, 10), range(7, 10)),
        (range(3, 6), range(3, 99)),
        (
            range(3, 6),
            LspRange::new(Position::new(0, 3), Position::new(1, 4)),
        ),
    ] {
        assert!(
            Candidate::new(
                &document,
                Position::new(0, 6),
                range(3, 6),
                make(insert, replace)
            )
            .is_none()
        );
    }
}

#[test]
fn 요청_커서도_utf16_문자경계와_실제_문서위치를_지켜야한다() {
    let document = snapshot("한😀console");
    let item = CompletionItem {
        label: "completed".into(),
        ..Default::default()
    };
    assert!(Candidate::new(&document, Position::new(0, 2), range(0, 10), item.clone()).is_none());
    assert!(Candidate::new(&document, Position::new(0, 3), range(0, 10), item).is_some());
}

#[test]
fn 잘못된_빈라벨_범위는_개별_제외하고_누락_kind는_text로_표시한다() {
    let document = snapshot("con");
    let valid = CompletionItem {
        label: "console".into(),
        ..Default::default()
    };
    let result = Candidates::new(
        &document,
        Position::new(0, 3),
        range(0, 3),
        Some(CompletionResponse::Array(vec![
            CompletionItem::default(),
            CompletionItem {
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: range(3, 0),
                    new_text: "bad".into(),
                })),
                ..valid.clone()
            },
            valid,
        ])),
    );
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].item.kind, Some(CompletionItemKind::TEXT));
}

#[test]
fn 다중커서는_주커서와_같은_접두사만_utf16_단위로_확장한다() {
    let document = snapshot("😀con\n한con\nfoof");
    let candidate = Candidate::new(
        &document,
        Position::new(0, 5),
        range(2, 5),
        CompletionItem {
            label: "console".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let selection = SelectionSet {
        primary: 0,
        selections: [7, 14, 19]
            .map(|byte| Selection {
                anchor: byte,
                head: byte,
            })
            .into(),
    };
    assert_eq!(
        candidate
            .replacement_ranges(&document, &selection, false)
            .unwrap(),
        vec![4..7, 11..14, 19..19]
    );
    let moved = SelectionSet {
        primary: 1,
        ..selection
    };
    assert_eq!(
        candidate.replacement_ranges(&document, &moved, false),
        Err(taide_native_editor::document::EditorError::Refused)
    );
}

#[test]
fn 대체_수락은_같은_접미사만_확장하고_다른_커서의_뒤_텍스트를_보존한다() {
    let document = snapshot("conso\nconso\nconX");
    let candidate = Candidate::new(
        &document,
        Position::new(0, 3),
        range(0, 3),
        CompletionItem {
            label: "console".into(),
            text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
                new_text: "console".into(),
                insert: range(0, 3),
                replace: range(0, 5),
            })),
            ..Default::default()
        },
    )
    .unwrap();
    let selection = SelectionSet {
        primary: 0,
        selections: [3, 9, 15]
            .map(|byte| Selection {
                anchor: byte,
                head: byte,
            })
            .into(),
    };
    assert_eq!(
        candidate
            .replacement_ranges(&document, &selection, false)
            .unwrap(),
        vec![0..3, 6..9, 12..15]
    );
    assert_eq!(
        candidate
            .replacement_ranges(&document, &selection, true)
            .unwrap(),
        vec![0..5, 6..11, 12..15]
    );
}

#[test]
fn 다른_revision에_이전_후보의_삽입범위를_재사용하지않는다() {
    let document = snapshot("con");
    let candidate = Candidate::new(
        &document,
        Position::new(0, 3),
        range(0, 3),
        CompletionItem {
            label: "console".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let selection = SelectionSet {
        primary: 0,
        selections: vec![Selection { anchor: 3, head: 3 }],
    };
    let mut changed = document.clone();
    changed.revision += 1;
    assert_eq!(
        candidate.replacement_ranges(&changed, &selection, false),
        Err(taide_native_editor::document::EditorError::StaleRevision)
    );
}

#[test]
fn 명시_명령과_목록_명령은_원본_id를_소비하고_무관한_명령을_거절한다() {
    assert_eq!(
        Command::from_action("editor.action.triggerSuggest"),
        Some(Command::Trigger)
    );
    assert_eq!(
        Command::from_action("acceptSelectedSuggestionOnEnter"),
        Some(Command::Accept { alternate: false })
    );
    assert_eq!(
        Command::from_action("acceptAlternativeSelectedSuggestion"),
        Some(Command::Accept { alternate: true })
    );
    assert_eq!(
        Command::from_action("hideSuggestWidget"),
        Some(Command::Hide)
    );
    assert_eq!(Command::from_action("editor.action.showHover"), None);
}

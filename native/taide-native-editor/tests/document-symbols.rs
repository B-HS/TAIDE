use lsp_types::{
    DocumentSymbol, DocumentSymbolResponse, Location, Position, Range, SymbolInformation,
    SymbolKind, SymbolTag, Uri,
};
use taide_model::ids::TabId;
use taide_native_editor::document::{Edit, EditorError, UndoGroup};
use taide_native_editor::document_symbols::DocumentSymbols;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

const BYTE_LIMIT: usize = 4096;
const URI: &str = "file:///synthetic/symbols.rs";

fn fixture() -> (EditorStore, taide_native_editor::document::DocumentId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_untitled(
            TabId::new(),
            "class\n  \u{1f600}method\n    value\nend\n",
            "rust".into(),
        )
        .unwrap();
    (store, document)
}

fn node(
    name: &str,
    start: Position,
    end: Position,
    selection: Range,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    DocumentSymbol {
        name: name.into(),
        detail: Some("detail".into()),
        kind: SymbolKind::METHOD,
        tags: Some(vec![SymbolTag::DEPRECATED]),
        deprecated: None,
        range: Range::new(start, end),
        selection_range: selection,
        children: Some(children),
    }
}

fn response() -> DocumentSymbolResponse {
    DocumentSymbolResponse::Nested(vec![node(
        "Class",
        Position::new(0, 0),
        Position::new(3, 3),
        Range::new(Position::new(0, 0), Position::new(0, 5)),
        vec![node(
            "method",
            Position::new(1, 0),
            Position::new(2, 9),
            Range::new(Position::new(1, 4), Position::new(1, 10)),
            vec![node(
                "value",
                Position::new(2, 4),
                Position::new(2, 9),
                Range::new(Position::new(2, 4), Position::new(2, 9)),
                vec![],
            )],
        )],
    )])
}

#[test]
fn 계층과_preorder_breadcrumb은_utf16_선택과_고정줄_범위를_보존한다() {
    let (store, id) = fixture();
    let document = store.documents().snapshot(id).unwrap();
    let model = DocumentSymbols::new(&document, &URI.parse().unwrap(), Some(response())).unwrap();
    let symbols = model.symbols();
    assert_eq!(
        symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["Class", "method", "value"]
    );
    assert_eq!(symbols[1].parent, Some(0));
    assert_eq!(symbols[2].parent, Some(1));
    assert_eq!(symbols[1].container_label, "Class");
    assert_eq!(symbols[2].container_label, "Class > method");
    assert_eq!(
        document
            .rope
            .byte_slice(symbols[1].selection.clone())
            .to_string(),
        "method"
    );
    assert_eq!(symbols[1].tags, [SymbolTag::DEPRECATED]);
    assert_eq!(symbols[1].detail, "detail");
    let sticky = model.sticky_model().unwrap();
    let candidates = sticky.candidates(1..3, &[]);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].scope.start_line, 0);
    assert_eq!(candidates[0].scope.end_line, 4);
    assert_eq!(candidates[1].scope.start_line, 1);
    assert_eq!(candidates[1].level, 1);
}

#[test]
fn flat_결과는_같은_uri만_허용하고_빈결과는_접기_fallback을_보존한다() {
    let (store, id) = fixture();
    let document = store.documents().snapshot(id).unwrap();
    let uri: Uri = URI.parse().unwrap();
    let entries = [uri.clone(), "file:///synthetic/foreign.rs".parse().unwrap()]
        .into_iter()
        .map(|uri| SymbolInformation {
            name: "flat".into(),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            location: Location {
                uri,
                range: Range::new(Position::new(1, 4), Position::new(1, 10)),
            },
            container_name: Some("not-a-tree".into()),
        })
        .collect();
    let model =
        DocumentSymbols::new(&document, &uri, Some(DocumentSymbolResponse::Flat(entries))).unwrap();
    assert_eq!(model.symbols().len(), 1);
    assert!(model.symbols()[0].container_label.is_empty());
    assert_eq!(model.symbols()[0].selection, model.symbols()[0].bytes);
    assert!(model.sticky_model().is_some());
    for response in [
        None,
        Some(DocumentSymbolResponse::Nested(vec![])),
        Some(DocumentSymbolResponse::Flat(vec![])),
    ] {
        let empty = DocumentSymbols::new(&document, &uri, response).unwrap();
        assert!(empty.symbols().is_empty());
        assert!(empty.sticky_model().is_none());
    }
}

#[test]
fn 잘못된_선택과_surrogate_중간은_거절하고_편집_언어_신원_교체는_만료한다() {
    let (mut store, id) = fixture();
    let document = store.documents().snapshot(id).unwrap();
    let uri = URI.parse().unwrap();
    let malformed = node(
        "bad",
        Position::new(0, 0),
        Position::new(0, 5),
        Range::new(Position::new(1, 4), Position::new(1, 10)),
        vec![],
    );
    assert_eq!(
        DocumentSymbols::new(
            &document,
            &uri,
            Some(DocumentSymbolResponse::Nested(vec![malformed]))
        ),
        Err(EditorError::InvalidBoundary)
    );
    let surrogate = node(
        "bad",
        Position::new(1, 0),
        Position::new(1, 10),
        Range::new(Position::new(1, 3), Position::new(1, 4)),
        vec![],
    );
    assert_eq!(
        DocumentSymbols::new(
            &document,
            &uri,
            Some(DocumentSymbolResponse::Nested(vec![surrogate]))
        ),
        Err(EditorError::InvalidBoundary)
    );
    let model = DocumentSymbols::new(&document, &uri, Some(response())).unwrap();
    assert!(model.describes(&document));
    let mut different_language = document.clone();
    different_language.metadata.language_id = "python".into();
    assert!(!model.describes(&different_language));
    let other = store
        .open_untitled(TabId::new(), &document.rope.to_string(), "rust".into())
        .unwrap();
    assert!(!model.describes(&store.documents().snapshot(other).unwrap()));
    store
        .apply(
            id,
            Transaction {
                revision: document.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "new\n".into(),
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    assert!(!model.describes(&store.documents().snapshot(id).unwrap()));
}

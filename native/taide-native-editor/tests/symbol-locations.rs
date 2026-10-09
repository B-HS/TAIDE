use lsp_types::{GotoDefinitionResponse, Location, LocationLink, Position, Range};
use taide_model::ids::TabId;
use taide_native_editor::document::EditorError;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::symbol_locations::{Locations, Target, normalize, preview};

const URI_A: &str = "file:///synthetic/a.rs";
const URI_B: &str = "file:///synthetic/b.rs";
const BYTE_LIMIT: usize = 4096;

fn range(line: u32, start: u32, end: u32) -> Range {
    Range::new(Position::new(line, start), Position::new(line, end))
}

fn target(uri: &str, line: u32) -> Target {
    Target::from_location(Location::new(uri.parse().unwrap(), range(line, 1, 4)))
}

#[test]
fn scalar_array_link는_전체범위와_선택_origin을_보존한다() {
    let scalar = Location::new(URI_A.parse().unwrap(), range(0, 2, 5));
    assert_eq!(
        normalize(Some(GotoDefinitionResponse::Scalar(scalar.clone()))),
        normalize(Some(GotoDefinitionResponse::Array(vec![scalar])))
    );
    let link = LocationLink {
        target_uri: URI_B.parse().unwrap(),
        target_range: Range::new(Position::new(1, 0), Position::new(6, 0)),
        target_selection_range: range(1, 4, 8),
        origin_selection_range: Some(range(0, 1, 5)),
    };
    let entries = normalize(Some(GotoDefinitionResponse::Link(vec![link.clone()])));
    assert_eq!(entries[0].range, link.target_range);
    assert_eq!(entries[0].selection, link.target_selection_range);
    assert_eq!(entries[0].origin, link.origin_selection_range);
    assert!(normalize(None).is_empty());
}

#[test]
fn 파일과_전체범위_정렬_중복제거는_provider_첫위치와_선택범위를_보존한다() {
    let mut first = target(URI_B, 3);
    first.selection = range(3, 2, 3);
    let duplicate = target(URI_B, 3);
    let a_last = target(URI_A, 7);
    let a_first = target(URI_A, 0);
    let model = Locations::new(vec![first.clone(), a_last, duplicate, a_first]);
    assert_eq!(model.targets().len(), 3);
    assert_eq!(model.first(), Some(2));
    assert_eq!(model.targets()[2], first);
    assert_eq!(model.groups()[0].references, 0..2);
    assert_eq!(model.groups()[1].references, 2..3);
    assert_eq!(model.at(&URI_B.parse().unwrap(), Position::new(3, 1)), None);
    assert_eq!(
        model.at(&URI_B.parse().unwrap(), Position::new(3, 2)),
        Some(2)
    );
}

#[test]
fn 반전_선택탈출_origin반전은_거절하고_빈모델은_순환하지않는다() {
    let mut reversed = target(URI_A, 0);
    reversed.range = range(0, 4, 1);
    let mut escaped = target(URI_A, 0);
    escaped.selection = range(0, 3, 8);
    let mut origin = target(URI_A, 0);
    origin.origin = Some(range(0, 8, 3));
    let model = Locations::new(vec![reversed, escaped, origin]);
    assert!(model.targets().is_empty());
    assert!(model.groups().is_empty());
    assert_eq!(model.first(), None);
    assert_eq!(model.next(0, true), None);
}

#[test]
fn 근접선택은_파일을_우선하고_파일경계를_넘어_양방향으로_순환한다() {
    let model = Locations::new(vec![target(URI_B, 1), target(URI_A, 3), target(URI_A, 1)]);
    assert_eq!(
        model.nearest(&URI_A.parse().unwrap(), Position::new(3, 0)),
        Some(1)
    );
    assert_eq!(
        model.nearest(&URI_B.parse().unwrap(), Position::new(3, 0)),
        Some(2)
    );
    assert_eq!(model.next(1, true), Some(2));
    assert_eq!(model.next(2, true), Some(0));
    assert_eq!(model.next(0, false), Some(2));
    assert_eq!(model.next(3, true), None);
}

#[test]
fn 다중_origin_밑줄은_기본_단어와_유효한_모든_범위를_포함한다() {
    let mut a = target(URI_A, 0);
    a.origin = Some(range(0, 1, 7));
    let mut b = target(URI_B, 0);
    b.origin = Some(range(0, 5, 10));
    let model = Locations::new(vec![a, b]);
    assert_eq!(model.origin_range(range(0, 3, 5)), range(0, 1, 10));
}

#[test]
fn 미리보기는_utf16_다중줄과_앞뒤공백_보조단위_문맥을_안전하게_처리한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_untitled(
            TabId::new(),
            "  \u{1f600} name  \n  target trailing  \n",
            "rust".into(),
        )
        .unwrap();
    let document = store.documents().snapshot(id).unwrap();
    let result = preview(&document, range(0, 5, 9), 2, |byte| byte).unwrap();
    assert_eq!(result.text, "\u{1f600} name");
    assert_eq!(&result.text[result.highlight], "name");
    let result = preview(
        &document,
        Range::new(Position::new(0, 5), Position::new(1, 8)),
        8,
        |byte| byte,
    )
    .unwrap();
    assert_eq!(result.text, "\u{1f600} name  \n  target trailing");
    assert_eq!(&result.text[result.highlight], "name  \n  target");
    assert_eq!(
        preview(&document, range(0, 3, 9), 8, |byte| byte),
        Err(EditorError::InvalidBoundary)
    );
}

use std::sync::Arc;

use taide_model::ids::TabId;
use taide_native_editor::diagnostics::{Marker, Message, Severity};
use taide_native_editor::problem_navigation::{Command, Navigation, Problem};
use taide_native_editor::store::{EditorLimits, EditorStore};

const DOCUMENT_BYTES: usize = 1024;

fn fixture() -> (EditorStore, Vec<Problem>) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 3,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: DOCUMENT_BYTES,
    })
    .unwrap();
    let a = store
        .open_untitled(TabId::new(), "abc def ghi", "plaintext".into())
        .unwrap();
    let b = store
        .open_untitled(TabId::new(), "abc def ghi", "plaintext".into())
        .unwrap();
    let problems = [
        (b, "b", 4..7, Severity::Error),
        (a, "a", 0..3, Severity::Warning),
        (a, "a", 8..11, Severity::Hint),
        (a, "a", 4..7, Severity::Error),
    ]
    .into_iter()
    .map(|(document, resource, bytes, severity)| Problem {
        document,
        resource: format!("file:///synthetic/{resource}"),
        initial_range: bytes.clone(),
        marker: Marker {
            bytes,
            message: Arc::new(Message {
                severity,
                text: "diagnostic".into(),
                source: None,
                code: None,
            }),
        },
    })
    .collect();
    (store, problems)
}

#[test]
fn 문제는_uri_severity_범위순으로_정렬하고_hint를_제외하며_반복_이동은_순환한다() {
    let (_store, problems) = fixture();
    let mut navigation = Navigation::default();
    navigation.update(problems);
    let resource = "file:///synthetic/a";
    let first = navigation.navigate(resource, 0, true).unwrap();
    assert_eq!(first.index, 1);
    assert_eq!(first.total, 3);
    assert_eq!(first.problem.marker.bytes, 4..7);
    assert_eq!(
        navigation
            .navigate(resource, 4, true)
            .unwrap()
            .problem
            .marker
            .bytes,
        0..3
    );
    assert_eq!(
        navigation
            .navigate(resource, 0, true)
            .unwrap()
            .problem
            .resource,
        "file:///synthetic/b"
    );
    assert_eq!(
        navigation
            .navigate("file:///synthetic/b", 4, true)
            .unwrap()
            .index,
        1
    );
    assert_eq!(navigation.navigate(resource, 4, false).unwrap().index, 3);
    assert!(Command::NextInFiles.all_files());
    assert!(!Command::Previous.forward());
}

#[test]
fn 첫_위치와_빈범위_단어는_이전방향을_보존하고_커서_이탈과_목록_교체는_순서를_초기화한다() {
    let (_store, mut problems) = fixture();
    problems[3].marker.bytes = 5..5;
    problems[3].initial_range = 4..7;
    let id = problems[3].document;
    let mut navigation = Navigation::default();
    navigation.update(problems.clone());
    assert_eq!(
        navigation
            .navigate("file:///synthetic/a", 6, false)
            .unwrap()
            .index,
        1
    );
    navigation.follow_cursor(id, 10);
    assert_eq!(
        navigation
            .navigate("file:///synthetic/a", 10, true)
            .unwrap()
            .index,
        3
    );
    navigation.reset();
    assert_eq!(
        navigation
            .navigate("file:///synthetic/ab", 0, false)
            .unwrap()
            .index,
        2
    );
    navigation.update(Vec::new());
    assert!(
        navigation
            .navigate("file:///synthetic/a", 0, true)
            .is_none()
    );
    navigation.update(problems);
    assert_eq!(
        navigation
            .navigate("file:///synthetic/a", 5, true)
            .unwrap()
            .index,
        1
    );
    navigation.follow_cursor(id, 5);
    assert_eq!(
        navigation
            .navigate("file:///synthetic/a", 5, true)
            .unwrap()
            .index,
        2
    );
}

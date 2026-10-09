use super::*;
use taide_model::tree::TreeEntryKind;
use taide_native_editor::document_symbols::SymbolKind;

fn symbol(name: &str, parent: Option<usize>, bytes: std::ops::Range<usize>) -> Symbol {
    Symbol {
        name: name.into(),
        detail: String::new(),
        kind: SymbolKind::FUNCTION,
        tags: Vec::new(),
        parent,
        container_label: String::new(),
        selection: bytes.clone(),
        bytes,
    }
}

#[test]
fn outline은_동명이인과_접힌_자손_부모_이동을_트리_위치로_구별한다() {
    let symbols = vec![
        symbol("A", None, 0..10),
        symbol("overload", Some(0), 1..2),
        symbol("overload", Some(0), 3..8),
        symbol("inner", Some(2), 4..5),
        symbol("B", None, 11..20),
    ];
    let tree = Tree::new(&symbols);
    let expanded = tree.rows(&HashSet::new());
    assert_eq!(
        expanded
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["/0", "/0/0", "/0/1", "/0/1/0", "/1"]
    );
    assert_eq!(
        expanded.iter().map(|row| row.depth).collect::<Vec<_>>(),
        [0, 1, 1, 2, 0]
    );
    assert_eq!(tree.children(None), [0, 4]);
    assert_eq!(tree.children(Some(0)), [1, 2]);
    assert_eq!(tree.parent(3), Some(2));
    assert_eq!(tree.parent(0), None);
    assert_eq!(tree.id(3), Some("/0/1/0"));
    let collapsed = HashSet::from(["/0/1".into()]);
    let rows = tree.rows(&collapsed);
    assert_eq!(
        rows.iter().map(|row| row.symbol).collect::<Vec<_>>(),
        [0, 1, 2, 4]
    );
    assert!(rows[2].collapsed && rows[2].has_children);
    assert_eq!(tree.rows(&HashSet::from(["/0".into()])).len(), 2);
    assert!(Tree::new(&[]).rows(&HashSet::new()).is_empty());
}

#[test]
fn breadcrumb은_경계와_원본_첫_포함_순서_형제를_보존하고_깊은_계층도_순회한다() {
    let symbols = vec![
        symbol("outer", None, 10..90),
        symbol("first", Some(0), 20..40),
        symbol("inner", Some(1), 25..35),
        symbol("overlap", Some(0), 30..60),
        symbol("next", None, 100..120),
    ];
    let tree = Tree::new(&symbols);
    assert_eq!(tree.enclosing(&symbols, 30), [0, 1, 2]);
    assert_eq!(tree.enclosing(&symbols, 20), [0, 1]);
    assert_eq!(tree.enclosing(&symbols, 40), [0, 1]);
    assert_eq!(tree.enclosing(&symbols, 75), [0]);
    assert_eq!(tree.enclosing(&symbols, 90), [0]);
    assert_eq!(tree.enclosing(&symbols, 100), [4]);
    assert!(tree.enclosing(&symbols, 9).is_empty());
    const DEPTH: usize = 4096;
    let deep = (0..DEPTH)
        .map(|index| symbol("deep", index.checked_sub(1), 0..1))
        .collect::<Vec<_>>();
    assert_eq!(Tree::new(&deep).enclosing(&deep, 1).len(), DEPTH);
}

#[test]
fn 경로_메뉴는_루트와_직계_형제_및_루트_밖의_실제_경로를_보존한다() {
    let segments = path_segments("/project/", "/project/src/widgets/문서.rs");
    assert_eq!(
        segments
            .iter()
            .map(|segment| segment.path.as_str())
            .collect::<Vec<_>>(),
        [
            "/project/src",
            "/project/src/widgets",
            "/project/src/widgets/문서.rs"
        ]
    );
    assert_eq!(segments[0].parent, "/project");
    assert_eq!(path_segments("/", "/src/file.rs")[0].path, "/src");
    let outside = path_segments("/project", "/project-other/file.rs");
    assert_eq!(outside[0].path, "/project-other");
    assert_eq!(outside[0].parent, "/");
    let rows = [
        ("/project/src/a.rs", "a.rs", TreeEntryKind::File),
        ("/project/src/deep", "deep", TreeEntryKind::Directory),
        ("/project/src/deep/b.rs", "b.rs", TreeEntryKind::File),
        ("/project/readme", "readme", TreeEntryKind::File),
    ]
    .into_iter()
    .map(|(path, name, kind)| TreeRow {
        path: path.into(),
        name: name.into(),
        kind,
        depth: 0,
        expanded: false,
        has_children: false,
    })
    .collect::<Vec<_>>();
    assert_eq!(
        direct_children(&rows, "/project/src")
            .iter()
            .map(|row| row.path.as_str())
            .collect::<Vec<_>>(),
        ["/project/src/a.rs", "/project/src/deep"]
    );
    assert!(direct_children(&rows, "/missing").is_empty());
}

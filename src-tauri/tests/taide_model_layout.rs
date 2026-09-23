use taide_lib::domain::layout::types::{ProjectLayout, TabKind};
use taide_model::layout::{ProjectLayout as ModelProjectLayout, TabKind as ModelTabKind};

fn facade_layout(layout: ModelProjectLayout) -> ProjectLayout {
    layout
}

#[test]
fn 구버전_레이아웃의_필드_기본값과_공개_타입을_유지한다() {
    let legacy = serde_json::json!({
        "version": 1,
        "root": {
            "node": "leaf",
            "id": "pane-1",
            "tabs": [{ "id": "tab-1", "kind": { "kind": "file", "path": "src/main.rs" }, "title": "main.rs" }],
            "active": "tab-1"
        },
        "focusedPane": "pane-1"
    });
    let layout: ModelProjectLayout = serde_json::from_value(legacy).expect("구버전 layout.json");
    let facade = facade_layout(layout);
    assert_eq!(facade.version, 1);
    assert_eq!(facade.revision, 0);
    assert!(facade.closed_tabs.is_empty());
    assert!(facade.auxiliary_windows.is_empty());
    assert!(!facade.shell_view.zen);
    assert!(!facade.shell_view.sidebar_collapsed);
}

#[test]
fn diff_tab과_앱_파일_검색_탭의_기존_wire가_유지된다() {
    let diff: ModelTabKind = serde_json::from_value(serde_json::json!({
        "kind": "diff", "path": "main.rs", "staged": false
    }))
    .expect("구버전 Diff 탭");
    let _: TabKind = diff.clone();
    match diff {
        ModelTabKind::Diff {
            rev,
            parent_rev,
            before_path,
            ..
        } => {
            assert!(rev.is_none() && parent_rev.is_none() && before_path.is_none());
        }
        _ => panic!("Diff 탭이어야 합니다"),
    }

    let app_file = serde_json::json!({ "kind": "appFile", "target": { "kind": "prompt", "id": "auto-tab-default" } });
    let target: ModelTabKind = serde_json::from_value(app_file.clone()).expect("앱 파일 탭");
    assert_eq!(serde_json::to_value(target).expect("앱 파일 탭 직렬화"), app_file);

    let search: ModelTabKind = serde_json::from_value(serde_json::json!({
        "kind": "searchEditor", "query": { "text": "needle" }
    }))
    .expect("구버전 검색 탭");
    match search {
        ModelTabKind::SearchEditor { query } => assert!(query.respect_gitignore),
        _ => panic!("검색 탭이어야 합니다"),
    }
}

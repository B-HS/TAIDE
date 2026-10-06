use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui::ViewportId;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{PaneNode, Tab, TabKind};
use taide_native_app::editor_reveal::{Position, REVEAL_TTL, Reveals, Target};
use taide_native_app::terminal_tabs::OpenedFileLink;

const REPLACEMENT_LINE: f64 = 7.0;
const NEXT_REQUEST: Duration = Duration::from_millis(1);
const TARGET_LINE: f64 = 120.0;
const TARGET_COLUMN: f64 = 9.0;

#[test]
fn 이미_열린_파일_탭의_줄_이동은_열기_없이_같은_reveal_대기열을_쓴다() {
    let project = ProjectId::new();
    let pane = PaneId::new();
    let path = "/synthetic/editor.rs";
    let file = |kind| Tab {
        id: TabId::new(),
        kind,
        title: "synthetic reveal".into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    let editor = file(TabKind::File { path: path.into() });
    let untitled = file(TabKind::Untitled { index: 1 });
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane,
        tabs: vec![editor.clone(), untitled.clone()],
        active: Some(editor.id.clone()),
    };
    let layouts = HashMap::from([(project.clone(), layout)]);
    let mut reveals = Reveals::default();
    let now = Instant::now();
    let position = || Position {
        line: TARGET_LINE,
        column: TARGET_COLUMN,
    };
    assert!(reveals.queue_position(
        Target {
            project: &project,
            tab: &editor.id,
            path,
            viewport: ViewportId::ROOT,
        },
        position(),
        &layouts,
        now,
    ));
    let revealed = reveals
        .consume(&editor.id, path, ViewportId::ROOT, now)
        .unwrap();
    assert_eq!(
        (revealed.line, revealed.column),
        (TARGET_LINE, TARGET_COLUMN)
    );
    assert!(
        !reveals.queue_position(
            Target {
                project: &project,
                tab: &untitled.id,
                path,
                viewport: ViewportId::ROOT,
            },
            position(),
            &layouts,
            now,
        ),
        "파일 탭이 아니면 줄 이동을 대기시키지 않는다"
    );
    assert!(
        !reveals.queue_position(
            Target {
                project: &ProjectId::new(),
                tab: &editor.id,
                path,
                viewport: ViewportId::ROOT,
            },
            position(),
            &layouts,
            now,
        ),
        "다른 프로젝트의 탭으로는 대기시키지 않는다"
    );
}

#[test]
fn reveal은_실제_tab_id와_최신요청_5초기한_창과_닫힘을_보존한다() {
    let project = ProjectId::new();
    let pane = PaneId::new();
    let tab = TabId::new();
    let path = "/synthetic/editor.rs";
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::File { path: path.into() },
            title: "editor.rs".into(),
            pinned: false,
            preview: true,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    let mut layouts = HashMap::from([(project.clone(), layout.clone())]);
    let mut opened = OpenedFileLink {
        project: project.clone(),
        pane,
        tab: tab.clone(),
        path: path.into(),
        line: 1.0,
        column: 1.0,
        viewport: ViewportId::ROOT,
        layout,
    };
    let mut reveals = Reveals::default();
    let now = Instant::now();
    assert!(reveals.queue(&opened, &layouts, now));
    assert!(
        reveals
            .consume(&TabId::new(), path, ViewportId::ROOT, now)
            .is_none()
    );
    assert!(
        reveals
            .consume(&tab, "/synthetic/other.rs", ViewportId::ROOT, now)
            .is_none()
    );
    assert!(
        reveals
            .consume(&tab, path, ViewportId::from_hash_of("other"), now)
            .is_none()
    );
    opened.line = REPLACEMENT_LINE;
    assert!(reveals.queue(&opened, &layouts, now + NEXT_REQUEST));
    let position = reveals
        .consume(&tab, path, ViewportId::ROOT, now + REVEAL_TTL)
        .unwrap();
    assert_eq!(position.line, REPLACEMENT_LINE);
    assert!(
        reveals
            .consume(&tab, path, ViewportId::ROOT, now + REVEAL_TTL)
            .is_none()
    );
    assert!(reveals.queue(&opened, &layouts, now));
    assert!(
        reveals
            .consume(&tab, path, ViewportId::ROOT, now + REVEAL_TTL)
            .is_none()
    );
    assert!(reveals.queue(&opened, &layouts, now));
    taide_layout::service::close_tab(layouts.get_mut(&project).unwrap(), &tab).unwrap();
    reveals.reconcile(&layouts, now);
    assert!(reveals.consume(&tab, path, ViewportId::ROOT, now).is_none());
    assert!(!reveals.queue(&opened, &layouts, now));
    layouts.insert(project.clone(), opened.layout.clone());
    assert!(reveals.queue(&opened, &layouts, now));
    reveals.clear();
    assert!(reveals.consume(&tab, path, ViewportId::ROOT, now).is_none());
    assert!(reveals.queue(&opened, &layouts, now));
    layouts.clear();
    reveals.reconcile(&layouts, now);
    assert!(reveals.consume(&tab, path, ViewportId::ROOT, now).is_none());
}

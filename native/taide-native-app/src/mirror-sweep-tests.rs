use std::path::PathBuf;

use super::*;
use taide_model::ids::PaneId;
use taide_model::layout::{AuxWindowLayout, ClosedTab, PaneNode, SplitDir};

const AUXILIARY_SLOT: u32 = 1;

fn tab(kind: TabKind) -> Tab {
    Tab {
        id: TabId::new(),
        kind,
        title: "synthetic".into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn leaf(tabs: Vec<Tab>) -> PaneNode {
    PaneNode::Leaf {
        id: PaneId::new(),
        active: tabs.first().map(|tab| tab.id.clone()),
        tabs,
    }
}

fn file(path: &Path) -> Tab {
    tab(TabKind::File {
        path: path.to_str().unwrap().into(),
    })
}

fn untitled(index: u32) -> Tab {
    tab(TabKind::Untitled { index })
}

fn slots(leaves: &[(&ShellSlotId, &ProjectId)]) -> ShellSlotTree {
    ShellSlotTree::Split {
        dir: SplitDir::Horizontal,
        children: leaves
            .iter()
            .map(|(slot_id, project_id)| ShellSlotTree::Leaf {
                slot_id: (*slot_id).clone(),
                project_id: (*project_id).clone(),
            })
            .collect(),
        sizes: vec![1.0; leaves.len()],
    }
}

#[test]
fn sweep는_editor_영역이_프로젝트를_처음_보일_때마다_한번씩만_예약한다() {
    let (alpha, beta, gamma) = (ProjectId::new(), ProjectId::new(), ProjectId::new());
    let (first, second, window) = (ShellSlotId::new(), ShellSlotId::new(), ShellSlotId::new());
    let mut layouts: HashMap<ProjectId, ProjectLayout> = [&alpha, &beta]
        .into_iter()
        .map(|project| (project.clone(), taide_layout::service::default_layout()))
        .collect();
    let windows = HashMap::from([(window.clone(), alpha.clone())]);
    let mut sweeps = MirrorSweeps::default();

    let tree = slots(&[(&first, &alpha), (&second, &beta)]);
    let mut due = sweeps.due(Some(&tree), &windows, &layouts);
    due.sort_by_key(ToString::to_string);
    let mut expected = vec![alpha.clone(), beta.clone()];
    expected.sort_by_key(ToString::to_string);
    assert_eq!(due, expected);
    assert!(sweeps.due(Some(&tree), &windows, &layouts).is_empty());

    let switched = slots(&[(&first, &beta), (&second, &beta)]);
    assert_eq!(
        sweeps.due(Some(&switched), &windows, &layouts),
        vec![beta.clone()]
    );
    assert_eq!(
        sweeps.due(Some(&tree), &windows, &layouts),
        vec![alpha.clone()]
    );

    let pending = slots(&[(&first, &alpha), (&second, &gamma)]);
    assert!(sweeps.due(Some(&pending), &windows, &layouts).is_empty());
    layouts.insert(gamma.clone(), taide_layout::service::default_layout());
    assert_eq!(
        sweeps.due(Some(&pending), &windows, &layouts),
        vec![gamma.clone()]
    );

    assert!(
        sweeps
            .due(Some(&pending), &HashMap::new(), &layouts)
            .is_empty()
    );
    assert_eq!(
        sweeps.due(Some(&pending), &windows, &layouts),
        vec![alpha.clone()]
    );
    assert!(sweeps.due(None, &HashMap::new(), &layouts).is_empty());
    assert_eq!(
        sweeps.due(Some(&pending), &HashMap::new(), &layouts).len(),
        2
    );
}

#[test]
fn sweep_제출에_실패한_프로젝트는_다음_pass에_한번_다시_예약한다() {
    let (alpha, beta) = (ProjectId::new(), ProjectId::new());
    let (first, second, window) = (ShellSlotId::new(), ShellSlotId::new(), ShellSlotId::new());
    let layouts: HashMap<ProjectId, ProjectLayout> = [&alpha, &beta]
        .into_iter()
        .map(|project| (project.clone(), taide_layout::service::default_layout()))
        .collect();
    let windows = HashMap::from([(window.clone(), alpha.clone())]);
    let tree = slots(&[(&first, &alpha), (&second, &beta)]);
    let mut sweeps = MirrorSweeps::default();

    assert_eq!(sweeps.due(Some(&tree), &windows, &layouts).len(), 2);
    sweeps.retry(&alpha);
    assert_eq!(
        sweeps.due(Some(&tree), &windows, &layouts),
        vec![alpha.clone()]
    );
    assert!(sweeps.due(Some(&tree), &windows, &layouts).is_empty());

    sweeps.retry(&ProjectId::new());
    assert!(sweeps.due(Some(&tree), &windows, &layouts).is_empty());
}

#[cfg(unix)]
#[test]
fn keep_목록은_모든_창의_열린_파일_경로와_canonical_경로_및_열린_닫힌_untitled_탭이다() {
    let directory = std::env::temp_dir().join(format!("taide-native-mirror-keep-{}", TabId::new()));
    let root = directory.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let link = directory.join("link");
    std::os::unix::fs::symlink(&root, &link).unwrap();
    let canonical_root = root.canonicalize().unwrap();
    let main_file = file(&link.join("main.rs"));
    let auxiliary_file = file(&link.join("deleted.rs"));
    let closed_file = file(&root.join("closed.rs"));
    let open_untitled = untitled(1);
    let closed_untitled = untitled(2);
    std::fs::write(root.join("main.rs"), "synthetic\n").unwrap();
    let mut layout = taide_layout::service::default_layout();
    layout.root = leaf(vec![main_file, open_untitled.clone()]);
    layout.auxiliary_windows = vec![AuxWindowLayout {
        slot: AUXILIARY_SLOT,
        root: leaf(vec![auxiliary_file]),
        focused_pane: PaneId::new(),
    }];
    layout.closed_tabs = [closed_file, closed_untitled.clone()]
        .into_iter()
        .map(|tab| ClosedTab {
            tab,
            pane_id: PaneId::new(),
            index: 0,
        })
        .collect();

    let (paths, tab_ids) = keep(&layout);
    let text = |path: PathBuf| path.to_str().unwrap().to_owned();
    assert_eq!(
        paths,
        vec![
            text(link.join("main.rs")),
            text(canonical_root.join("main.rs")),
            text(link.join("deleted.rs")),
            text(canonical_root.join("deleted.rs")),
        ]
    );
    assert_eq!(tab_ids, vec![open_untitled.id, closed_untitled.id]);
    std::fs::remove_dir_all(directory).unwrap();
}

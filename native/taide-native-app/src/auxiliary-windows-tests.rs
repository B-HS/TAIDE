use super::*;

const FIRST_SLOT: u32 = 1;
const SECOND_SLOT: u32 = 2;
const OTHER_SLOT: u32 = 5;

fn project(name: &str, root_missing: bool) -> (ProjectId, Project) {
    let id = ProjectId::from(format!("prj-{name}"));
    (
        id.clone(),
        Project {
            id,
            root: format!("/synthetic/{name}"),
            name: name.into(),
            capabilities: Vec::new(),
            root_missing,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    )
}

fn layout_with_slots(slots: &[u32]) -> ProjectLayout {
    let mut layout = taide_layout::service::default_layout();
    layout.auxiliary_windows = slots
        .iter()
        .map(|slot| AuxWindowLayout {
            slot: *slot,
            root: layout.root.clone(),
            focused_pane: layout.focused_pane.clone(),
        })
        .collect();
    layout
}

#[test]
fn 복원_계획은_root가_있는_열린_프로젝트의_슬롯만_기록_순서대로_펼친다() {
    let (alive, alive_project) = project("alive", false);
    let (gone, gone_project) = project("gone", true);
    let (other, other_project) = project("other", false);
    let (closed, _) = project("closed", false);
    let projects = HashMap::from([
        (alive.clone(), alive_project),
        (gone.clone(), gone_project),
        (other.clone(), other_project),
    ]);
    let layouts = HashMap::from([
        (alive.clone(), layout_with_slots(&[SECOND_SLOT, FIRST_SLOT])),
        (gone.clone(), layout_with_slots(&[FIRST_SLOT])),
        (other.clone(), layout_with_slots(&[OTHER_SLOT])),
        (closed.clone(), layout_with_slots(&[FIRST_SLOT])),
    ]);

    assert_eq!(
        restoration_plan(
            &[alive.clone(), gone.clone(), other.clone(), closed],
            &projects,
            &layouts
        ),
        vec![
            (alive.clone(), SECOND_SLOT),
            (alive.clone(), FIRST_SLOT),
            (other.clone(), OTHER_SLOT)
        ]
    );
    assert!(restoration_plan(&[alive], &projects, &HashMap::new()).is_empty());
    assert!(restoration_plan(&[], &projects, &layouts).is_empty());
}

#[test]
fn 보조_창_라벨은_사용중인_가장_작은_editor_번호를_쓴다() {
    assert_eq!(next_label(std::iter::empty()), "editor-1");
    assert_eq!(next_label(["editor-1", "editor-2"].into_iter()), "editor-3");
    assert_eq!(next_label(["editor-1", "editor-3"].into_iter()), "editor-2");
    assert_eq!(
        next_label(["main", "editor", "editor-x"].into_iter()),
        "editor-1"
    );
}

#[test]
fn 복원과_닫기는_runtime_창_등록부와_viewport_식별자를_함께_관리한다() {
    let (alive, alive_project) = project("alive", false);
    let projects = HashMap::from([(alive.clone(), alive_project)]);
    let layouts = HashMap::from([(alive.clone(), layout_with_slots(&[FIRST_SLOT, SECOND_SLOT]))]);
    let registry = WindowRegistry::default();
    let mut windows = Windows::restore(&[alive.clone()], &projects, &layouts, &registry);

    let labels: Vec<_> = windows
        .windows()
        .iter()
        .map(|window| window.label.clone())
        .collect();
    assert_eq!(labels, vec!["editor-1", "editor-2"]);
    assert_eq!(
        registry.label_for(&alive, FIRST_SLOT).as_deref(),
        Some("editor-1")
    );
    assert_ne!(windows.windows()[0].viewport, windows.windows()[1].viewport);
    assert_ne!(windows.windows()[0].viewport, egui::ViewportId::ROOT);
    assert!(
        windows
            .windows()
            .iter()
            .all(|window| window.is_explorer_open)
    );
    assert!(windows.windows()[0].is_alive(&layouts));

    windows.toggle_explorer(&alive, SECOND_SLOT);
    assert!(!windows.find(&alive, SECOND_SLOT).unwrap().is_explorer_open);
    assert!(windows.find(&alive, FIRST_SLOT).unwrap().is_explorer_open);

    let closed = windows.close("editor-1", &registry).unwrap();
    assert_eq!(closed.slot, FIRST_SLOT);
    assert!(registry.label_for(&alive, FIRST_SLOT).is_none());
    assert!(windows.close("editor-1", &registry).is_none());
    windows.open(alive.clone(), OTHER_SLOT, &registry);
    assert_eq!(
        registry.label_for(&alive, OTHER_SLOT).as_deref(),
        Some("editor-1")
    );
    assert_eq!(windows.explorer_mounts().len(), windows.windows().len());

    let mut emptied = layouts.clone();
    emptied.get_mut(&alive).unwrap().auxiliary_windows[1].root =
        taide_model::layout::PaneNode::Leaf {
            id: taide_model::ids::PaneId::new(),
            tabs: Vec::new(),
            active: None,
        };
    let second = windows.find(&alive, SECOND_SLOT).unwrap();
    assert!(!second.is_alive(&emptied));
    assert!(!second.is_alive(&HashMap::new()));
}

#[test]
fn 이동용_예약_라벨은_열린_창과_다른_예약을_피하고_채택_해제로_반납된다() {
    let (alive, alive_project) = project("alive", false);
    let projects = HashMap::from([(alive.clone(), alive_project)]);
    let layouts = HashMap::from([(alive.clone(), layout_with_slots(&[FIRST_SLOT]))]);
    let registry = WindowRegistry::default();
    let mut windows = Windows::restore(&[alive.clone()], &projects, &layouts, &registry);

    let reserved = windows.reserve();
    let other = windows.reserve();
    assert_eq!(
        (reserved.as_str(), other.as_str()),
        ("editor-2", "editor-3")
    );
    assert!(registry.label_for(&alive, SECOND_SLOT).is_none());

    windows.adopt(alive.clone(), SECOND_SLOT, reserved.clone());
    let adopted = windows.find(&alive, SECOND_SLOT).unwrap();
    assert_eq!(adopted.label, reserved);
    assert!(adopted.is_explorer_open);
    assert_eq!(windows.reserve(), "editor-4");

    windows.release(&other);
    windows.release("editor-4");
    assert_eq!(windows.reserve(), "editor-3");

    windows.toggle_explorer(&alive, SECOND_SLOT);
    let mount = windows.show_explorer(&alive, SECOND_SLOT).unwrap();
    let shown = windows.find(&alive, SECOND_SLOT).unwrap();
    assert!(shown.is_explorer_open);
    assert_eq!(mount, shown.explorer);
    assert!(windows.show_explorer(&alive, OTHER_SLOT).is_none());
}

#[test]
fn 연결을_다시_맺으면_응답을_잃은_새_창_이동은_등록부로_채택하고_남은_예약은_푼다() {
    let (alive, alive_project) = project("alive", false);
    let projects = HashMap::from([(alive.clone(), alive_project)]);
    let mut layouts = HashMap::from([(alive.clone(), layout_with_slots(&[FIRST_SLOT]))]);
    let registry = WindowRegistry::default();
    let mut windows = Windows::restore(&[alive.clone()], &projects, &layouts, &registry);
    let moved = windows.reserve();
    let emptied = windows.reserve();
    let unsent = windows.reserve();
    registry.register(moved.clone(), alive.clone(), SECOND_SLOT);
    registry.register(emptied.clone(), alive.clone(), OTHER_SLOT);
    layouts.insert(alive.clone(), layout_with_slots(&[FIRST_SLOT, SECOND_SLOT]));

    windows.settle_reserved(&layouts, &registry);

    let labels: Vec<_> = windows
        .windows()
        .iter()
        .map(|window| window.label.as_str())
        .collect();
    assert_eq!(labels, vec!["editor-1", moved.as_str()]);
    assert!(windows.find(&alive, SECOND_SLOT).unwrap().is_explorer_open);
    assert_eq!(
        registry.label_for(&alive, FIRST_SLOT).as_deref(),
        Some("editor-1")
    );
    assert_eq!(
        registry.label_for(&alive, SECOND_SLOT).as_deref(),
        Some(moved.as_str())
    );
    assert!(registry.label_for(&alive, OTHER_SLOT).is_none());
    assert_eq!(windows.reserve(), emptied);
    assert_eq!(windows.reserve(), unsent);
}

#[test]
fn 닫힌_창의_탭_복귀는_제출될_때까지_대기하고_같은_슬롯을_다시_열면_취소된다() {
    let (alive, alive_project) = project("alive", false);
    let projects = HashMap::from([(alive.clone(), alive_project)]);
    let layouts = HashMap::from([(alive.clone(), layout_with_slots(&[FIRST_SLOT, SECOND_SLOT]))]);
    let registry = WindowRegistry::default();
    let mut windows = Windows::restore(&[alive.clone()], &projects, &layouts, &registry);
    assert!(windows.pending_returns().is_empty());

    windows.close("editor-1", &registry).unwrap();
    windows.close("editor-2", &registry).unwrap();
    assert!(windows.close("editor-1", &registry).is_none());
    assert_eq!(
        windows.pending_returns(),
        vec![(alive.clone(), FIRST_SLOT), (alive.clone(), SECOND_SLOT)]
    );

    windows.returned(&alive, FIRST_SLOT);
    assert_eq!(
        windows.pending_returns(),
        vec![(alive.clone(), SECOND_SLOT)]
    );
    windows.returned(&alive, OTHER_SLOT);
    assert_eq!(
        windows.pending_returns(),
        vec![(alive.clone(), SECOND_SLOT)]
    );

    windows.open(alive.clone(), SECOND_SLOT, &registry);
    assert!(windows.pending_returns().is_empty());
    assert!(windows.find(&alive, SECOND_SLOT).is_some());
}

#[test]
fn 보조_창_viewport는_원본_크기와_overlay_타이틀바와_프로젝트_제목을_쓴다() {
    let builder = builder(title("alpha", "TAIDE"));
    assert_eq!(builder.title.as_deref(), Some("alpha — TAIDE"));
    assert_eq!(
        builder.inner_size,
        Some(egui::vec2(DEFAULT_SIZE[0], DEFAULT_SIZE[1]))
    );
    assert_eq!(
        builder.min_inner_size,
        Some(egui::vec2(MINIMUM_SIZE[0], MINIMUM_SIZE[1]))
    );
    assert_eq!(builder.fullsize_content_view, Some(true));
    assert_eq!(builder.title_shown, Some(false));
    assert_eq!(builder.titlebar_shown, Some(false));
}

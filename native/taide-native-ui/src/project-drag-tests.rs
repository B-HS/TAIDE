use super::*;
use egui::Vec2;
use std::collections::HashMap;
use taide_model::layout::SplitDir;
use taide_model::project::{
    ProjectDisplay, ProjectGroup, ProjectRef, SessionShellState, ShellSlotTree, WindowChrome,
};

const RAIL: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(56.0, 600.0));
const RAIL_ICON: Rect = Rect::from_min_max(pos2(8.0, 8.0), pos2(48.0, 48.0));
const SLOT_BODY: Rect = Rect::from_min_max(pos2(56.0, 24.0), pos2(1000.0, 600.0));
const RAIL_GAP: Pos2 = pos2(28.0, 300.0);
const SLOT_HEADER: Pos2 = pos2(400.0, 8.0);
const SLOT_CENTER: Pos2 = pos2(400.0, 300.0);
const ITEM_SIZE: f32 = 40.0;
const ITEM_GAP: f32 = 4.0;
const SECTION_GAP: f32 = 82.5;
const HEADER_HEIGHT: f32 = 17.5;

struct Fixture {
    snapshot: ShellSnapshot,
    projects: [ProjectId; 3],
    groups: [ProjectGroupId; 2],
    slots: [ShellSlotId; 2],
}

fn fixture() -> Fixture {
    let projects = [ProjectId::new(), ProjectId::new(), ProjectId::new()];
    let groups = [ProjectGroupId::new(), ProjectGroupId::new()];
    let slots = [ShellSlotId::new(), ShellSlotId::new()];
    let snapshot = ShellSnapshot {
        projects: projects
            .iter()
            .map(|id| ProjectRef {
                id: id.clone(),
                root: "/synthetic".into(),
                name: id.to_string(),
                display: ProjectDisplay::default(),
                root_missing: false,
            })
            .collect(),
        groups: groups
            .iter()
            .zip(&projects)
            .map(|(id, member)| ProjectGroup {
                id: id.clone(),
                name: id.to_string(),
                color: None,
                members: vec![member.clone()],
                collapsed: false,
            })
            .collect(),
        shell: SessionShellState {
            tree: Some(ShellSlotTree::Split {
                dir: SplitDir::Horizontal,
                children: slots
                    .iter()
                    .zip(&projects)
                    .map(|(slot, project)| ShellSlotTree::Leaf {
                        slot_id: slot.clone(),
                        project_id: project.clone(),
                    })
                    .collect(),
                sizes: vec![50.0, 50.0],
            }),
            focused: Some(slots[0].clone()),
            window_chrome: WindowChrome::default(),
        },
        layouts: HashMap::new(),
        hide_status_in_zen: true,
        resizer_thickness: 1.0,
        welcome_on_empty_editor: true,
    };
    Fixture {
        snapshot,
        projects,
        groups,
        slots,
    }
}

fn slot_target(slot: &ShellSlotId, edge: DropEdge) -> DropTarget {
    DropTarget::Slot {
        slot: slot.clone(),
        edge,
    }
}

fn project(id: &ProjectId) -> RailItem {
    RailItem::Project(id.clone())
}

fn collision_frame(icon: &ProjectId, slot: &ShellSlotId) -> Frame {
    Frame {
        rail: Some(RAIL),
        items: vec![Placed {
            item: project(icon),
            tier: RailTier::Ungrouped,
            rect: RAIL_ICON,
            visible: RAIL_ICON,
        }],
        slots: vec![(slot.clone(), SLOT_BODY)],
    }
}

#[test]
fn 슬롯_드롭존은_네_가장자리_사분의일_띠와_가운데_절반이고_미리보기는_절반을_덮는다() {
    let body = Rect::from_min_size(pos2(100.0, 40.0), vec2(800.0, 400.0));
    let zones = slot_drop_zones(body);
    assert_eq!(
        zones.map(|(edge, _)| edge),
        [
            DropEdge::Left,
            DropEdge::Right,
            DropEdge::Top,
            DropEdge::Bottom,
            DropEdge::Center
        ]
    );
    assert_eq!(
        zones.map(|(_, zone)| zone),
        [
            Rect::from_min_max(pos2(100.0, 40.0), pos2(300.0, 440.0)),
            Rect::from_min_max(pos2(700.0, 40.0), pos2(900.0, 440.0)),
            Rect::from_min_max(pos2(100.0, 40.0), pos2(900.0, 140.0)),
            Rect::from_min_max(pos2(100.0, 340.0), pos2(900.0, 440.0)),
            Rect::from_min_max(pos2(300.0, 140.0), pos2(700.0, 340.0)),
        ]
    );
    assert_eq!(
        slot_drop_preview(body, DropEdge::Left),
        Rect::from_min_max(pos2(100.0, 40.0), pos2(500.0, 440.0))
    );
    assert_eq!(
        slot_drop_preview(body, DropEdge::Bottom),
        Rect::from_min_max(pos2(100.0, 240.0), pos2(900.0, 440.0))
    );
    assert_eq!(slot_drop_preview(body, DropEdge::Center), body);
    assert_eq!(shell_slot_edge(DropEdge::Center), ShellSlotEdge::Replace);
    assert_eq!(shell_slot_edge(DropEdge::Top), ShellSlotEdge::Top);
}

#[test]
fn 충돌_판정은_포인터_안_드롭존을_고르고_레일_밖에서_빗나가면_아무것도_고르지_않는다() {
    let Fixture {
        projects, slots, ..
    } = fixture();
    let frame = collision_frame(&projects[2], &slots[1]);
    let dragging = project(&projects[0]);
    let dragged = |pointer: Pos2| Rect::from_center_size(pointer, Vec2::splat(ITEM_SIZE));
    assert_eq!(
        resolve_target(&frame, &dragging, SLOT_CENTER, dragged(SLOT_CENTER)),
        Some(slot_target(&slots[1], DropEdge::Center))
    );
    assert_eq!(
        resolve_target(&frame, &dragging, SLOT_HEADER, dragged(SLOT_HEADER)),
        None,
        "a release on a slot header, a resizer or the title bar does nothing"
    );
    assert_eq!(
        resolve_target(&frame, &dragging, RAIL_GAP, dragged(RAIL_GAP)),
        Some(DropTarget::Item(project(&projects[2]))),
        "a gap in the rail still sorts against the nearest icon, never a slot"
    );
    let corner = pos2(SLOT_BODY.left() + 10.0, SLOT_BODY.top() + 100.0);
    assert_eq!(
        resolve_target(&frame, &dragging, corner, dragged(corner)),
        Some(slot_target(&slots[1], DropEdge::Left)),
        "overlapping corner strips go to the zone whose corners are nearest on average"
    );
    let header = RailItem::Group(ProjectGroupId::new());
    assert_eq!(
        resolve_target(&frame, &header, SLOT_CENTER, dragged(SLOT_CENTER)),
        None,
        "slot zones exist only while a project is dragged"
    );
    let mut scrolled = frame.clone();
    scrolled.items[0].visible = Rect::NOTHING;
    assert_eq!(
        resolve_target(&scrolled, &dragging, RAIL_GAP, dragged(RAIL_GAP)),
        None
    );
}

#[test]
fn 드롭_결과는_슬롯_분할_교체와_자기교체_무동작을_원본처럼_고른다() {
    let Fixture {
        snapshot,
        projects,
        slots,
        ..
    } = fixture();
    assert_eq!(
        resolve_drop(
            &project(&projects[2]),
            Some(&slot_target(&slots[1], DropEdge::Bottom)),
            &snapshot
        ),
        Some(DropRequest::OpenInSlot {
            project: projects[2].clone(),
            slot: slots[1].clone(),
            edge: ShellSlotEdge::Bottom,
        })
    );
    assert_eq!(
        resolve_drop(
            &project(&projects[2]),
            Some(&slot_target(&slots[0], DropEdge::Center)),
            &snapshot
        ),
        Some(DropRequest::OpenInSlot {
            project: projects[2].clone(),
            slot: slots[0].clone(),
            edge: ShellSlotEdge::Replace,
        })
    );
    assert_eq!(
        resolve_drop(
            &project(&projects[0]),
            Some(&slot_target(&slots[0], DropEdge::Center)),
            &snapshot
        ),
        None
    );
    assert_eq!(
        resolve_drop(
            &project(&projects[1]),
            Some(&slot_target(&slots[0], DropEdge::Right)),
            &snapshot
        ),
        Some(DropRequest::OpenInSlot {
            project: projects[1].clone(),
            slot: slots[0].clone(),
            edge: ShellSlotEdge::Right,
        }),
        "a project already in another slot is left for the server to refuse"
    );
    assert_eq!(resolve_drop(&project(&projects[1]), None, &snapshot), None);
}

#[test]
fn 재정렬은_전역_프로젝트_순서와_그룹_순서를_따로_쓰고_두_단은_섞이지_않는다() {
    let Fixture {
        snapshot,
        projects,
        groups,
        ..
    } = fixture();
    let over = |item: RailItem| Some(DropTarget::Item(item));
    assert_eq!(
        resolve_drop(
            &project(&projects[0]),
            over(project(&projects[2])).as_ref(),
            &snapshot
        ),
        Some(DropRequest::ReorderProjects(vec![
            projects[1].clone(),
            projects[2].clone(),
            projects[0].clone(),
        ]))
    );
    assert_eq!(
        resolve_drop(
            &project(&projects[1]),
            over(project(&projects[0])).as_ref(),
            &snapshot
        ),
        Some(DropRequest::ReorderProjects(vec![
            projects[1].clone(),
            projects[0].clone(),
            projects[2].clone(),
        ])),
        "a member moved onto another group's member edits session.projects, not membership"
    );
    assert_eq!(
        resolve_drop(
            &project(&projects[0]),
            over(project(&projects[0])).as_ref(),
            &snapshot
        ),
        None
    );
    assert_eq!(
        resolve_drop(
            &RailItem::Group(groups[1].clone()),
            over(RailItem::Group(groups[0].clone())).as_ref(),
            &snapshot
        ),
        Some(DropRequest::ReorderGroups(vec![
            groups[1].clone(),
            groups[0].clone(),
        ]))
    );
    assert_eq!(
        resolve_drop(
            &RailItem::Group(groups[0].clone()),
            over(project(&projects[2])).as_ref(),
            &snapshot
        ),
        None
    );
    assert_eq!(
        resolve_drop(
            &project(&projects[2]),
            over(RailItem::Group(groups[0].clone())).as_ref(),
            &snapshot
        ),
        None
    );
}

#[test]
fn 정렬_이동량은_dnd_kit_세로목록_전략처럼_원본자리와_간격을_밀어낸다() {
    let column = |tops: &[f32], height: f32| -> Vec<Rect> {
        tops.iter()
            .map(|top| Rect::from_min_size(pos2(8.0, *top), vec2(ITEM_SIZE, height)))
            .collect()
    };
    let icons = column(
        &[0.0, ITEM_SIZE + ITEM_GAP, 2.0 * (ITEM_SIZE + ITEM_GAP)],
        ITEM_SIZE,
    );
    let step = ITEM_SIZE + ITEM_GAP;
    assert_eq!(sorting_offset(&icons, 0, 2, 0), 2.0 * step);
    assert_eq!(sorting_offset(&icons, 0, 2, 1), -step);
    assert_eq!(sorting_offset(&icons, 0, 2, 2), -step);
    assert_eq!(sorting_offset(&icons, 2, 0, 2), -2.0 * step);
    assert_eq!(sorting_offset(&icons, 2, 0, 0), step);
    assert_eq!(sorting_offset(&icons, 2, 1, 0), 0.0);
    assert_eq!(sorting_offset(&icons, 1, 1, 1), 0.0);
    let headers = column(&[0.0, HEADER_HEIGHT + SECTION_GAP], HEADER_HEIGHT);
    assert_eq!(
        sorting_offset(&headers, 0, 1, 1),
        -(HEADER_HEIGHT + SECTION_GAP),
        "headers swap across the members between them, which stay put"
    );
    assert_eq!(
        sorting_offset(&headers, 0, 1, 0),
        HEADER_HEIGHT + SECTION_GAP
    );
}

#[test]
fn 눌린_항목의_보기는_활성화_뒤에만_흐려지고_클릭을_막는다() {
    let Fixture { projects, .. } = fixture();
    let placed = |id: &ProjectId, top: f32| Placed {
        item: project(id),
        tier: RailTier::Ungrouped,
        rect: Rect::from_min_size(pos2(8.0, top), Vec2::splat(ITEM_SIZE)),
        visible: Rect::from_min_size(pos2(8.0, top), Vec2::splat(ITEM_SIZE)),
    };
    let mut press = Press {
        item: project(&projects[0]),
        origin: pos2(28.0, 20.0),
        rect: Rect::from_min_size(pos2(8.0, 0.0), Vec2::splat(ITEM_SIZE)),
        content: Rect::EVERYTHING,
        phase: Phase::Pending,
        over: Some(DropTarget::Item(project(&projects[1]))),
        layout: vec![
            placed(&projects[0], 0.0),
            placed(&projects[1], ITEM_SIZE + ITEM_GAP),
        ],
    };
    assert_eq!(press.view(&project(&projects[0])), RailItemView::default());
    press.phase = Phase::Active;
    assert_eq!(
        press.view(&project(&projects[0])),
        RailItemView {
            offset: ITEM_SIZE + ITEM_GAP,
            is_dragging: true,
            suppresses_click: true,
        }
    );
    assert_eq!(
        press.view(&project(&projects[1])).offset,
        -(ITEM_SIZE + ITEM_GAP)
    );
    press.layout[1].tier = RailTier::Members(ProjectGroupId::new());
    assert_eq!(
        press.view(&project(&projects[1])).offset,
        0.0,
        "an entry of another tier never shifts"
    );
    press.phase = Phase::Cancelled;
    assert_eq!(
        press.view(&project(&projects[0])),
        RailItemView {
            offset: 0.0,
            is_dragging: false,
            suppresses_click: true,
        }
    );
}

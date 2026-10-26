use egui::{Context, Id, Key, Pos2, Rect, Response, Ui, pos2, vec2};
use taide_model::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::layout::DropEdge;
use taide_model::project::ShellSlotEdge;

use crate::snapshot::{ShellSnapshot, slot_project};

/// `DRAG_ACTIVATION_DISTANCE_PX` (`use-project-drag.ts` 22): the pointer must travel strictly
/// farther than this before a press on the rail becomes a drag.
pub const DRAG_ACTIVATION_DISTANCE: f32 = 4.0;
/// `DRAGGING_OPACITY` (`sortable-project-icon.tsx` 33): the rail icon left behind as a ghost.
pub const DRAGGING_OPACITY: f32 = 0.4;
const DROP_ZONE_EDGE_FRACTION: f32 = 0.25;
const DROP_PREVIEW_FRACTION: f32 = 0.5;
const RECT_CORNER_COUNT: f32 = 4.0;
const MEMORY_KEY: &str = "taide-project-drag";

/// A sortable rail entry: a project icon or a group header (`project-drag.ts` 23-25).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RailItem {
    Project(ProjectId),
    Group(ProjectGroupId),
}

/// One `SortableContext` of the rail (`app-sidebar.tsx` 154-176): the group headers, one group's
/// members, or the ungrouped projects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RailTier {
    Groups,
    Members(ProjectGroupId),
    Ungrouped,
}

/// What a drag is over: a rail entry or one of a slot's five drop zones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropTarget {
    Item(RailItem),
    Slot { slot: ShellSlotId, edge: DropEdge },
}

/// What a finished drag asks the host for (`use-project-drag.ts` 97-128).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropRequest {
    ReorderProjects(Vec<ProjectId>),
    ReorderGroups(Vec<ProjectGroupId>),
    OpenInSlot {
        project: ProjectId,
        slot: ShellSlotId,
        edge: ShellSlotEdge,
    },
}

/// How one rail entry draws this frame while a drag is pending or running.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RailItemView {
    pub offset: f32,
    pub is_dragging: bool,
    pub suppresses_click: bool,
}

/// What the shell paints and requests at the end of a frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DragFeedback {
    pub overlay: Option<(ProjectId, Rect)>,
    pub slot_preview: Option<Rect>,
    pub request: Option<DropRequest>,
}

#[derive(Clone, Debug)]
struct Placed {
    item: RailItem,
    tier: RailTier,
    rect: Rect,
    visible: Rect,
}

#[derive(Clone, Debug, Default)]
struct Frame {
    rail: Option<Rect>,
    items: Vec<Placed>,
    slots: Vec<(ShellSlotId, Rect)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Pending,
    Active,
    Cancelled,
}

#[derive(Clone, Debug)]
struct Press {
    item: RailItem,
    origin: Pos2,
    rect: Rect,
    content: Rect,
    phase: Phase,
    over: Option<DropTarget>,
    layout: Vec<Placed>,
}

#[derive(Clone, Debug, Default)]
struct Memory {
    press: Option<Press>,
    frame: Frame,
}

#[derive(Clone, Copy, Debug)]
struct PointerInput {
    pointer: Option<Pos2>,
    is_down: bool,
    is_escape: bool,
    content: Rect,
}

impl PointerInput {
    fn read(context: &Context) -> Self {
        let (pointer, is_down, is_escape) = context.input(|input| {
            (
                input.pointer.latest_pos(),
                input.pointer.primary_down(),
                input.key_pressed(Key::Escape),
            )
        });
        Self {
            pointer,
            is_down,
            is_escape,
            content: context.content_rect(),
        }
    }
}

fn with_memory<R>(context: &Context, update: impl FnOnce(&mut Memory) -> R) -> R {
    context.data_mut(|data| update(data.get_temp_mut_or_default::<Memory>(Id::new(MEMORY_KEY))))
}

/// The five zones a slot shows while a project is dragged (`split-drop-zones.tsx` 8-14): quarter
/// strips along each edge, which overlap in the corners, and the middle half.
pub fn slot_drop_zones(body: Rect) -> [(DropEdge, Rect); 5] {
    let edge = body.size() * DROP_ZONE_EDGE_FRACTION;
    [
        (
            DropEdge::Left,
            Rect::from_min_max(body.min, pos2(body.left() + edge.x, body.bottom())),
        ),
        (
            DropEdge::Right,
            Rect::from_min_max(pos2(body.right() - edge.x, body.top()), body.max),
        ),
        (
            DropEdge::Top,
            Rect::from_min_max(body.min, pos2(body.right(), body.top() + edge.y)),
        ),
        (
            DropEdge::Bottom,
            Rect::from_min_max(pos2(body.left(), body.bottom() - edge.y), body.max),
        ),
        (DropEdge::Center, body.shrink2(edge)),
    ]
}

/// Where the dragged project would land (`split-drop-zones.tsx` 16-22).
pub fn slot_drop_preview(body: Rect, edge: DropEdge) -> Rect {
    let half = body.size() * DROP_PREVIEW_FRACTION;
    match edge {
        DropEdge::Left => Rect::from_min_size(body.min, vec2(half.x, body.height())),
        DropEdge::Right => Rect::from_min_max(pos2(body.right() - half.x, body.top()), body.max),
        DropEdge::Top => Rect::from_min_size(body.min, vec2(body.width(), half.y)),
        DropEdge::Bottom => Rect::from_min_max(pos2(body.left(), body.bottom() - half.y), body.max),
        DropEdge::Center => body,
    }
}

/// `SHELL_SLOT_EDGE_BY_DROP_EDGE` (`project-drag.ts` 42-48): a slot's centre replaces its project.
pub fn shell_slot_edge(edge: DropEdge) -> ShellSlotEdge {
    match edge {
        DropEdge::Left => ShellSlotEdge::Left,
        DropEdge::Right => ShellSlotEdge::Right,
        DropEdge::Top => ShellSlotEdge::Top,
        DropEdge::Bottom => ShellSlotEdge::Bottom,
        DropEdge::Center => ShellSlotEdge::Replace,
    }
}

/// `reorderById` (`project-drag.ts` 92-97): `arrayMove` of `active` to `over`'s index, or `None`
/// when either is not in this tier or nothing moves.
pub fn reorder_by_id<T: Clone + PartialEq>(ids: &[T], active: &T, over: &T) -> Option<Vec<T>> {
    let from = ids.iter().position(|id| id == active)?;
    let to = ids.iter().position(|id| id == over)?;
    if from == to {
        return None;
    }
    let mut order = ids.to_vec();
    let moved = order.remove(from);
    order.insert(to, moved);
    Some(order)
}

fn item_gap(rects: &[Rect], index: usize, active: usize) -> f32 {
    let Some(current) = rects.get(index) else {
        return 0.0;
    };
    let before = index
        .checked_sub(1)
        .and_then(|previous| rects.get(previous))
        .map(|previous| current.top() - previous.bottom());
    let after = rects
        .get(index + 1)
        .map(|next| next.top() - current.bottom());
    let gap = if active < index {
        before.or(after)
    } else {
        after.or(before)
    };
    gap.unwrap_or_default()
}

/// `verticalListSortingStrategy` (`@dnd-kit/sortable`): how far the entry at `index` of one tier
/// moves while the entry at `active` is held over the entry at `over`.
pub fn sorting_offset(rects: &[Rect], active: usize, over: usize, index: usize) -> f32 {
    let Some(active_rect) = rects.get(active) else {
        return 0.0;
    };
    if index == active {
        return rects.get(over).map_or(0.0, |over_rect| {
            if active < over {
                over_rect.bottom() - active_rect.bottom()
            } else {
                over_rect.top() - active_rect.top()
            }
        });
    }
    let gap = item_gap(rects, index, active);
    if index > active && index <= over {
        return -active_rect.height() - gap;
    }
    if index < active && index >= over {
        return active_rect.height() + gap;
    }
    0.0
}

fn average_corner_distance(rect: Rect, pointer: Pos2) -> f32 {
    [
        rect.left_top(),
        rect.right_top(),
        rect.left_bottom(),
        rect.right_bottom(),
    ]
    .iter()
    .map(|corner| corner.distance(pointer))
    .sum::<f32>()
        / RECT_CORNER_COUNT
}

fn closest<T>(candidates: impl Iterator<Item = (T, f32)>) -> Option<T> {
    candidates
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(target, _)| target)
}

/// `projectCollisionDetection` (`use-project-drag.ts` 59-65): `pointerWithin` over the rail
/// entries and, for a project, the slots' zones; otherwise `closestCenter` over the rail entries
/// only while the pointer is over the rail.
fn resolve_target(
    frame: &Frame,
    dragging: &RailItem,
    pointer: Pos2,
    dragged: Rect,
) -> Option<DropTarget> {
    let visible = frame
        .items
        .iter()
        .filter(|placed| placed.visible.is_positive());
    let items = visible
        .clone()
        .map(|placed| (DropTarget::Item(placed.item.clone()), placed.visible));
    let zones = frame
        .slots
        .iter()
        .filter(|_| matches!(dragging, RailItem::Project(_)))
        .flat_map(|(slot, body)| {
            slot_drop_zones(*body).map(|(edge, zone)| {
                (
                    DropTarget::Slot {
                        slot: slot.clone(),
                        edge,
                    },
                    zone,
                )
            })
        });
    closest(
        items
            .chain(zones)
            .filter(|(_, rect)| rect.contains(pointer))
            .map(|(target, rect)| (target, average_corner_distance(rect, pointer))),
    )
    .or_else(|| {
        frame.rail.filter(|rail| rail.contains(pointer))?;
        closest(visible.map(|placed| {
            (
                DropTarget::Item(placed.item.clone()),
                placed.rect.center().distance(dragged.center()),
            )
        }))
    })
}

/// `handleDragEnd` (`use-project-drag.ts` 121-128) with `resolveShellSlotDrop`
/// (`project-drag.ts` 79-83): a header only reorders headers, a project splits or replaces a slot
/// unless it would replace itself, and otherwise reorders the global project order.
pub fn resolve_drop(
    dragging: &RailItem,
    over: Option<&DropTarget>,
    snapshot: &ShellSnapshot,
) -> Option<DropRequest> {
    match (dragging, over?) {
        (RailItem::Group(active), DropTarget::Item(RailItem::Group(over))) => {
            let ids: Vec<_> = snapshot
                .groups
                .iter()
                .map(|group| group.id.clone())
                .collect();
            reorder_by_id(&ids, active, over).map(DropRequest::ReorderGroups)
        }
        (RailItem::Project(project), DropTarget::Slot { slot, edge }) => {
            let replaces_itself = *edge == DropEdge::Center
                && snapshot
                    .shell
                    .tree
                    .as_ref()
                    .and_then(|tree| slot_project(tree, slot))
                    == Some(project);
            (!replaces_itself).then(|| DropRequest::OpenInSlot {
                project: project.clone(),
                slot: slot.clone(),
                edge: shell_slot_edge(*edge),
            })
        }
        (RailItem::Project(active), DropTarget::Item(RailItem::Project(over))) => {
            let ids: Vec<_> = snapshot
                .projects
                .iter()
                .map(|project| project.id.clone())
                .collect();
            reorder_by_id(&ids, active, over).map(DropRequest::ReorderProjects)
        }
        (RailItem::Group(_), _) | (RailItem::Project(_), DropTarget::Item(RailItem::Group(_))) => {
            None
        }
    }
}

impl Press {
    fn dragged(&self, pointer: Pos2) -> Rect {
        self.rect.translate(pointer - self.origin)
    }

    fn tier_of(&self, item: &RailItem) -> Option<&RailTier> {
        self.layout
            .iter()
            .find(|placed| &placed.item == item)
            .map(|placed| &placed.tier)
    }

    fn sorting_offset(&self, item: &RailItem) -> f32 {
        let Some(DropTarget::Item(over)) = &self.over else {
            return 0.0;
        };
        let Some(tier) = self.tier_of(&self.item) else {
            return 0.0;
        };
        if self.tier_of(over) != Some(tier) {
            return 0.0;
        }
        let members: Vec<&Placed> = self
            .layout
            .iter()
            .filter(|placed| &placed.tier == tier)
            .collect();
        let index_of = |target: &RailItem| members.iter().position(|placed| &placed.item == target);
        let rects: Vec<Rect> = members.iter().map(|placed| placed.rect).collect();
        match (index_of(&self.item), index_of(over), index_of(item)) {
            (Some(active), Some(over), Some(index)) => sorting_offset(&rects, active, over, index),
            _ => 0.0,
        }
    }

    fn view(&self, item: &RailItem) -> RailItemView {
        let is_source = &self.item == item;
        let is_active = self.phase == Phase::Active;
        RailItemView {
            offset: if is_active {
                self.sorting_offset(item)
            } else {
                0.0
            },
            is_dragging: is_active && is_source && matches!(item, RailItem::Project(_)),
            suppresses_click: is_source && self.phase != Phase::Pending,
        }
    }
}

/// Cancellation on Escape or a window resize (`AbstractPointerSensor` `handleKeydown` and its
/// `resize` listener), activation past [`DRAG_ACTIVATION_DISTANCE`], and the drop target under
/// the pointer measured against `frame`.
fn advance(press: &mut Option<Press>, frame: &Frame, input: PointerInput) {
    let Some(current) = press.as_mut() else {
        return;
    };
    if input.is_escape || input.content != current.content {
        if current.phase == Phase::Pending {
            *press = None;
            return;
        }
        current.phase = Phase::Cancelled;
        current.over = None;
        return;
    }
    let Some(pointer) = input.pointer else {
        return;
    };
    if current.phase == Phase::Pending
        && (pointer - current.origin).length() > DRAG_ACTIVATION_DISTANCE
    {
        current.phase = Phase::Active;
    }
    if current.phase == Phase::Active {
        current.over = resolve_target(frame, &current.item, pointer, current.dragged(pointer));
    }
}

/// Advances a held press against the previous frame's layout, so this frame already draws the
/// entries where the current pointer puts them, then starts a fresh layout record.
pub(crate) fn begin_frame(context: &Context) {
    let input = PointerInput::read(context);
    with_memory(context, |memory| {
        let Memory { press, frame } = memory;
        if input.is_down {
            if let Some(current) = press.as_mut() {
                current.layout = frame.items.clone();
            }
            advance(press, frame, input);
        }
        *frame = Frame::default();
    });
}

pub(crate) fn register_rail(context: &Context, rail: Rect) {
    with_memory(context, |memory| memory.frame.rail = Some(rail));
}

pub(crate) fn register_slot(context: &Context, slot: &ShellSlotId, body: Rect) {
    with_memory(context, |memory| {
        memory.frame.slots.push((slot.clone(), body));
    });
}

/// Records a rail entry for this frame's collisions, starts a press on a primary button press,
/// and says how the entry draws while a drag is pending or running.
pub(crate) fn rail_item(
    ui: &Ui,
    response: &Response,
    item: RailItem,
    tier: RailTier,
) -> RailItemView {
    let origin = ui.input(|input| {
        input
            .pointer
            .primary_pressed()
            .then(|| input.pointer.press_origin())
            .flatten()
    });
    let origin = origin.filter(|_| response.is_pointer_button_down_on());
    let content = ui.ctx().content_rect();
    let visible = response.rect.intersect(ui.clip_rect());
    with_memory(ui.ctx(), |memory| {
        memory.frame.items.push(Placed {
            item: item.clone(),
            tier,
            rect: response.rect,
            visible,
        });
        if let Some(origin) = origin {
            memory.press = Some(Press {
                item: item.clone(),
                origin,
                rect: response.rect,
                content,
                phase: Phase::Pending,
                over: None,
                layout: memory.frame.items.clone(),
            });
        }
        memory
            .press
            .as_ref()
            .map(|press| press.view(&item))
            .unwrap_or_default()
    })
}

/// Settles the press once every entry and slot of the frame was laid out: the request a release
/// makes (`handleDragEnd`), or the preview tile and slot preview while the drag runs.
pub(crate) fn end_frame(context: &Context, snapshot: &ShellSnapshot) -> DragFeedback {
    let input = PointerInput::read(context);
    with_memory(context, |memory| {
        let Memory { press, frame } = memory;
        if !input.is_down {
            let request = press
                .take()
                .filter(|current| current.phase == Phase::Active)
                .and_then(|current| {
                    let over = input
                        .pointer
                        .and_then(|pointer| {
                            resolve_target(frame, &current.item, pointer, current.dragged(pointer))
                        })
                        .or(current.over);
                    resolve_drop(&current.item, over.as_ref(), snapshot)
                });
            return DragFeedback {
                request,
                ..DragFeedback::default()
            };
        }
        advance(press, frame, input);
        let (Some(current), Some(pointer)) = (press.as_ref(), input.pointer) else {
            return DragFeedback::default();
        };
        if current.phase != Phase::Active {
            return DragFeedback::default();
        }
        DragFeedback {
            overlay: match &current.item {
                RailItem::Project(project) => Some((project.clone(), current.dragged(pointer))),
                RailItem::Group(_) => None,
            },
            slot_preview: match &current.over {
                Some(DropTarget::Slot { slot, edge }) => frame
                    .slots
                    .iter()
                    .find(|(candidate, _)| candidate == slot)
                    .map(|(_, body)| slot_drop_preview(*body, *edge)),
                _ => None,
            },
            request: None,
        }
    })
}

/// Whether a rail drag is past its activation distance; a host hides native overlays such as web
/// previews meanwhile so the slot drop zones stay visible and keep the pointer.
pub fn is_dragging(context: &Context) -> bool {
    with_memory(context, |memory| {
        memory
            .press
            .as_ref()
            .is_some_and(|press| press.phase == Phase::Active)
    })
}

#[cfg(test)]
#[path = "project-drag-tests.rs"]
mod tests;

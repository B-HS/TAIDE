use egui::{Context, Id, Pos2, Rect, Response, Ui, pos2};
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{DropEdge, Tab};

use crate::commands::ShellMutation;
use crate::project_drag::{
    Gesture, Phase, PointerInput, pointer_within, slot_drop_preview, slot_drop_zones,
    sorting_offset,
};

const MEMORY_KEY: &str = "taide-tab-drag";
const AUTO_SCROLL_THRESHOLD_FRACTION: f32 = 0.2;
const AUTO_SCROLL_ACCELERATION: f32 = 10.0;
const AUTO_SCROLL_INTERVAL_SECONDS: f32 = 0.005;

/// The `EditorArea` whose `DndContext` a tab belongs to (`editor-area.tsx` 509-515): every shell
/// slot and every auxiliary window mounts its own, so a tab only lands inside its own area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TabArea {
    pub(crate) project: ProjectId,
    pub(crate) slot: Option<ShellSlotId>,
}

/// What a tab drag is over: a tab, a tab bar's empty filler (`pane-tab-bar.tsx` 96-99), or one of
/// a pane's five drop zones (`pane-node-view.tsx` 69-73).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TabDropTarget {
    Tab { pane: PaneId, tab: TabId },
    Container(PaneId),
    Split { pane: PaneId, edge: DropEdge },
}

/// How one tab draws this frame while a drag is pending or running.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TabItemView {
    pub(crate) offset: f32,
    pub(crate) is_dragging: bool,
    pub(crate) suppresses_click: bool,
}

/// What the shell paints and requests at the end of a frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct TabDragFeedback {
    pub(crate) overlay: Option<(Tab, Rect)>,
    pub(crate) split_preview: Option<Rect>,
    pub(crate) request: Option<ShellMutation>,
}

#[derive(Clone, Debug)]
struct PlacedTab {
    area: TabArea,
    pane: PaneId,
    tab: TabId,
    pinned: bool,
    rect: Rect,
    visible: Rect,
}

#[derive(Clone, Debug)]
struct PlacedPane {
    area: TabArea,
    pane: PaneId,
    body: Rect,
    tabs: Vec<(TabId, bool)>,
}

#[derive(Clone, Debug, Default)]
struct Frame {
    tabs: Vec<PlacedTab>,
    containers: Vec<(TabArea, PaneId, Rect)>,
    panes: Vec<PlacedPane>,
}

/// `useScrollIntent` (`@dnd-kit/core`): the horizontal directions the pointer has moved in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ScrollIntent {
    backward: bool,
    forward: bool,
}

#[derive(Clone, Debug)]
struct Press {
    area: TabArea,
    pane: PaneId,
    tab: Tab,
    origin: Pos2,
    rect: Rect,
    content: Rect,
    phase: Phase,
    over: Option<TabDropTarget>,
    layout: Vec<PlacedTab>,
    pointer: Option<Pos2>,
    intent: ScrollIntent,
}

#[derive(Clone, Debug, Default)]
struct Memory {
    press: Option<Press>,
    frame: Frame,
}

fn with_memory<R>(context: &Context, update: impl FnOnce(&mut Memory) -> R) -> R {
    let id = Id::new(MEMORY_KEY).with(context.viewport_id());
    context.data_mut(|data| update(data.get_temp_mut_or_default::<Memory>(id)))
}

fn transposed(rect: Rect) -> Rect {
    Rect::from_min_max(pos2(rect.min.y, rect.min.x), pos2(rect.max.y, rect.max.x))
}

/// `horizontalListSortingStrategy` (`@dnd-kit/sortable`): the vertical strategy along the x axis.
pub(crate) fn horizontal_sorting_offset(
    rects: &[Rect],
    active: usize,
    over: usize,
    index: usize,
) -> f32 {
    let rects: Vec<Rect> = rects.iter().copied().map(transposed).collect();
    sorting_offset(&rects, active, over, index)
}

/// `pointerWithin` (`editor-area.tsx` 511) over the area's tabs, tab bar fillers and pane zones.
fn resolve_target(frame: &Frame, area: &TabArea, pointer: Pos2) -> Option<TabDropTarget> {
    let tabs = frame
        .tabs
        .iter()
        .filter(|placed| &placed.area == area)
        .map(|placed| {
            (
                TabDropTarget::Tab {
                    pane: placed.pane.clone(),
                    tab: placed.tab.clone(),
                },
                placed.visible,
            )
        });
    let containers = frame
        .containers
        .iter()
        .filter(|(owner, ..)| owner == area)
        .map(|(_, pane, rect)| (TabDropTarget::Container(pane.clone()), *rect));
    let zones = frame
        .panes
        .iter()
        .filter(|placed| &placed.area == area)
        .flat_map(|placed| {
            slot_drop_zones(placed.body).map(|(edge, zone)| {
                (
                    TabDropTarget::Split {
                        pane: placed.pane.clone(),
                        edge,
                    },
                    zone,
                )
            })
        });
    pointer_within(tabs.chain(containers).chain(zones), pointer)
}

/// `handleDragEnd` (`editor-area.tsx` 471-504): a pane's centre or tab bar filler appends, an
/// edge splits, and a tab takes the hovered tab's index clamped to its own side of the pinned run.
fn resolve_drop(
    press: &Press,
    over: Option<&TabDropTarget>,
    panes: &[PlacedPane],
) -> Option<ShellMutation> {
    let leaf = |pane: &PaneId| {
        panes
            .iter()
            .find(|placed| placed.area == press.area && &placed.pane == pane)
    };
    let move_tab = |pane: &PaneId, index: usize| {
        Some(ShellMutation::MoveTab {
            tab: press.tab.id.clone(),
            pane: pane.clone(),
            index: index.try_into().ok()?,
        })
    };
    match over? {
        TabDropTarget::Split {
            pane,
            edge: DropEdge::Center,
        }
        | TabDropTarget::Container(pane) => move_tab(pane, leaf(pane)?.tabs.len()),
        TabDropTarget::Split { pane, edge } => Some(ShellMutation::SplitTab {
            pane: pane.clone(),
            edge: *edge,
            tab: press.tab.id.clone(),
        }),
        TabDropTarget::Tab { pane, tab } => {
            let tabs = &leaf(pane)?.tabs;
            let raw = tabs.iter().position(|(id, _)| id == tab)?;
            let pinned = tabs.iter().filter(|(_, pinned)| *pinned).count();
            let index = if press.tab.pinned {
                raw.min(pinned)
            } else {
                raw.max(pinned)
            };
            move_tab(pane, index)
        }
    }
}

impl Press {
    fn dragged(&self, pointer: Pos2) -> Rect {
        self.rect.translate(pointer - self.origin)
    }

    fn sorting_offset(&self, tab: &TabId) -> f32 {
        let Some(TabDropTarget::Tab { pane, tab: over }) = &self.over else {
            return 0.0;
        };
        if pane != &self.pane {
            return 0.0;
        }
        let group: Vec<&PlacedTab> = self
            .layout
            .iter()
            .filter(|placed| {
                placed.area == self.area
                    && placed.pane == self.pane
                    && placed.pinned == self.tab.pinned
            })
            .collect();
        let index_of = |target: &TabId| group.iter().position(|placed| &placed.tab == target);
        let rects: Vec<Rect> = group.iter().map(|placed| placed.rect).collect();
        match (index_of(&self.tab.id), index_of(over), index_of(tab)) {
            (Some(active), Some(over), Some(index)) => {
                horizontal_sorting_offset(&rects, active, over, index)
            }
            _ => 0.0,
        }
    }

    fn view(&self, tab: &TabId) -> TabItemView {
        let is_source = &self.tab.id == tab;
        let is_active = self.phase == Phase::Active;
        TabItemView {
            offset: if is_active {
                self.sorting_offset(tab)
            } else {
                0.0
            },
            is_dragging: is_active && is_source,
            suppresses_click: is_source && self.phase != Phase::Pending,
        }
    }
}

fn advance(press: &mut Option<Press>, frame: &Frame, input: PointerInput) {
    let Some(current) = press.as_mut() else {
        return;
    };
    match input.advance(&mut current.phase, current.origin, current.content) {
        Gesture::Abandoned => *press = None,
        Gesture::Cancelled => current.over = None,
        Gesture::Held => {}
        Gesture::Dragging(pointer) => {
            if let Some(previous) = current.pointer {
                current.intent.backward |= pointer.x < previous.x;
                current.intent.forward |= pointer.x > previous.x;
            }
            current.pointer = Some(pointer);
            current.over = resolve_target(frame, &current.area, pointer);
        }
    }
}

/// `getScrollDirectionAndSpeed` (`@dnd-kit/core`): the points per [`AUTO_SCROLL_INTERVAL_SECONDS`]
/// a pointer at `x` scrolls a bar whose viewport is `viewport`, forward when positive.
fn auto_scroll_speed(viewport: Rect, x: f32, intent: ScrollIntent) -> f32 {
    let threshold = viewport.width() * AUTO_SCROLL_THRESHOLD_FRACTION;
    if threshold <= 0.0 {
        return 0.0;
    }
    if intent.forward && x >= viewport.right() - threshold {
        return AUTO_SCROLL_ACCELERATION * ((viewport.right() - threshold - x) / threshold).abs();
    }
    if intent.backward && x <= viewport.left() + threshold {
        return -AUTO_SCROLL_ACCELERATION * ((viewport.left() + threshold - x) / threshold).abs();
    }
    0.0
}

/// `useAutoScroller` (`@dnd-kit/core`): how far this frame scrolls the tab bar of `pane` forward.
/// It is the scrollable ancestor of the hovered tab or filler, or of the dragged tab while nothing
/// is hovered; a pane zone has none.
pub(crate) fn auto_scroll(context: &Context, area: &TabArea, pane: &PaneId, viewport: Rect) -> f32 {
    let elapsed = context.input(|input| input.stable_dt);
    with_memory(context, |memory| {
        let Some(press) = memory
            .press
            .as_ref()
            .filter(|press| press.phase == Phase::Active && &press.area == area)
        else {
            return 0.0;
        };
        let scrolled = match &press.over {
            Some(TabDropTarget::Tab { pane: over, .. } | TabDropTarget::Container(over)) => over,
            Some(TabDropTarget::Split { .. }) => return 0.0,
            None => &press.pane,
        };
        match press.pointer {
            Some(pointer) if scrolled == pane => {
                auto_scroll_speed(viewport, pointer.x, press.intent) * elapsed
                    / AUTO_SCROLL_INTERVAL_SECONDS
            }
            _ => 0.0,
        }
    })
}

/// Advances a held press against the previous frame's layout, so this frame already draws the
/// tabs where the current pointer puts them, then starts a fresh layout record.
pub(crate) fn begin_frame(context: &Context) {
    let input = PointerInput::read(context);
    with_memory(context, |memory| {
        let Memory { press, frame } = memory;
        if input.is_down {
            if let Some(current) = press.as_mut() {
                current.layout = frame.tabs.clone();
            }
            advance(press, frame, input);
        }
        *frame = Frame::default();
    });
}

/// Records a leaf's content box, where its five zones appear while a tab is dragged
/// (`pane-node-view.tsx` 275-280), and the tabs a drop on it counts.
pub(crate) fn register_pane(
    context: &Context,
    area: &TabArea,
    pane: &PaneId,
    body: Rect,
    tabs: &[Tab],
) {
    with_memory(context, |memory| {
        memory.frame.panes.push(PlacedPane {
            area: area.clone(),
            pane: pane.clone(),
            body,
            tabs: tabs
                .iter()
                .map(|tab| (tab.id.clone(), tab.pinned))
                .collect(),
        });
    });
}

pub(crate) fn register_container(context: &Context, area: &TabArea, pane: &PaneId, visible: Rect) {
    with_memory(context, |memory| {
        memory
            .frame
            .containers
            .push((area.clone(), pane.clone(), visible));
    });
}

/// How the tab draws while a drag is pending or running; read before the tab lays itself out.
pub(crate) fn tab_view(context: &Context, tab: &TabId) -> TabItemView {
    with_memory(context, |memory| {
        memory
            .press
            .as_ref()
            .map(|press| press.view(tab))
            .unwrap_or_default()
    })
}

/// Records a laid out tab for this frame's collisions and starts a press on a primary button press.
/// The original tabs sit flush against each other, so a tab also takes the drops on the row gap
/// that follows it.
pub(crate) fn register_tab(ui: &Ui, response: &Response, area: &TabArea, pane: &PaneId, tab: &Tab) {
    let origin = ui
        .input(|input| {
            input
                .pointer
                .primary_pressed()
                .then(|| input.pointer.press_origin())
                .flatten()
        })
        .filter(|_| response.is_pointer_button_down_on());
    let content = ui.ctx().content_rect();
    let mut droppable = response.rect;
    droppable.max.x += ui.spacing().item_spacing.x;
    let visible = droppable.intersect(ui.clip_rect());
    with_memory(ui.ctx(), |memory| {
        memory.frame.tabs.push(PlacedTab {
            area: area.clone(),
            pane: pane.clone(),
            tab: tab.id.clone(),
            pinned: tab.pinned,
            rect: response.rect,
            visible,
        });
        if let Some(origin) = origin {
            memory.press = Some(Press {
                area: area.clone(),
                pane: pane.clone(),
                tab: tab.clone(),
                origin,
                rect: response.rect,
                content,
                phase: Phase::Pending,
                over: None,
                layout: memory.frame.tabs.clone(),
                pointer: None,
                intent: ScrollIntent::default(),
            });
        }
    });
}

/// Settles the press once every tab and pane of the frame was laid out: the request a release
/// makes (`handleDragEnd`), or the `DragOverlay` tab and the hovered zone's preview while it runs.
pub(crate) fn end_frame(context: &Context) -> TabDragFeedback {
    let input = PointerInput::read(context);
    with_memory(context, |memory| {
        let Memory { press, frame } = memory;
        if !input.is_down {
            let request = press
                .take()
                .filter(|current| current.phase == Phase::Active && !input.cancels(current.content))
                .and_then(|current| {
                    let over = match input.pointer {
                        Some(pointer) => resolve_target(frame, &current.area, pointer),
                        None => current.over.clone(),
                    };
                    resolve_drop(&current, over.as_ref(), &frame.panes)
                });
            return TabDragFeedback {
                request,
                ..TabDragFeedback::default()
            };
        }
        advance(press, frame, input);
        let (Some(current), Some(pointer)) = (press.as_ref(), input.pointer) else {
            return TabDragFeedback::default();
        };
        if current.phase != Phase::Active {
            return TabDragFeedback::default();
        }
        TabDragFeedback {
            overlay: Some((current.tab.clone(), current.dragged(pointer))),
            split_preview: match &current.over {
                Some(TabDropTarget::Split { pane, edge }) => frame
                    .panes
                    .iter()
                    .find(|placed| placed.area == current.area && &placed.pane == pane)
                    .map(|placed| slot_drop_preview(placed.body, *edge)),
                _ => None,
            },
            request: None,
        }
    })
}

/// Whether a tab drag is past its activation distance; a host hides native overlays such as web
/// previews meanwhile so the pane drop zones stay visible and keep the pointer.
pub fn is_dragging(context: &Context) -> bool {
    with_memory(context, |memory| {
        memory
            .press
            .as_ref()
            .is_some_and(|press| press.phase == Phase::Active)
    })
}

#[cfg(test)]
#[path = "tab-drag-tests.rs"]
mod tests;

use std::sync::Arc;

use eframe::egui::{self, FontId, Id, Key, Rect, Sense, Stroke, Ui, pos2, vec2};
use taide_model::error::AppResult;
use taide_model::git::GitBranch;
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::intents::git::Intent;
use taide_native_ui::modal::Composition;

use crate::git_scm::View;
use crate::navigation_icons::{Icon, PaintIcon};

pub(crate) const TRIGGER_HEIGHT: f32 = 20.0;
const TRIGGER_PADDING_X: f32 = 4.0;
const TRIGGER_ICON_SIZE: f32 = 14.0;
const TRIGGER_GAP: f32 = 6.0;
const TRIGGER_TEXT_SIZE: f32 = 12.0;
const TRIGGER_RADIUS: u8 = 2;
const DISABLED_OPACITY: f32 = 0.5;
const POPUP_WIDTH: f32 = 256.0;
const POPUP_OFFSET: f32 = 4.0;
const POPUP_RADIUS: u8 = 6;
const BORDER_WIDTH: f32 = 1.0;
const SHADOW_OFFSET: [i8; 2] = [0, 2];
const SHADOW_BLUR: u8 = 8;
const INPUT_HEIGHT: f32 = 36.0;
const INPUT_PADDING_X: f32 = 12.0;
const INPUT_ICON_SIZE: f32 = 16.0;
const INPUT_GAP: f32 = 8.0;
const INPUT_TEXT_SIZE: f32 = 14.0;
const SEARCH_ICON_OPACITY: f32 = 0.5;
const LIST_MAX_HEIGHT: f32 = 300.0;
const GROUP_PADDING: f32 = 4.0;
const HEADING_HEIGHT: f32 = 28.0;
const HEADING_PADDING_X: f32 = 8.0;
const HEADING_TEXT_SIZE: f32 = 12.0;
const ITEM_HEIGHT: f32 = 32.0;
const ITEM_PADDING_X: f32 = 8.0;
const ITEM_GAP: f32 = 8.0;
const ITEM_ICON_SIZE: f32 = 16.0;
const ITEM_TEXT_SIZE: f32 = 14.0;
const ITEM_RADIUS: u8 = 2;
const RING_WIDTH: f32 = 1.0;
const CHECKOUT_TEXT_SIZE: f32 = 10.0;
const EMPTY_HEIGHT: f32 = 68.0;
const ELLIPSIS: char = '…';
const INPUT_EVENTS: egui::EventFilter = egui::EventFilter {
    tab: false,
    horizontal_arrows: true,
    vertical_arrows: true,
    escape: false,
};

/// The branch picker's filter, open state and highlighted item: popover-local state kept for as
/// long as the panel stays mounted (`branch-switcher.tsx`, cmdk's selected value).
#[derive(Clone, Default)]
struct Picker {
    is_open: bool,
    filter: String,
    active: usize,
    focus_input: bool,
    composition: Composition,
}

/// What the picker lists for a filter (`branch-switcher.tsx`, `branch-group.tsx`): a create item
/// for a trimmed name no branch has exactly, then the local and remote branches whose name
/// contains it, ignoring case.
pub(crate) struct Listing<'a> {
    pub create: Option<&'a str>,
    pub local: Vec<&'a GitBranch>,
    pub remote: Vec<&'a GitBranch>,
}

pub(crate) fn listing<'a>(branches: &'a [GitBranch], filter: &'a str) -> Listing<'a> {
    let trimmed = filter.trim();
    let needle = trimmed.to_lowercase();
    let visible = |is_remote: bool| {
        branches
            .iter()
            .filter(|branch| {
                branch.is_remote == is_remote && branch.name.to_lowercase().contains(&needle)
            })
            .collect()
    };
    Listing {
        create: (!trimmed.is_empty() && !branches.iter().any(|branch| branch.name == trimmed))
            .then_some(trimmed),
        local: visible(false),
        remote: visible(true),
    }
}

/// A picker item's effect (`handleSelect`/`handleCreate`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Create(String),
    Checkout(String),
    CheckoutRemote(String),
}

impl Listing<'_> {
    pub(crate) fn choices(&self) -> Vec<Choice> {
        self.create
            .map(|name| Choice::Create(name.to_owned()))
            .into_iter()
            .chain(self.local.iter().chain(&self.remote).map(|branch| {
                if branch.is_remote {
                    Choice::CheckoutRemote(branch.name.clone())
                } else {
                    Choice::Checkout(branch.name.clone())
                }
            }))
            .collect()
    }
}

fn label(view: &View<'_>, branch: Option<&str>) -> String {
    branch.map_or_else(|| view.text("git.noRepositoryLabel"), str::to_owned)
}

fn trigger_font(ui: &Ui) -> FontId {
    FontId::new(
        TRIGGER_TEXT_SIZE,
        taide_native_ui::font_families::medium(ui),
    )
}

/// The switcher button's natural width before the header shrinks it.
pub(crate) fn trigger_width(ui: &Ui, view: &View<'_>, branch: Option<&str>) -> f32 {
    let text = ui
        .painter()
        .layout_no_wrap(
            label(view, branch),
            trigger_font(ui),
            view.appearance.foreground,
        )
        .size()
        .x;
    2.0 * TRIGGER_PADDING_X + TRIGGER_ICON_SIZE + TRIGGER_GAP + text
}

fn truncated(
    ui: &Ui,
    text: String,
    font: FontId,
    color: egui::Color32,
    width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, width.max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some(ELLIPSIS);
    ui.painter().layout_job(job)
}

/// The header's branch switcher (`branch-switcher.tsx`): a trigger showing the current branch,
/// disabled without one, that toggles a filterable popover of branches to check out or create.
pub(crate) fn show(
    ui: &mut Ui,
    rect: Rect,
    view: &View<'_>,
    branch: Option<&str>,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let id = view.mounted_id("git-branch-switcher");
    let mut picker: Picker = ui.data(|data| data.get_temp(id)).unwrap_or_default();
    let appearance = view.appearance;
    let is_enabled = branch.is_some();
    let aria = view.text("git.branchSwitch");
    let trigger = ui.interact(
        rect,
        id.with("trigger"),
        if is_enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    trigger.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, &aria));
    ui.ctx().accesskit_node_builder(trigger.id, |node| {
        node.set_expanded(picker.is_open);
    });
    if is_enabled {
        view.tooltips.show(
            &trigger,
            &aria,
            egui::RectAlign::BOTTOM,
            view.tooltip_appearance,
        );
    }
    if is_enabled && trigger.hovered() {
        ui.painter()
            .rect_filled(rect, TRIGGER_RADIUS, appearance.hover);
    }
    let opacity = if is_enabled { 1.0 } else { DISABLED_OPACITY };
    let color = appearance.foreground.gamma_multiply(opacity);
    let painter = ui.painter_at(rect);
    let icon = Rect::from_min_size(
        pos2(
            rect.left() + TRIGGER_PADDING_X,
            rect.center().y - TRIGGER_ICON_SIZE / 2.0,
        ),
        vec2(TRIGGER_ICON_SIZE, TRIGGER_ICON_SIZE),
    );
    if rect.width() > 0.0 {
        icons.paint(ui, icon, Icon::GitBranch, color, 0.0)?;
    }
    let text_left = icon.right() + TRIGGER_GAP;
    let name = truncated(
        ui,
        label(view, branch),
        trigger_font(ui),
        color,
        rect.right() - TRIGGER_PADDING_X - text_left,
    );
    painter.galley(
        pos2(text_left, rect.center().y - name.size().y / 2.0),
        name,
        color,
    );
    if trigger.clicked() {
        picker.is_open = !picker.is_open;
        if picker.is_open {
            picker.active = 0;
            picker.focus_input = true;
        }
    }
    if picker.is_open {
        popup(ui, &trigger, id, view, &mut picker, icons, intents)?;
    }
    ui.data_mut(|data| data.insert_temp(id, picker));
    Ok(())
}

/// The popover (`PopoverContent` + cmdk `Command`): the filter input keeps focus, ↑/↓/Home/End
/// move the highlight without wrapping, the pointer highlights what it moves over, Enter or a
/// click runs the highlighted item, and Escape or a click outside closes it with the filter kept.
fn popup(
    ui: &mut Ui,
    trigger: &egui::Response,
    id: Id,
    view: &View<'_>,
    picker: &mut Picker,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let appearance = view.appearance;
    let input_id = id.with("filter");
    let events = ui.input(|input| input.events.clone());
    let is_composing = picker.composition.observe(&events);
    let filter = picker.filter.clone();
    let listing = listing(view.branches, &filter);
    let choices = listing.choices();
    let last = choices.len().saturating_sub(1);
    let mut chosen = None;
    let mut is_moved = false;
    if ui.memory(|memory| memory.has_focus(input_id)) && !is_composing {
        let pressed = |key| ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key));
        let previous = picker.active;
        if pressed(Key::ArrowDown) {
            picker.active = (picker.active + 1).min(last);
        }
        if pressed(Key::ArrowUp) {
            picker.active = picker.active.saturating_sub(1);
        }
        if pressed(Key::Home) {
            picker.active = 0;
        }
        if pressed(Key::End) {
            picker.active = last;
        }
        is_moved = picker.active != previous;
        if pressed(Key::Enter) {
            chosen = choices.get(picker.active).cloned();
        }
    }
    picker.active = picker.active.min(last);
    let shown = egui::Popup::new(
        id.with("popup"),
        ui.ctx().clone(),
        egui::PopupAnchor::ParentRect(trigger.rect),
        ui.layer_id(),
    )
    .open(true)
    .align(egui::RectAlign::BOTTOM_START)
    .gap(POPUP_OFFSET)
    .width(POPUP_WIDTH)
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .frame(egui::Frame::NONE)
    .show(|ui| -> AppResult<Option<(usize, Option<Choice>)>> {
        egui::Frame::NONE
            .fill(appearance.panel)
            .stroke(Stroke::new(BORDER_WIDTH, appearance.popover_border))
            .shadow(egui::Shadow {
                offset: SHADOW_OFFSET,
                blur: SHADOW_BLUR,
                spread: 0,
                color: appearance.shadow,
            })
            .corner_radius(POPUP_RADIUS)
            .show(ui, |ui| {
                let width = POPUP_WIDTH - 2.0 * BORDER_WIDTH;
                ui.set_width(width);
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                let (row, _) = ui.allocate_exact_size(vec2(width, INPUT_HEIGHT), Sense::hover());
                let search = Rect::from_min_size(
                    pos2(
                        row.left() + INPUT_PADDING_X,
                        row.center().y - INPUT_ICON_SIZE / 2.0,
                    ),
                    vec2(INPUT_ICON_SIZE, INPUT_ICON_SIZE),
                );
                icons.paint(
                    ui,
                    search,
                    Icon::Search,
                    appearance.foreground.gamma_multiply(SEARCH_ICON_OPACITY),
                    0.0,
                )?;
                let input = ui.put(
                    Rect::from_min_max(
                        pos2(search.right() + INPUT_GAP, row.top()),
                        pos2(row.right() - INPUT_PADDING_X, row.bottom()),
                    ),
                    egui::TextEdit::singleline(&mut picker.filter)
                        .id(input_id)
                        .return_key(None)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .min_size(vec2(0.0, INPUT_HEIGHT))
                        .desired_width(f32::INFINITY)
                        .vertical_align(egui::Align::Center)
                        .text_color(appearance.foreground)
                        .font(FontId::proportional(INPUT_TEXT_SIZE))
                        .hint_text(
                            egui::RichText::new(view.text("git.branchFilterPlaceholder"))
                                .color(appearance.muted),
                        ),
                );
                if input.changed() {
                    picker.active = 0;
                }
                if picker.focus_input && !ui.is_sizing_pass() {
                    input.request_focus();
                    picker.focus_input = false;
                }
                if input.has_focus() {
                    ui.memory_mut(|memory| memory.set_focus_lock_filter(input_id, INPUT_EVENTS));
                }
                let separator = row.bottom() - BORDER_WIDTH / 2.0;
                ui.painter().line_segment(
                    [pos2(row.left(), separator), pos2(row.right(), separator)],
                    Stroke::new(BORDER_WIDTH, appearance.popover_separator),
                );
                egui::ScrollArea::vertical()
                    .id_salt(id.with("list"))
                    .max_height(LIST_MAX_HEIGHT)
                    .show(ui, |ui| {
                        Items {
                            view,
                            id,
                            active: picker.active,
                            is_moved,
                            width,
                            index: 0,
                        }
                        .show(ui, &listing, &choices, icons)
                    })
                    .inner
            })
            .inner
    });
    let Some(shown) = shown else {
        return Ok(());
    };
    let pointed = shown.inner?;
    if let Some((active, _)) = pointed {
        picker.active = active;
    }
    let is_escape = ui.input(|input| input.key_pressed(Key::Escape));
    let closes = shown.response.should_close() && !(is_escape && is_composing);
    if let Some(choice) = chosen.or(pointed.and_then(|(_, choice)| choice)) {
        let project = view.project.clone();
        intents.push(ShellIntent::Git(match choice {
            Choice::Create(name) => Intent::CreateBranch { project, name },
            Choice::Checkout(name) => Intent::CheckoutBranch { project, name },
            Choice::CheckoutRemote(remote_ref) => Intent::CheckoutRemoteBranch {
                project,
                remote_ref,
            },
        }));
        *picker = Picker::default();
        trigger.request_focus();
    } else if closes {
        picker.is_open = false;
        picker.composition.reset();
        if is_escape {
            ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape));
            trigger.request_focus();
        }
    }
    Ok(())
}

/// The popover's list body, drawing items in cmdk order and reporting the hovered and clicked
/// ones.
struct Items<'a, 'v> {
    view: &'a View<'v>,
    id: Id,
    active: usize,
    is_moved: bool,
    width: f32,
    index: usize,
}

impl Items<'_, '_> {
    fn show(
        &mut self,
        ui: &mut Ui,
        listing: &Listing<'_>,
        choices: &[Choice],
        icons: &mut impl PaintIcon<Icon>,
    ) -> AppResult<Option<(usize, Option<Choice>)>> {
        let appearance = self.view.appearance;
        if choices.is_empty() {
            let (rect, _) = ui.allocate_exact_size(vec2(self.width, EMPTY_HEIGHT), Sense::hover());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.view.text("git.branchNotFound"),
                FontId::proportional(ITEM_TEXT_SIZE),
                appearance.foreground,
            );
            return Ok(None);
        }
        let mut outcome = None;
        if let Some(name) = listing.create {
            ui.add_space(GROUP_PADDING);
            let text = crate::presentation::message(
                self.view.locale,
                "git.branchCreateNamed",
                &[("name", name)],
            );
            if let Some(found) = self.item(ui, &text, Icon::Plus, None, &choices[0], icons)? {
                outcome = Some(found);
            }
            ui.add_space(GROUP_PADDING);
        }
        for (heading, branches) in [
            ("git.branchLocal", &listing.local),
            ("git.branchRemote", &listing.remote),
        ] {
            if branches.is_empty() {
                continue;
            }
            ui.add_space(GROUP_PADDING);
            let (rect, _) =
                ui.allocate_exact_size(vec2(self.width, HEADING_HEIGHT), Sense::hover());
            ui.painter().text(
                pos2(
                    rect.left() + GROUP_PADDING + HEADING_PADDING_X,
                    rect.center().y,
                ),
                egui::Align2::LEFT_CENTER,
                self.view.text(heading),
                FontId::new(
                    HEADING_TEXT_SIZE,
                    taide_native_ui::font_families::medium(ui),
                ),
                appearance.muted,
            );
            for branch in branches {
                let choice = choices[self.index].clone();
                if let Some(found) = self.item(
                    ui,
                    &branch.name,
                    Icon::GitBranch,
                    Some(branch),
                    &choice,
                    icons,
                )? {
                    outcome = Some(found);
                }
            }
            ui.add_space(GROUP_PADDING);
        }
        Ok(outcome)
    }

    /// One cmdk item: highlighted with the list's active background and an inset accent ring,
    /// the current branch's icon and trailing check in the accent color, remote branches marked
    /// with the checkout hint.
    fn item(
        &mut self,
        ui: &mut Ui,
        text: &str,
        icon: Icon,
        branch: Option<&GitBranch>,
        choice: &Choice,
        icons: &mut impl PaintIcon<Icon>,
    ) -> AppResult<Option<(usize, Option<Choice>)>> {
        let appearance = self.view.appearance;
        let index = self.index;
        self.index += 1;
        let (slot, _) = ui.allocate_exact_size(vec2(self.width, ITEM_HEIGHT), Sense::hover());
        let rect = slot.shrink2(vec2(GROUP_PADDING, 0.0));
        let response = ui.interact(
            rect,
            self.id.with(("item", choice_key(choice))),
            Sense::click(),
        );
        let is_active = index == self.active;
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, is_active, text)
        });
        if is_active && self.is_moved {
            ui.scroll_to_rect(slot, None);
        }
        if is_active {
            ui.painter().rect(
                rect,
                ITEM_RADIUS,
                appearance.list_active,
                Stroke::new(RING_WIDTH, appearance.accent),
                egui::StrokeKind::Inside,
            );
        }
        let is_head = branch.is_some_and(|branch| branch.is_head);
        let icon_color = if is_head {
            appearance.accent
        } else {
            appearance.muted
        };
        let glyph = Rect::from_min_size(
            pos2(
                rect.left() + ITEM_PADDING_X,
                rect.center().y - ITEM_ICON_SIZE / 2.0,
            ),
            vec2(ITEM_ICON_SIZE, ITEM_ICON_SIZE),
        );
        icons.paint(ui, glyph, icon, icon_color, 0.0)?;
        let mut right = rect.right() - ITEM_PADDING_X;
        if is_head {
            right -= ITEM_ICON_SIZE;
            icons.paint(
                ui,
                Rect::from_min_size(
                    pos2(right, rect.center().y - ITEM_ICON_SIZE / 2.0),
                    vec2(ITEM_ICON_SIZE, ITEM_ICON_SIZE),
                ),
                Icon::Check,
                appearance.accent,
                0.0,
            )?;
            right -= ITEM_GAP;
        }
        if branch.is_some_and(|branch| branch.is_remote) {
            let hint = ui.painter().layout_no_wrap(
                self.view.text("git.checkoutRemoteBranch"),
                FontId::proportional(CHECKOUT_TEXT_SIZE),
                appearance.muted,
            );
            right -= hint.size().x;
            ui.painter().galley(
                pos2(right, rect.center().y - hint.size().y / 2.0),
                hint,
                appearance.muted,
            );
            right -= ITEM_GAP;
        }
        let color = if is_active {
            appearance.button_hover_foreground
        } else {
            appearance.foreground
        };
        let left = glyph.right() + ITEM_GAP;
        let name = truncated(
            ui,
            text.to_owned(),
            FontId::proportional(ITEM_TEXT_SIZE),
            color,
            right - left,
        );
        ui.painter().galley(
            pos2(left, rect.center().y - name.size().y / 2.0),
            name,
            color,
        );
        let is_pointer_moving = ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO);
        if response.clicked() {
            return Ok(Some((index, Some(choice.clone()))));
        }
        if response.hovered() && is_pointer_moving && !is_active {
            return Ok(Some((index, None)));
        }
        Ok(None)
    }
}

fn choice_key(choice: &Choice) -> (&'static str, &str) {
    match choice {
        Choice::Create(_) => ("create", ""),
        Choice::Checkout(name) => ("local", name),
        Choice::CheckoutRemote(name) => ("remote", name),
    }
}

#[cfg(test)]
#[path = "git-branch-tests.rs"]
mod tests;

use eframe::egui::{self, Align2, FontId, Id, Key, Popup, Response, Sense, Ui, vec2};
use taide_model::locale::ResolvedLocale;

use crate::symbol_outline::Appearance;

const TEXT_SIZE: f32 = 12.0;
const TRIGGER_PADDING: egui::Vec2 = vec2(4.0, 2.0);
const TRIGGER_LINE_HEIGHT: f32 = 16.0;
const RADIUS: u8 = 2;
const MENU_RADIUS: u8 = 6;
const MENU_PADDING: i8 = 4;
const MENU_GAP: f32 = 4.0;
const MENU_MIN_WIDTH: f32 = 128.0;
const MENU_ROW_HEIGHT: f32 = 32.0;
const MENU_TEXT_SIZE: f32 = 14.0;
const MENU_ROW_PADDING: f32 = 8.0;
const DISABLED_OPACITY: f32 = 0.5;
const BORDER_WIDTH: f32 = 1.0;
const TYPEAHEAD_RESET: f64 = 1.0;

pub(crate) enum Target {
    File(String),
    Symbol(usize),
}

pub(crate) struct Entry {
    pub key: String,
    pub label: String,
    pub target: Option<Target>,
}

#[derive(Default)]
pub(crate) struct Menu {
    offset: f32,
    was_focused: bool,
    search: String,
    search_time: f64,
}

pub(crate) struct Output {
    pub opened: bool,
    pub selected: Option<Target>,
    pub trigger: Response,
    pub entries: Vec<Response>,
}

impl Menu {
    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        id: Id,
        label: &str,
        emphasized: bool,
        interactive: bool,
        entries: impl FnOnce() -> Vec<Entry>,
        keyboard: bool,
        locale: &ResolvedLocale,
        appearance: &Appearance,
    ) -> Output {
        let color = if emphasized {
            appearance.editor_foreground
        } else {
            appearance.muted
        };
        let galley = ui.painter().layout_no_wrap(
            label.replace(['\n', '\r'], " "),
            FontId::proportional(TEXT_SIZE),
            color,
        );
        let (rect, _) = ui.allocate_exact_size(
            vec2(
                galley.size().x + TRIGGER_PADDING.x * 2.0,
                TRIGGER_LINE_HEIGHT + TRIGGER_PADDING.y * 2.0,
            ),
            Sense::hover(),
        );
        let trigger = ui.interact(
            rect,
            id.with("trigger"),
            if interactive {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        if interactive && trigger.hovered() {
            ui.painter().rect_filled(rect, RADIUS, appearance.hover);
        }
        ui.painter().galley(
            rect.center() - galley.size() / 2.0,
            galley,
            if interactive && trigger.hovered() {
                appearance.foreground
            } else {
                color
            },
        );
        trigger.widget_info(|| {
            egui::WidgetInfo::labeled(
                if interactive {
                    egui::WidgetType::Button
                } else {
                    egui::WidgetType::Label
                },
                ui.is_enabled(),
                if interactive {
                    format!(
                        "{}: {label}",
                        crate::presentation::message(locale, "breadcrumbs.dropdownAriaLabel", &[])
                    )
                } else {
                    label.into()
                },
            )
        });
        if !interactive {
            return Output {
                opened: false,
                selected: None,
                trigger,
                entries: Vec::new(),
            };
        }
        let was_open = Popup::is_id_open(ui.ctx(), id);
        let activated = keyboard
            && ui.is_enabled()
            && trigger.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, Key::Enter)
                    || input.consume_key(egui::Modifiers::NONE, Key::Space)
                    || (!was_open && input.consume_key(egui::Modifiers::NONE, Key::ArrowDown))
            });
        if trigger.clicked_by(egui::PointerButton::Primary)
            || (keyboard && trigger.clicked())
            || activated
        {
            Popup::toggle_id(ui.ctx(), id);
            trigger.request_focus();
        }
        let opened = !was_open && Popup::is_id_open(ui.ctx(), id);
        if !Popup::is_id_open(ui.ctx(), id) {
            self.was_focused = false;
            self.search.clear();
            ui.ctx().accesskit_node_builder(trigger.id, |node| {
                node.set_role(egui::accesskit::Role::Button);
                node.set_expanded(false);
            });
            return Output {
                opened,
                selected: None,
                trigger,
                entries: Vec::new(),
            };
        }
        let entries = entries();
        let mut selected = None;
        let mut responses = Vec::new();
        let enabled = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.target.is_some())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let focused = ui.ctx().memory(|memory| memory.focused());
        let index = enabled
            .iter()
            .position(|index| focused == Some(id.with(&entries[*index].key)));
        let owns_focus = index.is_some()
            || trigger.has_focus()
            || (self.was_focused
                && focused.is_none()
                && ui.input(|input| input.key_pressed(Key::Escape)));
        let time = ui.input(|input| input.time);
        if !owns_focus
            || !Popup::is_id_open(ui.ctx(), id)
            || time - self.search_time >= TYPEAHEAD_RESET
        {
            self.search.clear();
        }
        let space_typing = !self.search.is_empty()
            && ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: Key::Space,
                            ..
                        }
                    )
                })
            });
        let mut typed = None;
        if keyboard && owns_focus && Popup::is_id_open(ui.ctx(), id) && ui.is_enabled() {
            ui.input_mut(|input| {
                if input.modifiers.alt || input.modifiers.ctrl || input.modifiers.command {
                    return;
                }
                input.events.retain(|event| {
                    if let egui::Event::Text(text) = event
                        && text.chars().count() == 1
                        && !text.chars().any(char::is_control)
                    {
                        self.search.push_str(text);
                        self.search_time = time;
                        typed = next_match(
                            &entries,
                            &enabled,
                            &self.search,
                            index.map(|index| enabled[index]),
                        );
                        return false;
                    }
                    true
                });
            });
        }
        if owns_focus
            && Popup::is_id_open(ui.ctx(), id)
            && ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: Key::ArrowDown | Key::ArrowUp | Key::Tab,
                            pressed: true,
                            ..
                        }
                    )
                })
            })
        {
            ui.ctx()
                .memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
        }
        let navigation = (keyboard && owns_focus && Popup::is_id_open(ui.ctx(), id))
            .then(|| {
                ui.input_mut(|input| {
                    for key in [
                        Key::ArrowDown,
                        Key::ArrowUp,
                        Key::Home,
                        Key::End,
                        Key::Escape,
                        Key::Tab,
                    ] {
                        if input.consume_key(egui::Modifiers::NONE, key)
                            || (key == Key::Tab && input.consume_key(egui::Modifiers::SHIFT, key))
                        {
                            return Some(key);
                        }
                    }
                    None
                })
            })
            .flatten();
        if navigation == Some(Key::Escape) {
            Popup::close_id(ui.ctx(), id);
            trigger.request_focus();
        }
        let next = match navigation {
            Some(Key::ArrowDown) => {
                index.map_or(0, |index| (index + 1).min(enabled.len().saturating_sub(1)))
            }
            Some(Key::ArrowUp) => index.map_or(enabled.len().saturating_sub(1), |index| {
                index.saturating_sub(1)
            }),
            Some(Key::End) => enabled.len().saturating_sub(1),
            Some(Key::Home) => 0,
            _ => index.unwrap_or(0),
        };
        let focus = typed.or_else(|| {
            enabled.get(next).copied().filter(|_| {
                opened
                    || (keyboard
                        && (navigation.is_some_and(|key| !matches!(key, Key::Escape | Key::Tab))
                            || (Popup::is_id_open(ui.ctx(), id) && trigger.has_focus())))
            })
        });
        if let Some(index) = focus {
            self.offset = index as f32 * MENU_ROW_HEIGHT;
        }
        let expanded = Popup::is_id_open(ui.ctx(), id);
        ui.ctx().accesskit_node_builder(trigger.id, |node| {
            node.set_role(egui::accesskit::Role::Button);
            node.set_expanded(expanded);
        });
        let width = entries
            .iter()
            .map(|entry| {
                ui.painter()
                    .layout_no_wrap(
                        entry.label.clone(),
                        FontId::proportional(MENU_TEXT_SIZE),
                        appearance.foreground,
                    )
                    .size()
                    .x
                    + MENU_ROW_PADDING * 2.0
            })
            .fold(MENU_MIN_WIDTH, f32::max)
            .min(ui.ctx().content_rect().width());
        let frame = egui::Frame::popup(ui.style())
            .fill(appearance.menu)
            .stroke(egui::Stroke::new(BORDER_WIDTH, appearance.menu_border))
            .corner_radius(MENU_RADIUS)
            .inner_margin(MENU_PADDING);
        Popup::menu(&trigger)
            .id(id)
            .open_memory(None)
            .gap(MENU_GAP)
            .width(width)
            .frame(frame)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let scroll = egui::ScrollArea::vertical()
                    .id_salt(id.with("scroll"))
                    .vertical_scroll_offset(self.offset)
                    .max_height(ui.ctx().content_rect().height() - MENU_ROW_HEIGHT)
                    .show_rows(ui, MENU_ROW_HEIGHT, entries.len(), |ui, range| {
                        for index in range {
                            let entry = &entries[index];
                            let (rect, _) = ui
                                .allocate_exact_size(vec2(width, MENU_ROW_HEIGHT), Sense::hover());
                            let row = ui.interact(
                                rect,
                                id.with(&entry.key),
                                if entry.target.is_some() {
                                    Sense::click()
                                } else {
                                    Sense::hover()
                                },
                            );
                            if focus == Some(index) && ui.is_enabled() {
                                row.request_focus();
                            }
                            if row.has_focus() {
                                ui.ctx().memory_mut(|memory| {
                                    memory.set_focus_lock_filter(
                                        row.id,
                                        egui::EventFilter {
                                            tab: true,
                                            vertical_arrows: true,
                                            ..Default::default()
                                        },
                                    )
                                });
                            }
                            if row.has_focus() || (entry.target.is_some() && row.hovered()) {
                                ui.painter()
                                    .rect_filled(rect, RADIUS, appearance.menu_hover);
                            }
                            let color = if entry.target.is_some() {
                                appearance.foreground
                            } else {
                                appearance.foreground.gamma_multiply(DISABLED_OPACITY)
                            };
                            ui.painter().text(
                                rect.left_center() + vec2(MENU_ROW_PADDING, 0.0),
                                Align2::LEFT_CENTER,
                                entry.label.replace(['\n', '\r'], " "),
                                FontId::proportional(MENU_TEXT_SIZE),
                                color,
                            );
                            row.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled() && entry.target.is_some(),
                                    &entry.label,
                                )
                            });
                            ui.ctx().accesskit_node_builder(row.id, |node| {
                                node.set_role(egui::accesskit::Role::MenuItem)
                            });
                            let activated = keyboard
                                && row.has_focus()
                                && ui.is_enabled()
                                && ui.input_mut(|input| {
                                    input.consume_key(egui::Modifiers::NONE, Key::Enter)
                                        || (!space_typing
                                            && input.consume_key(egui::Modifiers::NONE, Key::Space))
                                });
                            if (row.clicked_by(egui::PointerButton::Primary)
                                || (keyboard && !space_typing && row.clicked())
                                || activated)
                                && let Some(target) = &entry.target
                            {
                                selected = Some(match target {
                                    Target::File(path) => Target::File(path.clone()),
                                    Target::Symbol(index) => Target::Symbol(*index),
                                });
                                ui.close();
                                trigger.request_focus();
                            }
                            if entry.target.is_some()
                                && row.hovered()
                                && ui.input(|input| input.pointer.delta() != egui::Vec2::ZERO)
                            {
                                row.request_focus();
                            }
                            responses.push(row);
                        }
                    });
                self.offset = scroll.state.offset.y;
            });
        self.was_focused = trigger.has_focus() || responses.iter().any(Response::has_focus);
        Output {
            opened,
            selected,
            trigger,
            entries: responses,
        }
    }
}

fn next_match(
    entries: &[Entry],
    enabled: &[usize],
    search: &str,
    current: Option<usize>,
) -> Option<usize> {
    let first = search.chars().next()?;
    let repeated = search.chars().all(|character| character == first);
    let normalized = if repeated {
        first.to_string()
    } else {
        search.to_owned()
    }
    .to_lowercase();
    let exclude_current = normalized.chars().count() == 1;
    let start = current
        .and_then(|current| enabled.iter().position(|index| *index == current))
        .unwrap_or(0);
    enabled
        .iter()
        .cycle()
        .skip(start)
        .take(enabled.len())
        .copied()
        .find(|index| {
            (!exclude_current || Some(*index) != current)
                && entries[*index]
                    .label
                    .to_lowercase()
                    .starts_with(&normalized)
        })
        .filter(|index| Some(*index) != current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 이름_키검색은_긴_접두사가_현재_행과_맞으면_다른_형제로_이동하지_않는다() {
        let entries = ["Alpha", "Alpine", "Algae"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| Entry {
                key: index.to_string(),
                label: label.into(),
                target: Some(Target::Symbol(index)),
            })
            .collect::<Vec<_>>();
        let enabled = [0, 1, 2];
        assert_eq!(next_match(&entries, &enabled, "a", Some(0)), Some(1));
        assert_eq!(next_match(&entries, &enabled, "al", Some(0)), None);
        assert_eq!(next_match(&entries, &enabled, "ALP", Some(1)), None);
        assert_eq!(next_match(&entries, &enabled, "alpha", Some(1)), Some(0));
        assert_eq!(next_match(&entries, &enabled, "aaa", Some(0)), Some(1));
    }
}

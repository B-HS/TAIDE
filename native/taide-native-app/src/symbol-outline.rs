use std::collections::HashSet;

use eframe::egui::{self, Color32, FontId, Id, Key, Rect, Response, Sense, Ui, vec2};
use taide_model::{error::AppResult, locale::ResolvedLocale, theme::ResolvedTheme};
use taide_native_ui::command_palette::SymbolIndex;

use crate::navigation_icons::{Icon, Icons};
use crate::symbol_navigation::{Row, Tree};

const ROW_HEIGHT: f32 = 20.0;
const OVERSCAN: usize = 12;
const TEXT_SIZE: f32 = 12.0;
const BASE_INDENT: f32 = 8.0;
const DEPTH_INDENT: f32 = 16.0;
const CHEVRON_SIZE: f32 = 12.0;
const ICON_SIZE: f32 = 14.0;
const EMPTY_ICON_SIZE: f32 = 20.0;
const GAP: f32 = 6.0;
const EMPTY_GAP: f32 = 8.0;
const EMPTY_OPACITY: f32 = 0.6;
const EXPANDED_ANGLE: f32 = std::f32::consts::FRAC_PI_2;

pub(crate) struct Appearance {
    pub panel: Color32,
    pub sidebar: Color32,
    pub sidebar_border: Color32,
    pub editor: Color32,
    pub foreground: Color32,
    pub editor_foreground: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub selected: Color32,
    pub focused: Color32,
    pub hover: Color32,
    pub menu: Color32,
    pub menu_border: Color32,
    pub menu_hover: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| crate::presentation::color(theme, key);
        Ok(Self {
            panel: color("panel.background")?,
            sidebar: color("explorer.background")?,
            sidebar_border: color("tabBar.tabBorder")?,
            editor: color("editor.background")?,
            foreground: color("app.foreground")?,
            editor_foreground: color("editor.foreground")?,
            muted: color("appSidebar.iconDefault")?,
            border: color("app.border")?,
            selected: color("explorer.itemSelected")?,
            focused: color("explorer.itemFocused")?,
            hover: color("explorer.itemHover")?,
            menu: color("menu.background")?,
            menu_border: color("menu.border")?,
            menu_hover: color("menu.itemHover")?,
        })
    }
}

pub(crate) struct Scope<'a> {
    pub path: Option<&'a str>,
    pub symbols: SymbolIndex<'a>,
}

#[derive(Default)]
pub(crate) struct Panel {
    path: Option<String>,
    generation: Option<u64>,
    tree: Option<Tree>,
    rows: Vec<Row>,
    collapsed: HashSet<String>,
    selected: Option<String>,
    reveal: Option<usize>,
    scroll_offset: f32,
    composing: bool,
}

#[derive(Default)]
pub(crate) struct Output {
    pub reveal: Option<(u64, usize)>,
    pub rows: Vec<(usize, Response)>,
    pub tree: Option<Response>,
}

impl Panel {
    fn reconcile(&mut self, scope: &Scope<'_>) {
        if self.path.as_deref() != scope.path {
            *self = Self {
                path: scope.path.map(str::to_owned),
                ..Default::default()
            };
        }
        let Some(symbols) = scope.path.and(scope.symbols.entries) else {
            self.tree = None;
            self.rows.clear();
            self.generation = None;
            return;
        };
        if self.generation != Some(scope.symbols.generation) || self.tree.is_none() {
            self.tree = Some(Tree::new(symbols));
            self.generation = Some(scope.symbols.generation);
            self.refresh_rows();
        }
    }

    fn refresh_rows(&mut self) {
        self.rows = self
            .tree
            .as_ref()
            .map_or_else(Vec::new, |tree| tree.rows(&self.collapsed));
        self.reveal = None;
    }

    fn select(&mut self, index: usize) {
        if let Some(row) = self.rows.get(index) {
            self.selected = Some(row.id.clone());
            self.reveal = Some(index);
        }
    }

    fn toggle(&mut self, index: usize) {
        if let Some(row) = self.rows.get(index).filter(|row| row.has_children) {
            if !self.collapsed.remove(&row.id) {
                self.collapsed.insert(row.id.clone());
            }
            self.refresh_rows();
        }
    }

    fn key(&mut self, key: Key, output: &mut Output) -> bool {
        let selected = self
            .rows
            .iter()
            .position(|row| self.selected.as_ref() == Some(&row.id));
        match key {
            Key::ArrowDown => self.select(selected.map_or(0, |index| index + 1)),
            Key::ArrowUp => {
                if let Some(index) = selected.map_or_else(
                    || self.rows.len().checked_sub(1),
                    |index| index.checked_sub(1),
                ) {
                    self.select(index);
                }
            }
            Key::ArrowRight | Key::ArrowLeft | Key::Enter => {
                let Some(index) = selected else {
                    return false;
                };
                let row = &self.rows[index];
                match key {
                    Key::ArrowRight if row.has_children && row.collapsed => self.toggle(index),
                    Key::ArrowRight if row.has_children => self.select(index + 1),
                    Key::ArrowLeft if row.has_children && !row.collapsed => self.toggle(index),
                    Key::ArrowLeft => {
                        if let Some(parent) =
                            self.tree.as_ref().and_then(|tree| tree.parent(row.symbol))
                            && let Some(index) =
                                self.rows.iter().position(|row| row.symbol == parent)
                        {
                            self.select(index);
                        }
                    }
                    Key::Enter => {
                        output.reveal = self.generation.map(|generation| (generation, row.symbol))
                    }
                    _ => {}
                }
            }
            _ => return false,
        }
        true
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        id: Id,
        scope: Scope<'_>,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &mut Icons,
    ) -> AppResult<Output> {
        self.reconcile(&scope);
        let mut output = Output::default();
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, appearance.panel);
        if self.rows.is_empty() {
            let icon = Rect::from_center_size(
                rect.center() - vec2(0.0, EMPTY_ICON_SIZE / 2.0 + EMPTY_GAP / 2.0),
                vec2(EMPTY_ICON_SIZE, EMPTY_ICON_SIZE),
            );
            icons.paint(
                ui,
                icon,
                Icon::ListTree,
                appearance.muted.gamma_multiply(EMPTY_OPACITY),
                0.0,
            )?;
            let key = if scope.path.is_some() {
                "outline.empty"
            } else {
                "outline.noActiveFile"
            };
            ui.painter().text(
                rect.center() + vec2(0.0, EMPTY_GAP / 2.0),
                egui::Align2::CENTER_TOP,
                crate::presentation::message(locale, key, &[]),
                FontId::proportional(TEXT_SIZE),
                appearance.muted,
            );
            ui.allocate_rect(rect, Sense::hover());
            return Ok(output);
        }
        let tree = ui.interact(rect, id, Sense::focusable_noninteractive());
        let has_focus = tree.has_focus();
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(egui::accesskit::Role::Tree);
            node.set_label(crate::presentation::message(locale, "outline.title", &[]));
        });
        if has_focus {
            let has_arrow = ui.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Key {
                            key: Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight,
                            pressed: true,
                            ..
                        }
                    )
                })
            });
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
                if has_arrow {
                    memory.move_focus(egui::FocusDirection::None);
                }
            });
        } else {
            self.composing = false;
        }
        let ime = ui.input(|input| {
            let mut found = false;
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    found = true;
                    if has_focus {
                        match event {
                            egui::ImeEvent::Preedit { text, .. } => {
                                self.composing = !text.is_empty()
                            }
                            egui::ImeEvent::Commit(_) => self.composing = false,
                            _ => {}
                        }
                    }
                }
            }
            found
        });
        if has_focus
            && ui.is_enabled()
            && !ime
            && !self.composing
            && !egui::Popup::is_any_open(ui.ctx())
        {
            ui.input_mut(|input| {
                input.events.retain(|event| {
                    if let egui::Event::Key {
                        key, pressed: true, ..
                    } = event
                    {
                        return !self.key(*key, &mut output);
                    }
                    true
                })
            });
        }
        if let Some(index) = self.reveal.take() {
            let top = index as f32 * ROW_HEIGHT;
            let bottom = top + ROW_HEIGHT;
            if top < self.scroll_offset {
                self.scroll_offset = top;
            }
            if bottom > self.scroll_offset + rect.height() {
                self.scroll_offset = (bottom - rect.height()).max(0.0);
            }
        }
        let symbols = scope.symbols.entries.unwrap_or_default();
        let mut toggled = None;
        ui.spacing_mut().item_spacing.y = 0.0;
        let scroll = egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .vertical_scroll_offset(self.scroll_offset)
            .show_viewport(ui, |ui, viewport| -> AppResult<()> {
                ui.set_height(self.rows.len() as f32 * ROW_HEIGHT);
                let start = (viewport.min.y.max(0.0) / ROW_HEIGHT).floor() as usize;
                let end = (viewport.max.y.max(0.0) / ROW_HEIGHT).ceil() as usize;
                for index in start.saturating_sub(OVERSCAN)
                    ..end.saturating_add(OVERSCAN).min(self.rows.len())
                {
                    let row = &self.rows[index];
                    let symbol = &symbols[row.symbol];
                    let rect = Rect::from_min_size(
                        ui.max_rect().min + vec2(0.0, index as f32 * ROW_HEIGHT),
                        vec2(ui.available_width(), ROW_HEIGHT),
                    );
                    let response = ui.interact(rect, id.with(&row.id), Sense::CLICK);
                    let selected = self.selected.as_ref() == Some(&row.id);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            &symbol.name,
                        )
                    });
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_role(egui::accesskit::Role::TreeItem);
                        node.set_selected(selected);
                        if row.has_children {
                            node.set_expanded(!row.collapsed);
                        }
                        if !symbol.detail.is_empty() {
                            node.set_description(symbol.detail.clone());
                        }
                    });
                    if selected || response.hovered() {
                        let color = if selected {
                            if has_focus {
                                appearance.selected
                            } else {
                                appearance.focused
                            }
                        } else {
                            appearance.hover
                        };
                        ui.painter().rect_filled(rect, 0.0, color);
                    }
                    let left = rect.left() + BASE_INDENT + row.depth as f32 * DEPTH_INDENT;
                    let chevron = Rect::from_center_size(
                        egui::pos2(left + CHEVRON_SIZE / 2.0, rect.center().y),
                        vec2(CHEVRON_SIZE, CHEVRON_SIZE),
                    );
                    let mut toggle_clicked = false;
                    if row.has_children {
                        let toggle =
                            ui.interact(chevron, id.with((&row.id, "toggle")), Sense::CLICK);
                        toggle_clicked = toggle.clicked();
                        icons.paint(
                            ui,
                            chevron,
                            Icon::ChevronRight,
                            appearance.foreground,
                            if row.collapsed { 0.0 } else { EXPANDED_ANGLE },
                        )?;
                    }
                    let icon = Rect::from_center_size(
                        egui::pos2(chevron.right() + GAP + ICON_SIZE / 2.0, rect.center().y),
                        vec2(ICON_SIZE, ICON_SIZE),
                    );
                    icons.paint(ui, icon, Icon::symbol(symbol.kind), appearance.muted, 0.0)?;
                    paint_text(
                        ui,
                        rect,
                        icon.right() + GAP,
                        &symbol.name,
                        &symbol.detail,
                        appearance,
                    );
                    if toggle_clicked {
                        toggled = Some(index);
                        tree.request_focus();
                    } else if response.clicked() {
                        self.selected = Some(row.id.clone());
                        output.reveal = Some((scope.symbols.generation, row.symbol));
                        tree.request_focus();
                    }
                    if response.has_focus() {
                        tree.request_focus();
                    }
                    output.rows.push((row.symbol, response));
                }
                Ok(())
            });
        scroll.inner?;
        self.scroll_offset = scroll.state.offset.y;
        if let Some(index) = toggled {
            self.toggle(index);
        }
        output.tree = Some(tree);
        Ok(output)
    }
}

fn paint_text(ui: &Ui, rect: Rect, left: f32, name: &str, detail: &str, appearance: &Appearance) {
    let width = (rect.right() - BASE_INDENT - left).max(0.0);
    let font = FontId::proportional(TEXT_SIZE);
    let natural_name = ui
        .painter()
        .layout_no_wrap(name.into(), font.clone(), appearance.foreground)
        .size()
        .x;
    let natural_detail = ui
        .painter()
        .layout_no_wrap(detail.into(), font.clone(), appearance.muted)
        .size()
        .x;
    let natural = natural_name + natural_detail;
    let available = if detail.is_empty() {
        width
    } else {
        (width - GAP).max(0.0)
    };
    let name_width = if natural <= available || natural == 0.0 {
        natural_name.min(available)
    } else {
        available * natural_name / natural
    };
    let label = |text: &str, width: f32, color: Color32| {
        let mut job = egui::text::LayoutJob::simple(text.into(), font.clone(), color, width);
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.wrap.overflow_character = Some('…');
        ui.painter().layout_job(job)
    };
    let name = label(name, name_width, appearance.foreground);
    ui.painter().galley(
        egui::pos2(left, rect.center().y - name.size().y / 2.0),
        name,
        appearance.foreground,
    );
    if !detail.is_empty() {
        let detail = label(detail, (available - name_width).max(0.0), appearance.muted);
        ui.painter().galley(
            egui::pos2(
                left + name_width + GAP,
                rect.center().y - detail.size().y / 2.0,
            ),
            detail,
            appearance.muted,
        );
    }
}

#[cfg(test)]
#[path = "symbol-outline-tests.rs"]
mod tests;

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};
use taide_model::error::{AppError, AppResult};
use taide_model::git::LogEntry;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::locale::ResolvedLocale;
use taide_model::theme::ResolvedTheme;

use crate::git_cache::Key;
use crate::navigation_icons::{Icon, PaintIcon};
use crate::presentation;

pub(crate) const PANEL_WIDTH: f32 = 384.0;
pub(crate) const HEADER_HEIGHT: f32 = 36.0;
pub(crate) const ENTRY_HEIGHT: f32 = 40.0;
const PANEL_RADIUS: u8 = 8;
const PADDING_X: f32 = 8.0;
const GAP: f32 = 6.0;
const BUTTON_SIZE: f32 = 20.0;
const BUTTON_RADIUS: u8 = 2;
const ICON_SIZE: f32 = 14.0;
const TEXT_SIZE: f32 = 12.0;
const META_SIZE: f32 = 10.0;
const ENTRY_LINE_GAP: f32 = 2.0;
const NOTICE_HEIGHT: f32 = 32.0;
const ELLIPSIS: char = '…';

/// One window's file history dialog (`file-history-panel.tsx`): the file, the commit whose diff
/// replaces the list, and the owner of that diff's documents.
pub(crate) struct Panel {
    pub(crate) project: ProjectId,
    pub(crate) path: String,
    pub(crate) selected: Option<LogEntry>,
    pub(crate) owner: TabId,
    pub(crate) pane: PaneId,
}

impl Panel {
    /// The selected commit's file diff (`CommitFileDiff` with `beforePath = path`).
    pub(crate) fn diff_key(&self) -> Option<Key> {
        self.selected.as_ref().map(|entry| Key::CommitDiff {
            rev: entry.id.clone(),
            parent: entry.parents.first().cloned(),
            path: self.path.clone(),
            before_path: self.path.clone(),
        })
    }
}

#[derive(Default)]
pub(crate) struct Panels {
    panels: HashMap<egui::ViewportId, Panel>,
}

impl Panels {
    /// Opens or retargets the window's dialog at `path` with the list showing.
    pub(crate) fn open(&mut self, window: egui::ViewportId, project: ProjectId, path: String) {
        self.panels.insert(
            window,
            Panel {
                project,
                path,
                selected: None,
                owner: TabId::new(),
                pane: PaneId::new(),
            },
        );
    }

    pub(crate) fn get(&self, window: egui::ViewportId) -> Option<&Panel> {
        self.panels.get(&window)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    pub(crate) fn apply(&mut self, window: egui::ViewportId, action: Action) {
        match action {
            Action::Close => {
                self.panels.remove(&window);
            }
            Action::Back => {
                if let Some(panel) = self.panels.get_mut(&window) {
                    panel.selected = None;
                }
            }
            Action::Select(entry) => {
                if let Some(panel) = self.panels.get_mut(&window) {
                    panel.selected = Some(entry);
                }
            }
        }
    }

    /// Closes dialogs whose window or project is gone.
    pub(crate) fn retain(
        &mut self,
        windows: &HashSet<egui::ViewportId>,
        projects: &HashSet<ProjectId>,
    ) {
        self.panels
            .retain(|window, panel| windows.contains(window) && projects.contains(&panel.project));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Action {
    Close,
    Back,
    Select(LogEntry),
}

/// The dialog's colors: `DialogContent` chrome, the header border, list hover and texts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Appearance {
    pub(crate) chrome: taide_native_ui::modal::Chrome,
    pub(crate) border: Color32,
    pub(crate) foreground: Color32,
    pub(crate) muted: Color32,
    pub(crate) hover: Color32,
    pub(crate) error: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| presentation::color(theme, key);
        Ok(Self {
            chrome: taide_native_ui::modal::Chrome {
                background: color("modal.background")?,
                border: color("modal.border")?,
                shadow: color("app.shadow")?,
            },
            border: color("app.border")?,
            foreground: color("app.foreground")?,
            muted: color("appSidebar.iconDefault")?,
            hover: color("explorer.itemHover")?,
            error: color("statusIndicator.error")?,
        })
    }
}

pub(crate) struct View<'a> {
    pub(crate) panel: &'a Panel,
    pub(crate) entries: Option<Result<&'a [LogEntry], &'a AppError>>,
    pub(crate) appearance: Appearance,
    pub(crate) locale: &'a ResolvedLocale,
    pub(crate) tooltips: &'a crate::tooltips::Provider,
    pub(crate) tooltip_appearance: &'a crate::tooltips::Appearance,
    pub(crate) now_unix: f64,
}

impl View<'_> {
    fn text(&self, key: &str) -> String {
        presentation::message(self.locale, key, &[])
    }
}

/// The dialog's frame: the right edge of the window at full height, rounded on its left side.
pub(crate) fn modal(id: egui::Id, context: &egui::Context, appearance: Appearance) -> egui::Modal {
    let content = context.content_rect();
    egui::Modal::new(id)
        .area(
            egui::Modal::default_area(id)
                .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::ZERO)
                .default_size(vec2(PANEL_WIDTH.min(content.width()), content.height())),
        )
        .backdrop_color(appearance.chrome.scrim(1.0))
        .frame(appearance.chrome.frame().corner_radius(CornerRadius {
            nw: PANEL_RADIUS,
            sw: PANEL_RADIUS,
            ne: 0,
            se: 0,
        }))
}

/// Draws the 36px header (back while a commit is selected, title, close) and returns the body
/// below it.
pub(crate) fn header(
    ui: &mut Ui,
    rect: Rect,
    view: &View<'_>,
    icons: &mut impl PaintIcon<Icon>,
) -> AppResult<(Rect, Option<Action>)> {
    let appearance = view.appearance;
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), HEADER_HEIGHT));
    ui.painter().hline(
        header.x_range(),
        header.bottom() - 0.5,
        Stroke::new(1.0, appearance.border),
    );
    let mut action = None;
    let mut left = header.left() + PADDING_X;
    let mut button = |ui: &mut Ui, rect: Rect, icon, label: String, id| -> AppResult<bool> {
        let response = ui.interact(
            rect,
            ui.id().with(("taide-file-history", id)),
            Sense::click(),
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
        view.tooltips.show(
            &response,
            &label,
            egui::RectAlign::BOTTOM,
            view.tooltip_appearance,
        );
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(BUTTON_RADIUS), appearance.hover);
        }
        icons.paint(
            ui,
            Rect::from_center_size(rect.center(), vec2(ICON_SIZE, ICON_SIZE)),
            icon,
            appearance.foreground,
            0.0,
        )?;
        Ok(response.clicked())
    };
    let button_rect = |left: f32| {
        Rect::from_min_size(
            pos2(left, header.center().y - BUTTON_SIZE / 2.0),
            vec2(BUTTON_SIZE, BUTTON_SIZE),
        )
    };
    if view.panel.selected.is_some() {
        if button(
            ui,
            button_rect(left),
            Icon::ArrowLeft,
            view.text("git.fileHistory"),
            "back",
        )? {
            action = Some(Action::Back);
        }
        left += BUTTON_SIZE + GAP;
    }
    let close = button_rect(header.right() - PADDING_X - BUTTON_SIZE);
    if button(ui, close, Icon::X, view.text("common.close"), "close")? {
        action = Some(Action::Close);
    }
    let title = match view.panel.selected {
        Some(_) => std::path::Path::new(&view.panel.path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&view.panel.path)
            .to_owned(),
        None => view.text("git.fileHistory"),
    };
    let galley = truncated(
        ui,
        title,
        FontId::new(TEXT_SIZE, taide_native_ui::font_families::medium(ui)),
        appearance.foreground,
        close.left() - GAP - left,
    );
    ui.painter().galley(
        pos2(left, header.center().y - galley.size().y / 2.0),
        galley,
        appearance.foreground,
    );
    Ok((
        Rect::from_min_max(pos2(rect.left(), header.bottom()), rect.max),
        action,
    ))
}

/// Draws the commit list (or its failure, loading and empty notices) inside `rect`.
pub(crate) fn list(ui: &mut Ui, rect: Rect, view: &View<'_>) -> Option<Action> {
    let appearance = view.appearance;
    let notice = match view.entries {
        Some(Err(_)) => Some(("editor.diffLoadFailed", appearance.error)),
        None => Some(("common.loading", appearance.muted)),
        Some(Ok([])) => Some(("git.noFileHistory", appearance.muted)),
        Some(Ok(_)) => None,
    };
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    body.set_clip_rect(rect.intersect(ui.clip_rect()));
    egui::ScrollArea::vertical()
        .id_salt(("taide-file-history-list", &view.panel.owner))
        .auto_shrink([false, false])
        .show(&mut body, |ui| {
            let mut action = None;
            if let Some((key, color)) = notice {
                let (row, _) = ui
                    .allocate_exact_size(vec2(ui.available_width(), NOTICE_HEIGHT), Sense::hover());
                ui.painter().text(
                    pos2(row.left() + PADDING_X, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    view.text(key),
                    FontId::proportional(TEXT_SIZE),
                    color,
                );
            }
            for entry in view.entries.and_then(Result::ok).unwrap_or_default() {
                let (row, response) = ui
                    .allocate_exact_size(vec2(ui.available_width(), ENTRY_HEIGHT), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &entry.summary)
                });
                if response.hovered() {
                    ui.painter().rect_filled(row, 0.0, appearance.hover);
                }
                let width = row.width() - PADDING_X * 2.0;
                let summary = truncated(
                    ui,
                    entry.summary.clone(),
                    FontId::proportional(TEXT_SIZE),
                    appearance.foreground,
                    width,
                );
                let (key, count) = crate::git_graph::relative_time(view.now_unix, entry.time_unix);
                let age = count.map_or_else(
                    || view.text(key),
                    |count| presentation::message(view.locale, key, &[("n", &count.to_string())]),
                );
                let fixed = [
                    ui.painter().layout_no_wrap(
                        age,
                        FontId::proportional(META_SIZE),
                        appearance.muted,
                    ),
                    ui.painter().layout_no_wrap(
                        crate::git_graph::short_hash(&entry.id).to_owned(),
                        FontId::monospace(META_SIZE),
                        appearance.muted,
                    ),
                ];
                let author = truncated(
                    ui,
                    entry.author.clone(),
                    FontId::proportional(META_SIZE),
                    appearance.muted,
                    width
                        - fixed
                            .iter()
                            .map(|galley| galley.size().x + GAP)
                            .sum::<f32>(),
                );
                let height = summary.size().y + ENTRY_LINE_GAP + author.size().y;
                let top = row.center().y - height / 2.0;
                let meta_top = top + summary.size().y + ENTRY_LINE_GAP;
                ui.painter().galley(
                    pos2(row.left() + PADDING_X, top),
                    summary,
                    appearance.foreground,
                );
                let mut left = row.left() + PADDING_X;
                for galley in std::iter::once(author).chain(fixed) {
                    let galley_width = galley.size().x;
                    ui.painter()
                        .galley(pos2(left, meta_top), galley, appearance.muted);
                    left += galley_width + GAP;
                }
                if response.clicked() {
                    action = Some(Action::Select(entry.clone()));
                }
            }
            action
        })
        .inner
}

fn truncated(
    ui: &Ui,
    text: String,
    font: FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, width.max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some(ELLIPSIS);
    ui.painter().layout_job(job)
}

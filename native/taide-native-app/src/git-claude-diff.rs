use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, CornerRadius, FontId, Rect, Sense, Stroke, Ui};
use taide_model::error::AppResult;
use taide_model::ids::{ProjectId, TabId};
use taide_model::theme::ResolvedTheme;
use taide_native_ui::diff_view::Hunk;

use crate::presentation;

pub(crate) const RECOMPUTE_DELAY: Duration = Duration::from_millis(200);
pub(crate) const FALLBACK_LANGUAGE_ID: &str = "plaintext";
pub(crate) const BAR_HEIGHT: f32 = 32.0;
const BAR_PADDING: f32 = 8.0;
const BUTTON_GAP: f32 = 8.0;
const BUTTON_HEIGHT: f32 = 24.0;
const BUTTON_PADDING: f32 = 8.0;
const BUTTON_RADIUS: u8 = 6;
const BUTTON_TEXT_SIZE: f32 = 12.0;
const PRIMARY_HOVER_OPACITY: f32 = 0.9;
const DISABLED_OPACITY: f32 = 0.5;

/// An IDE diff request or close-tab notice the shell's provider inbox recorded off the UI thread
/// (`ide-sync-provider.tsx` `ideDiffRequested`/`ideCloseTabRequested`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Requested {
        request_id: String,
        project: ProjectId,
        path: String,
        contents: String,
        title: String,
    },
    Closed(String),
}

/// The revisions of a Claude diff's original and modified documents.
pub(crate) type Revisions = (u64, u64);

#[derive(Default)]
struct Lines {
    hunks: Vec<Hunk>,
    computed: Option<Revisions>,
    wanted: Option<(Revisions, Instant)>,
    in_flight: Option<Revisions>,
}

struct Pending {
    contents: String,
    shown_in: Option<TabId>,
}

/// `claude-diff-registry.ts` plus the panes' resolving flags and line diffs: one registry for the
/// whole app, like the original's main-window provider.
#[derive(Default)]
pub(crate) struct State {
    pending: HashMap<String, Pending>,
    resolving: HashSet<String>,
    lines: HashMap<TabId, Lines>,
}

impl State {
    pub(crate) fn insert(&mut self, request_id: String, contents: String) {
        self.pending.insert(
            request_id,
            Pending {
                contents,
                shown_in: None,
            },
        );
    }

    pub(crate) fn remove(&mut self, request_id: &str) {
        self.pending.remove(request_id);
    }

    /// The requested text (`getPendingClaudeDiff(requestId)?.newContents ?? ''`).
    pub(crate) fn contents(&self, request_id: &str) -> &str {
        self.pending
            .get(request_id)
            .map_or("", |pending| pending.contents.as_str())
    }

    pub(crate) fn is_resolving(&self, request_id: &str) -> bool {
        self.resolving.contains(request_id)
    }

    /// Starts resolving; `false` when that request is already resolving (`if (isResolving) return`).
    pub(crate) fn begin_resolve(&mut self, request_id: &str) -> bool {
        self.resolving.insert(request_id.to_owned())
    }

    pub(crate) fn finish_resolve(&mut self, request_id: &str) {
        self.resolving.remove(request_id);
    }

    pub(crate) fn hunks(&self, tab: &TabId) -> &[Hunk] {
        self.lines
            .get(tab)
            .map_or(&[], |lines| lines.hunks.as_slice())
    }

    /// Records a drawn pane and schedules its line diff: the first one at once, later ones
    /// [`RECOMPUTE_DELAY`] after the last change (`diffEditorViewModel.js` `RunOnceScheduler`).
    pub(crate) fn shown(
        &mut self,
        request_id: &str,
        tab: TabId,
        revisions: Revisions,
        now: Instant,
    ) {
        if let Some(pending) = self.pending.get_mut(request_id) {
            pending.shown_in = Some(tab.clone());
        }
        let lines = self.lines.entry(tab).or_default();
        if lines.computed == Some(revisions)
            || lines.in_flight == Some(revisions)
            || lines.wanted.is_some_and(|(wanted, _)| wanted == revisions)
        {
            return;
        }
        let due = if lines.computed.is_none() && lines.in_flight.is_none() {
            now
        } else {
            now + RECOMPUTE_DELAY
        };
        lines.wanted = Some((revisions, due));
    }

    /// The line diffs due now, marked in flight.
    pub(crate) fn due(&mut self, now: Instant) -> Vec<(TabId, Revisions)> {
        let mut due = Vec::new();
        for (tab, lines) in &mut self.lines {
            if lines.in_flight.is_some() {
                continue;
            }
            if let Some((revisions, at)) = lines.wanted
                && at <= now
            {
                lines.wanted = None;
                lines.in_flight = Some(revisions);
                due.push((tab.clone(), revisions));
            }
        }
        due
    }

    pub(crate) fn next_due(&self, now: Instant) -> Option<Duration> {
        self.lines
            .values()
            .filter(|lines| lines.in_flight.is_none())
            .filter_map(|lines| {
                lines
                    .wanted
                    .map(|(_, at)| at.saturating_duration_since(now))
            })
            .min()
    }

    /// Drops a request that could not be sent so the next frame asks again.
    pub(crate) fn cancel(&mut self, tab: &TabId) {
        if let Some(lines) = self.lines.get_mut(tab) {
            lines.in_flight = None;
        }
    }

    /// Keeps an answer only while the documents still hold the texts it was computed for.
    pub(crate) fn finish(
        &mut self,
        tab: &TabId,
        revisions: Revisions,
        result: AppResult<Vec<Hunk>>,
        current: Option<Revisions>,
    ) {
        let Some(lines) = self.lines.get_mut(tab) else {
            return;
        };
        if lines.in_flight == Some(revisions) {
            lines.in_flight = None;
        }
        if current != Some(revisions) {
            return;
        }
        lines.computed = Some(revisions);
        if let Ok(hunks) = result {
            lines.hunks = hunks;
        }
    }

    pub(crate) fn reconnect(&mut self) {
        self.resolving.clear();
        for lines in self.lines.values_mut() {
            lines.in_flight = None;
        }
    }

    /// Ends a frame: forgets panes that did not draw, and requests whose drawn tab is gone from
    /// every window (`takePendingClaudeDiffIfUnresolved` on unmount).
    pub(crate) fn retain(&mut self, shown: &HashSet<TabId>, open: &HashSet<TabId>) {
        self.lines.retain(|tab, _| shown.contains(tab));
        self.pending.retain(|_, pending| {
            pending
                .shown_in
                .as_ref()
                .is_none_or(|tab| open.contains(tab))
        });
    }
}

/// The accept/reject bar's colors (`claude-diff-pane.tsx` bar and the `xs` outline/default
/// buttons of `button.tsx`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Appearance {
    background: Color32,
    border: Color32,
    outline_background: Color32,
    outline_foreground: Color32,
    hover_background: Color32,
    hover_foreground: Color32,
    primary_background: Color32,
    primary_foreground: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| presentation::color(theme, key);
        Ok(Self {
            background: color("editor.background")?,
            border: color("app.border")?,
            outline_background: color("app.background")?,
            outline_foreground: color("app.foreground")?,
            hover_background: color("list.hoverBackground")?,
            hover_foreground: color("list.foreground")?,
            primary_background: color("button.primaryBackground")?,
            primary_foreground: color("button.primaryForeground")?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Choice {
    Reject,
    Accept,
}

/// Draws the 32px bar above the diff with Reject (outline) then Accept (default) at its right end,
/// both disabled while resolving; returns the clicked one.
pub(crate) fn bar(
    ui: &mut Ui,
    rect: Rect,
    appearance: Appearance,
    labels: [&str; 2],
    is_resolving: bool,
) -> Option<Choice> {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, appearance.background);
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, appearance.border),
    );
    let font = FontId::proportional(BUTTON_TEXT_SIZE);
    let mut right = rect.right() - BAR_PADDING;
    let mut clicked = None;
    for (choice, label) in [Choice::Accept, Choice::Reject]
        .into_iter()
        .zip(labels.into_iter().rev())
    {
        let galley =
            ui.painter()
                .layout_no_wrap(label.to_owned(), font.clone(), Color32::PLACEHOLDER);
        let width = galley.size().x + BUTTON_PADDING * 2.0;
        let button = Rect::from_min_size(
            egui::pos2(right - width, rect.center().y - BUTTON_HEIGHT / 2.0),
            egui::vec2(width, BUTTON_HEIGHT),
        );
        right = button.left() - BUTTON_GAP;
        let response = ui
            .interact(
                button,
                ui.id().with(("taide-claude-diff-choice", label)),
                if is_resolving {
                    Sense::hover()
                } else {
                    Sense::click()
                },
            )
            .on_hover_cursor(egui::CursorIcon::Default);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, !is_resolving, label)
        });
        let is_hovered = response.hovered() && !is_resolving;
        let (fill, text, stroke) = match choice {
            Choice::Reject if is_hovered => (
                appearance.hover_background,
                appearance.hover_foreground,
                Stroke::new(1.0, appearance.border),
            ),
            Choice::Reject => (
                appearance.outline_background,
                appearance.outline_foreground,
                Stroke::new(1.0, appearance.border),
            ),
            Choice::Accept if is_hovered => (
                appearance
                    .primary_background
                    .gamma_multiply(PRIMARY_HOVER_OPACITY),
                appearance.primary_foreground,
                Stroke::NONE,
            ),
            Choice::Accept => (
                appearance.primary_background,
                appearance.primary_foreground,
                Stroke::NONE,
            ),
        };
        let opacity = if is_resolving { DISABLED_OPACITY } else { 1.0 };
        painter.rect(
            button,
            CornerRadius::same(BUTTON_RADIUS),
            fill.gamma_multiply(opacity),
            Stroke::new(stroke.width, stroke.color.gamma_multiply(opacity)),
            egui::StrokeKind::Inside,
        );
        painter.galley(
            button.center() - galley.size() / 2.0,
            galley,
            text.gamma_multiply(opacity),
        );
        if response.clicked() {
            clicked = Some(choice);
        }
    }
    clicked
}

#[cfg(test)]
#[path = "git-claude-diff-tests.rs"]
mod tests;

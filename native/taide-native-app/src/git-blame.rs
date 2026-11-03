use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, FontId, Rect, Stroke, Ui};
use taide_model::error::AppError;
use taide_model::git::BlameLine;
use taide_native_editor::document::DocumentSnapshot;
use taide_native_ui::editor_geometry::EditorGeometry;

/// How long the cursor line rests before its blame is read (`BLAME_DEBOUNCE_MS`).
pub(crate) const DEBOUNCE: Duration = Duration::from_millis(300);
/// The footer bar under a file editor (`blame-footer-bar.tsx` `h-6`).
pub(crate) const FOOTER_HEIGHT: f32 = 24.0;
const FOOTER_FONT_SIZE: f32 = 11.0;
const FOOTER_PADDING_X: f32 = 8.0;
const FOOTER_BORDER_WIDTH: f32 = 1.0;
const MESSAGE_MAX_UTF16_UNITS: usize = 50;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;
const SECONDS_PER_MONTH: u64 = 30 * SECONDS_PER_DAY;
const SECONDS_PER_YEAR: u64 = 365 * SECONDS_PER_DAY;
const UNCOMMITTED: &str = "You, now - Uncommitted changes";
const CURRENT_USER: &str = "You";
const UNKNOWN_TIME: &str = "unknown time";
const JUST_NOW: &str = "just now";
const TRUNCATED: &str = "...";
const ELLIPSIS: char = '\u{2026}';
const OVERLAY_PREFIX: &str = "    ";
const OVERLAY_OPACITY: f32 = 0.8;

/// `blame-format.ts` `formatRelativeTime`: whole elapsed units, future times as now.
pub(crate) fn relative_time(time_unix: Option<f64>, now_unix: f64) -> String {
    let Some(time_unix) = time_unix else {
        return UNKNOWN_TIME.into();
    };
    let elapsed = (now_unix - time_unix).floor().max(0.0) as u64;
    let ago = |count: u64, unit: &str| {
        let plural = if count == 1 { "" } else { "s" };
        format!("{count} {unit}{plural} ago")
    };
    match elapsed {
        _ if elapsed < SECONDS_PER_MINUTE => JUST_NOW.into(),
        _ if elapsed < SECONDS_PER_HOUR => ago(elapsed / SECONDS_PER_MINUTE, "minute"),
        _ if elapsed < SECONDS_PER_DAY => ago(elapsed / SECONDS_PER_HOUR, "hour"),
        _ if elapsed < SECONDS_PER_MONTH => ago(elapsed / SECONDS_PER_DAY, "day"),
        _ if elapsed < SECONDS_PER_YEAR => ago(elapsed / SECONDS_PER_MONTH, "month"),
        _ => ago(elapsed / SECONDS_PER_YEAR, "year"),
    }
}

/// `truncateBlameMessage`: whitespace runs fold to one space, and a summary over 50 UTF-16 units
/// (JavaScript `length`) keeps the characters within its first 50 (trailing space trimmed)
/// followed by `...`.
pub(crate) fn truncate_message(message: &str) -> String {
    let single_line = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.encode_utf16().count() <= MESSAGE_MAX_UTF16_UNITS {
        return single_line;
    }
    let kept = single_line
        .chars()
        .scan(0, |units, character| {
            *units += character.len_utf16();
            (*units <= MESSAGE_MAX_UTF16_UNITS).then_some(character)
        })
        .collect::<String>();
    format!("{}{TRUNCATED}", kept.trim_end())
}

/// `formatBlameLine`: the fixed uncommitted text, or `author, ago • summary` with the current user
/// shown as `You`.
pub(crate) fn format_line(blame: &BlameLine, now_unix: f64, current_user: Option<&str>) -> String {
    if blame.is_uncommitted {
        return UNCOMMITTED.into();
    }
    let author = if current_user.is_some_and(|user| !user.is_empty() && user == blame.author) {
        CURRENT_USER
    } else {
        &blame.author
    };
    format!(
        "{author}, {} \u{2022} {}",
        relative_time(Some(blame.time_unix), now_unix),
        truncate_message(&blame.summary)
    )
}

pub(crate) fn now_unix() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |elapsed| elapsed.as_secs_f64())
}

/// One file editor's blame (`use-editor-blame.ts`): the cursor line rests for [`DEBOUNCE`] before
/// its blame is read, the footer keeps its last answer until the rested line answers, and the
/// overlay remembers the line count it was turned on with.
#[derive(Default)]
pub(crate) struct Blame {
    cursor: Option<(u32, Instant)>,
    settled: Option<u32>,
    footer: Option<BlameLine>,
    adopted: Option<Instant>,
    formatted_at: f64,
    overlay: Option<u32>,
}

impl Blame {
    /// Records the cursor line and settles it once it rested; answers how long until it settles.
    pub(crate) fn cursor(&mut self, line: u32, now: Instant) -> Option<Duration> {
        if self.cursor.is_none_or(|(current, _)| current != line) {
            self.cursor = Some((line, now));
        }
        let (line, moved) = self.cursor?;
        if self.settled == Some(line) {
            return None;
        }
        let rested = now.saturating_duration_since(moved);
        if rested >= DEBOUNCE {
            self.settled = Some(line);
            return None;
        }
        Some(DEBOUNCE - rested)
    }

    pub(crate) fn settled(&self) -> Option<u32> {
        self.settled
    }

    /// Takes the rested line's newest answer: its first line, or nothing on failure.
    pub(crate) fn adopt(
        &mut self,
        answer: Option<Result<&[BlameLine], &AppError>>,
        fetched_at: Option<Instant>,
    ) {
        let Some(answer) = answer else {
            return;
        };
        if fetched_at == self.adopted {
            return;
        }
        self.adopted = fetched_at;
        self.footer = answer.ok().and_then(|lines| lines.first().cloned());
        self.formatted_at = now_unix();
    }

    /// The footer text: empty without an answer or when the line is past the document end.
    pub(crate) fn footer_text(&self, line_count: u32, current_user: Option<&str>) -> String {
        match &self.footer {
            Some(line) if line.line <= line_count => {
                format_line(line, self.formatted_at, current_user)
            }
            _ => String::new(),
        }
    }

    pub(crate) fn overlay(&self) -> Option<u32> {
        self.overlay
    }

    /// `git.toggleBlame`: turns the overlay on for the current line count, or off.
    pub(crate) fn toggle_overlay(&mut self, line_count: u32) {
        self.overlay = match self.overlay {
            Some(_) => None,
            None => Some(line_count),
        };
    }
}

/// Splits a file editor's area into the editor and the footer bar under it.
pub(crate) fn split_footer(rect: Rect, has_footer: bool) -> (Rect, Option<Rect>) {
    if !has_footer {
        return (rect, None);
    }
    let top = (rect.bottom() - FOOTER_HEIGHT).max(rect.top());
    (
        Rect::from_min_max(rect.min, egui::pos2(rect.right(), top)),
        Some(Rect::from_min_max(egui::pos2(rect.left(), top), rect.max)),
    )
}

/// `BlameFooterBar`: the blame background, a top border and one truncated 11px line.
pub(crate) fn paint_footer(
    ui: &Ui,
    rect: Rect,
    text: String,
    (foreground, background, border): (Color32, Color32, Color32),
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, background);
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(FOOTER_BORDER_WIDTH, border),
    );
    let mut job = egui::text::LayoutJob::simple(
        text,
        FontId::proportional(FOOTER_FONT_SIZE),
        foreground,
        (rect.width() - FOOTER_PADDING_X * 2.0).max(0.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some(ELLIPSIS);
    let galley = painter.layout_job(job);
    let top = rect.center().y - galley.size().y / 2.0;
    painter.galley(
        egui::pos2(rect.left() + FOOTER_PADDING_X, top),
        galley,
        foreground,
    );
}

/// The overlay's italic `    author, ago • summary` after each blamed line that is still in the
/// document, painted over the rendered rows without moving text or the cursor.
pub(crate) fn paint_overlay(
    ui: &Ui,
    geometry: &EditorGeometry,
    snapshot: &DocumentSnapshot,
    lines: &[BlameLine],
    (current_user, now_unix): (Option<&str>, f64),
    (font, color): (FontId, Color32),
) {
    let painter = ui.painter_at(geometry.content_rect);
    let line_count = snapshot.rope.len_lines();
    let color = color.gamma_multiply(OVERLAY_OPACITY);
    for blame in lines {
        let Some(index) = (blame.line as usize).checked_sub(1) else {
            continue;
        };
        if index >= line_count {
            continue;
        }
        let start = snapshot.rope.line_to_byte(index);
        let mut end = if index + 1 < line_count {
            snapshot.rope.line_to_byte(index + 1)
        } else {
            snapshot.rope.len_bytes()
        };
        while end > start && matches!(snapshot.rope.byte(end - 1), b'\n' | b'\r') {
            end -= 1;
        }
        let Some(caret) = geometry.caret_rect(end) else {
            continue;
        };
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &format!(
                "{OVERLAY_PREFIX}{}",
                format_line(blame, now_unix, current_user)
            ),
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color,
                italics: true,
                ..Default::default()
            },
        );
        let galley = painter.layout_job(job);
        let top = caret.center().y - galley.size().y / 2.0;
        painter.galley(egui::pos2(caret.left(), top), galley, color);
    }
}

#[cfg(test)]
#[path = "git-blame-tests.rs"]
mod tests;

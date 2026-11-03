use eframe::egui::{self, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};
use taide_model::error::{AppError, AppResult};
use taide_model::git::{CommitFile, GitChangeKind, LogEntry};
use taide_model::layout::TabKind;
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::intents::git::Intent;

use crate::git_graph::short_hash;
use crate::git_scm::View;
use crate::navigation_icons::{Icon, PaintIcon};

const MARGIN_TOP: f32 = 4.0;
const BORDER_WIDTH: f32 = 1.0;
const PADDING_TOP: f32 = 4.0;
const PADDING_X: f32 = 8.0;
const GAP: f32 = 6.0;
const TITLE_PADDING_Y: f32 = 4.0;
const CLOSE_SIZE: f32 = 20.0;
const CLOSE_ICON_SIZE: f32 = 14.0;
const CLOSE_RADIUS: u8 = 2;
const TITLE_HEIGHT: f32 = 2.0 * TITLE_PADDING_Y + CLOSE_SIZE;
const ROW_HEIGHT: f32 = 24.0;
const NOTICE_HEIGHT: f32 = 28.0;
const TEXT_SIZE: f32 = 12.0;
const HASH_TEXT_SIZE: f32 = 10.0;
const HEADER_TEXT_SIZE: f32 = 11.0;
const LETTER_WIDTH: f32 = 12.0;
const BADGE_HEIGHT: f32 = 16.0;
const BADGE_PADDING_X: f32 = 4.0;
const BADGE_TEXT_SIZE: f32 = 10.0;
const ELLIPSIS: char = '…';
const DETAIL_ID: &str = "git-commit-detail";

/// The detail panel's natural height: top margin and border, title row, files header, the loading
/// or failure line, then one row per file.
pub(crate) fn height(files: Option<Result<&[CommitFile], &AppError>>) -> f32 {
    let (notice, rows) = match files {
        None | Some(Err(_)) => (NOTICE_HEIGHT, 0),
        Some(Ok(files)) => (0.0, files.len()),
    };
    MARGIN_TOP
        + BORDER_WIDTH
        + PADDING_TOP
        + TITLE_HEIGHT
        + ROW_HEIGHT
        + notice
        + rows as f32 * ROW_HEIGHT
}

/// `buildCommitFileDiffOpenTabInput`: the commit diff tab of one changed file, absolute paths, a
/// rename compared against its old path, titled `"{file name} @ {short hash}"`.
pub(crate) fn file_tab(commit: &LogEntry, file: &CommitFile) -> (TabKind, String) {
    let before_path = match file.kind {
        GitChangeKind::Renamed => file
            .orig_abs_path
            .clone()
            .unwrap_or_else(|| file.abs_path.clone()),
        _ => file.abs_path.clone(),
    };
    let name = file.path.rsplit('/').next().unwrap_or(&file.path);
    (
        TabKind::Diff {
            path: file.abs_path.clone(),
            staged: false,
            compare_with: None,
            rev: Some(commit.id.clone()),
            parent_rev: commit.parents.first().cloned(),
            before_path: Some(before_path),
        },
        format!("{name} @ {}", short_hash(&commit.id)),
    )
}

fn truncated(
    ui: &Ui,
    text: String,
    font: FontId,
    color: egui::Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, width.max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some(ELLIPSIS);
    ui.painter().layout_job(job)
}

/// `CommitDetailPanel`: summary, short hash and close button, the changed files header with its
/// count, then the loading or failure line or the files with their status letters. Scrolls on
/// its own once taller than `rect`.
pub(crate) fn show(
    ui: &mut Ui,
    rect: Rect,
    view: &View<'_>,
    commit: &LogEntry,
    icons: &mut impl PaintIcon<Icon>,
    intents: &mut Vec<ShellIntent>,
) -> AppResult<()> {
    let appearance = view.appearance;
    let files = view.graph.commit_files;
    let id = view.mounted_id(DETAIL_ID).with(&commit.id);
    let mut panel = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt(id.with("scroll"))
        .auto_shrink([false, false])
        .show(&mut panel, |ui| -> AppResult<()> {
            let (content, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height(files)), Sense::hover());
            let x = content.x_range();
            let border = content.top() + MARGIN_TOP;
            let surface = Rect::from_x_y_ranges(x, border..=content.bottom());
            ui.painter().rect_filled(surface, 0.0, appearance.panel);
            ui.painter().hline(
                x,
                border + BORDER_WIDTH / 2.0,
                Stroke::new(BORDER_WIDTH, appearance.border),
            );
            let title = Rect::from_x_y_ranges(
                x,
                border + BORDER_WIDTH + PADDING_TOP
                    ..=border + BORDER_WIDTH + PADDING_TOP + TITLE_HEIGHT,
            );
            let close = Rect::from_center_size(
                pos2(
                    title.right() - PADDING_X - CLOSE_SIZE / 2.0,
                    title.center().y,
                ),
                vec2(CLOSE_SIZE, CLOSE_SIZE),
            );
            let hash = ui.painter().layout_no_wrap(
                short_hash(&commit.id).to_owned(),
                FontId::monospace(HASH_TEXT_SIZE),
                appearance.muted,
            );
            let left = title.left() + PADDING_X;
            let summary = truncated(
                ui,
                commit.summary.clone(),
                FontId::new(TEXT_SIZE, taide_native_ui::font_families::medium(ui)),
                appearance.foreground,
                close.left() - GAP - hash.size().x - GAP - left,
            );
            let summary_width = summary.size().x;
            ui.painter().galley(
                pos2(left, title.center().y - summary.size().y / 2.0),
                summary,
                appearance.foreground,
            );
            ui.painter().galley(
                pos2(
                    left + summary_width + GAP,
                    title.center().y - hash.size().y / 2.0,
                ),
                hash,
                appearance.muted,
            );
            let label = view.text("common.close");
            let closed = ui.interact(close, id.with("close"), Sense::click());
            closed
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            view.tooltips.show(
                &closed,
                &label,
                egui::RectAlign::BOTTOM,
                view.tooltip_appearance,
            );
            if closed.hovered() {
                ui.painter()
                    .rect_filled(close, CLOSE_RADIUS, appearance.hover);
            }
            icons.paint(
                ui,
                Rect::from_center_size(close.center(), vec2(CLOSE_ICON_SIZE, CLOSE_ICON_SIZE)),
                Icon::X,
                appearance.foreground,
                0.0,
            )?;
            if closed.clicked() {
                intents.push(ShellIntent::Git(Intent::SelectCommit {
                    project: view.project.clone(),
                    slot: view.slot.clone(),
                    commit: None,
                }));
            }
            let header = Rect::from_x_y_ranges(x, title.bottom()..=title.bottom() + ROW_HEIGHT);
            let count = files.and_then(Result::ok).map_or(0, <[CommitFile]>::len);
            let badge_text = ui.painter().layout_no_wrap(
                count.to_string(),
                FontId::proportional(BADGE_TEXT_SIZE),
                appearance.header,
            );
            let badge_width = (badge_text.size().x + 2.0 * BADGE_PADDING_X).max(BADGE_HEIGHT);
            let heading = truncated(
                ui,
                view.text("git.changedFiles").to_uppercase(),
                FontId::new(
                    HEADER_TEXT_SIZE,
                    taide_native_ui::font_families::semibold(ui),
                ),
                appearance.header,
                header.right() - PADDING_X - badge_width - GAP - (header.left() + PADDING_X),
            );
            let heading_width = heading.size().x;
            ui.painter().galley(
                pos2(
                    header.left() + PADDING_X,
                    header.center().y - heading.size().y / 2.0,
                ),
                heading,
                appearance.header,
            );
            let badge = Rect::from_min_size(
                pos2(
                    header.left() + PADDING_X + heading_width + GAP,
                    header.center().y - BADGE_HEIGHT / 2.0,
                ),
                vec2(badge_width, BADGE_HEIGHT),
            );
            ui.painter()
                .rect_filled(badge, BADGE_HEIGHT / 2.0, appearance.badge);
            ui.painter().galley(
                badge.center() - badge_text.size() / 2.0,
                badge_text,
                appearance.header,
            );
            let mut top = header.bottom();
            let notice = match files {
                Some(Err(_)) => Some((view.text("editor.diffLoadFailed"), appearance.error)),
                None => Some((view.text("common.loading"), appearance.muted)),
                Some(Ok(_)) => None,
            };
            if let Some((text, color)) = notice {
                ui.painter().text(
                    pos2(x.min + PADDING_X, top + NOTICE_HEIGHT / 2.0),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(TEXT_SIZE),
                    color,
                );
                top += NOTICE_HEIGHT;
            }
            for file in files.and_then(Result::ok).unwrap_or_default() {
                let row = Rect::from_x_y_ranges(x, top..=top + ROW_HEIGHT);
                let response = ui.interact(row, id.with(("file", &file.path)), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &file.path)
                });
                if response.hovered() {
                    ui.painter().rect_filled(row, 0.0, appearance.hover);
                }
                let right = row.right() - PADDING_X;
                ui.painter().text(
                    pos2(right - LETTER_WIDTH / 2.0, row.center().y),
                    egui::Align2::CENTER_CENTER,
                    crate::git_scm_model::status_letter(file.kind),
                    FontId::new(TEXT_SIZE, taide_native_ui::font_families::semibold(ui)),
                    appearance.status(file.kind),
                );
                let path = truncated(
                    ui,
                    file.path.clone(),
                    FontId::proportional(TEXT_SIZE),
                    appearance.foreground,
                    right - LETTER_WIDTH - GAP - (row.left() + PADDING_X),
                );
                ui.painter().galley(
                    pos2(row.left() + PADDING_X, row.center().y - path.size().y / 2.0),
                    path,
                    appearance.foreground,
                );
                if response.clicked() {
                    let (kind, title) = file_tab(commit, file);
                    intents.push(ShellIntent::Git(Intent::OpenCommitFile {
                        project: view.project.clone(),
                        kind,
                        title,
                    }));
                }
                top += ROW_HEIGHT;
            }
            Ok(())
        })
        .inner
}

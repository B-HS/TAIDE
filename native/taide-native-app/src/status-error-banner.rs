use std::sync::Arc;

use eframe::egui::{self, Color32, FontId, Id, Rect, Sense, Stroke, Vec2, pos2, vec2};
use taide_model::{error::AppResult, locale::ResolvedLocale, theme::ResolvedTheme};

use crate::problems_icons::{Glyph, Icons};

/// `STATUS_ERROR_BANNER_HEIGHT_PX` (`status-error-banner.tsx` 9): how far below the locale banner
/// the theme banner sits while both show.
pub(crate) const STACK_OFFSET: f32 = 32.0;
/// `LOCALE_LOAD_ERROR_FALLBACK_MESSAGE` (`locale-provider.tsx` 8): the language pack is what failed,
/// so this message cannot come from it.
pub(crate) const LOCALE_MESSAGE: &str =
    "Failed to load the app language pack. Some labels may show as raw keys.";
const THEME_MESSAGE: &str = "theme.loadFailed";
const RETRY: &str = "common.retry";
const ERROR_COLOR: &str = "statusIndicator.error";
const PADDING_X: f32 = 12.0;
const PADDING_Y: f32 = 6.0;
const GAP: f32 = 8.0;
const ICON_SIZE: f32 = 14.0;
const FONT_SIZE: f32 = 12.0;
const LINE_HEIGHT: f32 = 16.0;
const BACKGROUND_OPACITY: f32 = 0.15;
const UNDERLINE_WIDTH: f32 = 1.0;
const LAYER: &str = "native-status-error-banner";

/// Whether the last theme and locale loads failed (`useQuery(...).isError` of
/// `theme-provider.tsx` 23 and `locale-provider.tsx` 11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Failures {
    pub(crate) theme: bool,
    pub(crate) locale: bool,
}

/// Draws the locale banner at the top and the theme banner under it (`locale-provider.tsx` 27-29,
/// `theme-provider.tsx` 102-109); answers whether a banner's retry was pressed.
pub(crate) fn show(
    context: &egui::Context,
    failures: Failures,
    theme: &ResolvedTheme,
    locale: &ResolvedLocale,
    icons: &mut Icons,
    enabled: bool,
) -> AppResult<bool> {
    if failures == Failures::default() {
        return Ok(false);
    }
    let color = crate::presentation::color(theme, ERROR_COLOR)?;
    let retry = crate::presentation::message(locale, RETRY, &[]);
    let banners = [
        (failures.locale, "locale", LOCALE_MESSAGE.to_owned()),
        (
            failures.theme,
            "theme",
            crate::presentation::message(locale, THEME_MESSAGE, &[]),
        ),
    ];
    let content = context.content_rect();
    let mut offset = 0.0;
    let mut retried = false;
    for (kind, message) in banners
        .into_iter()
        .filter_map(|(failed, kind, message)| failed.then_some((kind, message)))
    {
        let shown = egui::Area::new(Id::new((LAYER, kind)))
            .order(egui::Order::Middle)
            .fixed_pos(pos2(content.left(), content.top() + offset))
            .constrain(false)
            .fade_in(false)
            .show(context, |ui| {
                if !enabled {
                    ui.disable();
                    ui.set_opacity(1.0);
                }
                banner(ui, content.width(), &message, &retry, color, icons)
            });
        retried |= shown.inner?;
        offset += STACK_OFFSET;
    }
    Ok(retried)
}

fn text(
    ui: &egui::Ui,
    text: &str,
    color: Color32,
    underline: Stroke,
    wrap: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(
        text.to_owned(),
        egui::TextFormat {
            font_id: FontId::proportional(FONT_SIZE),
            line_height: Some(LINE_HEIGHT),
            color,
            underline,
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap;
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn banner(
    ui: &mut egui::Ui,
    width: f32,
    message: &str,
    retry: &str,
    color: Color32,
    icons: &mut Icons,
) -> AppResult<bool> {
    let retry_text = text(
        ui,
        retry,
        color,
        Stroke::new(UNDERLINE_WIDTH, color),
        f32::INFINITY,
    );
    let message_width =
        (width - 2.0 * PADDING_X - ICON_SIZE - 2.0 * GAP - retry_text.size().x).max(0.0);
    let message_text = text(ui, message, color, Stroke::NONE, message_width);
    let height = 2.0 * PADDING_Y + message_text.size().y.max(LINE_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, color.gamma_multiply(BACKGROUND_OPACITY));
    let icon = Rect::from_center_size(
        pos2(rect.left() + PADDING_X + ICON_SIZE / 2.0, rect.center().y),
        Vec2::splat(ICON_SIZE),
    );
    icons.paint(ui, icon, Glyph::TriangleAlert, color, 0.0)?;
    let message_rect = Rect::from_min_size(
        pos2(
            icon.right() + GAP,
            rect.center().y - message_text.size().y / 2.0,
        ),
        message_text.size(),
    );
    ui.put(
        message_rect,
        egui::Label::new(message_text).selectable(false),
    );
    let retry_rect = Rect::from_min_size(
        pos2(
            rect.right() - PADDING_X - retry_text.size().x,
            rect.center().y - retry_text.size().y / 2.0,
        ),
        retry_text.size(),
    );
    let response = ui.interact(retry_rect, ui.id().with("retry"), Sense::click());
    let is_enabled = ui.is_enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, is_enabled, retry));
    ui.painter().galley(retry_rect.min, retry_text, color);
    Ok(response.clicked())
}

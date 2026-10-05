use egui::{Color32, FontId};
use taide_model::error::{AppError, AppResult};
use taide_model::locale::ResolvedLocale;
use taide_model::settings::Settings;
use taide_model::theme::ResolvedTheme;

use crate::conflict_banner::BannerAppearance;
use crate::editor_surface::EditorAppearance;
use crate::shell::ShellColors;

const HEX_RGB: usize = 6;
const HEX_RGBA: usize = 8;
const HEX_SHORT: usize = 3;
const HEX_SHORT_CHANNEL_FACTOR: u8 = 17;
const HEX_CHANNEL: usize = 2;
pub const EDITOR_LINE_HEIGHT_FACTOR: f32 = 1.5;
pub const EDITOR_PADDING: f32 = 8.0;
pub const MIN_CODE_FONT_SIZE: u32 = 6;
pub const MAX_CODE_FONT_SIZE: u32 = 48;
const CODE_FONT_SIZE_STEP: u32 = 1;

pub fn next_editor_font_size(current: u32, increase: bool) -> u32 {
    let next = if increase {
        current.saturating_add(CODE_FONT_SIZE_STEP)
    } else {
        current.saturating_sub(CODE_FONT_SIZE_STEP)
    };
    next.clamp(MIN_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE)
}

pub fn update_editor_font_size(appearance: &mut EditorAppearance, size: u32) -> bool {
    let size = size as f32;
    let line_height = size * EDITOR_LINE_HEIGHT_FACTOR;
    if appearance.font.size == size && appearance.line_height == line_height {
        return false;
    }
    appearance.font.size = size;
    appearance.line_height = line_height;
    true
}

pub fn color(theme: &ResolvedTheme, key: &str) -> AppResult<Color32> {
    let value = theme
        .colors
        .get(key)
        .ok_or_else(|| AppError::Internal(format!("native theme color is unavailable: {key}")))?;
    parse_color(value, key)
}

pub fn parse_color(value: &str, key: &str) -> AppResult<Color32> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("transparent") {
        return Ok(Color32::TRANSPARENT);
    }
    let value = value.strip_prefix('#').ok_or_else(|| {
        AppError::Internal(format!("native theme color format is unsupported: {key}"))
    })?;
    if !value.is_ascii() || !matches!(value.len(), HEX_SHORT | HEX_RGB | HEX_RGBA) {
        return Err(AppError::Internal(format!(
            "native theme color format is unsupported: {key}"
        )));
    }
    let mut channels = [u8::MAX; 4];
    if value.len() == HEX_SHORT {
        for (channel, digit) in channels.iter_mut().zip(value.as_bytes()) {
            *channel = char::from(*digit)
                .to_digit(16)
                .and_then(|digit| u8::try_from(digit).ok())
                .map(|digit| digit * HEX_SHORT_CHANNEL_FACTOR)
                .ok_or_else(|| AppError::Internal(format!("invalid native theme color: {key}")))?;
        }
        return Ok(Color32::from_rgb(channels[0], channels[1], channels[2]));
    }
    for (channel, chunk) in channels
        .iter_mut()
        .zip(value.as_bytes().as_chunks::<HEX_CHANNEL>().0)
    {
        let hex = std::str::from_utf8(chunk)
            .map_err(|_| AppError::Internal("invalid native color encoding".into()))?;
        *channel = u8::from_str_radix(hex, 16)
            .map_err(|_| AppError::Internal(format!("invalid native theme color: {key}")))?;
    }
    Ok(Color32::from_rgba_unmultiplied(
        channels[0],
        channels[1],
        channels[2],
        channels[3],
    ))
}

pub fn shell_colors(theme: &ResolvedTheme) -> AppResult<ShellColors> {
    Ok(ShellColors {
        background: color(theme, "app.background")?,
        foreground: color(theme, "app.foreground")?,
        sidebar: color(theme, "appSidebar.background")?,
        muted: color(theme, "appSidebar.iconDefault")?,
        border: color(theme, "app.border")?,
        focus_border: color(theme, "app.focusBorder")?,
        active_tab: color(theme, "tabBar.tabActiveBackground")?,
        inactive_tab: color(theme, "tabBar.tabInactiveBackground")?,
        active_indicator: color(theme, "tabBar.tabActiveIndicator")?,
    })
}

pub fn editor_appearance(
    theme: &ResolvedTheme,
    settings: &Settings,
) -> AppResult<EditorAppearance> {
    let font_size = settings.editor_font_size as f32;
    Ok(EditorAppearance {
        font: FontId::monospace(font_size),
        line_height: font_size * EDITOR_LINE_HEIGHT_FACTOR,
        horizontal_padding: EDITOR_PADDING,
        background: color(theme, "editor.background")?,
        foreground: color(theme, "editor.foreground")?,
        muted: color(theme, "editor.lineNumber")?,
        selection: color(theme, "editor.selection")?,
        cursor: color(theme, "editor.cursor")?,
        current_line: color(theme, "editor.lineHighlight")?,
        line_numbers: settings.editor_line_numbers,
        indent: if settings.editor_insert_spaces {
            " ".repeat(settings.editor_tab_size as usize)
        } else {
            "\t".into()
        },
    })
}

pub fn banner_appearance(theme: &ResolvedTheme) -> AppResult<BannerAppearance> {
    Ok(BannerAppearance {
        error: color(theme, "statusIndicator.error")?,
        warning: color(theme, "statusIndicator.warning")?,
    })
}

pub fn message(locale: &ResolvedLocale, key: &str, arguments: &[(&str, &str)]) -> String {
    let mut text = locale
        .messages
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.into());
    for (name, value) in arguments {
        text = text.replace(&format!("{{{{{name}}}}}"), value);
    }
    text
}

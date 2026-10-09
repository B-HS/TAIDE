use egui::epaint::Shadow;
use egui::style::Selection;
use egui::{Color32, CornerRadius, FontId, Stroke, Visuals};
use taide_model::error::{AppError, AppResult};
use taide_model::file::FileSizeTier;
use taide_model::locale::ResolvedLocale;
use taide_model::settings::Settings;
use taide_model::theme::{ResolvedTheme, ThemeType};

use crate::conflict_banner::BannerAppearance;
use crate::editor_surface::{EditorAppearance, EditorDisplayOptions, EditorPresentation};
use crate::shell::ShellColors;

const HEX_RGB: usize = 6;
const HEX_RGBA: usize = 8;
const HEX_SHORT: usize = 3;
const HEX_SHORT_CHANNEL_FACTOR: u8 = 17;
const HEX_CHANNEL: usize = 2;
pub const EDITOR_LINE_HEIGHT_FACTOR: f32 = 1.5;
const EDITOR_MIN_LINE_HEIGHT: f32 = 8.0;
pub const EDITOR_PADDING: f32 = 8.0;
pub const MIN_CODE_FONT_SIZE: u32 = 6;
pub const MAX_CODE_FONT_SIZE: u32 = 48;
const CODE_FONT_SIZE_STEP: u32 = 1;
const BORDER_WIDTH: f32 = 1.0;
const WIDGET_CORNER_RADIUS: u8 = 6;
const DIALOG_CORNER_RADIUS: u8 = 8;
const OVERLAY_SHADOW_OFFSET: [i8; 2] = [0, 2];
const OVERLAY_SHADOW_BLUR: u8 = 8;
const DIALOG_SHADOW_OFFSET: [i8; 2] = [0, 8];
const DIALOG_SHADOW_BLUR: u8 = 24;

pub fn next_editor_font_size(current: u32, increase: bool) -> u32 {
    let next = if increase {
        current.saturating_add(CODE_FONT_SIZE_STEP)
    } else {
        current.saturating_sub(CODE_FONT_SIZE_STEP)
    };
    next.clamp(MIN_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE)
}

fn editor_line_height(font_size: f32) -> f32 {
    (font_size * EDITOR_LINE_HEIGHT_FACTOR)
        .round()
        .max(EDITOR_MIN_LINE_HEIGHT)
}

pub fn update_editor_font_size(appearance: &mut EditorAppearance, size: u32) -> bool {
    let size = size as f32;
    let line_height = editor_line_height(size);
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
        editor_background: color(theme, "editor.background")?,
        editor_foreground: color(theme, "editor.foreground")?,
    })
}

pub fn visuals(theme: &ResolvedTheme) -> AppResult<Visuals> {
    let mut visuals = match theme.theme_type {
        ThemeType::Dark => Visuals::dark(),
        ThemeType::Light => Visuals::light(),
    };
    let foreground = color(theme, "app.foreground")?;
    let background = color(theme, "app.background")?;
    let button_foreground = color(theme, "button.foreground")?;
    let button_hover = color(theme, "button.hoverBackground")?;
    let input_background = color(theme, "panel.inputBackground")?;
    let input_border = Stroke::new(BORDER_WIDTH, color(theme, "panel.inputBorder")?);
    let shadow = color(theme, "app.shadow")?;
    let corner_radius = CornerRadius::same(WIDGET_CORNER_RADIUS);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = background;
    widgets.noninteractive.weak_bg_fill = background;
    widgets.noninteractive.bg_stroke = Stroke::new(BORDER_WIDTH, color(theme, "app.border")?);
    widgets.noninteractive.fg_stroke.color = foreground;
    widgets.noninteractive.corner_radius = corner_radius;
    for (state, fill, stroke, text) in [
        (
            &mut widgets.inactive,
            color(theme, "button.background")?,
            input_border,
            button_foreground,
        ),
        (
            &mut widgets.hovered,
            button_hover,
            input_border,
            button_foreground,
        ),
        (
            &mut widgets.active,
            button_hover,
            Stroke::new(BORDER_WIDTH, color(theme, "app.focusBorder")?),
            button_foreground,
        ),
        (
            &mut widgets.open,
            color(theme, "menu.itemHover")?,
            input_border,
            foreground,
        ),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = stroke;
        state.fg_stroke.color = text;
        state.corner_radius = corner_radius;
    }

    visuals.weak_text_color = Some(color(theme, "appSidebar.iconDefault")?);
    visuals.selection = Selection {
        bg_fill: color(theme, "list.activeBackground")?,
        stroke: Stroke::new(BORDER_WIDTH, color(theme, "list.foreground")?),
    };
    visuals.hyperlink_color = color(theme, "app.accent")?;
    visuals.extreme_bg_color = input_background;
    visuals.text_edit_bg_color = Some(input_background);
    visuals.text_cursor.stroke.color = foreground;
    visuals.warn_fg_color = color(theme, "statusIndicator.warning")?;
    visuals.error_fg_color = color(theme, "statusIndicator.error")?;
    visuals.panel_fill = background;
    visuals.window_fill = color(theme, "menu.background")?;
    visuals.window_stroke = Stroke::new(BORDER_WIDTH, color(theme, "menu.border")?);
    visuals.window_corner_radius = CornerRadius::same(DIALOG_CORNER_RADIUS);
    visuals.window_shadow = Shadow {
        offset: DIALOG_SHADOW_OFFSET,
        blur: DIALOG_SHADOW_BLUR,
        spread: 0,
        color: shadow,
    };
    visuals.menu_corner_radius = corner_radius;
    visuals.popup_shadow = Shadow {
        offset: OVERLAY_SHADOW_OFFSET,
        blur: OVERLAY_SHADOW_BLUR,
        spread: 0,
        color: shadow,
    };
    Ok(visuals)
}

pub fn apply_visuals(context: &egui::Context, visuals: &Visuals) {
    context.all_styles_mut(|style| style.visuals = visuals.clone());
}

pub fn editor_appearance(
    theme: &ResolvedTheme,
    settings: &Settings,
) -> AppResult<EditorAppearance> {
    let font_size = settings.editor_font_size as f32;
    Ok(EditorAppearance {
        font: FontId::monospace(font_size),
        line_height: editor_line_height(font_size),
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

pub fn editor_presentation(settings: &Settings) -> EditorPresentation {
    #[cfg(feature = "native-host")]
    {
        use crate::editor_surface::{CursorBlinking, CursorStyle, RenderWhitespace};
        use taide_model::settings::{
            EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace,
        };
        return EditorPresentation {
            options: EditorDisplayOptions {
                word_wrap: settings.editor_word_wrap,
                render_whitespace: match settings.editor_render_whitespace {
                    EditorRenderWhitespace::None => RenderWhitespace::None,
                    EditorRenderWhitespace::Boundary => RenderWhitespace::Boundary,
                    EditorRenderWhitespace::Selection => RenderWhitespace::Selection,
                    EditorRenderWhitespace::All => RenderWhitespace::All,
                },
                rulers: settings.editor_rulers.clone(),
                cursor_style: match settings.editor_cursor_style {
                    EditorCursorStyle::Line => CursorStyle::Line,
                    EditorCursorStyle::Block => CursorStyle::Block,
                    EditorCursorStyle::Underline => CursorStyle::Underline,
                },
                cursor_blinking: match settings.editor_cursor_blinking {
                    EditorCursorBlinking::Blink => CursorBlinking::Blink,
                    EditorCursorBlinking::Smooth => CursorBlinking::Smooth,
                    EditorCursorBlinking::Phase => CursorBlinking::Phase,
                    EditorCursorBlinking::Expand => CursorBlinking::Expand,
                    EditorCursorBlinking::Solid => CursorBlinking::Solid,
                },
                smooth_caret: settings.editor_cursor_smooth_caret_animation,
                scroll_beyond_last_line: settings.editor_scroll_beyond_last_line,
                smooth_scrolling: settings.editor_smooth_scrolling,
                bracket_pair_colorization: settings.editor_bracket_pair_colorization,
                bracket_pair_guides: settings.editor_bracket_pair_guides,
                sticky_scroll: settings.editor_sticky_scroll_enabled,
                minimap: settings.editor_minimap,
                ..Default::default()
            },
        };
    }
    #[cfg(not(feature = "native-host"))]
    EditorPresentation {
        options: EditorDisplayOptions {
            word_wrap: settings.editor_word_wrap,
            ..Default::default()
        },
    }
}

pub fn editor_folding(tier: FileSizeTier) -> bool {
    !matches!(tier, FileSizeTier::Large | FileSizeTier::ReadOnly)
}

#[cfg(feature = "native-host")]
pub fn editor_display_colors(
    theme: &ResolvedTheme,
) -> AppResult<crate::editor_display::EditorDisplayColors> {
    const DARK_RULER: Color32 = Color32::from_rgb(90, 90, 90);
    const LIGHT_RULER: Color32 = Color32::from_rgb(211, 211, 211);
    Ok(crate::editor_display::EditorDisplayColors {
        whitespace: color(theme, "editor.whitespace")?,
        ruler: match theme.theme_type {
            ThemeType::Dark => DARK_RULER,
            ThemeType::Light => LIGHT_RULER,
        },
        scrollbar: color(theme, "scrollbar.thumb")?,
        scrollbar_hover: color(theme, "scrollbar.thumbHover")?,
    })
}

#[cfg(feature = "native-host")]
pub fn editor_bracket_colors(
    theme: &ResolvedTheme,
) -> AppResult<crate::editor_brackets::EditorBracketColors> {
    const DARK_PALETTE: [Color32; 3] = [
        Color32::from_rgb(255, 215, 0),
        Color32::from_rgb(218, 112, 214),
        Color32::from_rgb(23, 159, 255),
    ];
    const LIGHT_PALETTE: [Color32; 3] = [
        Color32::from_rgb(4, 49, 250),
        Color32::from_rgb(49, 147, 49),
        Color32::from_rgb(123, 56, 20),
    ];
    const UNEXPECTED_ALPHA: u8 = 204;
    Ok(crate::editor_brackets::EditorBracketColors {
        palette: match theme.theme_type {
            ThemeType::Dark => DARK_PALETTE,
            ThemeType::Light => LIGHT_PALETTE,
        },
        unexpected: Color32::from_rgba_unmultiplied(255, 18, 18, UNEXPECTED_ALPHA),
        indent: color(theme, "editor.indentGuide")?,
        active_indent: color(theme, "editor.whitespace")?,
    })
}

#[cfg(feature = "native-host")]
pub fn editor_sticky_colors(
    theme: &ResolvedTheme,
) -> AppResult<crate::editor_sticky_scroll::EditorStickyColors> {
    const DARK_HOVER: Color32 = Color32::from_rgb(42, 45, 46);
    const LIGHT_HOVER: Color32 = Color32::from_rgb(240, 240, 240);
    Ok(crate::editor_sticky_scroll::EditorStickyColors {
        background: color(theme, "editor.widgetBackground")?,
        border: color(theme, "editor.widgetBorder")?,
        hover: match theme.theme_type {
            ThemeType::Dark => DARK_HOVER,
            ThemeType::Light => LIGHT_HOVER,
        },
        shadow: color(theme, "app.shadow")?,
    })
}

#[cfg(feature = "native-host")]
pub fn editor_minimap_colors(
    theme: &ResolvedTheme,
) -> AppResult<crate::editor_minimap::EditorMinimapColors> {
    Ok(crate::editor_minimap::EditorMinimapColors {
        background: color(theme, "editor.background")?,
        selection: color(theme, "editor.selection")?,
        slider: color(theme, "scrollbar.thumb")?,
        slider_hover: color(theme, "scrollbar.thumbHover")?,
        slider_active: color(theme, "scrollbar.thumbHover")?,
        shadow: color(theme, "app.shadow")?,
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

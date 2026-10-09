use std::ops::Range;

pub const MINIMAP_GUTTER_WIDTH: usize = 8;
const MAX_COLUMN: f64 = 120.0;
const CARET_SPACE: f64 = 2.0;
const RETINA_RATIO: f64 = 2.0;
const GLYPH_HEIGHT: usize = 2;
const GLYPH_COUNT: usize = 96;
const ASCII_START: i32 = 32;
const CHANNEL_MAX: f64 = 255.0;
const NORMAL_SOFTEN: f64 = 12.0 / 15.0;
const LIGHT_SOFTEN: f64 = 50.0 / 60.0;
const SCALE_ONE: &[u8] = include_bytes!("../resources/minimap/scale-1.bin");
const SCALE_TWO: &[u8] = include_bytes!("../resources/minimap/scale-2.bin");

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapDimensions {
    pub width: f64,
    pub pixel_ratio: f64,
    pub scale: usize,
    pub image_width: usize,
    pub image_height: usize,
}

impl MinimapDimensions {
    pub fn new(
        remaining_width: f64,
        height: f64,
        character_width: f64,
        scrollbar_width: f64,
        pixel_ratio: f64,
    ) -> Self {
        let pixel_ratio = if pixel_ratio.is_finite() && pixel_ratio > 0.0 {
            pixel_ratio
        } else {
            1.0
        };
        let scale = if pixel_ratio >= RETINA_RATIO { 2 } else { 1 };
        let character_width = character_width.max(1.0);
        let glyph_width = scale as f64 / pixel_ratio;
        let maximum = (MAX_COLUMN * glyph_width).floor();
        let calculated = (((remaining_width - scrollbar_width - CARET_SPACE) * glyph_width)
            / (character_width + glyph_width))
            .floor()
            .max(0.0)
            + MINIMAP_GUTTER_WIDTH as f64;
        let width = calculated.min(maximum).min(remaining_width.max(0.0));
        Self {
            width,
            pixel_ratio,
            scale,
            image_width: (pixel_ratio * width).floor().max(0.0) as usize,
            image_height: (pixel_ratio * height).floor().max(0.0) as usize,
        }
    }

    pub fn line_pixels(self) -> usize {
        GLYPH_HEIGHT * self.scale
    }

    pub fn line_height(self) -> f64 {
        self.line_pixels() as f64 / self.pixel_ratio
    }

    pub fn image_point_width(self) -> f64 {
        self.image_width as f64 / self.pixel_ratio
    }

    pub fn image_point_height(self) -> f64 {
        self.image_height as f64 / self.pixel_ratio
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MinimapViewport {
    pub line_count: usize,
    pub first_row: usize,
    pub last_row: usize,
    pub first_row_top: f64,
    pub height: f64,
    pub line_height: f64,
    pub scroll_top: f64,
    pub scroll_height: f64,
    pub beyond_last_line: bool,
}

impl MinimapViewport {
    pub fn new(
        line_count: usize,
        height: f64,
        line_height: f64,
        scroll_top: f64,
        scroll_height: f64,
        beyond_last_line: bool,
    ) -> Self {
        let last = line_count.saturating_sub(1);
        let first_row = ((scroll_top / line_height).floor().max(0.0) as usize).min(last);
        Self {
            line_count,
            first_row,
            last_row: ((scroll_top + height) / line_height).ceil().max(1.0) as usize - 1,
            first_row_top: first_row as f64 * line_height,
            height,
            line_height,
            scroll_top,
            scroll_height: scroll_height.max(height),
            beyond_last_line,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MinimapLayout {
    pub rows: Range<usize>,
    pub slider_needed: bool,
    pub slider_top: f64,
    pub slider_height: f64,
    pub slider_ratio: f64,
    pub scroll_top: f64,
    pub scroll_height: f64,
}

impl MinimapLayout {
    pub fn new(
        dimensions: MinimapDimensions,
        viewport: MinimapViewport,
        previous: Option<&Self>,
    ) -> Self {
        if viewport.line_count == 0
            || !viewport.height.is_finite()
            || viewport.height <= 0.0
            || !viewport.line_height.is_finite()
            || viewport.line_height <= 0.0
            || !viewport.scroll_top.is_finite()
            || !viewport.scroll_height.is_finite()
        {
            return Self::default();
        }
        let line_height = dimensions.line_height();
        let fitting = dimensions.image_height / dimensions.line_pixels();
        let expected = viewport.height / viewport.line_height;
        let slider_height = (expected * line_height).floor();
        let extra = if viewport.beyond_last_line {
            (expected - 1.0).max(0.0)
        } else {
            0.0
        };
        let maximum_top = if extra > 0.0 {
            (viewport.line_count as f64 + extra - expected - 1.0) * line_height
        } else {
            (viewport.line_count as f64 * line_height - slider_height).max(0.0)
        }
        .min(viewport.height - slider_height)
        .max(0.0);
        let maximum_scroll = (viewport.scroll_height - viewport.height).max(0.0);
        let ratio = if maximum_scroll > 0.0 {
            maximum_top / maximum_scroll
        } else {
            0.0
        };
        let scroll_top = viewport.scroll_top.clamp(0.0, maximum_scroll);
        let slider_top = scroll_top * ratio;
        if fitting as f64 >= viewport.line_count as f64 + extra {
            return Self {
                rows: 0..viewport.line_count,
                slider_needed: maximum_top > 0.0,
                slider_top,
                slider_height,
                slider_ratio: ratio,
                scroll_top,
                scroll_height: viewport.scroll_height,
            };
        }
        let considering = if viewport.first_row > 0 {
            viewport.first_row as f64 + 1.0
        } else {
            (scroll_top / viewport.line_height).max(1.0)
        };
        let mut start =
            ((considering - slider_top / line_height).floor().max(1.0) as usize).saturating_sub(1);
        if let Some(previous) =
            previous.filter(|previous| previous.scroll_height == viewport.scroll_height)
        {
            if previous.scroll_top > scroll_top {
                start = start.min(previous.rows.start);
            }
            if previous.scroll_top < scroll_top {
                start = start.max(previous.rows.start);
            }
        }
        start = start.min(viewport.line_count - 1);
        let end = start.saturating_add(fitting).min(viewport.line_count);
        let partial = (scroll_top - viewport.first_row_top) / viewport.line_height;
        Self {
            rows: start..end,
            slider_needed: maximum_top > 0.0,
            slider_top: (viewport.first_row as f64 - start as f64 + partial) * line_height,
            slider_height,
            slider_ratio: ratio,
            scroll_top,
            scroll_height: viewport.scroll_height,
        }
    }

    pub fn scroll_from_delta(&self, delta: f64) -> f64 {
        if self.slider_ratio <= 0.0 || !delta.is_finite() {
            return self.scroll_top;
        }
        (self.scroll_top + delta / self.slider_ratio + 0.5).floor()
    }

    pub fn scroll_from_touch(&self, y: f64) -> f64 {
        if self.slider_ratio <= 0.0 || !y.is_finite() {
            return self.scroll_top;
        }
        ((y - self.slider_height / 2.0) / self.slider_ratio + 0.5).floor()
    }
}

pub fn glyph_rgba(
    scale: usize,
    code: u16,
    foreground: [u8; 3],
    background: [u8; 3],
    foreground_alpha: u8,
    light: bool,
) -> Vec<u8> {
    let scale = scale.clamp(1, 2);
    let pixels = scale * scale * GLYPH_HEIGHT;
    (0..pixels)
        .flat_map(|pixel| {
            glyph_pixel(
                glyph_intensity(scale, code, pixel, light),
                foreground,
                background,
                foreground_alpha,
            )
        })
        .collect()
}

pub(crate) fn glyph_intensity(scale: usize, code: u16, pixel: usize, light: bool) -> u8 {
    let data = if scale == 1 { SCALE_ONE } else { SCALE_TWO };
    let index = i32::from(code) - ASCII_START;
    let index = if (0..GLYPH_COUNT as i32).contains(&index) {
        index as usize
    } else {
        index.rem_euclid(GLYPH_COUNT as i32) as usize
    };
    let pixels = scale * scale * GLYPH_HEIGHT;
    let soften = if light { LIGHT_SOFTEN } else { NORMAL_SOFTEN };
    (f64::from(data[index * pixels + pixel]) * soften) as u8
}

pub(crate) fn glyph_pixel(
    intensity: u8,
    foreground: [u8; 3],
    background: [u8; 3],
    foreground_alpha: u8,
) -> [u8; 4] {
    let opacity = f64::from(intensity) / CHANNEL_MAX * (f64::from(foreground_alpha) / CHANNEL_MAX);
    let channel = |index| {
        let background = f64::from(background[index]);
        let value = background + (f64::from(foreground[index]) - background) * opacity;
        value.round_ties_even().clamp(0.0, CHANNEL_MAX) as u8
    };
    [channel(0), channel(1), channel(2), u8::MAX]
}

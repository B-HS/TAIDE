use egui::{Color32, FontFamily, FontId, Rect, Ui};
use taide_model::error::{AppError, AppResult};

pub const MAX_TEXTURES: usize = 256;
pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SIDE: u32 = 8192;
pub const ELLIPSIS: &str = "…";
pub const UV: Rect = Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
const BYTES_PER_PIXEL: usize = 4;

pub struct Text<'a> {
    pub family: &'a str,
    pub value: &'a str,
    pub rect: Rect,
    pub size: f32,
    pub foreground: Color32,
}

pub trait Painter {
    fn paint(&mut self, ui: &Ui, text: &Text<'_>) -> AppResult<bool>;
    fn clear(&mut self);
}

#[cfg(target_arch = "wasm32")]
type PlatformPainter = dyn Painter;
#[cfg(not(target_arch = "wasm32"))]
type PlatformPainter = dyn Painter + Send;

#[derive(Clone, PartialEq, Eq)]
pub struct RasterKey {
    pub family: String,
    pub value: String,
    pub width: u32,
    pub height: u32,
    pub size: u32,
    pub scale: u32,
}

impl RasterKey {
    pub fn new(text: &Text<'_>, scale: f32) -> AppResult<Self> {
        let width = (text.rect.width() * scale).ceil();
        let height = (text.rect.height() * scale).ceil();
        if !scale.is_finite()
            || scale <= 0.0
            || !text.size.is_finite()
            || text.size <= 0.0
            || text.size * scale > MAX_SIDE as f32
            || !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
            || width > MAX_SIDE as f32
            || height > MAX_SIDE as f32
        {
            return Err(AppError::InvalidArgument(
                "font preview geometry is invalid".into(),
            ));
        }
        let key = Self {
            family: text.family.into(),
            value: text.value.into(),
            width: width as u32,
            height: height as u32,
            size: text.size.to_bits(),
            scale: scale.to_bits(),
        };
        key.bytes()?;
        Ok(key)
    }

    pub fn bytes(&self) -> AppResult<usize> {
        usize::try_from(self.width)
            .ok()
            .and_then(|width| {
                usize::try_from(self.height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(BYTES_PER_PIXEL))
            .filter(|bytes| *bytes <= MAX_BYTES)
            .ok_or_else(|| {
                AppError::InvalidArgument("font preview exceeds its pixel budget".into())
            })
    }

    pub fn pixel_size(&self) -> f32 {
        f32::from_bits(self.size) * f32::from_bits(self.scale)
    }
}

pub fn quoted_family(family: &str) -> String {
    let escaped = family
        .chars()
        .map(|character| {
            if character == '\\' || character == '"' {
                return format!("\\{character}");
            }
            if character.is_control() {
                return format!("\\{:X} ", u32::from(character));
            }
            character.to_string()
        })
        .collect::<String>();
    format!("\"{escaped}\"")
}

#[derive(Default)]
pub struct Previews {
    painter: Option<Box<PlatformPainter>>,
}

impl Previews {
    #[cfg(target_arch = "wasm32")]
    pub fn set_painter(&mut self, painter: impl Painter + 'static) {
        self.painter = Some(Box::new(painter));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_painter(&mut self, painter: impl Painter + Send + 'static) {
        self.painter = Some(Box::new(painter));
    }

    pub fn paint(&mut self, ui: &Ui, text: &Text<'_>) -> AppResult<()> {
        if text.rect.width() <= 0.0 || text.rect.height() <= 0.0 || !ui.is_rect_visible(text.rect) {
            return Ok(());
        }
        let mut error = None;
        if let Some(painter) = self.painter.as_mut() {
            match painter.paint(ui, text) {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(failure) => error = Some(failure),
            }
        }
        let named = FontFamily::Name(text.family.into());
        let family = if ui.fonts(|fonts| fonts.families().contains(&named)) {
            named
        } else {
            FontFamily::Proportional
        };
        let mut job = egui::text::LayoutJob::simple(
            text.value.into(),
            FontId::new(text.size, family),
            text.foreground,
            text.rect.width(),
        );
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        ui.painter()
            .with_clip_rect(text.rect.intersect(ui.clip_rect()))
            .galley(text.rect.min, galley, text.foreground);
        error.map_or(Ok(()), Err)
    }

    pub fn clear(&mut self) {
        if let Some(painter) = self.painter.as_mut() {
            painter.clear();
        }
    }
}

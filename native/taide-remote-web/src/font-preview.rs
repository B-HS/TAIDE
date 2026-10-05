use std::cell::Cell;
use std::rc::Rc;

use egui::{ColorImage, TextureHandle, TextureOptions, Ui};
use taide_model::error::{AppError, AppResult};
use taide_native_ui::font_preview::{
    ELLIPSIS, MAX_BYTES, MAX_TEXTURES, Painter, RasterKey, Text, UV, quoted_family,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{CanvasRenderingContext2d, FontFaceSet, HtmlCanvasElement};

const HALF: f64 = 0.5;

struct Entry {
    key: RasterKey,
    texture: TextureHandle,
    bytes: usize,
}

struct Rasterizer {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    fonts: FontFaceSet,
    changed: Rc<Cell<bool>>,
    listener: Closure<dyn FnMut(web_sys::Event)>,
}

#[derive(Default)]
pub(crate) struct Renderer {
    rasterizer: Option<Rasterizer>,
    entries: Vec<Entry>,
    bytes: usize,
}

impl Rasterizer {
    fn new(ui: &Ui) -> AppResult<Self> {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| AppError::Internal("font preview document is unavailable".into()))?;
        let canvas = document
            .create_element("canvas")
            .map_err(|_| AppError::Internal("font preview canvas could not be created".into()))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| AppError::Internal("font preview canvas has an invalid type".into()))?;
        let context = canvas
            .get_context("2d")
            .map_err(|_| AppError::Internal("font preview context could not be created".into()))?
            .ok_or_else(|| AppError::Internal("font preview context is unavailable".into()))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| AppError::Internal("font preview context has an invalid type".into()))?;
        let fonts = document.fonts();
        let changed = Rc::new(Cell::new(false));
        let signal = Rc::clone(&changed);
        let repaint = ui.ctx().clone();
        let listener = Closure::wrap(Box::new(move |_: web_sys::Event| {
            signal.set(true);
            repaint.request_repaint();
        }) as Box<dyn FnMut(web_sys::Event)>);
        fonts
            .add_event_listener_with_callback("loadingdone", listener.as_ref().unchecked_ref())
            .map_err(|_| {
                AppError::Internal("font preview loading listener could not be registered".into())
            })?;
        Ok(Self {
            canvas,
            context,
            fonts,
            changed,
            listener,
        })
    }

    fn image(&self, key: &RasterKey) -> AppResult<ColorImage> {
        self.canvas.set_width(key.width);
        self.canvas.set_height(key.height);
        self.context.set_font(&format!(
            "{}px {}",
            key.pixel_size(),
            quoted_family(&key.family)
        ));
        self.context.set_fill_style_str("#FFFFFF");
        self.context.set_text_align("left");
        self.context.set_text_baseline("alphabetic");
        let mut value = key.value.clone();
        let mut metrics = self
            .context
            .measure_text(&value)
            .map_err(|_| AppError::Internal("font preview metrics are unavailable".into()))?;
        if metrics.width() > f64::from(key.width) {
            let ellipsis_width = self
                .context
                .measure_text(ELLIPSIS)
                .map_err(|_| {
                    AppError::Internal("font preview ellipsis metrics are unavailable".into())
                })?
                .width();
            while !value.is_empty() && metrics.width() + ellipsis_width > f64::from(key.width) {
                value.pop();
                metrics = self.context.measure_text(&value).map_err(|_| {
                    AppError::Internal("font preview metrics are unavailable".into())
                })?;
            }
            value.push_str(ELLIPSIS);
        }
        let ascent = metrics.font_bounding_box_ascent();
        let descent = metrics.font_bounding_box_descent();
        let baseline = (f64::from(key.height) + ascent - descent) * HALF;
        if !baseline.is_finite() {
            return Err(AppError::Internal(
                "font preview baseline is invalid".into(),
            ));
        }
        self.context
            .fill_text(&value, 0.0, baseline)
            .map_err(|_| AppError::Internal("font preview text could not be painted".into()))?;
        let data = self
            .context
            .get_image_data(0.0, 0.0, f64::from(key.width), f64::from(key.height))
            .map_err(|_| AppError::Internal("font preview pixels are unavailable".into()))?
            .data();
        let width = usize::try_from(key.width)
            .map_err(|_| AppError::InvalidArgument("font preview width is invalid".into()))?;
        let height = usize::try_from(key.height)
            .map_err(|_| AppError::InvalidArgument("font preview height is invalid".into()))?;
        Ok(ColorImage::from_rgba_unmultiplied([width, height], &data.0))
    }
}

impl Drop for Rasterizer {
    fn drop(&mut self) {
        drop(self.fonts.remove_event_listener_with_callback(
            "loadingdone",
            self.listener.as_ref().unchecked_ref(),
        ));
    }
}

impl Painter for Renderer {
    fn paint(&mut self, ui: &Ui, text: &Text<'_>) -> AppResult<bool> {
        if self
            .rasterizer
            .as_ref()
            .is_some_and(|rasterizer| rasterizer.changed.replace(false))
        {
            self.entries.clear();
            self.bytes = 0;
        }
        let key = RasterKey::new(text, ui.ctx().pixels_per_point())?;
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            let entry = self.entries.remove(index);
            ui.painter()
                .with_clip_rect(text.rect.intersect(ui.clip_rect()))
                .image(entry.texture.id(), text.rect, UV, text.foreground);
            self.entries.push(entry);
            return Ok(true);
        }
        let bytes = key.bytes()?;
        if self.rasterizer.is_none() {
            self.rasterizer = Some(Rasterizer::new(ui)?);
        }
        let image = self
            .rasterizer
            .as_ref()
            .expect("font preview rasterizer is initialized")
            .image(&key)?;
        while self.entries.len() >= MAX_TEXTURES || self.bytes + bytes > MAX_BYTES {
            let entry = self.entries.remove(0);
            self.bytes -= entry.bytes;
        }
        let texture = ui
            .ctx()
            .load_texture("settings-font-preview", image, TextureOptions::LINEAR);
        ui.painter()
            .with_clip_rect(text.rect.intersect(ui.clip_rect()))
            .image(texture.id(), text.rect, UV, text.foreground);
        self.bytes += bytes;
        self.entries.push(Entry {
            key,
            texture,
            bytes,
        });
        Ok(true)
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
        self.rasterizer = None;
    }
}

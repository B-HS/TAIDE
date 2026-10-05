use std::sync::{Arc, Mutex};

use eframe::egui::{ColorImage, TextureHandle, TextureOptions, Ui};
use resvg::{tiny_skia, usvg};
use taide_model::error::{AppError, AppResult};
use taide_native_ui::font_preview::{
    ELLIPSIS, MAX_BYTES, MAX_TEXTURES, Painter, RasterKey, Text, UV, quoted_family,
};
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

const MAX_PENDING: usize = 4;
const HALF: f32 = 0.5;

struct Entry {
    key: RasterKey,
    texture: Option<TextureHandle>,
    bytes: usize,
}

struct Pending {
    key: RasterKey,
    bytes: usize,
    reply: oneshot::Receiver<AppResult<ColorImage>>,
}

pub(crate) struct Renderer {
    tasks: TaskSupervisor,
    entries: Vec<Entry>,
    pending: Vec<Pending>,
    bytes: usize,
    active: Arc<Mutex<bool>>,
}

impl Renderer {
    pub(crate) fn new(tasks: TaskSupervisor) -> Self {
        Self {
            tasks,
            entries: Vec::new(),
            pending: Vec::new(),
            bytes: 0,
            active: Arc::new(Mutex::new(true)),
        }
    }

    fn poll(&mut self, ui: &Ui) -> AppResult<()> {
        let mut error = None;
        let mut index = 0;
        while index < self.pending.len() {
            let result = match self.pending[index].reply.try_recv() {
                Ok(result) => result,
                Err(oneshot::error::TryRecvError::Empty) => {
                    index += 1;
                    continue;
                }
                Err(oneshot::error::TryRecvError::Closed) => Err(AppError::Internal(
                    "native font preview worker stopped".into(),
                )),
            };
            let pending = self.pending.remove(index);
            let texture = match result {
                Ok(image) => Some(ui.ctx().load_texture(
                    "settings-font-preview",
                    image,
                    TextureOptions::LINEAR,
                )),
                Err(failure) => {
                    error = Some(failure);
                    None
                }
            };
            let bytes = if texture.is_some() { pending.bytes } else { 0 };
            self.bytes -= pending.bytes - bytes;
            self.entries.push(Entry {
                key: pending.key,
                texture,
                bytes,
            });
        }
        error.map_or(Ok(()), Err)
    }

    fn release(&mut self) {
        *self
            .active
            .lock()
            .expect("font preview lifetime lock poisoned") = false;
        self.entries.clear();
        self.pending.clear();
        self.bytes = 0;
    }
}

impl Painter for Renderer {
    fn paint(&mut self, ui: &Ui, text: &Text<'_>) -> AppResult<bool> {
        self.poll(ui)?;
        let key = RasterKey::new(text, ui.ctx().pixels_per_point())?;
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            let entry = self.entries.remove(index);
            if let Some(texture) = &entry.texture {
                ui.painter()
                    .with_clip_rect(text.rect.intersect(ui.clip_rect()))
                    .image(texture.id(), text.rect, UV, text.foreground);
            }
            let painted = entry.texture.is_some();
            self.entries.push(entry);
            return Ok(painted);
        }
        if self.pending.iter().any(|pending| pending.key == key)
            || self.pending.len() >= MAX_PENDING
        {
            return Ok(false);
        }
        let bytes = key.bytes()?;
        while !self.entries.is_empty()
            && (self.entries.len() + self.pending.len() >= MAX_TEXTURES
                || self.bytes + bytes > MAX_BYTES)
        {
            let entry = self.entries.remove(0);
            self.bytes -= entry.bytes;
        }
        if self.bytes + bytes > MAX_BYTES {
            return Ok(false);
        }
        let (sender, reply) = oneshot::channel();
        let active = Arc::clone(&self.active);
        let lifetime = Arc::clone(&active);
        let repaint = ui.ctx().clone();
        let request = key.clone();
        let supervisor = self.tasks.clone();
        if !self
            .tasks
            .spawn_transient("native-settings-font-preview", async move {
                let result = supervisor
                    .run_blocking_result("native-settings-font-raster", move || {
                        if !*lifetime
                            .lock()
                            .expect("font preview lifetime lock poisoned")
                        {
                            return Err(AppError::Forbidden(
                                "native font preview owner is closed".into(),
                            ));
                        }
                        rasterize(&request, crate::system_fonts::database())
                    })
                    .await;
                let lifetime = active.lock().expect("font preview lifetime lock poisoned");
                if *lifetime && sender.send(result).is_ok() {
                    repaint.request_repaint();
                }
            })
        {
            self.entries.push(Entry {
                key,
                texture: None,
                bytes: 0,
            });
            return Err(AppError::Forbidden(
                "native font preview worker is stopping".into(),
            ));
        }
        self.bytes += bytes;
        self.pending.push(Pending { key, bytes, reply });
        Ok(false)
    }

    fn clear(&mut self) {
        if self.entries.is_empty() && self.pending.is_empty() {
            return;
        }
        self.release();
        self.active = Arc::new(Mutex::new(true));
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.release();
    }
}

fn escaped_xml(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            character if character.is_control() && !matches!(character, '\t' | '\n' | '\r') => {
                "\u{FFFD}".into()
            }
            character => character.to_string(),
        })
        .collect()
}

fn rasterize(key: &RasterKey, database: Arc<usvg::fontdb::Database>) -> AppResult<ColorImage> {
    key.bytes()?;
    let family = crate::terminal_fonts::named(&database, &key.family, usvg::fontdb::Weight::NORMAL)
        .and_then(|id| database.face(id))
        .and_then(|face| face.families.first())
        .map(|(name, _)| quoted_family(name))
        .unwrap_or_else(|| "serif".into());
    let options = usvg::Options {
        fontdb: database,
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            resolve_data: Box::new(|_, _, _| None),
        },
        ..Default::default()
    };
    let parse = |value: &str| {
        let source = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><text x=\"0\" y=\"0\" fill=\"white\" font-family=\"{}\" font-size=\"{}\">{}</text></svg>",
            key.width,
            key.height,
            escaped_xml(&family),
            key.pixel_size(),
            escaped_xml(value),
        );
        usvg::Tree::from_str(&source, &options)
            .map_err(|error| AppError::Internal(error.to_string()))
    };
    let mut tree = parse(&key.value)?;
    if tree.root().bounding_box().width() > key.width as f32 {
        let boundaries = key
            .value
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(key.value.len()))
            .collect::<Vec<_>>();
        let mut lower = 0;
        let mut upper = boundaries.len();
        tree = parse(ELLIPSIS)?;
        while lower < upper {
            let middle = lower + (upper - lower) / 2;
            let candidate = parse(&format!("{}{ELLIPSIS}", &key.value[..boundaries[middle]]))?;
            if candidate.root().bounding_box().width() <= key.width as f32 {
                tree = candidate;
                lower = middle + 1;
            } else {
                upper = middle;
            }
        }
    }
    let bounds = tree.root().bounding_box();
    let y = (key.height as f32 - bounds.height()) * HALF - bounds.y();
    let mut pixmap = tiny_skia::Pixmap::new(key.width, key.height)
        .ok_or_else(|| AppError::Internal("native font preview raster allocation failed".into()))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_translate(0.0, y),
        &mut pixmap.as_mut(),
    );
    Ok(ColorImage::from_rgba_unmultiplied(
        [key.width as usize, key.height as usize],
        &pixmap.take_demultiplied(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Color32, FontDefinitions, Pos2, Rect};

    const WIDTH: f32 = 180.0;
    const HEIGHT: f32 = 16.0;
    const SIZE: f32 = 12.0;

    fn database() -> Arc<usvg::fontdb::Database> {
        let mut database = usvg::fontdb::Database::new();
        for font in FontDefinitions::default().font_data.values() {
            database.load_font_data(font.font.to_vec());
        }
        database.set_serif_family("Ubuntu");
        Arc::new(database)
    }

    fn key(family: &str, value: &str) -> RasterKey {
        RasterKey::new(
            &Text {
                family,
                value,
                rect: Rect::from_min_size(Pos2::ZERO, egui::vec2(WIDTH, HEIGHT)),
                size: SIZE,
                foreground: Color32::WHITE,
            },
            1.0,
        )
        .unwrap()
    }

    #[test]
    fn 실제_합성_글꼴과_대체_문자열은_서로_다른_픽셀을_만들고_주입과_초과_폭을_격리한다() {
        let database = database();
        let mono = rasterize(&key("Hack", "MMMM iiii"), Arc::clone(&database)).unwrap();
        let proportional = rasterize(&key("Ubuntu", "MMMM iiii"), Arc::clone(&database)).unwrap();
        assert_ne!(mono.pixels, proportional.pixels);
        assert!(mono.pixels.iter().any(|pixel| pixel.a() > 0));
        assert!(proportional.pixels.iter().any(|pixel| pixel.a() > 0));
        let unavailable = rasterize(
            &key("missing\"/><image href=\"file:///synthetic\"/>", "<&>\"'"),
            Arc::clone(&database),
        )
        .unwrap();
        assert!(unavailable.pixels.iter().any(|pixel| pixel.a() > 0));
        let mut narrow = key("Hack", "글꼴 long family name");
        narrow.width = 24;
        let truncated = rasterize(&narrow, database).unwrap();
        assert_eq!(truncated.size, [24, HEIGHT as usize]);
        assert!(truncated.pixels.iter().any(|pixel| pixel.a() > 0));
        assert_eq!(
            escaped_xml("<image href=\"x\">&\0"),
            "&lt;image href=&quot;x&quot;&gt;&amp;\u{FFFD}"
        );
        assert!(
            RasterKey::new(
                &Text {
                    family: "Hack",
                    value: "test",
                    rect: Rect::EVERYTHING,
                    size: SIZE,
                    foreground: Color32::WHITE
                },
                1.0
            )
            .is_err()
        );
    }

    #[test]
    fn 종료된_렌더러는_응답과_픽셀_예산을_회수하고_이전_세대의_깨우기를_금지한다() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let tasks = TaskSupervisor::new(runtime.handle().clone());
        let mut renderer = Renderer::new(tasks);
        let (sender, reply) = oneshot::channel();
        let request = key("Hack", "test");
        let lifetime = Arc::clone(&renderer.active);
        renderer.bytes = request.bytes().unwrap();
        renderer.pending.push(Pending {
            key: request,
            bytes: renderer.bytes,
            reply,
        });
        renderer.clear();
        assert!(!*lifetime.lock().unwrap());
        assert!(
            sender
                .send(Ok(ColorImage::new([1, 1], vec![Color32::WHITE])))
                .is_err()
        );
        assert!(renderer.pending.is_empty());
        assert!(renderer.entries.is_empty());
        assert_eq!(renderer.bytes, 0);
        assert!(*renderer.active.lock().unwrap());
    }

    #[test]
    fn 실제_painter는_중복과_동시_작업을_제한하며_종료_후_자동_재시도하지_않는다() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let tasks = TaskSupervisor::new(runtime.handle().clone());
        let mut renderer = Renderer::new(tasks.clone());
        let context = egui::Context::default();
        let show = |renderer: &mut Renderer, count: usize| {
            let mut output = context.run_ui(Default::default(), |ui| {
                for index in 0..count {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(WIDTH, HEIGHT), egui::Sense::hover());
                    let text = Text {
                        family: "Hack",
                        value: &format!("entry{index}"),
                        rect,
                        size: SIZE,
                        foreground: Color32::WHITE,
                    };
                    assert!(!renderer.paint(ui, &text).unwrap());
                }
            });
            output.textures_delta.clear();
        };
        show(&mut renderer, MAX_PENDING + 1);
        assert_eq!(renderer.pending.len(), MAX_PENDING);
        assert_eq!(tasks.tracked_count(), MAX_PENDING);
        let reserved = renderer.bytes;
        show(&mut renderer, MAX_PENDING + 1);
        assert_eq!(renderer.pending.len(), MAX_PENDING);
        assert_eq!(renderer.bytes, reserved);
        renderer.clear();
        tasks.stop_all();
        runtime.block_on(tasks.shutdown());
        assert_eq!(tasks.tracked_count(), 0);
        let mut output = context.run_ui(Default::default(), |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(WIDTH, HEIGHT), egui::Sense::hover());
            let text = Text {
                family: "Hack",
                value: "entry0",
                rect,
                size: SIZE,
                foreground: Color32::WHITE,
            };
            assert!(renderer.paint(ui, &text).is_err());
            assert!(!renderer.paint(ui, &text).unwrap());
        });
        output.textures_delta.clear();
        assert!(renderer.pending.is_empty());
        assert_eq!(renderer.entries.len(), 1);
        assert_eq!(renderer.bytes, 0);
        assert_eq!(tasks.tracked_count(), 0);
    }
}

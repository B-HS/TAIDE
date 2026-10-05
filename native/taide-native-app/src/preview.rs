use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;
use std::time::Duration;

use eframe::egui::{self, ColorImage, TextureHandle, Ui};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_runtime::AppServices;

use crate::preview_animation::Playback;
pub use crate::preview_animation::{Animation, Frame};

pub const MAX_RGBA_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TEXTURE_BYTES: usize = 128 * 1024 * 1024;
const CHANNELS: usize = 4;
const PADDING: f32 = 16.0;

#[derive(Debug)]
pub enum Failure {
    Read(AppError),
    Decode(AppError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub token: u64,
    pub max_side: usize,
}

#[derive(Debug)]
pub struct Raster {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
    pub animation: Option<Animation>,
}

pub(crate) fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}

pub(crate) fn rgba_bytes(size: [usize; 2], max_side: usize) -> AppResult<usize> {
    if size.contains(&0) || size.iter().any(|side| *side > max_side) {
        return Err(invalid("preview dimensions exceed the renderer limit"));
    }
    let bytes = size[0]
        .checked_mul(size[1])
        .and_then(|pixels| pixels.checked_mul(CHANNELS))
        .filter(|bytes| *bytes <= MAX_RGBA_BYTES)
        .ok_or_else(|| invalid("decoded preview exceeds the pixel budget"))?;
    Ok(bytes)
}

pub(crate) fn raster_limits(max_side: usize) -> Limits {
    let side = u32::try_from(max_side).unwrap_or(u32::MAX);
    let mut limits = Limits::default();
    limits.max_image_width = Some(side);
    limits.max_image_height = Some(side);
    limits.max_alloc = Some(MAX_RGBA_BYTES as u64);
    limits
}

pub(crate) fn raster_decoder(bytes: &[u8], max_side: usize) -> AppResult<impl ImageDecoder + '_> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded preview exceeds the file preview budget"));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(
            ImageFormat::Png
                | ImageFormat::Jpeg
                | ImageFormat::Gif
                | ImageFormat::WebP
                | ImageFormat::Bmp
        )
    ) {
        return Err(invalid(
            "native raster decoding does not support this format yet",
        ));
    }
    reader.limits(raster_limits(max_side));
    let decoder = reader
        .into_decoder()
        .map_err(|error| invalid(error.to_string()))?;
    let (width, height) = decoder.dimensions();
    rgba_bytes([width as usize, height as usize], max_side)?;
    Ok(decoder)
}

pub fn decode(bytes: &[u8], max_side: usize) -> AppResult<Raster> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded preview exceeds the file preview budget"));
    }
    let Ok(format) = image::guess_format(bytes) else {
        return crate::preview_svg::decode(bytes, max_side);
    };
    #[cfg(target_os = "macos")]
    if format == ImageFormat::Avif {
        return crate::preview_macos::decode(bytes, max_side);
    }
    if let Some(raster) = crate::preview_animation::decode(bytes, format, max_side)? {
        return Ok(raster);
    }
    let mut decoder = raster_decoder(bytes, max_side)?;
    #[cfg(target_os = "macos")]
    if decoder
        .icc_profile()
        .map_err(|error| invalid(error.to_string()))?
        .is_some()
    {
        drop(decoder);
        return crate::preview_macos::decode(bytes, max_side);
    }
    let orientation = decoder
        .orientation()
        .map_err(|error| invalid(error.to_string()))?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|error| invalid(error.to_string()))?;
    image.apply_orientation(orientation);
    let (width, height) = (image.width(), image.height());
    let size = [width as usize, height as usize];
    let expected = rgba_bytes(size, max_side)?;
    let rgba = image.into_rgba8().into_raw();
    if rgba.len() != expected {
        return Err(invalid("decoded preview has an invalid pixel length"));
    }
    Ok(Raster {
        size,
        rgba,
        animation: None,
    })
}

pub async fn read(services: &AppServices, request: &Request) -> AppResult<Raster> {
    let max_side = request.max_side;
    read_approved(
        services,
        request.path.clone(),
        "native-preview",
        move |source| decode(&taide_file::service::read_raw(source)?, max_side),
    )
    .await
}

pub(crate) async fn read_approved<T, F>(
    services: &AppServices,
    path: String,
    name: &'static str,
    decode: F,
) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> AppResult<T> + Send + 'static,
{
    let operation = services
        .tasks
        .begin_operation(name)
        .ok_or_else(|| AppError::Forbidden("native preview is shutting down".into()))?;
    let guard = services.state.begin_owned_mutation().await;
    let state = services.state.clone();
    services
        .tasks
        .run_blocking_result(name, move || {
            let _operation = operation;
            let _guard = guard;
            if state.is_shutting_down() || !Path::new(&path).is_absolute() {
                return Err(AppError::Forbidden(
                    "native preview requires a live approved absolute path".into(),
                ));
            }
            let projects = state.projects.read().clone();
            let (_, canonical) = root_guard::resolve_owning_project_or_cli_opened(
                &projects,
                &state.cli_opened_paths.read(),
                Path::new(&path),
            )?;
            let raster = decode(&canonical)?;
            let (_, current) = root_guard::resolve_owning_project_or_cli_opened(
                &state.projects.read(),
                &state.cli_opened_paths.read(),
                Path::new(&path),
            )?;
            if canonical != current || state.is_shutting_down() {
                return Err(AppError::Forbidden(
                    "native preview approval changed while decoding".into(),
                ));
            }
            Ok(raster)
        })
        .await
}

enum Entry {
    Loading(u64),
    Ready {
        texture: TextureHandle,
        playback: Option<Playback>,
        bytes: usize,
    },
    Failed(AppError),
}

#[derive(Default)]
pub struct Cache {
    entries: HashMap<String, Entry>,
    active: Option<Request>,
    token: u64,
}

impl Cache {
    pub fn retain(&mut self, paths: &HashSet<String>) {
        self.entries.retain(|path, _| paths.contains(path));
    }

    pub fn invalidate(&mut self, path: &str) {
        self.entries.remove(path);
    }

    pub fn invalidate_all(&mut self) {
        self.entries.clear();
    }

    pub fn invalidate_root(&mut self, root: &str) {
        self.entries
            .retain(|path, _| !Path::new(path).starts_with(root));
    }

    pub fn begin(&mut self, path: &str, max_side: usize) -> Option<Request> {
        if self.active.is_some() || self.entries.contains_key(path) {
            return None;
        }
        self.token = self.token.checked_add(1)?;
        let request = Request {
            path: path.into(),
            token: self.token,
            max_side,
        };
        self.entries
            .insert(path.into(), Entry::Loading(request.token));
        self.active = Some(request.clone());
        Some(request)
    }

    pub fn cancelled(&mut self, request: &Request) {
        if self.active.as_ref() == Some(request) {
            self.active = None;
            if matches!(self.entries.get(&request.path), Some(Entry::Loading(token)) if *token == request.token)
            {
                self.entries.remove(&request.path);
            }
        }
    }

    pub fn reset_pending(&mut self) {
        if let Some(request) = self.active.clone() {
            self.cancelled(&request);
        }
    }

    pub fn accept(&mut self, context: &egui::Context, request: Request, result: AppResult<Raster>) {
        self.accept_with_other_bytes(context, request, result, 0);
    }

    pub(crate) fn retained_bytes(&self) -> usize {
        self.entries
            .values()
            .filter_map(|entry| match entry {
                Entry::Ready { bytes, .. } => Some(*bytes),
                _ => None,
            })
            .sum()
    }

    pub(crate) fn accept_with_other_bytes(
        &mut self,
        context: &egui::Context,
        request: Request,
        result: AppResult<Raster>,
        other_bytes: usize,
    ) {
        if self.active.as_ref() != Some(&request) {
            return;
        }
        self.active = None;
        if !matches!(self.entries.get(&request.path), Some(Entry::Loading(token)) if *token == request.token)
        {
            return;
        }
        let result = result.and_then(|raster| {
            let mut first = raster.rgba;
            let renderer_side = context.input(|input| input.max_texture_side);
            let mut bytes = rgba_bytes(raster.size, renderer_side.min(request.max_side))?;
            if first.len() != bytes {
                return Err(invalid("preview reply has an invalid pixel length"));
            }
            let frame_bytes = bytes;
            let mut playback = None;
            if let Some(animation) = raster.animation {
                if animation.frames.len() >= crate::preview_animation::MAX_FRAMES {
                    return Err(invalid("animation reply exceeds the frame budget"));
                }
                let mut decoded = bytes;
                for frame in &animation.frames {
                    if frame.rgba.len() != frame_bytes {
                        return Err(invalid("animation reply has an invalid pixel length"));
                    }
                    decoded = decoded
                        .checked_add(frame_bytes)
                        .filter(|total| *total <= MAX_RGBA_BYTES)
                        .ok_or_else(|| invalid("animation reply exceeds the decoded budget"))?;
                }
                bytes = bytes
                    .checked_add(decoded)
                    .ok_or_else(|| invalid("animation cache byte overflow"))?;
                playback = Some(Playback::new(std::mem::take(&mut first), animation)?);
            }
            let used = self
                .entries
                .values()
                .filter_map(|entry| match entry {
                    Entry::Ready { bytes, .. } => Some(*bytes),
                    _ => None,
                })
                .sum::<usize>();
            if used
                .checked_add(other_bytes)
                .and_then(|used| used.checked_add(bytes))
                .is_none_or(|total| total > MAX_TEXTURE_BYTES)
            {
                return Err(invalid("native preview texture cache is full"));
            }
            let rgba = match &playback {
                Some(playback) => &playback.frames[0].rgba,
                None => &first,
            };
            let texture = context.load_texture(
                format!("native-preview-{}", request.token),
                ColorImage::from_rgba_unmultiplied(raster.size, rgba),
                egui::TextureOptions::LINEAR,
            );
            Ok(Entry::Ready {
                texture,
                playback,
                bytes,
            })
        });
        self.entries.insert(
            request.path,
            match result {
                Ok(entry) => entry,
                Err(error) => Entry::Failed(error),
            },
        );
    }

    pub fn texture(&self, path: &str) -> Option<&TextureHandle> {
        match self.entries.get(path) {
            Some(Entry::Ready { texture, .. }) => Some(texture),
            _ => None,
        }
    }

    pub fn advance(&mut self, path: &str, context: &egui::Context) -> Option<&TextureHandle> {
        let Some(Entry::Ready {
            texture, playback, ..
        }) = self.entries.get_mut(path)
        else {
            return None;
        };
        if let Some(playback) = playback {
            let now = context.input(|input| input.time);
            if let Ok(now) = Duration::try_from_secs_f64(now) {
                let (index, delay) = playback.step(now);
                if index != playback.current {
                    texture.set(
                        ColorImage::from_rgba_unmultiplied(
                            texture.size(),
                            &playback.frames[index].rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    playback.current = index;
                }
                if let Some(delay) = delay {
                    context.request_repaint_after(delay);
                }
            }
        }
        Some(texture)
    }

    pub fn error(&self, path: &str) -> Option<&AppError> {
        match self.entries.get(path) {
            Some(Entry::Failed(error)) => Some(error),
            _ => None,
        }
    }
}

pub fn show_image(ui: &mut Ui, texture: &TextureHandle, file_name: &str) -> egui::Response {
    let viewport = ui.available_size();
    let natural = texture.size_vec2();
    let room = (viewport - egui::Vec2::splat(PADDING * 2.0)).max(egui::Vec2::ZERO);
    let scale = (room.x / natural.x).min(room.y / natural.y).min(1.0);
    egui::ScrollArea::both()
        .show(ui, |ui| {
            ui.allocate_ui_with_layout(
                viewport,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.add(
                        egui::Image::new(texture)
                            .fit_to_exact_size(natural * scale)
                            .alt_text(file_name)
                            .sense(egui::Sense::click()),
                    )
                },
            )
            .inner
        })
        .inner
}

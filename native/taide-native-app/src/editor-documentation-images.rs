use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use eframe::egui::{self, Context, TextureHandle, Ui};
use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_native_editor::documentation::{Block, Inline, RichDocument};
use taide_runtime::{AppServices, AppState, TaskSupervisor};
use tokio::sync::{oneshot, watch};

use crate::preview::{MAX_RGBA_BYTES, MAX_TEXTURE_BYTES, Raster, invalid};
use crate::preview_animation::Playback;

const MAX_ACTIVE_LOADS: usize = MAX_TEXTURE_BYTES / MAX_RGBA_BYTES;
const BASE64_INPUT_BLOCK: u64 = 4;
const BASE64_OUTPUT_BLOCK: u64 = 3;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    project: ProjectId,
    root: PathBuf,
    source: String,
    max_side: usize,
}

struct Loading {
    receiver: oneshot::Receiver<AppResult<Raster>>,
    cancel: watch::Sender<bool>,
}

impl Drop for Loading {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

enum Entry {
    Queued,
    Loading(Loading),
    Ready {
        texture: TextureHandle,
        playback: Option<Playback>,
        bytes: usize,
    },
    Failed,
}

#[derive(Default)]
pub(crate) struct Cache {
    entries: HashMap<Key, Entry>,
}

impl Cache {
    pub(crate) fn retain_documents<'a>(
        &mut self,
        documents: impl Iterator<Item = (&'a ProjectId, &'a RichDocument)>,
    ) {
        let mut active = HashMap::new();
        for (project, document) in documents {
            let mut sources = HashSet::new();
            collect(&document.blocks, &mut sources);
            active
                .entry(project.clone())
                .or_insert_with(HashSet::new)
                .extend(sources);
        }
        self.entries.retain(|key, _| {
            active
                .get(&key.project)
                .is_some_and(|sources| sources.contains(key.source.as_str()))
        });
    }

    pub(crate) fn prepare<'a>(
        &mut self,
        context: &Context,
        services: &AppServices,
        documents: impl Iterator<Item = (&'a ProjectId, &'a RichDocument)>,
    ) {
        let max_side = context.input(|input| input.max_texture_side);
        let projects = services.state.projects.read();
        let mut active = HashSet::new();
        for (project, document) in documents {
            let Ok(root) = root_guard::project_root(&projects, project) else {
                continue;
            };
            let mut sources = HashSet::new();
            collect(&document.blocks, &mut sources);
            active.extend(sources.into_iter().map(|source| Key {
                project: project.clone(),
                root: root.clone(),
                source: source.into(),
                max_side,
            }));
        }
        drop(projects);
        self.entries.retain(|key, _| active.contains(key));
        for key in active {
            self.entries.entry(key).or_insert(Entry::Queued);
        }
        let mut completed = Vec::new();
        for (key, entry) in &mut self.entries {
            if let Entry::Loading(loading) = entry {
                match loading.receiver.try_recv() {
                    Ok(result) => completed.push((key.clone(), result)),
                    Err(oneshot::error::TryRecvError::Closed) => completed.push((
                        key.clone(),
                        Err(invalid("documentation image request ended")),
                    )),
                    Err(oneshot::error::TryRecvError::Empty) => {}
                }
            }
        }
        for (key, result) in completed {
            let used = self
                .entries
                .values()
                .filter_map(|entry| match entry {
                    Entry::Ready { bytes, .. } => Some(*bytes),
                    _ => None,
                })
                .sum::<usize>();
            let entry = result
                .and_then(|raster| ready(context, raster, used))
                .unwrap_or(Entry::Failed);
            self.entries.insert(key, entry);
        }
        let loading = self
            .entries
            .values()
            .filter(|entry| matches!(entry, Entry::Loading(_)))
            .count();
        let next = self
            .entries
            .iter()
            .filter_map(|(key, entry)| matches!(entry, Entry::Queued).then_some(key.clone()))
            .take(MAX_ACTIVE_LOADS.saturating_sub(loading))
            .collect::<Vec<_>>();
        for key in next {
            let (sender, receiver) = oneshot::channel();
            let (cancel, mut cancelled) = watch::channel(false);
            let state = services.state.clone();
            let tasks = services.tasks.clone();
            let request = key.clone();
            let repaint = context.clone();
            let started = services.tasks.spawn_transient("native-documentation-image", async move {
                tokio::select! {
                    _ = cancelled.changed() => {}
                    result = load(state, tasks, request) => { drop(sender.send(result)); repaint.request_repaint(); }
                }
            });
            self.entries.insert(
                key,
                if started {
                    Entry::Loading(Loading { receiver, cancel })
                } else {
                    Entry::Failed
                },
            );
        }
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        source: &str,
        alt: &str,
        dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<egui::Response> {
        let entry = self.entries.iter_mut().find_map(|(key, entry)| {
            (&key.project == project && key.source == source).then_some(entry)
        })?;
        let Entry::Ready {
            texture, playback, ..
        } = entry
        else {
            return None;
        };
        if let Some(playback) = playback {
            let now = Duration::try_from_secs_f64(ui.input(|input| input.time)).unwrap_or_default();
            let (index, delay) = playback.step(now);
            if index != playback.current {
                texture.set(
                    egui::ColorImage::from_rgba_unmultiplied(
                        texture.size(),
                        &playback.frames[index].rgba,
                    ),
                    egui::TextureOptions::LINEAR,
                );
                playback.current = index;
            }
            if let Some(delay) = delay {
                ui.ctx().request_repaint_after(delay);
            }
        }
        let natural = texture.size_vec2();
        let size = match (dimensions.width, dimensions.height) {
            (Some(width), Some(height)) => egui::vec2(width as f32, height as f32),
            (Some(width), None) => natural * (width as f32 / natural.x),
            (None, Some(height)) => natural * (height as f32 / natural.y),
            (None, None) => natural,
        };
        Some(
            ui.add(
                egui::Image::new(&*texture)
                    .fit_to_exact_size(size)
                    .maintain_aspect_ratio(false)
                    .alt_text(alt)
                    .sense(egui::Sense::click()),
            ),
        )
    }
}

fn ready(context: &Context, raster: Raster, used: usize) -> AppResult<Entry> {
    let mut first = raster.rgba;
    let playback = raster
        .animation
        .map(|animation| Playback::new(std::mem::take(&mut first), animation))
        .transpose()?;
    let frame_bytes = crate::preview::rgba_bytes(raster.size, usize::MAX)?;
    let retained = playback.as_ref().map_or(first.len(), |playback| {
        playback.frames.iter().map(|frame| frame.rgba.len()).sum()
    });
    let bytes = retained
        .checked_add(frame_bytes)
        .filter(|bytes| {
            used.checked_add(*bytes)
                .is_some_and(|total| total <= MAX_TEXTURE_BYTES)
        })
        .ok_or_else(|| invalid("documentation images exceed the texture budget"))?;
    let rgba = playback.as_ref().map_or(first.as_slice(), |playback| {
        playback.frames[0].rgba.as_slice()
    });
    let texture = context.load_texture(
        "native-documentation-image",
        egui::ColorImage::from_rgba_unmultiplied(raster.size, rgba),
        egui::TextureOptions::LINEAR,
    );
    Ok(Entry::Ready {
        texture,
        playback,
        bytes,
    })
}

fn collect<'a>(blocks: &'a [Block], sources: &mut HashSet<&'a str>) {
    for block in blocks {
        match block {
            Block::Paragraph(contents) | Block::Heading { contents, .. } => {
                collect_inline(contents, sources)
            }
            Block::Quote(blocks) => collect(blocks, sources),
            Block::List { items, .. } => {
                for item in items {
                    collect(&item.blocks, sources);
                }
            }
            Block::Table { header, rows, .. } => {
                for row in std::iter::once(header).chain(rows) {
                    for cell in row {
                        collect_inline(cell, sources);
                    }
                }
            }
            Block::Code { .. } | Block::Rule => {}
        }
    }
}

fn collect_inline<'a>(contents: &'a [Inline], sources: &mut HashSet<&'a str>) {
    sources.extend(contents.iter().filter_map(|inline| match inline {
        Inline::Image { source, .. } if !source.is_empty() => Some(source.as_str()),
        _ => None,
    }));
}

async fn load(state: AppState, tasks: TaskSupervisor, key: Key) -> AppResult<Raster> {
    let uri = url::Url::parse(&key.source).map_err(|error| invalid(error.to_string()))?;
    if state.is_shutting_down()
        || root_guard::project_root(&state.projects.read(), &key.project)? != key.root
    {
        return Err(AppError::Forbidden(
            "documentation image project changed".into(),
        ));
    }
    if uri.scheme() == "file" {
        let path = uri
            .to_file_path()
            .map_err(|_| invalid("documentation image file URI is invalid"))?;
        let guard = state.begin_owned_mutation().await;
        return tasks
            .run_blocking_result("native-documentation-image-file", move || {
                let _guard = guard;
                if state.is_shutting_down()
                    || root_guard::project_root(&state.projects.read(), &key.project)? != key.root
                {
                    return Err(AppError::Forbidden(
                        "documentation image project changed".into(),
                    ));
                }
                let path = root_guard::ensure_within_root(&key.root, &path)?;
                root_guard::ensure_existing_file(&path, &key.source)?;
                let mut bytes = Vec::new();
                std::fs::File::open(path)?
                    .take(taide_model::file::READ_ONLY_FILE_BYTES + 1)
                    .read_to_end(&mut bytes)?;
                check_bytes(bytes.len())?;
                crate::preview::decode(&bytes, key.max_side)
            })
            .await;
    }
    if uri.scheme() == "data" {
        return tasks
            .run_blocking_result("native-documentation-image-data", move || {
                crate::preview::decode(&data_bytes(uri.as_str())?, key.max_side)
            })
            .await;
    }
    if !matches!(uri.scheme(), "http" | "https") {
        return Err(invalid("documentation image scheme is invalid"));
    }
    let mut response =
        taide_infra::http::outbound_http_client(taide_infra::http::HttpClientProfile::Api)
            .get(uri)
            .send()
            .await
            .map_err(|error| invalid(error.to_string()))?
            .error_for_status()
            .map_err(|error| invalid(error.to_string()))?;
    if response
        .content_length()
        .is_some_and(|length| length > taide_model::file::READ_ONLY_FILE_BYTES)
    {
        return Err(invalid(
            "documentation image exceeds the file preview budget",
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| invalid(error.to_string()))?
    {
        check_bytes(
            bytes
                .len()
                .checked_add(chunk.len())
                .ok_or_else(|| invalid("documentation image length overflow"))?,
        )?;
        bytes.extend_from_slice(&chunk);
    }
    tasks
        .run_blocking_result("native-documentation-image-decode", move || {
            crate::preview::decode(&bytes, key.max_side)
        })
        .await
}

fn check_bytes(length: usize) -> AppResult<()> {
    if length as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid(
            "documentation image exceeds the file preview budget",
        ));
    }
    Ok(())
}

fn data_bytes(source: &str) -> AppResult<Vec<u8>> {
    let (metadata, payload) = source
        .strip_prefix("data:image/")
        .and_then(|source| source.split_once(','))
        .ok_or_else(|| invalid("documentation image data URI is invalid"))?;
    let payload = percent_decode(payload)?;
    let bytes = if metadata
        .split(';')
        .any(|parameter| parameter.eq_ignore_ascii_case("base64"))
    {
        let max_encoded = taide_model::file::READ_ONLY_FILE_BYTES.div_ceil(BASE64_OUTPUT_BLOCK)
            * BASE64_INPUT_BLOCK;
        if payload.len() as u64 > max_encoded {
            return Err(invalid(
                "documentation image data exceeds the file preview budget",
            ));
        }
        base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|error| invalid(error.to_string()))?
    } else {
        payload
    };
    check_bytes(bytes.len())?;
    Ok(bytes)
}

fn percent_decode(source: &str) -> AppResult<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut input = source.bytes();
    while let Some(byte) = input.next() {
        let byte = if byte == b'%' {
            let high = input.next().and_then(|byte| char::from(byte).to_digit(16));
            let low = input.next().and_then(|byte| char::from(byte).to_digit(16));
            let (Some(high), Some(low)) = (high, low) else {
                return Err(invalid("documentation image data escape is invalid"));
            };
            (high * 16 + low) as u8
        } else {
            byte
        };
        bytes.push(byte);
        if bytes.len() as u64
            > taide_model::file::READ_ONLY_FILE_BYTES.div_ceil(BASE64_OUTPUT_BLOCK)
                * BASE64_INPUT_BLOCK
        {
            return Err(invalid(
                "documentation image data exceeds the file preview budget",
            ));
        }
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "editor-documentation-images-tests.rs"]
mod tests;

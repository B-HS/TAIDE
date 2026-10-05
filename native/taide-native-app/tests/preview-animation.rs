use std::borrow::Cow;
use std::collections::HashSet;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Context, RawInput};
use image::{DynamicImage, ImageFormat, RgbaImage};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::preview::{Animation, Cache, Frame, Raster, decode, show_image};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SIDE: usize = 32;
const RED: [u8; 8] = [255, 0, 0, 255, 255, 0, 0, 255];
const BLUE: [u8; 8] = [0, 0, 255, 255, 0, 0, 255, 255];
const PALETTE: [u8; 9] = [255, 0, 0, 0, 255, 0, 0, 0, 255];
const FIRST_DELAY: Duration = Duration::from_millis(50);
const SECOND_DELAY: Duration = Duration::from_millis(100);
const RIFF_HEADER: usize = 12;
const CHUNK_ALIGN: usize = 2;
const OVER_FRAME_BUDGET: usize = 4096;

struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

fn gif(repeat: Option<gif::Repeat>, delay: u16, partial: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 2, 1, &PALETTE).unwrap();
        if let Some(repeat) = repeat {
            encoder.set_repeat(repeat).unwrap();
        }
        encoder
            .write_frame(&gif::Frame {
                width: 2,
                height: 1,
                delay,
                buffer: Cow::Borrowed(&[0, 0]),
                ..Default::default()
            })
            .unwrap();
        if partial {
            encoder
                .write_frame(&gif::Frame {
                    width: 1,
                    height: 1,
                    left: 1,
                    delay: 10,
                    dispose: gif::DisposalMethod::Previous,
                    buffer: Cow::Borrowed(&[1]),
                    ..Default::default()
                })
                .unwrap();
            encoder
                .write_frame(&gif::Frame {
                    width: 1,
                    height: 1,
                    delay: 10,
                    buffer: Cow::Borrowed(&[2]),
                    ..Default::default()
                })
                .unwrap();
        } else {
            encoder
                .write_frame(&gif::Frame {
                    width: 2,
                    height: 1,
                    delay: 10,
                    buffer: Cow::Borrowed(&[2, 2]),
                    ..Default::default()
                })
                .unwrap();
        }
    }
    bytes
}

fn apng(partial: bool, plays: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .set_animated(if partial { 3 } else { 2 }, plays)
            .unwrap();
        encoder.set_frame_delay(1, 20).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&RED).unwrap();
        writer.set_frame_delay(1, 10).unwrap();
        if partial {
            writer.set_frame_dimension(1, 1).unwrap();
            writer.set_frame_position(1, 0).unwrap();
            writer.set_dispose_op(png::DisposeOp::Previous).unwrap();
            writer.write_image_data(&[0, 255, 0, 255]).unwrap();
            writer.set_frame_position(0, 0).unwrap();
            writer.set_dispose_op(png::DisposeOp::None).unwrap();
            writer.write_image_data(&[0, 0, 255, 255]).unwrap();
        } else {
            writer.write_image_data(&BLUE).unwrap();
        }
        writer.finish().unwrap();
    }
    bytes
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = kind.to_vec();
    bytes.extend_from_slice(&u32::try_from(data.len()).unwrap().to_le_bytes());
    bytes.extend_from_slice(data);
    if !data.len().is_multiple_of(CHUNK_ALIGN) {
        bytes.push(0);
    }
    bytes
}

fn webp(plays: u16) -> Vec<u8> {
    let mut content = b"WEBP".to_vec();
    content.extend(chunk(b"VP8X", &[0x12, 0, 0, 0, 1, 0, 0, 0, 0, 0]));
    let mut animation = vec![0, 0, 0, 0];
    animation.extend(plays.to_le_bytes());
    content.extend(chunk(b"ANIM", &animation));
    for (pixels, delay) in [(RED, 50u32), (BLUE, 100u32)] {
        let mut encoded = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_raw(2, 1, pixels.to_vec()).unwrap())
            .write_to(&mut encoded, ImageFormat::WebP)
            .unwrap();
        assert_eq!(&encoded.get_ref()[RIFF_HEADER..RIFF_HEADER + 4], b"VP8L");
        let mut frame = vec![0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0];
        frame.extend_from_slice(&delay.to_le_bytes()[..3]);
        frame.push(2);
        frame.extend_from_slice(&encoded.get_ref()[RIFF_HEADER..]);
        content.extend(chunk(b"ANMF", &frame));
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&u32::try_from(content.len()).unwrap().to_le_bytes());
    bytes.extend(content);
    bytes
}

fn painted(cache: &mut Cache, context: &Context, path: &str, time: f64) -> Vec<egui::Color32> {
    let id = cache.texture(path).unwrap().id();
    let mut output = context.run_ui(
        RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            let texture = cache.advance(path, ui.ctx()).unwrap();
            show_image(ui, texture, "synthetic animation");
        },
    );
    let mut pixels = Vec::new();
    for (texture, updates) in &output.textures_delta.set {
        if *texture != id {
            continue;
        }
        for update in updates {
            let egui::ImageData::Color(image) = &update.image;
            pixels = image.pixels.clone();
        }
    }
    output.textures_delta.clear();
    pixels
}

#[test]
fn 실제_host의_gif_apng_webp는_frame과_repeat_texture_close를_연결한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-animation-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic animation".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state, tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    runtime.block_on(async {
        for (extension, bytes) in [
            ("gif", gif(Some(gif::Repeat::Finite(1)), 5, false)),
            ("png", apng(false, 2)),
            ("webp", webp(2)),
        ] {
            let path = root
                .join(format!("synthetic.{extension}"))
                .to_str()
                .unwrap()
                .to_owned();
            std::fs::write(&path, &bytes).unwrap();
            let context = Context::default();
            let mut cache = Cache::default();
            let request = cache.begin(&path, MAX_SIDE).unwrap();
            host.submit(HostCommand::ReadPreview(request.clone()))
                .unwrap();
            let reply = tokio::time::timeout(TIMEOUT, async {
                loop {
                    if let Some(reply) = host.poll() {
                        return reply;
                    }
                    ready.notified().await;
                }
            })
            .await
            .unwrap();
            let HostReply::Preview {
                request: returned,
                result,
            } = reply
            else {
                panic!("wrong reply");
            };
            assert_eq!(returned, request);
            let raster = result.unwrap();
            assert_eq!(raster.size, [2, 1]);
            assert_eq!(raster.rgba, RED);
            let animation = raster.animation.as_ref().unwrap();
            assert_eq!(animation.plays, Some(2), "{extension}");
            assert_eq!(animation.first_delay, FIRST_DELAY);
            assert_eq!(animation.frames[0].delay, SECOND_DELAY);
            assert_eq!(animation.frames[0].rgba, BLUE);
            cache.accept(&context, request, Ok(raster));
            let id = cache.texture(&path).unwrap().id();
            assert_eq!(
                painted(&mut cache, &context, &path, 0.0),
                vec![egui::Color32::RED; 2]
            );
            assert!(painted(&mut cache, &context, &path, 0.04).is_empty());
            assert_eq!(
                painted(&mut cache, &context, &path, 0.06),
                vec![egui::Color32::BLUE; 2]
            );
            assert_eq!(
                painted(&mut cache, &context, &path, 0.16),
                vec![egui::Color32::RED; 2]
            );
            assert_eq!(
                painted(&mut cache, &context, &path, 0.21),
                vec![egui::Color32::BLUE; 2]
            );
            assert!(painted(&mut cache, &context, &path, 0.31).is_empty());
            assert!(painted(&mut cache, &context, &path, 10.0).is_empty());
            assert_eq!(cache.texture(&path).unwrap().id(), id);
            cache.retain(&HashSet::new());
            assert!(cache.advance(&path, &context).is_none());
            assert!(decode(&bytes, 1).is_err());
        }
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn partial_disposal와_무반복_무한반복_짧은지연_및_불량회신을_구분한다() {
    for bytes in [gif(None, 0, true), apng(true, 1)] {
        let raster = decode(&bytes, MAX_SIDE).unwrap();
        assert_eq!(raster.rgba, RED);
        let animation = raster.animation.unwrap();
        assert_eq!(animation.plays, Some(1));
        assert_eq!(animation.frames[0].rgba, [255, 0, 0, 255, 0, 255, 0, 255]);
        assert_eq!(animation.frames[1].rgba, [0, 0, 255, 255, 255, 0, 0, 255]);
    }
    let short = decode(&gif(None, 0, false), MAX_SIDE)
        .unwrap()
        .animation
        .unwrap();
    assert_eq!(short.first_delay, SECOND_DELAY);
    for bytes in [
        gif(Some(gif::Repeat::Infinite), 5, false),
        apng(false, 0),
        webp(0),
    ] {
        assert_eq!(
            decode(&bytes, MAX_SIDE).unwrap().animation.unwrap().plays,
            None
        );
    }
    let context = Context::default();
    let mut cache = Cache::default();
    let request = cache.begin("/root/animation.gif", MAX_SIDE).unwrap();
    cache.accept(
        &context,
        request,
        Ok(Raster {
            size: [2, 1],
            rgba: RED.to_vec(),
            animation: Some(Animation {
                first_delay: FIRST_DELAY,
                plays: Some(1),
                frames: vec![Frame {
                    rgba: vec![0],
                    delay: SECOND_DELAY,
                }],
            }),
        }),
    );
    assert!(cache.texture("/root/animation.gif").is_none());
    assert!(cache.error("/root/animation.gif").is_some());
}

#[test]
fn animation_admission은_프레임상한_zero_metadata_stale를_거절한다() {
    for (count, first_delay, plays) in [
        (OVER_FRAME_BUDGET, FIRST_DELAY, Some(1)),
        (1, Duration::ZERO, Some(1)),
        (1, FIRST_DELAY, Some(0)),
    ] {
        let context = Context::default();
        let mut cache = Cache::default();
        let path = "/root/rejected.gif";
        let request = cache.begin(path, MAX_SIDE).unwrap();
        cache.accept(
            &context,
            request,
            Ok(Raster {
                size: [2, 1],
                rgba: RED.to_vec(),
                animation: Some(Animation {
                    first_delay,
                    plays,
                    frames: (0..count)
                        .map(|_| Frame {
                            rgba: BLUE.to_vec(),
                            delay: SECOND_DELAY,
                        })
                        .collect(),
                }),
            }),
        );
        assert!(cache.texture(path).is_none());
        assert!(cache.error(path).is_some());
        assert!(cache.begin("/root/next.gif", MAX_SIDE).is_some());
    }
    let context = Context::default();
    let mut cache = Cache::default();
    let path = "/root/stale.gif";
    let request = cache.begin(path, MAX_SIDE).unwrap();
    cache.retain(&HashSet::new());
    cache.accept(&context, request, decode(&gif(None, 5, false), MAX_SIDE));
    assert!(cache.advance(path, &context).is_none());
    assert!(cache.begin(path, MAX_SIDE).is_some());
}

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
use taide_native_app::preview::{Cache, Raster, Request, decode, show_image};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const MAX_SIDE: usize = 32;
const WIDTH: f32 = 100.0;
const HEIGHT: f32 = 80.0;
const PIXELS: [u8; 8] = [255, 0, 0, 255, 0, 255, 0, 128];

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

#[test]
fn 실제_host_png는_승인경로와_픽셀_상한_stale_close_및_texture표시를_검사한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-native-preview-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("합성.png").to_str().unwrap().to_owned();
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(RgbaImage::from_raw(2, 1, PIXELS.to_vec()).unwrap())
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    std::fs::write(&path, bytes.get_ref()).unwrap();
    let outside = fixture.0.join("outside.png").to_str().unwrap().to_owned();
    std::fs::write(&outside, bytes.get_ref()).unwrap();
    let malformed = root.join("broken.png").to_str().unwrap().to_owned();
    std::fs::write(&malformed, "not a PNG").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic preview".into(),
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
    let _runtime = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host =
        HostBridge::connect(services.clone(), Arc::new(move || signal.notify_one())).unwrap();
    let context = Context::default();
    let mut cache = Cache::default();
    runtime.block_on(async {
        let request = cache.begin(&path, MAX_SIDE).unwrap();
        assert!(cache.begin(&outside, MAX_SIDE).is_none());
        host.submit(HostCommand::ReadPreview(request.clone()))
            .unwrap();
        let response = tokio::time::timeout(TIMEOUT, async {
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
        } = response
        else {
            panic!("wrong preview reply");
        };
        assert_eq!(returned, request);
        let raster = result.unwrap();
        assert_eq!(raster.size, [2, 1]);
        assert_eq!(raster.rgba, PIXELS);
        cache.accept(&context, returned, Ok(raster));
        let texture = cache.texture(&path).unwrap();
        assert_eq!(texture.size(), [2, 1]);
        let texture_id = texture.id();
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(WIDTH, HEIGHT),
                )),
                ..Default::default()
            },
            |ui| {
                show_image(ui, cache.texture(&path).unwrap(), "合成.png");
            },
        );
        let painted = output
            .shapes
            .iter()
            .find(|shape| shape.shape.texture_id() == texture_id)
            .map(|shape| shape.shape.visual_bounding_rect());
        let uploaded = output
            .textures_delta
            .set
            .iter()
            .find(|(id, _)| **id == texture_id)
            .unwrap();
        let egui::ImageData::Color(image) = &uploaded.1[0].image;
        let image = image.clone();
        output.textures_delta.clear();
        let painted = painted.expect("preview texture must be painted");
        assert_eq!(painted.size(), egui::vec2(2.0, 1.0));
        assert_eq!(painted.center(), egui::pos2(WIDTH / 2.0, HEIGHT / 2.0));
        assert_eq!(image.size, [2, 1]);
        assert_eq!(
            image.pixels,
            vec![
                egui::Color32::RED,
                egui::Color32::from_rgba_unmultiplied(0, 255, 0, 128)
            ]
        );
        cache.retain(&HashSet::new());
        assert!(cache.texture(&path).is_none());
        let stale = cache.begin(&path, MAX_SIDE).unwrap();
        cache.invalidate(&path);
        cache.accept(
            &context,
            stale.clone(),
            Ok(Raster {
                size: [2, 1],
                rgba: PIXELS.to_vec(),
                animation: None,
            }),
        );
        assert!(cache.texture(&path).is_none());
        let fresh = cache.begin(&path, MAX_SIDE).unwrap();
        assert!(fresh.token > stale.token);
        cache.accept(
            &context,
            stale,
            Ok(Raster {
                size: [2, 1],
                rgba: PIXELS.to_vec(),
                animation: None,
            }),
        );
        assert!(cache.texture(&path).is_none());
        cache.accept(
            &context,
            fresh,
            Ok(Raster {
                size: [2, 1],
                rgba: vec![0],
                animation: None,
            }),
        );
        assert!(cache.error(&path).is_some());
        cache.invalidate(&path);
        let pending = cache.begin(&path, MAX_SIDE).unwrap();
        cache.cancelled(&pending);
        assert!(cache.begin(&path, MAX_SIDE).is_some());
        cache.reset_pending();
        for (path, max_side) in [
            (outside, MAX_SIDE),
            ("relative.png".into(), MAX_SIDE),
            (malformed, MAX_SIDE),
            (path.clone(), 1),
        ] {
            host.submit(HostCommand::ReadPreview(Request {
                path,
                token: 0,
                max_side,
            }))
            .unwrap();
            let response = tokio::time::timeout(TIMEOUT, async {
                loop {
                    if let Some(reply) = host.poll() {
                        return reply;
                    }
                    ready.notified().await;
                }
            })
            .await
            .unwrap();
            assert!(matches!(
                response,
                HostReply::Preview { result: Err(_), .. }
            ));
        }
        state.projects.write().remove(&project);
        assert!(
            taide_native_app::preview::read(
                &services,
                &Request {
                    path: path.clone(),
                    token: 0,
                    max_side: MAX_SIDE
                }
            )
            .await
            .is_err()
        );
        state.begin_shutdown();
        assert!(
            taide_native_app::preview::read(
                &services,
                &Request {
                    path,
                    token: 0,
                    max_side: MAX_SIDE
                }
            )
            .await
            .is_err()
        );
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
    assert!(decode(b"not an image", MAX_SIDE).is_err());
}

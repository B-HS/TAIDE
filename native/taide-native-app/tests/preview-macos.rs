#![cfg(target_os = "macos")]

use std::borrow::Cow;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use image::codecs::{jpeg::JpegEncoder, webp::WebPEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, ImageReader};
use objc2_core_graphics::{CGColorSpace, kCGColorSpaceLinearSRGB};
use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::preview::{Request, decode};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SIDE: usize = 32;
const JPEG_QUALITY: u8 = 100;
const AVIF_QUALITY: f32 = 100.0;
const AVIF_SPEED: u8 = 10;
const PIXEL_TOLERANCE: u8 = 3;
const LINEAR: [u8; 16] = [
    0, 0, 0, 255, 64, 64, 64, 128, 128, 128, 128, 255, 255, 255, 255, 128,
];
const SRGB: [u8; 16] = [
    0, 0, 0, 255, 137, 137, 137, 128, 188, 188, 188, 255, 255, 255, 255, 128,
];
const TIFF_ROTATE_90: [u8; 26] = [
    b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
];

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

fn profile() -> Vec<u8> {
    let space = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceLinearSRGB })).unwrap();
    CGColorSpace::icc_data(Some(&space)).unwrap().to_vec()
}

fn png(profile: Vec<u8>, rotated: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut info = png::Info::with_size(2, 2);
    info.color_type = png::ColorType::Rgba;
    info.bit_depth = png::BitDepth::Eight;
    info.icc_profile = Some(Cow::Owned(profile));
    if rotated {
        info.exif_metadata = Some(Cow::Borrowed(&TIFF_ROTATE_90));
    }
    {
        let mut writer = png::Encoder::with_info(&mut bytes, info)
            .unwrap()
            .write_header()
            .unwrap();
        writer.write_image_data(&LINEAR).unwrap();
        writer.finish().unwrap();
    }
    bytes
}

fn webp(profile: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = WebPEncoder::new_lossless(&mut bytes);
    encoder.set_icc_profile(profile).unwrap();
    encoder
        .write_image(&LINEAR, 2, 2, ExtendedColorType::Rgba8)
        .unwrap();
    bytes
}

fn jpeg(profile: Vec<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY);
    encoder.set_icc_profile(profile).unwrap();
    encoder
        .write_image(&[128; 6], 2, 1, ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

fn avif() -> Vec<u8> {
    let pixels = [ravif::RGBA8::new(255, 255, 255, 128); 2];
    ravif::Encoder::new()
        .with_quality(AVIF_QUALITY)
        .with_alpha_quality(AVIF_QUALITY)
        .with_speed(AVIF_SPEED)
        .with_bit_depth(ravif::BitDepth::Eight)
        .with_num_threads(Some(1))
        .encode_rgba(ravif::Img::new(&pixels[..], 2, 1))
        .unwrap()
        .avif_file
}

fn assert_pixels(actual: &[u8], expected: &[u8], label: &str) {
    assert_eq!(actual.len(), expected.len(), "{label}");
    for (index, (component, expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            component.abs_diff(*expected) <= PIXEL_TOLERANCE,
            "{label} component {index}: {component} != {expected}; actual pixels {actual:?}"
        );
    }
}

#[test]
fn avif_코덱_접근_실패는_빈_pixel_성공으로_보고하지_않는다() {
    match decode(&avif(), MAX_SIDE) {
        Ok(raster) => assert_pixels(
            &raster.rgba,
            &[255, 255, 255, 128, 255, 255, 255, 128],
            "available AVIF codec",
        ),
        Err(AppError::InvalidArgument(message)) => {
            assert_eq!(message, "ImageIO could not materialize decoded pixels");
        }
        result => panic!("unexpected AVIF result: {result:?}"),
    }
}

#[test]
fn 실제_host는_avif와_icc_srgb_alpha_orientation_및_손상을_검사한다() {
    let fixture = Fixture(std::env::temp_dir().join(format!("taide-imageio-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let png_bytes = png(profile(), false);
    let raw = ImageReader::new(Cursor::new(&png_bytes))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();
    assert_eq!(raw.into_rgba8().into_raw(), LINEAR);
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic ImageIO".into(),
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
        for (extension, bytes, expected, size) in [
            ("png", png_bytes, SRGB.to_vec(), [2, 2]),
            ("webp", webp(profile()), SRGB.to_vec(), [2, 2]),
            (
                "jpg",
                jpeg(profile()),
                vec![188, 188, 188, 255, 188, 188, 188, 255],
                [2, 1],
            ),
            (
                "avif",
                avif(),
                vec![255, 255, 255, 128, 255, 255, 255, 128],
                [2, 1],
            ),
        ] {
            let path = root
                .join(format!("synthetic.{extension}"))
                .to_str()
                .unwrap()
                .to_owned();
            std::fs::write(&path, &bytes).unwrap();
            host.submit(HostCommand::ReadPreview(Request {
                path,
                token: 1,
                max_side: MAX_SIDE,
            }))
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
            let HostReply::Preview { result, .. } = reply else {
                panic!("wrong ImageIO reply");
            };
            let raster = result.unwrap();
            assert_eq!(raster.size, size, "{extension}");
            assert_pixels(&raster.rgba, &expected, extension);
            assert!(raster.animation.is_none());
            assert!(decode(&bytes, 1).is_err(), "limit {extension}");
            assert!(
                decode(&bytes[..bytes.len() / 2], MAX_SIDE).is_err(),
                "malformed {extension}"
            );
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
    let raster = decode(&png(profile(), true), MAX_SIDE).unwrap();
    assert_eq!(raster.size, [2, 2]);
    assert_pixels(
        &raster.rgba,
        &[
            188, 188, 188, 255, 0, 0, 0, 255, 255, 255, 255, 128, 137, 137, 137, 128,
        ],
        "EXIF rotate 90",
    );
    let malformed = decode(&png(b"not an ICC profile".to_vec(), false), MAX_SIDE).unwrap();
    assert_pixels(&malformed.rgba, &LINEAR, "invalid ICC fallback");
    let mut unsupported = Cursor::new(Vec::new());
    DynamicImage::new_rgb8(2, 1)
        .write_to(&mut unsupported, ImageFormat::Png)
        .unwrap();
    assert!(decode(unsupported.get_ref(), MAX_SIDE).is_ok());
}

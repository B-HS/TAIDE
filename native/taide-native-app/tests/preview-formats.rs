use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use image::{DynamicImage, ImageFormat, RgbaImage};
use taide_model::app_event::AppEvent;
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
const EXIF_ROTATE_90: [u8; 36] = [
    0xff, 0xe1, 0, 34, b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1,
    3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
];
const WHITE: [u8; 8] = [255; 8];
const RED_HALF_ALPHA: [u8; 8] = [255, 0, 0, 128, 255, 0, 0, 128];
const GREEN: [u8; 8] = [0, 128, 0, 255, 0, 128, 0, 255];
const NESTING_LIMIT: usize = 16;
const LARGE_SIDE: usize = 4096;

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

fn encoded(format: ImageFormat, pixels: &[u8]) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let image = DynamicImage::ImageRgba8(RgbaImage::from_raw(2, 1, pixels.to_vec()).unwrap());
    if format == ImageFormat::Jpeg {
        image.to_rgb8().write_to(&mut bytes, format).unwrap();
    } else {
        image.write_to(&mut bytes, format).unwrap();
    }
    bytes.into_inner()
}

fn embedded_svg(source: &str) -> String {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(source);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="white"/><image href="data:image/svg+xml;base64,{data}" width="2" height="1"/></svg>"#
    )
}

#[test]
fn svg_embedded는_승인된_인라인_raster와_크기_오류_외부entity를_구분한다() {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(encoded(ImageFormat::Png, &WHITE));
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><image href="data:image/png;base64,{data}" width="2" height="1"/></svg>"#
    );
    let raster = decode(svg.as_bytes(), MAX_SIDE).unwrap();
    assert_eq!(raster.size, [2, 1]);
    assert_eq!(raster.rgba, WHITE);
    let limited = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><rect width="1" height="1" fill="red"/><image href="data:image/png;base64,{data}" width="1" height="1"/></svg>"#
    );
    let raster = decode(limited.as_bytes(), 1).unwrap();
    assert_eq!(raster.rgba, [255, 0, 0, 255]);
    let nested = base64::engine::general_purpose::STANDARD.encode(b"<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><rect width='2' height='1' fill='green'/></svg>");
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="white"/><image href="data:image/svg+xml;base64,{nested}" width="2" height="1"/></svg>"#
    );
    assert_eq!(decode(svg.as_bytes(), MAX_SIDE).unwrap().rgba, GREEN);
    assert!(decode(b"<!DOCTYPE svg [<!ENTITY remote SYSTEM 'https://example.invalid/entity'>]><svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><text>&remote;</text></svg>", MAX_SIDE).is_err());

    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-svg-nested-{}", ProjectId::new())));
    std::fs::create_dir_all(&fixture.0).unwrap();
    let foreign = fixture.0.join("outside.png");
    std::fs::write(&foreign, encoded(ImageFormat::Png, &WHITE)).unwrap();
    let source = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="green"/><image href="{}" width="2" height="1"/><image href="file://{}" width="2" height="1"/><image href="https://example.invalid/image.png" width="2" height="1"/><script>throw new Error('not executed')</script></svg>"#,
        foreign.display(),
        foreign.display(),
    );
    let source = embedded_svg(&source);
    assert_eq!(decode(source.as_bytes(), MAX_SIDE).unwrap().rgba, GREEN);
    assert_eq!(
        decode(embedded_svg(&source).as_bytes(), MAX_SIDE)
            .unwrap()
            .rgba,
        GREEN
    );
    let raster_svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><image href="data:image/png;base64,{data}" width="2" height="1"/></svg>"#
    );
    assert_eq!(
        decode(embedded_svg(&raster_svg).as_bytes(), MAX_SIDE)
            .unwrap()
            .rgba,
        WHITE
    );
    let mime_fallback = source.replace("image/svg+xml", "text/plain");
    assert_eq!(
        decode(mime_fallback.as_bytes(), MAX_SIDE).unwrap().rgba,
        GREEN
    );
    let wrong_mime = source.replace("image/svg+xml", "image/png");
    assert_eq!(decode(wrong_mime.as_bytes(), MAX_SIDE).unwrap().rgba, WHITE);
    assert_eq!(
        decode(embedded_svg("<not-svg/>").as_bytes(), MAX_SIDE)
            .unwrap()
            .rgba,
        WHITE
    );
    let entity = "<!DOCTYPE svg [<!ENTITY remote SYSTEM 'https://example.invalid/entity'>]><svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><text>&remote;</text></svg>";
    assert_eq!(
        decode(embedded_svg(entity).as_bytes(), MAX_SIDE)
            .unwrap()
            .rgba,
        WHITE
    );
    let leaf = "<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><rect width='2' height='1' fill='green'/></svg>";
    let mut nested = leaf.to_owned();
    for _ in 0..NESTING_LIMIT {
        nested = embedded_svg(&nested);
    }
    assert_eq!(decode(nested.as_bytes(), MAX_SIDE).unwrap().rgba, GREEN);
    assert_eq!(
        decode(embedded_svg(&nested).as_bytes(), MAX_SIDE)
            .unwrap()
            .rgba,
        WHITE
    );
    let oversized = embedded_svg(leaf).replacen(
        "width=\"2\" height=\"1\"",
        &format!("width=\"{LARGE_SIDE}\" height=\"{LARGE_SIDE}\""),
        1,
    );
    assert!(decode(oversized.as_bytes(), LARGE_SIDE).is_err());
}

#[test]
fn svg_nested_raster와_nested_고유_크기_상한을_확인한다() {
    use base64::Engine;
    let data = base64::engine::general_purpose::STANDARD.encode(encoded(ImageFormat::Png, &WHITE));
    let source = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="green"/><image href="data:image/png;base64,{data}" width="2" height="1"/></svg>"#
    );
    assert_eq!(
        decode(embedded_svg(&source).as_bytes(), MAX_SIDE)
            .unwrap()
            .rgba,
        WHITE
    );
    let leaf = "<svg xmlns='http://www.w3.org/2000/svg' width='2' height='1'><rect width='2' height='1' fill='green'/></svg>";
    let limited =
        embedded_svg(leaf).replacen("width=\"2\" height=\"1\"", "width=\"1\" height=\"1\"", 1);
    let raster = decode(limited.as_bytes(), 1).unwrap();
    assert_eq!(raster.size, [1, 1]);
    assert_eq!(raster.rgba, [255; 4]);
}

#[test]
fn 실제_host는_jpeg_gif_webp_bmp_svg와_exif_및_비신뢰_svg_경계를_읽는다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-preview-formats-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let foreign = fixture.0.join("outside.png");
    std::fs::write(
        &foreign,
        encoded(ImageFormat::Png, &[0, 255, 0, 255, 0, 255, 0, 255]),
    )
    .unwrap();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="1"><rect width="2" height="1" fill="red" opacity="0.5"/><image href="{}" width="2" height="1"/><script>throw new Error('not executed')</script><image href="https://example.invalid/image.png" width="2" height="1"/></svg>"#,
        foreign.display()
    );
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project,
            root: root.to_str().unwrap().into(),
            name: "synthetic formats".into(),
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
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state, tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    runtime.block_on(async {
        for (extension, bytes, expected) in [
            ("jpg", encoded(ImageFormat::Jpeg, &WHITE), WHITE),
            ("gif", encoded(ImageFormat::Gif, &WHITE), WHITE),
            (
                "webp",
                encoded(ImageFormat::WebP, &RED_HALF_ALPHA),
                RED_HALF_ALPHA,
            ),
            ("bmp", encoded(ImageFormat::Bmp, &WHITE), WHITE),
            ("SVG", svg.into_bytes(), RED_HALF_ALPHA),
        ] {
            let path = root
                .join(format!("合成.{extension}"))
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
                panic!("wrong format reply");
            };
            let raster = result.unwrap();
            assert_eq!(raster.size, [2, 1], "{extension}");
            assert_eq!(raster.rgba, expected, "{extension}");
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
    let jpeg = encoded(ImageFormat::Jpeg, &WHITE);
    let rotated = [&jpeg[..2], &EXIF_ROTATE_90, &jpeg[2..]].concat();
    let raster = decode(&rotated, MAX_SIDE).unwrap();
    assert_eq!(raster.size, [1, 2]);
    assert_eq!(raster.rgba, WHITE);
    assert!(
        decode(
            b"<svg xmlns='http://www.w3.org/2000/svg' width='200000' height='200000'/>",
            MAX_SIDE
        )
        .is_err()
    );
    assert!(decode(b"<not-svg/>", MAX_SIDE).is_err());
}

#![cfg(target_os = "macos")]

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Color32, Context, RawInput};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind};
use taide_model::ids::{ProjectId, TabId};
use taide_model::locale::ResolvedLocale;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::preview::{MAX_TEXTURE_BYTES, Raster};
use taide_native_app::preview_pdf::{
    Cache, Failure, INITIAL_ZOOM, MAX_ZOOM, MIN_ZOOM, Page, Request, Selection, decode,
};
use taide_native_app::preview_pdf_surface::{self, Appearance, control_id};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SIDE: usize = 32;
const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 180.0;
const BLUE: [u8; 4] = [0, 0, 255, 255];
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const WHITE: [u8; 4] = [255; 4];

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

fn pdf(pages: &[(&str, &str)]) -> Vec<u8> {
    let kids = pages
        .iter()
        .enumerate()
        .map(|(index, _)| format!("{} 0 R", 3 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!("<< /Type /Pages /Count {} /Kids [{kids}] >>", pages.len()),
    ];
    for (index, (attributes, content)) in pages.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /Resources <<>> {attributes} /Contents {} 0 R >>",
            4 + index * 2
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
    }
    let mut data = "%PDF-1.7\n".to_owned();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(data.len());
        writeln!(data, "{} 0 obj\n{object}\nendobj", index + 1).unwrap();
    }
    let xref = data.len();
    writeln!(data, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).unwrap();
    for offset in offsets {
        writeln!(data, "{offset:010} 00000 n ").unwrap();
    }
    writeln!(
        data,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF",
        objects.len() + 1
    )
    .unwrap();
    data.into_bytes()
}

fn document() -> Vec<u8> {
    pdf(&[
        (
            "/MediaBox [0 0 4 2]",
            "1 0 0 rg 0 0 4 2 re f 0 0 1 rg 0 1 4 1 re f",
        ),
        (
            "/MediaBox [0 0 8 4] /CropBox [2 1 6 3] /UserUnit 2 /Rotate 90",
            "1 1 0 rg 0 0 8 4 re f 0 1 0 rg 2 1 2 2 re f 0 0 1 rg 4 1 2 2 re f",
        ),
        ("/MediaBox [0 0 4.2 2.7]", ""),
    ])
}

async fn reply(host: &mut HostBridge, ready: &Notify, request: Request) -> Result<Page, Failure> {
    host.submit(HostCommand::ReadPdfPreview(request.clone()))
        .unwrap();
    let reply = tokio::time::timeout(TIMEOUT, async {
        let mut saw_source = false;
        loop {
            if let Some(reply) = host.poll() {
                if let HostReply::PdfSourceReady(progress) = reply {
                    assert_eq!(progress, request);
                    assert!(!saw_source, "source progress must arrive once");
                    saw_source = true;
                    continue;
                }
                if let HostReply::PdfPreview { result, .. } = &reply
                    && !matches!(result, Err(Failure::Read(_)))
                {
                    assert!(
                        saw_source,
                        "PDF decode must follow approved source progress"
                    );
                }
                return reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap();
    let HostReply::PdfPreview {
        request: returned,
        result,
    } = reply
    else {
        panic!("wrong PDF reply");
    };
    assert_eq!(returned, request);
    result
}

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: serde_json::from_str(include_str!(
            "../../../crates/taide-locale/resources/locales/en.json"
        ))
        .unwrap(),
        warnings: Vec::new(),
    }
}

fn surface(context: &Context, cache: &mut Cache, tab: &TabId, events: Vec<egui::Event>) -> bool {
    let mut opened = false;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(WIDTH, HEIGHT),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            opened = preview_pdf_surface::show(
                ui,
                cache,
                tab,
                "synthetic.pdf",
                "synthetic.pdf",
                &locale(),
                &Appearance {
                    background: Color32::BLACK,
                    header: Color32::DARK_GRAY,
                    border: Color32::GRAY,
                    foreground: Color32::WHITE,
                    muted: Color32::LIGHT_GRAY,
                },
            );
        },
    );
    let painted = cache.texture(tab).map(|texture| {
        let rect = output
            .shapes
            .iter()
            .find(|shape| shape.shape.texture_id() == texture.id())
            .map(|shape| shape.shape.visual_bounding_rect());
        (rect, texture.size_vec2())
    });
    output.textures_delta.clear();
    if let Some((rect, size)) = painted {
        let rect = rect.expect("the current PDF texture must be painted");
        assert_eq!(rect.size(), size);
        assert_eq!(rect.center().x, WIDTH / 2.0);
    }
    opened
}

fn activate(context: &Context, cache: &mut Cache, tab: &TabId, key: &str) -> bool {
    let id = control_id(egui::ViewportId::ROOT, tab, key);
    context.memory_mut(|memory| memory.request_focus(id));
    let opened = surface(
        context,
        cache,
        tab,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    surface(
        context,
        cache,
        tab,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    opened
}

fn page(pages: usize) -> Page {
    Page::new(
        pages,
        Raster {
            size: [2, 1],
            rgba: [BLUE, RED].concat(),
            animation: None,
        },
    )
}

#[test]
fn 실제_host_pdf의_page_zoom_crop_rotation_snapshot과_승인_stale_close를_검사한다() {
    let fixture = Fixture(std::env::temp_dir().join(format!("taide-pdf-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.pdf").to_str().unwrap().to_owned();
    let bytes = document();
    std::fs::write(&path, &bytes).unwrap();
    let outside = fixture.0.join("outside.pdf").to_str().unwrap().to_owned();
    std::fs::write(&outside, &bytes).unwrap();
    let other = root.join("other.pdf").to_str().unwrap().to_owned();
    std::fs::write(&other, &bytes).unwrap();
    let broken = root.join("broken.pdf").to_str().unwrap().to_owned();
    std::fs::write(&broken, b"not a PDF").unwrap();
    let replacement = pdf(&[("/MediaBox [0 0 1 1]", "0 1 0 rg 0 0 1 1 re f")]);
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic PDF".into(),
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
        services(state.clone(), tasks.clone(), Arc::new(Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    let context = Context::default();
    let mut cache = Cache::default();
    let tab = TabId::new();
    let second_tab = TabId::new();
    runtime.block_on(async {
        let request = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        assert_eq!((request.page, request.zoom), (1, INITIAL_ZOOM));
        assert!(request.snapshot.is_none());
        assert!(cache.begin(&second_tab, &path, MAX_SIDE).is_none());
        let page = reply(&mut host, &ready, request.clone()).await.unwrap();
        assert_eq!(page.total_pages, 3);
        assert_eq!(page.raster.size, [4, 2]);
        assert_eq!(page.raster.rgba, [BLUE.repeat(4), RED.repeat(4)].concat());
        cache.accept(&context, request, Ok(page), 0);
        assert_eq!(cache.retained_bytes(), bytes.len() + 4 * 2 * 4);
        let texture = cache.texture(&tab).unwrap().id();
        let side_request = cache.begin(&second_tab, &path, MAX_SIDE).unwrap();
        assert!(side_request.snapshot.is_some());
        let snapshot = side_request.snapshot.clone();
        let page = reply(&mut host, &ready, side_request.clone())
            .await
            .unwrap();
        cache.accept(&context, side_request, Ok(page), 0);
        assert_eq!(cache.retained_bytes(), bytes.len() + 2 * 4 * 2 * 4);
        assert_eq!(cache.selection(&second_tab), Some(Selection::default()));

        std::fs::write(&path, &replacement).unwrap();
        assert!(cache.change(
            &tab,
            Selection {
                page: 2,
                zoom: INITIAL_ZOOM
            }
        ));
        assert_eq!(cache.texture(&tab).unwrap().id(), texture);
        let request = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        assert_eq!(request.snapshot, snapshot);
        let page = reply(&mut host, &ready, request.clone()).await.unwrap();
        assert_eq!(page.total_pages, 3);
        assert_eq!(page.raster.size, [4, 8]);
        assert_eq!(
            page.raster.rgba,
            [GREEN.repeat(16), BLUE.repeat(16)].concat()
        );
        cache.accept(&context, request.clone(), Ok(page), 0);
        let altered_identity = Request {
            path: other,
            ..request.clone()
        };
        assert!(matches!(
            reply(&mut host, &ready, altered_identity).await,
            Err(Failure::Read(AppError::Forbidden(_)))
        ));
        assert!(cache.change(
            &tab,
            Selection {
                page: 2,
                zoom: INITIAL_ZOOM + 1
            }
        ));
        let zoom_request = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        let page = reply(&mut host, &ready, zoom_request.clone())
            .await
            .unwrap();
        assert_eq!(page.raster.size, [5, 10]);
        assert_eq!(
            page.raster.rgba,
            [GREEN.repeat(25), BLUE.repeat(25)].concat()
        );
        cache.accept(&context, zoom_request, Ok(page), 0);
        assert!(cache.change(
            &tab,
            Selection {
                page: 3,
                zoom: INITIAL_ZOOM
            }
        ));
        let request = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        let page = reply(&mut host, &ready, request.clone()).await.unwrap();
        assert_eq!(page.raster.size, [4, 2]);
        assert_eq!(page.raster.rgba, WHITE.repeat(8));
        cache.accept(&context, request, Ok(page), 0);

        assert!(cache.change(
            &tab,
            Selection {
                page: 2,
                zoom: INITIAL_ZOOM
            }
        ));
        let stale = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        let stale_page = reply(&mut host, &ready, stale.clone()).await.unwrap();
        cache.invalidate(&path);
        assert_eq!(cache.selection(&tab), Some(Selection::default()));
        assert_eq!(cache.selection(&second_tab), Some(Selection::default()));
        assert_eq!(cache.total_pages(&tab), None);
        assert_eq!(cache.retained_bytes(), 0);
        assert!(cache.begin(&tab, &path, MAX_SIDE).is_none());
        cache.accept(&context, stale.clone(), Ok(stale_page), 0);
        assert!(cache.texture(&tab).is_none());
        let fresh = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        assert!(fresh.token > stale.token);
        assert!(fresh.snapshot.is_none());
        let page = reply(&mut host, &ready, fresh.clone()).await.unwrap();
        assert_eq!(page.total_pages, 1);
        assert_eq!(page.raster.size, [1, 1]);
        assert_eq!(page.raster.rgba, GREEN);
        cache.accept(&context, fresh, Ok(page), 0);
        assert_eq!(cache.retained_bytes(), replacement.len() + 4);
        assert!(!cache.change(
            &tab,
            Selection {
                page: 2,
                zoom: INITIAL_ZOOM
            }
        ));

        for (path, is_read) in [
            (outside, true),
            ("synthetic.pdf".into(), true),
            (broken, false),
        ] {
            let request = Request {
                tab: TabId::new(),
                path,
                token: 1,
                page: 1,
                zoom: INITIAL_ZOOM,
                max_side: MAX_SIDE,
                snapshot: None,
            };
            match (reply(&mut host, &ready, request).await, is_read) {
                (Err(Failure::Read(error)), true) => {
                    assert_eq!(error.kind(), AppErrorKind::Forbidden);
                }
                (Err(Failure::Decode(error)), false) => {
                    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
                }
                result => panic!("wrong failure stage: {result:?}"),
            }
        }
        let cached = cache.begin(&second_tab, &path, MAX_SIDE).unwrap();
        state.projects.write().remove(&project);
        let Err(Failure::Read(error)) = reply(&mut host, &ready, cached.clone()).await else {
            panic!("closed project must reject cached PDF snapshot");
        };
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        cache.reconcile(&HashMap::new());
        cache.accept(
            &context,
            cached,
            Err(Failure::Read(AppError::Forbidden("closed".into()))),
            0,
        );
        assert!(cache.selection(&tab).is_none());
        assert!(cache.texture(&second_tab).is_none());
        assert_eq!(cache.retained_bytes(), 0);
        tokio::time::timeout(TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
    assert!(decode(&bytes, 0, INITIAL_ZOOM, MAX_SIDE).is_err());
    assert!(decode(&bytes, 4, INITIAL_ZOOM, MAX_SIDE).is_err());
    assert!(decode(&bytes, 1, MIN_ZOOM - 1, MAX_SIDE).is_err());
    assert!(decode(&bytes, 1, MAX_ZOOM + 1, MAX_SIDE).is_err());
    assert!(decode(&bytes, 1, INITIAL_ZOOM, 1).is_err());
    assert!(
        decode(
            &pdf(&[("/MediaBox [0 0 8192 8192]", "")]),
            1,
            INITIAL_ZOOM,
            8192
        )
        .is_err()
    );
}

#[test]
fn pdf_control의_키보드_페이지_zoom_상한과_실패단계_및_cache_admission을_검사한다() {
    let context = Context::default();
    let mut cache = Cache::default();
    let tab = TabId::new();
    surface(&context, &mut cache, &tab, Vec::new());
    assert!(cache.texture(&tab).is_none());
    let request = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
    assert!(!cache.has_source(&tab));
    cache.source_ready(&request);
    assert!(cache.has_source(&tab));
    surface(&context, &mut cache, &tab, Vec::new());
    cache.accept(&context, request, Ok(page(2)), 0);
    surface(&context, &mut cache, &tab, Vec::new());
    activate(&context, &mut cache, &tab, "preview.pdf.previousPage");
    assert_eq!(cache.selection(&tab), Some(Selection::default()));
    activate(&context, &mut cache, &tab, "preview.pdf.nextPage");
    assert_eq!(cache.selection(&tab).unwrap().page, 2);
    activate(&context, &mut cache, &tab, "preview.pdf.nextPage");
    assert_eq!(cache.selection(&tab).unwrap().page, 2);
    activate(&context, &mut cache, &tab, "preview.pdf.zoomIn");
    assert_eq!(cache.selection(&tab).unwrap().zoom, INITIAL_ZOOM + 1);
    for (bound, key) in [
        (MIN_ZOOM, "preview.pdf.zoomOut"),
        (MAX_ZOOM, "preview.pdf.zoomIn"),
    ] {
        assert!(cache.change(
            &tab,
            Selection {
                page: 2,
                zoom: bound
            }
        ));
        surface(&context, &mut cache, &tab, Vec::new());
        activate(&context, &mut cache, &tab, key);
        assert_eq!(cache.selection(&tab).unwrap().zoom, bound);
    }
    let texture = cache.texture(&tab).unwrap().id();
    let stale = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
    assert!(cache.change(
        &tab,
        Selection {
            page: 1,
            zoom: INITIAL_ZOOM
        }
    ));
    cache.accept(&context, stale.clone(), Ok(page(2)), 0);
    assert_eq!(cache.texture(&tab).unwrap().id(), texture);
    assert!(cache.begin(&tab, "synthetic.pdf", MAX_SIDE).is_none());
    assert!(cache.change(
        &tab,
        Selection {
            page: 1,
            zoom: INITIAL_ZOOM + 1
        }
    ));
    let current = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
    cache.accept(&context, stale, Ok(page(2)), 0);
    cache.accept(&context, current, Ok(page(2)), 0);
    assert_ne!(cache.texture(&tab).unwrap().id(), texture);
    for result in [
        Err(Failure::Read(AppError::Forbidden("denied".into()))),
        Err(Failure::Decode(AppError::InvalidArgument(
            "malformed".into(),
        ))),
    ] {
        cache.invalidate("synthetic.pdf");
        let request = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
        cache.accept(&context, request, result, 0);
        assert!(cache.texture(&tab).is_none());
        assert!(cache.begin(&tab, "synthetic.pdf", MAX_SIDE).is_none());
        surface(&context, &mut cache, &tab, Vec::new());
        assert!(activate(
            &context,
            &mut cache,
            &tab,
            "preview.openExternally"
        ));
    }
    for (page, other_bytes) in [
        (page(0), 0),
        (
            Page::new(
                1,
                Raster {
                    size: [2, 1],
                    rgba: vec![0],
                    animation: None,
                },
            ),
            0,
        ),
        (page(1), MAX_TEXTURE_BYTES),
    ] {
        cache.invalidate("synthetic.pdf");
        let request = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
        cache.accept(&context, request, Ok(page), other_bytes);
        assert!(matches!(cache.error(&tab), Some(Failure::Decode(_))));
        assert_eq!(cache.retained_bytes(), 0);
    }
    cache.invalidate("synthetic.pdf");
    let cancelled = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
    cache.reset_pending();
    let next = cache.begin(&tab, "synthetic.pdf", MAX_SIDE).unwrap();
    assert!(next.token > cancelled.token);
    cache.accept(&context, cancelled, Ok(page(1)), 0);
    assert!(cache.texture(&tab).is_none());
    cache.cancelled(&next);
    cache.invalidate("synthetic.pdf");
    cache.source_ready(&next);
    assert!(!cache.has_source(&tab));
    cache.reconcile(&HashMap::new());
    assert!(cache.selection(&tab).is_none());
}

#[test]
fn pdf_viewbox는_media_crop_교차와_빈교차_fallback_및_소수_canvas_원점을_보존한다() {
    let content = "1 0 0 rg 0 0 8 4 re f 0 0 1 rg 0 1 8 3 re f";
    for (attributes, size, expected) in [
        (
            "/MediaBox [0 0 4 2] /CropBox [2 -1 6 3]",
            [2, 2],
            [BLUE.repeat(2), RED.repeat(2)].concat(),
        ),
        (
            "/MediaBox [0 0 4 2] /CropBox [8 8 10 10]",
            [4, 2],
            [BLUE.repeat(4), RED.repeat(4)].concat(),
        ),
        (
            "/MediaBox [0 0 8 4] /CropBox [6 3 2 1]",
            [4, 2],
            BLUE.repeat(8),
        ),
    ] {
        let page = decode(&pdf(&[(attributes, content)]), 1, INITIAL_ZOOM, MAX_SIDE).unwrap();
        assert_eq!(page.raster.size, size, "{attributes}");
        assert_eq!(page.raster.rgba, expected, "{attributes}");
    }
    let fractional = pdf(&[(
        "/MediaBox [0 0 4.2 2.7]",
        "1 0 0 rg 0 0 4.2 2.7 re f 0 0 1 rg 0 1.7 4.2 1 re f",
    )]);
    let page = decode(&fractional, 1, INITIAL_ZOOM, MAX_SIDE).unwrap();
    assert_eq!(page.raster.size, [4, 2]);
    assert_eq!(page.raster.rgba, [BLUE.repeat(4), RED.repeat(4)].concat());
}

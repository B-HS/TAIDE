use std::collections::HashMap;
use std::io::{Cursor, Write};
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
use taide_native_app::presentation::message;
use taide_native_app::preview::{Failure, MAX_TEXTURE_BYTES};
use taide_native_app::preview_presentation::{Outline, Request, Slide};
use taide_native_app::preview_presentation_cache::Cache;
use taide_native_app::preview_presentation_surface::{self, Appearance};
use taide_native_app::preview_status;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;
use zip::write::SimpleFileOptions;

const TIMEOUT: Duration = Duration::from_secs(5);
const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 360.0;
const PATH: &str = "synthetic.pptx";
const BODY: &str = "A  \tB\n<tag>\u{a0}C";

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

fn document(paragraph: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (index, text) in [(1, paragraph), (2, ""), (3, "last slide")] {
        writer
            .start_file(format!("ppt/slides/slide{index}.xml"), options)
            .unwrap();
        write!(writer, "<a:p><a:t>{text}</a:t></a:p>").unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn outline() -> Outline {
    Outline {
        slides: vec![
            Slide {
                index: 1,
                paragraphs: vec![BODY.into()],
            },
            Slide {
                index: 2,
                paragraphs: Vec::new(),
            },
        ],
    }
}

async fn reply(
    host: &mut HostBridge,
    ready: &Notify,
    request: Request,
    cache: &mut Cache,
) -> Result<Outline, Failure> {
    host.submit(HostCommand::ReadPresentationPreview(request.clone()))
        .unwrap();
    tokio::time::timeout(TIMEOUT, async {
        let mut saw_source = false;
        loop {
            match host.poll() {
                Some(HostReply::PresentationSourceReady(progress)) => {
                    assert_eq!(progress, request);
                    assert!(!saw_source);
                    saw_source = true;
                    cache.source_ready(&progress);
                }
                Some(HostReply::PresentationPreview {
                    request: returned,
                    result,
                }) => {
                    assert_eq!(returned, request);
                    assert_eq!(saw_source, !matches!(result, Err(Failure::Read(_))));
                    return result;
                }
                Some(_) => panic!("unexpected presentation reply"),
                None => ready.notified().await,
            }
        }
    })
    .await
    .unwrap()
}

#[test]
fn 실제_host_pptx의_승인_progress_공유_outline_재해석_stale_close를_검사한다() {
    let fixture = Fixture(std::env::temp_dir().join(format!("taide-pptx-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join(PATH).to_str().unwrap().to_owned();
    let moved = root.join("moved.pptx").to_str().unwrap().to_owned();
    let outside = fixture.0.join("outside.pptx").to_str().unwrap().to_owned();
    let broken = root.join("broken.pptx").to_str().unwrap().to_owned();
    std::fs::write(&path, document("first")).unwrap();
    std::fs::write(&outside, document("outside")).unwrap();
    std::fs::write(&broken, b"not a ZIP").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic presentation".into(),
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
    let mut cache = Cache::default();
    let tab = TabId::new();
    let side = TabId::new();
    runtime.block_on(async {
        let request = cache.begin(&tab, &path).unwrap();
        assert!(cache.begin(&side, &path).is_none());
        assert!(!cache.has_source(&tab));
        let result = reply(&mut host, &ready, request.clone(), &mut cache).await;
        assert!(cache.has_source(&tab));
        cache.accept(request, result, 0);
        assert_eq!(cache.outline(&tab).unwrap().slides.len(), 3);
        assert!(Arc::ptr_eq(
            cache.outline(&tab).unwrap(),
            cache.outline(&side).unwrap()
        ));
        assert_eq!(
            cache.retained_bytes(),
            cache.outline(&tab).unwrap().retained_bytes()
        );
        assert!(cache.select(&tab, 2));
        assert_eq!(cache.selected(&side), Some(0));
        assert!(!cache.select(&tab, 3));
        let old = cache.outline(&tab).unwrap().clone();
        std::fs::write(&path, document("replacement")).unwrap();
        cache.invalidate_root(fixture.0.join("roo").to_str().unwrap());
        assert!(cache.begin(&tab, &path).is_none());
        cache.invalidate(&path);
        assert_eq!(cache.selected(&tab), Some(2));
        let stale = cache.begin(&tab, &path).unwrap();
        let stale_result = reply(&mut host, &ready, stale.clone(), &mut cache).await;
        assert!(Arc::ptr_eq(&old, cache.outline(&tab).unwrap()));
        cache.invalidate(&path);
        cache.accept(stale.clone(), stale_result, 0);
        assert!(Arc::ptr_eq(&old, cache.outline(&tab).unwrap()));
        let current = cache.begin(&tab, &path).unwrap();
        assert!(current.token > stale.token);
        cache.source_ready(&stale);
        assert!(!cache.has_source(&tab));
        let result = reply(&mut host, &ready, current.clone(), &mut cache).await;
        assert_eq!(cache.selected(&tab), Some(2));
        cache.accept(current, result, 0);
        assert_eq!(
            cache.outline(&tab).unwrap().slides[0].paragraphs,
            ["replacement"]
        );
        assert_eq!(cache.selected(&tab), Some(0));
        assert_eq!(cache.selected(&side), Some(0));
        cache.invalidate_root(root.to_str().unwrap());
        let cancelled = cache.begin(&tab, &path).unwrap();
        cache.cancelled(&cancelled);
        let retry = cache.begin(&tab, &path).unwrap();
        cache.accept(cancelled, Ok(outline()), 0);
        assert!(cache.begin(&side, &path).is_none());
        cache.reset_pending();
        cache.source_ready(&retry);
        assert!(!cache.has_source(&tab));
        let moving = cache.begin(&tab, &path).unwrap();
        let result = reply(&mut host, &ready, moving.clone(), &mut cache).await;
        std::fs::rename(&path, &moved).unwrap();
        cache.reconcile(&HashMap::from([(tab.clone(), moved.clone())]));
        cache.accept(moving, result, 0);
        assert!(cache.outline(&tab).is_none());
        assert!(cache.outline(&side).is_none());
        assert_eq!(cache.retained_bytes(), 0);
        let moved_request = cache.begin(&tab, &moved).unwrap();
        let result = reply(&mut host, &ready, moved_request.clone(), &mut cache).await;
        cache.accept(moved_request, result, 0);
        assert_eq!(
            cache.outline(&tab).unwrap().slides[0].paragraphs,
            ["replacement"]
        );
        for (path, expected_read) in [(outside, true), (PATH.into(), true), (broken, false)] {
            let request = Request { path, token: 1 };
            match (
                reply(&mut host, &ready, request, &mut cache).await,
                expected_read,
            ) {
                (Err(Failure::Read(error)), true) => {
                    assert_eq!(error.kind(), AppErrorKind::Forbidden)
                }
                (Err(Failure::Decode(error)), false) => {
                    assert_eq!(error.kind(), AppErrorKind::InvalidArgument)
                }
                result => panic!("wrong failure stage: {result:?}"),
            }
        }
        cache.invalidate_all();
        let closed = cache.begin(&tab, &moved).unwrap();
        state.projects.write().remove(&project);
        let result = reply(&mut host, &ready, closed.clone(), &mut cache).await;
        assert!(
            matches!(&result, Err(Failure::Read(error)) if error.kind() == AppErrorKind::Forbidden)
        );
        cache.reconcile(&HashMap::new());
        cache.accept(closed, result, 0);
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
}

fn locale(id: &str) -> ResolvedLocale {
    let source = match id {
        "ko" => include_str!("../../../crates/taide-locale/resources/locales/ko.json"),
        "ja" => include_str!("../../../crates/taide-locale/resources/locales/ja.json"),
        _ => include_str!("../../../crates/taide-locale/resources/locales/en.json"),
    };
    ResolvedLocale {
        id: id.into(),
        name: id.into(),
        messages: serde_json::from_str(source).unwrap(),
        warnings: Vec::new(),
    }
}

fn surface(
    context: &Context,
    cache: &mut Cache,
    tab: &TabId,
    locale: &ResolvedLocale,
    key: Option<egui::Id>,
) -> (bool, Vec<egui::epaint::TextShape>) {
    let events = if let Some(id) = key {
        context.memory_mut(|memory| memory.request_focus(id));
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }]
    } else {
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Default::default(),
        }]
    };
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
            opened = preview_presentation_surface::show(
                ui,
                cache,
                tab,
                PATH,
                PATH,
                locale,
                &Appearance {
                    status: preview_status::Appearance {
                        background: Color32::BLACK,
                        border: Color32::GRAY,
                        foreground: Color32::WHITE,
                        muted: Color32::LIGHT_GRAY,
                    },
                    border: Color32::GRAY,
                    selected: Color32::BLUE,
                    hover: Color32::DARK_GRAY,
                    warning: Color32::YELLOW,
                },
            );
        },
    );
    let text = output
        .shapes
        .iter()
        .filter_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                Some(text.clone())
            } else {
                None
            }
        })
        .collect();
    output.textures_delta.clear();
    (opened, text)
}

#[test]
fn presentation_surface의_키보드_문단_empty_locale_실패_재해석_admission을_검사한다() {
    for id in ["en", "ko", "ja"] {
        let context = Context::default();
        let mut cache = Cache::default();
        let tab = TabId::new();
        let locale = locale(id);
        let (_, raw) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(raw.is_empty(), "raw read must leave a blank background");
        let request = cache.begin(&tab, PATH).unwrap();
        cache.source_ready(&request);
        let (_, loading) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(
            loading
                .iter()
                .any(|text| text.galley.text() == message(&locale, "common.loading", &[]))
        );
        cache.accept(request, Ok(outline()), 0);
        let (_, ready) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(ready.iter().any(|text| text.galley.text()
            == message(&locale, "preview.presentation.layoutDisclaimer", &[])));
        let body = ready
            .iter()
            .find(|text| text.galley.text() == "A B <tag>\u{a0}C")
            .unwrap();
        assert!(body.pos.x >= 192.0 + 16.0);
        assert_eq!(body.galley.job.sections[0].format.font_id.size, 14.0);
        assert_eq!(body.galley.job.sections[0].format.line_height, Some(20.0));
        let slide_key = preview_presentation_surface::slide_id(egui::ViewportId::ROOT, &tab, 2);
        let (_, selected) = surface(&context, &mut cache, &tab, &locale, Some(slide_key));
        assert_eq!(cache.selected(&tab), Some(1));
        assert!(
            selected
                .iter()
                .any(|text| text.galley.text()
                    == message(&locale, "preview.presentation.noText", &[]))
        );
        surface(&context, &mut cache, &tab, &locale, None);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.source_ready(&request);
        let (_, reparsing) = surface(&context, &mut cache, &tab, &locale, None);
        assert_eq!(cache.selected(&tab), Some(1));
        assert!(
            reparsing
                .iter()
                .any(|text| text.galley.text()
                    == message(&locale, "preview.presentation.noText", &[]))
        );
        cache.accept(
            request,
            Err(Failure::Decode(AppError::InvalidArgument(
                "synthetic".into(),
            ))),
            0,
        );
        let (_, failure) = surface(&context, &mut cache, &tab, &locale, None);
        assert_eq!(cache.retained_bytes(), 0);
        assert!(
            failure.iter().any(|text| text.galley.text()
                == message(&locale, "preview.presentation.loadFailed", &[]))
        );
        let external = preview_status::control_id(
            egui::ViewportId::ROOT,
            &tab,
            preview_status::EXTERNAL_ACTION_KEY,
        );
        assert!(surface(&context, &mut cache, &tab, &locale, Some(external)).0);
        surface(&context, &mut cache, &tab, &locale, None);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.source_ready(&request);
        assert!(matches!(cache.error(&tab), Some(Failure::Decode(_))));
        cache.accept(
            request,
            Err(Failure::Read(AppError::Forbidden("synthetic".into()))),
            0,
        );
        let (_, read_failure) = surface(&context, &mut cache, &tab, &locale, None);
        assert!(
            read_failure
                .iter()
                .any(|text| text.galley.text() == message(&locale, "preview.notSupported", &[]))
        );
        assert!(read_failure.iter().any(|text| text.galley.text() == PATH));
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.source_ready(&request);
        assert!(cache.error(&tab).is_none());
        cache.accept(request, Ok(outline()), MAX_TEXTURE_BYTES);
        assert!(matches!(cache.error(&tab), Some(Failure::Decode(_))));
        assert_eq!(cache.retained_bytes(), 0);
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(
            request,
            Ok(Outline {
                slides: vec![Slide {
                    index: 2,
                    paragraphs: Vec::new(),
                }],
            }),
            0,
        );
        assert!(cache.outline(&tab).is_none());
        cache.invalidate(PATH);
        let request = cache.begin(&tab, PATH).unwrap();
        cache.accept(request, Ok(outline()), 0);
        assert_eq!(cache.selected(&tab), Some(0));
        assert!(cache.error(&tab).is_none());
        surface(&context, &mut cache, &tab, &locale, None);
    }
}

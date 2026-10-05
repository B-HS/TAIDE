use std::{collections::HashMap, sync::Arc, time::Duration};

use eframe::egui::{self, Color32, Context, RawInput};
use rhwp::DocumentCore;
use taide_model::{
    app_event::AppEvent,
    error::{AppError, AppErrorKind},
    ids::{ProjectId, TabId},
    locale::ResolvedLocale,
    paths::AppPaths,
    project::Project,
};
use taide_native_app::{
    bootstrap::services,
    host::{HostBridge, HostCommand, HostReply},
    preview::{MAX_TEXTURE_BYTES, Raster},
    preview_hwp::{Cache, Failure, Page, Request},
    preview_hwp_surface::{self, Appearance, control_id},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const BLANK: &[u8] = include_bytes!("../vendor/rhwp/saved/blank2010.hwp");
const TIMEOUT: Duration = Duration::from_secs(20);
const MAX_SIDE: usize = 4096;
const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 220.0;
const CHANNELS: usize = 4;

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

async fn reply(host: &mut HostBridge, ready: &Notify, request: Request) -> Result<Page, Failure> {
    host.submit(HostCommand::ReadHwpPreview(request.clone()))
        .unwrap();
    tokio::time::timeout(TIMEOUT, async {
        let mut saw_source = false;
        loop {
            if let Some(reply) = host.poll() {
                match reply {
                    HostReply::HwpSourceReady(progress) => {
                        assert_eq!(progress, request);
                        assert!(!saw_source);
                        saw_source = true;
                    }
                    HostReply::HwpPreview {
                        request: returned,
                        result,
                    } => {
                        assert_eq!(returned, request);
                        if !matches!(result, Err(Failure::Read(_))) {
                            assert!(saw_source);
                        }
                        return result;
                    }
                    _ => panic!("wrong HWP reply"),
                }
                continue;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap()
}

#[test]
fn hwp의_실제_host_승인_snapshot_page_교체와_stale_close를_검사한다() {
    let fixture = Fixture(std::env::temp_dir().join(format!("taide-hwp-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.hwpx").to_str().unwrap().to_owned();
    let mut core = DocumentCore::from_bytes(BLANK).unwrap();
    core.insert_text_native(0, 0, 0, "First page").unwrap();
    core.insert_page_break_native(0, 0, "First page".len())
        .unwrap();
    core.insert_text_native(0, 1, 0, "Second page").unwrap();
    let bytes = core.export_hwpx_native().unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let outside = fixture.0.join("outside.hwp").to_str().unwrap().to_owned();
    std::fs::write(&outside, BLANK).unwrap();
    let other = root.join("other.hwp").to_str().unwrap().to_owned();
    std::fs::write(&other, BLANK).unwrap();
    let broken = root.join("broken.hwp").to_str().unwrap().to_owned();
    std::fs::write(&broken, b"not a document").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic HWP".into(),
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
    let second = TabId::new();
    runtime.block_on(async {
        let request = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        assert_eq!(request.page, 0);
        assert!(cache.begin(&second, &path, MAX_SIDE).is_none());
        let page = reply(&mut host, &ready, request.clone()).await.unwrap();
        assert_eq!(page.total_pages, 2);
        let pixels = page.raster.as_ref().unwrap().rgba.len();
        cache.accept(&context, request, Ok(page), 0);
        let source_cost = cache.retained_bytes() - pixels;
        assert!(source_cost > bytes.len());
        let shared = cache.begin(&second, &path, MAX_SIDE).unwrap();
        let snapshot = shared.snapshot.clone();
        assert!(snapshot.is_some());
        let page = reply(&mut host, &ready, shared.clone()).await.unwrap();
        cache.accept(&context, shared, Ok(page), 0);
        assert_eq!(cache.retained_bytes(), source_cost + pixels * 2);
        assert_eq!(cache.selection(&second), Some(0));
        assert!(cache.change(&tab, 1));
        let old_texture = cache.texture(&tab).unwrap().id();
        let request = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        assert_eq!(request.snapshot, snapshot);
        std::fs::write(&path, BLANK).unwrap();
        let page = reply(&mut host, &ready, request.clone()).await.unwrap();
        assert_eq!(page.total_pages, 2);
        cache.accept(&context, request.clone(), Ok(page), 0);
        assert_ne!(cache.texture(&tab).unwrap().id(), old_texture);
        assert_eq!(cache.selection(&tab), Some(1));
        assert_eq!(cache.selection(&second), Some(0));
        assert!(matches!(
            reply(
                &mut host,
                &ready,
                Request {
                    path: other,
                    ..request
                }
            )
            .await,
            Err(Failure::Read(AppError::Forbidden(_)))
        ));
        assert!(cache.change(&tab, 0));
        let stale = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        let page = reply(&mut host, &ready, stale.clone()).await.unwrap();
        cache.invalidate(&path);
        assert_eq!(cache.retained_bytes(), 0);
        assert_eq!(cache.selection(&tab), Some(0));
        assert_eq!(cache.total_pages(&tab), None);
        cache.source_ready(&stale);
        assert!(!cache.has_source(&tab));
        cache.accept(&context, stale, Ok(page), 0);
        assert!(cache.texture(&tab).is_none());
        let fresh = cache.begin(&tab, &path, MAX_SIDE).unwrap();
        assert!(fresh.snapshot.is_none());
        let page = reply(&mut host, &ready, fresh.clone()).await.unwrap();
        assert_eq!(page.total_pages, 1);
        cache.accept(&context, fresh, Ok(page), 0);
        assert!(!cache.change(&tab, 1));
        for (path, is_read) in [
            (outside, true),
            ("synthetic.hwp".into(), true),
            (broken, false),
        ] {
            let request = Request {
                tab: TabId::new(),
                path,
                token: 1,
                page: 0,
                max_side: MAX_SIDE,
                snapshot: None,
            };
            match (reply(&mut host, &ready, request).await, is_read) {
                (Err(Failure::Read(error)), true) => {
                    assert_eq!(error.kind(), AppErrorKind::Forbidden)
                }
                (Err(Failure::Decode(error)), false) => {
                    assert_eq!(error.kind(), AppErrorKind::InvalidArgument)
                }
                result => panic!("wrong failure stage: {result:?}"),
            }
        }
        let pending = cache.begin(&second, &path, MAX_SIDE).unwrap();
        state.projects.write().remove(&project);
        let Err(Failure::Read(error)) = reply(&mut host, &ready, pending.clone()).await else {
            panic!("closed project must reject the cached HWP snapshot");
        };
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        cache.reconcile(&HashMap::new());
        cache.accept(&context, pending, Ok(Page::new(1, None)), 0);
        assert_eq!(cache.retained_bytes(), 0);
        assert!(cache.selection(&tab).is_none());
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
    let json = match id {
        "ko" => include_str!("../../../crates/taide-locale/resources/locales/ko.json"),
        "ja" => include_str!("../../../crates/taide-locale/resources/locales/ja.json"),
        _ => include_str!("../../../crates/taide-locale/resources/locales/en.json"),
    };
    ResolvedLocale {
        id: id.into(),
        name: id.into(),
        messages: serde_json::from_str(json).unwrap(),
        warnings: Vec::new(),
    }
}

fn surface(
    context: &Context,
    cache: &mut Cache,
    tab: &TabId,
    locale: &ResolvedLocale,
    events: Vec<egui::Event>,
) -> bool {
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
            opened = preview_hwp_surface::show(
                ui,
                cache,
                tab,
                "synthetic.hwp",
                "synthetic.hwp",
                locale,
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
    if let Some(texture) = cache.texture(tab) {
        let rect = output
            .shapes
            .iter()
            .find(|shape| shape.shape.texture_id() == texture.id())
            .map(|shape| shape.shape.visual_bounding_rect())
            .expect("current HWP image must be painted");
        assert!(rect.width() <= WIDTH);
        assert_eq!(rect.center().x, WIDTH / 2.0);
        let natural = texture.size_vec2();
        assert!((rect.width() / rect.height() - natural.x / natural.y).abs() < f32::EPSILON);
    }
    output.textures_delta.clear();
    opened
}

fn activate(
    context: &Context,
    cache: &mut Cache,
    tab: &TabId,
    locale: &ResolvedLocale,
    key: &str,
) -> bool {
    context.memory_mut(|memory| memory.request_focus(control_id(egui::ViewportId::ROOT, tab, key)));
    let event = |pressed| egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Default::default(),
    };
    let opened = surface(context, cache, tab, locale, vec![event(true)]);
    surface(context, cache, tab, locale, vec![event(false)]);
    opened
}

fn page(total: usize) -> Page {
    Page::new(
        total,
        Some(Raster {
            size: [2, 1],
            rgba: vec![255; 2 * CHANNELS],
            animation: None,
        }),
    )
}

#[test]
fn hwp_surface의_locale_키보드_빈_page와_cache_상한을_검사한다() {
    let context = Context::default();
    let mut cache = Cache::default();
    let tab = TabId::new();
    for id in ["en", "ko", "ja"] {
        let locale = locale(id);
        surface(&context, &mut cache, &tab, &locale, Vec::new());
        let request = cache.begin(&tab, "synthetic.hwp", MAX_SIDE).unwrap();
        cache.source_ready(&request);
        surface(&context, &mut cache, &tab, &locale, Vec::new());
        cache.accept(&context, request, Ok(page(2)), 0);
        surface(&context, &mut cache, &tab, &locale, Vec::new());
        activate(
            &context,
            &mut cache,
            &tab,
            &locale,
            "preview.hwp.previousPage",
        );
        assert_eq!(cache.selection(&tab), Some(0));
        activate(&context, &mut cache, &tab, &locale, "preview.hwp.nextPage");
        assert_eq!(cache.selection(&tab), Some(1));
        activate(&context, &mut cache, &tab, &locale, "preview.hwp.nextPage");
        assert_eq!(cache.selection(&tab), Some(1));
        let request = cache.begin(&tab, "synthetic.hwp", MAX_SIDE).unwrap();
        cache.accept(&context, request, Ok(Page::new(2, None)), 0);
        assert!(cache.texture(&tab).is_none());
        assert!(cache.begin(&tab, "synthetic.hwp", MAX_SIDE).is_none());
        surface(&context, &mut cache, &tab, &locale, Vec::new());
        cache.invalidate_all();
        let request = cache.begin(&tab, "synthetic.hwp", MAX_SIDE).unwrap();
        cache.accept(&context, request, Ok(Page::new(0, None)), 0);
        assert!(!surface(&context, &mut cache, &tab, &locale, Vec::new()));
        assert!(cache.begin(&tab, "synthetic.hwp", MAX_SIDE).is_none());
        assert!(!activate(
            &context,
            &mut cache,
            &tab,
            &locale,
            "preview.openExternally"
        ));
        cache.invalidate_all();
        let request = cache.begin(&tab, "synthetic.hwp", MAX_SIDE).unwrap();
        cache.accept(
            &context,
            request,
            Err(Failure::Decode(AppError::InvalidArgument(
                "synthetic".into(),
            ))),
            0,
        );
        assert!(activate(
            &context,
            &mut cache,
            &tab,
            &locale,
            "preview.openExternally"
        ));
        cache.invalidate_all();
    }
    for other_bytes in [MAX_TEXTURE_BYTES - 2 * CHANNELS, MAX_TEXTURE_BYTES] {
        let request = cache.begin(&tab, "synthetic.hwp", MAX_SIDE).unwrap();
        cache.accept(&context, request, Ok(page(1)), other_bytes);
        assert_eq!(
            cache.error(&tab).is_some(),
            other_bytes == MAX_TEXTURE_BYTES
        );
        cache.invalidate_all();
    }
    let request = cache.begin(&tab, "synthetic.hwp", MAX_SIDE).unwrap();
    cache.accept(&context, request, Ok(Page::new(0, page(1).raster)), 0);
    assert!(cache.error(&tab).is_some());
}

use super::*;
use std::io::Cursor;
use std::sync::Arc;
use taide_model::{app_event::AppEvent, paths::AppPaths, project::Project};
use taide_native_editor::documentation::{Content, ImageDimensions};
use taide_runtime::EventSink;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const DEADLINE: Duration = Duration::from_secs(5);
const MAX_SIDE: usize = 64;
const WIDTH: f32 = 40.0;
const HEIGHT: f32 = 20.0;
const PIXELS: [u8; 8] = [255, 0, 0, 255, 0, 255, 0, 128];
const BYTE_LIMIT: usize = 4096;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    project: ProjectId,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-documentation-images-{}", ProjectId::new()));
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic documentation images".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        );
        Self {
            directory,
            services,
            project,
            root,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn png() -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_raw(2, 1, PIXELS.to_vec()).unwrap())
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

fn image_document(source: &str) -> RichDocument {
    crate::editor_markup::parse(&Content::Markdown(format!(
        "[![diagram](<{source}|width=40 height=20> \"title\")](https://example.com/docs)"
    )))
}

async fn settle(cache: &mut Cache, context: &Context, fixture: &Fixture, document: &RichDocument) {
    tokio::time::timeout(DEADLINE, async {
        loop {
            cache.prepare(
                context,
                &fixture.services,
                std::iter::once((&fixture.project, document)),
            );
            if cache
                .entries
                .values()
                .all(|entry| !matches!(entry, Entry::Loading(_) | Entry::Queued))
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn drained(tasks: &TaskSupervisor) {
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn 이미지_data_uri는_base64와_percent_svg_및_폭높이를_보존하고_잘못된_형식을_거절한다() {
    let bytes = png();
    let source = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    );
    assert_eq!(data_bytes(&source).unwrap(), bytes);
    assert_eq!(
        crate::preview::decode(&bytes, MAX_SIDE).unwrap().rgba,
        PIXELS
    );
    assert!(crate::preview::decode(&bytes, 1).is_err());
    let svg = "data:image/svg+xml,%3Csvg%20xmlns=%22http://www.w3.org/2000/svg%22%20width=%222%22%20height=%221%22%3E%3C/svg%3E";
    assert_eq!(
        crate::preview::decode(&data_bytes(svg).unwrap(), MAX_SIDE)
            .unwrap()
            .size,
        [2, 1]
    );
    for invalid in [
        "data:text/plain,hello",
        "data:image/png;base64,?",
        "data:image/png,%",
        "data:image/png,%0x",
        "javascript:alert(0)",
    ] {
        assert!(data_bytes(invalid).is_err(), "{invalid}");
    }
    assert!(check_bytes(taide_model::file::READ_ONLY_FILE_BYTES as usize + 1).is_err());
    let document = image_document(&source);
    let Block::Paragraph(inlines) = &document.blocks[0] else {
        panic!()
    };
    assert!(
        matches!(&inlines[0], Inline::Image { source: actual, dimensions, title, link: Some(link), .. }
        if actual == &source && dimensions == &ImageDimensions { width: Some(WIDTH as u32), height: Some(HEIGHT as u32) }
        && title == "title" && link.target == "https://example.com/docs")
    );
}

#[tokio::test]
async fn 실제_도움말_이미지는_root_격리와_중복_표시_닫힘_및_project_task_회수를_보존한다() {
    let fixture = Fixture::new();
    let context = Context::default();
    let bytes = png();
    let path = fixture.root.join("synthetic.png");
    let outside = fixture.directory.join("outside.png");
    std::fs::write(&path, &bytes).unwrap();
    std::fs::write(&outside, &bytes).unwrap();
    let source = url::Url::from_file_path(&path).unwrap().to_string();
    let document = image_document(&source);
    let mut cache = Cache::default();
    settle(&mut cache, &context, &fixture, &document).await;
    assert_eq!(cache.entries.len(), 1);
    let Entry::Ready { texture, .. } = cache.entries.values().next().unwrap() else {
        panic!()
    };
    let texture_id = texture.id();
    cache.prepare(
        &context,
        &fixture.services,
        [(&fixture.project, &document), (&fixture.project, &document)].into_iter(),
    );
    assert_eq!(cache.entries.len(), 1);
    let mut rect = None;
    let mut output = context.run_ui(egui::RawInput::default(), |ui| {
        rect = Some(
            cache
                .show(
                    ui,
                    &fixture.project,
                    &source,
                    "diagram",
                    ImageDimensions {
                        width: Some(WIDTH as u32),
                        height: Some(HEIGHT as u32),
                    },
                )
                .unwrap()
                .rect,
        );
    });
    let primitives = context.tessellate(output.shapes.clone(), output.pixels_per_point);
    output.textures_delta.clear();
    assert_eq!(rect.unwrap().size(), egui::vec2(WIDTH, HEIGHT));
    assert!(primitives.iter().any(|shape| matches!(&shape.primitive, egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == texture_id)));
    for source in [
        url::Url::from_file_path(&outside).unwrap().to_string(),
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(b"invalid")
        ),
    ] {
        settle(&mut cache, &context, &fixture, &image_document(&source)).await;
        assert!(matches!(
            cache.entries.values().next().unwrap(),
            Entry::Failed
        ));
    }
    #[cfg(unix)]
    {
        let escape = fixture.root.join("escape.png");
        std::os::unix::fs::symlink(&outside, &escape).unwrap();
        settle(
            &mut cache,
            &context,
            &fixture,
            &image_document(url::Url::from_file_path(escape).unwrap().as_str()),
        )
        .await;
        assert!(matches!(
            cache.entries.values().next().unwrap(),
            Entry::Failed
        ));
    }
    fixture
        .services
        .state
        .projects
        .write()
        .remove(&fixture.project);
    cache.prepare(
        &context,
        &fixture.services,
        std::iter::once((&fixture.project, &document)),
    );
    assert!(cache.entries.is_empty());
    drained(&fixture.services.tasks).await;
}

#[tokio::test]
async fn 실제_http_도움말_이미지는_픽셀을_표시하고_닫힌_응답대기를_취소한다() {
    let fixture = Fixture::new();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let pending = Arc::new(tokio::sync::Notify::new());
    let signal = pending.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; BYTE_LIMIT];
        stream.read(&mut request).await.unwrap();
        let bytes = png();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        stream.write_all(&bytes).await.unwrap();
        drop(stream);
        let (mut stream, _) = listener.accept().await.unwrap();
        stream.read(&mut request).await.unwrap();
        signal.notify_one();
        assert_eq!(
            tokio::time::timeout(DEADLINE, stream.read(&mut request))
                .await
                .unwrap()
                .unwrap(),
            0
        );
    });
    let context = Context::default();
    let mut cache = Cache::default();
    let source = format!("http://{address}/synthetic.png");
    settle(&mut cache, &context, &fixture, &image_document(&source)).await;
    assert!(matches!(
        cache.entries.values().next().unwrap(),
        Entry::Ready { .. }
    ));
    let mut store =
        taide_native_editor::store::EditorStore::new(taide_native_editor::store::EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
    let document = store
        .open_untitled(taide_model::ids::TabId::new(), "method", "rust".into())
        .unwrap();
    let view = store
        .attach_view(
            taide_native_editor::view::ViewKey {
                window: "synthetic".into(),
                pane: taide_model::ids::PaneId::new(),
                tab: taide_model::ids::TabId::new(),
            },
            document,
        )
        .unwrap();
    let provider = crate::editor_symbols::ProviderIdentity {
        owner: crate::diagnostics::Owner::new(),
        generation: 1,
        capability_revision: 0,
    };
    let providers = HashSet::from([provider]);
    let mut state = crate::editor_documentation::State::default();
    let request = state
        .begin(
            &store,
            crate::editor_documentation::Context {
                project: fixture.project.clone(),
                source: view,
                owner: view,
                kind: taide_native_editor::documentation::Kind::Hover,
                byte: 0,
                fallback: 0..1,
                viewport: egui::ViewportId::ROOT,
                keyboard: false,
            },
            providers.clone(),
        )
        .unwrap();
    state
        .accept(
            &store,
            &request,
            providers,
            Ok(crate::editor_documentation::Response::Hover {
                part: Some((
                    0,
                    crate::editor_documentation::HoverGroup {
                        provider,
                        part: taide_native_editor::documentation::HoverPart {
                            range: taide_native_editor::lsp::LspRange::new(
                                taide_native_editor::lsp::Position::new(0, 0),
                                taide_native_editor::lsp::Position::new(0, 1),
                            ),
                            contents: vec![Content::Markdown(format!(
                                "![pending](http://{address}/pending.png)"
                            ))],
                        },
                    },
                )),
                complete: true,
            }),
        )
        .unwrap();
    state.prepare_images(&store, &context, &fixture.services);
    tokio::time::timeout(DEADLINE, pending.notified())
        .await
        .unwrap();
    state.close(view, taide_native_editor::documentation::Kind::Hover);
    assert!(request.is_cancelled());
    tokio::time::timeout(DEADLINE, server)
        .await
        .unwrap()
        .unwrap();
    drained(&fixture.services.tasks).await;
}

#[test]
fn 도움말_애니메이션은_기존_playback과_전체_텍스처_상한을_사용한다() {
    let context = Context::default();
    let Entry::Ready {
        mut playback,
        bytes,
        ..
    } = ready(
        &context,
        Raster {
            size: [2, 1],
            rgba: PIXELS.to_vec(),
            animation: Some(crate::preview::Animation {
                first_delay: Duration::from_secs(1),
                frames: vec![crate::preview::Frame {
                    rgba: PIXELS.iter().rev().copied().collect(),
                    delay: Duration::from_secs(1),
                }],
                plays: None,
            }),
        },
        0,
    )
    .unwrap()
    else {
        panic!()
    };
    assert_eq!(bytes, PIXELS.len() * 3);
    let playback = playback.as_mut().unwrap();
    assert_eq!(playback.step(Duration::ZERO).0, 0);
    assert_eq!(playback.step(Duration::from_secs(1)).0, 1);
    assert!(
        ready(
            &context,
            Raster {
                size: [2, 1],
                rgba: PIXELS.to_vec(),
                animation: None
            },
            MAX_TEXTURE_BYTES
        )
        .is_err()
    );
}

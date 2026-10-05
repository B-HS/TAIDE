#![cfg(unix)]

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use dom_query::Document;
use taide_model::{
    app_event::AppEvent, file::READ_ONLY_FILE_BYTES, ids::ProjectId, paths::AppPaths,
    project::Project,
};
use taide_native_app::{
    bootstrap::services,
    preview_web::{Prepared, Request},
    preview_web_cache::Cache,
    preview_web_document::source_url,
    preview_web_host::{Bridge, Reply},
    preview_web_http::Limits,
    preview_web_media::{Appearance, MAX_DOCUMENT_BYTES},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use url::Url;

const TIMEOUT: Duration = Duration::from_secs(20);
const RESPONSE_BYTES: u64 = 32 * 1024;
const MEDIA_MULTIPLIER: u64 = 3;
const CONNECTIONS: usize = 2;
const SOURCES: usize = 4;
const AUDIO_BYTES: &[u8] = b"synthetic audio bytes";

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

fn project(state: &AppState, root: &Path) -> ProjectId {
    let id = ProjectId::new();
    state.projects.write().insert(
        id.clone(),
        Project {
            id: id.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic media".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    id
}

async fn response(url: &Url, method: &str, extra: &str) -> (String, Vec<u8>) {
    tokio::time::timeout(TIMEOUT, async {
        let address = SocketAddr::new(
            url.host_str().unwrap().parse().unwrap(),
            url.port().unwrap(),
        );
        let mut socket = TcpStream::connect(address).await.unwrap();
        socket
            .write_all(
                format!(
                    "{method} {} HTTP/1.1\r\nHost: {address}\r\n{extra}\r\n",
                    url.path()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut bytes = Vec::new();
        socket
            .take(RESPONSE_BYTES)
            .read_to_end(&mut bytes)
            .await
            .unwrap();
        let separator = bytes
            .windows(b"\r\n\r\n".len())
            .position(|part| part == b"\r\n\r\n")
            .unwrap();
        let body = bytes.split_off(separator + b"\r\n\r\n".len());
        (
            String::from_utf8(bytes[..separator].to_vec())
                .unwrap()
                .to_ascii_lowercase(),
            body,
        )
    })
    .await
    .unwrap()
}

async fn prepared(bridge: &mut Bridge, expected: &Request) -> Prepared {
    tokio::time::timeout(TIMEOUT, async {
        let mut source_ready = false;
        loop {
            match bridge.poll() {
                Some(Reply::SourceReady(request)) => {
                    assert_eq!(&request, expected);
                    assert!(!source_ready);
                    source_ready = true;
                }
                Some(Reply::Prepared { request, result }) => {
                    assert_eq!(&request, expected);
                    assert!(source_ready);
                    return result.unwrap();
                }
                None => tokio::task::yield_now().await,
            }
        }
    })
    .await
    .unwrap()
}

fn asset(document: &Prepared, selector: &str) -> Url {
    let dom = Document::from(document.html.as_str());
    let url = Url::parse(dom.select_single(selector).attr("src").unwrap().as_ref()).unwrap();
    assert_ne!(url.path(), document.source.path());
    assert_eq!(url.host_str(), document.source.host_str());
    assert_eq!(url.port(), document.source.port());
    assert_eq!(
        url.path_segments().unwrap().next(),
        document.source.path_segments().unwrap().next()
    );
    url
}

#[tokio::test(flavor = "multi_thread")]
async fn media_host는_helper_없이_문서와_range_theme_cache_인가와_회수를_연결한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-web-media-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    let other = fixture.0.join("other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let audio = root.join("한글 # ? & <synthetic>.wav");
    let video = root.join("large.mp4");
    let sibling = root.join("sibling.mp3");
    let outside = other.join("outside.mp3");
    std::fs::write(&audio, AUDIO_BYTES).unwrap();
    std::fs::write(&sibling, b"sibling synthetic audio").unwrap();
    std::fs::write(&outside, b"outside synthetic audio").unwrap();
    let size = READ_ONLY_FILE_BYTES * MEDIA_MULTIPLIER;
    std::fs::File::create(&video)
        .unwrap()
        .set_len(size)
        .unwrap();
    let audio_path = audio.to_str().unwrap();
    let video_path = video.to_str().unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let root_id = project(&state, &root);
    project(&state, &other);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
    let missing_helper = fixture.0.join("does-not-exist-helper");
    assert!(!missing_helper.exists());
    let mut bridge = Bridge::connect_lazy(
        services,
        Arc::new(|| {}),
        missing_helper,
        TIMEOUT,
        Limits {
            connections: CONNECTIONS,
            sources: SOURCES,
            header_timeout: TIMEOUT,
            idle_timeout: TIMEOUT,
        },
    )
    .unwrap();
    let appearance = Appearance {
        background: [0, 1, 0, 1],
        foreground: [1, 0, 0, 1],
        muted: [0, 0, 1, 1],
    };
    assert!(bridge.set_media_appearance(appearance));
    assert!(!bridge.set_media_appearance(appearance));
    let mut cache = Cache::default();
    let command = cache.begin(audio_path).unwrap();
    bridge.submit(command.clone()).unwrap();
    let audio_document = prepared(&mut bridge, &command).await;
    let audio_scope = audio_document.owner.scope();
    let audio_url = audio_document.source.clone();
    let audio_asset = asset(&audio_document, "audio");
    assert!(audio_document.html.len() < MAX_DOCUMENT_BYTES);
    let (headers, body) = response(&audio_url, "GET", "").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains("content-type: text/html; charset=utf-8"));
    assert!(headers.contains("media-src 'self'"));
    assert!(headers.contains("script-src 'none'"));
    assert_eq!(body, audio_document.html.as_bytes());
    let dom = Document::from(std::str::from_utf8(&body).unwrap());
    assert_eq!(
        dom.select_single("span").text().as_ref(),
        audio.file_name().unwrap().to_str().unwrap()
    );
    assert!(
        dom.select_single("style")
            .text()
            .contains("background:#00010001")
    );
    let (headers, body) = response(&audio_asset, "GET", "Range: bytes=0-2\r\n").await;
    assert!(headers.starts_with("http/1.1 206"));
    assert!(headers.contains("content-type: audio/wav"));
    assert_eq!(body, &AUDIO_BYTES[..3]);
    cache.source_ready(&command);
    cache.accept(command, Ok(audio_document), 0);
    let command = cache.begin(video_path).unwrap();
    bridge.submit(command.clone()).unwrap();
    let video_document = prepared(&mut bridge, &command).await;
    let video_scope = video_document.owner.scope();
    let video_asset = asset(&video_document, "video");
    assert!(video_document.html.len() < MAX_DOCUMENT_BYTES);
    let (headers, body) = response(&video_asset, "HEAD", "").await;
    assert!(headers.starts_with("http/1.1 200"));
    assert!(headers.contains(&format!("content-length: {size}")));
    assert!(headers.contains("content-type: video/mp4"));
    assert!(body.is_empty());
    let (headers, body) = response(&video_asset, "GET", "Range: bytes=-1\r\n").await;
    assert!(headers.starts_with("http/1.1 206"));
    assert_eq!(body, &[0]);
    let outside_url = video_document
        .ticket
        .as_ref()
        .unwrap()
        .url(&source_url(&outside).unwrap())
        .unwrap();
    assert!(
        response(&outside_url, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    cache.source_ready(&command);
    cache.accept(command, Ok(video_document), 0);
    let html_path = root.join("synthetic.html");
    let html_request = cache.begin(html_path.to_str().unwrap()).unwrap();
    let changed = Appearance {
        background: [1, 1, 0, 1],
        ..appearance
    };
    assert!(bridge.set_media_appearance(changed));
    cache.invalidate_media();
    assert_eq!(cache.active_request(), Some(&html_request));
    assert!(audio_scope.is_closed());
    assert!(video_scope.is_closed());
    assert!(
        response(&audio_url, "GET", "")
            .await
            .0
            .starts_with("http/1.1 404")
    );
    assert!(
        response(&video_asset, "GET", "")
            .await
            .0
            .starts_with("http/1.1 404")
    );
    cache.cancelled(&html_request);
    let command = cache.begin(audio_path).unwrap();
    bridge.submit(command.clone()).unwrap();
    let changed_document = prepared(&mut bridge, &command).await;
    assert!(changed_document.html.contains("background:#01010001"));
    state.projects.write().remove(&root_id);
    assert!(
        response(&changed_document.source, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    drop(changed_document);
    cache.cancelled(&command);
    state.projects.write().clear();
    state.authorize_cli_opened_path(&audio);
    let command = cache.begin(audio_path).unwrap();
    bridge.submit(command.clone()).unwrap();
    let cli_document = prepared(&mut bridge, &command).await;
    let cli_asset = asset(&cli_document, "audio");
    assert_eq!(response(&cli_asset, "GET", "").await.1, AUDIO_BYTES);
    let sibling_url = cli_document
        .ticket
        .as_ref()
        .unwrap()
        .url(&source_url(&sibling).unwrap())
        .unwrap();
    assert!(
        response(&sibling_url, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    std::fs::write(&audio, b"changed synthetic audio").unwrap();
    assert!(
        response(&cli_document.source, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    assert!(
        response(&cli_asset, "GET", "")
            .await
            .0
            .starts_with("http/1.1 403")
    );
    let address = SocketAddr::new(
        cli_asset.host_str().unwrap().parse().unwrap(),
        cli_asset.port().unwrap(),
    );
    drop(cli_document);
    bridge.disconnect().await.unwrap();
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(TcpStream::connect(address).await.is_err());
}

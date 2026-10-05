#![cfg(unix)]

use std::{
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use taide_model::{
    app_event::AppEvent, file::READ_ONLY_FILE_BYTES, ids::ProjectId, paths::AppPaths,
    project::Project,
};
use taide_native_app::{
    bootstrap::services,
    preview_web::{Request, read},
    preview_web_document::source_url,
    preview_web_range::Selection,
    preview_web_resource::{CHUNK_BYTES, media_owner},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const TIMEOUT: Duration = Duration::from_secs(20);
const MEDIA_MULTIPLIER: u64 = 3;

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
            name: "synthetic resource".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    id
}

#[tokio::test(flavor = "multi_thread")]
async fn web_resource는_source_root_cli_range_큰_media와_owner_종료를_검사한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-web-resource-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    let other = fixture.0.join("other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    let html = root.join("source.html");
    std::fs::write(&html, b"<link rel='stylesheet' href='space%20%23%3F.css'>").unwrap();
    let css = root.join("space #?.css");
    std::fs::write(&css, b"abcdefghij").unwrap();
    let outside = other.join("outside.png");
    std::fs::write(&outside, b"outside synthetic bytes").unwrap();
    let disallowed = root.join("private.synthetic");
    std::fs::write(&disallowed, b"private synthetic bytes").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let root_id = project(&state, &root);
    project(&state, &other);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
    let prepared = read(
        &services,
        &Request {
            path: html.to_str().unwrap().into(),
            token: 1,
        },
        PathBuf::from(EXECUTABLE),
        TIMEOUT,
        || {},
        |_| {},
    )
    .await
    .unwrap();
    let scope = prepared.owner.scope();
    let mut url = source_url(&css).unwrap();
    url.set_query(Some("v=1"));
    let mut response = scope.open(&url, Some("bytes=2-4")).unwrap();
    assert_eq!(response.mime, "text/css; charset=utf-8");
    assert_eq!(response.content_length(), 3);
    assert_eq!(response.content_range().as_deref(), Some("bytes 2-4/10"));
    let body = response.body.as_mut().unwrap();
    assert_eq!(body.next_chunk().unwrap().unwrap(), b"cde");
    assert!(body.next_chunk().unwrap().is_none());
    let mut suffix = scope.open(&url, Some("bytes=-3")).unwrap();
    assert_eq!(
        suffix.body.as_mut().unwrap().next_chunk().unwrap().unwrap(),
        b"hij"
    );
    let unsatisfied = scope.open(&url, Some("bytes=10-")).unwrap();
    assert_eq!(unsatisfied.selection, Selection::Unsatisfiable);
    assert_eq!(unsatisfied.content_range().as_deref(), Some("bytes */10"));
    assert!(unsatisfied.body.is_none());
    assert!(scope.open(&source_url(&outside).unwrap(), None).is_err());
    assert!(scope.open(&source_url(&disallowed).unwrap(), None).is_err());
    assert!(
        scope
            .open(
                &url::Url::parse("https://example.invalid/image.png").unwrap(),
                None
            )
            .is_err()
    );
    let alias = root.join("outside-alias.png");
    std::os::unix::fs::symlink(&outside, &alias).unwrap();
    assert!(scope.open(&source_url(&alias).unwrap(), None).is_err());
    let mut in_flight = scope.open(&url, None).unwrap().body.unwrap();
    drop(prepared);
    assert!(scope.check().is_err());
    assert!(in_flight.next_chunk().is_err());

    let media = root.join("large.mp4");
    let size = READ_ONLY_FILE_BYTES * MEDIA_MULTIPLIER;
    let mut file = std::fs::File::create(&media).unwrap();
    file.set_len(size).unwrap();
    file.write_all(b"HEAD").unwrap();
    file.seek(SeekFrom::End(-4)).unwrap();
    file.write_all(b"TAIL").unwrap();
    drop(file);
    let owner = media_owner(&services, media.to_str().unwrap().into())
        .await
        .unwrap();
    let scope = owner.scope();
    let url = source_url(&media).unwrap();
    let mut full = scope.open(&url, None).unwrap();
    assert_eq!(full.content_length(), size);
    assert_eq!(full.mime, "video/mp4");
    let mut total = 0;
    while let Some(chunk) = full.body.as_mut().unwrap().next_chunk().unwrap() {
        assert!(chunk.len() <= CHUNK_BYTES);
        assert!(chunk.capacity() <= CHUNK_BYTES);
        if total == 0 {
            assert_eq!(&chunk[..4], b"HEAD");
        }
        total += chunk.len() as u64;
        if total == size {
            assert_eq!(&chunk[chunk.len() - 4..], b"TAIL");
        }
    }
    assert_eq!(total, size);
    let mut tail = scope.open(&url, Some("bytes=-4")).unwrap();
    assert_eq!(
        tail.body.as_mut().unwrap().next_chunk().unwrap().unwrap(),
        b"TAIL"
    );
    let mut changed = scope.open(&url, Some("bytes=0-3")).unwrap().body.unwrap();
    state.projects.write().remove(&root_id);
    assert!(changed.next_chunk().is_err());

    let cli = other.join("cli.mp3");
    std::fs::write(&cli, b"CLI audio bytes").unwrap();
    state.projects.write().clear();
    state.authorize_cli_opened_path(&cli);
    let owner = media_owner(&services, cli.to_str().unwrap().into())
        .await
        .unwrap();
    let scope = owner.scope();
    assert!(scope.open(&source_url(&cli).unwrap(), None).is_ok());
    assert!(scope.open(&source_url(&outside).unwrap(), None).is_err());
    std::fs::write(&cli, b"modified CLI audio bytes").unwrap();
    assert!(scope.check().is_err());
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
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
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const TIMEOUT: Duration = Duration::from_secs(20);

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

#[tokio::test(flavor = "multi_thread")]
async fn html_read는_실제_root_cli_승인과_helper_뒤_변경을_검사한다() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("taide-html-read-{}", ProjectId::new())));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("synthetic.html");
    std::fs::write(&source, b"<img src='asset.png'><p>approved root").unwrap();
    let outside = fixture.0.join("outside.html");
    std::fs::write(&outside, b"<p>CLI source").unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let project = ProjectId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().into(),
            name: "synthetic HTML".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
    let request = Request {
        path: source.to_str().unwrap().into(),
        token: 1,
    };
    let ready = Arc::new(AtomicBool::new(false));
    let progress = ready.clone();
    let prepared = read(
        &services,
        &request,
        PathBuf::from(EXECUTABLE),
        TIMEOUT,
        move || progress.store(true, Ordering::Release),
        |_| {},
    )
    .await
    .unwrap();
    assert!(ready.load(Ordering::Acquire));
    assert_eq!(prepared.canonical, std::fs::canonicalize(&source).unwrap());
    assert_eq!(prepared.source, source_url(&prepared.canonical).unwrap());
    assert!(prepared.html.contains("<p>approved root</p>"));
    assert_eq!(tasks.tracked_count(), 0);

    let cli = Request {
        path: outside.to_str().unwrap().into(),
        token: 1,
    };
    assert!(
        read(
            &services,
            &cli,
            PathBuf::from(EXECUTABLE),
            TIMEOUT,
            || panic!("denied file must not be read"),
            |_| panic!("denied file must not spawn")
        )
        .await
        .is_err()
    );
    state.authorize_cli_opened_path(&outside);
    let prepared = read(
        &services,
        &cli,
        PathBuf::from(EXECUTABLE),
        TIMEOUT,
        || {},
        |_| {},
    )
    .await
    .unwrap();
    assert!(prepared.html.contains("<p>CLI source</p>"));
    let changed = state.clone();
    assert!(
        read(
            &services,
            &cli,
            PathBuf::from(EXECUTABLE),
            TIMEOUT,
            || {},
            move |_| changed.cli_opened_paths.write().clear()
        )
        .await
        .is_err()
    );

    let oversized = root.join("oversized.html");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(READ_ONLY_FILE_BYTES + 1)
        .unwrap();
    assert!(
        read(
            &services,
            &Request {
                path: oversized.to_str().unwrap().into(),
                token: 1
            },
            PathBuf::from(EXECUTABLE),
            TIMEOUT,
            || panic!("oversized must not be ready"),
            |_| panic!("oversized must not spawn")
        )
        .await
        .is_err()
    );

    #[cfg(unix)]
    {
        let alias = root.join("alias.html");
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        let original = alias.clone();
        let replacement = outside.clone();
        assert!(
            read(
                &services,
                &Request {
                    path: alias.to_str().unwrap().into(),
                    token: 1
                },
                PathBuf::from(EXECUTABLE),
                TIMEOUT,
                || {},
                move |_| {
                    std::fs::remove_file(&original).unwrap();
                    std::os::unix::fs::symlink(&replacement, &original).unwrap();
                }
            )
            .await
            .is_err()
        );
    }

    let changed = state.clone();
    assert!(
        read(
            &services,
            &request,
            PathBuf::from(EXECUTABLE),
            TIMEOUT,
            || {},
            move |_| {
                changed.projects.write().remove(&project);
            }
        )
        .await
        .is_err()
    );
    state.authorize_cli_opened_path(&source);
    let changed = state.clone();
    assert!(
        read(
            &services,
            &request,
            PathBuf::from(EXECUTABLE),
            TIMEOUT,
            || {},
            move |_| changed.begin_shutdown()
        )
        .await
        .is_err()
    );
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

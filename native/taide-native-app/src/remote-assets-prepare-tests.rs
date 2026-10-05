use std::path::PathBuf;
use std::time::Duration;

use axum::extract::State;
use axum::http::Uri;
use http_body_util::BodyExt;
use taide_model::app::AppInfo;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::{AppState, EventSink};

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const PREPARE_AND_BLOCKING_OWNERS: usize = 2;
const SECOND_PREPARE_OWNERS: usize = PREPARE_AND_BLOCKING_OWNERS + 1;
const CONTENT: &[u8] = b"<!doctype html><title>synthetic prepared assets</title>";
const QUEUE_BYTES: usize = 256 * 1024;
const QUEUE_COUNT: usize = 64;
const QUEUE_VISITS: usize = 4096;

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture() -> (Directory, Arc<Catalog>) {
    let directory = Directory(
        std::env::temp_dir().join(format!("taide-native-assets-prepare-{}", ProjectId::new())),
    );
    std::fs::create_dir_all(&directory.0).unwrap();
    let catalog = Arc::new(Catalog::new(
        directory.0.join("public"),
        vec![INDEX_DOCUMENT.into()],
        Limits {
            payload_bytes: CONTENT.len(),
            count: 1,
        },
    ));
    (directory, catalog)
}

async fn wait_for_owners(tasks: &TaskSupervisor, count: usize) {
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn 준비_waiter를_취소해도_worker가_직렬_완료하고_frozen_cache를_재사용한다() {
    let (directory, catalog) = fixture();
    std::fs::create_dir_all(&catalog.root).unwrap();
    std::fs::write(catalog.root.join(INDEX_DOCUMENT), CONTENT).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let resolver = catalog.resolver();
    assert!(resolver(INDEX_DOCUMENT).is_none());
    let (entered, started) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let blocker = runtime.spawn_blocking(move || {
        entered.send(()).unwrap();
        let _ = released.recv();
    });
    runtime.block_on(async {
        started.await.unwrap();
        let waiter = tokio::spawn(catalog.clone().prepare(tasks.clone()));
        wait_for_owners(&tasks, PREPARE_AND_BLOCKING_OWNERS).await;
        assert!(catalog.loading.try_lock().is_err());
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert_eq!(tasks.tracked_count(), PREPARE_AND_BLOCKING_OWNERS);
        let second = tokio::spawn(catalog.clone().prepare(tasks.clone()));
        wait_for_owners(&tasks, SECOND_PREPARE_OWNERS).await;
        assert!(resolver(INDEX_DOCUMENT).is_none());
        release.send(()).unwrap();
        blocker.await.unwrap();
        tokio::time::timeout(DEADLINE, second)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        wait_for_owners(&tasks, 0).await;
        assert_eq!(resolver(INDEX_DOCUMENT).unwrap().bytes, CONTENT);
        std::fs::remove_dir_all(&catalog.root).unwrap();
        catalog.clone().prepare(tasks.clone()).await.unwrap();
        assert_eq!(resolver(INDEX_DOCUMENT).unwrap().bytes, CONTENT);
        tasks.shutdown().await;
        assert!(catalog.clone().prepare(tasks.clone()).await.is_err());
        assert_eq!(tasks.tracked_count(), 0);
    });
    let owner = Arc::downgrade(&catalog);
    drop(catalog);
    assert!(owner.upgrade().is_none());
    assert_eq!(resolver(INDEX_DOCUMENT).unwrap().bytes, CONTENT);
    drop(resolver);
    drop(runtime);
    drop(directory);
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _event: AppEvent) {}
}

#[tokio::test]
async fn production_startup과_settings는_자산_준비_실패시_listen하지_않고_성공후에만_시작한다() {
    let (directory, catalog) = fixture();
    let state = AppState::new(AppPaths::new(directory.0.join("data")));
    {
        let mut settings = state.settings.write();
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        settings.remote_access_enabled = false;
    }
    let assembly = crate::bootstrap::assemble(
        state,
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        Arc::new(Sink),
    );
    let services = &assembly.services;
    let hub = Arc::new(
        crate::terminal_host::Hub::new(
            services.clone(),
            crate::terminal_host::Limits {
                sessions: 1,
                core: Default::default(),
                frames: crate::terminal_frames::Limits {
                    bytes: QUEUE_BYTES,
                    count: QUEUE_COUNT,
                    visits: QUEUE_VISITS,
                },
                writer: crate::terminal_writer::Limits {
                    bytes: QUEUE_BYTES,
                    count: QUEUE_COUNT,
                },
            },
        )
        .unwrap(),
    );
    let ports = crate::application_ports::Ports::with_loading_assets(
        services,
        assembly.git_events.clone(),
        crate::remote_terminal::Ports {
            terminals: hub.clone(),
            environment: Arc::new(|_| panic!("asset startup fixture must not spawn a terminal")),
            history: Arc::new(|_, _| {
                panic!("asset startup fixture must not read terminal history")
            }),
            effects: Arc::new(|_, _| {
                panic!("asset startup fixture must not create terminal effects")
            }),
        },
        catalog.clone(),
        AppInfo {
            name: "synthetic".into(),
            version: "synthetic".into(),
            platform: "synthetic".into(),
            arch: "synthetic".into(),
        },
    );
    ports.start(services.clone()).await;
    assert_eq!(services.tasks.tracked_count(), 0);
    assert!(catalog.resolver()(INDEX_DOCUMENT).is_none());
    services.state.settings.write().remote_access_enabled = true;
    ports.start(services.clone()).await;
    assert!(!services.remote.is_running());
    assert_eq!(services.tasks.tracked_count(), 0);
    assert!(catalog.resolver()(INDEX_DOCUMENT).is_none());
    std::fs::create_dir_all(&catalog.root).unwrap();
    std::fs::write(catalog.root.join(INDEX_DOCUMENT), CONTENT).unwrap();
    ports.start(services.clone()).await;
    assert!(services.remote.is_running());
    let response = crate::remote_serving::serve_static(
        State(crate::remote_http::Context {
            services: services.clone(),
            ports: ports.remote.clone(),
        }),
        "/".parse::<Uri>().unwrap(),
    )
    .await;
    assert_eq!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
        CONTENT
    );
    std::fs::remove_dir_all(&catalog.root).unwrap();
    let current = services.state.settings.read().clone();
    let mut updated = current.clone();
    updated.remote_access_enabled = false;
    (ports.reconcile)(services.clone(), current, updated).await;
    assert!(!services.remote.is_running());
    ports.stop(services);
    services.tasks.shutdown().await;
    assert_eq!(services.tasks.tracked_count(), 0);
    let remote_owner = Arc::downgrade(&ports.remote);
    let asset_owner = Arc::downgrade(&catalog);
    drop(catalog);
    drop(ports);
    assert!(remote_owner.upgrade().is_none());
    assert!(asset_owner.upgrade().is_none());
    drop(hub);
}

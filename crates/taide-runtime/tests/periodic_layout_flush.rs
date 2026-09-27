use std::path::PathBuf;
use std::time::Duration;

use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::layout_actions::flush_layouts_periodically;
use taide_runtime::{AppState, ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;
use uuid::Uuid;

const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const TICK_MS: u64 = 5;
const PENDING_PROBE_MS: u64 = 30;

struct Fixture {
    base: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("taide-periodic-flush-{}", Uuid::new_v4()));
        let state = AppState::new(AppPaths::new(base.clone()));
        let project_id = ProjectId::new();
        state
            .layouts
            .write()
            .insert(project_id.clone(), taide_layout::service::default_layout());
        state.dirty_layouts.write().insert(project_id.clone());
        Self { base, state, project_id }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.base.exists() {
            std::fs::remove_dir_all(&self.base).expect("자기 UUID fixture만 정리");
        }
    }
}

struct Release {
    sender: Option<std::sync::mpsc::Sender<()>>,
    holder: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            sender.send(()).ok();
        }
        if let Some(holder) = self.holder.take() {
            holder.join().unwrap();
        }
    }
}

async fn hold_layouts(state: &AppState) -> Release {
    let state = state.clone();
    let (held, held_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let _guard = state.layouts.write();
        held.send(()).ok();
        release_rx.recv().unwrap();
    });
    let release = Release {
        sender: Some(release),
        holder: Some(holder),
    };
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), held_rx)
        .await
        .unwrap()
        .unwrap();
    release
}

async fn wait_for_drain(state: &AppState) {
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
        while !state.dirty_layouts.read().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 첫_tick은_같은_state의_dirty_layout을_실제로_저장한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let expected = fixture.state.layouts.read().get(&fixture.project_id).unwrap().clone();
    assert!(tasks.spawn(
        "layout-flush",
        flush_layouts_periodically(fixture.state.clone(), tasks.clone(), Duration::from_millis(FIXTURE_TIMEOUT_MS))
    ));
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
        while !fixture.state.paths.layout_file(&fixture.project_id).exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tasks.shutdown().await;
    assert_eq!(
        taide_layout::service::load_layout(&fixture.state.paths, &fixture.project_id),
        expected
    );
    assert!(fixture.state.dirty_layouts.read().is_empty());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 닫힌_감독자는_tick_worker와_파일_저장_dirty_drain을_시작하지_않는다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    tasks.stop_all();
    flush_layouts_periodically(fixture.state.clone(), tasks.clone(), Duration::from_millis(TICK_MS)).await;
    assert!(fixture.state.dirty_layouts.read().contains(&fixture.project_id));
    assert!(!fixture.base.exists());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 정상_root는_취소한_ticker가_아닌_잠금_대기중인_실제_저장_worker를_기다린다() {
    let fixture = Fixture::new();
    let expected = fixture.state.layouts.read().get(&fixture.project_id).unwrap().clone();
    let release = hold_layouts(&fixture.state).await;
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    assert!(tasks.spawn(
        "layout-flush",
        flush_layouts_periodically(fixture.state.clone(), tasks.clone(), Duration::from_millis(TICK_MS))
    ));
    wait_for_drain(&fixture.state).await;
    assert_eq!(tasks.tracked_count(), 2);
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        }
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    assert!(!fixture.state.paths.layout_file(&fixture.project_id).exists());
    assert_eq!(tasks.tracked_count(), 1);
    drop(release);
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(drain.is_ready());
    assert_eq!(
        taide_layout::service::load_layout(&fixture.state.paths, &fixture.project_id),
        expected
    );
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 첫_worker가_잠겨_있으면_여러_tick이_지나도_새_dirty를_중복_drain하지_않는다() {
    let fixture = Fixture::new();
    let release = hold_layouts(&fixture.state).await;
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    assert!(tasks.spawn(
        "layout-flush",
        flush_layouts_periodically(fixture.state.clone(), tasks.clone(), Duration::from_millis(TICK_MS))
    ));
    wait_for_drain(&fixture.state).await;
    fixture.state.dirty_layouts.write().insert(fixture.project_id.clone());
    tokio::time::sleep(Duration::from_millis(PENDING_PROBE_MS)).await;
    assert!(fixture.state.dirty_layouts.read().contains(&fixture.project_id));
    assert_eq!(tasks.tracked_count(), 2);
    drop(release);
    wait_for_drain(&fixture.state).await;
    tasks.shutdown().await;
    assert!(fixture.state.paths.layout_file(&fixture.project_id).exists());
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn native는_같은_state_supervisor_interval로_runtime_loop를_등록한다() {
    let native = include_str!("../../../src-tauri/src/lib.rs");
    let tick = native
        .split_once("let flush_state =")
        .unwrap()
        .1
        .split_once("log::info!(")
        .unwrap()
        .0;
    assert!(tick.contains("(*app.state::<AppState>()).clone()"));
    assert!(tick.contains("(*app.state::<TaskSupervisor>()).clone()"));
    assert!(tick.contains(".spawn(\"layout-flush\""));
    assert!(tick.contains("layout_actions::flush_layouts_periodically("));
    assert!(tick.contains("domain::layout::service::LAYOUT_FLUSH_INTERVAL_MS"));
    assert!(!tick.contains("AppHandle"));
    assert!(!tick.contains("tauri::async_runtime::spawn_blocking"));
}

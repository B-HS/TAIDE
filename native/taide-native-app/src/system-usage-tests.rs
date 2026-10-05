use std::{
    sync::{
        Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use taide_model::{
    app_event::AppEvent, ids::ProjectId, paths::AppPaths, system::SystemUsageProcessKind,
};
use taide_runtime::{AppState, EventSink};
use taide_system::service::ProcessRecord;
use tokio::sync::Notify;

use super::*;
use crate::remote_utilities::{UsageLabels, UsageProvider};

const DEADLINE: Duration = Duration::from_secs(3);
const ROOT_PID: u32 = 4000;
const CHILD_PID: u32 = ROOT_PID + 1;
const MEMORY: u64 = 1024 * 1024;
const CPU: f64 = 42.5;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Samples {
    gate: Mutex<bool>,
    released: Condvar,
    started: Notify,
    usage_calls: AtomicUsize,
    detail_calls: AtomicUsize,
    fail: AtomicBool,
}

impl Samples {
    fn release(&self) {
        *self.gate.lock().unwrap() = true;
        self.released.notify_all();
    }
}
struct Release(Arc<Samples>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.release();
    }
}

impl UsageProvider for Samples {
    fn collect_app_usage(&self) -> AppResult<SystemUsage> {
        let call = self.usage_calls.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        let mut open = self.gate.lock().unwrap();
        while !*open {
            open = self.released.wait(open).unwrap();
        }
        if self.fail.load(Ordering::SeqCst) {
            return Err(AppError::Internal("synthetic sampling error".into()));
        }
        Ok(SystemUsage {
            cpu_percent: (call > 0).then_some(CPU),
            memory_bytes: MEMORY as f64,
        })
    }
    fn refresh_process_records(&self) -> Vec<ProcessRecord> {
        self.detail_calls.fetch_add(1, Ordering::SeqCst);
        vec![
            ProcessRecord {
                pid: ROOT_PID,
                parent_pid: None,
                name: "synthetic app".into(),
                cpu_usage: 0.0,
                memory: MEMORY,
                has_previous_cpu_sample: false,
            },
            ProcessRecord {
                pid: CHILD_PID,
                parent_pid: Some(ROOT_PID),
                name: "synthetic child".into(),
                cpu_usage: 100.0,
                memory: MEMORY * 2,
                has_previous_cpu_sample: true,
            },
        ]
    }
}

async fn settle(
    sampler: &mut Sampler,
    services: &Arc<AppServices>,
    context: &egui::Context,
    visible: bool,
    now: Instant,
) {
    tokio::time::timeout(DEADLINE, async {
        loop {
            sampler.tick(services, context, visible, now);
            if sampler.summary.pending.is_none() && sampler.detail.pending.is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

fn foreground(context: &egui::Context, focused: bool) {
    let mut output = context.run_ui(
        egui::RawInput {
            focused,
            ..Default::default()
        },
        |_| {},
    );
    output.textures_delta.clear();
}

#[tokio::test]
async fn system_usage는_실제감독수집기를_비차단_단일요청_주기_숨김과_종료수명으로_연결한다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-usage-{}", ProjectId::new())),
    ));
    let services = crate::bootstrap::services(
        state,
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        Arc::new(Sink),
    );
    let samples = Arc::new(Samples {
        gate: Mutex::new(false),
        released: Condvar::new(),
        started: Notify::new(),
        usage_calls: AtomicUsize::new(0),
        detail_calls: AtomicUsize::new(0),
        fail: AtomicBool::new(false),
    });
    let _release = Release(samples.clone());
    let ports = Ports {
        fonts: Arc::new(|| panic!("usage must not enumerate fonts")),
        usage: samples.clone(),
        root_pid: Arc::new(|| Ok(ROOT_PID)),
        cpu_count: Arc::new(|| 2),
        label_providers: vec![Arc::new(|_| {
            UsageLabels::from([(
                CHILD_PID,
                (
                    SystemUsageProcessKind::Lsp,
                    "Synthetic LSP · Project".into(),
                ),
            )])
        })],
    };
    let mut sampler = Sampler::new(ports);
    let context = egui::Context::default();
    foreground(&context, false);
    let now = Instant::now();
    sampler.tick(&services, &context, true, now);
    tokio::time::timeout(DEADLINE, samples.started.notified())
        .await
        .unwrap();
    assert!(sampler.summary.pending.is_some());
    assert!(sampler.summary().is_none());
    sampler.tick(&services, &context, false, now);
    sampler.tick(&services, &context, true, now);
    sampler.detail_open = true;
    sampler.tick(&services, &context, false, now);
    tokio::time::timeout(DEADLINE, async {
        while sampler.detail.data.is_none() {
            sampler.tick(&services, &context, false, now);
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(samples.usage_calls.load(Ordering::SeqCst), 1);
    assert_eq!(samples.detail_calls.load(Ordering::SeqCst), 1);
    let rows = sampler.processes();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].pid, CHILD_PID);
    assert_eq!(rows[0].label, "Synthetic LSP · Project");
    assert_eq!(rows[0].kind, SystemUsageProcessKind::Lsp);
    assert_eq!(rows[0].cpu_percent, Some(50.0));
    assert_eq!(rows[1].cpu_percent, None);
    assert_eq!(rows[1].label, "TAIDE");
    sampler.detail_open = false;
    sampler.tick(&services, &context, false, now);
    assert!(sampler.processes().is_empty());
    samples.release();
    settle(&mut sampler, &services, &context, false, now).await;
    assert!(sampler.summary.data.is_none());
    sampler.tick(&services, &context, true, now);
    settle(&mut sampler, &services, &context, true, now).await;
    assert_eq!(samples.usage_calls.load(Ordering::SeqCst), 2);
    assert_eq!(sampler.summary().unwrap().cpu_percent, Some(CPU));
    sampler.tick(&services, &context, false, now);
    assert!(sampler.summary().is_none());
    assert!(sampler.summary.data.is_some());
    sampler.tick(&services, &context, true, now);
    assert!(sampler.summary().is_some());
    settle(&mut sampler, &services, &context, true, now).await;
    let due = now + POLL_INTERVAL;
    sampler.tick(&services, &context, true, due);
    assert_eq!(samples.usage_calls.load(Ordering::SeqCst), 3);
    foreground(&context, true);
    sampler.tick(&services, &context, true, due - Duration::from_millis(1));
    assert!(sampler.summary.pending.is_none());
    samples.fail.store(true, Ordering::SeqCst);
    sampler.tick(&services, &context, true, due);
    settle(&mut sampler, &services, &context, true, due).await;
    assert_eq!(samples.usage_calls.load(Ordering::SeqCst), 4);
    assert_eq!(sampler.summary().unwrap().cpu_percent, Some(CPU));
    sampler.tick(&services, &context, true, due);
    assert!(sampler.summary.pending.is_none());
    *samples.gate.lock().unwrap() = false;
    sampler.tick(&services, &context, true, due + POLL_INTERVAL);
    tokio::time::timeout(DEADLINE, async {
        while samples.usage_calls.load(Ordering::SeqCst) != 5 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    services.state.begin_shutdown();
    sampler.tick(&services, &context, true, due + POLL_INTERVAL);
    assert!(sampler.summary().is_none());
    samples.release();
    drop(sampler);
    tokio::time::timeout(DEADLINE, services.tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(services.tasks.tracked_count(), 0);
    let owner = Arc::downgrade(&services);
    drop(services);
    assert!(owner.upgrade().is_none());
}

#[test]
fn system_usage_zen_unmount는_캐시와_modal을_비우고_같은_pending을_복제하지_않는다() {
    let provider = Arc::new(Samples {
        gate: Mutex::new(true),
        released: Condvar::new(),
        started: Notify::new(),
        usage_calls: AtomicUsize::new(0),
        detail_calls: AtomicUsize::new(0),
        fail: AtomicBool::new(false),
    });
    let mut sampler = Sampler::new(Ports {
        fonts: Arc::new(Vec::new),
        usage: provider.clone(),
        root_pid: Arc::new(|| Ok(ROOT_PID)),
        cpu_count: Arc::new(|| 1),
        label_providers: Vec::new(),
    });
    let now = Instant::now();
    sampler.summary.gate(true, now);
    sampler.summary.data = Some(SystemUsage {
        cpu_percent: Some(CPU),
        memory_bytes: MEMORY as f64,
    });
    sampler.detail_open = true;
    sampler.detail.gate(true, now);
    sampler.detail.data = Some(Vec::new());
    let (sender, receiver) = oneshot::channel();
    sampler.summary.pending = Some(Pending {
        generation: sampler.summary.generation.clone(),
        receiver,
    });
    sampler.unmount(now);
    assert!(sampler.summary().is_none());
    assert!(sampler.summary.data.is_none());
    assert!(sampler.detail.data.is_none());
    assert!(!sampler.detail_open);
    assert!(sampler.summary.pending.is_some());
    sampler.summary.gate(true, now);
    sender
        .send(Ok(SystemUsage {
            cpu_percent: Some(CPU),
            memory_bytes: MEMORY as f64,
        }))
        .unwrap();
    sampler.summary.poll(now);
    assert!(sampler.summary().is_none());
    assert!(sampler.summary.pending.is_none());
    assert!(sampler.summary.initial);
    assert_eq!(provider.usage_calls.load(Ordering::SeqCst), 0);
}

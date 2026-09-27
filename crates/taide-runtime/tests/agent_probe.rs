#[cfg(unix)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use taide_agent::constants::AGENT_NAME_CLAUDE;
use taide_agent::service::{DetectedAgentProbe, HookEmitter};
#[cfg(unix)]
use taide_agent::store::AgentStore;
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::error::AppError;
#[cfg(unix)]
use taide_runtime::agent_probe::probe_process_names;
use taide_runtime::agent_probe::{probe_process_tree, resolve_claude_hook_emitter};
use taide_runtime::{ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;

const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_PROBE_MS: u64 = 20;
const VERSION_DEADLINE_MS: u64 = 50;
const FIRST_PID: u32 = 41;
const SECOND_PID: u32 = 42;

struct Release(Option<std::sync::mpsc::Sender<()>>);

impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            sender.send(()).ok();
        }
    }
}

fn deadline() -> Duration {
    Duration::from_millis(FIXTURE_TIMEOUT_MS)
}

fn pids() -> Vec<(String, u32)> {
    vec![("first".to_string(), FIRST_PID), ("second".to_string(), SECOND_PID)]
}

#[cfg(unix)]
#[tokio::test]
async fn 빈_pid와_캐시된_이름_없음은_os_factory를_실행하지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let agents = AgentStore::new();
    assert!(probe_process_names(&tasks, &agents, Vec::new(), |_| panic!("empty probe"))
        .await
        .unwrap()
        .is_empty());
    agents.remember_process_names(HashMap::from([(FIRST_PID, Some(AGENT_NAME_CLAUDE)), (SECOND_PID, None)]));
    let probes = probe_process_names(&tasks, &agents, pids(), |_| panic!("cached probe"))
        .await
        .unwrap();
    assert_eq!(probes.len(), 1);
    assert_eq!(probes[0].session_id, "first");
    assert_eq!(probes[0].pid, FIRST_PID);
    assert_eq!(probes[0].name, AGENT_NAME_CLAUDE);
    assert_eq!(tasks.tracked_count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn 미해결_pid만_정렬_중복제거하고_없음도_캐시한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let agents = AgentStore::new();
    agents.remember_process_names(HashMap::from([(SECOND_PID, None)]));
    let mut requested = pids();
    requested.push(("duplicate".to_string(), FIRST_PID));
    let probes = probe_process_names(&tasks, &agents, requested, |unresolved| {
        assert_eq!(unresolved, vec![FIRST_PID]);
        HashMap::from([(FIRST_PID, Some(AGENT_NAME_CLAUDE))])
    })
    .await
    .unwrap();
    assert_eq!(
        probes.iter().map(|probe| probe.session_id.as_str()).collect::<Vec<_>>(),
        vec!["first", "duplicate"]
    );
    assert!(agents.unresolved_pids(&pids()).is_empty());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn process_tree_port는_empty를_건너뛰고_실제_probe_순서와_pid를_보존한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    assert!(probe_process_tree(&tasks, Vec::new(), |_| panic!("empty snapshot"))
        .await
        .unwrap()
        .is_empty());
    let probes = probe_process_tree(&tasks, pids(), |requested| {
        assert_eq!(requested, pids());
        vec![DetectedAgentProbe {
            session_id: "second".to_string(),
            name: AGENT_NAME_CLAUDE,
            pid: FIRST_PID,
        }]
    })
    .await
    .unwrap();
    assert_eq!(probes.len(), 1);
    assert_eq!(probes[0].session_id, "second");
    assert_eq!(probes[0].pid, FIRST_PID);
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 닫힌_입장은_pid_cli_factory를_시작하지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    tasks.stop_all();
    assert!(matches!(
        probe_process_tree(&tasks, pids(), |_| panic!("closed snapshot")).await,
        Err(AppError::Forbidden(_))
    ));
    #[cfg(unix)]
    {
        let agents = AgentStore::new();
        assert!(matches!(
            probe_process_names(&tasks, &agents, pids(), |_| panic!("closed names")).await,
            Err(AppError::Forbidden(_))
        ));
        assert_eq!(agents.unresolved_pids(&pids()), vec![FIRST_PID, SECOND_PID]);
    }
    let cache = OnceLock::new();
    assert_eq!(
        resolve_claude_hook_emitter(&tasks, &cache, deadline(), || panic!("closed CLI")).await,
        HookEmitter::DevTty
    );
    assert_eq!(cache.get(), Some(&HookEmitter::DevTty));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn emitter는_기존_버전_판정과_실패_캐시를_보존한다() {
    for (stdout, expected) in [
        (b"2.1.141 (Claude Code)".to_vec(), HookEmitter::TerminalSequence),
        (b"1.0.0 (Claude Code)".to_vec(), HookEmitter::DevTty),
        (b"invalid version".to_vec(), HookEmitter::DevTty),
    ] {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let cache = OnceLock::new();
        assert_eq!(
            resolve_claude_hook_emitter(&tasks, &cache, deadline(), || Ok(stdout)).await,
            expected
        );
        tasks.stop_all();
        assert_eq!(
            resolve_claude_hook_emitter(&tasks, &cache, deadline(), || panic!("cached CLI")).await,
            expected
        );
        assert_eq!(tasks.tracked_count(), 0);
    }
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let cache = OnceLock::new();
    assert_eq!(
        resolve_claude_hook_emitter(&tasks, &cache, deadline(), || Err(std::io::Error::other("fixture missing CLI"))).await,
        HookEmitter::DevTty
    );
    assert_eq!(cache.get(), Some(&HookEmitter::DevTty));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn worker_panic은_pid_오류와_cli_fallback으로_회수된다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    assert!(matches!(
        probe_process_tree(&tasks, pids(), |_| panic!("fixture snapshot panic")).await,
        Err(AppError::Internal(_))
    ));
    let cache = OnceLock::new();
    assert_eq!(
        resolve_claude_hook_emitter(&tasks, &cache, deadline(), || panic!("fixture CLI panic")).await,
        HookEmitter::DevTty
    );
    assert_eq!(cache.get(), Some(&HookEmitter::DevTty));
    assert_eq!(tasks.tracked_count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn pid_요청_취소_뒤_root는_실제_worker를_기다리고_캐시는_갱신되지_않는다() {
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let agents = AgentStore::new();
    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let completed = Arc::new(AtomicBool::new(false));
    let marker = completed.clone();
    let request_tasks = tasks.clone();
    let request_agents = agents.clone();
    let request = tokio::spawn(async move {
        probe_process_names(&request_tasks, &request_agents, pids(), move |_| {
            started.send(()).ok();
            release_rx.recv().unwrap();
            marker.store(true, Ordering::SeqCst);
            HashMap::from([(FIRST_PID, Some(AGENT_NAME_CLAUDE))])
        })
        .await
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
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
    assert!(!completed.load(Ordering::SeqCst));
    assert!(tasks.tracked_count() > 0);
    drop(release);
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert!(completed.load(Ordering::SeqCst));
    assert_eq!(agents.unresolved_pids(&pids()), vec![FIRST_PID, SECOND_PID]);
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn cli_timeout_뒤에도_root는_worker를_기다리고_늦은_답은_캐시를_바꾸지_않는다() {
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let cache = Arc::new(OnceLock::new());
    let request_cache = cache.clone();
    let request_tasks = tasks.clone();
    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let completed = Arc::new(AtomicBool::new(false));
    let marker = completed.clone();
    let request = tokio::spawn(async move {
        resolve_claude_hook_emitter(
            &request_tasks,
            &request_cache,
            Duration::from_millis(VERSION_DEADLINE_MS),
            move || {
                started.send(()).ok();
                release_rx.recv().unwrap();
                marker.store(true, Ordering::SeqCst);
                Ok(b"2.1.141 (Claude Code)".to_vec())
            },
        )
        .await
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert_eq!(
        tokio::time::timeout(deadline(), request).await.unwrap().unwrap(),
        HookEmitter::DevTty
    );
    assert_eq!(cache.get(), Some(&HookEmitter::DevTty));
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
    assert!(!completed.load(Ordering::SeqCst));
    assert!(tasks.tracked_count() > 0);
    drop(release);
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert!(completed.load(Ordering::SeqCst));
    assert_eq!(cache.get(), Some(&HookEmitter::DevTty));
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다() {
    let native = include_str!("../../../src-tauri/src/domain/agent/commands.rs");
    let hooks = include_str!("../../../src-tauri/src/domain/agent/hooks.rs");
    assert!(!native.contains("tauri::async_runtime::spawn_blocking"));
    assert!(native.contains("agent_probe::probe_process_names(tasks, agents, pids, |unresolved| resolve_agent_names(&unresolved)).await"));
    assert!(native.contains("agent_probe::probe_process_tree(tasks, pids, detect_agents_for_pids).await"));
    assert_eq!(native.matches("detect_agents_for_pids_blocking(&tasks, &agents, pids)").count(), 2);
    assert_eq!(native.matches("let tasks = app.state::<TaskSupervisor>();").count(), 3);
    assert!(native.contains("|| resolve_claude_hook_emitter(&tasks)"));
    assert_eq!(hooks.matches("|| commands::resolve_claude_hook_emitter(&tasks)").count(), 2);
    assert_eq!(hooks.matches("let tasks = app.state::<TaskSupervisor>();").count(), 5);
    assert!(hooks
        .contains("agent_actions::apply_hook_payload(&state, &agents, &agent_hooks, &TauriEventSink(app), &tasks, agent_name, payload)"));
    let reconcile = include_str!("../src/agent_hook_reconcile.rs");
    assert!(reconcile.contains("let emitter = resolve_project_emitter().await;"));
    assert!(native.contains("Duration::from_secs(CLAUDE_VERSION_TIMEOUT_SECONDS)"));
    let emitter = native
        .split("pub(super) async fn resolve_claude_hook_emitter")
        .nth(1)
        .unwrap()
        .split("const CLAUDE_VERSION_FLAG")
        .next()
        .unwrap();
    let compact = emitter.split_whitespace().collect::<String>();
    assert!(compact.contains("std::process::Command::new(AGENT_NAME_CLAUDE).arg(CLAUDE_VERSION_FLAG).output()"));
}

#[tokio::test]
async fn 기존_미감독_timeout_패턴은_worker가_남아도_root를_해제한다() {
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let mut worker = tokio::task::spawn_blocking(move || {
        started.send(()).ok();
        release_rx.recv().unwrap();
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(VERSION_DEADLINE_MS), &mut worker)
        .await
        .is_err());
    let mut drain = ExitDrain::default();
    let (ready, ready_rx) = oneshot::channel();
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
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert!(!worker.is_finished());
    assert_eq!(tasks.tracked_count(), 0);
    drop(release);
    tokio::time::timeout(deadline(), worker).await.unwrap().unwrap();
}

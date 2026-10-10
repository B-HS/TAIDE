use std::sync::{Arc, Weak};

use taide_infra::lsp_proc::LspProcConfig;
use taide_lsp::native::session::{SessionClient, SessionSnapshot};
use taide_lsp::native::{Failure, Phase};
use taide_lsp::process::{HEALTHY_RESTART_WINDOW, restart_backoff_delay};
use taide_runtime::AppServices;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::{Command, SessionKey};

pub(super) fn copy_config(config: &LspProcConfig) -> LspProcConfig {
    LspProcConfig {
        command: config.command.clone(),
        args: config.args.clone(),
        cwd: config.cwd.clone(),
    }
}

pub(super) fn attach(
    services: &Arc<AppServices>,
    client: SessionClient,
    config: LspProcConfig,
    key: SessionKey,
    commands: mpsc::Sender<Command>,
) -> bool {
    let Some(project_root) = services
        .state
        .projects
        .read()
        .get(&key.project)
        .map(|project| project.root.clone())
    else {
        return false;
    };
    services.tasks.spawn_transient(
        "native-editor-lsp-recovery",
        run(
            Arc::downgrade(services),
            client,
            config,
            key,
            project_root,
            commands,
        ),
    )
}

#[derive(Clone, Copy)]
struct Timer {
    generation: u64,
    phase: Phase,
    deadline: Instant,
}

#[derive(Default)]
struct Policy {
    restarts: u32,
    timer: Option<Timer>,
    healthy_generation: Option<u64>,
}

impl Policy {
    fn ready(&self, now: Instant) -> bool {
        self.timer.is_some_and(|timer| timer.deadline <= now)
    }

    fn observe(&mut self, state: &SessionSnapshot, now: Instant) -> bool {
        if self
            .timer
            .is_some_and(|timer| timer.generation == state.generation && timer.phase == state.phase)
        {
            return true;
        }
        self.timer = None;
        let delay = match state.phase {
            Phase::Degraded => {
                let Some(restarts) = self.restarts.checked_add(1) else {
                    return false;
                };
                let Some(delay) = restart_backoff_delay(restarts) else {
                    return false;
                };
                self.restarts = restarts;
                delay
            }
            Phase::Running if self.healthy_generation != Some(state.generation) => {
                HEALTHY_RESTART_WINDOW
            }
            _ => return true,
        };
        self.timer = now.checked_add(delay).map(|deadline| Timer {
            generation: state.generation,
            phase: state.phase,
            deadline,
        });
        self.timer.is_some()
    }

    fn elapsed(&mut self, state: &SessionSnapshot) -> bool {
        let Some(timer) = self.timer.take() else {
            return false;
        };
        if state.generation != timer.generation || state.phase != timer.phase {
            return false;
        }
        if state.phase == Phase::Running {
            self.restarts = 0;
            self.healthy_generation = Some(state.generation);
            return false;
        }
        state.phase == Phase::Degraded
    }
}

async fn run(
    owner: Weak<AppServices>,
    client: SessionClient,
    config: LspProcConfig,
    key: SessionKey,
    project_root: String,
    commands: mpsc::Sender<Command>,
) {
    let mut source = client.subscribe();
    let mut policy = Policy::default();
    let mut last_state: Option<SessionSnapshot> = None;
    loop {
        source.borrow_and_update();
        let state = client.snapshot();
        if last_state.as_ref().is_none_or(|previous| {
            previous.phase != state.phase
                || previous.generation != state.generation
                || previous.failure != state.failure
        }) {
            if commands
                .send(Command::StateChanged {
                    key: key.clone(),
                    source: source.clone(),
                    state: state.clone(),
                })
                .await
                .is_err()
            {
                return;
            }
            last_state = Some(state.clone());
        }
        if matches!(state.phase, Phase::Stopping | Phase::Stopped)
            || state.failure == Some(Failure::ReinitializeExhausted)
            || source.has_changed().is_err()
        {
            return;
        }
        if !policy.observe(&state, Instant::now()) {
            log::warn!("native editor language server restart attempts exhausted");
            return;
        }
        let elapsed = if policy.ready(Instant::now()) {
            true
        } else {
            tokio::select! {
                biased;
                changed = source.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    false
                }
                _ = wait_timer(policy.timer) => true,
            }
        };
        let expected = client.snapshot();
        if !elapsed || !policy.elapsed(&expected) {
            continue;
        }
        let Some(services) = owner.upgrade() else {
            return;
        };
        let _guard = services.state.begin_mutation().await;
        if services.state.is_shutting_down()
            || services
                .state
                .projects
                .read()
                .get(&key.project)
                .is_none_or(|project| project.root_missing || project.root != project_root)
            || client.snapshot().generation != expected.generation
            || client.snapshot().phase != Phase::Degraded
        {
            return;
        }
        if let Err(error) = client.restart(copy_config(&config)).await {
            log::warn!("native editor language server restart failed: {error:?}");
            return;
        }
        drop(_guard);
        drop(services);
        if source.changed().await.is_err() {
            return;
        }
    }
}

async fn wait_timer(timer: Option<Timer>) {
    match timer {
        Some(timer) => tokio::time::sleep_until(timer.deadline).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_lsp::native::Failure;
    use taide_lsp::process::RESTART_BACKOFF_LIMIT;

    #[test]
    fn 중복_status는_backoff를_늦추지_않고_동일_generation의_healthy만_재시작수를_초기화한다() {
        let mut state = SessionSnapshot {
            phase: Phase::Degraded,
            generation: 0,
            pending: 0,
            server_pending: 0,
            progress_tokens: 0,
            registrations: 0,
            capability_revision: 0,
            document_methods: Default::default(),
            document_signature_options: Default::default(),
            document_completion_options: Default::default(),
            pid: None,
            failure: Some(Failure::TransportClosed),
        };
        let mut policy = Policy::default();
        let now = Instant::now();
        for restart in 1..=RESTART_BACKOFF_LIMIT {
            state.generation = u64::from(restart - 1);
            assert!(policy.observe(&state, now));
            let timer = policy.timer.unwrap();
            assert!(!policy.ready(now));
            assert!(policy.ready(timer.deadline));
            assert_eq!(
                timer.deadline - now,
                restart_backoff_delay(restart).unwrap()
            );
            assert!(policy.observe(&state, now + HEALTHY_RESTART_WINDOW));
            assert_eq!(policy.timer.unwrap().deadline, timer.deadline);
            assert!(policy.ready(now + HEALTHY_RESTART_WINDOW));
            assert_eq!(policy.restarts, restart);
            assert!(policy.elapsed(&state));
        }
        state.generation = u64::from(RESTART_BACKOFF_LIMIT);
        assert!(!policy.observe(&state, now));
        state.phase = Phase::Running;
        state.failure = None;
        assert!(policy.observe(&state, now));
        assert_eq!(policy.timer.unwrap().deadline - now, HEALTHY_RESTART_WINDOW);
        state.generation += 1;
        assert!(!policy.elapsed(&state));
        assert_eq!(policy.restarts, RESTART_BACKOFF_LIMIT);
        assert!(policy.observe(&state, now));
        assert!(!policy.elapsed(&state));
        assert_eq!(policy.restarts, 0);
        assert!(policy.observe(&state, now));
        assert!(policy.timer.is_none());
        assert!(!policy.ready(now));
        state.phase = Phase::Degraded;
        assert!(policy.observe(&state, now));
        assert_eq!(policy.restarts, 1);
        state.phase = Phase::Stopping;
        assert!(policy.observe(&state, now));
        assert!(policy.timer.is_none());
        assert!(!policy.elapsed(&state));
    }
}

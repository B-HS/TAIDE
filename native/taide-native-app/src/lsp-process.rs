use std::sync::Arc;

use taide_infra::lsp_proc::LspProcHandle;
use taide_lsp::process::{
    HEALTHY_RESTART_WINDOW, confirms_healthy_restart, restart_backoff_delay, spawn_language_server,
};
use taide_lsp::session::LspLifecycleSnapshot;
use taide_lsp::store::LspSessionEntry;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::lsp::{LanguageServerSpec, LspSessionStatus};
use taide_runtime::AppServices;

const STDERR_TAIL_LOG_SEPARATOR: &str = " / ";

pub fn create_process(
    services: Arc<AppServices>,
    session: String,
    epoch: u64,
    spec: LanguageServerSpec,
    root: String,
) -> AppResult<Arc<LspProcHandle>> {
    if services.state.is_shutting_down() {
        return Err(AppError::Forbidden(
            "language server runtime is shutting down".into(),
        ));
    }
    let message_owner = Arc::downgrade(&services);
    let message_session = session.clone();
    let exit_owner = message_owner.clone();
    spawn_language_server(
        &services.state.paths,
        &spec,
        &root,
        move |message| {
            let Some(services) = message_owner.upgrade() else {
                return;
            };
            let Some(entry) = services.lsp.get(&message_session) else {
                return;
            };
            if entry.lifecycle.is_active_process_epoch(epoch) {
                entry.subscribers.broadcast(&message);
            }
        },
        move |code, stderr| {
            let Some(services) = exit_owner.upgrade() else {
                return;
            };
            let worker = services.clone();
            services
                .tasks
                .spawn_transient("lsp-process-exit", async move {
                    let _guard = worker.state.begin_mutation().await;
                    handle_process_exit(worker.clone(), session, epoch, code, stderr);
                });
        },
    )
}

fn emit_status(services: &AppServices, session: &str, snapshot: LspLifecycleSnapshot) {
    services.events.publish(AppEvent::LspSessionStatusChanged {
        session_id: session.into(),
        status: snapshot.status,
        last_error: snapshot.last_error,
        generation: snapshot.generation,
    });
}

fn set_status(
    services: &AppServices,
    session: &str,
    entry: &LspSessionEntry,
    status: LspSessionStatus,
    error: String,
) {
    emit_status(
        services,
        session,
        entry.lifecycle.set_status(status, Some(error)),
    );
}

fn masked_stderr_tail(tail: &str) -> String {
    taide_infra::redact::mask_known_secrets(tail)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(STDERR_TAIL_LOG_SEPARATOR)
}

fn handle_process_exit(
    services: Arc<AppServices>,
    session: String,
    epoch: u64,
    code: Option<i32>,
    stderr: String,
) {
    let Some(entry) = services.lsp.get(&session) else {
        return;
    };
    let Some(restarts) = entry.lifecycle.begin_exit_recovery(epoch) else {
        log::info!(
            "lsp {}: ignored stopped or superseded process exit (code={code:?})",
            entry.server_id
        );
        return;
    };
    log::warn!(
        "lsp {}: exited code={code:?} restarts={restarts} stderr_tail={}",
        entry.server_id,
        masked_stderr_tail(&stderr)
    );
    let Some(backoff) = restart_backoff_delay(restarts) else {
        set_status(
            &services,
            &session,
            &entry,
            LspSessionStatus::Crashed,
            format!(
                "서버가 반복적으로 종료되어 재시작을 중지했습니다 (마지막 종료 코드: {code:?})"
            ),
        );
        return;
    };
    set_status(
        &services,
        &session,
        &entry,
        LspSessionStatus::Starting,
        format!("서버가 종료되어 재시작합니다 (마지막 종료 코드: {code:?})"),
    );
    let worker = services.clone();
    let spec = entry.spec.clone();
    let root = entry.root.clone();
    services.tasks.spawn_transient("lsp-auto-restart", async move {
        tokio::time::sleep(backoff).await;
        let _guard = worker.state.begin_mutation().await;
        let Some(entry) = worker.lsp.get(&session) else {
            return;
        };
        if !entry.lifecycle.is_active_process_epoch(epoch) {
            return;
        }
        let next_epoch = entry.lifecycle.advance_process_epoch();
        match worker.lsp.spawn_process(|| {
            create_process(worker.clone(), session.clone(), next_epoch, spec, root)
        }) {
            Ok(process) => {
                *entry.proc.lock() = Some(process.clone());
                emit_status(
                    &worker,
                    &session,
                    entry.lifecycle.auto_respawned(
                        "서버 프로세스가 자동으로 재시작됐습니다. 초기화 핸드셰이크가 다시 완료될 때까지 기다려주세요.".into(),
                    ),
                );
                worker.tasks.spawn_transient("lsp-healthy-reset", async move {
                    tokio::time::sleep(HEALTHY_RESTART_WINDOW).await;
                    if confirms_healthy_restart(&entry.proc, &process) {
                        entry.lifecycle.reset_restart_count();
                    }
                });
            }
            Err(error) => {
                set_status(&worker, &session, &entry, LspSessionStatus::Crashed, error.to_string());
            }
        }
    });
}

#[cfg(test)]
#[path = "lsp-process-tests.rs"]
mod tests;

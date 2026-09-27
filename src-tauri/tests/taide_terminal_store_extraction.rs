use std::sync::Arc;

use parking_lot::Mutex;
use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_terminal::metadata::TerminalSessionMetadata;
use taide_terminal::session::{TerminalSessionOutput, TERMINAL_REPLAY_PREAMBLE};
use taide_terminal::store::{TerminalSessionEntry, TerminalStore};

const TEST_COLS: u16 = 80;
const TEST_ROWS: u16 = 24;

#[test]
fn spawn_adapter는_감독_worker와_동일한_guard를_등록까지_유지한다() {
    let source = include_str!("../src/domain/terminal/commands.rs");
    let spawn = source.split_once("pub async fn pty_spawn(").unwrap().1;
    let spawn = spawn.split_once("pub async fn pty_write(").unwrap().0;
    assert!(spawn.contains("tasks: State<'_, TaskSupervisor>"));
    assert!(spawn.contains("state.is_shutting_down()"));
    let environment = spawn.find("env_provider.extra_env(&app).await").unwrap();
    let guard = spawn.find("state.begin_owned_mutation().await").unwrap();
    let project = spawn.find("ensure_project_open(&state, &opts.project_id)?").unwrap();
    let worker = spawn
        .find("let (handle, _guard) = taide_runtime::terminal_actions::run_terminal_spawn(&tasks, &store, guard,")
        .unwrap();
    let insert = spawn
        .find("store.insert(session_id.clone(), TerminalSessionEntry::new(handle, metadata, output))?")
        .unwrap();
    let event = spawn.find("TauriEventSink(&app).publish(spawned)").unwrap();
    assert!(environment < guard && guard < project && project < worker && worker < insert && insert < event);
    assert!(!spawn.contains("tauri::async_runtime::spawn_blocking("));
    let root = include_str!("../src/lib.rs");
    let drain = root.split_once("exit_drain.begin(").unwrap().1;
    let drain = drain.split_once("move || handle.exit(exit_code)").unwrap().0;
    assert!(drain.contains("(*app_handle.state::<TerminalStore>()).clone()"));
}

#[test]
fn 없는_세션_조회는_기존_not_found_계약을_유지한다() {
    let store = TerminalStore::new();

    assert!(matches!(store.writer_handle("missing"), Err(AppError::NotFound(_))));
    assert!(store.resize("missing", TEST_COLS, TEST_ROWS).is_err());
    assert!(store.set_paused("missing", true).is_err());
    assert!(store.attach("missing", |_| true).is_err());
    assert!(matches!(store.kill("missing"), Err(AppError::NotFound(_))));
    assert!(store.detach("missing", 0).is_ok());
    assert!(!store.update_cwd("missing", "/other".to_string()));
    assert!(store.cwd("missing").is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn 프로젝트_회수와_출력_연결은_다른_세션을_유지한다() {
    const TEST_SCROLLBACK_BYTES: usize = 64 * 1024;

    let project_a = ProjectId::from("prj-a".to_string());
    let project_b = ProjectId::from("prj-b".to_string());
    let store = TerminalStore::new();
    let first_output = Arc::new(TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES));
    first_output.append_and_broadcast(b"before");

    for (session_id, project_id, output) in [
        ("term-a", project_a.clone(), first_output),
        (
            "term-b",
            project_b.clone(),
            Arc::new(TerminalSessionOutput::new(TEST_SCROLLBACK_BYTES)),
        ),
    ] {
        let pty = store
            .begin_spawn()
            .unwrap()
            .spawn(|| {
                taide_infra::pty::spawn(
                    taide_infra::pty::PtySpawnConfig {
                        shell: Some("/bin/sh".to_string()),
                        cwd: std::env::temp_dir().to_string_lossy().to_string(),
                        cols: TEST_COLS,
                        rows: TEST_ROWS,
                        extra_env: vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())],
                    },
                    |_| {},
                    |_| {},
                )
            })
            .expect("테스트 셸 실행");
        let metadata = Arc::new(TerminalSessionMetadata::new(project_id, "/project".to_string(), "sh".to_string()));
        store
            .insert(session_id.to_string(), TerminalSessionEntry::new(pty, metadata, output))
            .unwrap();
    }

    assert_eq!(store.sessions_for_project(&project_a).len(), 1);
    assert_eq!(store.sessions_for_project(&project_b).len(), 1);
    assert!(store.update_cwd("term-a", "/project/src".to_string()));
    assert_eq!(store.cwd("term-a"), Some("/project/src".to_string()));

    let received = Arc::new(Mutex::new(Vec::new()));
    let sink_received = received.clone();
    let attached = store
        .attach("term-a", move |bytes| {
            sink_received.lock().push(bytes.to_vec());
            true
        })
        .expect("출력 연결");
    assert_eq!(received.lock().concat(), [TERMINAL_REPLAY_PREAMBLE, b"before"].concat());
    assert!(store.detach("term-a", attached.subscription_id).is_ok());

    let legacy_store = store.clone();
    legacy_store.kill_project(&project_a);
    assert!(store.sessions_for_project(&project_a).is_empty());
    assert_eq!(store.sessions_for_project(&project_b).len(), 1);
    store.kill_session("term-b");
    assert!(legacy_store.sessions_for_project(&project_b).is_empty());
    store.shutdown();
    store.wait_for_idle().await.unwrap();
}

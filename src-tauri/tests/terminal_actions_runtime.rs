use std::cell::Cell;
use std::path::PathBuf;

use futures_util::FutureExt;
use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::{terminal_actions, AppState};
use taide_terminal::service::MAX_LINK_CANDIDATES_PER_ROW;
use taide_terminal::store::TerminalStore;
use uuid::Uuid;

const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

struct Fixture(AppState);

impl Fixture {
    fn new() -> Self {
        Self(AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-terminal-actions-{}", Uuid::new_v4())),
        )))
    }

    fn project(&self) -> ProjectId {
        let project = Project {
            id: ProjectId::new(),
            root: self.0.paths.data_dir.join("root").to_string_lossy().into_owned(),
            name: "fixture".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        let id = project.id.clone();
        self.0.projects.write().insert(id.clone(), project);
        id
    }

    fn files(&self) -> (ProjectId, PathBuf, PathBuf) {
        let id = self.project();
        let root = self.0.paths.data_dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let inside = root.join("fixture.txt");
        let outside = self.0.paths.data_dir.join("outside.txt");
        std::fs::write(&inside, b"fixture").unwrap();
        std::fs::write(&outside, b"fixture").unwrap();
        (id, inside.canonicalize().unwrap(), outside.canonicalize().unwrap())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.0.paths.data_dir).expect("자기 UUID 파일 fixture만 정리");
        }
    }
}

#[tokio::test]
async fn 없는_session의_오류와_detach의_멱등성을_유지한다() {
    let fixture = Fixture::new();
    let store = TerminalStore::new();
    for error in [
        terminal_actions::pty_resize(&store, "missing".to_string(), DEFAULT_COLS, DEFAULT_ROWS)
            .await
            .unwrap_err(),
        terminal_actions::pty_set_paused(&store, "missing".to_string(), true)
            .await
            .unwrap_err(),
        terminal_actions::pty_kill(&fixture.0, &store, "missing".to_string())
            .await
            .unwrap_err(),
    ] {
        assert_eq!(error.kind(), AppErrorKind::NotFound);
    }
    terminal_actions::pty_detach(&fixture.0, &store, "missing".to_string(), 1)
        .await
        .unwrap();
    assert!(terminal_actions::terminal_sessions(&store, ProjectId::new())
        .await
        .unwrap()
        .is_empty());
    assert!(!fixture.0.paths.data_dir.exists());
}

#[tokio::test]
async fn attach는_mutation_guard_뒤_sink_factory를_생성한다() {
    let fixture = Fixture::new();
    let store = TerminalStore::new();
    let called = Cell::new(false);
    let error = terminal_actions::pty_attach(&fixture.0, &store, "missing".to_string(), || {
        called.set(true);
        assert!(fixture.0.begin_mutation().now_or_never().is_none());
        |_: &[u8]| true
    })
    .await
    .unwrap_err();
    assert!(called.get());
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(fixture.0.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn guard를_기다리는_attach는_sink_factory를_선실행하지_않는다() {
    let fixture = Fixture::new();
    let store = TerminalStore::new();
    let _guard = fixture.0.begin_mutation().await;
    let called = Cell::new(false);
    let result = terminal_actions::pty_attach(&fixture.0, &store, "missing".to_string(), || {
        called.set(true);
        |_: &[u8]| true
    })
    .now_or_never();
    assert!(result.is_none());
    assert!(!called.get());
    assert!(terminal_actions::pty_kill(&fixture.0, &store, "missing".to_string())
        .now_or_never()
        .is_none());
    assert!(terminal_actions::pty_detach(&fixture.0, &store, "missing".to_string(), 1)
        .now_or_never()
        .is_none());
}

#[tokio::test]
async fn default_options는_프로젝트_gate와_기존_설정_기본값을_유지한다() {
    let fixture = Fixture::new();
    let error = terminal_actions::pty_default_options(&fixture.0, ProjectId::new(), None)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    let id = fixture.project();
    fixture.0.settings.write().shell_override = Some("fixture-shell".to_string());
    let options = terminal_actions::pty_default_options(&fixture.0, id.clone(), None).await.unwrap();
    assert_eq!(options.project_id, id);
    assert_eq!(options.cwd, fixture.0.paths.data_dir.join("root").to_string_lossy());
    assert_eq!(options.shell.as_deref(), Some("fixture-shell"));
    assert_eq!((options.cols, options.rows), (DEFAULT_COLS, DEFAULT_ROWS));
    assert!(options.scrollback_bytes.is_none());
    assert!(!fixture.0.paths.data_dir.exists());
}

#[tokio::test]
async fn default_options의_요청_cwd는_기존_root_guard를_사용한다() {
    let fixture = Fixture::new();
    let (id, inside, outside) = fixture.files();
    let options = terminal_actions::pty_default_options(&fixture.0, id.clone(), Some(inside.to_string_lossy().into_owned()))
        .await
        .unwrap();
    assert_eq!(options.cwd, inside.to_string_lossy());
    let error = terminal_actions::pty_default_options(&fixture.0, id, Some(outside.to_string_lossy().into_owned()))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
}

#[tokio::test]
async fn link는_열린_root_안의_canonical_path만_반환하고_거절_사유를_노출하지_않는다() {
    let fixture = Fixture::new();
    let (_, inside, outside) = fixture.files();
    let cwd = inside.parent().unwrap().to_string_lossy().into_owned();
    assert_eq!(
        terminal_actions::resolve_terminal_path(&fixture.0, "fixture.txt:12:3".to_string(), cwd.clone())
            .await
            .unwrap(),
        inside.to_string_lossy()
    );
    for path in [outside.to_string_lossy().into_owned(), "missing.txt".to_string()] {
        let error = terminal_actions::resolve_terminal_path(&fixture.0, path, cwd.clone())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::NotFound);
    }
    let mut candidates = vec!["fixture.txt".to_string(); MAX_LINK_CANDIDATES_PER_ROW + 1];
    candidates[0] = outside.to_string_lossy().into_owned();
    let resolved = terminal_actions::terminal_resolve_link_candidates(&fixture.0, cwd, candidates)
        .await
        .unwrap();
    assert_eq!(resolved.len(), MAX_LINK_CANDIDATES_PER_ROW + 1);
    assert!(resolved[0].is_none());
    assert_eq!(resolved[1].as_deref(), inside.to_str());
    assert!(resolved[MAX_LINK_CANDIDATES_PER_ROW].is_none());
}

#[test]
fn 열_action은_runtime에_위임하고_spawn과_write는_이_단위의_범위가_아니다() {
    let source = include_str!("../src/domain/terminal/commands.rs");
    for name in [
        "pty_resize",
        "pty_kill",
        "pty_set_paused",
        "pty_attach",
        "pty_detach",
        "terminal_sessions",
        "shell_profiles",
        "resolve_terminal_path",
        "terminal_resolve_link_candidates",
        "pty_default_options",
    ] {
        let body = source
            .split_once(&format!("pub async fn {name}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains(&format!("terminal_actions::{name}(")), "{name}");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn 자기_pty의_attach_replay_detach_resize_pause_kill은_같은_store를_사용한다() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use taide_terminal::metadata::TerminalSessionMetadata;
    use taide_terminal::session::{TerminalSessionOutput, TERMINAL_REPLAY_PREAMBLE};
    use taide_terminal::store::TerminalSessionEntry;

    const SCROLLBACK_BYTES: usize = 64 * 1024;
    const FIXTURE_TIMEOUT_MS: u64 = 2_000;

    let fixture = Fixture::new();
    let (id, inside, _) = fixture.files();
    let cwd = inside.parent().unwrap().to_string_lossy().into_owned();
    let store = TerminalStore::new();
    let output = Arc::new(TerminalSessionOutput::new(SCROLLBACK_BYTES));
    output.append_and_broadcast(b"before");
    let pty = store
        .begin_spawn()
        .unwrap()
        .spawn(|| {
            taide_infra::pty::spawn(
                taide_infra::pty::PtySpawnConfig {
                    shell: Some("/bin/sh".to_string()),
                    cwd: cwd.clone(),
                    cols: DEFAULT_COLS,
                    rows: DEFAULT_ROWS,
                    extra_env: vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())],
                },
                |_| {},
                |_| {},
            )
        })
        .unwrap();
    let completion = pty.completion_handle();
    let metadata = Arc::new(TerminalSessionMetadata::new(id.clone(), cwd.clone(), "fixture-shell".to_string()));
    store
        .insert("fixture".to_string(), TerminalSessionEntry::new(pty, metadata, output.clone()))
        .unwrap();
    let sessions = terminal_actions::terminal_sessions(&store, id).await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, "fixture");
    let received = Arc::new(Mutex::new(Vec::new()));
    let subscriber = received.clone();
    let attached = terminal_actions::pty_attach(&fixture.0, &store, "fixture".to_string(), || {
        assert!(fixture.0.begin_mutation().now_or_never().is_none());
        move |bytes: &[u8]| {
            subscriber.lock().unwrap().extend_from_slice(bytes);
            true
        }
    })
    .await
    .unwrap();
    assert_eq!(*received.lock().unwrap(), [TERMINAL_REPLAY_PREAMBLE, b"before"].concat());
    output.append_and_broadcast(b"after");
    let before_detach = received.lock().unwrap().clone();
    terminal_actions::pty_detach(&fixture.0, &store, "fixture".to_string(), attached.subscription_id)
        .await
        .unwrap();
    output.append_and_broadcast(b"detached");
    assert_eq!(*received.lock().unwrap(), before_detach);
    terminal_actions::pty_resize(&store, "fixture".to_string(), DEFAULT_COLS, DEFAULT_ROWS)
        .await
        .unwrap();
    terminal_actions::pty_set_paused(&store, "fixture".to_string(), true).await.unwrap();
    terminal_actions::pty_set_paused(&store, "fixture".to_string(), false)
        .await
        .unwrap();
    terminal_actions::pty_kill(&fixture.0, &store, "fixture".to_string()).await.unwrap();
    assert!(store.cwd("fixture").is_none());
    store.shutdown();
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), store.wait_for_idle())
        .await
        .unwrap()
        .unwrap();
    assert!(completion.is_finished());
}

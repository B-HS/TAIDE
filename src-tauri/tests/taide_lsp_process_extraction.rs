use std::ffi::OsStr;
use std::path::PathBuf;

use taide_lsp::process::{resolve_process_config, shutdown_process, spawn_language_server};
use taide_model::error::{AppError, AppErrorKind};
use taide_model::lsp::{LanguageServerSpec, LspCommandSpec, LspInstallSpec, LspInstallStrategy, LspRootStrategy, LspServerId};
use taide_model::paths::AppPaths;

const PROCESS_EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

fn test_spec(bin: String, args: Vec<String>) -> LanguageServerSpec {
    LanguageServerSpec {
        id: LspServerId::from("test-server"),
        name: "Test Server".to_string(),
        language_ids: vec!["rust".to_string()],
        shares_sessions: false,
        command: LspCommandSpec::Path { bin, args },
        root_markers: vec![],
        root_strategy: LspRootStrategy::NearestMarker,
        initialization_options: None,
        install: LspInstallSpec {
            strategy: LspInstallStrategy::Toolchain,
            hint: None,
            download: None,
            toolchain: None,
            sdk_detect: None,
        },
    }
}

#[test]
fn lsp_프로세스_설정은_독립_crate에서_실행파일과_workspace_인자를_해석한다() {
    let executable = std::env::current_exe().unwrap();
    let root = std::env::temp_dir();
    let paths = AppPaths::new(PathBuf::from("/data"));
    let spec = test_spec(executable.to_string_lossy().to_string(), vec!["--root={workspaceDir}".to_string()]);

    let config = resolve_process_config(&paths, &spec, root.to_str().unwrap(), OsStr::new("")).unwrap();

    assert_eq!(config.command, executable.to_string_lossy());
    assert_eq!(config.args, vec![format!("--root={}", root.display())]);
    assert_eq!(config.cwd, root);
}

#[test]
fn lsp_프로세스_설정은_없는_실행파일과_미해결_템플릿을_거부한다() {
    let paths = AppPaths::new(PathBuf::from("/data"));
    let root = std::env::temp_dir();
    let root = root.to_str().unwrap();
    let missing = test_spec("taide-lsp-test-missing-executable".to_string(), vec![]);
    let error = resolve_process_config(&paths, &missing, root, OsStr::new("")).err().unwrap();
    assert_eq!(error.kind(), AppErrorKind::NotFound);

    let executable = std::env::current_exe().unwrap();
    let unresolved = test_spec(executable.to_string_lossy().to_string(), vec!["{serverDir}".to_string()]);
    let error = resolve_process_config(&paths, &unresolved, root, OsStr::new("")).err().unwrap();
    let AppError::Localized(localized) = error else {
        panic!("미해결 템플릿은 기존 지역화 오류를 유지해야 합니다");
    };
    assert_eq!(localized.key, "error.lsp.unresolvedArgTemplate");
    assert_eq!(localized.args.get("serverId").map(String::as_str), Some("test-server"));
}

#[cfg(unix)]
#[tokio::test]
async fn lsp_프로세스는_독립_crate에서_실행되고_종료_콜백을_호출한다() {
    let paths = AppPaths::new(PathBuf::from("/data"));
    let root = std::env::temp_dir();
    let spec = test_spec("/bin/sh".to_string(), vec!["-c".to_string(), "exit 0".to_string()]);
    let (sender, receiver) = tokio::sync::oneshot::channel();

    let process = spawn_language_server(
        &paths,
        &spec,
        root.to_str().unwrap(),
        |_| {},
        move |code, _tail| {
            let _ = sender.send(code);
        },
    )
    .unwrap();
    let code = tokio::time::timeout(PROCESS_EXIT_TIMEOUT, receiver).await.unwrap().unwrap();

    assert_eq!(code, Some(0));
    assert!(process.is_exited());
}

#[cfg(unix)]
#[tokio::test]
async fn lsp_종료_정책은_서버가_먼저_끝나면_타임아웃_전_반환한다() {
    let paths = AppPaths::new(PathBuf::from("/data"));
    let root = std::env::temp_dir();
    let spec = test_spec("/bin/sh".to_string(), vec!["-c".to_string(), "read _; exit 0".to_string()]);
    let process = spawn_language_server(&paths, &spec, root.to_str().unwrap(), |_| {}, |_, _| {}).unwrap();

    tokio::time::timeout(PROCESS_EXIT_TIMEOUT, shutdown_process(&process))
        .await
        .unwrap();

    assert!(process.is_exited());
}

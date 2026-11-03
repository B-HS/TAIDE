use std::path::PathBuf;

use super::*;

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("taide-agent-host-cli-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn 자기_pid_batch조회_빈probe와_비agent_cache는_원본동작을_유지한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let agents = AgentStore::new();
    assert!(resolve_process_infos(&[]).is_empty());
    assert!(resolve_agent_names(&[]).is_empty());
    assert!(detect_agents_for_pids_blocking(&tasks, &agents, Vec::new())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(tasks.tracked_count(), 0);
    let own = std::process::id();
    let infos = resolve_process_infos(&[own, own]);
    let info = infos.get(&own).unwrap();
    assert!(!info.comm.is_empty());
    assert!(!info.cmdline.is_empty());
    let pids = vec![("synthetic".into(), own)];
    assert_eq!(agents.unresolved_pids(&pids), [own]);
    assert!(detect_agents_for_pids_blocking(&tasks, &agents, pids.clone())
        .await
        .unwrap()
        .is_empty());
    assert!(agents.unresolved_pids(&pids).is_empty());
    agents.retain_process_names(&std::collections::HashSet::new());
    assert_eq!(agents.unresolved_pids(&pids), [own]);
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
}

#[cfg(unix)]
#[test]
fn 실제_cli_metadata는_미설치_일반파일_정상링크_끊어진링크를_구분한다() {
    let directory = Directory::new();
    let missing = directory.0.join("missing");
    let absent = cli_install_status(&missing);
    assert!(!absent.installed);
    assert!(!absent.dangling);
    assert!(absent.resolved_path.is_none());
    assert_eq!(absent.target_path, missing.to_str().unwrap());
    let file = directory.0.join("synthetic-cli");
    std::fs::write(&file, b"synthetic-cli-fixture").unwrap();
    let present = cli_install_status(&file);
    assert!(present.installed);
    assert!(!present.dangling);
    assert_eq!(present.resolved_path, Some(file.canonicalize().unwrap().to_str().unwrap().into()));
    let link = directory.0.join("link");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    let resolved = cli_install_status(&link);
    assert!(resolved.installed);
    assert!(!resolved.dangling);
    assert_eq!(resolved.resolved_path, present.resolved_path);
    let dangling = directory.0.join("dangling");
    std::os::unix::fs::symlink(missing, &dangling).unwrap();
    let unresolved = cli_install_status(&dangling);
    assert!(unresolved.installed);
    assert!(unresolved.dangling);
    assert!(unresolved.resolved_path.is_none());
}

#[cfg(unix)]
struct Bundle {
    directory: Directory,
    exe: PathBuf,
    sidecar: PathBuf,
    source: PathBuf,
}

#[cfg(unix)]
impl Bundle {
    fn new() -> Self {
        let directory = Directory::new();
        let macos = directory.0.join("TAIDE.app/Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        let sidecar = macos.join("taide-cli");
        std::fs::write(&sidecar, b"synthetic-sidecar").unwrap();
        let bin = directory.0.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        Self {
            exe: macos.join("taide"),
            sidecar,
            source: bin.join("taide"),
            directory,
        }
    }
}

#[cfg(unix)]
fn refuse_script(_: &str) -> AppResult<AdminPrompt> {
    panic!("administrator script must not run")
}

#[cfg(unix)]
#[test]
fn cli_설치_해제는_번들_밖_실행파일을_원본_문구로_거절하고_스크립트를_부르지_않는다() {
    let bundle = Bundle::new();
    let outside = bundle.directory.0.join("target/debug/taide-native");
    for result in [
        cli_install(&outside, &bundle.source, refuse_script),
        cli_uninstall(&outside, &bundle.source, refuse_script),
    ] {
        let Err(AppError::InvalidArgument(message)) = result else {
            panic!("dev build must be refused: {result:?}");
        };
        assert_eq!(message, CLI_BUNDLE_ONLY);
    }
    assert!(std::fs::symlink_metadata(&bundle.source).is_err());
}

#[cfg(unix)]
#[test]
fn cli_설치는_같은_대상이면_생략하고_아니면_관리자_스크립트로_연결하며_취소를_구분한다() {
    let bundle = Bundle::new();
    let mut scripts = Vec::new();
    let cancelled = cli_install(&bundle.exe, &bundle.source, |script| {
        scripts.push(script.to_string());
        Ok(AdminPrompt::Cancelled)
    })
    .unwrap();
    assert!(cancelled.is_cancelled);
    assert!(!cancelled.status.installed);
    assert_eq!(scripts.len(), 1);
    assert!(scripts[0].contains("ln -sf"), "{}", scripts[0]);
    assert!(scripts[0].contains(bundle.sidecar.to_str().unwrap()));
    assert!(scripts[0].contains(bundle.source.to_str().unwrap()));
    assert!(scripts[0].ends_with("with administrator privileges"));
    let installed = cli_install(&bundle.exe, &bundle.source, |_| {
        std::os::unix::fs::symlink(&bundle.sidecar, &bundle.source).unwrap();
        Ok(AdminPrompt::Completed)
    })
    .unwrap();
    assert!(!installed.is_cancelled);
    assert!(installed.status.installed);
    assert!(!installed.status.dangling);
    assert_eq!(installed.status.target_path, bundle.source.to_str().unwrap());
    let repeated = cli_install(&bundle.exe, &bundle.source, refuse_script).unwrap();
    assert_eq!(repeated, installed);
    let failed = cli_install(&bundle.exe, &bundle.directory.0.join("bin/other"), |_| {
        Err(AppError::Internal("osascript failed: synthetic".into()))
    });
    assert!(matches!(failed, Err(AppError::Internal(message)) if message == "osascript failed: synthetic"));
}

#[cfg(unix)]
#[test]
fn cli_해제는_없으면_현재상태_소유아니면_거절_소유링크는_직접_지운다() {
    let bundle = Bundle::new();
    let absent = cli_uninstall(&bundle.exe, &bundle.source, refuse_script).unwrap();
    assert!(!absent.is_cancelled);
    assert!(!absent.status.installed);
    let foreign = bundle.directory.0.join("bin/other");
    std::fs::write(&foreign, b"foreign").unwrap();
    std::os::unix::fs::symlink(&foreign, &bundle.source).unwrap();
    let refused = cli_uninstall(&bundle.exe, &bundle.source, refuse_script);
    assert!(matches!(refused, Err(AppError::InvalidArgument(message)) if message == CLI_NOT_MANAGED));
    assert!(std::fs::symlink_metadata(&bundle.source).is_ok());
    std::fs::remove_file(&bundle.source).unwrap();
    std::os::unix::fs::symlink(&bundle.sidecar, &bundle.source).unwrap();
    let removed = cli_uninstall(&bundle.exe, &bundle.source, refuse_script).unwrap();
    assert!(!removed.is_cancelled);
    assert!(!removed.status.installed);
    assert!(std::fs::symlink_metadata(&bundle.source).is_err());
    assert!(bundle.sidecar.exists());
}

#[cfg(unix)]
#[test]
fn cli_해제는_권한이_없으면_관리자_스크립트로_지우고_취소를_구분한다() {
    use std::os::unix::fs::PermissionsExt;
    const READ_ONLY_DIRECTORY: u32 = 0o555;
    const WRITABLE_DIRECTORY: u32 = 0o755;
    let bundle = Bundle::new();
    std::os::unix::fs::symlink(&bundle.sidecar, &bundle.source).unwrap();
    let bin = bundle.directory.0.join("bin");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(READ_ONLY_DIRECTORY)).unwrap();
    let mut scripts = Vec::new();
    let cancelled = cli_uninstall(&bundle.exe, &bundle.source, |script| {
        scripts.push(script.to_string());
        Ok(AdminPrompt::Cancelled)
    });
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(WRITABLE_DIRECTORY)).unwrap();
    let cancelled = cancelled.unwrap();
    assert!(cancelled.is_cancelled);
    assert!(cancelled.status.installed);
    assert_eq!(scripts.len(), 1);
    assert!(scripts[0].starts_with("do shell script \"rm "), "{}", scripts[0]);
    assert!(scripts[0].contains(bundle.source.to_str().unwrap()));
}

#[cfg(unix)]
#[test]
fn 관리자_osascript_실행기는_성공과_실패를_나누고_실패는_원본_문구로_알린다() {
    assert_eq!(
        run_admin_osascript(Path::new("/usr/bin/true"), "synthetic").unwrap(),
        AdminPrompt::Completed
    );
    let failed = run_admin_osascript(Path::new("/usr/bin/false"), "synthetic");
    assert!(matches!(failed, Err(AppError::Internal(message)) if message.starts_with("osascript failed: ")));
}

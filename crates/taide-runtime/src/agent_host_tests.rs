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

#![cfg(unix)]

use std::path::Path;

use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_runtime::{AppState, terminal_env};

struct Fixture(std::path::PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[tokio::test]
async fn 원본_cli_소유와_sidecar_fallback_공백경로를_보존한다() {
    let fixture = Fixture(
        std::env::temp_dir().join(format!("taide-native-terminal-env-{}", ProjectId::new())),
    );
    let bundle = fixture.0.join("Synthetic.app/Contents/MacOS");
    std::fs::create_dir_all(&bundle).unwrap();
    let executable = bundle.join("Synthetic");
    let sidecar = bundle.join("taide-cli");
    let alias = fixture.0.join("taide");
    std::fs::write(&sidecar, b"synthetic non-executable fixture").unwrap();
    std::os::unix::fs::symlink(&sidecar, &alias).unwrap();
    assert_eq!(
        terminal_env::editor_cli_path(&alias, &executable),
        Some(alias.to_str().unwrap().into())
    );
    let foreign = fixture.0.join("foreign-editor");
    std::fs::write(&foreign, b"synthetic foreign fixture").unwrap();
    let foreign_alias = fixture.0.join("foreign-alias");
    std::os::unix::fs::symlink(&foreign, &foreign_alias).unwrap();
    assert_eq!(
        terminal_env::editor_cli_path(&foreign_alias, &executable),
        Some(sidecar.to_str().unwrap().into())
    );
    assert_eq!(
        terminal_env::editor_cli_path(&foreign_alias, Path::new("/synthetic/dev-executable")),
        None
    );
    let missing = fixture.0.join("missing-sidecar");
    let dangling = fixture.0.join("dangling");
    std::os::unix::fs::symlink(&missing, &dangling).unwrap();
    assert_eq!(
        terminal_env::editor_cli_path(&dangling, &executable),
        Some(sidecar.to_str().unwrap().into())
    );
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    state.settings.write().ide_integration_enabled = false;
    let ide = taide_ide::store::IdeStore::default();
    let env =
        terminal_env::resolve(&state, &ide, "synthetic", Some("/synthetic space/taide")).await;
    assert_eq!(
        env,
        vec![
            ("TAIDE_AGENT_PROTOCOL_VERSION".into(), "1".into()),
            ("TAIDE_APP_VERSION".into(), "synthetic".into()),
        ]
    );
}

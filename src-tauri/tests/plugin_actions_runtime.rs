use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use taide_model::error::AppErrorKind;
use taide_model::paths::AppPaths;
use taide_model::plugin::{PluginContributions, PluginLanguageContribution, PluginManifest, PLUGIN_MANIFEST_FILE, PLUGIN_MANIFEST_VERSION};
use taide_model::vsix::VSIX_MANIFEST_ENTRY;
use taide_plugin::service::PluginStore;
use taide_runtime::{plugin_actions, vsix_actions, AppState, TaskSupervisor};
use uuid::Uuid;
use zip::write::SimpleFileOptions;

const FIXTURE_PLUGIN_ID: &str = "fixture-plugin";
const FIXTURE_GRAMMAR: &str = r#"{ "scopeName": "source.fixture", "patterns": [] }"#;
const STAGE_FAILURE_TIMEOUT_MS: u64 = 1_000;
const STAGING_WAIT_TIMEOUT_MS: u64 = 2_000;
const STAGING_POLL_INTERVAL_MS: u64 = 10;
const PLUGIN_COMMANDS: &[&str] = &[
    "plugin_list",
    "plugin_reload",
    "plugin_install",
    "plugin_uninstall",
    "plugin_read_grammar",
];
const VSIX_COMMANDS: &[&str] = &["vsix_extract_themes", "vsix_import_plugin"];

struct Fixture(AppState);

impl Fixture {
    fn new() -> Self {
        Self(AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-plugin-actions-{}", Uuid::new_v4())),
        )))
    }

    fn source(&self) -> PathBuf {
        let source = self.0.paths.data_dir.join("source");
        std::fs::create_dir_all(source.join("grammars")).unwrap();
        std::fs::write(source.join(PLUGIN_MANIFEST_FILE), serde_json::to_vec(&manifest()).unwrap()).unwrap();
        std::fs::write(source.join("grammars/fixture.json"), FIXTURE_GRAMMAR).unwrap();
        source
    }

    fn tasks(&self) -> TaskSupervisor {
        TaskSupervisor::new(tokio::runtime::Handle::current())
    }

    fn archive(&self, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        std::fs::create_dir_all(&self.0.paths.data_dir).unwrap();
        let path = self.0.paths.data_dir.join(name);
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (entry, bytes) in entries {
            writer.start_file(*entry, SimpleFileOptions::default()).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    fn vsix(&self) -> PathBuf {
        let package = serde_json::to_vec(&serde_json::json!({
            "name": "fixture",
            "publisher": "local",
            "version": "1.0.0",
            "contributes": {
                "languages": [{ "id": "fixture", "extensions": [".fixture"] }],
            },
        }))
        .unwrap();
        self.archive("fixture.vsix", &[(VSIX_MANIFEST_ENTRY, &package)])
    }

    fn assert_no_staged_bytes(&self) {
        let staging = self.0.paths.plugins_dir().join(".tmp");
        if staging.exists() {
            assert_eq!(std::fs::read_dir(staging).unwrap().count(), 0);
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.0.paths.data_dir).expect("자기 UUID fixture만 정리");
        }
    }
}

fn manifest() -> PluginManifest {
    PluginManifest {
        manifest_version: PLUGIN_MANIFEST_VERSION,
        id: FIXTURE_PLUGIN_ID.to_string(),
        name: "fixture".to_string(),
        version: "1.0.0".to_string(),
        contributes: PluginContributions {
            languages: vec![PluginLanguageContribution {
                id: "fixture".to_string(),
                extensions: vec![".fixture".to_string()],
                aliases: Vec::new(),
                grammar: Some("grammars/fixture.json".to_string()),
                embedded_languages: None,
            }],
            ..PluginContributions::default()
        },
    }
}

#[tokio::test]
async fn 요청_abort는_commit_guard를_기다리는_자기_staging을_정리한다() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let store = PluginStore::new();
    let state = fixture.0.clone();
    let _guard = state.begin_mutation().await;
    let operation_state = state.clone();
    let operation_store = store.clone();
    let tasks = fixture.tasks();
    let operation_tasks = tasks.clone();
    let request = tokio::spawn(async move {
        plugin_actions::plugin_install(
            &operation_state,
            &operation_store,
            &operation_tasks,
            source.to_string_lossy().into_owned(),
        )
        .await
    });
    tokio::time::timeout(Duration::from_millis(STAGING_WAIT_TIMEOUT_MS), async {
        loop {
            let staging = state.paths.plugins_dir().join(".tmp");
            if std::fs::read_dir(staging).is_ok_and(|entries| entries.count() > 0) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(STAGING_POLL_INTERVAL_MS)).await;
        }
    })
    .await
    .unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    tasks.shutdown().await;
    fixture.assert_no_staged_bytes();
    assert!(store.0.read().is_none());
    assert!(!state.paths.plugins_dir().join(FIXTURE_PLUGIN_ID).exists());
}

#[tokio::test]
async fn list는_cache를_재사용하고_reload는_현재_디스크_snapshot으로_교체한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let store = PluginStore::new();
    assert!(plugin_actions::plugin_list(state, &store).await.unwrap().is_empty());
    assert_eq!(*store.0.read(), Some(Vec::new()));
    assert!(!state.paths.data_dir.exists());
    let source = fixture.source();
    std::fs::create_dir_all(state.paths.plugins_dir()).unwrap();
    std::fs::rename(&source, state.paths.plugins_dir().join(FIXTURE_PLUGIN_ID)).unwrap();
    assert!(
        plugin_actions::plugin_list(state, &store).await.unwrap().is_empty(),
        "read-through의 기존 cache가 유지된다"
    );
    let loaded = plugin_actions::plugin_reload(state, &store).await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert!(loaded[0].enabled);
    assert_eq!(*store.0.read(), Some(loaded));
}

#[tokio::test]
async fn directory_install은_commit과_cache_뒤_grammar를_읽고_uninstall은_설치본만_제거한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let source = fixture.source();
    let store = PluginStore::new();
    let installed = plugin_actions::plugin_install(state, &store, &fixture.tasks(), source.to_string_lossy().into_owned())
        .await
        .unwrap();
    assert_eq!(installed.manifest, manifest());
    assert!(installed.enabled);
    assert_eq!(*store.0.read(), Some(vec![installed.clone()]));
    assert_eq!(plugin_actions::plugin_list(state, &store).await.unwrap(), [installed]);
    let grammar = plugin_actions::plugin_read_grammar(state, &store, FIXTURE_PLUGIN_ID.to_string(), "fixture".to_string())
        .await
        .unwrap();
    assert_eq!(grammar, FIXTURE_GRAMMAR);
    fixture.assert_no_staged_bytes();
    assert!(plugin_actions::plugin_uninstall(state, &store, FIXTURE_PLUGIN_ID.to_string())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(*store.0.read(), Some(Vec::new()));
    assert!(!state.paths.plugins_dir().join(FIXTURE_PLUGIN_ID).exists());
    assert!(
        source.join(PLUGIN_MANIFEST_FILE).exists(),
        "사용자 source에 해당하는 fixture 원본은 유지한다"
    );
}

#[tokio::test]
async fn archive_install도_같은_commit_cache와_grammar_경계를_사용한다() {
    let fixture = Fixture::new();
    let encoded = serde_json::to_vec(&manifest()).unwrap();
    let archive = fixture.archive(
        "plugin.zip",
        &[
            (PLUGIN_MANIFEST_FILE, &encoded),
            ("grammars/fixture.json", FIXTURE_GRAMMAR.as_bytes()),
        ],
    );
    let store = PluginStore::new();
    let installed = plugin_actions::plugin_install(&fixture.0, &store, &fixture.tasks(), archive.to_string_lossy().into_owned())
        .await
        .unwrap();
    assert!(installed.enabled);
    assert_eq!(*store.0.read(), Some(vec![installed]));
    assert_eq!(
        plugin_actions::plugin_read_grammar(&fixture.0, &store, FIXTURE_PLUGIN_ID.to_string(), "fixture".to_string())
            .await
            .unwrap(),
        FIXTURE_GRAMMAR
    );
    fixture.assert_no_staged_bytes();
    assert!(archive.exists());
}

#[tokio::test]
async fn 중복_install과_잘못된_uninstall은_기존_설치본과_cache를_유지한다() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let state = &fixture.0;
    let store = PluginStore::new();
    let installed = plugin_actions::plugin_install(state, &store, &fixture.tasks(), source.to_string_lossy().into_owned())
        .await
        .unwrap();
    let duplicate = plugin_actions::plugin_install(state, &store, &fixture.tasks(), source.to_string_lossy().into_owned())
        .await
        .unwrap_err();
    assert_eq!(duplicate.kind(), AppErrorKind::InvalidArgument);
    assert!(plugin_actions::plugin_uninstall(state, &store, "../source".to_string())
        .await
        .is_err());
    let missing = plugin_actions::plugin_uninstall(state, &store, "missing".to_string())
        .await
        .unwrap_err();
    assert_eq!(missing.kind(), AppErrorKind::NotFound);
    assert_eq!(*store.0.read(), Some(vec![installed]));
    assert!(state
        .paths
        .plugins_dir()
        .join(FIXTURE_PLUGIN_ID)
        .join(PLUGIN_MANIFEST_FILE)
        .exists());
    fixture.assert_no_staged_bytes();
}

#[tokio::test]
async fn stage_실패는_mutation_guard를_기다리지_않고_cache와_disk를_변경하지_않는다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let store = PluginStore::new();
    let _guard = state.begin_mutation().await;
    let result = tokio::time::timeout(
        Duration::from_millis(STAGE_FAILURE_TIMEOUT_MS),
        plugin_actions::plugin_install(
            state,
            &store,
            &fixture.tasks(),
            state.paths.data_dir.join("missing.zip").to_string_lossy().into_owned(),
        ),
    )
    .await
    .expect("stage 오류는 guard 전에 반환해야 한다");
    assert!(result.is_err());
    assert!(store.0.read().is_none());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn vsix_import는_stage_뒤_guard_안에서_주입된_commit과_cache를_완료한다() {
    let fixture = Fixture::new();
    let path = fixture.vsix();
    let state = &fixture.0;
    let store = PluginStore::new();
    let did_commit = Arc::new(AtomicBool::new(false));
    let marker = did_commit.clone();
    let commit_state = fixture.0.clone();
    let commit_store = store.clone();
    let installed = vsix_actions::vsix_import_plugin(
        state,
        &fixture.tasks(),
        move |temp_dir, plugin_id| {
            marker.store(true, Ordering::SeqCst);
            assert!(temp_dir.starts_with(commit_state.paths.plugins_dir().join(".tmp")));
            assert_eq!(plugin_id, "local-fixture");
            assert!(commit_state.begin_mutation().now_or_never().is_none());
            let installed = plugin_actions::commit_staged_vsix_plugin(&commit_state, &commit_store, temp_dir, plugin_id)?;
            assert!(!temp_dir.exists());
            Ok(installed)
        },
        path.to_string_lossy().into_owned(),
    )
    .await
    .unwrap();
    assert!(did_commit.load(Ordering::SeqCst));
    assert!(installed.enabled);
    assert_eq!(*store.0.read(), Some(vec![installed]));
    fixture.assert_no_staged_bytes();
    assert!(path.exists());
}

#[tokio::test]
async fn vsix_stage_오류는_guard나_commit_port_전에_반환한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let path = state.paths.data_dir.join("missing.vsix").to_string_lossy().into_owned();
    let _guard = state.begin_mutation().await;
    let result = tokio::time::timeout(
        Duration::from_millis(STAGE_FAILURE_TIMEOUT_MS),
        vsix_actions::vsix_import_plugin(
            state,
            &fixture.tasks(),
            |_: &Path, _: &str| panic!("stage 실패 뒤 commit 금지"),
            path.clone(),
        ),
    )
    .await
    .expect("stage 오류는 guard 전에 반환해야 한다");
    assert_eq!(result.unwrap_err().kind(), AppErrorKind::InvalidArgument);
    assert_eq!(
        vsix_actions::vsix_extract_themes(path).await.unwrap_err().kind(),
        AppErrorKind::InvalidArgument
    );
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn vsix_theme_조회는_기존_extension_shape를_읽고_plugin을_설치하지_않는다() {
    let fixture = Fixture::new();
    let path = fixture.vsix();
    let extracted = vsix_actions::vsix_extract_themes(path.to_string_lossy().into_owned())
        .await
        .unwrap();
    assert_eq!(extracted.extension.name, "fixture");
    assert_eq!(extracted.extension.publisher, "local");
    assert_eq!(extracted.extension.version, "1.0.0");
    assert!(extracted.themes.is_empty());
    assert!(!fixture.0.paths.plugins_dir().exists());
}

#[test]
fn 일곱_command와_root_commit은_runtime으로_위임한다() {
    for (source, commands, module) in [
        (include_str!("../src/domain/plugin/commands.rs"), PLUGIN_COMMANDS, "plugin_actions"),
        (include_str!("../src/domain/vsix/commands.rs"), VSIX_COMMANDS, "vsix_actions"),
    ] {
        for command in commands {
            let body = source
                .split_once(&format!("pub async fn {command}("))
                .unwrap()
                .1
                .split_once("\n}")
                .unwrap()
                .0;
            assert!(body.contains(&format!("{module}::{command}(")), "{command}");
        }
    }
    let root = include_str!("../src/lib.rs")
        .split_once("fn commit_staged_vsix_plugin(")
        .unwrap()
        .1
        .split_once("\n}")
        .unwrap()
        .0;
    assert!(root.contains("taide_runtime::plugin_actions::commit_staged_vsix_plugin("));
    assert!(!root.contains("service::commit_staged_install("));
}

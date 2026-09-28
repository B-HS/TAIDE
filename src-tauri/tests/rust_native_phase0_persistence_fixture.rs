use std::fs;

use serde_json::{json, Value};
use taide_model::{
    ai::AiProviderId,
    ids::ProjectId,
    layout::{PaneNode, TabKind},
    paths::AppPaths,
};
use uuid::Uuid;

const PERSISTENCE_FIXTURE: &str = include_str!("fixtures/rust-native/persistence-v1.json");

#[test]
fn legacy_settings는_현재_키와_기본값으로_복원된다() {
    let fixture: Value = serde_json::from_str(PERSISTENCE_FIXTURE).expect("persistence fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    let settings = taide_settings::service::parse_settings_json(&fixture["settingsLegacy"].to_string()).expect("legacy settings");

    assert_eq!(settings.version, taide_model::settings::SETTINGS_SCHEMA_VERSION);
    assert_eq!(settings.theme_id, "taide-dark");
    assert!(!settings.editor_minimap);
    assert_eq!(settings.ai_provider, Some(AiProviderId::Codex));
    assert!(settings.ide_integration_enabled);
    let saved = serde_json::to_value(settings).expect("current settings wire");
    assert_eq!(saved["aiProvider"], "codex");
    assert!(saved.get("aiAutoTabProvider").is_none());

    let data_dir = std::env::temp_dir().join(format!("taide-phase0-settings-{}", Uuid::new_v4()));
    fs::create_dir(&data_dir).expect("fixture settings directory");
    let paths = AppPaths::new(data_dir.clone());
    fs::write(paths.settings_file(), fixture["settingsLegacy"].to_string()).expect("legacy settings file");
    let loaded = taide_settings::service::load_settings(&paths);
    fs::remove_dir_all(&data_dir).expect("remove fixture data");
    assert_eq!(loaded.ai_provider, Some(AiProviderId::Codex));
    assert!(!loaded.editor_minimap);
}

#[test]
fn legacy_session과_project와_layout은_실제_저장_경로에서_복원된다() {
    let fixture: Value = serde_json::from_str(PERSISTENCE_FIXTURE).expect("persistence fixture");
    let data_dir = std::env::temp_dir().join(format!("taide-phase0-persistence-{}", Uuid::new_v4()));
    let workspace = data_dir.join("workspace");
    fs::create_dir_all(&workspace).expect("fixture workspace");
    let file = workspace.join("main.rs");
    fs::write(&file, "disk content").expect("fixture source");
    let paths = AppPaths::new(data_dir.clone());
    let project_id = ProjectId("prj-fixture".to_owned());
    fs::create_dir_all(paths.project_dir(&project_id)).expect("fixture project directory");

    let mut session = fixture["sessionLegacy"].clone();
    session["projects"][0]["root"] = json!(workspace.to_string_lossy());
    let mut project = fixture["projectLegacy"].clone();
    project["root"] = json!(workspace.to_string_lossy());
    fs::write(paths.session_file(), session.to_string()).expect("legacy session file");
    fs::write(paths.project_file(&project_id), project.to_string()).expect("legacy project file");
    let mut layout = fixture["layoutLegacy"].clone();
    layout["root"]["tabs"][0]["kind"]["path"] = json!(file.to_string_lossy());
    fs::write(paths.layout_file(&project_id), layout.to_string()).expect("legacy layout file");

    let (restored, projects, warnings) = taide_project::service::restore_session(&paths).expect("restore session");
    let layout = taide_layout::service::load_layout(&paths, &project_id);
    fs::remove_dir_all(&data_dir).expect("remove fixture data");

    assert!(warnings.is_empty());
    assert_eq!(restored.version, taide_model::project::SESSION_SCHEMA_VERSION);
    assert_eq!(restored.active_project, Some(project_id.clone()));
    assert!(restored.shell_slots.is_some());
    assert_eq!(restored.projects.len(), 1);
    assert!(!restored.projects[0].root_missing);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Fixture");
    assert_eq!(projects[0].last_opened_at, 0.0);
    assert!(!projects[0].root_missing);
    assert_eq!(layout.version, taide_model::layout::LAYOUT_SCHEMA_VERSION);
    assert_eq!(layout.focused_pane.as_str(), "pane-fixture");
    let PaneNode::Leaf { tabs, active, .. } = layout.root else {
        panic!("expected file pane")
    };
    assert_eq!(active.expect("active tab").as_str(), "tab-fixture");
    assert_eq!(tabs.len(), 1);
    assert!(matches!(&tabs[0].kind, TabKind::File { path } if path == &file.to_string_lossy()));
    assert!(tabs[0].dirty);
}

#[test]
fn legacy_hot_exit_mirror는_누락된_disk_baseline으로_복원된다() {
    let fixture: Value = serde_json::from_str(PERSISTENCE_FIXTURE).expect("persistence fixture");
    let data_dir = std::env::temp_dir().join(format!("taide-phase0-mirror-{}", Uuid::new_v4()));
    let workspace = data_dir.join("workspace");
    fs::create_dir_all(&workspace).expect("fixture workspace");
    let file = workspace.join("main.rs");
    fs::write(&file, "disk content").expect("fixture source");
    let paths = AppPaths::new(data_dir.clone());
    let project_id = ProjectId("prj-fixture".to_owned());
    taide_file::service::mirror_dirty(&paths, &project_id, &file, &file.to_string_lossy(), "temporary draft").expect("create mirror path");
    let mirror_file = fs::read_dir(paths.buffers_dir(&project_id))
        .expect("buffer directory")
        .next()
        .expect("mirror entry")
        .expect("mirror path")
        .path();
    let mut legacy = fixture["hotExitLegacy"].clone();
    legacy["path"] = json!(file.to_string_lossy());
    fs::write(mirror_file, legacy.to_string()).expect("legacy mirror file");

    let mirrors = taide_file::service::list_mirrors(&paths, &project_id).expect("restore mirrors");
    fs::remove_dir_all(&data_dir).expect("remove fixture data");

    assert_eq!(mirrors.len(), 1);
    assert_eq!(mirrors[0].content, "unsaved fixture content");
    assert_eq!(mirrors[0].saved_at_ms, 1000.0);
    assert_eq!(mirrors[0].disk_modified_ms, None);
    assert!(!mirrors[0].conflict);
    assert!(!mirrors[0].source_missing);
}

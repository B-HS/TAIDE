use std::sync::Arc;

use taide_lsp::session::LspMessageSubscribers;
use taide_lsp::store::{LspSessionEntry, LspStore};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LanguageServerSpec, LspCommandSpec, LspInstallSpec, LspInstallStrategy, LspRootStrategy, LspServerId};

#[test]
fn 같은_프로젝트_서버_owner만_세션을_재사용하고_종료_중이면_제외한다() {
    let project_id = ProjectId::new();
    let server_id = LspServerId::from("test-server");
    let spec = LanguageServerSpec {
        id: server_id.clone(),
        name: "Test Server".to_string(),
        language_ids: vec!["rust".to_string()],
        shares_sessions: true,
        command: LspCommandSpec::Path {
            bin: "test-lsp".to_string(),
            args: vec![],
        },
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
    };
    let subscribers = LspMessageSubscribers::new();
    subscribers.insert("owner-a".to_string(), |_| true);
    let entry = Arc::new(LspSessionEntry::new(
        project_id.clone(),
        spec,
        "/workspace".to_string(),
        subscribers,
    ));
    let store = LspStore::new();
    store.insert("session-a".to_string(), entry.clone());

    assert_eq!(store.get("session-a").expect("저장된 세션").root, "/workspace");
    let sessions = store.sessions_for_project(&project_id);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "session-a");
    assert_eq!(sessions[0].root, "/workspace");
    assert_eq!(
        store.find_reusable(&project_id, &server_id, "owner-a").expect("같은 owner 세션").0,
        "session-a"
    );
    assert!(store.find_reusable(&project_id, &server_id, "owner-b").is_none());

    entry.lifecycle.mark_stopping();
    assert!(store.find_reusable(&project_id, &server_id, "owner-a").is_none());
    assert!(store.remove("session-a").is_some());
    assert!(!store.contains("session-a"));
}

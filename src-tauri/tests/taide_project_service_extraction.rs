use taide_lib::domain::project::service as legacy_service;
use taide_model::error::AppResult;
use taide_model::project::{ProjectRef, SessionState};

#[test]
fn 프로젝트_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_list: fn(&SessionState) -> Vec<ProjectRef> = taide_project::service::list_projects;
    let legacy_list: fn(&SessionState) -> Vec<ProjectRef> = legacy_service::list_projects;
    assert!(std::ptr::fn_addr_eq(extracted_list, legacy_list));

    let extracted_migrate: fn(serde_json::Value) -> AppResult<SessionState> = taide_project::service::migrate_session;
    let legacy_migrate: fn(serde_json::Value) -> AppResult<SessionState> = legacy_service::migrate_session;
    assert!(std::ptr::fn_addr_eq(extracted_migrate, legacy_migrate));

    let restored = legacy_migrate(serde_json::json!({ "version": 1, "projects": [], "activeProject": null })).expect("구버전 세션");
    assert!(legacy_list(&restored).is_empty());
}

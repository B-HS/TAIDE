use taide_lib::domain::layout::service as legacy_service;
use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::layout::ProjectLayout;
use taide_model::paths::AppPaths;

#[test]
fn layout_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_default: fn() -> ProjectLayout = taide_layout::service::default_layout;
    let legacy_default: fn() -> ProjectLayout = legacy_service::default_layout;
    assert!(std::ptr::fn_addr_eq(extracted_default, legacy_default));

    let extracted_save: fn(&AppPaths, &ProjectId, &ProjectLayout) -> AppResult<()> = taide_layout::service::save_layout;
    let legacy_save: fn(&AppPaths, &ProjectId, &ProjectLayout) -> AppResult<()> = legacy_service::save_layout;
    assert!(std::ptr::fn_addr_eq(extracted_save, legacy_save));

    let extracted_load: fn(&AppPaths, &ProjectId) -> ProjectLayout = taide_layout::service::load_layout;
    let legacy_load: fn(&AppPaths, &ProjectId) -> ProjectLayout = legacy_service::load_layout;
    assert!(std::ptr::fn_addr_eq(extracted_load, legacy_load));
}

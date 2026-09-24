use std::path::PathBuf;

use taide_infra::perf::PerfRegistry;
use taide_lib::domain::app::service as legacy_service;
use taide_model::app::{AppFileTarget, PerfSnapshot};
use taide_model::paths::AppPaths;

#[test]
fn app_파일과_성능_경로는_독립_crate와_같다() {
    let extracted_path: fn(&AppPaths, AppFileTarget) -> PathBuf = taide_app::service::app_file_path;
    let legacy_path: fn(&AppPaths, AppFileTarget) -> PathBuf = legacy_service::app_file_path;
    assert!(std::ptr::fn_addr_eq(extracted_path, legacy_path));

    let extracted_snapshot: fn(&PerfRegistry) -> PerfSnapshot = taide_app::service::perf_snapshot;
    let legacy_snapshot: fn(&PerfRegistry) -> PerfSnapshot = legacy_service::perf_snapshot;
    assert!(std::ptr::fn_addr_eq(extracted_snapshot, legacy_snapshot));

    assert_eq!(legacy_service::app_info().version, env!("CARGO_PKG_VERSION"));
}

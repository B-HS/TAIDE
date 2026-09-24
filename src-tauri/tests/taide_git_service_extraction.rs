use std::path::{Path, PathBuf};

use taide_lib::domain::git::service as legacy_service;
use taide_model::error::AppResult;
use taide_model::git::{GitStatus, StagedDiffText};

#[test]
fn git_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_discover: fn(&Path) -> AppResult<PathBuf> = taide_git::service::discover;
    let legacy_discover: fn(&Path) -> AppResult<PathBuf> = legacy_service::discover;
    assert!(std::ptr::fn_addr_eq(extracted_discover, legacy_discover));

    let extracted_status: fn(&Path) -> AppResult<GitStatus> = taide_git::service::status;
    let legacy_status: fn(&Path) -> AppResult<GitStatus> = legacy_service::status;
    assert!(std::ptr::fn_addr_eq(extracted_status, legacy_status));

    let extracted_diff: fn(&Path) -> AppResult<StagedDiffText> = taide_git::service::diff_staged_text;
    let legacy_diff: fn(&Path) -> AppResult<StagedDiffText> = legacy_service::diff_staged_text;
    assert!(std::ptr::fn_addr_eq(extracted_diff, legacy_diff));
}

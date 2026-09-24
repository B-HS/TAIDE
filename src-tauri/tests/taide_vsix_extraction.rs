use std::path::Path;

use taide_lib::domain::vsix::{service as legacy_service, types as legacy_types};
use taide_model::error::AppResult;
use taide_model::vsix::VsixThemeExtractionResult;

#[test]
fn vsix_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted: fn(&Path) -> AppResult<VsixThemeExtractionResult> = taide_vsix::service::extract_themes;
    let legacy: fn(&Path) -> AppResult<VsixThemeExtractionResult> = legacy_service::extract_themes;
    assert!(std::ptr::fn_addr_eq(extracted, legacy));

    assert_eq!(taide_model::vsix::VSIX_MANIFEST_ENTRY, legacy_types::VSIX_MANIFEST_ENTRY);
    assert_eq!(taide_model::vsix::VSIX_ENTRY_MAX_BYTES, legacy_types::VSIX_ENTRY_MAX_BYTES);
}

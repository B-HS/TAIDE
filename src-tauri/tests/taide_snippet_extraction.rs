use std::path::PathBuf;

use taide_lib::domain::snippet::service as legacy_service;
use taide_model::error::{AppErrorKind, AppResult};
use taide_model::paths::AppPaths;
use taide_model::snippet::SnippetFile;

#[test]
fn 스니펫_서비스는_독립_crate와_기존_경로에서_같은_안전_정책을_쓴다() {
    let extracted_list: fn(&AppPaths) -> Vec<SnippetFile> = taide_snippet::service::list_snippet_files;
    let legacy_list: fn(&AppPaths) -> Vec<SnippetFile> = legacy_service::list_snippet_files;
    let extracted_save: fn(&AppPaths, &str, &str) -> AppResult<SnippetFile> = taide_snippet::service::save_snippet_file;
    let legacy_save: fn(&AppPaths, &str, &str) -> AppResult<SnippetFile> = legacy_service::save_snippet_file;
    let extracted_delete: fn(&AppPaths, &str) -> AppResult<()> = taide_snippet::service::delete_snippet_file;
    let legacy_delete: fn(&AppPaths, &str) -> AppResult<()> = legacy_service::delete_snippet_file;

    assert!(std::ptr::fn_addr_eq(extracted_list, legacy_list));
    assert!(std::ptr::fn_addr_eq(extracted_save, legacy_save));
    assert!(std::ptr::fn_addr_eq(extracted_delete, legacy_delete));

    let paths = AppPaths::new(PathBuf::from("unused-snippet-boundary"));
    for name in ["../outside.json", "C:outside.json", "notes.txt"] {
        assert_eq!(
            extracted_save(&paths, name, "{}").unwrap_err().kind(),
            AppErrorKind::InvalidArgument
        );
        assert_eq!(legacy_delete(&paths, name).unwrap_err().kind(), AppErrorKind::InvalidArgument);
    }
}

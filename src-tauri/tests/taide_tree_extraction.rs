use std::path::PathBuf;

use taide_lib::domain::tree::service as legacy_service;
use taide_model::tree::TreeRowPage;

#[test]
fn 트리_상태와_페이지는_독립_crate와_기존_경로를_공유한다() {
    let _: Option<legacy_service::TreeState> = None::<taide_tree::service::TreeState>;
    let extracted_new: fn(PathBuf) -> taide_tree::service::TreeState = taide_tree::service::new_tree_state;
    let legacy_new: fn(PathBuf) -> legacy_service::TreeState = legacy_service::new_tree_state;
    let extracted_page: fn(&taide_tree::service::TreeState, u32, Option<u32>) -> TreeRowPage = taide_tree::service::rows_page;
    let legacy_page: fn(&legacy_service::TreeState, u32, Option<u32>) -> TreeRowPage = legacy_service::rows_page;

    assert!(std::ptr::fn_addr_eq(extracted_new, legacy_new));
    assert!(std::ptr::fn_addr_eq(extracted_page, legacy_page));
    let state = legacy_new(PathBuf::from("unused-tree-boundary"));
    assert_eq!(
        extracted_page(&state, 0, None),
        TreeRowPage {
            rows: Vec::new(),
            total: 0
        }
    );
}

use std::collections::HashMap;

use taide_lib::domain::window::service as legacy_service;

#[test]
fn window_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_normalize: fn(&str) -> &str = taide_window::service::normalize_window_state_label;
    let legacy_normalize: fn(&str) -> &str = legacy_service::normalize_window_state_label;
    assert!(std::ptr::fn_addr_eq(extracted_normalize, legacy_normalize));

    assert_eq!(
        taide_window::service::next_auxiliary_label(["main", "editor-1"].into_iter()),
        legacy_service::next_auxiliary_label(["main", "editor-1"].into_iter())
    );
    assert_eq!(
        taide_window::service::restorable_project_ids(&HashMap::new()),
        legacy_service::restorable_project_ids(&HashMap::new())
    );
    assert_eq!(
        taide_window::service::AUXILIARY_WINDOW_LABEL_PREFIX,
        taide_lib::domain::window::types::AUXILIARY_WINDOW_LABEL_PREFIX
    );
}

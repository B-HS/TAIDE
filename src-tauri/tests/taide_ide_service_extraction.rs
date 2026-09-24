use std::collections::HashMap;

use taide_infra::language::LanguageOverlay;
use taide_lib::domain::ide::service as legacy_service;

#[test]
fn ide_서비스는_독립_crate와_기존_경로에서_같다() {
    let extracted_language: fn(&str, &[LanguageOverlay]) -> String = taide_ide::service::guess_language_id;
    let legacy_language: fn(&str, &[LanguageOverlay]) -> String = legacy_service::guess_language_id;
    assert!(std::ptr::fn_addr_eq(extracted_language, legacy_language));

    let legacy_entry: Option<legacy_service::OpenEditorEntry> = None;
    let _: Option<taide_ide::service::OpenEditorEntry> = legacy_entry;
    assert_eq!(
        taide_ide::service::open_editors_snapshot(&HashMap::new(), &[]),
        legacy_service::open_editors_snapshot(&HashMap::new(), &[])
    );

    assert_eq!(
        taide_ide::service::IDE_PORT_RANGE_START,
        taide_lib::domain::ide::types::IDE_PORT_RANGE_START
    );
}

use std::any::TypeId;

use taide_lib::domain::agent::service as legacy_service;

#[test]
fn 에이전트_정책은_독립_crate와_기존_경로에서_같다() {
    let extracted_detect: fn(&str, &str) -> Option<&'static str> = taide_agent::service::detect_agent_name;
    let legacy_detect: fn(&str, &str) -> Option<&'static str> = legacy_service::detect_agent_name;
    assert!(std::ptr::fn_addr_eq(extracted_detect, legacy_detect));

    assert_eq!(
        TypeId::of::<taide_agent::service::AgentEvent>(),
        TypeId::of::<legacy_service::AgentEvent>()
    );

    assert_eq!(legacy_detect("codex", "codex --help"), Some("codex"));
}

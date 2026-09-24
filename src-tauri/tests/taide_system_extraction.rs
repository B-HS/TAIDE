use std::collections::HashMap;

use taide_lib::domain::system::service as legacy_service;
use taide_model::system::SystemUsageProcessKind;

#[test]
fn 시스템_사용량_정책은_독립_crate와_기존_경로에서_같다() {
    let _: Option<legacy_service::ProcessRecord> = None::<taide_system::service::ProcessRecord>;
    let extracted = taide_system::service::build_usage_processes;
    let legacy = legacy_service::build_usage_processes;

    let records = [taide_system::service::ProcessRecord {
        pid: 1,
        parent_pid: None,
        name: "taide".to_string(),
        cpu_usage: 50.0,
        memory: 100,
        has_previous_cpu_sample: true,
    }];
    let processes = legacy(&records, 1, "TAIDE", &HashMap::new(), 2);
    assert_eq!(processes, extracted(&records, 1, "TAIDE", &HashMap::new(), 2));
    assert_eq!(processes[0].kind, SystemUsageProcessKind::App);
    assert_eq!(processes[0].cpu_percent, Some(25.0));
}

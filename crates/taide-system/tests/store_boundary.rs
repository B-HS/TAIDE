use taide_system::store::SystemUsageStore;

#[test]
fn 복제한_저장소는_앱_cpu_샘플_상태를_공유한다() {
    let store = SystemUsageStore::new();
    let legacy_store = store.clone();

    assert!(store.collect_app_usage().expect("첫 샘플").cpu_percent.is_none());
    assert!(legacy_store.collect_app_usage().expect("후속 샘플").cpu_percent.is_some());
}

#[test]
fn 앱과_전체_프로세스는_서로_독립된_샘플_상태를_사용한다() {
    let store = SystemUsageStore::new();
    let legacy_store = store.clone();
    store.collect_app_usage().expect("앱 샘플");
    let current_pid = std::process::id();

    let first = legacy_store.refresh_process_records();
    let app_first = first
        .iter()
        .find(|record| record.pid == current_pid)
        .expect("첫 전체 프로세스 샘플");
    assert!(!app_first.has_previous_cpu_sample);

    let second = store.refresh_process_records();
    let app_second = second
        .iter()
        .find(|record| record.pid == current_pid)
        .expect("후속 전체 프로세스 샘플");
    assert!(app_second.has_previous_cpu_sample);
}

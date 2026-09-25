use std::sync::atomic::Ordering;

use taide_runtime::SearchStore;

#[test]
fn 검색_세션_레지스트리는_runtime과_기존_명령에서_같은_타입이다() {
    let store = SearchStore::new();
    let legacy: &taide_lib::domain::search::commands::SearchStore = &store;
    let first = legacy.begin("main", "panel");
    let second = store.begin("main", "panel");

    assert!(first.load(Ordering::SeqCst));
    assert!(!second.load(Ordering::SeqCst));
}

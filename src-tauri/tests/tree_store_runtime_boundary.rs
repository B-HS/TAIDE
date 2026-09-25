use taide_runtime::TreeStore;

#[test]
fn 트리_캐시는_runtime과_기존_명령에서_같은_타입이다() {
    let store = TreeStore::new();
    let legacy: &taide_lib::domain::tree::commands::TreeStore = &store;

    assert!(legacy.0.read().is_empty());
}

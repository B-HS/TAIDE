use taide_plugin::service::PluginStore;

#[test]
fn 플러그인_캐시_복제본은_같은_목록을_공유한다() {
    let store = PluginStore::new();
    let clone = store.clone();

    *store.0.write() = Some(Vec::new());

    assert!(clone.0.read().is_some());
}

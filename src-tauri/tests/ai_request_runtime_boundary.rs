use taide_runtime::AiRequestStore;

#[tokio::test]
async fn ai_요청_레지스트리는_runtime과_기존_명령에서_같은_타입이다() {
    let store = AiRequestStore::new();
    let legacy: &taide_lib::domain::ai::commands::AiRequestStore = &store;
    let (_token, receiver) = legacy.begin("main", "req-1").expect("first request");

    store.cancel("main", "req-1");

    assert!(receiver.await.is_ok());
}

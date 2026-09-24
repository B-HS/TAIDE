use std::sync::atomic::Ordering;

use taide_lsp::install::LspInstallStore;
use taide_model::lsp::LspServerId;

#[test]
fn lsp_설치_슬롯은_중복을_막고_취소와_drop_후_재진입을_허용한다() {
    let store = LspInstallStore::new();
    let server_id = LspServerId::from("test-server");
    let guard = store.begin(&server_id).expect("첫 설치는 슬롯을 얻는다");

    assert!(store.begin(&server_id).is_none());
    let cancellation_token = guard.cancellation_token();
    store.cancel(&server_id);
    assert!(cancellation_token.load(Ordering::SeqCst));

    drop(guard);
    assert!(store.begin(&server_id).is_some());
}

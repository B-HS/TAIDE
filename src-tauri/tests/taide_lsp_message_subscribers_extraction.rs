use std::sync::Arc;

use parking_lot::Mutex;
use taide_lsp::session::LspMessageSubscribers;

#[test]
fn lsp_구독자는_owner별_교체와_명시_제거를_유지한다() {
    let subscribers = LspMessageSubscribers::new();
    let original = Arc::new(Mutex::new(Vec::new()));
    let replacement = Arc::new(Mutex::new(Vec::new()));

    let original_messages = original.clone();
    subscribers.insert("window-a".to_string(), move |message| {
        original_messages.lock().push(message.to_string());
        true
    });
    subscribers.broadcast("first");

    let replacement_messages = replacement.clone();
    subscribers.insert("window-a".to_string(), move |message| {
        replacement_messages.lock().push(message.to_string());
        true
    });
    subscribers.broadcast("second");
    subscribers.remove("window-a");
    subscribers.broadcast("third");

    assert!(!subscribers.contains("window-a"));
    assert_eq!(*original.lock(), vec!["first"]);
    assert_eq!(*replacement.lock(), vec!["second"]);
}

#[test]
fn lsp_전송_실패는_해당_owner만_제거한다() {
    let subscribers = LspMessageSubscribers::new();
    let received = Arc::new(Mutex::new(Vec::new()));

    subscribers.insert("closed".to_string(), |_| false);
    let messages = received.clone();
    subscribers.insert("open".to_string(), move |message| {
        messages.lock().push(message.to_string());
        true
    });

    subscribers.broadcast("first");
    assert!(!subscribers.contains("closed"));
    assert!(subscribers.contains("open"));
    subscribers.broadcast("second");
    assert_eq!(*received.lock(), vec!["first", "second"]);
}

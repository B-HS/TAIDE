use taide_lib::domain::notification::types::{NotificationCategory, NotificationDelivery, NotificationSuppressionReason};
use taide_model::notification::{
    NotificationCategory as ModelNotificationCategory, NotificationDelivery as ModelNotificationDelivery,
    NotificationSuppressionReason as ModelNotificationSuppressionReason,
};

#[test]
fn 알림_카테고리와_억제_사유의_기존_wire와_공개_타입을_유지한다() {
    let category: ModelNotificationCategory = serde_json::from_str("\"agentAwaitingInput\"").expect("기존 카테고리");
    let facade: NotificationCategory = category;
    assert_eq!(facade, NotificationCategory::AgentAwaitingInput);
    assert_eq!(
        serde_json::to_value(NotificationCategory::LspInstall).expect("LSP 이름"),
        "lspInstall"
    );

    let reason: ModelNotificationSuppressionReason = serde_json::from_str("\"windowFocused\"").expect("기존 억제 사유");
    let facade: NotificationSuppressionReason = reason;
    assert_eq!(facade, NotificationSuppressionReason::WindowFocused);
    assert_eq!(serde_json::to_value(reason).expect("억제 사유 직렬화"), "windowFocused");
}

#[test]
fn 알림_전달_결과의_태그와_내용_wire를_유지한다() {
    let delivered: ModelNotificationDelivery = serde_json::from_value(serde_json::json!({
        "outcome": "delivered"
    }))
    .expect("기존 전달 결과");
    let facade: NotificationDelivery = delivered;
    assert_eq!(facade, NotificationDelivery::Delivered);

    let legacy = serde_json::json!({"outcome": "suppressed", "reason": "categoryDisabled"});
    let suppressed: ModelNotificationDelivery = serde_json::from_value(legacy.clone()).expect("기존 억제 결과");
    let facade: NotificationDelivery = suppressed;
    assert_eq!(
        facade,
        NotificationDelivery::Suppressed(NotificationSuppressionReason::CategoryDisabled)
    );
    assert_eq!(serde_json::to_value(facade).expect("억제 결과 직렬화"), legacy);
}

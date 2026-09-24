use taide_lib::domain::notification::service as legacy_service;
use taide_model::notification::{NotificationCategory, NotificationDelivery, NotificationSuppressionReason};
use taide_model::settings::Settings;

#[test]
fn 알림_정책은_독립_crate와_기존_경로에서_같은_판정을_쓴다() {
    let extracted: fn(&Settings, NotificationCategory, bool) -> NotificationDelivery = taide_notification::service::decide_delivery;
    let legacy: fn(&Settings, NotificationCategory, bool) -> NotificationDelivery = legacy_service::decide_delivery;
    assert!(std::ptr::fn_addr_eq(extracted, legacy));

    let settings = Settings {
        notifications_enabled: false,
        ..Settings::default()
    };
    assert_eq!(
        legacy(&settings, NotificationCategory::GitRemote, true),
        NotificationDelivery::Suppressed(NotificationSuppressionReason::NotificationsDisabled)
    );
}

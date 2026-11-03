use super::*;

#[test]
fn 알림_설정_열기는_macos에서만_원본_url을_열고_그밖은_지역화_오류로_거절한다() {
    let mut opened = Vec::new();
    open_notification_settings(true, |url| {
        opened.push(url.to_string());
        Ok(())
    })
    .unwrap();
    assert_eq!(
        opened,
        ["x-apple.systempreferences:com.apple.Notifications-Settings.extension"]
    );
    let refused = open_notification_settings(false, |_| panic!("other systems open nothing"));
    let Err(AppError::Localized(error)) = refused else {
        panic!("unsupported system must be refused: {refused:?}");
    };
    assert_eq!(error.kind, AppErrorKind::InvalidArgument);
    assert_eq!(error.key, "error.notification.settingsUnsupported");
    let failed = open_notification_settings(true, |_| Err(AppError::Internal("opener".into())));
    assert!(matches!(failed, Err(AppError::Internal(message)) if message == "opener"));
}

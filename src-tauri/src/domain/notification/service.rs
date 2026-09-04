use super::types::{NotificationCategory, NotificationDelivery, NotificationSuppressionReason};
use crate::domain::settings::types::Settings;

/// The whole OS-notification gate, as a pure function of the settings snapshot, the category, and
/// whether *any* TAIDE window currently has focus. Every caller path — the `notification_notify`
/// command, its tests, and the settings screen's test button — runs this one decision, so a
/// notification can never escape a switch by taking a different route.
///
/// Order matters and is checked left to right: the master switch, then the category switch, then
/// focus. It is the reporting order, not just an optimization — with three reasons possible at
/// once, the returned [`NotificationSuppressionReason`] should name the switch the user would have
/// to flip *first* to see this notification.
///
/// `any_window_focused` is passed in rather than read here because "focused" is an app-wide fact
/// only the Tauri `AppHandle` can answer (`webview_windows().values().any(is_focused)`); keeping it
/// a parameter is what makes every combination testable without a running app.
pub fn decide_delivery(settings: &Settings, category: NotificationCategory, any_window_focused: bool) -> NotificationDelivery {
    if !settings.notifications_enabled {
        return NotificationDelivery::Suppressed(NotificationSuppressionReason::NotificationsDisabled);
    }
    if !is_category_enabled(settings, category) {
        return NotificationDelivery::Suppressed(NotificationSuppressionReason::CategoryDisabled);
    }
    if settings.notifications_only_when_unfocused && any_window_focused {
        return NotificationDelivery::Suppressed(NotificationSuppressionReason::WindowFocused);
    }
    NotificationDelivery::Delivered
}

/// Maps a category to its own `Settings` switch. Exhaustive on purpose — a new
/// [`NotificationCategory`] variant fails to compile until it is given a switch, which is the
/// mechanism that keeps "every category can be turned off" true without a runtime check.
fn is_category_enabled(settings: &Settings, category: NotificationCategory) -> bool {
    match category {
        NotificationCategory::AgentCompleted => settings.notify_agent_completed,
        NotificationCategory::TaskCompleted => settings.notify_task_completed,
        NotificationCategory::GitRemote => settings.notify_git_remote,
        NotificationCategory::SearchReplace => settings.notify_search_replace,
        NotificationCategory::LspInstall => settings.notify_lsp_install,
        NotificationCategory::Error => settings.notify_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_CATEGORIES: &[NotificationCategory] = &[
        NotificationCategory::AgentCompleted,
        NotificationCategory::TaskCompleted,
        NotificationCategory::GitRemote,
        NotificationCategory::SearchReplace,
        NotificationCategory::LspInstall,
        NotificationCategory::Error,
    ];

    fn disable_category(settings: &mut Settings, category: NotificationCategory) {
        match category {
            NotificationCategory::AgentCompleted => settings.notify_agent_completed = false,
            NotificationCategory::TaskCompleted => settings.notify_task_completed = false,
            NotificationCategory::GitRemote => settings.notify_git_remote = false,
            NotificationCategory::SearchReplace => settings.notify_search_replace = false,
            NotificationCategory::LspInstall => settings.notify_lsp_install = false,
            NotificationCategory::Error => settings.notify_error = false,
        }
    }

    #[test]
    fn 기본_설정에서_비포커스면_모든_카테고리가_전달된다() {
        let settings = Settings::default();
        for category in ALL_CATEGORIES {
            assert_eq!(
                decide_delivery(&settings, *category, false),
                NotificationDelivery::Delivered,
                "{category:?} 는 비포커스 기본 설정에서 전달되어야 한다"
            );
        }
    }

    #[test]
    fn 마스터_스위치가_꺼져_있으면_모든_카테고리가_억제된다() {
        let settings = Settings {
            notifications_enabled: false,
            ..Settings::default()
        };
        for focused in [false, true] {
            for category in ALL_CATEGORIES {
                assert_eq!(
                    decide_delivery(&settings, *category, focused),
                    NotificationDelivery::Suppressed(NotificationSuppressionReason::NotificationsDisabled),
                    "{category:?}(focused={focused}) 는 마스터 스위치 off 로 억제되어야 한다"
                );
            }
        }
    }

    #[test]
    fn 카테고리_스위치를_끄면_그_카테고리만_억제된다() {
        for target in ALL_CATEGORIES {
            let mut settings = Settings::default();
            disable_category(&mut settings, *target);

            for category in ALL_CATEGORIES {
                let expected = if category == target {
                    NotificationDelivery::Suppressed(NotificationSuppressionReason::CategoryDisabled)
                } else {
                    NotificationDelivery::Delivered
                };
                assert_eq!(
                    decide_delivery(&settings, *category, false),
                    expected,
                    "{target:?} 만 끈 상태에서 {category:?} 의 판정이 다르다"
                );
            }
        }
    }

    #[test]
    fn 비포커스_전용이_켜져_있고_창이_포커스면_억제된다() {
        let settings = Settings::default();
        for category in ALL_CATEGORIES {
            assert_eq!(
                decide_delivery(&settings, *category, true),
                NotificationDelivery::Suppressed(NotificationSuppressionReason::WindowFocused),
                "{category:?} 는 포커스 상태에서 억제되어야 한다"
            );
        }
    }

    #[test]
    fn 비포커스_전용을_끄면_포커스_중에도_전달된다() {
        let settings = Settings {
            notifications_only_when_unfocused: false,
            ..Settings::default()
        };
        for category in ALL_CATEGORIES {
            assert_eq!(
                decide_delivery(&settings, *category, true),
                NotificationDelivery::Delivered,
                "{category:?} 는 비포커스 전용 off 에서 포커스 중에도 전달되어야 한다"
            );
        }
    }

    #[test]
    fn 억제_사유는_마스터_카테고리_포커스_순으로_보고된다() {
        let mut settings = Settings {
            notifications_enabled: false,
            ..Settings::default()
        };
        disable_category(&mut settings, NotificationCategory::GitRemote);

        assert_eq!(
            decide_delivery(&settings, NotificationCategory::GitRemote, true),
            NotificationDelivery::Suppressed(NotificationSuppressionReason::NotificationsDisabled),
            "세 조건이 동시에 성립하면 마스터 스위치가 먼저 보고되어야 한다"
        );

        settings.notifications_enabled = true;
        assert_eq!(
            decide_delivery(&settings, NotificationCategory::GitRemote, true),
            NotificationDelivery::Suppressed(NotificationSuppressionReason::CategoryDisabled),
            "카테고리와 포커스가 동시에 성립하면 카테고리가 먼저 보고되어야 한다"
        );
    }
}

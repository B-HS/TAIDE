use taide_infra::redact::mask_known_secrets;
use taide_model::error::AppResult;
use taide_model::notification::{NotificationCategory, NotificationDelivery};
use taide_notification::service;

use crate::{AppState, PlatformServices};

fn masked_notification_text(title: &str, body: &str) -> (String, String) {
    (mask_known_secrets(title), mask_known_secrets(body))
}

pub fn notification_notify<F>(
    state: &AppState,
    platform: &dyn PlatformServices,
    category: NotificationCategory,
    title: &str,
    body: &str,
    has_focused_window: F,
) -> AppResult<NotificationDelivery>
where
    F: FnOnce() -> bool,
{
    let (title, body) = masked_notification_text(title, body);
    let settings = state.settings.read().clone();
    let any_window_focused = has_focused_window();

    let decision = service::decide_delivery(&settings, category, any_window_focused);
    if decision == NotificationDelivery::Delivered {
        platform.send_notification(&title, &body)?;
    }
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 알림_제목과_본문의_자격증명은_마스킹된다() {
        let token = format!("ghp_{}", "abcdefghijklmnopqrstuvwxyz012345");
        let (title, body) = masked_notification_text(
            &format!("git push 실패 ({token})"),
            &format!("remote: https://taide:{token}@github.com/org/repo.git 인증 거부"),
        );

        assert!(!title.contains(&token), "알림 제목에 토큰이 남아 있습니다: {title}");
        assert!(!body.contains(&token), "알림 본문에 토큰이 남아 있습니다: {body}");
        assert!(title.contains("[redacted:github]"));
        assert!(body.contains("https://taide:[redacted:url_password]@github.com/org/repo.git"));
    }

    #[test]
    fn 시크릿이_없는_알림_문자열은_그대로_전달된다() {
        let (title, body) = masked_notification_text("빌드 완료", "12개 파일이 3.4초에 컴파일되었습니다");

        assert_eq!(title, "빌드 완료");
        assert_eq!(body, "12개 파일이 3.4초에 컴파일되었습니다");
    }
}

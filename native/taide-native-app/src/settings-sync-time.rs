use std::sync::OnceLock;

use chrono::{DateTime, Local, NaiveDateTime, Timelike};

const FALLBACK_LOCALE: &str = "en-US";

static SYSTEM_LOCALE: OnceLock<String> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pattern {
    Korean,
    Japanese,
    English,
}

impl Pattern {
    fn of(locale: &str) -> Self {
        match locale
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "ko" => Self::Korean,
            "ja" => Self::Japanese,
            _ => Self::English,
        }
    }
}

fn render(time: &NaiveDateTime, locale: &str) -> String {
    let date = time.date();
    match Pattern::of(locale) {
        Pattern::Korean => {
            let (is_pm, hour) = time.hour12();
            format!(
                "{}. {}. {}. {} {}:{}",
                date.format("%Y"),
                date.format("%-m"),
                date.format("%-d"),
                if is_pm { "오후" } else { "오전" },
                hour,
                time.format("%M:%S"),
            )
        }
        Pattern::Japanese => time.format("%Y/%-m/%-d %-H:%M:%S").to_string(),
        Pattern::English => {
            let (is_pm, hour) = time.hour12();
            format!(
                "{}, {}:{} {}",
                date.format("%-m/%-d/%Y"),
                hour,
                time.format("%M:%S"),
                if is_pm { "PM" } else { "AM" },
            )
        }
    }
}

/// Formats an RFC 3339 sync time in the local time zone with the browser default pattern of the
/// system locale; `None` when the text is not RFC 3339.
pub(crate) fn format_last_synced(iso: &str, system_locale: &str) -> Option<String> {
    let local = DateTime::parse_from_rfc3339(iso)
        .ok()?
        .with_timezone(&Local)
        .naive_local();
    Some(render(&local, system_locale))
}

/// [`format_last_synced`] with the system locale read once per process.
pub(crate) fn format_system_time(iso: &str) -> Option<String> {
    let locale = SYSTEM_LOCALE
        .get_or_init(|| sys_locale::get_locale().unwrap_or_else(|| FALLBACK_LOCALE.into()));
    format_last_synced(iso, locale)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn at(hour: u32, minute: u32, second: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 28)
            .unwrap()
            .and_hms_opt(hour, minute, second)
            .unwrap()
    }

    #[test]
    fn 마지막_동기화_시각은_시스템_locale의_브라우저_기본_형식을_따른다() {
        let afternoon = at(14, 5, 9);
        assert_eq!(render(&afternoon, "ko-KR"), "2026. 10. 28. 오후 2:05:09");
        assert_eq!(render(&afternoon, "ja-JP"), "2026/10/28 14:05:09");
        assert_eq!(render(&afternoon, "en-US"), "10/28/2026, 2:05:09 PM");
        assert_eq!(render(&afternoon, "fr-FR"), "10/28/2026, 2:05:09 PM");
        assert_eq!(render(&afternoon, "ko"), "2026. 10. 28. 오후 2:05:09");
        assert_eq!(render(&afternoon, "ja_JP"), "2026/10/28 14:05:09");
        let early = NaiveDate::from_ymd_opt(2026, 1, 5)
            .unwrap()
            .and_hms_opt(0, 7, 3)
            .unwrap();
        assert_eq!(render(&early, "ko-KR"), "2026. 1. 5. 오전 12:07:03");
        assert_eq!(render(&early, "ja-JP"), "2026/1/5 0:07:03");
        assert_eq!(render(&early, "en-US"), "1/5/2026, 12:07:03 AM");
        assert_eq!(render(&at(12, 0, 0), "en-US"), "10/28/2026, 12:00:00 PM");
    }

    #[test]
    fn 마지막_동기화_시각은_rfc_3339가_아니면_원문으로_남긴다() {
        assert_eq!(format_last_synced("not a time", "ko-KR"), None);
        assert_eq!(format_last_synced("", "en-US"), None);
        let expected = render(
            &DateTime::parse_from_rfc3339("2026-10-28T05:05:09Z")
                .unwrap()
                .with_timezone(&Local)
                .naive_local(),
            "en-US",
        );
        assert_eq!(
            format_last_synced("2026-10-28T05:05:09Z", "en-US"),
            Some(expected)
        );
    }
}

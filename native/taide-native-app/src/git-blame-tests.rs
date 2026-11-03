use std::time::{Duration, Instant};

use taide_model::error::AppError;
use taide_model::git::BlameLine;

use super::{Blame, DEBOUNCE, format_line, relative_time, split_footer, truncate_message};

const NOW: f64 = 1_786_060_800.0;
const MINUTE: f64 = 60.0;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;
const LONG_SUMMARY_CHARS: usize = 60;
const KEPT_SUMMARY_CHARS: usize = 50;
const WIDE_SUMMARY_CHARS: usize = 30;
const KEPT_WIDE_CHARS: usize = 25;

fn line(author: &str, time_unix: f64, summary: &str) -> BlameLine {
    BlameLine {
        line: 1,
        commit_id: "abc".into(),
        author: author.into(),
        time_unix,
        summary: summary.into(),
        is_uncommitted: false,
    }
}

#[test]
fn 상대_시간은_원본_blame_format_사례와_같다() {
    assert_eq!(relative_time(None, NOW), "unknown time");
    assert_eq!(relative_time(Some(NOW - 30.0), NOW), "just now");
    assert_eq!(
        relative_time(Some(NOW - 5.0 * MINUTE), NOW),
        "5 minutes ago"
    );
    assert_eq!(relative_time(Some(NOW - MINUTE), NOW), "1 minute ago");
    assert_eq!(relative_time(Some(NOW - 3.0 * HOUR), NOW), "3 hours ago");
    assert_eq!(relative_time(Some(NOW - 4.0 * DAY), NOW), "4 days ago");
    assert_eq!(relative_time(Some(NOW - 60.0 * DAY), NOW), "2 months ago");
    assert_eq!(relative_time(Some(NOW - 400.0 * DAY), NOW), "1 year ago");
    assert_eq!(relative_time(Some(NOW + MINUTE), NOW), "just now");
}

#[test]
fn 요약은_공백을_접고_50자를_넘으면_잘라_점_세개를_붙인다() {
    assert_eq!(truncate_message("fix: short message"), "fix: short message");
    assert_eq!(
        truncate_message(&"a".repeat(LONG_SUMMARY_CHARS)),
        format!("{}...", "a".repeat(KEPT_SUMMARY_CHARS))
    );
    assert_eq!(
        truncate_message("fix:\nmultiline   message"),
        "fix: multiline message"
    );
}

#[test]
fn 요약_길이는_원본_string_length처럼_utf16_단위로_센다() {
    let wide = "\u{1D400}";
    assert_eq!(
        truncate_message(&wide.repeat(WIDE_SUMMARY_CHARS)),
        format!("{}...", wide.repeat(KEPT_WIDE_CHARS))
    );
    assert_eq!(
        truncate_message(&format!("a{}", wide.repeat(KEPT_WIDE_CHARS))),
        format!("a{}...", wide.repeat(KEPT_WIDE_CHARS - 1)),
        "50번째 단위에 걸친 글자는 버립니다"
    );
}

#[test]
fn blame_문구는_미커밋_현재_사용자_작성자와_긴_요약을_원본대로_만든다() {
    let uncommitted = BlameLine {
        is_uncommitted: true,
        ..line("", 0.0, "")
    };
    assert_eq!(
        format_line(&uncommitted, NOW, None),
        "You, now - Uncommitted changes"
    );
    assert_eq!(
        format_line(
            &line("HS", NOW - 4.0 * DAY, "fix: something broken"),
            NOW,
            None
        ),
        "HS, 4 days ago \u{2022} fix: something broken"
    );
    assert_eq!(
        format_line(&line("HS", NOW, &"a".repeat(LONG_SUMMARY_CHARS)), NOW, None),
        format!(
            "HS, just now \u{2022} {}...",
            "a".repeat(KEPT_SUMMARY_CHARS)
        )
    );
    assert_eq!(
        format_line(&line("HS", NOW, "fix: something"), NOW, Some("HS")),
        "You, just now \u{2022} fix: something"
    );
    assert_eq!(
        format_line(&line("Other", NOW, "fix: something"), NOW, Some("HS")),
        "Other, just now \u{2022} fix: something"
    );
}

#[test]
fn 커서_줄은_300ms_머문_뒤에야_정해지고_움직이면_다시_기다린다() {
    let start = Instant::now();
    let mut blame = Blame::default();
    assert_eq!(blame.cursor(3, start), Some(DEBOUNCE));
    assert_eq!(blame.settled(), None);
    let half = DEBOUNCE / 2;
    assert_eq!(blame.cursor(3, start + half), Some(DEBOUNCE - half));
    assert_eq!(blame.cursor(4, start + half), Some(DEBOUNCE));
    assert_eq!(blame.cursor(4, start + DEBOUNCE), Some(half));
    assert_eq!(blame.cursor(4, start + half + DEBOUNCE), None);
    assert_eq!(blame.settled(), Some(4));
    assert_eq!(
        blame.cursor(5, start + DEBOUNCE * 2),
        Some(DEBOUNCE),
        "새 줄을 기다리는 동안 이전 줄을 유지합니다"
    );
    assert_eq!(blame.settled(), Some(4));
}

#[test]
fn footer는_새_응답까지_이전_값을_유지하고_오류는_비우며_문서_밖_줄은_비운다() {
    let first = Instant::now();
    let second = first + Duration::from_secs(1);
    let mut blame = Blame::default();
    assert_eq!(blame.footer_text(10, None), "");
    let answered = [BlameLine {
        line: 2,
        ..line("HS", 0.0, "fix: two")
    }];
    blame.adopt(Some(Ok(&answered)), Some(first));
    assert!(blame.footer_text(10, None).starts_with("HS, "));
    assert!(blame.footer_text(10, Some("HS")).starts_with("You, "));
    assert_eq!(blame.footer_text(1, None), "", "문서 줄 수를 넘는 줄");
    blame.adopt(None, None);
    assert!(
        blame.footer_text(10, None).starts_with("HS, "),
        "응답 전에는 이전 값"
    );
    let error = AppError::Internal("blame".into());
    blame.adopt(Some(Err(&error)), Some(second));
    assert_eq!(blame.footer_text(10, None), "");
    blame.adopt(Some(Ok(&answered)), Some(second));
    assert_eq!(
        blame.footer_text(10, None),
        "",
        "같은 응답은 다시 받지 않습니다"
    );
    blame.adopt(Some(Ok(&[])), Some(first));
    assert_eq!(blame.footer_text(10, None), "");
}

#[test]
fn overlay는_켤_때의_줄_수를_기억하고_다시_누르면_꺼진다() {
    let mut blame = Blame::default();
    blame.toggle_overlay(7);
    assert_eq!(blame.overlay(), Some(7));
    blame.toggle_overlay(9);
    assert_eq!(blame.overlay(), None);
    blame.toggle_overlay(9);
    assert_eq!(blame.overlay(), Some(9));
}

#[test]
fn footer_칸은_파일_편집기_아래_24px을_떼어낸다() {
    let rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(0.0, 10.0),
        eframe::egui::vec2(200.0, 100.0),
    );
    assert_eq!(split_footer(rect, false), (rect, None));
    let (editor, footer) = split_footer(rect, true);
    assert_eq!(editor.bottom(), 86.0);
    assert_eq!(footer.unwrap().height(), super::FOOTER_HEIGHT);
}

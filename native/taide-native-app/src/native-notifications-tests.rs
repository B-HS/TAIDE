use super::*;
use taide_model::ids::{PaneId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, Tab};
use taide_model::project::ProjectDisplay;

const NINE_SECONDS_999: Duration = Duration::from_millis(9_999);
const ELEVEN_SECONDS: Duration = Duration::from_secs(11);

fn locale() -> ResolvedLocale {
    let state = taide_runtime::AppState::new(taide_model::paths::AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-notifications-{}", ProjectId::new())),
    ));
    taide_runtime::locale_actions::locale_get_current(&state, "en").unwrap()
}

fn agent(session: &str, activity: AgentActivity, reason: Option<BlockedReason>) -> DetectedAgent {
    DetectedAgent {
        session_id: session.into(),
        name: "claude".into(),
        pid: 1,
        activity,
        blocked_reason: reason,
    }
}

fn terminal_tab(session: &str, title: &str) -> Tab {
    Tab {
        id: TabId::new(),
        kind: TabKind::Terminal {
            session_id: session.into(),
            cwd: None,
        },
        title: title.into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn project_ref(id: &ProjectId, name: &str, label: Option<&str>) -> ProjectRef {
    ProjectRef {
        id: id.clone(),
        root: format!("/synthetic/{name}"),
        name: name.into(),
        display: ProjectDisplay {
            label: label.map(str::to_owned),
            ..Default::default()
        },
        root_missing: false,
    }
}

fn completion(awaiting_input: bool, reason: Option<BlockedReason>) -> Completion {
    Completion {
        session: "s".into(),
        name: "claude".into(),
        worked_for: Duration::from_secs(75),
        awaiting_input,
        blocked_reason: reason,
    }
}

#[test]
fn 알림_평가는_10초_경계와_working_시작시각을_원본처럼_다룬다() {
    let start = Instant::now();
    let (working, completed) = evaluate(
        &HashMap::new(),
        &[agent("a", AgentActivity::Working, None)],
        start,
    );
    assert!(completed.is_empty());
    assert_eq!(working["a"], start);
    let (kept, _) = evaluate(
        &working,
        &[agent("a", AgentActivity::Working, None)],
        start + ELEVEN_SECONDS,
    );
    assert_eq!(kept["a"], start);
    let (_, early) = evaluate(
        &kept,
        &[agent("a", AgentActivity::Idle, None)],
        start + NINE_SECONDS_999,
    );
    assert!(early.is_empty());
    let (after, done) = evaluate(
        &kept,
        &[agent("a", AgentActivity::Idle, None)],
        start + AGENT_COMPLETION_NOTIFY_MIN_WORKING,
    );
    assert!(after.is_empty());
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].worked_for, AGENT_COMPLETION_NOTIFY_MIN_WORKING);
    assert_eq!(
        agent_category(&done[0]),
        NotificationCategory::AgentCompleted
    );
    let (_, unknown) = evaluate(
        &kept,
        &[agent("a", AgentActivity::Unknown, None)],
        start + ELEVEN_SECONDS,
    );
    assert!(unknown.is_empty());
    let (vanished, gone) = evaluate(&kept, &[], start + ELEVEN_SECONDS);
    assert!(vanished.is_empty() && gone.is_empty());
    let (_, waiting) = evaluate(
        &kept,
        &[agent(
            "a",
            AgentActivity::AwaitingInput,
            Some(BlockedReason::Question),
        )],
        start + ELEVEN_SECONDS,
    );
    assert_eq!(
        agent_category(&waiting[0]),
        NotificationCategory::AgentAwaitingInput
    );
    assert_eq!(waiting[0].blocked_reason, Some(BlockedReason::Question));
}

#[test]
fn 알림_tracker는_프로젝트마다_지도를_나누고_닫힘에서_지운다() {
    let start = Instant::now();
    let (first, second) = (ProjectId::new(), ProjectId::new());
    let mut tracker = Tracker::default();
    tracker.observe(&first, &[agent("a", AgentActivity::Working, None)], start);
    tracker.observe(&second, &[agent("b", AgentActivity::Working, None)], start);
    let finished = tracker.observe(
        &first,
        &[agent("a", AgentActivity::Idle, None)],
        start + ELEVEN_SECONDS,
    );
    assert_eq!(finished.len(), 1);
    tracker.forget(&second);
    assert!(
        tracker
            .observe(
                &second,
                &[agent("b", AgentActivity::Idle, None)],
                start + ELEVEN_SECONDS,
            )
            .is_empty()
    );
}

#[test]
fn 알림_본문은_대기이유_탭꼬리_경과를_원본_문구로_만든다() {
    let locale = locale();
    assert_eq!(
        agent_body(
            &locale,
            &completion(true, Some(BlockedReason::Permission)),
            Some("Build")
        ),
        "claude · Waiting for permission · Build"
    );
    assert_eq!(
        agent_body(
            &locale,
            &completion(true, Some(BlockedReason::Question)),
            None
        ),
        "claude · Waiting for an answer"
    );
    let fallback = "claude · Waiting for input (permission request or question)";
    assert_eq!(
        agent_body(
            &locale,
            &completion(true, Some(BlockedReason::Dialog)),
            None
        ),
        fallback
    );
    assert_eq!(
        agent_body(&locale, &completion(true, None), Some("   ")),
        fallback
    );
    assert_eq!(
        agent_body(&locale, &completion(false, None), Some("Agent tab")),
        "claude · Finished · 1m 15s · Agent tab"
    );
}

#[test]
fn 알림_경과는_분과_시간_경계에서_두_단위만_쓴다() {
    let locale = locale();
    let cases = [
        (59_999, "59s"),
        (60_000, "1m 0s"),
        (3_599_000, "59m 59s"),
        (3_600_000, "1h 0m"),
    ];
    for (duration_ms, expected) in cases {
        assert_eq!(format_duration(&locale, duration_ms), expected);
    }
}

#[test]
fn 알림_자르기는_코드포인트_기준이다() {
    let exact = "가".repeat(NOTIFICATION_TEXT_MAX_CODE_POINTS);
    assert_eq!(truncate(&exact, NOTIFICATION_TEXT_MAX_CODE_POINTS), exact);
    let over = "\u{1f600}".repeat(NOTIFICATION_TEXT_MAX_CODE_POINTS + 1);
    let cut = truncate(&over, NOTIFICATION_TEXT_MAX_CODE_POINTS);
    assert_eq!(cut.chars().count(), NOTIFICATION_TEXT_MAX_CODE_POINTS);
    assert!(cut.ends_with('…'));
    assert_eq!(
        cut.chars()
            .filter(|character| *character == '\u{1f600}')
            .count(),
        NOTIFICATION_TEXT_MAX_CODE_POINTS - 1
    );
}

#[test]
fn 명령_완료_본문은_대상_종료코드_경과를_잇고_빈_조각을_뺀다() {
    let locale = locale();
    assert_eq!(
        task_body(&locale, "cargo build", Some(1), 12_000),
        "cargo build · Exit code 1 · 12s"
    );
    assert_eq!(
        task_body(&locale, "cargo build", Some(0), 12_000),
        "cargo build · Exit code 0 · 12s"
    );
    assert_eq!(task_body(&locale, "/repo", None, 12_000), "/repo · 12s");
    assert_eq!(task_body(&locale, "", None, 12_000), "12s");
}

#[test]
fn 알림_제목과_소유는_표시라벨과_보조창_터미널탭을_찾는다() {
    let (labelled, plain) = (ProjectId::new(), ProjectId::new());
    let projects = vec![
        project_ref(&labelled, "alpha", Some("AL")),
        project_ref(&plain, "beta", None),
    ];
    assert_eq!(
        project_title(&projects, Some(&labelled), "fallback".into()),
        "AL"
    );
    assert_eq!(
        project_title(&projects, Some(&plain), "fallback".into()),
        "beta"
    );
    assert_eq!(
        project_title(&projects, Some(&ProjectId::new()), "fallback".into()),
        "fallback"
    );
    assert_eq!(
        project_title(&projects, None, "fallback".into()),
        "fallback"
    );
    let mut layout = taide_layout::service::default_layout();
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: 1,
        root: PaneNode::Leaf {
            id: PaneId::new(),
            tabs: vec![terminal_tab("aux-session", "Background agent")],
            active: None,
        },
        focused_pane: PaneId::new(),
    });
    assert_eq!(
        terminal_tab_title(&layout, "aux-session"),
        Some("Background agent")
    );
    assert_eq!(terminal_tab_title(&layout, ""), None);
    let layouts = HashMap::from([(plain.clone(), layout)]);
    assert_eq!(
        session_owner(&projects, &layouts, "aux-session"),
        Some((plain, "Background agent"))
    );
    assert_eq!(session_owner(&projects, &layouts, "missing"), None);
}

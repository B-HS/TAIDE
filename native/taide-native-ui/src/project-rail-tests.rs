use super::*;

fn display(icon: Option<&str>, label: Option<&str>, color: Option<&str>) -> ProjectDisplay {
    ProjectDisplay {
        icon: icon.map(str::to_owned),
        label: label.map(str::to_owned),
        color: color.map(str::to_owned),
    }
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

#[test]
fn 표시_해석은_라벨_아이콘_기본_순위와_색_토큰_검증을_원본처럼_따른다() {
    let resolved = resolve_project_display(&display(
        Some("rocket"),
        Some("  가나다라마 "),
        Some(" lane3 "),
    ));
    assert_eq!(resolved.glyph, ProjectGlyph::Label("가나다라".into()));
    assert_eq!(resolved.lane, Some(2));
    assert_eq!(resolved.icon(), None);
    let resolved = resolve_project_display(&display(Some(" rocket "), Some("   "), Some("lane13")));
    assert_eq!(resolved.glyph, ProjectGlyph::Icon("rocket".into()));
    assert_eq!(resolved.icon(), Some("rocket"));
    assert_eq!(resolved.lane, None);
    let resolved = resolve_project_display(&display(None, None, Some("lane12")));
    assert_eq!(resolved.glyph, ProjectGlyph::Default);
    assert_eq!(resolved.lane, Some(PROJECT_COLOR_TOKENS.len() - 1));
    assert_eq!(
        resolve_project_display(&ProjectDisplay::default()),
        ProjectDisplayResolution {
            glyph: ProjectGlyph::Default,
            lane: None
        }
    );
    assert_eq!(
        resolve_project_display(&display(Some("unknown-icon"), None, Some("Lane1"))),
        ProjectDisplayResolution {
            glyph: ProjectGlyph::Icon("unknown-icon".into()),
            lane: None
        }
    );
}

#[test]
fn 라벨_글자수별_크기와_네글자_자간은_원본_사다리를_따른다() {
    assert_eq!(label_font_size("A"), 16.0);
    assert_eq!(label_font_size("AB"), 14.0);
    assert_eq!(label_font_size("ABC"), 11.0);
    assert_eq!(label_font_size("가나다라"), 9.0);
    assert_eq!(label_font_size("ABCDEF"), 9.0);
    assert_eq!(label_letter_spacing("ABC"), 0.0);
    assert!((label_letter_spacing("ABCD") + 0.225).abs() < 1e-6);
}

#[test]
fn 상태_문구_키는_차단_이유를_입력대기에만_붙인다() {
    let cases = [
        (
            AgentActivity::AwaitingInput,
            Some(BlockedReason::Permission),
            "agent.blocked.permission",
        ),
        (
            AgentActivity::AwaitingInput,
            Some(BlockedReason::Question),
            "agent.blocked.question",
        ),
        (
            AgentActivity::AwaitingInput,
            Some(BlockedReason::Dialog),
            "agent.blocked.dialog",
        ),
        (
            AgentActivity::AwaitingInput,
            None,
            "agent.status.awaitingInput",
        ),
        (
            AgentActivity::Working,
            Some(BlockedReason::Dialog),
            "agent.status.working",
        ),
        (AgentActivity::Idle, None, "agent.status.idle"),
        (AgentActivity::Unknown, None, "agent.status.unknown"),
    ];
    for (activity, reason, key) in cases {
        assert_eq!(agent_status_label_key(activity, reason), key);
    }
}

#[test]
fn 대표_에이전트는_입력대기_작업_대기_알수없음_순위의_첫_최댓값이다() {
    assert!(top_priority_agent(&[]).is_none());
    let agents = [
        agent("a", AgentActivity::Idle, None),
        agent("b", AgentActivity::Working, None),
        agent("c", AgentActivity::Working, None),
        agent("d", AgentActivity::Unknown, None),
    ];
    assert_eq!(top_priority_agent(&agents).unwrap().session_id, "b");
    let agents = [
        agent("a", AgentActivity::Working, None),
        agent(
            "b",
            AgentActivity::AwaitingInput,
            Some(BlockedReason::Question),
        ),
    ];
    assert_eq!(top_priority_agent(&agents).unwrap().session_id, "b");
    let agents = [agent("a", AgentActivity::Unknown, None)];
    assert_eq!(top_priority_agent(&agents).unwrap().session_id, "a");
}

#[test]
fn pulse_불투명도는_2초_주기의_50퍼센트에서_절반이다() {
    assert_eq!(pulse_opacity(0.0), 1.0);
    assert!((pulse_opacity(1.0) - PULSE_MIN_OPACITY).abs() < 1e-4);
    assert!((pulse_opacity(2.0) - 1.0).abs() < 1e-4);
    assert!(pulse_opacity(0.5) < 1.0 && pulse_opacity(0.5) > PULSE_MIN_OPACITY);
    assert!((pulse_opacity(0.5) - pulse_opacity(1.5)).abs() < 1e-3);
}

#[test]
fn 배지_상자는_버튼_오른쪽_아래에_붙고_상태별_크기를_가진다() {
    let button = Rect::from_min_size(egui::pos2(8.0, 8.0), Vec2::splat(40.0));
    for (activity, size) in [
        (AgentActivity::Working, WORKING_BADGE_SIZE),
        (AgentActivity::AwaitingInput, AWAITING_BADGE_SIZE),
        (AgentActivity::Idle, IDLE_BADGE_SIZE),
        (AgentActivity::Unknown, IDLE_BADGE_SIZE),
    ] {
        let badge = agent_badge_rect(button, activity);
        assert_eq!(badge.right_bottom(), button.right_bottom());
        assert_eq!(badge.size(), Vec2::splat(size));
    }
}

#[test]
fn 표시_편집_보조함수는_라벨_정리_아이콘_색_해석과_초기화_조건을_원본처럼_따른다() {
    assert_eq!(clamp_project_label(" \u{7}가\n나다라마"), " 가나다");
    assert_eq!(normalize_project_label(" \u{7} A B \t"), "A B");
    assert_eq!(normalize_project_label("\u{1}  가나다라마"), "가나다라");
    assert_eq!(resolve_project_icon_name(Some("rocket")), "rocket");
    for unknown in [None, Some(" rocket"), Some("Rocket"), Some("constructor")] {
        assert_eq!(
            resolve_project_icon_name(unknown),
            DEFAULT_PROJECT_ICON_NAME
        );
    }
    assert_eq!(resolve_project_color_lane(Some("lane1")), Some(0));
    assert_eq!(resolve_project_color_lane(Some("lane0")), None);
    assert_eq!(resolve_project_color_lane(None), None);
    assert!(!is_project_display_customized(&resolve_project_display(
        &display(None, Some("  "), Some("lane99"))
    )));
    for customized in [
        display(Some("x"), None, None),
        display(None, Some("A"), None),
        display(None, None, Some("lane5")),
    ] {
        assert!(is_project_display_customized(&resolve_project_display(
            &customized
        )));
    }
    let cleared = cleared_project_display_patch();
    assert_eq!(
        [cleared.icon, cleared.label, cleared.color],
        [
            Some(String::new()),
            Some(String::new()),
            Some(String::new())
        ]
    );
}

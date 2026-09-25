use taide_lsp::session::LspSessionLifecycle;
use taide_model::lsp::LspSessionStatus;

#[test]
fn 자동_재시작은_세대와_상태를_함께_바꾸고_오래된_확인을_거부한다() {
    let lifecycle = LspSessionLifecycle::new();
    let first = lifecycle.auto_respawned("첫 번째 재시작".to_string());
    let second = lifecycle.auto_respawned("두 번째 재시작".to_string());

    assert_eq!(first.generation, 1);
    assert_eq!(second.generation, 2);
    assert_eq!(second.status, LspSessionStatus::Crashed);
    assert_eq!(second.last_error.as_deref(), Some("두 번째 재시작"));
    assert!(lifecycle.confirm_reinitialized(first.generation).is_none());
    assert!(lifecycle
        .report_reinitialize_failure(first.generation, "오래된 실패".to_string())
        .is_none());
    assert_eq!(lifecycle.snapshot(), second);

    let failed = lifecycle
        .report_reinitialize_failure(second.generation, "현재 실패".to_string())
        .expect("현재 세대의 실패는 반영되어야 한다");
    assert_eq!(failed.status, LspSessionStatus::Crashed);
    assert_eq!(failed.last_error.as_deref(), Some("현재 실패"));

    let confirmed = lifecycle
        .confirm_reinitialized(second.generation)
        .expect("현재 세대는 확인되어야 한다");
    assert_eq!(confirmed.status, LspSessionStatus::Running);
    assert!(confirmed.last_error.is_none());
}

#[test]
fn 수동_재시작은_늦은_재초기화_결과를_무시하고_종료_중에는_재시도를_막는다() {
    let lifecycle = LspSessionLifecycle::new();
    let crashed = lifecycle.auto_respawned("재초기화 대기".to_string());
    lifecycle.begin_manual_restart();

    assert!(lifecycle
        .report_reinitialize_failure(crashed.generation, "실패".to_string())
        .is_none());
    assert!(lifecycle.confirm_reinitialized(crashed.generation).is_none());
    assert_eq!(lifecycle.snapshot().status, LspSessionStatus::Starting);

    lifecycle.mark_stopping();
    assert!(lifecycle.begin_exit_recovery().is_none());
    assert!(lifecycle.is_stopping());
}

#[test]
fn 비정상_종료_횟수는_건강한_복구_후_초기화된다() {
    let lifecycle = LspSessionLifecycle::new();
    assert_eq!(lifecycle.begin_exit_recovery(), Some(1));
    assert_eq!(lifecycle.begin_exit_recovery(), Some(2));
    lifecycle.reset_restart_count();
    assert_eq!(lifecycle.begin_exit_recovery(), Some(1));
}

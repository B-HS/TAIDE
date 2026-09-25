use std::future::pending;
use std::time::Duration;

use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

#[tokio::test]
async fn 장기_작업은_이름별로_한_번만_등록되고_종료시_취소된다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (sender, receiver) = oneshot::channel::<()>();

    assert!(supervisor.spawn("agent-poll", async move {
        let _sender = sender;
        pending::<()>().await;
    }));
    assert!(!supervisor.spawn("agent-poll", async {}));
    assert_eq!(supervisor.tracked_count(), 1);

    supervisor.stop_all();

    assert_eq!(supervisor.tracked_count(), 0);
    assert!(tokio::time::timeout(Duration::from_secs(1), receiver).await.unwrap().is_err());
    assert!(!supervisor.spawn("agent-poll", async {}));
    supervisor.stop_all();
}

#[test]
fn 앱_조립은_세_장기_작업을_등록하고_종료시_취소한다() {
    let app = include_str!("../src/lib.rs");
    let setup = app.split_once("app.manage(WindowRegistry::default());").unwrap().1;
    let exit = app.split_once(".run(|app_handle, event| {").unwrap().1;

    assert!(setup.contains("app.manage(TaskSupervisor::new("));
    assert!(setup.contains(".spawn(\"ide-reconcile\""));
    assert!(setup.contains(".spawn(\"agent-poll\""));
    assert!(setup.contains(".spawn(\"layout-flush\""));
    assert!(exit.contains("app_handle.state::<TaskSupervisor>().stop_all()"));
}

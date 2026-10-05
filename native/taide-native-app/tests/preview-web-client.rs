use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use taide_native_app::{
    preview_web_client::{Request, prepare},
    preview_web_document::source_url,
};
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

const EXECUTABLE: &str = env!("CARGO_BIN_EXE_taide-native-app");
const TIMEOUT: Duration = Duration::from_secs(20);
const SOURCE: &str = "/synthetic project/pages/index.html";
const PENDING_BYTES: usize = 1024 * 1024;

fn request(timeout: Duration, bytes: Vec<u8>) -> Request {
    Request {
        source: source_url(Path::new(SOURCE)).unwrap(),
        bytes,
        timeout,
    }
}

fn assert_reaped(pid: u32) {
    #[cfg(unix)]
    {
        let result = std::process::Command::new("/bin/kill")
            .env("LC_ALL", "C")
            .args(["-0", &pid.to_string()])
            .output()
            .unwrap();
        assert!(
            !result.status.success(),
            "owned helper must have exited and been reaped"
        );
        assert!(
            String::from_utf8(result.stderr)
                .unwrap()
                .to_ascii_lowercase()
                .contains("no such process")
        );
    }
    #[cfg(not(unix))]
    let _pid = pid;
}

#[tokio::test(flavor = "multi_thread")]
async fn html_client는_실제_성공_deadline_요청취소와_root_shutdown의_child를_회수한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (started, pid) = oneshot::channel();
    let document = prepare(
        &tasks,
        PathBuf::from(EXECUTABLE),
        request(TIMEOUT, b"<p>synthetic client".to_vec()),
        move |pid| {
            let _result = started.send(pid);
        },
    )
    .await
    .unwrap();
    assert!(document.contains("<p>synthetic client</p>"));
    assert!(document.contains("Content-Security-Policy"));
    assert_eq!(tasks.tracked_count(), 0);
    assert_reaped(pid.await.unwrap());

    let (started, pid) = oneshot::channel();
    let result = prepare(
        &tasks,
        PathBuf::from(EXECUTABLE),
        request(Duration::ZERO, b"<p>deadline".to_vec()),
        move |pid| {
            let _result = started.send(pid);
        },
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("deadline"));
    assert_eq!(tasks.tracked_count(), 0);
    assert_reaped(pid.await.unwrap());

    let (started, pid) = oneshot::channel();
    let owned = tasks.clone();
    let caller = tokio::spawn(async move {
        prepare(
            &owned,
            PathBuf::from(EXECUTABLE),
            request(TIMEOUT, vec![b'x'; PENDING_BYTES]),
            move |pid| {
                let _result = started.send(pid);
            },
        )
        .await
    });
    let pid = tokio::time::timeout(TIMEOUT, pid).await.unwrap().unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert_reaped(pid);

    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (started, pid) = oneshot::channel();
    let owned = tasks.clone();
    let caller = tasks
        .spawn_transient_handle("synthetic-html-caller", async move {
            let _result = prepare(
                &owned,
                PathBuf::from(EXECUTABLE),
                request(TIMEOUT, vec![b'x'; PENDING_BYTES]),
                move |pid| {
                    let _result = started.send(pid);
                },
            )
            .await;
        })
        .unwrap();
    let pid = tokio::time::timeout(TIMEOUT, pid).await.unwrap().unwrap();
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert_eq!(tasks.tracked_count(), 0);
    assert_reaped(pid);
    assert!(
        prepare(
            &tasks,
            PathBuf::from(EXECUTABLE),
            request(TIMEOUT, Vec::new()),
            |_| panic!("stopped supervisor must not spawn")
        )
        .await
        .is_err()
    );
}

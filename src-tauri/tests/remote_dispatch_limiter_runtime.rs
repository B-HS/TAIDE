use std::time::Duration;

use taide_runtime::RemoteDispatchLimiter;

const TEST_WAIT_MS: u64 = 100;

#[tokio::test]
async fn 복제한_제한기는_같은_세마포어에서_대기한다() {
    let limiter = RemoteDispatchLimiter::new(1);
    let legacy_limiter = limiter.clone();
    let permit = limiter.acquire().await.expect("첫 permit");

    assert!(tokio::time::timeout(Duration::from_millis(TEST_WAIT_MS), legacy_limiter.acquire())
        .await
        .is_err());

    drop(permit);
    assert!(tokio::time::timeout(Duration::from_millis(TEST_WAIT_MS), legacy_limiter.acquire())
        .await
        .expect("permit 회수")
        .is_some());
}

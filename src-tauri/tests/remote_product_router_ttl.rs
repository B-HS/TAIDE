use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use taide_infra::secret::test_support::InMemorySecretStore;
use taide_infra::secret::SecretStoreState;
use taide_lib::domain::remote::commands::RemoteStore;
use taide_lib::domain::remote::server;
use taide_lib::domain::remote::types::{REMOTE_SESSION_COOKIE_NAME, REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED};
use taide_model::paths::AppPaths;
use taide_remote::types::REMOTE_SESSION_TTL_MS;
use taide_runtime::{AppState, TaskSupervisor};
use tauri::Manager;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let app = tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("제품 Tauri 앱 조립");
    let remote = RemoteStore::default();
    app.manage(AppState::new(AppPaths::new(std::env::temp_dir())));
    app.manage(SecretStoreState(Arc::new(InMemorySecretStore::default())));
    app.manage(TaskSupervisor::new(tokio::runtime::Handle::current()));
    app.manage(remote.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("루프백 테스트 리스너");
    let port = listener.local_addr().expect("루프백 주소").port();
    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let router = server::build_router(app.handle().clone());
    let server_handle = tokio::spawn(server::serve(listener, router, shutdown_rx));
    assert!(remote.mark_started(u32::from(port), shutdown_tx, server_handle));

    let session = remote.issue_session_without_nonce();
    let cookie = format!("{REMOTE_SESSION_COOKIE_NAME}={session}");
    let mut request = format!("ws://127.0.0.1:{port}/__taide/ws")
        .into_client_request()
        .expect("제품 웹소켓 요청");
    request.headers_mut().insert("Cookie", cookie.parse().expect("세션 쿠키"));
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.expect("제품 웹소켓 연결");

    tokio::time::timeout(REQUEST_TIMEOUT, async {
        while remote.status().client_count != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("제품 웹소켓 연결 등록");
    tokio::task::yield_now().await;
    tokio::time::pause();
    tokio::time::advance(Duration::from_millis(REMOTE_SESSION_TTL_MS)).await;

    let response = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{port}/"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("만료된 제품 HTTP 요청");
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    let frame = tokio::time::timeout(REQUEST_TIMEOUT, socket.next())
        .await
        .expect("만료 종료 대기")
        .expect("제품 웹소켓 프레임")
        .expect("유효한 제품 웹소켓 프레임");
    let Message::Close(Some(close)) = frame else {
        panic!("제품 웹소켓 만료 종료 프레임이 아닙니다");
    };
    assert_eq!(u16::from(close.code), REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED);

    let shutdown = remote.take_shutdown_state().expect("제품 원격 서버 상태");
    if let Some(handle) = shutdown.server_handle {
        handle.abort();
    }
    drop(app);
}

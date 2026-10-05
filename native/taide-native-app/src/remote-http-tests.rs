use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use taide_infra::secret::{SecretStore, SecretStoreState};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::types::REMOTE_LOGIN_MAX_ATTEMPTS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const SYNTHETIC_PASSWORD: &str = "synthetic + 한글 password";
const FORM_PASSWORD: &str = "password=synthetic+%2B+%ED%95%9C%EA%B8%80+password";
const INDEX_CONTENT: &[u8] = b"<!doctype html><title>synthetic remote asset</title>";
const FILE_CONTENT: &[u8] = b"synthetic remote project file";
const HEADER_END: &[u8] = b"\r\n\r\n";
const STREAM_FIXTURE_BYTES: usize = 64 * 1024 + 1;
const PARTIAL_BODY_BYTES: usize = 100;
const SERVER_AND_CONNECTION_OWNERS: usize = 3;

#[derive(Default)]
struct Secret {
    hash: Mutex<Option<String>>,
    failed: AtomicBool,
    reads: AtomicUsize,
}

impl SecretStore for Secret {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
        assert_eq!(account, SecretAccount::RemoteAccess);
        *self.hash.lock().unwrap() = Some(value.into());
        Ok(())
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        assert_eq!(account, SecretAccount::RemoteAccess);
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.failed.load(Ordering::SeqCst) {
            return Err(AppError::Internal("synthetic secret access failure".into()));
        }
        Ok(self.hash.lock().unwrap().clone())
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        assert_eq!(account, SecretAccount::RemoteAccess);
        *self.hash.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    secret: Arc<Secret>,
    sink: Arc<Sink>,
    project: Project,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-http-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = Project {
            id: ProjectId::new(),
            root: directory.join("project").to_str().unwrap().into(),
            name: "synthetic remote project".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        };
        state
            .projects
            .write()
            .insert(project.id.clone(), project.clone());
        let sink = Arc::new(Sink::default());
        let secret = Arc::new(Secret::default());
        let mut services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        Arc::get_mut(&mut services).unwrap().secrets = SecretStoreState(secret.clone());
        let ports = Arc::new(Ports {
            socket: Arc::new(|_, _, _| panic!("HTTP fixture must not run the WebSocket consumer")),
            assets: Arc::new(|path| {
                (path == "index.html").then(|| remote_serving::Asset {
                    mime: "text/html; charset=utf-8".into(),
                    bytes: INDEX_CONTENT.to_vec(),
                })
            }),
        });
        Self {
            directory,
            services,
            ports,
            secret,
            sink,
            project,
        }
    }

    async fn start(&self) -> RemoteStatus {
        start(self.services.clone(), self.ports.clone())
            .await
            .unwrap()
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.services.remote.port())
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        host: &str,
        extra: &[(&str, &str)],
        body: &str,
    ) -> WireResponse {
        let mut socket = TcpStream::connect((
            "127.0.0.1",
            u16::try_from(self.services.remote.port()).unwrap(),
        ))
        .await
        .unwrap();
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in extra {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        request.push_str(body);
        socket.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        socket.read_to_end(&mut response).await.unwrap();
        WireResponse::parse(response)
    }

    async fn finish(&self) {
        stop(&self.services);
        self.services.tasks.shutdown().await;
        assert!(!self.services.remote.is_running());
        assert_eq!(self.services.remote.port(), 0);
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        stop(&self.services);
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

struct WireResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl WireResponse {
    fn parse(bytes: Vec<u8>) -> Self {
        let split = bytes
            .windows(HEADER_END.len())
            .position(|window| window == HEADER_END)
            .unwrap();
        let head = std::str::from_utf8(&bytes[..split]).unwrap();
        let mut lines = head.lines();
        let status = StatusCode::from_u16(
            lines
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap(),
        )
        .unwrap();
        let mut headers = HeaderMap::new();
        for line in lines {
            let (key, value) = line.split_once(':').unwrap();
            headers.append(
                axum::http::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                HeaderValue::from_str(value.trim()).unwrap(),
            );
        }
        Self {
            status,
            headers,
            body: bytes[split + HEADER_END.len()..].to_vec(),
        }
    }

    fn cookie(&self, name: &str) -> String {
        self.headers
            .get_all(header::SET_COOKIE)
            .iter()
            .find_map(|value| {
                let pair = value.to_str().unwrap().split(';').next().unwrap();
                let (key, _) = pair.split_once('=')?;
                (key == name).then(|| pair.to_string())
            })
            .unwrap()
    }
}

#[tokio::test]
async fn native_remote_http는_실제_host_origin_단일링크_cookie_파일과_서버수명을_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let status = fixture.start().await;
        assert!(status.running);
        assert_eq!(fixture.start().await.port, status.port);
        let host = fixture.host();
        assert_eq!(
            fixture.request("GET", "/", &host, &[], "").await.status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            fixture
                .request("GET", "/__taide/ws", &host, &[], "")
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
        for forbidden in ["attacker.invalid", "127.0.0.1", "127.0.0.1:1"] {
            assert_eq!(
                fixture.request("GET", "/", forbidden, &[], "").await.status,
                StatusCode::FORBIDDEN
            );
        }
        assert_eq!(
            fixture
                .request("POST", REMOTE_LOGIN_PATH, &host, &[], "")
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            fixture
                .request(
                    "GET",
                    "/",
                    &host,
                    &[("Origin", "https://attacker.invalid")],
                    ""
                )
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        let token = fixture.services.remote.issue_link_token();
        let response = fixture
            .request(
                "GET",
                &format!("/?t={token}"),
                &host,
                &[("X-Forwarded-Proto", "https")],
                "",
            )
            .await;
        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, INDEX_CONTENT);
        assert_eq!(response.headers[header::CACHE_CONTROL], "no-store");
        assert!(
            response.headers[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .contains("script-src 'self'")
        );
        let set_cookie = response.headers[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.contains("HttpOnly; SameSite=Strict; Path=/"));
        assert!(!set_cookie.contains("; Secure"));
        let cookie = response.cookie(SESSION_COOKIE_NAME);
        assert_eq!(
            fixture
                .request("GET", &format!("/?t={token}"), &host, &[], "")
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            fixture
                .request("GET", "/synthetic/route", &host, &[("Cookie", &cookie)], "")
                .await
                .body,
            INDEX_CONTENT
        );
        let file = PathBuf::from(&fixture.project.root).join("synthetic space.txt");
        std::fs::write(&file, FILE_CONTENT).unwrap();
        let encoded = file.to_str().unwrap().replace(' ', "%20");
        let path = format!("/__taide/file?path={encoded}");
        let full = fixture
            .request("GET", &path, &host, &[("Cookie", &cookie)], "")
            .await;
        assert_eq!(full.status, StatusCode::OK);
        assert_eq!(full.body, FILE_CONTENT);
        assert_eq!(full.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
        let range = fixture
            .request(
                "GET",
                &path,
                &host,
                &[("Cookie", &cookie), ("Range", "bytes=0-0")],
                "",
            )
            .await;
        assert_eq!(range.status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(range.body, &FILE_CONTENT[..1]);
        assert_eq!(
            fixture
                .request(
                    "GET",
                    &path,
                    &host,
                    &[("Cookie", &cookie), ("Range", "bytes=999999-999999")],
                    ""
                )
                .await
                .status,
            StatusCode::RANGE_NOT_SATISFIABLE
        );
        assert_eq!(
            fixture
                .request("GET", "/__taide/file", &host, &[("Cookie", &cookie)], "")
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
        let outside = fixture.directory.join("synthetic outside.txt");
        std::fs::write(&outside, b"synthetic outside").unwrap();
        let outside = format!(
            "/__taide/file?path={}",
            outside.to_str().unwrap().replace(' ', "%20")
        );
        assert_eq!(
            fixture
                .request("GET", &outside, &host, &[("Cookie", &cookie)], "")
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        let larger = vec![b'x'; STREAM_FIXTURE_BYTES];
        std::fs::write(&file, &larger).unwrap();
        assert_eq!(
            fixture
                .request("GET", &path, &host, &[("Cookie", &cookie)], "")
                .await
                .body,
            larger
        );
        fixture.services.remote.revoke_all_sessions();
        assert_eq!(
            fixture
                .request("GET", "/", &host, &[("Cookie", &cookie)], "")
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
        stop(&fixture.services);
        let next = fixture.start().await;
        assert!(next.running);
        assert_eq!(
            fixture
                .request("GET", "/", &fixture.host(), &[("Cookie", &cookie)], "")
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
        fixture.finish().await;
        assert!(fixture.sink.0.lock().unwrap().iter().any(
            |event| matches!(event, AppEvent::RemoteStateChanged { status } if !status.running)
        ));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_remote_http는_실제_password_nonce_검증_failclosed와_독립_lockout을_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        fixture
            .secret
            .set(
                SecretAccount::RemoteAccess,
                &service::hash_password(SYNTHETIC_PASSWORD),
            )
            .unwrap();
        fixture.services.state.settings.write().remote_allowed_hosts =
            vec!["synthetic-tunnel.invalid".into()];
        refresh_password_configured_cache(&fixture.services);
        assert!(fixture.services.remote.status().password_configured);
        fixture.start().await;
        let host = fixture.host();
        let origin = format!("http://{host}");
        let token = fixture.services.remote.issue_link_token();
        let linked = fixture
            .request("GET", &format!("/?t={token}"), &host, &[], "")
            .await;
        assert_eq!(linked.status, StatusCode::SEE_OTHER);
        assert_eq!(linked.headers[header::LOCATION], REMOTE_LOGIN_PATH);
        let nonce_cookie = linked.cookie(LOGIN_NONCE_COOKIE_NAME);
        let page = fixture
            .request(
                "GET",
                REMOTE_LOGIN_PATH,
                &host,
                &[("Cookie", &nonce_cookie)],
                "",
            )
            .await;
        assert_eq!(page.status, StatusCode::OK);
        assert_eq!(
            page.headers[header::CONTENT_SECURITY_POLICY],
            login_page::LOGIN_PAGE_CSP
        );
        let before = fixture.secret.reads.load(Ordering::SeqCst);
        let forged = format!("{LOGIN_NONCE_COOKIE_NAME}=synthetic-forged-nonce");
        let denied = fixture
            .request(
                "POST",
                REMOTE_LOGIN_PATH,
                &host,
                &[("Cookie", &forged), ("Origin", &origin)],
                FORM_PASSWORD,
            )
            .await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED);
        assert_eq!(fixture.secret.reads.load(Ordering::SeqCst), before);
        let wrong = fixture
            .request(
                "POST",
                REMOTE_LOGIN_PATH,
                &host,
                &[("Cookie", &nonce_cookie), ("Origin", &origin)],
                "password=synthetic-wrong",
            )
            .await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        let granted = fixture
            .request(
                "POST",
                REMOTE_LOGIN_PATH,
                &host,
                &[("Cookie", &nonce_cookie), ("Origin", &origin)],
                FORM_PASSWORD,
            )
            .await;
        assert_eq!(granted.status, StatusCode::SEE_OTHER);
        assert!(
            granted
                .headers
                .get_all(header::SET_COOKIE)
                .iter()
                .any(|value| value.to_str().unwrap().contains("Max-Age=0"))
        );
        let cookie = granted.cookie(SESSION_COOKIE_NAME);
        assert_eq!(
            fixture
                .request("GET", "/", &host, &[("Cookie", &cookie)], "")
                .await
                .status,
            StatusCode::OK
        );
        assert_eq!(
            fixture
                .request(
                    "POST",
                    REMOTE_LOGIN_PATH,
                    &host,
                    &[("Cookie", &nonce_cookie), ("Origin", &origin)],
                    FORM_PASSWORD
                )
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
        fixture
            .services
            .state
            .settings
            .write()
            .remote_password_only_login = true;
        for _ in 0..=REMOTE_LOGIN_MAX_ATTEMPTS {
            assert_eq!(
                fixture
                    .request(
                        "POST",
                        REMOTE_LOGIN_PATH,
                        &host,
                        &[("Origin", &origin)],
                        "password=synthetic-wrong"
                    )
                    .await
                    .status,
                StatusCode::UNAUTHORIZED
            );
        }
        assert_eq!(
            fixture
                .request(
                    "POST",
                    REMOTE_LOGIN_PATH,
                    &host,
                    &[("Origin", &origin)],
                    FORM_PASSWORD
                )
                .await
                .status,
            StatusCode::TOO_MANY_REQUESTS
        );
        let tunnel = "synthetic-tunnel.invalid";
        let token = fixture.services.remote.issue_link_token();
        let linked = fixture
            .request(
                "GET",
                &format!("/?t={token}"),
                tunnel,
                &[("X-Forwarded-Proto", " HTTPS , http")],
                "",
            )
            .await;
        assert!(
            linked.headers[header::SET_COOKIE]
                .to_str()
                .unwrap()
                .contains("; Secure")
        );
        let nonce_cookie = linked.cookie(LOGIN_NONCE_COOKIE_NAME);
        let tunnel_origin = "https://synthetic-tunnel.invalid";
        let granted = fixture
            .request(
                "POST",
                REMOTE_LOGIN_PATH,
                tunnel,
                &[
                    ("Cookie", &nonce_cookie),
                    ("Origin", tunnel_origin),
                    ("X-Forwarded-Proto", "https"),
                ],
                FORM_PASSWORD,
            )
            .await;
        assert_eq!(granted.status, StatusCode::SEE_OTHER);
        assert!(
            granted
                .headers
                .get_all(header::SET_COOKIE)
                .iter()
                .any(|value| value.to_str().unwrap().contains("; Secure"))
        );
        fixture.secret.failed.store(true, Ordering::SeqCst);
        refresh_password_configured_cache(&fixture.services);
        assert!(fixture.services.remote.status().password_configured);
        assert_eq!(
            fixture
                .request(
                    "POST",
                    REMOTE_LOGIN_PATH,
                    &host,
                    &[("Cookie", &nonce_cookie), ("Origin", &origin)],
                    FORM_PASSWORD
                )
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
        let token = fixture.services.remote.issue_link_token();
        let failed_link = fixture
            .request("GET", &format!("/?t={token}"), &host, &[], "")
            .await;
        assert_eq!(failed_link.status, StatusCode::SEE_OTHER);
        let failed_nonce = failed_link.cookie(LOGIN_NONCE_COOKIE_NAME);
        assert_eq!(
            fixture
                .request(
                    "POST",
                    REMOTE_LOGIN_PATH,
                    &host,
                    &[("Cookie", &failed_nonce), ("Origin", &origin)],
                    FORM_PASSWORD
                )
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        fixture.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_remote_http는_변경없는_toggle과_shutdown_등록거절을_서버기동으로_오인하지않는다() {
    let fixture = Fixture::new();
    apply_toggle(
        fixture.services.clone(),
        fixture.ports.clone(),
        false,
        false,
    )
    .await;
    assert!(!fixture.services.remote.is_running());
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    fixture.services.state.begin_shutdown();
    assert!(
        start(fixture.services.clone(), fixture.ports.clone())
            .await
            .is_err()
    );
    fixture.finish().await;
    let stopped = Fixture::new();
    stopped.services.tasks.stop_all();
    assert!(
        start(stopped.services.clone(), stopped.ports.clone())
            .await
            .is_err()
    );
    assert!(stopped.sink.0.lock().unwrap().is_empty());
    stopped.finish().await;
}

#[tokio::test]
async fn native_remote_http는_미완성_header_body의_연결도_stop에서_감독하여_회수한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let status = fixture.start().await;
        let address = ("127.0.0.1", u16::try_from(status.port).unwrap());
        let mut header = TcpStream::connect(address).await.unwrap();
        header.write_all(b"GET / HTTP/1.1\r\n").await.unwrap();
        let mut body = TcpStream::connect(address).await.unwrap();
        let host = fixture.host();
        body.write_all(format!("POST {REMOTE_LOGIN_PATH} HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nContent-Length: {PARTIAL_BODY_BYTES}\r\n\r\np").as_bytes()).await.unwrap();
        while fixture.services.tasks.tracked_count() <= SERVER_AND_CONNECTION_OWNERS { tokio::task::yield_now().await; }
        stop(&fixture.services);
        while fixture.services.tasks.tracked_count() != 0 { tokio::task::yield_now().await; }
        assert!(TcpStream::connect(address).await.is_err());
        fixture.finish().await;
        for mut socket in [header, body] {
            let mut bytes = Vec::new();
            match socket.read_to_end(&mut bytes).await {
                Ok(_) => {}
                Err(error) => assert!(matches!(error.kind(), std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::UnexpectedEof)),
            }
        }
    }).await.unwrap();
}

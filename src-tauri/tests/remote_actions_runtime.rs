use std::sync::{Arc, Mutex};

use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::paths::AppPaths;
use taide_model::remote::RemoteStatus;
use taide_remote::store::RemoteStore;
use taide_remote::types::{REMOTE_LINK_TOKEN_QUERY_KEY, REMOTE_PASSWORD_MIN_LEN};
use taide_runtime::{remote_actions, AppState};
use tokio::sync::watch;
use uuid::Uuid;

const FIXTURE_PORT: u32 = 53_211;
const REMOTE_COMMANDS: &[&str] = &[
    "remote_status",
    "remote_issue_link",
    "remote_revoke_sessions",
    "remote_set_password",
    "remote_clear_password",
];

struct MemorySecret {
    remote: RemoteStore,
    active_session: String,
    configured_before: bool,
    should_fail_set: bool,
    should_fail_delete: bool,
    stored: Mutex<Option<String>>,
    operations: Mutex<Vec<(&'static str, SecretAccount)>>,
}

impl MemorySecret {
    fn new(remote: &RemoteStore, configured_before: bool, should_fail_set: bool, should_fail_delete: bool) -> Arc<Self> {
        remote.set_password_configured(configured_before);
        Arc::new(Self {
            remote: remote.clone(),
            active_session: remote.issue_session_without_nonce(),
            configured_before,
            should_fail_set,
            should_fail_delete,
            stored: Mutex::new(None),
            operations: Mutex::new(Vec::new()),
        })
    }

    fn assert_before_mutation(&self) {
        assert_eq!(self.remote.status().password_configured, self.configured_before);
        assert!(self.remote.has_active_session(&self.active_session));
        assert_eq!(*self.remote.subscribe_session_epoch().borrow(), 0);
    }
}

impl SecretStore for MemorySecret {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
        self.assert_before_mutation();
        self.operations.lock().unwrap().push(("set", account));
        if self.should_fail_set {
            return Err(AppError::Internal("fixture set failure".to_string()));
        }
        *self.stored.lock().unwrap() = Some(value.to_string());
        Ok(())
    }

    fn get(&self, _: SecretAccount) -> AppResult<Option<String>> {
        panic!("이 다섯 action은 keyring 조회를 하지 않는다")
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        self.assert_before_mutation();
        self.operations.lock().unwrap().push(("delete", account));
        if self.should_fail_delete {
            return Err(AppError::Internal("fixture delete failure".to_string()));
        }
        *self.stored.lock().unwrap() = None;
        Ok(())
    }
}

fn state() -> AppState {
    AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-remote-actions-{}", Uuid::new_v4())),
    ))
}

#[tokio::test]
async fn status는_cache만_읽고_정지한_link는_token을_발급하지_않는다() {
    let state = state();
    let remote = RemoteStore::default();
    assert_eq!(remote_actions::remote_status(&remote).await.unwrap(), RemoteStatus::default());
    remote.set_password_configured(true);
    assert!(remote_actions::remote_status(&remote).await.unwrap().password_configured);
    let prior_token = remote.issue_link_token();
    let error = remote_actions::remote_issue_link(&state, &remote).await.unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(
        remote.consume_link_token(&prior_token).is_some(),
        "거절 경로가 기존 token을 대체하지 않는다"
    );
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn link는_settings의_첫_구체_host를_쓰고_단일_사용_token을_발급한다() {
    let state = state();
    let remote = RemoteStore::default();
    let (shutdown, _) = watch::channel(());
    assert!(remote.mark_started(FIXTURE_PORT, shutdown, tokio::spawn(async {})));
    state.settings.write().remote_allowed_hosts = vec![
        "*.example.test".to_string(),
        "first.example.test".to_string(),
        "second.example.test".to_string(),
    ];
    let link = remote_actions::remote_issue_link(&state, &remote).await.unwrap();
    let prefix = format!("https://first.example.test/?{REMOTE_LINK_TOKEN_QUERY_KEY}=");
    let token = link.url.strip_prefix(&prefix).unwrap();
    assert!(remote.consume_link_token(token).is_some());
    assert!(remote.consume_link_token(token).is_none());
    state.settings.write().remote_allowed_hosts.clear();
    let loopback = remote_actions::remote_issue_link(&state, &remote).await.unwrap();
    let loopback_prefix = format!("http://127.0.0.1:{FIXTURE_PORT}/?{REMOTE_LINK_TOKEN_QUERY_KEY}=");
    assert!(remote
        .consume_link_token(loopback.url.strip_prefix(&loopback_prefix).unwrap())
        .is_some());
    let shutdown = remote.take_shutdown_state().unwrap();
    shutdown.server_handle.unwrap().await.unwrap();
    assert!(
        !state.paths.data_dir.exists(),
        "no-op handle만 쓰고 실제 서버/파일을 생성하지 않는다"
    );
}

#[tokio::test]
async fn 짧은_unicode_password는_secret_접근과_cache_revoke_전에_거절한다() {
    let remote = RemoteStore::default();
    let memory = MemorySecret::new(&remote, false, false, false);
    let epoch = remote.subscribe_session_epoch();
    let password = format!("  {}  ", "가".repeat(REMOTE_PASSWORD_MIN_LEN - 1));
    let error = remote_actions::remote_set_password(&remote, &SecretStoreState(memory.clone()), password)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(memory.operations.lock().unwrap().is_empty());
    assert!(!remote.status().password_configured);
    assert!(remote.has_active_session(&memory.active_session));
    assert!(!epoch.has_changed().unwrap());
}

#[tokio::test]
async fn password_설정은_trim된_hash를_secret에_쓴_뒤_cache와_epoch를_갱신한다() {
    let remote = RemoteStore::default();
    let memory = MemorySecret::new(&remote, false, false, false);
    let epoch = remote.subscribe_session_epoch();
    let candidate = "가".repeat(REMOTE_PASSWORD_MIN_LEN);
    let supplied = format!("  {candidate}  ");
    remote_actions::remote_set_password(&remote, &SecretStoreState(memory.clone()), supplied.clone())
        .await
        .unwrap();
    let stored = memory.stored.lock().unwrap();
    let hash = stored.as_deref().unwrap();
    assert_ne!(hash, candidate);
    assert!(taide_remote::service::verify_password(hash, &candidate));
    assert!(!taide_remote::service::verify_password(hash, &supplied));
    assert_eq!(*memory.operations.lock().unwrap(), [("set", SecretAccount::RemoteAccess)]);
    assert!(remote.status().password_configured);
    assert!(!remote.has_active_session(&memory.active_session));
    assert!(epoch.has_changed().unwrap());
    assert_eq!(*epoch.borrow(), 1);
}

#[tokio::test]
async fn secret_설정_실패는_기존_cache와_session_epoch를_유지한다() {
    let remote = RemoteStore::default();
    let memory = MemorySecret::new(&remote, false, true, false);
    let epoch = remote.subscribe_session_epoch();
    let error = remote_actions::remote_set_password(&remote, &SecretStoreState(memory.clone()), "x".repeat(REMOTE_PASSWORD_MIN_LEN))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert_eq!(*memory.operations.lock().unwrap(), [("set", SecretAccount::RemoteAccess)]);
    assert!(!remote.status().password_configured);
    assert!(remote.has_active_session(&memory.active_session));
    assert!(!epoch.has_changed().unwrap());
}

#[tokio::test]
async fn password_해제는_secret_delete_성공_뒤에만_cache와_session을_비운다() {
    for should_fail in [false, true] {
        let remote = RemoteStore::default();
        let memory = MemorySecret::new(&remote, true, false, should_fail);
        let epoch = remote.subscribe_session_epoch();
        let result = remote_actions::remote_clear_password(&remote, &SecretStoreState(memory.clone())).await;
        assert_eq!(*memory.operations.lock().unwrap(), [("delete", SecretAccount::RemoteAccess)]);
        if should_fail {
            assert_eq!(result.unwrap_err().kind(), AppErrorKind::Internal);
            assert!(remote.status().password_configured);
            assert!(remote.has_active_session(&memory.active_session));
            assert!(!epoch.has_changed().unwrap());
            continue;
        }
        result.unwrap();
        assert!(!remote.status().password_configured);
        assert!(!remote.has_active_session(&memory.active_session));
        assert!(epoch.has_changed().unwrap());
        assert_eq!(*epoch.borrow(), 1);
    }
}

#[tokio::test]
async fn 명시적_revoke는_cache를_유지하고_모든_session과_epoch를_갱신한다() {
    let remote = RemoteStore::default();
    remote.set_password_configured(true);
    let first = remote.issue_session_without_nonce();
    let second = remote.issue_session_without_nonce();
    let epoch = remote.subscribe_session_epoch();
    remote_actions::remote_revoke_sessions(&remote).await.unwrap();
    assert!(!remote.has_active_session(&first));
    assert!(!remote.has_active_session(&second));
    assert!(remote.status().password_configured);
    assert!(epoch.has_changed().unwrap());
    assert_eq!(*epoch.borrow(), 1);
}

#[test]
fn 다섯_공개_command는_기존_tauri_경로에서_runtime으로_위임한다() {
    let source = include_str!("../src/domain/remote/commands.rs");
    for command in REMOTE_COMMANDS {
        let body = source
            .split_once(&format!("pub async fn {command}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains(&format!("remote_actions::{command}(")), "{command}");
    }
}

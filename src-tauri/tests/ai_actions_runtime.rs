use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::ai::{AiCommitMessageRequest, AiInlineCompleteRequest, AiInlineEditRequest, AiProviderId};
use taide_model::error::{AppError, AppResult};
use taide_model::paths::AppPaths;
use taide_runtime::{ai_actions, AiRequestStore, AppState};
use uuid::Uuid;

const PREFIX_MAX_BYTES: usize = 32 * 1024;
const SUFFIX_MAX_BYTES: usize = 16 * 1024;
const SELECTION_MAX_BYTES: usize = 100 * 1024;
const INSTRUCTION_MAX_BYTES: usize = 4 * 1024;
const DIFF_MAX_BYTES: usize = 64 * 1024;
const RECENT_COMMITS_MAX_BYTES: usize = 8 * 1024;

#[derive(Default)]
struct EmptySecretStore {
    reads: AtomicUsize,
    deleted: Mutex<Vec<SecretAccount>>,
}

impl SecretStore for EmptySecretStore {
    fn set(&self, _account: SecretAccount, _value: &str) -> AppResult<()> {
        Err(AppError::Internal("fixture does not accept credentials".to_string()))
    }

    fn get(&self, _account: SecretAccount) -> AppResult<Option<String>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(None)
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        self.deleted.lock().unwrap().push(account);
        Ok(())
    }
}

fn fixture() -> (AppState, SecretStoreState, Arc<EmptySecretStore>) {
    let dir = std::env::temp_dir().join(format!("taide-ai-actions-{}", Uuid::new_v4()));
    let state = AppState::new(AppPaths::new(dir));
    let memory = Arc::new(EmptySecretStore::default());
    let secret = SecretStoreState(memory.clone());
    (state, secret, memory)
}

fn complete_request() -> AiInlineCompleteRequest {
    AiInlineCompleteRequest {
        request_id: "fixture-request".to_string(),
        owner: "main".to_string(),
        provider: AiProviderId::Omlx,
        model: "fixture-model".to_string(),
        prefix: String::new(),
        suffix: String::new(),
        language: "rust".to_string(),
        file_path: String::new(),
    }
}

fn edit_request() -> AiInlineEditRequest {
    AiInlineEditRequest {
        request_id: "fixture-request".to_string(),
        owner: "main".to_string(),
        provider: Some(AiProviderId::Omlx),
        model: Some("fixture-model".to_string()),
        selection: String::new(),
        instruction: String::new(),
        language: "rust".to_string(),
        file_path: String::new(),
        prefix: String::new(),
        suffix: String::new(),
    }
}

fn commit_request() -> AiCommitMessageRequest {
    AiCommitMessageRequest {
        request_id: "fixture-request".to_string(),
        owner: "main".to_string(),
        provider: Some(AiProviderId::Omlx),
        model: Some("fixture-model".to_string()),
        diff_text: String::new(),
        recent_commits: String::new(),
    }
}

fn assert_idle(store: &AiRequestStore) {
    let (token, _receiver) = store.begin("main", "fixture-request").expect("요청 슬롯 회수");
    store.finish("main", "fixture-request", &token);
}

#[tokio::test]
async fn 토큰_상태는_현재_omlx_설정_snapshot과_주입된_메모리_port를_쓴다() {
    let (state, secret, _) = fixture();
    let empty = ai_actions::ai_token_status(&state, &secret).await.unwrap();
    assert!(!empty.ollama_cloud && !empty.codex && !empty.omlx);
    state.settings.write().ai_omlx_base_url = Some("http://127.0.0.1:1".to_string());
    assert!(ai_actions::ai_token_status(&state, &secret).await.unwrap().omlx);
}

#[tokio::test]
async fn 모든_입력_상한은_begin과_secret_접근_전에_거절한다() {
    let (state, secret, memory) = fixture();
    let store = AiRequestStore::new();
    for is_prefix in [true, false] {
        let mut request = complete_request();
        let field = if is_prefix { "prefix" } else { "suffix" };
        if is_prefix {
            request.prefix = "가".repeat(PREFIX_MAX_BYTES / "가".len() + 1);
        } else {
            request.suffix = "x".repeat(SUFFIX_MAX_BYTES + 1);
        }
        let error = ai_actions::ai_inline_complete(&state, &store, &secret, request).await.unwrap_err();
        assert!(matches!(error, AppError::InvalidArgument(message) if message.contains(field)));
        assert_idle(&store);
    }
    for is_selection in [true, false] {
        let mut request = edit_request();
        let field = if is_selection { "selection" } else { "instruction" };
        if is_selection {
            request.selection = "x".repeat(SELECTION_MAX_BYTES + 1);
        } else {
            request.instruction = "x".repeat(INSTRUCTION_MAX_BYTES + 1);
        }
        let error = ai_actions::ai_inline_edit(&state, &store, &secret, request).await.unwrap_err();
        assert!(matches!(error, AppError::InvalidArgument(message) if message.contains(field)));
        assert_idle(&store);
    }
    for is_diff in [true, false] {
        let mut request = commit_request();
        let field = if is_diff { "diffText" } else { "recentCommits" };
        if is_diff {
            request.diff_text = "x".repeat(DIFF_MAX_BYTES + 1);
        } else {
            request.recent_commits = "x".repeat(RECENT_COMMITS_MAX_BYTES + 1);
        }
        let error = ai_actions::ai_commit_message(&state, &store, &secret, request).await.unwrap_err();
        assert!(matches!(error, AppError::InvalidArgument(message) if message.contains(field)));
        assert_idle(&store);
    }
    assert_eq!(memory.reads.load(Ordering::SeqCst), 0);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn edit과_commit의_설정_해석_오류는_중복_begin보다_먼저다() {
    let (state, secret, memory) = fixture();
    let store = AiRequestStore::new();
    let (token, _receiver) = store.begin("main", "fixture-request").unwrap();
    let mut edit = edit_request();
    edit.provider = None;
    edit.model = None;
    let mut commit = commit_request();
    commit.provider = None;
    commit.model = None;
    let edit_error = ai_actions::ai_inline_edit(&state, &store, &secret, edit).await.unwrap_err();
    let commit_error = ai_actions::ai_commit_message(&state, &store, &secret, commit).await.unwrap_err();
    for error in [edit_error, commit_error] {
        assert!(matches!(error, AppError::InvalidArgument(message) if message == "AI provider is not configured"));
    }
    assert!(store.begin("main", "fixture-request").is_none());
    store.finish("main", "fixture-request", &token);
    assert_idle(&store);
    assert_eq!(memory.reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn complete_중복은_기존_요청을_유지하고_provider를_호출하지_않는다() {
    let (state, secret, memory) = fixture();
    let store = AiRequestStore::new();
    let (token, _receiver) = store.begin("main", "fixture-request").unwrap();
    let error = ai_actions::ai_inline_complete(&state, &store, &secret, complete_request())
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::InvalidArgument(message) if message.contains("inline completion request")));
    assert!(store.begin("main", "fixture-request").is_none());
    store.finish("main", "fixture-request", &token);
    assert_eq!(memory.reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn provider_구성_실패는_세_요청의_identity를_finish하고_네트워크를_시작하지_않는다() {
    let (state, secret, memory) = fixture();
    let store = AiRequestStore::new();
    let complete = ai_actions::ai_inline_complete(&state, &store, &secret, complete_request()).await;
    assert_idle(&store);
    let edit = ai_actions::ai_inline_edit(&state, &store, &secret, edit_request()).await;
    assert_idle(&store);
    let commit = ai_actions::ai_commit_message(&state, &store, &secret, commit_request()).await;
    assert_idle(&store);
    let models = ai_actions::ai_list_models(&state, &secret, AiProviderId::Omlx).await;
    for error in [complete.unwrap_err(), edit.unwrap_err(), commit.unwrap_err(), models.unwrap_err()] {
        assert!(matches!(error, AppError::InvalidArgument(message) if message == "OMLX base URL is not configured"));
    }
    assert_eq!(memory.reads.load(Ordering::SeqCst), 0);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 취소는_같은_owner만_깨우고_늦은_finish가_새_요청을_지우지_않는다() {
    let store = AiRequestStore::new();
    let (old, main) = store.begin("main", "fixture-request").unwrap();
    let (editor_token, mut editor) = store.begin("editor-2", "fixture-request").unwrap();
    ai_actions::ai_request_cancel(&store, "main".to_string(), "fixture-request".to_string())
        .await
        .unwrap();
    assert!(main.await.is_ok());
    assert!(matches!(editor.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Empty)));
    let (new, _receiver) = store.begin("main", "fixture-request").unwrap();
    store.finish("main", "fixture-request", &old);
    assert!(store.begin("main", "fixture-request").is_none());
    store.finish("main", "fixture-request", &new);
    store.finish("editor-2", "fixture-request", &editor_token);
}

#[tokio::test]
async fn token_삭제와_빈_omlx_token은_주입된_port만_호출한다() {
    let (_, secret, memory) = fixture();
    ai_actions::ai_clear_token(&secret, AiProviderId::OllamaCloud).await.unwrap();
    ai_actions::ai_set_token(&secret, AiProviderId::Omlx, String::new()).await.unwrap();
    assert_eq!(
        *memory.deleted.lock().unwrap(),
        [SecretAccount::AiOllamaCloud, SecretAccount::AiOmlx],
    );
    assert_eq!(memory.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn 기존_command_여덟_개는_같은_runtime_action으로_위임한다() {
    let source = include_str!("../src/domain/ai/commands.rs")
        .lines()
        .filter(|line| !line.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for command in [
        "ai_token_status",
        "ai_set_token",
        "ai_clear_token",
        "ai_list_models",
        "ai_inline_complete",
        "ai_inline_edit",
        "ai_commit_message",
        "ai_request_cancel",
    ] {
        assert!(source.contains(&format!("ai_actions::{command}(")), "{command}");
    }
    assert!(!source.contains("tokio::select!"));
    assert!(!source.contains("request_store.begin("));
}

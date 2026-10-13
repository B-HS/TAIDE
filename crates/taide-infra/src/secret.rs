use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use uuid::Uuid;

use taide_model::error::{AppError, AppErrorKind, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretAccount {
    AiOllamaCloud,
    AiCodex,
    AiOmlx,
    GithubSync,
    RemoteAccess,
}

impl SecretAccount {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AiOllamaCloud => "ai-ollama-cloud",
            Self::AiCodex => "ai-codex",
            Self::AiOmlx => "ai-omlx",
            Self::GithubSync => "github-sync",
            Self::RemoteAccess => "remote-access",
        }
    }
}

pub trait SecretStore: Send + Sync {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()>;
    fn get(&self, account: SecretAccount) -> AppResult<Option<String>>;
    fn delete(&self, account: SecretAccount) -> AppResult<()>;
    /// Returns the identity of the most recent successful credential write without reading its value.
    fn revision(&self, _account: SecretAccount) -> Option<Uuid> {
        None
    }
}

struct ObservedSecretStore {
    inner: Arc<dyn SecretStore>,
    revisions: Mutex<HashMap<&'static str, Uuid>>,
}

impl SecretStore for ObservedSecretStore {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
        self.inner.set(account, value)?;
        self.revisions.lock().insert(account.as_str(), Uuid::new_v4());
        Ok(())
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        self.inner.get(account)
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        self.inner.delete(account)?;
        self.revisions.lock().insert(account.as_str(), Uuid::new_v4());
        Ok(())
    }

    fn revision(&self, account: SecretAccount) -> Option<Uuid> {
        self.revisions.lock().get(account.as_str()).copied()
    }
}

pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    pub fn new(service: String) -> Self {
        Self { service }
    }

    fn entry(&self, account: SecretAccount) -> AppResult<keyring::Entry> {
        keyring::Entry::new(&self.service, account.as_str()).map_err(|error| {
            AppError::localized(
                AppErrorKind::Internal,
                "error.secret.accessFailed",
                format!("failed to access keyring: {error}"),
            )
            .with_arg("detail", &error)
        })
    }
}

impl SecretStore for KeyringSecretStore {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
        self.entry(account)?.set_password(value).map_err(|error| {
            AppError::localized(
                AppErrorKind::Internal,
                "error.secret.storeFailed",
                format!("failed to store to keyring: {error}"),
            )
            .with_arg("detail", &error)
        })
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        match self.entry(account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::localized(
                AppErrorKind::Internal,
                "error.secret.readFailed",
                format!("failed to read from keyring: {error}"),
            )
            .with_arg("detail", &error)),
        }
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        match self.entry(account)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::localized(
                AppErrorKind::Internal,
                "error.secret.deleteFailed",
                format!("failed to delete from keyring: {error}"),
            )
            .with_arg("detail", &error)),
        }
    }
}

#[derive(Clone)]
pub struct SecretStoreState(pub Arc<dyn SecretStore>);

impl SecretStoreState {
    pub fn new(service: String) -> Self {
        Self::with_store(Arc::new(KeyringSecretStore::new(service)))
    }

    /// Tracks successful writes made through a shared credential port without exposing credential values.
    pub fn with_store(inner: Arc<dyn SecretStore>) -> Self {
        Self(Arc::new(ObservedSecretStore {
            inner,
            revisions: Mutex::new(HashMap::new()),
        }))
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support {
    use std::collections::HashMap;

    use parking_lot::Mutex;

    use super::{AppResult, SecretAccount, SecretStore};

    #[derive(Default)]
    pub struct InMemorySecretStore(Mutex<HashMap<&'static str, String>>);

    impl SecretStore for InMemorySecretStore {
        fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
            self.0.lock().insert(account.as_str(), value.to_string());
            Ok(())
        }

        fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
            Ok(self.0.lock().get(account.as_str()).cloned())
        }

        fn delete(&self, account: SecretAccount) -> AppResult<()> {
            self.0.lock().remove(account.as_str());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::InMemorySecretStore;
    use super::*;

    #[derive(Default)]
    struct Writes {
        failed: std::sync::atomic::AtomicBool,
    }

    impl SecretStore for Writes {
        fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
            if self.failed.load(std::sync::atomic::Ordering::Acquire) {
                return Err(AppError::InvalidArgument("fixture write failed".into()));
            }
            Ok(())
        }
        fn get(&self, _: SecretAccount) -> AppResult<Option<String>> {
            panic!("credential revision must not read its value");
        }
        fn delete(&self, account: SecretAccount) -> AppResult<()> {
            self.set(account, "")
        }
    }

    #[test]
    fn 변경식별자는_성공한_저장과_삭제만_공유하고_다른계정을_보존한다() {
        let writes = Arc::new(Writes::default());
        let store = SecretStoreState::with_store(writes.clone());
        let shared = store.clone();
        assert!(store.0.revision(SecretAccount::AiOmlx).is_none());
        store.0.set(SecretAccount::AiOmlx, "fixture").unwrap();
        let revision = store.0.revision(SecretAccount::AiOmlx).unwrap();
        assert_eq!(shared.0.revision(SecretAccount::AiOmlx), Some(revision));
        assert!(shared.0.revision(SecretAccount::AiCodex).is_none());
        writes.failed.store(true, std::sync::atomic::Ordering::Release);
        assert!(shared.0.set(SecretAccount::AiOmlx, "fixture").is_err());
        assert!(shared.0.delete(SecretAccount::AiOmlx).is_err());
        assert_eq!(shared.0.revision(SecretAccount::AiOmlx), Some(revision));
        writes.failed.store(false, std::sync::atomic::Ordering::Release);
        shared.0.delete(SecretAccount::AiOmlx).unwrap();
        assert_ne!(store.0.revision(SecretAccount::AiOmlx), Some(revision));
    }

    #[test]
    fn 계정_열거값은_서로_다른_키_문자열로_매핑된다() {
        assert_eq!(SecretAccount::AiOllamaCloud.as_str(), "ai-ollama-cloud");
        assert_eq!(SecretAccount::AiCodex.as_str(), "ai-codex");
        assert_eq!(SecretAccount::GithubSync.as_str(), "github-sync");
    }

    #[test]
    fn 저장하지_않은_계정은_none_을_반환한다() {
        let store = InMemorySecretStore::default();
        assert_eq!(store.get(SecretAccount::AiCodex).unwrap(), None);
    }

    #[test]
    fn set_후_get_은_저장한_값을_반환하고_delete_후에는_none_이_된다() {
        let store = InMemorySecretStore::default();

        store.set(SecretAccount::AiOllamaCloud, "token-value").unwrap();
        assert_eq!(store.get(SecretAccount::AiOllamaCloud).unwrap(), Some("token-value".to_string()));

        store.delete(SecretAccount::AiOllamaCloud).unwrap();
        assert_eq!(store.get(SecretAccount::AiOllamaCloud).unwrap(), None);
    }

    #[test]
    fn 계정별로_값이_독립적으로_저장된다() {
        let store = InMemorySecretStore::default();

        store.set(SecretAccount::AiOllamaCloud, "ollama-token").unwrap();
        store.set(SecretAccount::AiCodex, "codex-token").unwrap();

        assert_eq!(store.get(SecretAccount::AiOllamaCloud).unwrap(), Some("ollama-token".to_string()));
        assert_eq!(store.get(SecretAccount::AiCodex).unwrap(), Some("codex-token".to_string()));
    }
}

use std::sync::Arc;

use crate::error::{AppError, AppResult};

pub const SECRET_SERVICE: &str = "dev.taide.app";

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
}

pub struct KeyringSecretStore;

impl KeyringSecretStore {
    fn entry(account: SecretAccount) -> AppResult<keyring::Entry> {
        keyring::Entry::new(SECRET_SERVICE, account.as_str()).map_err(|error| AppError::Internal(format!("keyring 접근 실패: {error}")))
    }
}

impl SecretStore for KeyringSecretStore {
    fn set(&self, account: SecretAccount, value: &str) -> AppResult<()> {
        Self::entry(account)?
            .set_password(value)
            .map_err(|error| AppError::Internal(format!("keyring 저장 실패: {error}")))
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        match Self::entry(account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::Internal(format!("keyring 조회 실패: {error}"))),
        }
    }

    fn delete(&self, account: SecretAccount) -> AppResult<()> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::Internal(format!("keyring 삭제 실패: {error}"))),
        }
    }
}

pub struct SecretStoreState(pub Arc<dyn SecretStore>);

impl Default for SecretStoreState {
    fn default() -> Self {
        Self(Arc::new(KeyringSecretStore))
    }
}

#[cfg(test)]
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

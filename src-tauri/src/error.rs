use std::collections::BTreeMap;

use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub enum AppErrorKind {
    Io,
    NotFound,
    InvalidArgument,
    Forbidden,
    Internal,
}

/// A user-facing error whose text lives in the locale catalog instead of the binary.
/// `kind` keeps the pre-taxonomy `AppError` variant so `IpcError.code` consumers keep branching
/// on the same five values; `fallback` is what the frontend shows when `key` is absent from the
/// active catalog.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedError {
    pub kind: AppErrorKind,
    pub key: String,
    pub args: BTreeMap<String, String>,
    pub fallback: String,
}

impl std::fmt::Display for LocalizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.fallback)
    }
}

#[derive(Debug, thiserror::Error, Serialize, Type)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    #[error("io error: {0}")]
    Io(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("operation failed: {0}")]
    Internal(String),

    #[error("{0}")]
    Localized(LocalizedError),
}

impl AppError {
    pub fn localized(kind: AppErrorKind, key: &str, fallback: impl Into<String>) -> Self {
        AppError::Localized(LocalizedError {
            kind,
            key: key.to_string(),
            args: BTreeMap::new(),
            fallback: fallback.into(),
        })
    }

    pub fn with_arg(mut self, name: &str, value: impl std::fmt::Display) -> Self {
        if let AppError::Localized(localized) = &mut self {
            localized.args.insert(name.to_string(), value.to_string());
        }
        self
    }

    pub fn kind(&self) -> AppErrorKind {
        match self {
            AppError::Io(_) => AppErrorKind::Io,
            AppError::NotFound(_) => AppErrorKind::NotFound,
            AppError::InvalidArgument(_) => AppErrorKind::InvalidArgument,
            AppError::Forbidden(_) => AppErrorKind::Forbidden,
            AppError::Internal(_) => AppErrorKind::Internal,
            AppError::Localized(localized) => localized.kind,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        match value.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound(value.to_string()),
            _ => AppError::Io(value.to_string()),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::Internal(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

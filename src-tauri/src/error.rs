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

#[cfg(test)]
mod tests {
    use super::*;

    fn localized(error: &AppError) -> &LocalizedError {
        match error {
            AppError::Localized(localized) => localized,
            other => panic!("localized 에러여야 합니다: {other:?}"),
        }
    }

    #[test]
    fn 다섯_변형은_각자의_kind_를_보고한다() {
        assert_eq!(AppError::Io("x".into()).kind(), AppErrorKind::Io);
        assert_eq!(AppError::NotFound("x".into()).kind(), AppErrorKind::NotFound);
        assert_eq!(AppError::InvalidArgument("x".into()).kind(), AppErrorKind::InvalidArgument);
        assert_eq!(AppError::Forbidden("x".into()).kind(), AppErrorKind::Forbidden);
        assert_eq!(AppError::Internal("x".into()).kind(), AppErrorKind::Internal);
    }

    #[test]
    fn localized_의_kind_는_감싼_kind_를_그대로_돌려준다() {
        for kind in [
            AppErrorKind::Io,
            AppErrorKind::NotFound,
            AppErrorKind::InvalidArgument,
            AppErrorKind::Forbidden,
            AppErrorKind::Internal,
        ] {
            assert_eq!(AppError::localized(kind, "error.some.key", "fallback").kind(), kind);
        }
    }

    #[test]
    fn localized_는_키와_폴백을_담고_인자는_비어_시작한다() {
        let error = AppError::localized(AppErrorKind::NotFound, "error.file.notFound", "file not found");
        let localized = localized(&error);

        assert_eq!(localized.key, "error.file.notFound");
        assert_eq!(localized.fallback, "file not found");
        assert!(localized.args.is_empty());
    }

    #[test]
    fn with_arg_는_인자를_이름순으로_쌓고_같은_이름은_덮어쓴다() {
        let error = AppError::localized(AppErrorKind::InvalidArgument, "error.archive.tooManyEntries", "too many")
            .with_arg("max", 5_000)
            .with_arg("count", 6_000)
            .with_arg("count", 7_000);

        let args = &localized(&error).args;
        assert_eq!(
            args.keys().collect::<Vec<_>>(),
            vec!["count", "max"],
            "BTreeMap 이므로 이름순입니다"
        );
        assert_eq!(args["count"], "7000", "같은 이름은 마지막 값이 남습니다");
        assert_eq!(args["max"], "5000");
    }

    /// `with_arg` on a non-`Localized` variant is a silent no-op — pinned because every call site
    /// chains it onto an `AppError::localized(..)` builder and a future refactor that chained it
    /// onto a plain variant would lose the argument without any compile-time signal.
    #[test]
    fn with_arg_는_localized_가_아닌_변형에서는_아무_일도_하지_않는다() {
        let error = AppError::Internal("boom".into()).with_arg("detail", "ignored");

        match error {
            AppError::Internal(message) => assert_eq!(message, "boom"),
            other => panic!("변형이 바뀌면 안 됩니다: {other:?}"),
        }
    }

    #[test]
    fn io_에러는_not_found_만_not_found_로_나머지는_io_로_변환된다() {
        let not_found = AppError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"));
        assert_eq!(not_found.kind(), AppErrorKind::NotFound);

        for kind in [
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::AlreadyExists,
            std::io::ErrorKind::InvalidData,
        ] {
            assert_eq!(AppError::from(std::io::Error::new(kind, "boom")).kind(), AppErrorKind::Io);
        }
    }

    #[test]
    fn json_파싱_실패는_internal_로_변환된다() {
        let error = AppError::from(serde_json::from_str::<serde_json::Value>("{ not json").expect_err("파싱 실패"));

        assert_eq!(error.kind(), AppErrorKind::Internal);
    }

    /// The wire shape `IpcError` consumers branch on: `code` is the variant name, `message` the
    /// payload. A rename or a change of the `tag`/`content` attributes would break every frontend
    /// `describeIpcError` path at runtime without a type error, so it is pinned here.
    #[test]
    fn 직렬화는_code_와_message_로_태깅된다() {
        let value = serde_json::to_value(AppError::Forbidden("outside root".into())).expect("직렬화");

        assert_eq!(value["code"], "Forbidden");
        assert_eq!(value["message"], "outside root");
    }

    #[test]
    fn localized_직렬화는_camel_case_필드로_중첩된다() {
        let error =
            AppError::localized(AppErrorKind::InvalidArgument, "error.archive.openFailed", "could not open").with_arg("detail", "eof");
        let value = serde_json::to_value(&error).expect("직렬화");

        assert_eq!(value["code"], "Localized");
        assert_eq!(value["message"]["kind"], "InvalidArgument");
        assert_eq!(value["message"]["key"], "error.archive.openFailed");
        assert_eq!(value["message"]["fallback"], "could not open");
        assert_eq!(value["message"]["args"]["detail"], "eof");
    }

    #[test]
    fn 표시_문자열은_변형별_접두사를_가지고_localized_는_폴백_그대로다() {
        assert_eq!(AppError::Io("disk".into()).to_string(), "io error: disk");
        assert_eq!(AppError::NotFound("a.txt".into()).to_string(), "path not found: a.txt");
        assert_eq!(AppError::InvalidArgument("bad".into()).to_string(), "invalid argument: bad");
        assert_eq!(AppError::Forbidden("nope".into()).to_string(), "forbidden: nope");
        assert_eq!(AppError::Internal("boom".into()).to_string(), "operation failed: boom");
        assert_eq!(
            AppError::localized(AppErrorKind::Internal, "error.some.key", "human readable").to_string(),
            "human readable",
            "localized 는 키가 아니라 폴백 문장을 보여줍니다"
        );
    }
}

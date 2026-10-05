use crate::error::{AppError, AppErrorKind, AppResult};

pub fn ensure_safe_component(value: &str) -> AppResult<()> {
    let is_traversal = value.is_empty() || value == "." || value == ".." || value.contains('/') || value.contains('\\');
    if is_traversal {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.path.invalidIdentifier",
            format!("invalid identifier: {value}"),
        )
        .with_arg("value", value));
    }
    Ok(())
}

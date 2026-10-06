use std::time::Instant;

use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_model::locale::ResolvedLocale;
use taide_native_editor::document::EditorError;
use taide_native_ui::toast::{Kind, Options};

pub use taide_native_ui::toast::describe_error;

use crate::explorer::{CreateRequest, RenameRequest};
use crate::presentation::message;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    RetryCreate {
        project: ProjectId,
        request: CreateRequest,
    },
    RetryRename {
        project: ProjectId,
        request: RenameRequest,
    },
}

pub type Toasts = taide_native_ui::toast::Toasts<Action>;

#[derive(Clone, Copy)]
pub enum CopyOrigin {
    Explorer,
    Terminal,
}

pub fn open_project_first(toasts: &mut Toasts, locale: &ResolvedLocale, now: Instant) {
    toasts.info(message(locale, "app.openProjectFirst", &[]), now);
}

pub fn pinned_close_blocked(
    toasts: &mut Toasts,
    locale: &ResolvedLocale,
    title: &str,
    now: Instant,
) {
    toasts.warning(
        message(locale, "tab.pinnedCloseBlocked", &[("title", title)]),
        now,
    );
}

pub fn copy_failed(toasts: &mut Toasts, locale: &ResolvedLocale, now: Instant) {
    toasts.error(message(locale, "common.copyFailed", &[]), now);
}

pub fn copy_finished(
    toasts: &mut Toasts,
    locale: &ResolvedLocale,
    origin: CopyOrigin,
    result: &AppResult<()>,
    now: Instant,
) {
    let Err(error) = result else {
        return;
    };
    match origin {
        CopyOrigin::Explorer => copy_failed(toasts, locale, now),
        CopyOrigin::Terminal => {
            log::warn!("native terminal copy failed: {:?}", error.kind())
        }
    }
}

pub fn open_link_failed(toasts: &mut Toasts, locale: &ResolvedLocale, now: Instant) {
    toasts.error(message(locale, "terminal.openLinkFailed", &[]), now);
}

pub fn error_once(toasts: &mut Toasts, locale: &ResolvedLocale, error: &AppError, now: Instant) {
    let title = describe_error(locale, error);
    if !toasts.is_showing(Kind::Error, &title) {
        toasts.error(title, now);
    }
}

pub fn save_error(error: EditorError) -> AppError {
    match error {
        EditorError::ReadOnly => AppError::localized(
            AppErrorKind::Forbidden,
            "editor.readOnlySaveBlocked",
            "native document is read-only",
        ),
        other => AppError::Internal(format!("native editor: {other:?}")),
    }
}

pub fn save_failed(
    toasts: &mut Toasts,
    locale: &ResolvedLocale,
    error: EditorError,
    auto_save: bool,
    now: Instant,
) {
    match (auto_save, error) {
        (true, EditorError::ReadOnly) => {}
        (true, error) => error_once(toasts, locale, &save_error(error), now),
        (false, error) => toasts.ipc_error(locale, &save_error(error), now),
    }
}

pub fn entry_failed(
    toasts: &mut Toasts,
    locale: &ResolvedLocale,
    title: String,
    retry: Action,
    now: Instant,
) {
    toasts.notify(
        Kind::Error,
        title,
        Options {
            description: None,
            action: Some(taide_native_ui::toast::Action {
                label: message(locale, "common.retry", &[]),
                id: retry,
            }),
        },
        now,
    );
}

#[cfg(test)]
#[path = "toast-tests.rs"]
mod tests;

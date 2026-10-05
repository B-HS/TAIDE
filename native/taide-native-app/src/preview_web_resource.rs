use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::Arc,
};

use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_runtime::{AppServices, AppState};
use url::Url;

use crate::{
    preview::invalid,
    preview_web::{Approval, approval},
    preview_web_file::{Anchor, Stamp},
    preview_web_range::{Selection, select},
};

pub const CHUNK_BYTES: usize = 64 * 1024;

struct Source {
    state: AppState,
    path: PathBuf,
    approval: Approval,
    anchor: Anchor,
    stamp: Stamp,
    closed: tokio::sync::watch::Sender<bool>,
}

#[derive(Clone)]
pub struct Scope(Arc<Source>);

pub struct Owner(Scope);

impl std::fmt::Debug for Owner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Owner")
            .field("source", &self.0.0.approval.canonical)
            .finish_non_exhaustive()
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.0.closed.send_replace(true);
    }
}

impl Owner {
    pub(crate) fn admit(
        state: AppState,
        path: PathBuf,
        approved: Approval,
    ) -> AppResult<(Self, File)> {
        let root = approved
            .root
            .clone()
            .or_else(|| approved.canonical.parent().map(Path::to_path_buf))
            .ok_or_else(|| invalid("preview source has no containing directory"))?;
        let anchor = Anchor::open(root)?;
        let file = anchor.file(&approved.canonical)?;
        let stamp = Stamp::read(&file)?;
        let owner = Self(Scope(Arc::new(Source {
            state,
            path,
            approval: approved,
            anchor,
            stamp,
            closed: tokio::sync::watch::channel(false).0,
        })));
        owner.scope().check()?;
        Ok((owner, file))
    }

    pub fn scope(&self) -> Scope {
        self.0.clone()
    }
}

impl Scope {
    pub(crate) fn canonical(&self) -> &Path {
        &self.0.approval.canonical
    }

    pub(crate) fn source_url(&self) -> AppResult<Url> {
        crate::preview_web_document::source_url(&self.0.approval.canonical)
    }

    pub fn is_closed(&self) -> bool {
        *self.0.closed.borrow() || self.0.state.is_shutting_down()
    }

    pub fn check(&self) -> AppResult<()> {
        let source = &self.0;
        if self.is_closed() {
            return Err(AppError::Forbidden(
                "preview source owner is no longer live".into(),
            ));
        }
        if approval(&source.state, &source.path)? != source.approval {
            return Err(AppError::Forbidden(
                "preview source approval changed".into(),
            ));
        }
        if Stamp::read(&source.anchor.file(&source.approval.canonical)?)? != source.stamp {
            return Err(AppError::Forbidden(
                "preview source identity or contents changed".into(),
            ));
        }
        Ok(())
    }

    pub async fn closed(&self) {
        let mut receiver = self.0.closed.subscribe();
        let _result = receiver.wait_for(|closed| *closed).await;
    }

    fn resolve(&self, path: &Path) -> AppResult<PathBuf> {
        self.check()?;
        let canonical = match &self.0.approval.root {
            Some(root) => root_guard::ensure_within_root(root, path)?,
            None => {
                let canonical = root_guard::canonicalize_lenient(path)?;
                if canonical != self.0.approval.canonical {
                    return Err(AppError::Forbidden(
                        "CLI preview does not authorize sibling resources".into(),
                    ));
                }
                canonical
            }
        };
        Ok(canonical)
    }

    pub fn open(&self, url: &Url, range: Option<&str>) -> AppResult<Response> {
        crate::preview_web_document::validate_source(url)?;
        let path = Url::parse(&format!("file://{}", url.path()))
            .map_err(|_| invalid("resource URL is invalid"))?
            .to_file_path()
            .map_err(|_| invalid("resource URL does not name a local path"))?;
        let canonical = self.resolve(&path)?;
        let mime = mime(&canonical)
            .ok_or_else(|| AppError::Forbidden("resource type is not allowed".into()))?;
        let mut file = self.0.anchor.file(&canonical)?;
        let stamp = Stamp::read(&file)?;
        let size = file.metadata()?.len();
        let selection = select(range, size);
        let body = match selection {
            Selection::Unsatisfiable => None,
            selection => {
                let (start, remaining) = match selection {
                    Selection::Partial { start, length } => (start, length),
                    _ => (0, size),
                };
                file.seek(SeekFrom::Start(start))?;
                Some(Body {
                    scope: self.clone(),
                    path,
                    canonical,
                    file,
                    stamp,
                    remaining,
                })
            }
        };
        self.check()?;
        Ok(Response {
            mime,
            size,
            selection,
            body,
        })
    }
}

pub struct Response {
    pub mime: &'static str,
    pub size: u64,
    pub selection: Selection,
    pub body: Option<Body>,
}

impl Response {
    pub fn content_length(&self) -> u64 {
        match self.selection {
            Selection::Full => self.size,
            Selection::Partial { length, .. } => length,
            Selection::Unsatisfiable => 0,
        }
    }

    pub fn content_range(&self) -> Option<String> {
        match self.selection {
            Selection::Full => None,
            Selection::Partial { start, length } => Some(format!(
                "bytes {}-{}/{}",
                start,
                start + length - 1,
                self.size
            )),
            Selection::Unsatisfiable => Some(format!("bytes */{}", self.size)),
        }
    }
}

pub struct Body {
    scope: Scope,
    path: PathBuf,
    canonical: PathBuf,
    file: File,
    stamp: Stamp,
    remaining: u64,
}

impl Body {
    fn check(&self) -> AppResult<()> {
        if self.scope.resolve(&self.path)? != self.canonical
            || Stamp::read(&self.scope.0.anchor.file(&self.canonical)?)? != self.stamp
            || Stamp::read(&self.file)? != self.stamp
        {
            return Err(AppError::Forbidden(
                "resource identity or contents changed during transfer".into(),
            ));
        }
        Ok(())
    }

    pub fn next_chunk(&mut self) -> AppResult<Option<Vec<u8>>> {
        self.check()?;
        if self.remaining == 0 {
            return Ok(None);
        }
        let length = usize::try_from(self.remaining.min(CHUNK_BYTES as u64))
            .map_err(|_| invalid("resource chunk size is invalid"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| invalid("resource chunk allocation failed"))?;
        bytes.resize(length, 0);
        self.file.read_exact(&mut bytes)?;
        self.check()?;
        self.remaining -= length as u64;
        Ok(Some(bytes))
    }
}

pub fn mime(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "css" => Some("text/css; charset=utf-8"),
        "png" | "apng" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "ico" => Some("image/vnd.microsoft.icon"),
        "svg" => Some("image/svg+xml"),
        "avif" => Some("image/avif"),
        "woff" => Some("font/woff"),
        "woff2" => Some("font/woff2"),
        "ttf" => Some("font/ttf"),
        "otf" => Some("font/otf"),
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "mp3" => Some("audio/mpeg"),
        "wav" => Some("audio/wav"),
        "flac" => Some("audio/flac"),
        "m4a" => Some("audio/mp4"),
        "ogg" | "oga" | "opus" => Some("audio/ogg"),
        "ogv" => Some("video/ogg"),
        "aac" => Some("audio/aac"),
        _ => None,
    }
}

pub async fn media_owner(services: &AppServices, path: String) -> AppResult<Owner> {
    let state = services.state.clone();
    crate::preview::read_approved(services, path.clone(), "native-media-approve", move |_| {
        let path = PathBuf::from(path);
        let approved = approval(&state, &path)?;
        if !matches!(
            crate::open_with::preview_kind(approved.canonical.to_str().unwrap_or_default()),
            Some(crate::open_with::PreviewKind::Audio | crate::open_with::PreviewKind::Video)
        ) {
            return Err(invalid(
                "media source is not a supported audio or video file",
            ));
        }
        let (owner, _file) = Owner::admit(state, path, approved)?;
        Ok(owner)
    })
    .await
}

use std::{io::Read, path::PathBuf, sync::Arc, time::Duration};

use taide_infra::root_guard;
use taide_model::{
    error::{AppError, AppResult},
    file::READ_ONLY_FILE_BYTES,
    ids::ProjectId,
};
use taide_runtime::{AppServices, AppState};
use url::Url;

use crate::{preview::read_approved, preview_web_client, preview_web_document::source_url};

const READ_CHUNK_BYTES: usize = 64 * 1024;

fn read_bounded(mut input: impl Read) -> AppResult<Vec<u8>> {
    let limit = usize::try_from(READ_ONLY_FILE_BYTES)
        .map_err(|_| crate::preview::invalid("HTML input budget is invalid"))?;
    let mut bytes = Vec::new();
    let mut chunk = [0; READ_CHUNK_BYTES];
    loop {
        let count = match input.read(&mut chunk) {
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        if count == 0 {
            return Ok(bytes);
        }
        let length = bytes
            .len()
            .checked_add(count)
            .filter(|length| *length <= limit)
            .ok_or_else(|| crate::preview::invalid("HTML preview input exceeds its budget"))?;
        if length > bytes.capacity() {
            let target = length.saturating_add(READ_CHUNK_BYTES).min(limit);
            bytes
                .try_reserve_exact(target - bytes.len())
                .map_err(|_| crate::preview::invalid("HTML preview input allocation failed"))?;
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub token: u64,
}

pub struct Prepared {
    pub canonical: PathBuf,
    pub source: Url,
    pub html: Arc<String>,
    pub owner: crate::preview_web_resource::Owner,
    pub ticket: Option<crate::preview_web_http::Ticket>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Approval {
    pub project: Option<ProjectId>,
    pub root: Option<PathBuf>,
    pub canonical: PathBuf,
}

pub(crate) fn approval(state: &AppState, path: &std::path::Path) -> AppResult<Approval> {
    let projects = state.projects.read();
    let (project, canonical) = root_guard::resolve_owning_project_or_cli_opened(
        &projects,
        &state.cli_opened_paths.read(),
        path,
    )?;
    let root = project
        .as_ref()
        .map(|id| {
            std::fs::canonicalize(root_guard::project_root(&projects, id)?).map_err(AppError::from)
        })
        .transpose()?;
    Ok(Approval {
        project,
        root,
        canonical,
    })
}

pub async fn read(
    services: &AppServices,
    request: &Request,
    executable: PathBuf,
    timeout: Duration,
    on_source_ready: impl FnOnce() + Send + 'static,
    on_started: impl FnOnce(u32) + Send + 'static,
) -> AppResult<Prepared> {
    read_internal(
        services,
        request,
        executable,
        timeout,
        None,
        on_source_ready,
        on_started,
    )
    .await
}

pub async fn read_served(
    services: &AppServices,
    request: &Request,
    executable: PathBuf,
    timeout: Duration,
    server: Arc<crate::preview_web_http::Server>,
    on_source_ready: impl FnOnce() + Send + 'static,
    on_started: impl FnOnce(u32) + Send + 'static,
) -> AppResult<Prepared> {
    read_internal(
        services,
        request,
        executable,
        timeout,
        Some(server),
        on_source_ready,
        on_started,
    )
    .await
}

pub(crate) async fn read_internal(
    services: &AppServices,
    request: &Request,
    executable: PathBuf,
    timeout: Duration,
    server: Option<Arc<crate::preview_web_http::Server>>,
    on_source_ready: impl FnOnce() + Send + 'static,
    on_started: impl FnOnce(u32) + Send + 'static,
) -> AppResult<Prepared> {
    let state = services.state.clone();
    let path = request.path.clone();
    let (approved, owner, bytes) = read_approved(
        services,
        path.clone(),
        "native-html-read",
        move |canonical| {
            let approved = approval(&state, std::path::Path::new(&path))?;
            if approved.canonical != canonical {
                return Err(AppError::Forbidden(
                    "HTML preview source changed before read".into(),
                ));
            }
            let (owner, mut file) = crate::preview_web_resource::Owner::admit(
                state,
                PathBuf::from(path),
                approved.clone(),
            )?;
            let metadata = file.metadata()?;
            if !metadata.is_file() || metadata.len() > READ_ONLY_FILE_BYTES {
                return Err(crate::preview::invalid(
                    "HTML preview requires a bounded regular file",
                ));
            }
            let bytes = read_bounded(&mut file)?;
            owner.scope().check()?;
            Ok((approved, owner, bytes))
        },
    )
    .await?;
    on_source_ready();
    let source = source_url(&approved.canonical)?;
    let ticket = server
        .as_ref()
        .map(|server| server.register(owner.scope()))
        .transpose()?;
    let source = match &ticket {
        Some(ticket) => ticket.url(&source)?,
        None => source,
    };
    let html = preview_web_client::prepare(
        &services.tasks,
        executable,
        preview_web_client::Request {
            source: source.clone(),
            bytes,
            timeout,
        },
        on_started,
    )
    .await?;
    let html = Arc::new(html);
    let state = services.state.clone();
    let path = request.path.clone();
    read_approved(
        services,
        path.clone(),
        "native-html-reapprove",
        move |canonical| {
            if approval(&state, std::path::Path::new(&path))? != approved
                || canonical != approved.canonical
            {
                return Err(AppError::Forbidden(
                    "HTML preview approval changed while preparing".into(),
                ));
            }
            owner.scope().check()?;
            if let Some(ticket) = &ticket {
                ticket.publish(html.clone())?;
            }
            Ok(Prepared {
                canonical: approved.canonical,
                source,
                html,
                owner,
                ticket,
            })
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_bounded_read는_metadata와_독립적으로_실제_입력을_제한한다() {
        assert_eq!(
            read_bounded(&b"synthetic HTML"[..]).unwrap(),
            b"synthetic HTML"
        );
        assert!(read_bounded(&b""[..]).unwrap().is_empty());
        let mut input = std::io::repeat(b'x').take(READ_ONLY_FILE_BYTES + 1);
        assert!(read_bounded(&mut input).is_err());
        assert_eq!(input.limit(), 0);
        let input = std::io::repeat(b'x').take(READ_ONLY_FILE_BYTES);
        let bytes = read_bounded(input).unwrap();
        assert_eq!(bytes.len() as u64, READ_ONLY_FILE_BYTES);
        assert!(bytes.capacity() as u64 <= READ_ONLY_FILE_BYTES);
    }
}

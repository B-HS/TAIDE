use std::{path::PathBuf, process::Stdio, time::Duration};

use taide_model::{
    error::{AppError, AppResult},
    file::READ_ONLY_FILE_BYTES,
};
use taide_runtime::TaskSupervisor;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    process::Command,
    sync::oneshot,
};
use url::Url;

use crate::{
    preview::invalid,
    preview_web_document::{MAX_OUTPUT_BYTES, validate_document_source},
    preview_web_helper::{FLAG, MAX_SOURCE_BYTES, REPLY_MAGIC, REQUEST_MAGIC},
};

const OPERATION: &str = "native-html-helper";

pub struct Request {
    pub source: Url,
    pub bytes: Vec<u8>,
    pub timeout: Duration,
}

struct CancelOnDrop(Option<oneshot::Sender<()>>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _result = sender.send(());
        }
    }
}

pub async fn prepare(
    tasks: &TaskSupervisor,
    executable: PathBuf,
    request: Request,
    on_started: impl FnOnce(u32) + Send + 'static,
) -> AppResult<String> {
    if !executable.is_absolute() {
        return Err(invalid(
            "HTML helper executable must be an absolute trusted path",
        ));
    }
    validate_document_source(&request.source)?;
    let source_length = u32::try_from(request.source.as_str().len())
        .map_err(|_| invalid("HTML helper source length is invalid"))?;
    let document_length = u32::try_from(request.bytes.len())
        .map_err(|_| invalid("HTML helper document length is invalid"))?;
    if source_length > MAX_SOURCE_BYTES || u64::from(document_length) > READ_ONLY_FILE_BYTES {
        return Err(invalid("HTML helper input exceeds its budget"));
    }
    let (cancel, mut cancelled) = oneshot::channel();
    let _cancel = CancelOnDrop(Some(cancel));
    tasks.run_nonabortable_result(OPERATION, async move {
        if !matches!(cancelled.try_recv(), Err(oneshot::error::TryRecvError::Empty)) {
            return Err(AppError::Forbidden("HTML helper request was cancelled before spawn".into()));
        }
        let mut child = Command::new(executable)
            .arg(FLAG).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .kill_on_drop(true).spawn()
            .map_err(|_| AppError::Internal("HTML helper process could not start".into()))?;
        let result = async {
            let input = child.stdin.take().ok_or_else(|| invalid("HTML helper input pipe is unavailable"))?;
            let output = child.stdout.take().ok_or_else(|| invalid("HTML helper output pipe is unavailable"))?;
            let pid = child.id().ok_or_else(|| invalid("HTML helper process has no live identity"))?;
            on_started(pid);
            tokio::select! {
                biased;
                _ = cancelled => Err(AppError::Forbidden("HTML helper request was cancelled".into())),
                result = tokio::time::timeout(request.timeout, async {
                    let (_, document) = tokio::try_join!(
                        write_request(input, &request.source, &request.bytes, source_length, document_length),
                        read_reply(output),
                    )?;
                    let status = child.wait().await.map_err(|_| invalid("HTML helper exit could not be collected"))?;
                    if !status.success() {
                        return Err(invalid("HTML helper exited unsuccessfully"));
                    }
                    Ok(document)
                }) => result.unwrap_or_else(|_| Err(invalid("HTML helper exceeded its deadline"))),
            }
        }.await;
        if result.is_err() {
            let _kill = child.start_kill();
            child.wait().await.map_err(|_| AppError::Internal("HTML helper could not be reaped after failure".into()))?;
        }
        result
    }).await
}

async fn write_request(
    mut input: impl AsyncWrite + Unpin,
    source: &Url,
    bytes: &[u8],
    source_length: u32,
    document_length: u32,
) -> AppResult<()> {
    input
        .write_all(&REQUEST_MAGIC)
        .await
        .map_err(|_| invalid("HTML helper request failed"))?;
    input
        .write_all(&source_length.to_le_bytes())
        .await
        .map_err(|_| invalid("HTML helper source header failed"))?;
    input
        .write_all(&document_length.to_le_bytes())
        .await
        .map_err(|_| invalid("HTML helper document header failed"))?;
    input
        .write_all(source.as_str().as_bytes())
        .await
        .map_err(|_| invalid("HTML helper source write failed"))?;
    input
        .write_all(bytes)
        .await
        .map_err(|_| invalid("HTML helper document write failed"))?;
    input
        .shutdown()
        .await
        .map_err(|_| invalid("HTML helper input shutdown failed"))
}

async fn read_reply(mut output: impl AsyncRead + Unpin) -> AppResult<String> {
    let mut magic = [0; REPLY_MAGIC.len()];
    output
        .read_exact(&mut magic)
        .await
        .map_err(|_| invalid("HTML helper reply is truncated"))?;
    if magic != REPLY_MAGIC {
        return Err(invalid("HTML helper reply version is invalid"));
    }
    let mut length = [0; size_of::<u32>()];
    output
        .read_exact(&mut length)
        .await
        .map_err(|_| invalid("HTML helper reply length is truncated"))?;
    let length = usize::try_from(u32::from_le_bytes(length))
        .map_err(|_| invalid("HTML helper reply length is invalid"))?;
    if length > MAX_OUTPUT_BYTES {
        return Err(invalid("HTML helper reply exceeds its budget"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| invalid("HTML helper reply allocation failed"))?;
    bytes.resize(length, 0);
    output
        .read_exact(&mut bytes)
        .await
        .map_err(|_| invalid("HTML helper reply payload is truncated"))?;
    let mut trailing = [0; 1];
    if output
        .read(&mut trailing)
        .await
        .map_err(|_| invalid("HTML helper reply closure failed"))?
        != 0
    {
        return Err(invalid("HTML helper reply contains trailing data"));
    }
    String::from_utf8(bytes).map_err(|_| invalid("HTML helper reply is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn html_reply는_잘못된_version_length_utf8_trailer와_잘린_frame을_거절한다() {
        let mut oversized = REPLY_MAGIC.to_vec();
        oversized.extend_from_slice(&(u32::try_from(MAX_OUTPUT_BYTES).unwrap() + 1).to_le_bytes());
        let mut invalid_utf8 = REPLY_MAGIC.to_vec();
        invalid_utf8.extend_from_slice(&1u32.to_le_bytes());
        invalid_utf8.push(u8::MAX);
        let mut trailing = REPLY_MAGIC.to_vec();
        trailing.extend_from_slice(&1u32.to_le_bytes());
        trailing.extend_from_slice(b"ab");
        let mut truncated = REPLY_MAGIC.to_vec();
        truncated.extend_from_slice(&1u32.to_le_bytes());
        for bytes in [
            Vec::new(),
            b"BAD1".to_vec(),
            oversized,
            invalid_utf8,
            trailing,
            truncated,
        ] {
            assert!(read_reply(bytes.as_slice()).await.is_err());
        }
    }
}

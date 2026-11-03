use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_runtime::AppServices;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

/// How long a second launch waits for the running instance to confirm the relay.
pub const RELAY_ACK_TIMEOUT: Duration = Duration::from_secs(5);
/// How long the running instance waits for a relayed argument line.
pub const RELAY_READ_TIMEOUT: Duration = Duration::from_secs(2);
/// Largest relayed argument line, newline included.
pub const MAX_RELAY_BYTES: usize = 64 * 1024;
const SOCKET_MODE: u32 = 0o600;
const RELAY_ACK: u8 = 1;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

/// The per-data-directory relay socket, kept under the OS temporary directory because a data
/// directory path can exceed the `sun_path` limit.
pub fn socket_path(data_dir: &Path) -> PathBuf {
    std::env::temp_dir().join(format!(
        "taide-native-{:016x}.sock",
        fnv1a(data_dir.as_os_str().as_bytes())
    ))
}

/// Removes the relay socket file when the first instance ends.
#[derive(Debug)]
pub struct SocketFile(PathBuf);

impl Drop for SocketFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The outcome of claiming a data directory, as `tauri-plugin-single-instance` decides it.
#[derive(Debug)]
pub enum Claim {
    /// This launch owns the data directory and serves later launches.
    Primary(UnixListener, SocketFile),
    /// A running instance accepted this launch's arguments; this launch ends without a window.
    Relayed,
    /// Neither relay nor claim worked; this launch runs alone.
    Unavailable,
}

/// Relays `arguments` to the instance that owns `path`, or claims it when nothing answers there.
pub fn acquire(path: &Path, arguments: &[String]) -> Claim {
    match UnixStream::connect(path) {
        Ok(stream) => match relay(stream, arguments) {
            Ok(()) => return Claim::Relayed,
            Err(error) => {
                log::warn!("native single-instance relay failed: {}", error.kind());
                return Claim::Unavailable;
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            let is_socket = std::fs::symlink_metadata(path)
                .is_ok_and(|metadata| metadata.file_type().is_socket());
            if is_socket && let Err(error) = std::fs::remove_file(path) {
                log::warn!(
                    "native single-instance stale socket stays: {}",
                    error.kind()
                );
            }
        }
        Err(_) => {}
    }
    match UnixListener::bind(path) {
        Ok(listener) => {
            let file = SocketFile(path.to_path_buf());
            if let Err(error) =
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(SOCKET_MODE))
            {
                log::warn!(
                    "native single-instance socket mode failed: {}",
                    error.kind()
                );
            }
            Claim::Primary(listener, file)
        }
        Err(error) => {
            log::warn!("native single-instance claim failed: {}", error.kind());
            Claim::Unavailable
        }
    }
}

fn relay(mut stream: UnixStream, arguments: &[String]) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(arguments)?;
    line.push(b'\n');
    stream.write_all(&line)?;
    stream.set_read_timeout(Some(RELAY_ACK_TIMEOUT))?;
    let mut ack = [0; 1];
    stream.read_exact(&mut ack)?;
    if ack[0] != RELAY_ACK {
        return Err(std::io::Error::other("unexpected relay acknowledgement"));
    }
    Ok(())
}

/// Serves later launches until the app stops: each relayed request is queued like a first-launch
/// request and announced, and every relay asks for the main window to come forward.
pub(crate) fn serve(
    services: Arc<AppServices>,
    inbox: Arc<crate::provider_events::Inbox>,
    listener: UnixListener,
) {
    if let Err(error) = listener.set_nonblocking(true) {
        log::warn!("native single-instance listener failed: {}", error.kind());
        return;
    }
    let tasks = services.tasks.clone();
    tasks.spawn_transient("native-single-instance", async move {
        let listener = match tokio::net::UnixListener::from_std(listener) {
            Ok(listener) => listener,
            Err(error) => {
                log::warn!("native single-instance listener failed: {}", error.kind());
                return;
            }
        };
        while !services.state.is_shutting_down() {
            match listener.accept().await {
                Ok((stream, _)) => receive(&services, &inbox, stream).await,
                Err(error) => {
                    log::warn!("native single-instance accept failed: {}", error.kind());
                    return;
                }
            }
        }
    });
}

async fn receive(
    services: &AppServices,
    inbox: &crate::provider_events::Inbox,
    stream: tokio::net::UnixStream,
) {
    let (reader, mut writer) = stream.into_split();
    let mut line = Vec::new();
    let mut reader = tokio::io::BufReader::new(reader.take(MAX_RELAY_BYTES as u64));
    let read = tokio::time::timeout(RELAY_READ_TIMEOUT, reader.read_until(b'\n', &mut line)).await;
    if !matches!(read, Ok(Ok(_))) || line.last() != Some(&b'\n') {
        return;
    }
    let Ok(arguments) = serde_json::from_slice::<Vec<String>>(&line) else {
        return;
    };
    if let Some(request) = crate::external_open::request_from(&arguments) {
        crate::external_open::queue(services, request.clone());
        services
            .events
            .publish(AppEvent::AgentExternalOpen { request });
    }
    inbox.request_main_window();
    if let Err(error) = writer.write_all(&[RELAY_ACK]).await {
        log::warn!("native single-instance ack failed: {}", error.kind());
    }
}

#[cfg(test)]
#[path = "single-instance-tests.rs"]
mod tests;

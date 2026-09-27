use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::http::{outbound_http_client, HttpClientProfile};
use taide_infra::lsp_install;
use taide_lsp::install::{install_cancelled_error, LspInstallLease};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::{LanguageServerSpec, LspArchiveKind, LspInstallPhase, LspServerId};
use taide_model::paths::AppPaths;

use crate::{EventSink, TaskSupervisor};

struct DownloadArtifacts {
    download_path: PathBuf,
    extract_dir: PathBuf,
}

impl Drop for DownloadArtifacts {
    fn drop(&mut self) {
        std::fs::remove_file(&self.download_path).ok();
        std::fs::remove_dir_all(&self.extract_dir).ok();
    }
}

struct OwnedDownloadFile {
    file: Mutex<std::fs::File>,
    _artifacts: Arc<DownloadArtifacts>,
    _lease: LspInstallLease,
}

struct OwnedDownloadFileIo {
    tasks: TaskSupervisor,
    lease: LspInstallLease,
    artifacts: Arc<DownloadArtifacts>,
}

struct InstallBlockingWork<F> {
    work: F,
    lease: LspInstallLease,
}

impl lsp_install::DownloadFileIo for OwnedDownloadFileIo {
    type File = Arc<OwnedDownloadFile>;

    async fn create(&self, path: &Path) -> AppResult<Self::File> {
        let path = path.to_path_buf();
        let artifacts = self.artifacts.clone();
        let file_lease = self.lease.clone();
        run_install_blocking_step(&self.tasks, &self.lease, move || {
            let file = std::fs::File::create(path)?;
            Ok(Arc::new(OwnedDownloadFile {
                file: Mutex::new(file),
                _artifacts: artifacts,
                _lease: file_lease,
            }))
        })
        .await
    }

    async fn write_all(&self, file: &mut Self::File, bytes: &[u8]) -> AppResult<()> {
        let file = file.clone();
        let bytes = bytes.to_vec();
        run_install_blocking_step(&self.tasks, &self.lease, move || {
            file.file.lock().write_all(&bytes).map_err(AppError::from)
        })
        .await
    }

    async fn flush(&self, file: &mut Self::File) -> AppResult<()> {
        let file = file.clone();
        run_install_blocking_step(&self.tasks, &self.lease, move || file.file.lock().flush().map_err(AppError::from)).await
    }
}

fn emit_install_progress(
    events: &dyn EventSink,
    server_id: &LspServerId,
    phase: LspInstallPhase,
    received_bytes: u64,
    total_bytes: Option<u64>,
    message: Option<String>,
) {
    events.publish(AppEvent::LspInstallProgress {
        server_id: server_id.clone(),
        phase,
        received_bytes: received_bytes as f64,
        total_bytes: total_bytes.map(|value| value as f64),
        message,
    });
}

async fn run_install_blocking_step<T: Send + 'static>(
    tasks: &TaskSupervisor,
    lease: &LspInstallLease,
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    lease.ensure_active()?;
    let owner = InstallBlockingWork {
        work,
        lease: lease.clone(),
    };
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = tasks
        .spawn_blocking_transient_handle("lsp-install-work", move || {
            let owner = owner;
            let result = owner.lease.ensure_active().and_then(|()| (owner.work)());
            let _ = sender.send(result);
        })
        .ok_or_else(install_cancelled_error)?;
    handle.await.map_err(|join_error| {
        if join_error.is_cancelled() {
            return install_cancelled_error();
        }
        AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.extractTaskFailed",
            format!("extraction task failed: {join_error}"),
        )
        .with_arg("detail", &join_error)
    })?;
    let result = receiver.await.map_err(|_| install_cancelled_error())??;
    lease.ensure_active()?;
    Ok(result)
}

/// Downloads and verifies an archive, retaining worker ownership until extraction finishes.
pub async fn run_download_install(
    events: &dyn EventSink,
    paths: &AppPaths,
    spec: &LanguageServerSpec,
    lease: &LspInstallLease,
    tasks: &TaskSupervisor,
) -> AppResult<()> {
    lease.ensure_active()?;
    let download = spec.install.download.as_ref().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.downloadInfoMissing",
            format!("{}: download info is not configured yet", spec.id),
        )
        .with_arg("serverId", &spec.id)
    })?;
    let platform = lsp_install::platform_key();
    let url = download.urls.get(&platform).ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.platformUnsupported",
            format!("{}: the current platform ({platform}) is not supported", spec.id),
        )
        .with_arg("serverId", &spec.id)
        .with_arg("platform", &platform)
    })?;
    let expected_sha256 = download.sha256.get(&platform).cloned().flatten().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.checksumUnpublished",
            format!("{}: cannot install because no checksum has been published yet", spec.id),
        )
        .with_arg("serverId", &spec.id)
    })?;

    let artifacts = Arc::new(DownloadArtifacts {
        download_path: lsp_install::temp_download_path(&paths.lsp_dir(), spec.id.as_str()),
        extract_dir: lsp_install::temp_install_dir(&paths.lsp_dir(), spec.id.as_str()),
    });
    emit_install_progress(events, &spec.id, LspInstallPhase::Downloading, 0, None, None);
    let client = outbound_http_client(HttpClientProfile::Download);
    let cancel = lease.cancellation_token();
    let file_io = OwnedDownloadFileIo {
        tasks: tasks.clone(),
        lease: lease.clone(),
        artifacts: artifacts.clone(),
    };
    let download_result = tokio::select! {
        biased;
        _ = lease.cancelled() => Err(install_cancelled_error()),
        result = lsp_install::download_to_file_with_io(&client, url, &artifacts.download_path, &cancel, &file_io, |update| {
            emit_install_progress(events, &spec.id, LspInstallPhase::Downloading, update.received_bytes, update.total_bytes, None);
        }) => result,
    };
    let downloaded = match download_result {
        Ok(downloaded) => downloaded,
        Err(error) => {
            emit_install_progress(events, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
            return Err(error);
        }
    };
    emit_install_progress(
        events,
        &spec.id,
        LspInstallPhase::Verifying,
        downloaded.total_bytes,
        Some(downloaded.total_bytes),
        None,
    );
    if !lsp_install::hashes_match(&downloaded.sha256, &expected_sha256) {
        emit_install_progress(
            events,
            &spec.id,
            LspInstallPhase::Failed,
            0,
            None,
            Some("체크섬이 일치하지 않습니다".to_string()),
        );
        return Err(AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.checksumMismatch",
            format!("{}: the downloaded file's checksum does not match", spec.id),
        )
        .with_arg("serverId", &spec.id));
    }

    emit_install_progress(
        events,
        &spec.id,
        LspInstallPhase::Extracting,
        downloaded.total_bytes,
        Some(downloaded.total_bytes),
        None,
    );
    let archive_kind = download.archive;
    let bin_path_in_archive = download.bin_path_in_archive.clone();
    let worker_artifacts = artifacts.clone();
    let extract_result = run_install_blocking_step(tasks, lease, move || {
        let source = &worker_artifacts.download_path;
        let target = &worker_artifacts.extract_dir;
        match archive_kind {
            LspArchiveKind::TarGz => lsp_install::extract_tar_gz(source, target),
            LspArchiveKind::TarXz => lsp_install::extract_tar_xz(source, target),
            LspArchiveKind::Zip => lsp_install::extract_zip(source, target),
            LspArchiveKind::Binary => lsp_install::write_binary_from_file(source, target, bin_path_in_archive.as_deref()),
            LspArchiveKind::Gz => lsp_install::write_gz_binary_from_file(source, target, bin_path_in_archive.as_deref()),
        }
    })
    .await;
    if let Err(error) = extract_result {
        emit_install_progress(events, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
        return Err(error);
    }

    let final_dir = paths.lsp_server_version_dir(spec.id.as_str(), &download.version);
    if let Err(error) = lease.commit(|| lsp_install::atomic_install(&artifacts.extract_dir, &final_dir)) {
        emit_install_progress(events, &spec.id, LspInstallPhase::Failed, 0, None, Some(error.to_string()));
        return Err(error);
    }
    emit_install_progress(
        events,
        &spec.id,
        LspInstallPhase::Done,
        downloaded.total_bytes,
        Some(downloaded.total_bytes),
        None,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::future::Future;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};

    use parking_lot::Mutex;
    use taide_infra::lsp_install::DownloadFileIo;
    use taide_lsp::install::LspInstallStore;
    use taide_model::lsp::{LspDownloadInstall, LspInstallStrategy};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::oneshot;

    use super::*;
    use crate::TaskSupervisor;

    const FIXTURE_TIMEOUT_MS: u64 = 1_000;
    const HTTP_READ_BUFFER_BYTES: usize = 1_024;
    const BINARY_CONTENT: &[u8] = b"synthetic language server";

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("taide-install-lifecycle-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    struct HttpFixture {
        url: String,
        requested: Option<oneshot::Receiver<()>>,
        task: Option<tokio::task::JoinHandle<()>>,
    }

    impl HttpFixture {
        async fn start(response: Option<String>) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/fixture", listener.local_addr().unwrap());
            let (requested, requested_rx) = oneshot::channel();
            let task = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; HTTP_READ_BUFFER_BYTES];
                while !request.windows(b"\r\n\r\n".len()).any(|window| window == b"\r\n\r\n") {
                    let count = socket.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                if let Some(response) = response {
                    socket.write_all(response.as_bytes()).await.unwrap();
                }
                let _ = requested.send(());
                std::future::pending::<()>().await;
                drop(socket);
            });
            Self {
                url,
                requested: Some(requested_rx),
                task: Some(task),
            }
        }

        async fn wait_requested(&mut self) {
            tokio::time::timeout(std::time::Duration::from_millis(FIXTURE_TIMEOUT_MS), self.requested.take().unwrap())
                .await
                .unwrap()
                .unwrap();
        }

        async fn finish(mut self) {
            let task = self.task.take().unwrap();
            task.abort();
            let result = task.await;
            assert!(result.is_ok() || result.unwrap_err().is_cancelled());
        }
    }

    impl Drop for HttpFixture {
        fn drop(&mut self) {
            if let Some(task) = &self.task {
                task.abort();
            }
        }
    }

    #[derive(Default)]
    struct RecordingEvents(Mutex<Vec<AppEvent>>);

    impl EventSink for RecordingEvents {
        fn publish(&self, event: AppEvent) {
            self.0.lock().push(event);
        }
    }

    fn fixture_spec(url: String, checksum: String) -> LanguageServerSpec {
        let mut spec = taide_lsp::manifest::find_spec("rustAnalyzer").unwrap();
        spec.id = LspServerId::from("synthetic-server");
        spec.install.strategy = LspInstallStrategy::Download;
        let platform = lsp_install::platform_key();
        spec.install.download = Some(LspDownloadInstall {
            version: "fixture-version".to_string(),
            urls: BTreeMap::from([(platform.clone(), url)]),
            sha256: BTreeMap::from([(platform, Some(checksum))]),
            archive: LspArchiveKind::Binary,
            bin_path_in_archive: Some("server".to_string()),
        });
        spec
    }

    fn has_phase(events: &RecordingEvents, expected: LspInstallPhase) -> bool {
        events
            .0
            .lock()
            .iter()
            .any(|event| matches!(event, AppEvent::LspInstallProgress { phase, .. } if *phase == expected))
    }

    fn binary_response() -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            BINARY_CONTENT.len(),
            std::str::from_utf8(BINARY_CONTENT).unwrap(),
        )
    }

    #[test]
    fn 시작_전_취소한_work의_cleanup도_마지막_lease_해제보다_앞선다() {
        struct CleanupWitness {
            store: LspInstallStore,
            server_id: LspServerId,
            did_cleanup: Arc<AtomicBool>,
        }

        impl Drop for CleanupWitness {
            fn drop(&mut self) {
                assert!(self.store.begin(&self.server_id).is_none());
                self.did_cleanup.store(true, Ordering::SeqCst);
            }
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let store = LspInstallStore::new();
            let server_id = LspServerId::from("test-server");
            let guard = store.begin(&server_id).unwrap();
            let lease = guard.lease();
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let did_cleanup = Arc::new(AtomicBool::new(false));
            let witness = CleanupWitness {
                store: store.clone(),
                server_id: server_id.clone(),
                did_cleanup: did_cleanup.clone(),
            };
            let (started, started_rx) = oneshot::channel();
            let (release, release_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                started.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            started_rx.await.unwrap();
            let mut operation = Box::pin(run_install_blocking_step::<()>(&tasks, &lease, move || {
                let _witness = witness;
                panic!("cancelled queued body must not run");
            }));
            let mut context = Context::from_waker(Waker::noop());
            assert!(matches!(operation.as_mut().poll(&mut context), Poll::Pending));
            drop(operation);
            drop(lease);
            drop(guard);
            assert!(store.begin(&server_id).is_none());
            tasks.stop_all();
            release.send(()).unwrap();
            blocker.await.unwrap();
            tokio::task::spawn_blocking(|| ()).await.unwrap();
            assert!(did_cleanup.load(Ordering::SeqCst));
            assert_eq!(tasks.tracked_count(), 0);
            assert!(store.begin(&server_id).is_some());
        });
    }

    #[test]
    fn write와_flush_대기_중_drop도_실제_파일_작업의_소유권을_유지한다() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for should_flush in [false, true] {
                let root = TestRoot::new();
                let download_path = root.0.join("pending-file-work.download");
                let store = LspInstallStore::new();
                let server_id = LspServerId::from("test-server");
                let guard = store.begin(&server_id).unwrap();
                let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
                let file_io = OwnedDownloadFileIo {
                    tasks: tasks.clone(),
                    lease: guard.lease(),
                    artifacts: Arc::new(DownloadArtifacts {
                        download_path: download_path.clone(),
                        extract_dir: root.0.join("pending-extract"),
                    }),
                };
                let mut file = file_io.create(&download_path).await.unwrap();
                let (started, started_rx) = oneshot::channel();
                let (release, release_rx) = std::sync::mpsc::channel();
                let blocker = tokio::task::spawn_blocking(move || {
                    started.send(()).unwrap();
                    release_rx.recv().unwrap();
                });
                started_rx.await.unwrap();
                let mut operation = Box::pin(async {
                    if should_flush {
                        return file_io.flush(&mut file).await;
                    }
                    file_io.write_all(&mut file, BINARY_CONTENT).await
                });
                let mut context = Context::from_waker(Waker::noop());
                assert!(matches!(operation.as_mut().poll(&mut context), Poll::Pending));
                assert_eq!(tasks.tracked_count(), 1);
                drop(operation);
                drop(file_io);
                drop(file);
                drop(guard);

                assert!(store.begin(&server_id).is_none());
                assert!(download_path.exists());
                release.send(()).unwrap();
                blocker.await.unwrap();
                tokio::task::spawn_blocking(|| ()).await.unwrap();
                assert_eq!(tasks.tracked_count(), 0);
                assert!(!download_path.exists());
                assert!(store.begin(&server_id).is_some());
            }
        });
    }

    #[tokio::test]
    async fn 열린_파일_소유자는_요청이_끝나도_닫힘과_cleanup까지_슬롯을_보유한다() {
        let root = TestRoot::new();
        let download_path = root.0.join("owned-file.download");
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let file_io = OwnedDownloadFileIo {
            tasks: tasks.clone(),
            lease: guard.lease(),
            artifacts: Arc::new(DownloadArtifacts {
                download_path: download_path.clone(),
                extract_dir: root.0.join("owned-extract"),
            }),
        };
        let file = file_io.create(&download_path).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
        drop(file_io);
        drop(guard);
        assert!(store.begin(&server_id).is_none());
        assert!(download_path.exists());
        drop(file);
        assert!(!download_path.exists());
        assert!(store.begin(&server_id).is_some());
    }

    #[tokio::test]
    async fn 파일_생성_오류는_io_오류로_반환하고_감독_작업을_정리한다() {
        let root = TestRoot::new();
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let file_io = OwnedDownloadFileIo {
            tasks: tasks.clone(),
            lease: guard.lease(),
            artifacts: Arc::new(DownloadArtifacts {
                download_path: root.0.join("not-created.download"),
                extract_dir: root.0.join("not-created-extract"),
            }),
        };
        let result = file_io.create(&root.0).await;
        assert!(matches!(result, Err(AppError::Io(_))));
        assert_eq!(tasks.tracked_count(), 0);
        drop(file_io);
        drop(guard);
        assert!(store.begin(&server_id).is_some());
    }

    #[test]
    fn 파일_생성_대기_중_요청_drop은_슬롯을_유지하고_늦은_임시_파일을_남기지_않는다() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let root = TestRoot::new();
            let paths = AppPaths::new(root.0.clone());
            let server = HttpFixture::start(Some(binary_response())).await;
            let spec = fixture_spec(server.url.clone(), lsp_install::sha256_hex(BINARY_CONTENT));
            let store = LspInstallStore::new();
            let guard = store.begin(&spec.id).unwrap();
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let _client = outbound_http_client(HttpClientProfile::Download);
            let (started, started_rx) = oneshot::channel();
            let (release, release_rx) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                started.send(()).unwrap();
                release_rx.recv().unwrap();
            });
            started_rx.await.unwrap();
            let request_paths = AppPaths::new(root.0.clone());
            let request_spec = spec.clone();
            let installer = tokio::spawn(async move {
                run_download_install(&RecordingEvents::default(), &request_paths, &request_spec, &guard.lease(), &tasks).await
            });
            let temp = paths.lsp_dir().join(".tmp");
            tokio::time::timeout(std::time::Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
                while !temp.exists() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert_eq!(std::fs::read_dir(&temp).unwrap().count(), 0);
            installer.abort();
            assert!(installer.await.unwrap_err().is_cancelled());
            let was_slot_reusable_before_file_work_finished = store.begin(&spec.id).is_some();
            release.send(()).unwrap();
            blocker.await.unwrap();
            tokio::task::spawn_blocking(|| ()).await.unwrap();
            let late_files = std::fs::read_dir(&temp).unwrap().count();
            server.finish().await;

            assert!(
                !was_slot_reusable_before_file_work_finished,
                "slot reopened before queued file I/O finished; late temporary files: {late_files}",
            );
            assert_eq!(late_files, 0);
            assert!(store.begin(&spec.id).is_some());
        });
    }

    #[tokio::test]
    async fn 설치_설정과_플랫폼_checksum이_없으면_네트워크와_worker를_시작하지_않는다() {
        let root = TestRoot::new();
        let paths = AppPaths::new(root.0.clone());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let events = RecordingEvents::default();
        let store = LspInstallStore::new();
        let original = fixture_spec(
            "http://127.0.0.1:1/not-contacted".to_string(),
            lsp_install::sha256_hex(BINARY_CONTENT),
        );
        let mut no_info = original.clone();
        no_info.install.download = None;
        let mut no_platform = original.clone();
        no_platform.install.download.as_mut().unwrap().urls.clear();
        let mut no_checksum = original;
        no_checksum.install.download.as_mut().unwrap().sha256.clear();

        for (spec, expected) in [
            (no_info, "error.lsp.downloadInfoMissing"),
            (no_platform, "error.lsp.platformUnsupported"),
            (no_checksum, "error.lsp.checksumUnpublished"),
        ] {
            let guard = store.begin(&spec.id).unwrap();
            let error = run_download_install(&events, &paths, &spec, &guard.lease(), &tasks)
                .await
                .unwrap_err();
            let AppError::Localized(localized) = error else {
                panic!("localized download validation error");
            };
            assert_eq!(localized.key, expected);
            assert!(events.0.lock().is_empty());
            assert!(!paths.lsp_dir().exists());
            assert_eq!(tasks.tracked_count(), 0);
        }
    }

    #[tokio::test]
    async fn 추출_직전_취소는_기존_binary를_보존하고_done을_발행하지_않는다() {
        struct CancelAtExtraction {
            events: RecordingEvents,
            store: LspInstallStore,
            server_id: LspServerId,
        }

        impl EventSink for CancelAtExtraction {
            fn publish(&self, event: AppEvent) {
                if matches!(
                    &event,
                    AppEvent::LspInstallProgress {
                        phase: LspInstallPhase::Extracting,
                        ..
                    }
                ) {
                    self.store.cancel(&self.server_id);
                }
                self.events.publish(event);
            }
        }

        let root = TestRoot::new();
        let server = HttpFixture::start(Some(binary_response())).await;
        let spec = fixture_spec(server.url.clone(), lsp_install::sha256_hex(BINARY_CONTENT));
        let paths = AppPaths::new(root.0.clone());
        let final_dir = paths.lsp_server_version_dir(spec.id.as_str(), "fixture-version");
        std::fs::create_dir_all(&final_dir).unwrap();
        std::fs::write(final_dir.join("server"), b"previous server").unwrap();
        let store = LspInstallStore::new();
        let guard = store.begin(&spec.id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let events = CancelAtExtraction {
            events: RecordingEvents::default(),
            store: store.clone(),
            server_id: spec.id.clone(),
        };

        assert!(run_download_install(&events, &paths, &spec, &guard.lease(), &tasks).await.is_err());
        assert_eq!(std::fs::read(final_dir.join("server")).unwrap(), b"previous server");
        assert!(has_phase(&events.events, LspInstallPhase::Failed));
        assert!(!has_phase(&events.events, LspInstallPhase::Done));
        assert_eq!(tasks.tracked_count(), 0);
        assert_eq!(std::fs::read_dir(paths.lsp_dir().join(".tmp")).unwrap().count(), 0);
        server.finish().await;
    }

    #[tokio::test]
    async fn 검증된_binary는_원자_적용_뒤_done을_발행하고_임시_파일을_정리한다() {
        let root = TestRoot::new();
        let server = HttpFixture::start(Some(binary_response())).await;
        let spec = fixture_spec(server.url.clone(), lsp_install::sha256_hex(BINARY_CONTENT));
        let paths = AppPaths::new(root.0.clone());
        let events = RecordingEvents::default();
        let store = LspInstallStore::new();
        let guard = store.begin(&spec.id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());

        run_download_install(&events, &paths, &spec, &guard.lease(), &tasks).await.unwrap();
        let final_dir = paths.lsp_server_version_dir(spec.id.as_str(), "fixture-version");
        assert_eq!(std::fs::read(final_dir.join("server")).unwrap(), BINARY_CONTENT);
        assert!(has_phase(&events, LspInstallPhase::Verifying));
        assert!(has_phase(&events, LspInstallPhase::Extracting));
        assert!(has_phase(&events, LspInstallPhase::Done));
        assert!(!has_phase(&events, LspInstallPhase::Failed));
        assert_eq!(std::fs::read_dir(paths.lsp_dir().join(".tmp")).unwrap().count(), 0);
        assert_eq!(tasks.tracked_count(), 0);
        server.finish().await;
    }

    #[tokio::test]
    async fn checksum_불일치는_기존_설치를_보존하고_extract와_done을_발행하지_않는다() {
        let root = TestRoot::new();
        let server = HttpFixture::start(Some(binary_response())).await;
        let spec = fixture_spec(server.url.clone(), lsp_install::sha256_hex(b"other content"));
        let paths = AppPaths::new(root.0.clone());
        let final_dir = paths.lsp_server_version_dir(spec.id.as_str(), "fixture-version");
        std::fs::create_dir_all(&final_dir).unwrap();
        std::fs::write(final_dir.join("server"), b"previous server").unwrap();
        let events = RecordingEvents::default();
        let store = LspInstallStore::new();
        let guard = store.begin(&spec.id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());

        let error = run_download_install(&events, &paths, &spec, &guard.lease(), &tasks)
            .await
            .unwrap_err();
        let AppError::Localized(localized) = error else {
            panic!("localized checksum error");
        };
        assert_eq!(localized.key, "error.lsp.checksumMismatch");
        assert_eq!(std::fs::read(final_dir.join("server")).unwrap(), b"previous server");
        assert!(!has_phase(&events, LspInstallPhase::Extracting));
        assert!(!has_phase(&events, LspInstallPhase::Done));
        assert!(has_phase(&events, LspInstallPhase::Failed));
        assert_eq!(std::fs::read_dir(paths.lsp_dir().join(".tmp")).unwrap().count(), 0);
        server.finish().await;
    }

    #[tokio::test]
    async fn header나_body가_멈춰도_종료_취소를_관찰하고_done을_발행하지_않는다() {
        for response in [None, Some("HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\npartial".to_string())] {
            let root = TestRoot::new();
            let mut server = HttpFixture::start(response).await;
            let spec = fixture_spec(server.url.clone(), lsp_install::sha256_hex(BINARY_CONTENT));
            let paths = AppPaths::new(root.0.clone());
            let store = LspInstallStore::new();
            let guard = store.begin(&spec.id).unwrap();
            let lease = guard.lease();
            let events = Arc::new(RecordingEvents::default());
            let request_events = events.clone();
            let request_paths = AppPaths::new(root.0.clone());
            let request_spec = spec.clone();
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let installer =
                tokio::spawn(
                    async move { run_download_install(request_events.as_ref(), &request_paths, &request_spec, &lease, &tasks).await },
                );
            server.wait_requested().await;
            store.shutdown();
            let result = tokio::time::timeout(std::time::Duration::from_millis(FIXTURE_TIMEOUT_MS), installer)
                .await
                .unwrap()
                .unwrap();
            assert!(result.is_err());
            assert!(has_phase(events.as_ref(), LspInstallPhase::Failed));
            assert!(!has_phase(events.as_ref(), LspInstallPhase::Done));
            assert!(!paths.lsp_server_dir(spec.id.as_str()).exists());
            let temp = paths.lsp_dir().join(".tmp");
            if temp.exists() {
                assert_eq!(std::fs::read_dir(temp).unwrap().count(), 0);
            }
            assert!(store.begin(&spec.id).is_none());
            server.finish().await;
        }
    }

    #[tokio::test]
    async fn extraction_요청이_취소돼도_실제_worker가_끝날_때까지_슬롯을_보유한다() {
        let root = TestRoot::new();
        let download_path = root.0.join("fixture.download");
        let extract_dir = root.0.join("fixture-extract");
        std::fs::write(&download_path, BINARY_CONTENT).unwrap();
        std::fs::create_dir_all(&extract_dir).unwrap();
        let artifacts = Arc::new(DownloadArtifacts {
            download_path: download_path.clone(),
            extract_dir: extract_dir.clone(),
        });
        let worker_artifacts = artifacts.clone();
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let cancellation_token = guard.cancellation_token();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let worker_tasks = tasks.clone();
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let (finished, finished_rx) = oneshot::channel();
        let installer = tokio::spawn(async move {
            let _artifacts = artifacts;
            let result = run_install_blocking_step(&worker_tasks, &guard.lease(), move || {
                let _artifacts = worker_artifacts;
                started.send(()).unwrap();
                release_rx.recv().unwrap();
                finished.send(()).unwrap();
                Ok(())
            })
            .await;
            drop(guard);
            result
        });

        started_rx.await.unwrap();
        installer.abort();
        assert!(installer.await.unwrap_err().is_cancelled());
        assert!(cancellation_token.load(Ordering::SeqCst));
        assert!(store.begin(&server_id).is_none());
        assert!(download_path.exists());
        assert!(extract_dir.exists());
        tasks.stop_all();
        assert_eq!(tasks.tracked_count(), 1);
        release.send(()).unwrap();
        finished_rx.await.unwrap();
        tokio::time::timeout(std::time::Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            while tasks.tracked_count() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(!download_path.exists());
        assert!(!extract_dir.exists());
        assert!(store.begin(&server_id).is_some());
    }

    #[tokio::test]
    async fn 명시_취소는_extraction_완료_후에도_적용을_거절한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let worker = guard.lease();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let installer = tokio::spawn(async move {
            let result = run_install_blocking_step(&tasks, &worker, move || {
                started.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
            .await;
            (result, guard)
        });

        started_rx.await.unwrap();
        store.cancel(&server_id);
        assert!(store.begin(&server_id).is_none());
        release.send(()).unwrap();
        let (result, guard) = installer.await.unwrap();
        assert!(result.is_err());
        assert!(guard.lease().commit(|| Ok(())).is_err());
        drop(guard);
        assert!(store.begin(&server_id).is_some());
    }

    #[tokio::test]
    async fn 종료한_감독자와_취소된_요청은_blocking_body를_실행하지_않는다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let did_run = Arc::new(AtomicBool::new(false));
        tasks.stop_all();
        let marker = did_run.clone();
        assert!(run_install_blocking_step(&tasks, &guard.lease(), move || {
            marker.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await
        .is_err());

        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        store.cancel(&server_id);
        let marker = did_run.clone();
        assert!(run_install_blocking_step(&tasks, &guard.lease(), move || {
            marker.store(true, Ordering::SeqCst);
            Ok(())
        })
        .await
        .is_err());
        assert!(!did_run.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn blocking_패닉과_작업_오류는_실제_종료_후_반환한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());

        let error = run_install_blocking_step::<()>(&tasks, &guard.lease(), || panic!("synthetic extraction panic"))
            .await
            .unwrap_err();
        let AppError::Localized(localized) = error else {
            panic!("extraction panic must return a localized error");
        };
        assert_eq!(localized.key, "error.lsp.extractTaskFailed");
        assert_eq!(tasks.tracked_count(), 0);
        let error = run_install_blocking_step::<()>(&tasks, &guard.lease(), || Err(AppError::Internal("synthetic error".to_string())))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("synthetic error"));
        assert_eq!(tasks.tracked_count(), 0);
    }
}

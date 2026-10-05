use std::{path::PathBuf, sync::Arc, time::Duration};

use taide_model::error::{AppError, AppResult};
use taide_runtime::AppServices;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};

use crate::preview_web::{Prepared, Request};

const REPLY_CAPACITY: usize = 1;

struct Preparation {
    executable: PathBuf,
    timeout: Duration,
    server: Option<Arc<crate::preview_web_http::Server>>,
    cancellation: watch::Receiver<u64>,
    appearance: crate::preview_web_media::Appearance,
}

pub enum Reply {
    SourceReady(Request),
    Prepared {
        request: Request,
        result: AppResult<Prepared>,
    },
}

pub struct Bridge {
    commands: mpsc::Sender<Request>,
    replies: mpsc::Receiver<Reply>,
    worker: JoinHandle<()>,
    cancelled: watch::Sender<u64>,
    appearance: watch::Sender<crate::preview_web_media::Appearance>,
}

impl Bridge {
    pub fn connect(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        executable: PathBuf,
        timeout: Duration,
    ) -> AppResult<Self> {
        Self::connect_internal(services, repaint, executable, timeout, None, None)
    }

    pub fn connect_served(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        executable: PathBuf,
        timeout: Duration,
        server: Arc<crate::preview_web_http::Server>,
    ) -> AppResult<Self> {
        Self::connect_internal(services, repaint, executable, timeout, Some(server), None)
    }

    pub fn connect_lazy(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        executable: PathBuf,
        timeout: Duration,
        limits: crate::preview_web_http::Limits,
    ) -> AppResult<Self> {
        Self::connect_internal(services, repaint, executable, timeout, None, Some(limits))
    }

    fn connect_internal(
        services: Arc<AppServices>,
        repaint: Arc<dyn Fn() + Send + Sync>,
        executable: PathBuf,
        timeout: Duration,
        mut server: Option<Arc<crate::preview_web_http::Server>>,
        limits: Option<crate::preview_web_http::Limits>,
    ) -> AppResult<Self> {
        if !executable.is_absolute() {
            return Err(crate::preview::invalid(
                "HTML helper executable must be absolute",
            ));
        }
        let (commands, mut receiver) = mpsc::channel::<Request>(crate::host::HOST_COMMAND_CAPACITY);
        let (sender, replies) = mpsc::channel(REPLY_CAPACITY);
        let (cancelled, cancellation) = watch::channel(0);
        let (appearance, media_appearance) =
            watch::channel(crate::preview_web_media::Appearance::default());
        let worker = services
            .tasks
            .clone()
            .spawn_transient_handle("native-html-host", async move {
                while let Some(request) = receiver.recv().await {
                    if services.state.is_shutting_down() {
                        break;
                    }
                    if *cancellation.borrow() >= request.token {
                        if sender.send(Reply::Prepared { request, result: Err(AppError::Forbidden("HTML preview request was superseded".into())) }).await.is_err() {
                            break;
                        }
                        repaint();
                        continue;
                    }
                    if server.is_none() && let Some(limits) = limits {
                        let result = tokio::select! {
                            biased;
                            _ = sender.closed() => break,
                            result = crate::preview_web_http::Server::start(services.tasks.clone(), limits) => result,
                        };
                        match result {
                            Ok(started) => server = Some(Arc::new(started)),
                            Err(error) => {
                                if sender.send(Reply::Prepared { request, result: Err(error) }).await.is_err() {
                                    break;
                                }
                                repaint();
                                continue;
                            }
                        }
                    }
                    let appearance = *media_appearance.borrow();
                    let Some(result) = dispatch(
                        &services,
                        &request,
                        &sender,
                        &repaint,
                        Preparation { executable: executable.clone(), timeout, server: server.clone(), cancellation: cancellation.clone(), appearance },
                    )
                    .await
                    else {
                        break;
                    };
                    if sender
                        .send(Reply::Prepared { request, result })
                        .await
                        .is_err()
                    {
                        break;
                    }
                    repaint();
                }
            })
            .ok_or_else(|| AppError::Forbidden("HTML host is shutting down".into()))?;
        Ok(Self {
            commands,
            replies,
            worker,
            cancelled,
            appearance,
        })
    }

    pub fn submit(&self, request: Request) -> AppResult<()> {
        self.commands
            .try_send(request)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    AppError::Internal("HTML host command queue is full".into())
                }
                mpsc::error::TrySendError::Closed(_) => {
                    AppError::Forbidden("HTML host is disconnected".into())
                }
            })
    }

    pub fn poll(&mut self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }

    pub fn cancel_through(&self, token: u64) {
        self.cancelled.send_if_modified(|current| {
            if *current >= token {
                return false;
            }
            *current = token;
            true
        });
    }

    pub fn set_media_appearance(&self, appearance: crate::preview_web_media::Appearance) -> bool {
        self.appearance.send_if_modified(|current| {
            if *current == appearance {
                return false;
            }
            *current = appearance;
            true
        })
    }

    pub fn disconnect(self) -> JoinHandle<()> {
        drop(self.commands);
        drop(self.replies);
        self.worker
    }
}

async fn dispatch(
    services: &AppServices,
    request: &Request,
    sender: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
    preparation: Preparation,
) -> Option<AppResult<Prepared>> {
    let Preparation {
        executable,
        timeout,
        server,
        mut cancellation,
        appearance,
    } = preparation;
    let (source_ready, mut ready) = oneshot::channel();
    let read = async move {
        let on_source_ready = move || {
            let _result = source_ready.send(());
        };
        match crate::open_with::preview_kind(&request.path) {
            Some(crate::open_with::PreviewKind::Audio | crate::open_with::PreviewKind::Video) => {
                let server = server.ok_or_else(|| {
                    crate::preview::invalid("media preview requires an isolated resource listener")
                })?;
                crate::preview_web_media::read(
                    services,
                    request,
                    server,
                    appearance,
                    on_source_ready,
                )
                .await
            }
            Some(crate::open_with::PreviewKind::Html) => {
                crate::preview_web::read_internal(
                    services,
                    request,
                    executable,
                    timeout,
                    server,
                    on_source_ready,
                    |_| {},
                )
                .await
            }
            _ => Err(crate::preview::invalid("web preview kind is not supported")),
        }
    };
    tokio::pin!(read);
    let (early, is_source_ready) = tokio::select! {
        biased;
        _ = sender.closed() => return None,
        _ = cancellation.wait_for(|token| *token >= request.token) => return Some(Err(AppError::Forbidden("HTML preview request was superseded".into()))),
        progress = &mut ready => (None, progress.is_ok()),
        result = &mut read => (Some(result), ready.try_recv().is_ok()),
    };
    if is_source_ready {
        if sender
            .send(Reply::SourceReady(request.clone()))
            .await
            .is_err()
        {
            return None;
        }
        repaint();
    }
    if let Some(result) = early {
        return Some(result);
    }
    tokio::select! {
        biased;
        _ = sender.closed() => None,
        _ = cancellation.wait_for(|token| *token >= request.token) => Some(Err(AppError::Forbidden("HTML preview request was superseded".into()))),
        result = &mut read => Some(result),
    }
}

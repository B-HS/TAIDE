use super::*;

const PAST_STALE: Duration = Duration::from_secs(61);
const PAST_GC: Duration = Duration::from_secs(601);

fn fetches(data: &mut Data) -> Vec<(Query, u64)> {
    data.take_requests()
        .into_iter()
        .filter_map(|request| match request {
            Request::Fetch { query, generation } => Some((query, generation)),
            Request::Act(_) => None,
        })
        .collect()
}

fn localized(key: &str) -> AppError {
    AppError::localized(
        taide_model::error::AppErrorKind::InvalidArgument,
        key,
        "fallback",
    )
}

fn progress(phase: LspInstallPhase) -> Progress {
    Progress {
        phase,
        received_bytes: 0.0,
        total_bytes: None,
        message: None,
    }
}

#[test]
fn 조회는_mount와_신선도와_무효화에서만_다시_요청한다() {
    let mut data = Data::default();
    let start = Instant::now();
    data.observe(Query::AiTokens, true, start);
    assert_eq!(fetches(&mut data), vec![(Query::AiTokens, 1)]);
    data.observe(Query::AiTokens, false, start);
    assert!(fetches(&mut data).is_empty());
    assert!(matches!(data.ai_tokens(), Status::Pending));
    data.accept(
        Reply::AiTokens {
            generation: 1,
            result: Ok(AiTokenStatus {
                ollama_cloud: true,
                codex: false,
                omlx: false,
            }),
        },
        start,
    );
    assert!(matches!(data.ai_tokens(), Status::Ready(status) if status.ollama_cloud));
    data.observe(Query::AiTokens, true, start);
    assert!(fetches(&mut data).is_empty());
    data.observe(Query::AiTokens, false, start + PAST_STALE);
    assert!(fetches(&mut data).is_empty());
    data.observe(Query::AiTokens, true, start + PAST_STALE);
    assert_eq!(fetches(&mut data), vec![(Query::AiTokens, 2)]);

    data.observe(Query::LspServers, true, start);
    assert_eq!(fetches(&mut data), vec![(Query::LspServers, 1)]);
    data.accept(
        Reply::LspServers {
            generation: 1,
            result: Ok(Vec::new()),
        },
        start,
    );
    data.observe(Query::LspServers, true, start + PAST_STALE);
    assert!(fetches(&mut data).is_empty());
    data.accept(
        Reply::LspInstalled {
            server: "rust-analyzer".into(),
            result: Ok(()),
        },
        start,
    );
    data.observe(Query::LspServers, false, start + PAST_STALE);
    assert_eq!(fetches(&mut data), vec![(Query::LspServers, 2)]);
}

#[test]
fn 실패와_늦은_세대와_gc는_원본_query_수명을_따른다() {
    let mut data = Data::default();
    let start = Instant::now();
    data.observe(Query::AiModels(AiProviderId::Omlx), true, start);
    assert_eq!(
        fetches(&mut data),
        vec![(Query::AiModels(AiProviderId::Omlx), 1)]
    );
    data.accept(
        Reply::AiModels {
            provider: AiProviderId::Omlx,
            generation: 0,
            result: Ok(Vec::new()),
        },
        start,
    );
    assert!(matches!(
        data.ai_models(AiProviderId::Omlx),
        Status::Pending
    ));
    data.accept(
        Reply::AiModels {
            provider: AiProviderId::Omlx,
            generation: 1,
            result: Err(AppError::Internal("offline".into())),
        },
        start,
    );
    assert!(matches!(
        data.ai_models(AiProviderId::Omlx),
        Status::Failed(_)
    ));
    data.observe(Query::AiModels(AiProviderId::Omlx), false, start);
    assert!(fetches(&mut data).is_empty());
    data.observe(Query::AiModels(AiProviderId::Omlx), true, start);
    assert_eq!(
        fetches(&mut data),
        vec![(Query::AiModels(AiProviderId::Omlx), 2)]
    );
    data.accept(
        Reply::AiModels {
            provider: AiProviderId::Omlx,
            generation: 2,
            result: Ok(vec![AiModelInfo {
                model_id: "m".into(),
                display_name: None,
            }]),
        },
        start,
    );
    data.observe(Query::AiModels(AiProviderId::Omlx), true, start + PAST_GC);
    assert_eq!(
        fetches(&mut data),
        vec![(Query::AiModels(AiProviderId::Omlx), 3)]
    );
    assert!(matches!(
        data.ai_models(AiProviderId::Omlx),
        Status::Pending
    ));
}

#[test]
fn lsp_설치_실패는_취소키를_조용히_넘기고_끝나지_않은_진행만_지운다() {
    let mut data = Data::default();
    let now = Instant::now();
    let server = LspServerId::from("rust-analyzer");
    data.record_progress(server.clone(), progress(LspInstallPhase::Downloading));
    assert!(data.has_active_install());
    let notice = data.accept(
        Reply::LspInstalled {
            server: server.clone(),
            result: Err(localized(INSTALL_CANCELLED_KEY)),
        },
        now,
    );
    assert!(notice.is_none());
    assert!(data.lsp_progress(&server).is_none());

    data.record_progress(server.clone(), progress(LspInstallPhase::Failed));
    let notice = data.accept(
        Reply::LspInstalled {
            server: server.clone(),
            result: Err(AppError::Internal("checksum".into())),
        },
        now,
    );
    assert!(matches!(notice, Some(Notice::Error { fallback: None, .. })));
    assert!(data.lsp_progress(&server).is_some_and(Progress::is_failed));
    assert!(!data.has_active_install());

    assert!(matches!(
        data.accept(Reply::HintCopied(Ok(())), now),
        Some(Notice::Success("settings.lspCommandCopied"))
    ));
    assert!(matches!(
        data.accept(
            Reply::HintCopied(Err(AppError::Internal("denied".into()))),
            now
        ),
        Some(Notice::Failure("settings.lspCommandCopyFailed"))
    ));
    assert!(matches!(
        data.accept(
            Reply::LspCancelled(Err(AppError::Internal("gone".into()))),
            now
        ),
        Some(Notice::Error { fallback: None, .. })
    ));
}

#[test]
fn ai_토큰_저장중은_마지막_요청만_따르고_성공하면_상태와_모델을_무효화한다() {
    let mut data = Data::default();
    let now = Instant::now();
    data.save_ai_token(AiProviderId::OllamaCloud, Secret::new("first".into()));
    data.save_ai_token(AiProviderId::Codex, Secret::new("second".into()));
    assert!(!data.is_saving_ai_token(AiProviderId::OllamaCloud));
    assert!(data.is_saving_ai_token(AiProviderId::Codex));
    let requests = data.take_requests();
    assert_eq!(requests.len(), 2);
    assert!(format!("{requests:?}").contains("Secret(<redacted>)"));
    assert!(!format!("{requests:?}").contains("second"));
    let notice = data.accept(
        Reply::AiTokenSaved {
            provider: AiProviderId::OllamaCloud,
            sequence: 1,
            result: Err(AppError::Internal(String::new())),
        },
        now,
    );
    assert!(matches!(
        notice,
        Some(Notice::Error {
            fallback: Some("settings.aiTokenSaveFailed"),
            ..
        })
    ));
    assert!(data.is_saving_ai_token(AiProviderId::Codex));

    data.observe(Query::AiTokens, true, now);
    data.observe(Query::AiModels(AiProviderId::Codex), true, now);
    let generations = fetches(&mut data);
    for (query, generation) in generations {
        let reply = match query {
            Query::AiTokens => Reply::AiTokens {
                generation,
                result: Ok(AiTokenStatus {
                    ollama_cloud: false,
                    codex: false,
                    omlx: false,
                }),
            },
            Query::AiModels(provider) => Reply::AiModels {
                provider,
                generation,
                result: Ok(Vec::new()),
            },
            Query::LspServers
            | Query::Plugins
            | Query::SyncStatus
            | Query::RemoteStatus
            | Query::CliStatus
            | Query::HooksStatus(_) => {
                unreachable!()
            }
        };
        data.accept(reply, now);
    }
    assert!(
        data.accept(
            Reply::AiTokenSaved {
                provider: AiProviderId::Codex,
                sequence: 2,
                result: Ok(()),
            },
            now,
        )
        .is_none()
    );
    assert!(!data.is_saving_ai_token(AiProviderId::Codex));
    data.observe(Query::AiTokens, false, now);
    data.observe(Query::AiModels(AiProviderId::Codex), false, now);
    data.observe(Query::AiModels(AiProviderId::OllamaCloud), false, now);
    assert_eq!(
        fetches(&mut data),
        vec![
            (Query::AiTokens, 2),
            (Query::AiModels(AiProviderId::Codex), 2),
            (Query::AiModels(AiProviderId::OllamaCloud), 1),
        ]
    );
    assert!(matches!(
        data.accept(
            Reply::AiTokenCleared {
                provider: AiProviderId::Codex,
                result: Err(AppError::Internal("keychain".into())),
            },
            now,
        ),
        Some(Notice::Error {
            fallback: Some("settings.aiTokenClearFailed"),
            ..
        })
    ));
}

#[test]
fn omlx_base_url_변경은_토큰상태와_omlx_모델을_무효화한다() {
    let mut data = Data::default();
    let now = Instant::now();
    let mut settings = Settings::default();
    data.observe_settings(&settings);
    data.observe(Query::AiTokens, true, now);
    data.observe(Query::AiModels(AiProviderId::Omlx), true, now);
    for (query, generation) in fetches(&mut data) {
        let reply = match query {
            Query::AiTokens => Reply::AiTokens {
                generation,
                result: Ok(AiTokenStatus {
                    ollama_cloud: false,
                    codex: false,
                    omlx: false,
                }),
            },
            Query::AiModels(provider) => Reply::AiModels {
                provider,
                generation,
                result: Ok(Vec::new()),
            },
            Query::LspServers
            | Query::Plugins
            | Query::SyncStatus
            | Query::RemoteStatus
            | Query::CliStatus
            | Query::HooksStatus(_) => {
                unreachable!()
            }
        };
        data.accept(reply, now);
    }
    data.observe_settings(&settings);
    data.observe(Query::AiTokens, false, now);
    assert!(fetches(&mut data).is_empty());
    settings.ai_omlx_base_url = Some("http://127.0.0.1:9".into());
    data.observe_settings(&settings);
    data.observe(Query::AiTokens, false, now);
    data.observe(Query::AiModels(AiProviderId::Omlx), false, now);
    assert_eq!(
        fetches(&mut data),
        vec![
            (Query::AiTokens, 2),
            (Query::AiModels(AiProviderId::Omlx), 2)
        ]
    );
}

#[test]
fn 제출_실패_응답은_요청의_식별자를_보존한다() {
    let request = Request::Act(Action::AiSetToken {
        provider: AiProviderId::Omlx,
        token: Secret::new("key".into()),
        sequence: 7,
    });
    assert!(matches!(
        request.failed(AppError::Internal("full".into())),
        Reply::AiTokenSaved {
            provider: AiProviderId::Omlx,
            sequence: 7,
            result: Err(_),
        }
    ));
    let request = Request::Fetch {
        query: Query::AiModels(AiProviderId::Codex),
        generation: 3,
    };
    assert!(matches!(
        request.failed(AppError::Internal("full".into())),
        Reply::AiModels {
            provider: AiProviderId::Codex,
            generation: 3,
            result: Err(_),
        }
    ));
}

fn extraction() -> VsixExtraction {
    VsixExtraction {
        path: "/tmp/theme.vsix".into(),
        extension: "Fixture".into(),
        existing_ids: Vec::new(),
        candidates: Vec::new(),
    }
}

#[test]
fn plugin_응답은_목록_교체_무효화와_원본_toast를_고르고_진행_상태를_푼다() {
    let mut data = Data::default();
    let now = Instant::now();
    data.observe(Query::Plugins, true, now);
    let [(Query::Plugins, generation)] = fetches(&mut data)[..] else {
        panic!("plugins fetched once");
    };
    data.accept(
        Reply::Plugins {
            generation,
            result: Ok(Vec::new()),
        },
        now,
    );
    data.reload_plugins();
    assert!(data.is_reloading_plugins());
    assert!(matches!(
        data.accept(
            Reply::PluginsReloaded(Err(localized("error.plugin.x"))),
            now
        ),
        Some(Notice::Error { fallback: None, .. })
    ));
    assert!(!data.is_reloading_plugins());
    assert!(matches!(
        data.accept(Reply::PluginUninstalled(Ok(Vec::new())), now),
        Some(Notice::Success("settings.pluginUninstallSuccess"))
    ));
    assert!(matches!(
        data.accept(
            Reply::PluginUninstalled(Err(localized("error.plugin.y"))),
            now
        ),
        Some(Notice::Error {
            fallback: Some("settings.pluginUninstallFailed"),
            ..
        })
    ));
    data.picked(PickPurpose::PluginArchive, Some("/tmp/plugin.zip".into()));
    assert!(data.is_installing_plugin());
    data.take_requests();
    assert!(matches!(
        data.accept(Reply::PluginInstalled(Ok(())), now),
        Some(Notice::Success("settings.pluginInstallSuccess"))
    ));
    assert!(!data.is_installing_plugin());
    data.observe(Query::Plugins, false, now);
    assert_eq!(fetches(&mut data), vec![(Query::Plugins, generation + 1)]);
    assert!(matches!(
        data.accept(
            Reply::PluginInstalled(Err(localized("error.plugin.z"))),
            now
        ),
        Some(Notice::Error {
            fallback: Some("settings.pluginInstallFailed"),
            ..
        })
    ));
}

#[test]
fn vsix_사건은_ticket별로_전달되고_저장_성공_문구에_수와_확장_이름을_싣는다() {
    let mut data = Data::default();
    let now = Instant::now();
    let first = data.next_vsix_ticket().unwrap();
    let second = data.next_vsix_ticket().unwrap();
    assert_ne!(first, second);
    data.picked(PickPurpose::Vsix(first), None);
    assert!(data.take_requests().is_empty());
    assert!(!data.is_extracting_vsix());
    data.picked(PickPurpose::Vsix(first), Some("/tmp/theme.vsix".into()));
    assert!(data.is_extracting_vsix());
    assert!(
        data.accept(
            Reply::VsixExtracted {
                ticket: first,
                result: Ok(extraction())
            },
            now
        )
        .is_none()
    );
    assert!(!data.is_extracting_vsix());
    assert!(data.take_vsix_events(second).is_empty());
    assert!(matches!(
        data.take_vsix_events(first).as_slice(),
        [VsixEvent::Extracted(_)]
    ));
    assert!(data.take_vsix_events(first).is_empty());
    match data.accept(
        Reply::VsixThemesSaved {
            ticket: first,
            count: 2,
            extension: "Fixture".into(),
            result: Ok(()),
        },
        now,
    ) {
        Some(Notice::SuccessWith { key, arguments }) => {
            assert_eq!(key, "settings.themeImportSuccess");
            assert_eq!(
                arguments,
                vec![
                    ("count", "2".to_owned()),
                    ("extension", "Fixture".to_owned())
                ]
            );
        }
        other => panic!("theme save success: {other:?}"),
    }
    assert!(
        data.accept(
            Reply::VsixGrammarsImported {
                ticket: first,
                result: Err(localized("error.vsix.x"))
            },
            now
        )
        .is_none()
    );
    assert!(matches!(
        data.take_vsix_events(first).as_slice(),
        [VsixEvent::Saved, VsixEvent::GrammarsImported(Err(_))]
    ));
    for _ in 0..VSIX_EVENT_LIMIT + 1 {
        data.accept(
            Reply::VsixGrammarsImported {
                ticket: second,
                result: Ok(()),
            },
            now,
        );
    }
    assert_eq!(data.take_vsix_events(second).len(), VSIX_EVENT_LIMIT);
    let failed = Request::Act(Action::VsixSaveThemes {
        themes: Vec::new(),
        extension: "Fixture".into(),
        ticket: first,
    })
    .failed(localized("error.host"));
    assert!(matches!(
        failed,
        Reply::VsixThemesSaved { ticket, count: 0, result: Err(_), .. } if ticket == first
    ));
}

const REMOTE_BEFORE_REFETCH: Duration = Duration::from_secs(4);
const REMOTE_REFETCH: Duration = Duration::from_secs(5);
const OWNER: u64 = 7;
const OTHER_OWNER: u64 = 8;

fn sync_status(connected: bool, last_synced_at: Option<&str>) -> SyncStatus {
    SyncStatus {
        connected,
        has_gist: connected,
        last_synced_at: last_synced_at.map(Into::into),
        remote_newer: None,
    }
}

fn remote_status(running: bool, password_configured: bool) -> RemoteStatus {
    RemoteStatus {
        running,
        port: 4100,
        client_count: 2,
        password_configured,
    }
}

fn notice_key(notice: Option<Notice>) -> Option<&'static str> {
    match notice? {
        Notice::Success(key) | Notice::Failure(key) | Notice::Warning(key) => Some(key),
        Notice::Error { fallback, .. } => fallback,
        Notice::SuccessWith { key, .. } => Some(key),
        Notice::SettingsFailed(_) => Some("settings.saveFailed"),
    }
}

#[test]
fn 동기화_응답은_상태를_교체하고_원본_문구와_충돌_소유자_사건을_만든다() {
    let mut data = Data::default();
    let now = Instant::now();
    data.observe(Query::SyncStatus, true, now);
    assert_eq!(fetches(&mut data), vec![(Query::SyncStatus, 1)]);
    data.accept(
        Reply::SyncStatus {
            generation: 1,
            result: Ok(sync_status(false, None)),
        },
        now,
    );
    data.act(Action::SyncConnect(Secret::new("ghp_fixture".into())));
    assert!(data.is_busy(Busy::SyncConnect));
    let requests = data.take_requests();
    assert!(!format!("{requests:?}").contains("ghp_fixture"));
    assert!(
        data.accept(Reply::SyncConnected(Ok(sync_status(true, None))), now)
            .is_none()
    );
    assert!(!data.is_busy(Busy::SyncConnect));
    assert!(matches!(data.sync_status(), Status::Ready(status) if status.connected));
    assert_eq!(
        notice_key(data.accept(Reply::SyncConnected(Err(localized("error.sync"))), now)),
        Some("settings.syncConnectFailed")
    );
    assert_eq!(
        notice_key(data.accept(Reply::SyncDisconnected(Err(localized("error.sync"))), now)),
        Some("settings.syncDisconnectFailed")
    );
    data.act(Action::SyncUpload);
    data.act(Action::SyncUpload);
    assert_eq!(
        notice_key(data.accept(Reply::SyncUploaded(Err(localized("error.sync"))), now)),
        Some("settings.syncUploadFailed")
    );
    assert!(data.is_busy(Busy::SyncUpload));
    assert_eq!(
        notice_key(data.accept(
            Reply::SyncUploaded(Ok(sync_status(true, Some("2026-10-28T05:05:09Z")))),
            now
        )),
        Some("settings.syncUploadSuccess")
    );
    assert!(!data.is_busy(Busy::SyncUpload));
    assert!(matches!(
        data.sync_status(),
        Status::Ready(status) if status.last_synced_at.as_deref() == Some("2026-10-28T05:05:09Z")
    ));
    data.act(Action::SyncDownload {
        force: false,
        owner: OWNER,
    });
    assert!(data.is_busy(Busy::SyncDownload));
    assert!(
        data.accept(
            Reply::SyncDownloaded {
                force: false,
                owner: OWNER,
                result: Ok(SyncDownloadResult::Conflict {
                    remote_updated_at: "2026-10-29T00:00:00Z".into(),
                }),
            },
            now,
        )
        .is_none()
    );
    assert!(!data.is_busy(Busy::SyncDownload));
    assert!(data.take_owner_events(OTHER_OWNER).is_empty());
    assert_eq!(
        data.take_owner_events(OWNER),
        vec![OwnerEvent::SyncConflict]
    );
    assert!(data.take_owner_events(OWNER).is_empty());
    assert_eq!(
        notice_key(data.accept(
            Reply::SyncDownloaded {
                force: true,
                owner: OWNER,
                result: Ok(SyncDownloadResult::Conflict {
                    remote_updated_at: "2026-10-29T00:00:00Z".into(),
                }),
            },
            now,
        )),
        Some("settings.syncDownloadSuccess")
    );
    assert_eq!(
        notice_key(data.accept(
            Reply::SyncDownloaded {
                force: false,
                owner: OWNER,
                result: Ok(SyncDownloadResult::Applied {
                    status: sync_status(true, Some("2026-10-29T00:00:00Z")),
                }),
            },
            now,
        )),
        Some("settings.syncDownloadSuccess")
    );
    assert!(matches!(
        data.sync_status(),
        Status::Ready(status) if status.last_synced_at.as_deref() == Some("2026-10-29T00:00:00Z")
    ));
    assert_eq!(
        notice_key(data.accept(
            Reply::SyncDownloaded {
                force: false,
                owner: OWNER,
                result: Err(localized("error.sync")),
            },
            now,
        )),
        Some("settings.syncDownloadFailed")
    );
    assert_eq!(
        notice_key(data.accept(Reply::SyncDisconnected(Ok(sync_status(false, None))), now)),
        Some("settings.syncDisconnected")
    );
    data.record_sync_status(sync_status(true, Some("event")), now);
    assert!(matches!(
        data.sync_status(),
        Status::Ready(status) if status.last_synced_at.as_deref() == Some("event")
    ));
}

#[test]
fn 원격_상태는_관찰_중_5초마다_다시_조회하고_비밀번호와_토글이_무효화한다() {
    let mut data = Data::default();
    let start = Instant::now();
    let mut settings = Settings::default();
    data.observe_settings(&settings);
    data.observe(Query::RemoteStatus, true, start);
    assert_eq!(fetches(&mut data), vec![(Query::RemoteStatus, 1)]);
    assert_eq!(data.remote_refetch_in(start), None);
    data.accept(
        Reply::RemoteStatus {
            generation: 1,
            result: Ok(remote_status(false, false)),
        },
        start,
    );
    let early = start + REMOTE_BEFORE_REFETCH;
    data.observe(Query::RemoteStatus, false, early);
    assert!(fetches(&mut data).is_empty());
    assert_eq!(
        data.remote_refetch_in(early),
        Some(REMOTE_REFETCH - REMOTE_BEFORE_REFETCH)
    );
    let due = start + REMOTE_REFETCH;
    data.observe(Query::RemoteStatus, false, due);
    assert_eq!(fetches(&mut data), vec![(Query::RemoteStatus, 2)]);
    data.accept(
        Reply::RemoteStatus {
            generation: 2,
            result: Ok(remote_status(true, false)),
        },
        due,
    );
    data.observe(Query::RemoteStatus, false, due);
    assert!(fetches(&mut data).is_empty());
    data.record_remote_status(remote_status(true, true), due);
    assert!(matches!(data.remote_status(), Status::Ready(status) if status.password_configured));

    data.act(Action::RemoteSetPassword(Secret::new("long-enough".into())));
    assert!(data.is_busy(Busy::RemotePassword));
    data.take_requests();
    assert_eq!(
        notice_key(data.accept(
            Reply::RemotePasswordSet(Err(localized("error.remote"))),
            due
        )),
        Some("remote.passwordSaveFailed")
    );
    assert!(!data.is_busy(Busy::RemotePassword));
    data.observe(Query::RemoteStatus, false, due);
    assert!(fetches(&mut data).is_empty());
    assert_eq!(
        notice_key(data.accept(Reply::RemotePasswordSet(Ok(())), due)),
        Some("remote.passwordSessionsRevoked")
    );
    data.observe(Query::RemoteStatus, false, due);
    assert_eq!(fetches(&mut data), vec![(Query::RemoteStatus, 3)]);
    data.accept(
        Reply::RemoteStatus {
            generation: 3,
            result: Ok(remote_status(true, true)),
        },
        due,
    );
    assert!(
        data.accept(
            Reply::RemotePasswordCleared(Err(localized("error.remote"))),
            due
        )
        .is_none()
    );
    assert_eq!(
        notice_key(data.accept(Reply::RemotePasswordCleared(Ok(())), due)),
        Some("remote.passwordSessionsRevoked")
    );
    data.observe(Query::RemoteStatus, false, due);
    assert_eq!(fetches(&mut data), vec![(Query::RemoteStatus, 4)]);
    data.accept(
        Reply::RemoteStatus {
            generation: 4,
            result: Ok(remote_status(true, false)),
        },
        due,
    );

    data.observe_settings(&settings);
    data.observe(Query::RemoteStatus, false, due);
    assert!(fetches(&mut data).is_empty());
    settings.remote_access_enabled = true;
    data.observe_settings(&settings);
    data.observe(Query::RemoteStatus, false, due);
    assert_eq!(fetches(&mut data), vec![(Query::RemoteStatus, 5)]);
}

#[test]
fn 원격_링크와_세션폐기와_설정저장은_원본_알림과_진행상태를_따른다() {
    let mut data = Data::default();
    let now = Instant::now();
    let link = RemoteLinkInfo {
        url: "http://127.0.0.1:4100/?t=token".into(),
    };
    data.act(Action::RemoteIssueLink { owner: OWNER });
    assert!(data.is_busy(Busy::RemoteIssueLink));
    assert_eq!(
        notice_key(data.accept(
            Reply::RemoteLinkIssued {
                owner: OWNER,
                result: Ok((link.clone(), Ok(()))),
            },
            now,
        )),
        Some("remote.linkCopied")
    );
    assert!(!data.is_busy(Busy::RemoteIssueLink));
    assert!(
        data.accept(
            Reply::RemoteLinkIssued {
                owner: OWNER,
                result: Ok((link.clone(), Err(localized("error.clipboard")))),
            },
            now,
        )
        .is_none()
    );
    assert_eq!(
        data.take_owner_events(OWNER),
        vec![
            OwnerEvent::RemoteLinkIssued(link.url.clone()),
            OwnerEvent::RemoteLinkIssued(link.url.clone())
        ]
    );
    assert_eq!(
        notice_key(data.accept(
            Reply::RemoteLinkIssued {
                owner: OWNER,
                result: Err(localized("error.remote.serverNotRunning")),
            },
            now,
        )),
        Some("remote.startFailed")
    );
    assert!(data.take_owner_events(OWNER).is_empty());
    for _ in 0..=OWNER_EVENT_LIMIT {
        data.accept(
            Reply::RemoteLinkIssued {
                owner: OTHER_OWNER,
                result: Ok((link.clone(), Ok(()))),
            },
            now,
        );
    }
    assert_eq!(data.take_owner_events(OTHER_OWNER).len(), OWNER_EVENT_LIMIT);
    data.act(Action::RemoteRevoke);
    assert!(data.is_busy(Busy::RemoteRevoke));
    assert_eq!(
        notice_key(data.accept(Reply::RemoteRevoked(Ok(())), now)),
        Some("remote.sessionsRevoked")
    );
    assert!(
        data.accept(Reply::RemoteRevoked(Err(localized("error.remote"))), now)
            .is_none()
    );
    data.act(Action::UpdateSettings(HostedChange::RemoteAllowedHosts(
        vec!["tunnel.example.com".into()],
    )));
    assert!(data.is_busy(Busy::Settings));
    assert!(matches!(
        data.accept(
            Reply::SettingsUpdated(Err(localized("error.settings"))),
            now
        ),
        Some(Notice::SettingsFailed(_))
    ));
    assert!(!data.is_busy(Busy::Settings));
    data.take_requests();

    let error = || localized("error.host");
    assert!(matches!(
        Request::Act(Action::SyncDownload {
            force: true,
            owner: OWNER
        })
        .failed(error()),
        Reply::SyncDownloaded {
            force: true,
            owner: OWNER,
            result: Err(_)
        }
    ));
    assert!(matches!(
        Request::Act(Action::RemoteIssueLink { owner: OWNER }).failed(error()),
        Reply::RemoteLinkIssued {
            owner: OWNER,
            result: Err(_)
        }
    ));
    assert!(matches!(
        Request::Act(Action::RemoteClearPassword).failed(error()),
        Reply::RemotePasswordCleared(Err(_))
    ));
    assert!(matches!(
        Request::Act(Action::UpdateSettings(HostedChange::RemoteAccess(true))).failed(error()),
        Reply::SettingsUpdated(Err(_))
    ));
    assert!(matches!(
        Request::Fetch {
            query: Query::SyncStatus,
            generation: 3
        }
        .failed(error()),
        Reply::SyncStatus {
            generation: 3,
            result: Err(_)
        }
    ));
    assert!(matches!(
        Request::Fetch {
            query: Query::RemoteStatus,
            generation: 4
        }
        .failed(error()),
        Reply::RemoteStatus {
            generation: 4,
            result: Err(_)
        }
    ));
}

fn error() -> AppError {
    localized("error.host")
}

fn cli_status(installed: bool) -> CliInstallStatus {
    CliInstallStatus {
        installed,
        resolved_path: None,
        dangling: false,
        target_path: "/usr/local/bin/taide".into(),
        editor_env_hint: String::new(),
    }
}

fn hooks_status(installed: bool) -> AgentHooksStatus {
    AgentHooksStatus {
        agent_name: "codex".into(),
        scope: taide_model::agent::HookInstallScope::User,
        installed,
        requires_taide_cli: true,
    }
}

fn user_target() -> HookTarget {
    HookTarget {
        project: ProjectId(String::new()),
        agent: "codex",
    }
}

#[test]
fn cli_설치_해제는_진행을_표시하고_응답상태로_교체하며_취소는_성공문구가_없다() {
    let mut data = Data::default();
    let now = Instant::now();
    data.observe(Query::CliStatus, true, now);
    assert_eq!(fetches(&mut data), vec![(Query::CliStatus, 1)]);
    assert!(matches!(data.cli_status(), Status::Pending));
    data.accept(
        Reply::CliStatus {
            generation: 1,
            result: Ok(cli_status(false)),
        },
        now,
    );
    assert!(matches!(data.cli_status(), Status::Ready(status) if !status.installed));
    data.act(Action::CliInstall);
    assert!(data.is_busy(Busy::CliInstall));
    assert!(!data.is_busy(Busy::CliUninstall));
    let cancelled = data.accept(
        Reply::CliInstalled(Ok(CliOutcome {
            status: cli_status(false),
            is_cancelled: true,
        })),
        now,
    );
    assert!(cancelled.is_none());
    assert!(!data.is_busy(Busy::CliInstall));
    data.act(Action::CliInstall);
    let installed = data.accept(
        Reply::CliInstalled(Ok(CliOutcome {
            status: cli_status(true),
            is_cancelled: false,
        })),
        now,
    );
    assert!(matches!(
        installed,
        Some(Notice::Success("settings.cliInstallSuccess"))
    ));
    assert!(matches!(data.cli_status(), Status::Ready(status) if status.installed));
    data.act(Action::CliInstall);
    assert!(matches!(
        data.accept(Reply::CliInstalled(Err(error())), now),
        Some(Notice::Failure("settings.cliInstallFailed"))
    ));
    assert!(matches!(data.cli_status(), Status::Ready(status) if status.installed));
    data.act(Action::CliUninstall);
    assert!(data.is_busy(Busy::CliUninstall));
    assert!(matches!(
        data.accept(Reply::CliUninstalled(Err(error())), now),
        Some(Notice::Failure("settings.cliUninstallFailed"))
    ));
    data.act(Action::CliUninstall);
    assert!(matches!(
        data.accept(
            Reply::CliUninstalled(Ok(CliOutcome {
                status: cli_status(false),
                is_cancelled: false,
            })),
            now,
        ),
        Some(Notice::Success("settings.cliUninstallSuccess"))
    ));
    assert!(!data.is_busy(Busy::CliUninstall));
    assert!(matches!(data.cli_status(), Status::Ready(status) if !status.installed));
}

#[test]
fn hooks_조회는_대상별로_나뉘고_설치_해제_응답이_그_대상만_교체한다() {
    let mut data = Data::default();
    let now = Instant::now();
    let project = HookTarget {
        project: ProjectId("prj-a".into()),
        agent: "claude",
    };
    data.observe(Query::HooksStatus(project.clone()), true, now);
    data.observe(Query::HooksStatus(user_target()), true, now);
    assert_eq!(
        fetches(&mut data),
        vec![
            (Query::HooksStatus(project.clone()), 1),
            (Query::HooksStatus(user_target()), 1)
        ]
    );
    data.observe(Query::HooksStatus(user_target()), false, now);
    assert!(fetches(&mut data).is_empty());
    data.accept(
        Reply::HooksStatus {
            target: user_target(),
            generation: 1,
            result: Ok(hooks_status(false)),
        },
        now,
    );
    assert!(matches!(data.hooks_status(&project), Status::Pending));
    assert!(
        matches!(data.hooks_status(&user_target()), Status::Ready(status) if !status.installed)
    );
    data.change_hooks(user_target(), HookChange::Install);
    assert!(data.is_changing_hooks(&user_target(), HookChange::Install));
    assert!(!data.is_changing_hooks(&user_target(), HookChange::Uninstall));
    assert!(!data.is_changing_hooks(&project, HookChange::Install));
    assert!(matches!(
        data.take_requests().as_slice(),
        [Request::Act(Action::Hooks {
            change: HookChange::Install,
            ..
        })]
    ));
    let notice = data.accept(
        Reply::HooksChanged {
            target: user_target(),
            change: HookChange::Install,
            result: Ok(hooks_status(true)),
        },
        now,
    );
    assert!(notice.is_none());
    assert!(!data.is_changing_hooks(&user_target(), HookChange::Install));
    assert!(matches!(data.hooks_status(&user_target()), Status::Ready(status) if status.installed));
    data.change_hooks(project.clone(), HookChange::Install);
    let failed = data.accept(
        Reply::HooksChanged {
            target: project.clone(),
            change: HookChange::Install,
            result: Err(localized("error.agent.synthetic")),
        },
        now,
    );
    assert!(matches!(failed, Some(Notice::Error { fallback: None, .. })));
    assert!(!data.is_changing_hooks(&project, HookChange::Install));
    assert!(matches!(data.hooks_status(&project), Status::Pending));
}

#[test]
fn 시험_알림은_포커스를_싣고_사유별_스위치_라벨과_전달_문구로_알린다() {
    assert_eq!(
        suppression_label(NotificationSuppressionReason::NotificationsDisabled),
        "settings.notificationsEnabled"
    );
    assert_eq!(
        suppression_label(NotificationSuppressionReason::CategoryDisabled),
        "settings.notificationsError"
    );
    assert_eq!(
        suppression_label(NotificationSuppressionReason::WindowFocused),
        "settings.notificationsOnlyWhenUnfocused"
    );
    let mut data = Data::default();
    let now = Instant::now();
    data.observe_window_focus(true);
    data.send_test_notification("title".into(), "body".into());
    data.observe_window_focus(false);
    data.send_test_notification("title".into(), "body".into());
    let focus = data
        .take_requests()
        .into_iter()
        .filter_map(|request| match request {
            Request::Act(Action::SendTestNotification {
                title,
                body,
                any_window_focused,
            }) => Some((title, body, any_window_focused)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        focus,
        vec![
            ("title".into(), "body".into(), true),
            ("title".into(), "body".into(), false)
        ]
    );
    for (reason, label) in [
        (
            NotificationSuppressionReason::NotificationsDisabled,
            "settings.notificationsEnabled",
        ),
        (
            NotificationSuppressionReason::CategoryDisabled,
            "settings.notificationsError",
        ),
        (
            NotificationSuppressionReason::WindowFocused,
            "settings.notificationsOnlyWhenUnfocused",
        ),
    ] {
        assert!(matches!(
            data.accept(
                Reply::TestNotification(Ok(NotificationDelivery::Suppressed(reason))),
                now
            ),
            Some(Notice::Warning(key)) if key == label
        ));
    }
    assert!(matches!(
        data.accept(
            Reply::TestNotification(Ok(NotificationDelivery::Delivered)),
            now
        ),
        Some(Notice::Success("notification.enableHint"))
    ));
    assert!(matches!(
        data.accept(Reply::TestNotification(Err(error())), now),
        Some(Notice::Error { fallback: None, .. })
    ));
    assert!(
        data.accept(Reply::NotificationSettingsOpened(Ok(())), now)
            .is_none()
    );
    assert!(matches!(
        data.accept(Reply::NotificationSettingsOpened(Err(error())), now),
        Some(Notice::Error { fallback: None, .. })
    ));
}

#[test]
fn 성능_읽기는_원본_json_모양으로_요청자에게_보내고_초기화와_실패를_구분한다() {
    let snapshot = PerfSnapshot {
        enabled: true,
        entries: vec![taide_model::app::PerfEntry {
            name: "search.walk".into(),
            count: 1.0,
            total_ms: 0.25,
            max_ms: 12.0,
        }],
        counters: Vec::new(),
    };
    assert_eq!(
        perf_json(&snapshot).unwrap(),
        "{\n  \"enabled\": true,\n  \"entries\": [\n    {\n      \"name\": \"search.walk\",\n      \"count\": 1,\n      \"totalMs\": 0.25,\n      \"maxMs\": 12\n    }\n  ],\n  \"counters\": []\n}"
    );
    let mut data = Data::default();
    let now = Instant::now();
    let ticket = data.next_owner().unwrap();
    let other = data.next_owner().unwrap();
    assert!(
        data.accept(
            Reply::PerfRead {
                ticket,
                result: Ok(snapshot.clone()),
            },
            now,
        )
        .is_none()
    );
    assert!(data.take_owner_events(other).is_empty());
    assert_eq!(
        data.take_owner_events(ticket),
        vec![OwnerEvent::PerfRead(perf_json(&snapshot).unwrap())]
    );
    assert!(
        data.accept(
            Reply::PerfReset {
                ticket,
                result: Ok(()),
            },
            now,
        )
        .is_none()
    );
    assert!(matches!(
        data.accept(
            Reply::PerfRead {
                ticket,
                result: Err(error()),
            },
            now,
        ),
        Some(Notice::Error { .. })
    ));
    assert!(matches!(
        data.accept(
            Reply::PerfReset {
                ticket,
                result: Err(error()),
            },
            now,
        ),
        Some(Notice::Error { .. })
    ));
    assert_eq!(
        data.take_owner_events(ticket),
        vec![
            OwnerEvent::PerfReset,
            OwnerEvent::PerfFailed,
            OwnerEvent::PerfFailed
        ]
    );
}

#[test]
fn s1_제출_실패는_요청별_응답으로_돌아온다() {
    let error = || AppError::Internal("stopped".into());
    assert!(matches!(
        Request::Fetch {
            query: Query::CliStatus,
            generation: 2
        }
        .failed(error()),
        Reply::CliStatus {
            generation: 2,
            result: Err(_)
        }
    ));
    assert!(matches!(
        Request::Fetch {
            query: Query::HooksStatus(user_target()),
            generation: 5
        }
        .failed(error()),
        Reply::HooksStatus {
            generation: 5,
            result: Err(_),
            ..
        }
    ));
    assert!(matches!(
        Request::Act(Action::CliInstall).failed(error()),
        Reply::CliInstalled(Err(_))
    ));
    assert!(matches!(
        Request::Act(Action::CliUninstall).failed(error()),
        Reply::CliUninstalled(Err(_))
    ));
    assert!(matches!(
        Request::Act(Action::Hooks {
            target: user_target(),
            change: HookChange::Uninstall
        })
        .failed(error()),
        Reply::HooksChanged {
            change: HookChange::Uninstall,
            result: Err(_),
            ..
        }
    ));
    assert!(matches!(
        Request::Act(Action::OpenNotificationSettings).failed(error()),
        Reply::NotificationSettingsOpened(Err(_))
    ));
    assert!(matches!(
        Request::Act(Action::SendTestNotification {
            title: String::new(),
            body: String::new(),
            any_window_focused: false
        })
        .failed(error()),
        Reply::TestNotification(Err(_))
    ));
    assert!(matches!(
        Request::Act(Action::PerfRead { ticket: 7 }).failed(error()),
        Reply::PerfRead {
            ticket: 7,
            result: Err(_)
        }
    ));
    assert!(matches!(
        Request::Act(Action::PerfReset { ticket: 8 }).failed(error()),
        Reply::PerfReset {
            ticket: 8,
            result: Err(_)
        }
    ));
}

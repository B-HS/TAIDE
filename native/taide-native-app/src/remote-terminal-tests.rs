use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::terminal::PtyAttachResult;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

use super::*;
use crate::remote_gateway;

const COMMAND_COUNT: usize = 12;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const RESIZED_COLUMNS: u16 = 96;
const RESIZED_ROWS: u16 = 32;
const HISTORY: usize = 128;
const FRAME_BYTES: usize = 256 * 1024;
const WRITER_BYTES: usize = 512;
const QUEUE_COUNT: usize = 64;
const VISITS: usize = 4096;
const LARGE_INPUT_BYTES: usize = WRITER_BYTES + WRITER_BYTES / 2;
const DEADLINE: Duration = Duration::from_secs(5);
const FIRST: &str = "synthetic-first-line\n";
const SECOND: &str = "synthetic-second-line\n";
const QUERY_TITLE: &str = "remote-query-consumed";
const STATUS_QUERY: &str = "\x1b[5n";
const STATUS_REPLY: &str = "\x1b[0n";
const CLIENT_REPLY_END: &str = "remote-client-reply";
const INITIAL_COMMAND_COLOR: &str = "#123456";
const UPDATED_COMMAND_COLOR: &str = "#abcdef";

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[derive(Default)]
struct Channels {
    created: Mutex<Vec<String>>,
    dropped: Mutex<Vec<String>>,
    output: Mutex<Vec<(String, Vec<u8>)>>,
    changed: Notify,
}

struct ChannelOwner {
    id: String,
    channels: Arc<Channels>,
}

impl Drop for ChannelOwner {
    fn drop(&mut self) {
        self.channels.dropped.lock().unwrap().push(self.id.clone());
        self.channels.changed.notify_waiters();
    }
}

impl Channels {
    fn factory(self: &Arc<Self>) -> ChannelFactory {
        let channels = self.clone();
        Arc::new(move |id| {
            channels.created.lock().unwrap().push(id.clone());
            let owner = ChannelOwner {
                id,
                channels: channels.clone(),
            };
            Box::new(move |body| {
                let ResponseBody::Raw(bytes) = body else {
                    panic!("PTY channels must contain raw bytes");
                };
                owner
                    .channels
                    .output
                    .lock()
                    .unwrap()
                    .push((owner.id.clone(), bytes));
                owner.channels.changed.notify_waiters();
                Ok(())
            })
        })
    }

    fn bytes(&self, id: &str) -> Vec<u8> {
        self.output
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name == id)
            .flat_map(|(_, bytes)| bytes.iter().copied())
            .collect()
    }

    async fn contains(&self, id: &str, text: &str) {
        tokio::time::timeout(DEADLINE, async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if String::from_utf8_lossy(&self.bytes(id)).contains(text) {
                    return;
                }
                changed.await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "channel {id} missing {} bytes; observed {:?}",
                text.len(),
                String::from_utf8_lossy(&self.bytes(id))
            )
        });
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
    hub: Arc<Hub>,
    channels: Arc<Channels>,
    environments: Arc<AtomicUsize>,
    query_seen: Arc<Notify>,
    backend: Arc<Dispatch>,
}

impl Fixture {
    fn new() -> Self {
        let project = ProjectId::new();
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-terminal-{project}"));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic remote terminal".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let hub = Arc::new(
            Hub::new(
                services.clone(),
                crate::terminal_host::Limits {
                    sessions: 1,
                    core: Default::default(),
                    frames: crate::terminal_frames::Limits {
                        bytes: FRAME_BYTES,
                        count: QUEUE_COUNT,
                        visits: VISITS,
                    },
                    writer: crate::terminal_writer::Limits {
                        bytes: WRITER_BYTES,
                        count: QUEUE_COUNT,
                    },
                },
            )
            .unwrap(),
        );
        let environments = Arc::new(AtomicUsize::new(0));
        let count = environments.clone();
        let environment: Environment = Arc::new(move |_| {
            count.fetch_add(1, Ordering::AcqRel);
            Box::pin(async { Vec::new() })
        });
        let query_seen = Arc::new(Notify::new());
        let backend = Self::backend(hub.clone(), environment, query_seen.clone());
        Self {
            directory,
            root,
            project,
            services,
            sink,
            hub,
            channels: Arc::default(),
            environments,
            query_seen,
            backend,
        }
    }

    fn backend(hub: Arc<Hub>, environment: Environment, query_seen: Arc<Notify>) -> Arc<Dispatch> {
        Arc::new(remote_gateway::with_policy(extend_backend(
            Ports {
                terminals: hub,
                environment,
                history: Arc::new(|_, _| HISTORY),
                effects: Arc::new(move |_, _| {
                    let query_seen = query_seen.clone();
                    Ok(ObservePorts {
                        command_colors: Default::default(),
                        updated: Arc::new(|| {}),
                        event: Arc::new(|_| Ok(())),
                        stream: Arc::new(move |event| {
                            if matches!(event, taide_infra::terminal_scan::ScanEvent::Title(title) if title == QUERY_TITLE)
                            {
                                query_seen.notify_one();
                            }
                            Ok(())
                        }),
                    })
                }),
            },
            Dispatch {
                json: Arc::new(|_, name, args, _| {
                    Box::pin(async move {
                        assert_eq!(name, "settings_get");
                        Ok(args.to_string())
                    })
                }),
                raw: Arc::new(|_, name, _| {
                    Box::pin(async move {
                        assert_eq!(name, "file_read_raw");
                        Ok(vec![0])
                    })
                }),
            },
        )))
    }

    fn opts(&self) -> PtySpawnOptions {
        PtySpawnOptions {
            project_id: self.project.clone(),
            cwd: self.root.to_str().unwrap().into(),
            shell: Some("/bin/cat".into()),
            cols: COLUMNS,
            rows: ROWS,
            scrollback_bytes: None,
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let text = (self.backend.json)(
            self.services.clone(),
            name.into(),
            args,
            self.channels.factory(),
        )
        .await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    async fn spawn(&self) -> String {
        self.ok(
            "pty_spawn",
            json!({"opts": self.opts(), "onData":"__CHANNEL__:initial"}),
        )
        .await
        .as_str()
        .unwrap()
        .into()
    }

    async fn attach(&self, id: &str, channel: &str) -> PtyAttachResult {
        serde_json::from_value(
            self.ok("pty_attach", json!({"sessionId":id, "onData":channel}))
                .await,
        )
        .unwrap()
    }

    async fn finish(&self) {
        for session in self.services.terminal.sessions_for_project(&self.project) {
            self.ok("pty_kill", json!({"sessionId":session.id})).await;
        }
        tokio::time::timeout(DEADLINE, self.services.terminal.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for session in self.services.terminal.sessions_for_project(&self.project) {
            self.hub.discard(&session.id);
        }
        self.services.terminal.kill_all();
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn catalog_입력_channel_정책과_remaining을_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMAND_COUNT);
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    let previous: BTreeSet<_> = [
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_projects::COMMANDS,
        crate::remote_search::COMMANDS,
        crate::remote_plugins::COMMANDS,
        crate::remote_git::COMMANDS,
        crate::remote_agents::COMMANDS,
        crate::remote_lsp::COMMANDS,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();
    assert!(declared.is_disjoint(&previous));
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-terminal.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new();
    for (name, args) in [
        ("pty_spawn", json!({"opts":fixture.opts()})),
        ("pty_attach", json!({"sessionId":"missing"})),
    ] {
        let error = fixture.call(name, args).await.unwrap_err();
        assert_eq!(error["code"], "Localized");
        assert_eq!(error["message"]["kind"], "InvalidArgument");
        assert_eq!(error["message"]["key"], "error.remote.channelArgRequired");
    }
    for (name, args) in [
        ("pty_spawn", json!({"opts":false,"onData":"initial"})),
        (
            "pty_resize",
            json!({"sessionId":"missing","cols":-1,"rows":ROWS}),
        ),
        ("pty_write", json!({"sessionId":"missing","data":false})),
    ] {
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["code"],
            "InvalidArgument"
        );
    }
    assert_eq!(fixture.environments.load(Ordering::Acquire), 0);
    for name in [
        "pty_write",
        "pty_resize",
        "pty_kill",
        "pty_set_paused",
        "pty_attach",
    ] {
        let args = json!({"sessionId":"missing","data":"","cols":COLUMNS,"rows":ROWS,
            "paused":true,"onData":"missing"});
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["code"],
            "NotFound"
        );
    }
    assert_eq!(
        fixture
            .ok(
                "pty_detach",
                json!({"sessionId":"missing","subscriptionId":0})
            )
            .await,
        Value::Null
    );
    assert_eq!(
        fixture
            .ok("terminal_sessions", json!({"projectId":ProjectId::new()}))
            .await,
        json!([])
    );
    assert_eq!(
        fixture.ok("settings_get", json!({"owner":"main"})).await["owner"],
        "remote"
    );
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            Value::Null
        )
        .await
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        fixture.call("app_exit", json!({})).await.unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert!(fixture.sink.0.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn 원격_query는_renderer_응답만_전송하고_서버에서_중복생성하지_않는다() {
    let fixture = Fixture::new();
    let session = fixture.spawn().await;
    fixture.attach(&session, "query-output").await;
    fixture
        .ok(
            "pty_write",
            json!({
                "sessionId": session,
                "data": format!("{STATUS_QUERY}\x1b]2;{QUERY_TITLE}\x07\n"),
            }),
        )
        .await;
    tokio::time::timeout(DEADLINE, fixture.query_seen.notified())
        .await
        .unwrap();
    fixture
        .channels
        .contains("query-output", STATUS_QUERY)
        .await;
    fixture
        .ok(
            "pty_write",
            json!({
                "sessionId": session,
                "data": format!("{STATUS_REPLY}{CLIENT_REPLY_END}\n"),
            }),
        )
        .await;
    fixture
        .channels
        .contains("query-output", &format!("{STATUS_REPLY}{CLIENT_REPLY_END}"))
        .await;
    let output = fixture.channels.bytes("query-output");
    let replies = output
        .windows(STATUS_REPLY.len())
        .filter(|bytes| *bytes == STATUS_REPLY.as_bytes())
        .count();
    fixture.finish().await;
    assert_eq!(
        replies, 1,
        "server Core added a duplicate renderer query reply"
    );
}

#[tokio::test]
async fn 실제_root_path_link_default와_shell_profile을_보존한다() {
    let fixture = Fixture::new();
    let file = fixture.root.join("synthetic.txt");
    let sub = fixture.root.join("nested");
    let outside = fixture.directory.join("outside.txt");
    std::fs::write(&file, "synthetic").unwrap();
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(&outside, "outside").unwrap();
    fixture.services.state.settings.write().shell_override = Some("synthetic-shell".into());
    let opts: PtySpawnOptions = serde_json::from_value(
        fixture
            .ok("pty_default_options", json!({"projectId":fixture.project}))
            .await,
    )
    .unwrap();
    assert_eq!(opts.cwd, fixture.root.to_str().unwrap());
    assert_eq!((opts.cols, opts.rows), (COLUMNS, ROWS));
    assert_eq!(opts.shell.as_deref(), Some("synthetic-shell"));
    assert_eq!(opts.scrollback_bytes, None);
    assert_eq!(
        fixture
            .ok(
                "pty_default_options",
                json!({"projectId":fixture.project,"cwd":sub})
            )
            .await["cwd"],
        sub.to_str().unwrap()
    );
    assert!(
        fixture
            .call(
                "pty_default_options",
                json!({"projectId":fixture.project,"cwd":fixture.directory})
            )
            .await
            .is_err()
    );
    assert_eq!(
        fixture
            .call("pty_default_options", json!({"projectId":ProjectId::new()}))
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    assert_eq!(
        fixture
            .ok(
                "resolve_terminal_path",
                json!({"path":"synthetic.txt","cwd":fixture.root})
            )
            .await,
        json!(file)
    );
    assert!(
        fixture
            .call(
                "resolve_terminal_path",
                json!({"path":outside,"cwd":fixture.root})
            )
            .await
            .is_err()
    );
    let candidates = fixture
        .ok(
            "terminal_resolve_link_candidates",
            json!({"cwd":fixture.root,
        "candidates":["synthetic.txt","missing.txt","../outside.txt"]}),
        )
        .await;
    assert_eq!(candidates, json!([file, null, null]));
    let profiles: Vec<taide_model::terminal::ShellProfile> =
        serde_json::from_value(fixture.ok("shell_profiles", json!({})).await).unwrap();
    assert_eq!(profiles, taide_terminal::service::list_shell_profiles());
    assert_eq!(fixture.environments.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn production_views_effects는_palette_실패를_spawn_전에_거절하고_현재색과_repaint를_공유한다()
{
    let mut fixture = Fixture::new();
    let context = eframe::egui::Context::default();
    let mut views = crate::terminal_surface::Views::default();
    let effects = views.remote_effects(&context);
    let environments = fixture.environments.clone();
    let previous = fixture.backend.clone();
    fixture.backend = Arc::new(extend_backend(
        Ports {
            terminals: fixture.hub.clone(),
            environment: Arc::new(move |_| {
                environments.fetch_add(1, Ordering::AcqRel);
                Box::pin(async { Vec::new() })
            }),
            history: Arc::new(|_, _| HISTORY),
            effects: effects.clone(),
        },
        Dispatch {
            json: previous.json.clone(),
            raw: previous.raw.clone(),
        },
    ));
    assert!(
        fixture
            .call(
                "pty_spawn",
                json!({"opts":fixture.opts(),"onData":"__CHANNEL__:unready"})
            )
            .await
            .is_err()
    );
    assert_eq!(fixture.environments.load(Ordering::Acquire), 0);
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    assert_eq!(*fixture.channels.dropped.lock().unwrap(), ["unready"]);
    let mut theme =
        taide_runtime::theme_actions::theme_get_current(&fixture.services.state, "dark").unwrap();
    theme.colors.insert(
        "statusIndicator.success".into(),
        INITIAL_COMMAND_COLOR.into(),
    );
    let appearance = crate::terminal_surface::Appearance::new(
        &theme,
        fixture.services.state.settings.read().terminal_font_size,
    )
    .unwrap();
    views.set_palette(&appearance).unwrap();
    let initial = effects(&fixture.services, &fixture.opts()).unwrap();
    let initial_rgb =
        crate::presentation::parse_color(INITIAL_COMMAND_COLOR, "statusIndicator.success").unwrap();
    assert_eq!(
        initial.command_colors.success,
        Some(taide_native_terminal::Rgb {
            r: initial_rgb.r(),
            g: initial_rgb.g(),
            b: initial_rgb.b()
        })
    );
    let id = fixture.spawn().await;
    fixture.attach(&id, "production-output").await;
    fixture
        .ok("pty_write", json!({"sessionId":id,"data":FIRST}))
        .await;
    fixture
        .channels
        .contains("production-output", FIRST.trim())
        .await;
    assert_eq!(fixture.environments.load(Ordering::Acquire), 1);
    theme.colors.insert(
        "statusIndicator.success".into(),
        UPDATED_COMMAND_COLOR.into(),
    );
    let appearance = crate::terminal_surface::Appearance::new(
        &theme,
        fixture.services.state.settings.read().terminal_font_size,
    )
    .unwrap();
    views.set_palette(&appearance).unwrap();
    let updated = effects(&fixture.services, &fixture.opts()).unwrap();
    let updated_rgb =
        crate::presentation::parse_color(UPDATED_COMMAND_COLOR, "statusIndicator.success").unwrap();
    assert_eq!(
        updated.command_colors.success,
        Some(taide_native_terminal::Rgb {
            r: updated_rgb.r(),
            g: updated_rgb.g(),
            b: updated_rgb.b()
        })
    );
    assert_ne!(
        initial.command_colors.success,
        updated.command_colors.success
    );
    drop(views);
    (updated.updated)();
    assert!(context.has_requested_repaint());
    assert_eq!(
        effects(&fixture.services, &fixture.opts())
            .unwrap()
            .command_colors
            .success,
        updated.command_colors.success
    );
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn 실제_single_core_pty의_raw_replay_write_resize_pause_detach_kill을_검증한다() {
    use taide_native_terminal::GridDimensions;
    let fixture = Fixture::new();
    let id = fixture.spawn().await;
    let session = fixture.hub.get(&id).unwrap();
    assert_eq!(fixture.environments.load(Ordering::Acquire), 1);
    assert_eq!(*fixture.channels.created.lock().unwrap(), ["initial"]);
    assert_eq!(*fixture.channels.dropped.lock().unwrap(), ["initial"]);
    assert!(fixture.channels.bytes("initial").is_empty());
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    fixture
        .ok("pty_write", json!({"sessionId":id,"data":FIRST}))
        .await;
    tokio::time::timeout(DEADLINE, async {
        loop {
            if session.snapshot(|state| state.revision > 0).unwrap() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        session
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch > epoch)
            .unwrap()
    );
    let attached = fixture.attach(&id, "__CHANNEL__:live").await;
    fixture.channels.contains("live", FIRST.trim_end()).await;
    let initial = fixture.channels.bytes("live");
    assert!(initial.starts_with(taide_terminal::session::TERMINAL_REPLAY_PREAMBLE));
    assert!(usize::try_from(attached.replay_bytes).unwrap() <= initial.len());
    fixture
        .ok("pty_write", json!({"sessionId":id,"data":SECOND}))
        .await;
    fixture.channels.contains("live", SECOND.trim_end()).await;
    fixture
        .ok("pty_write", json!({"sessionId":id,"data":""}))
        .await;
    let large = format!("{}\n", "x".repeat(LARGE_INPUT_BYTES));
    fixture
        .ok("pty_write", json!({"sessionId":id,"data":large}))
        .await;
    fixture.channels.contains("live", large.trim_end()).await;
    fixture
        .ok(
            "pty_resize",
            json!({"sessionId":id,"cols":RESIZED_COLUMNS,"rows":RESIZED_ROWS}),
        )
        .await;
    let geometry = session
        .snapshot(|state| {
            let grid = state.core.grid().unwrap();
            (grid.columns(), grid.screen_lines())
        })
        .unwrap();
    assert_eq!(
        geometry,
        (usize::from(RESIZED_COLUMNS), usize::from(RESIZED_ROWS))
    );
    fixture
        .ok("pty_set_paused", json!({"sessionId":id,"paused":true}))
        .await;
    fixture
        .ok("pty_set_paused", json!({"sessionId":id,"paused":false}))
        .await;
    fixture
        .ok(
            "pty_detach",
            json!({"sessionId":id,"subscriptionId":attached.subscription_id}),
        )
        .await;
    assert!(
        fixture
            .channels
            .dropped
            .lock()
            .unwrap()
            .iter()
            .any(|id| id == "live")
    );
    let detached = fixture.channels.bytes("live");
    fixture.attach(&id, "second").await;
    fixture
        .ok("pty_write", json!({"sessionId":id,"data":"after-detach\n"}))
        .await;
    fixture.channels.contains("second", "after-detach").await;
    assert_eq!(fixture.channels.bytes("live"), detached);
    let sessions = fixture
        .ok("terminal_sessions", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(sessions.as_array().unwrap().len(), 1);
    assert_eq!(sessions[0]["id"], id);
    assert_eq!(sessions[0]["shell"], "/bin/cat");
    assert_eq!(sessions[0]["running"], true);
    fixture.ok("pty_kill", json!({"sessionId":id})).await;
    assert!(fixture.hub.get(&id).is_none());
    assert_eq!(
        fixture
            .ok("terminal_sessions", json!({"projectId":fixture.project}))
            .await,
        json!([])
    );
    fixture.finish().await;
    assert!(session.is_finished());
    assert_eq!(session.failure(), None);
    assert!(
        matches!(&fixture.sink.0.lock().unwrap()[0], AppEvent::TerminalSpawned { session_id, .. } if session_id == &id)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn env_대기_취소와_shutdown_gate_입력_waiter_취소의_실제_worker를_검증한다() {
    use crate::terminal_host::InputResult;
    use taide_native_terminal::input::NativeInput;
    let fixture = Fixture::new();
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let environment: Environment = {
        let started = started.clone();
        let release = release.clone();
        Arc::new(move |_| {
            let started = started.clone();
            let release = release.clone();
            Box::pin(async move {
                started.notify_one();
                release.notified().await;
                Vec::new()
            })
        })
    };
    let backend = Fixture::backend(fixture.hub.clone(), environment, fixture.query_seen.clone());
    let mut pending = Box::pin((backend.json)(
        fixture.services.clone(),
        "pty_spawn".into(),
        json!({"opts":fixture.opts(),"onData":"cancelled"}),
        fixture.channels.factory(),
    ));
    tokio::select! {
        result = &mut pending => panic!("environment must block: {result:?}"),
        () = started.notified() => {}
    }
    assert!(fixture.channels.dropped.lock().unwrap().is_empty());
    drop(pending);
    assert_eq!(*fixture.channels.dropped.lock().unwrap(), ["cancelled"]);
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    let mut invalid = fixture.opts();
    invalid.project_id = ProjectId::new();
    assert_eq!(
        fixture
            .call("pty_spawn", json!({"opts":invalid,"onData":"bad-project"}))
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    let id = fixture.spawn().await;
    fixture.attach(&id, "output").await;
    let session = fixture.hub.get(&id).unwrap();
    let InputResult::Pending(first) = session
        .queue_input(
            &fixture.services,
            NativeInput::CommittedText(FIRST),
            FIRST.len(),
            true,
        )
        .unwrap()
    else {
        panic!("first input must retain its order");
    };
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let mut write = Box::pin(fixture.call("pty_write", json!({"sessionId":id,"data":SECOND})));
    tokio::select! {
        result = &mut write => panic!("retained order must block write: {result:?}"),
        () = async {
            tokio::time::timeout(DEADLINE, async {
                loop {
                    if session.snapshot(|state| state.core.selection_stamp().unwrap().input_epoch > epoch).unwrap() {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            }).await.unwrap();
        } => {}
    }
    drop(write);
    let InputResult::Write(receipt) = session.retry_input(&fixture.services, first).unwrap() else {
        panic!("first input must enqueue");
    };
    receipt.wait().await.unwrap();
    fixture.channels.contains("output", SECOND.trim_end()).await;
    let bytes = String::from_utf8(fixture.channels.bytes("output")).unwrap();
    assert!(bytes.find(FIRST.trim_end()).unwrap() < bytes.find(SECOND.trim_end()).unwrap());
    fixture.ok("pty_kill", json!({"sessionId":id})).await;
    tokio::time::timeout(DEADLINE, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    drop(session);
    tokio::time::timeout(DEADLINE, fixture.services.terminal.wait_for_idle())
        .await
        .unwrap()
        .unwrap();
    fixture.services.state.begin_shutdown();
    fixture
        .ok("terminal_sessions", json!({"projectId":fixture.project}))
        .await;
    fixture
        .ok("pty_default_options", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(
        fixture
            .call(
                "pty_spawn",
                json!({"opts":fixture.opts(),"onData":"shutdown"})
            )
            .await
            .unwrap_err()["code"],
        "Forbidden"
    );
    fixture.finish().await;
}

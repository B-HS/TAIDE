use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::search::SearchFileMatches;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppServices, AppState, EventSink, TaskSupervisor};

use crate::remote_gateway;
use crate::remote_ws::{ChannelFactory, Dispatch, ResponseBody};

const DEADLINE: Duration = Duration::from_secs(5);
const FIRST_TEXT: &str = "prefix\n\u{10400} needle\nsuffix\n";
const SECOND_TEXT: &str = "needle needle\n";
const GRAMMAR: &str = r#"{"scopeName":"source.synthetic","patterns":[]}"#;
const UPDATED_GRAMMAR: &str = r#"{"scopeName":"source.synthetic","patterns":[{"match":"needle"}]}"#;
const CHANNEL: &str = "42";
const MATCH_COUNT: u32 = 3;
const SUPPLEMENTARY_COLUMN: u32 = 4;
const SUPPLEMENTARY_MATCH_START: u32 = 3;
const SUPPLEMENTARY_MATCH_END: u32 = 9;

struct Sink;

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        panic!("search/plugin original commands have no application events: {event:?}");
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    backend: Dispatch,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-content-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project/nested")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        std::fs::write(root.join("first.txt"), FIRST_TEXT).unwrap();
        std::fs::write(root.join("nested/second.txt"), SECOND_TEXT).unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic search".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        );
        let remaining = Dispatch {
            json: Arc::new(|_, name, args, channels| {
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    channels(CHANNEL.into())(ResponseBody::Json("null".into())).unwrap();
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(|_, name, _| {
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    Ok(vec![0])
                })
            }),
        };
        let backend = remote_gateway::with_policy(crate::remote_search::extend_backend(
            crate::remote_plugins::extend_backend(remaining),
        ));
        Self {
            directory,
            root,
            project,
            services,
            backend,
        }
    }

    async fn call_channels(
        &self,
        name: &str,
        args: Value,
        channels: ChannelFactory,
    ) -> Result<Value, Value> {
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        self.call_channels(
            name,
            args,
            Arc::new(|_| Box::new(|_| panic!("unexpected channel"))),
        )
        .await
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    fn search_args(&self) -> Value {
        json!({"projectId":self.project,"owner":"main","sessionId":"panel", "query":{"text":"needle","contextLines":1},"onMatch":"__CHANNEL__:42"})
    }

    fn plugin(&self, name: &str, grammar: &str) -> PathBuf {
        let root = self.services.state.paths.plugins_dir().join("synthetic");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("grammar.json"), grammar).unwrap();
        std::fs::write(root.join("taide-plugin.json"), json!({"manifestVersion":1,"id":"synthetic", "name":name,"version":"1.0.0", "contributes":{"languages":[{"id":"synthetic","extensions":[".synthetic"],"grammar":"grammar.json"}]}}).to_string()).unwrap();
        root
    }

    async fn finish(&self) {
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

struct ChannelDrop(Arc<AtomicUsize>);

impl Drop for ChannelDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

#[tokio::test]
async fn catalog7명령의_actual_arm_정책_인자와_remaining을_보존한다() {
    let previous = [
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_projects::COMMANDS,
    ];
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    for (commands, source) in [
        (
            crate::remote_search::COMMANDS,
            include_str!("remote-search.rs"),
        ),
        (
            crate::remote_plugins::COMMANDS,
            include_str!("remote-plugins.rs"),
        ),
    ] {
        let declared: BTreeSet<_> = commands.iter().copied().collect();
        assert_eq!(declared.len(), commands.len());
        assert!(
            declared
                .iter()
                .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
        );
        assert!(
            previous
                .iter()
                .all(|list| declared.iter().all(|name| !list.contains(name)))
        );
        let arms: BTreeSet<_> = pattern
            .captures_iter(source)
            .map(|capture| capture.get(1).unwrap().as_str())
            .collect();
        assert_eq!(declared, arms);
    }
    let fixture = Fixture::new();
    for name in ["plugin_install", "plugin_uninstall", "synthetic_unknown"] {
        assert_eq!(
            fixture.call(name, Value::Null).await.unwrap_err()["message"]["kind"],
            "Forbidden"
        );
    }
    assert!(fixture.services.plugin.0.read().is_none());
    assert_eq!(
        fixture
            .call(
                "plugin_read_grammar",
                json!({"pluginId":0,"languageId":"synthetic"})
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert!(fixture.services.plugin.0.read().is_none());
    let mut args = fixture.search_args();
    args.as_object_mut().unwrap().remove("onMatch");
    let error = fixture.call("search_run", args).await.unwrap_err();
    assert_eq!(error["message"]["key"], "error.remote.channelArgRequired");
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    let called = Arc::new(AtomicUsize::new(0));
    let observed = called.clone();
    let channels: ChannelFactory = Arc::new(move |id| {
        assert_eq!(id, CHANNEL);
        observed.fetch_add(1, Ordering::AcqRel);
        Box::new(|body| {
            assert!(matches!(body, ResponseBody::Json(text) if text == "null"));
            Ok(())
        })
    });
    assert_eq!(
        fixture
            .call_channels("layout_get", json!({"owner":"main"}), channels)
            .await
            .unwrap()["owner"],
        "remote"
    );
    assert_eq!(called.load(Ordering::Acquire), 1);
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
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_검색_channel_utf16_문맥과_목록_치환_원본_파일을_보존한다() {
    let fixture = Fixture::new();
    let batches = Arc::new(Mutex::new(Vec::<SearchFileMatches>::new()));
    let dropped = Arc::new(AtomicUsize::new(0));
    let collected = batches.clone();
    let ended = dropped.clone();
    let channels: ChannelFactory = Arc::new(move |id| {
        assert_eq!(id, CHANNEL);
        let collected = collected.clone();
        let end = ChannelDrop(ended.clone());
        Box::new(move |body| {
            let _ = &end;
            let ResponseBody::Json(text) = body else {
                panic!("search must send JSON")
            };
            collected
                .lock()
                .unwrap()
                .push(serde_json::from_str(&text).unwrap());
            Ok(())
        })
    });
    assert_eq!(
        fixture
            .call_channels("search_run", fixture.search_args(), channels)
            .await
            .unwrap(),
        json!(MATCH_COUNT)
    );
    assert_eq!(dropped.load(Ordering::Acquire), 1);
    {
        let batches = batches.lock().unwrap();
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.matches.len())
                .sum::<usize>(),
            usize::try_from(MATCH_COUNT).unwrap()
        );
        let first = batches
            .iter()
            .find(|batch| batch.path == fixture.root.join("first.txt").to_str().unwrap())
            .unwrap();
        let item = &first.matches[0];
        assert_eq!(item.column, SUPPLEMENTARY_COLUMN);
        assert_eq!(item.match_start, SUPPLEMENTARY_MATCH_START);
        assert_eq!(item.match_end, SUPPLEMENTARY_MATCH_END);
        assert_eq!(item.before, vec!["prefix"]);
        assert_eq!(item.after, vec!["suffix"]);
    }
    let files: Vec<String> = serde_json::from_value(
        fixture
            .ok("search_list_files", json!({"projectId":fixture.project}))
            .await,
    )
    .unwrap();
    assert_eq!(
        files.into_iter().collect::<BTreeSet<_>>(),
        [
            fixture.root.join("first.txt"),
            fixture.root.join("nested/second.txt")
        ]
        .into_iter()
        .map(|path| path.to_str().unwrap().to_owned())
        .collect()
    );
    let outside = fixture.directory.join("outside.txt");
    std::fs::write(&outside, "needle").unwrap();
    let result = fixture.ok("search_replace", json!({"projectId":fixture.project,"query":{"text":"needle"},"replacement":"new", "paths":[fixture.root.join("first.txt"), outside]})).await;
    assert_eq!(result["changedFiles"], 1);
    assert_eq!(result["replacedMatches"], 1);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("first.txt")).unwrap(),
        "prefix\n\u{10400} new\nsuffix\n"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("nested/second.txt")).unwrap(),
        SECOND_TEXT
    );
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "needle");
    let resolved = taide_infra::self_write::resolve_from_app(
        &fixture.services.state.self_writes,
        [fixture.root.join("first.txt"), outside]
            .into_iter()
            .map(|path| FsChange {
                kind: FsChangeKind::Modified,
                paths: vec![path.to_str().unwrap().into()],
                from_app: false,
            })
            .collect(),
    );
    assert!(resolved[0].from_app);
    assert!(!resolved[1].from_app);
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_remote_owner_취소_channel_실패_regex와_worker_거절을_보존한다() {
    let fixture = Fixture::new();
    let desktop = fixture.services.search.begin("main", "panel");
    let remote = fixture.services.search.begin("remote", "panel");
    fixture
        .ok("search_cancel", json!({"owner":"main","sessionId":"panel"}))
        .await;
    assert!(remote.load(Ordering::Acquire));
    assert!(!desktop.load(Ordering::Acquire));
    let channels: ChannelFactory = Arc::new(|id| {
        assert_eq!(id, CHANNEL);
        Box::new(|_| Err(AppError::Internal("synthetic delivery rejected".into())))
    });
    let mut args = fixture.search_args();
    args["onMatch"] = json!(CHANNEL);
    assert_eq!(
        fixture
            .call_channels("search_run", args, channels)
            .await
            .unwrap(),
        json!(MATCH_COUNT)
    );
    assert!(!desktop.load(Ordering::Acquire));
    let closed = fixture
        .call("search_list_files", json!({"projectId":ProjectId::new()}))
        .await
        .unwrap_err();
    assert_eq!(closed["code"], "NotFound");
    let regex = fixture.call("search_replace", json!({"projectId":fixture.project,"query":{"text":"[","regex":true},"replacement":"new"})).await.unwrap_err();
    assert_eq!(regex["message"]["key"], "error.search.invalidRegex");
    fixture.services.tasks.shutdown().await;
    for (name, args) in [
        ("search_list_files", json!({"projectId":fixture.project})),
        (
            "search_replace",
            json!({"projectId":fixture.project,"query":{"text":"needle"},"replacement":"new"}),
        ),
    ] {
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["code"],
            "Forbidden"
        );
    }
    let channels: ChannelFactory =
        Arc::new(|_| Box::new(|_| panic!("stopped search must not send")));
    assert_eq!(
        fixture
            .call_channels("search_run", fixture.search_args(), channels)
            .await
            .unwrap_err()["code"],
        "Forbidden"
    );
    fixture
        .ok("search_cancel", json!({"owner":"main","sessionId":"panel"}))
        .await;
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("first.txt")).unwrap(),
        FIRST_TEXT
    );
    fixture.services.search.finish("main", "panel", &desktop);
    fixture.services.search.finish("remote", "panel", &remote);
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_plugin3명령의_캐시_reload_grammar와_읽기시점_root_guard를_보존한다() {
    let fixture = Fixture::new();
    assert_eq!(fixture.ok("plugin_list", Value::Null).await, json!([]));
    let root = fixture.plugin("Original", GRAMMAR);
    assert_eq!(fixture.ok("plugin_list", Value::Null).await, json!([]));
    let original = fixture.ok("plugin_reload", Value::Null).await;
    assert_eq!(original[0]["enabled"], true);
    assert_eq!(original[0]["manifest"]["name"], "Original");
    assert_eq!(
        fixture
            .ok(
                "plugin_read_grammar",
                json!({"pluginId":"synthetic","languageId":"synthetic"})
            )
            .await,
        GRAMMAR
    );
    fixture.plugin("Updated", UPDATED_GRAMMAR);
    assert_eq!(fixture.ok("plugin_list", Value::Null).await, original);
    assert_eq!(
        fixture.ok("plugin_reload", Value::Null).await[0]["manifest"]["name"],
        "Updated"
    );
    assert_eq!(
        fixture
            .ok(
                "plugin_read_grammar",
                json!({"pluginId":"synthetic","languageId":"synthetic"})
            )
            .await,
        UPDATED_GRAMMAR
    );
    assert_eq!(
        fixture
            .call(
                "plugin_read_grammar",
                json!({"pluginId":"absent","languageId":"synthetic"})
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    assert_eq!(
        fixture
            .call(
                "plugin_read_grammar",
                json!({"pluginId":"synthetic","languageId":"absent"})
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    #[cfg(unix)]
    {
        let outside = fixture.directory.join("outside-grammar.json");
        std::fs::write(&outside, GRAMMAR).unwrap();
        std::fs::remove_file(root.join("grammar.json")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("grammar.json")).unwrap();
        let error = fixture
            .call(
                "plugin_read_grammar",
                json!({"pluginId":"synthetic","languageId":"synthetic"}),
            )
            .await
            .unwrap_err();
        assert_eq!(error["message"]["key"], "error.plugin.pathOutsideRoot");
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), GRAMMAR);
    }
    fixture.services.tasks.shutdown().await;
    assert_eq!(
        fixture.ok("plugin_list", Value::Null).await[0]["enabled"],
        true
    );
    fixture.finish().await;
}

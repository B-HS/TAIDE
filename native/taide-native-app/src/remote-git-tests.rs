use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{OnceLock, Weak};
use std::time::Duration;

use serde_json::json;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

use super::*;
use crate::remote_gateway;
use crate::remote_ws::{ChannelFactory, ResponseBody};

const DEADLINE: Duration = Duration::from_secs(5);
const FILE: &str = "file.synthetic";
const ORIGINAL: &str = "one\ntwo\nthree\n";
const CHANGED: &str = "one\nchanged\nthree\n";
const OURS: &str = "one\nours\nthree\n";
const RESOLVED: &str = "one\nresolved\nthree\n";
const AUTHOR: &str = "Synthetic Test";
const AUTHOR_EMAIL: &str = "synthetic@example.invalid";
const LINE: u32 = 2;
const TAKE: u32 = 10;

#[derive(Default)]
struct Sink {
    events: Mutex<Vec<AppEvent>>,
    services: OnceLock<Weak<AppServices>>,
    listening: AtomicBool,
    installations: AtomicUsize,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        if self.listening.load(Ordering::Acquire) {
            let project = match &event {
                AppEvent::FsChanged { project_id, .. }
                | AppEvent::GitStatusChanged { project_id }
                | AppEvent::GitRefsChanged { project_id } => Some(project_id),
                _ => None,
            };
            if let Some(project) = project {
                self.services
                    .get()
                    .unwrap()
                    .upgrade()
                    .unwrap()
                    .git
                    .invalidate_status(project);
            }
        }
        self.events.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
    empty: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
    backend: Dispatch,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-git-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        std::fs::create_dir_all(directory.join("empty")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let empty = directory.join("empty");
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic remote git".into(),
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
        sink.services.set(Arc::downgrade(&services)).unwrap();
        let listener = sink.clone();
        let ports = Ports {
            install_status_invalidation: Arc::new(move |services| {
                assert_eq!(
                    services.state.paths.data_dir,
                    listener
                        .services
                        .get()
                        .unwrap()
                        .upgrade()
                        .unwrap()
                        .state
                        .paths
                        .data_dir
                );
                listener.installations.fetch_add(1, Ordering::AcqRel);
                listener.listening.store(true, Ordering::Release);
            }),
        };
        let remaining = Dispatch {
            json: Arc::new(|_, name, args, channels| {
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    channels("1".into())(ResponseBody::Json("null".into())).unwrap();
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
        let fixture = Self {
            directory,
            root,
            empty,
            project,
            services,
            sink,
            backend: remote_gateway::with_policy(extend_backend(ports, remaining)),
        };
        fixture.git(&[
            "init",
            "--initial-branch=main",
            "--template",
            fixture.empty.to_str().unwrap(),
        ]);
        for (key, value) in [
            ("user.name", AUTHOR),
            ("user.email", AUTHOR_EMAIL),
            ("core.hooksPath", fixture.empty.to_str().unwrap()),
            ("init.templateDir", fixture.empty.to_str().unwrap()),
            ("core.fsmonitor", "false"),
            ("core.attributesFile", "/dev/null"),
            ("core.excludesFile", "/dev/null"),
            ("commit.gpgSign", "false"),
            ("tag.gpgSign", "false"),
            ("credential.helper", ""),
            ("credential.interactive", "false"),
            ("protocol.allow", "never"),
            ("protocol.file.allow", "always"),
            ("pull.rebase", "false"),
            ("push.default", "upstream"),
        ] {
            fixture.git(&["config", "--local", key, value]);
        }
        std::fs::write(fixture.root.join(FILE), ORIGINAL).unwrap();
        fixture.git(&["add", "--", FILE]);
        fixture.git(&["commit", "-m", "test(fixture): initial"]);
        fixture
    }

    fn output(&self, args: &[&str]) -> Output {
        Command::new("git")
            .current_dir(&self.root)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .arg("-c")
            .arg(format!("core.hooksPath={}", self.empty.display()))
            .arg("-c")
            .arg("core.fsmonitor=false")
            .args(args)
            .output()
            .unwrap()
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self.output(args);
        assert!(
            output.status.success(),
            "synthetic git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    async fn call(&self, name: &str, mut args: Value) -> Result<Value, Value> {
        if let Some(fields) = args.as_object_mut() {
            fields.entry("projectId").or_insert(json!(self.project));
        }
        let channels: ChannelFactory = Arc::new(|_| Box::new(|_| panic!("git has no channels")));
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    async fn status(&self) -> Value {
        self.ok("git_status", json!({})).await
    }

    async fn commit(&self, text: &str, message: &str) -> String {
        std::fs::write(self.root.join(FILE), text).unwrap();
        self.ok("git_stage", json!({"paths":[FILE]})).await;
        serde_json::from_value(
            self.ok("git_commit", json!({"message":message,"opts":{}}))
                .await,
        )
        .unwrap()
    }

    async fn diff(&self, mode: &str) -> Value {
        self.ok("git_diff_file", json!({"path":FILE,"mode":mode}))
            .await
    }

    fn notify_file(&self) {
        self.sink.publish(AppEvent::FsChanged {
            project_id: self.project.clone(),
            change: FsChange {
                kind: FsChangeKind::Modified,
                paths: vec![self.root.join(FILE).to_str().unwrap().into()],
                from_app: false,
            },
        });
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

fn valid_args(project: &ProjectId) -> Value {
    json!({"projectId":project,"path":FILE,"paths":[FILE],"mode":"workdirVsIndex","beforePath":null,
        "rev":"HEAD","skip":0,"take":TAKE,"from":1,"to":LINE,"message":"test(fixture): synthetic","opts":{},
        "name":"synthetic","checkout":false,"force":false,"index":0,"hunkStart":LINE,"hunkEnd":LINE,
        "lineStart":LINE,"lineEnd":LINE,"content":RESOLVED,"target":"HEAD","remoteRef":"origin/synthetic"})
}

#[tokio::test]
async fn catalog41_actual_arm과_입력_remaining_없는프로젝트와_worker_거절을_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    for previous in [
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_projects::COMMANDS,
        crate::remote_search::COMMANDS,
        crate::remote_plugins::COMMANDS,
    ] {
        assert!(declared.iter().all(|name| !previous.contains(name)));
    }
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let source = include_str!("remote-git.rs");
    let arms: BTreeSet<_> = pattern
        .captures_iter(source)
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new();
    let head = fixture.git(&["rev-parse", "HEAD"]);
    let missing = ProjectId::new();
    for name in COMMANDS {
        assert_eq!(
            fixture.call(name, valid_args(&missing)).await.unwrap_err()["code"],
            "NotFound",
            "{name}"
        );
    }
    assert!(fixture.services.plugin.0.read().is_none());
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert_eq!(fixture.sink.installations.load(Ordering::Acquire), 1);
    assert_eq!(
        fixture
            .call("git_stage", json!({"paths":false}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert!(fixture.call("synthetic_unknown", json!({})).await.is_err());
    let channels: ChannelFactory = Arc::new(|id| {
        assert_eq!(id, "1");
        Box::new(|body| {
            assert!(matches!(body, ResponseBody::Json(text) if text == "null"));
            Ok(())
        })
    });
    let text = (fixture.backend.json)(
        fixture.services.clone(),
        "layout_get".into(),
        json!({"owner":"main"}),
        channels,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap()["owner"],
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
    fixture.services.tasks.shutdown().await;
    for name in COMMANDS {
        assert_eq!(
            fixture
                .call(name, valid_args(&fixture.project))
                .await
                .unwrap_err()["code"],
            "Forbidden",
            "{name}"
        );
    }
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        ORIGINAL
    );
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_조회_patch와_plugin_diff_캐시_이벤트무효화를_보존한다() {
    let fixture = Fixture::new();
    fixture.ok("git_init", json!({})).await;
    let first = fixture.status().await;
    assert_eq!(first["rows"], json!([]));
    assert_eq!(fixture.ok("git_current_user", json!({})).await, AUTHOR);
    assert_eq!(
        fixture.ok("git_ahead_behind", json!({})).await,
        json!({"ahead":0,"behind":0})
    );
    assert_eq!(fixture.ok("git_remotes", json!({})).await, json!([]));
    assert_eq!(
        fixture.ok("git_log", json!({"skip":0,"take":TAKE})).await[0]["summary"],
        "test(fixture): initial"
    );
    assert_eq!(
        fixture
            .ok("git_show_file", json!({"rev":"HEAD","path":FILE}))
            .await,
        ORIGINAL
    );
    assert_eq!(
        fixture.ok("git_commit_files", json!({"rev":"HEAD"})).await[0]["path"],
        FILE
    );
    assert_eq!(
        fixture
            .ok("git_file_log", json!({"path":FILE,"skip":0,"take":TAKE}))
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fixture
            .ok("git_blame_range", json!({"path":FILE,"from":1,"to":LINE}))
            .await[0]["author"],
        AUTHOR
    );
    let plugins = fixture.services.state.paths.plugins_dir().join("synthetic");
    std::fs::create_dir_all(&plugins).unwrap();
    std::fs::write(
        plugins.join("taide-plugin.json"),
        json!({"manifestVersion":1,"id":"synthetic","name":"Synthetic","version":"1.0.0",
        "contributes":{"languages":[{"id":"synthetic-language","extensions":[".synthetic"]}]}})
        .to_string(),
    )
    .unwrap();
    std::fs::write(fixture.root.join(FILE), CHANGED).unwrap();
    assert_eq!(fixture.status().await, first);
    fixture.notify_file();
    let changed = fixture.status().await;
    assert_eq!(changed["rows"][0]["unstaged"], "modified");
    assert_eq!(fixture.sink.installations.load(Ordering::Acquire), 1);
    let diff = fixture.diff("workdirVsIndex").await;
    assert_eq!(
        diff,
        json!({"original":ORIGINAL,"modified":CHANGED,"languageId":"synthetic-language"})
    );
    assert_eq!(
        fixture.ok("git_gutter", json!({"path":FILE})).await[0]["start"],
        LINE
    );
    let unstaged = fixture.ok("git_diff_staged_text", json!({})).await;
    assert_eq!(unstaged["usedFallback"], true);
    assert!(unstaged["diffText"].as_str().unwrap().contains("+changed"));
    for (stage, unstage, start, end) in [
        ("git_stage_hunk", "git_unstage_hunk", "hunkStart", "hunkEnd"),
        (
            "git_stage_lines",
            "git_unstage_lines",
            "lineStart",
            "lineEnd",
        ),
    ] {
        let mut args = json!({"path":FILE});
        args[start] = json!(LINE);
        args[end] = json!(LINE);
        fixture.ok(stage, args.clone()).await;
        assert_eq!(fixture.diff("indexVsHead").await["modified"], CHANGED);
        fixture.ok(unstage, args).await;
        let expected = if unstage == "git_unstage_lines" {
            "one\nthree\n"
        } else {
            ORIGINAL
        };
        assert_eq!(fixture.diff("indexVsHead").await["modified"], expected);
        fixture.ok("git_unstage", json!({"paths":[FILE]})).await;
        assert_eq!(fixture.diff("indexVsHead").await["modified"], ORIGINAL);
    }
    fixture
        .ok(
            "git_discard_hunk",
            json!({"path":FILE,"hunkStart":LINE,"hunkEnd":LINE}),
        )
        .await;
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        ORIGINAL
    );
    std::fs::write(fixture.root.join(FILE), CHANGED).unwrap();
    fixture.ok("git_stage", json!({"paths":[FILE]})).await;
    assert_eq!(
        fixture.ok("git_diff_staged_text", json!({})).await["usedFallback"],
        false
    );
    fixture.ok("git_unstage", json!({"paths":[FILE]})).await;
    fixture.ok("git_discard", json!({"paths":[FILE]})).await;
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        ORIGINAL
    );
    let outside = fixture.directory.join("outside.synthetic");
    std::fs::write(&outside, ORIGINAL).unwrap();
    assert!(
        fixture
            .call("git_stage", json!({"paths":[outside]}))
            .await
            .is_err()
    );
    assert!(
        fixture
            .call(
                "git_resolve_conflict",
                json!({"path":outside,"content":RESOLVED})
            )
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), ORIGINAL);
    let current = fixture.status().await;
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.status().await, current);
    assert_eq!(
        fixture
            .call("git_log", json!({"skip":0,"take":TAKE}))
            .await
            .unwrap_err()["code"],
        "Forbidden"
    );
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_branch_stash_commit_tag_revert_undo와_원본_이벤트를_보존한다() {
    let fixture = Fixture::new();
    fixture
        .ok(
            "git_branch_create",
            json!({"name":"synthetic-feature","checkout":false}),
        )
        .await;
    assert!(
        fixture
            .ok("git_branches", json!({}))
            .await
            .as_array()
            .unwrap()
            .iter()
            .any(|branch| branch["name"] == "synthetic-feature")
    );
    fixture
        .ok("git_branch_checkout", json!({"name":"synthetic-feature"}))
        .await;
    assert_eq!(fixture.status().await["branch"], "synthetic-feature");
    fixture
        .ok("git_branch_checkout", json!({"name":"main"}))
        .await;
    fixture
        .ok(
            "git_branch_delete",
            json!({"name":"synthetic-feature","force":false}),
        )
        .await;
    std::fs::write(fixture.root.join(FILE), CHANGED).unwrap();
    fixture
        .ok("git_stash_push", json!({"message":"synthetic stash"}))
        .await;
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        ORIGINAL
    );
    assert!(
        fixture.ok("git_stash_list", json!({})).await[0]["message"]
            .as_str()
            .unwrap()
            .contains("synthetic stash")
    );
    fixture.ok("git_stash_apply", json!({"index":0})).await;
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        CHANGED
    );
    fixture.ok("git_stash_drop", json!({"index":0})).await;
    assert_eq!(fixture.ok("git_stash_list", json!({})).await, json!([]));
    fixture.ok("git_discard", json!({"paths":[FILE]})).await;
    let before = fixture.sink.events.lock().unwrap().len();
    let oid = fixture.commit(CHANGED, "test(fixture): changed").await;
    {
        let events = fixture.sink.events.lock().unwrap();
        assert!(matches!(
            &events[before..],
            [
                AppEvent::GitStatusChanged { .. },
                AppEvent::GitStatusChanged { .. },
                AppEvent::GitRefsChanged { .. }
            ]
        ));
    }
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), oid);
    fixture.ok("git_tag_create", json!({"name":"synthetic-tag","target":oid,"opts":{"annotated":true,"message":"synthetic tag"}})).await;
    let tags = fixture.ok("git_tags", json!({})).await;
    assert_eq!(tags[0]["name"], "synthetic-tag");
    assert_eq!(tags[0]["annotated"], true);
    fixture
        .ok("git_tag_delete", json!({"name":"synthetic-tag"}))
        .await;
    assert_eq!(fixture.ok("git_tags", json!({})).await, json!([]));
    let reverted = fixture.ok("git_revert_commit", json!({"rev":oid})).await;
    assert_eq!(reverted["conflicted"], false);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        ORIGINAL
    );
    fixture.ok("git_undo_last_commit", json!({})).await;
    assert_eq!(fixture.git(&["rev-parse", "HEAD"]), oid);
    assert_eq!(fixture.diff("indexVsHead").await["modified"], ORIGINAL);
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_local_bare_push_pull_fetch_remote_branch와_conflict3side를_보존한다() {
    let fixture = Fixture::new();
    let remote = fixture.directory.join("remote.git");
    fixture.git(&[
        "init",
        "--bare",
        "--initial-branch=main",
        "--template",
        fixture.empty.to_str().unwrap(),
        remote.to_str().unwrap(),
    ]);
    fixture.git(&[
        "--git-dir",
        remote.to_str().unwrap(),
        "config",
        "--local",
        "core.hooksPath",
        fixture.empty.to_str().unwrap(),
    ]);
    fixture.git(&["remote", "add", "origin", remote.to_str().unwrap()]);
    fixture.git(&["config", "--local", "branch.main.remote", "origin"]);
    fixture.git(&["config", "--local", "branch.main.merge", "refs/heads/main"]);
    fixture.ok("git_push", json!({})).await;
    let oid = fixture.commit(CHANGED, "test(fixture): local remote").await;
    fixture.ok("git_push", json!({})).await;
    assert_eq!(
        fixture.git(&[
            "--git-dir",
            remote.to_str().unwrap(),
            "rev-parse",
            "refs/heads/main"
        ]),
        oid
    );
    let before = fixture.sink.events.lock().unwrap().len();
    fixture.ok("git_fetch", json!({})).await;
    fixture.ok("git_pull", json!({})).await;
    {
        let events = fixture.sink.events.lock().unwrap();
        assert!(matches!(
            &events[before..],
            [
                AppEvent::GitRefsChanged { .. },
                AppEvent::GitStatusChanged { .. },
                AppEvent::GitRefsChanged { .. }
            ]
        ));
    }
    assert_eq!(
        fixture.ok("git_remotes", json!({})).await[0]["url"],
        remote.to_str().unwrap()
    );
    assert_eq!(
        fixture.ok("git_ahead_behind", json!({})).await,
        json!({"ahead":0,"behind":0})
    );
    fixture.git(&["push", "origin", "main:remote-feature"]);
    fixture.ok("git_fetch", json!({})).await;
    fixture
        .ok(
            "git_checkout_remote_branch",
            json!({"remoteRef":"origin/remote-feature"}),
        )
        .await;
    assert_eq!(fixture.status().await["branch"], "remote-feature");
    fixture.finish().await;

    let fixture = Fixture::new();
    fixture
        .ok(
            "git_branch_create",
            json!({"name":"synthetic-other","checkout":true}),
        )
        .await;
    fixture.commit(CHANGED, "test(fixture): other").await;
    fixture
        .ok("git_branch_checkout", json!({"name":"main"}))
        .await;
    fixture.commit(OURS, "test(fixture): ours").await;
    let merged = fixture.output(&["merge", "synthetic-other", "--no-edit"]);
    assert!(!merged.status.success());
    assert!(String::from_utf8_lossy(&merged.stdout).contains("CONFLICT"));
    let sides = fixture.ok("git_conflict_sides", json!({"path":FILE})).await;
    assert_eq!(sides["base"], ORIGINAL);
    assert_eq!(sides["ours"], OURS);
    assert_eq!(sides["theirs"], CHANGED);
    assert!(sides["workdir"].as_str().unwrap().contains("<<<<<<<"));
    fixture
        .ok(
            "git_resolve_conflict",
            json!({"path":FILE,"content":RESOLVED}),
        )
        .await;
    assert_eq!(
        std::fs::read_to_string(fixture.root.join(FILE)).unwrap(),
        RESOLVED
    );
    assert_eq!(fixture.status().await["rows"][0]["isConflicted"], false);
    assert_eq!(fixture.diff("indexVsHead").await["modified"], RESOLVED);
    fixture.finish().await;
}

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::json;
use taide_model::ids::{ProjectId, TabId};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::tree::TreeRowPage;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

use super::*;
use crate::remote_gateway;
use crate::remote_ws::ChannelFactory;

const CONTENT: &str = "synthetic native remote content";
const DRAFT: &str = "synthetic unsaved draft";

#[tokio::test]
async fn mirror_쓰기_receipt는_동일_초안의_새_쓰기를_구별하고_legacy와_권한을_보존한다() {
    let fixture = Fixture::new();
    let path = fixture.root.join("receipt.txt");
    std::fs::write(&path, CONTENT).unwrap();
    let write = |receipt: Value| json!({"projectId":fixture.project,"path":path,"content":DRAFT,"receipt":receipt});
    let first = fixture.ok("file_mirror_dirty", write(json!(true))).await;
    assert!(
        first["writeId"].is_string(),
        "actual write receipt is required"
    );
    assert_eq!(first["entry"]["content"], DRAFT);
    assert_eq!(first["entry"]["path"], json!(path));
    let second = fixture.ok("file_mirror_dirty", write(json!(true))).await;
    assert_ne!(first["writeId"], second["writeId"]);
    let clear =
        |receipt: Value| json!({"projectId":fixture.project,"path":path,"expectedReceipt":receipt});
    assert_eq!(
        fixture.ok("file_clear_mirror", clear(first)).await,
        json!(false)
    );
    assert!(
        fixture
            .call("file_mirror_dirty", write(json!("true")))
            .await
            .is_err()
    );
    assert!(
        fixture
            .call("file_clear_mirror", clear(Value::Null))
            .await
            .is_err()
    );
    assert!(fixture.call("file_clear_mirror", json!({
        "projectId":fixture.project,"path":path,"expectedReceipt":second,"expected":null,
    })).await.is_err());
    assert_eq!(
        fixture.ok("file_clear_mirror", clear(second.clone())).await,
        json!(true)
    );
    assert_eq!(
        fixture.ok("file_clear_mirror", clear(second)).await,
        json!(true)
    );
    let legacy = fixture.ok("file_mirror_dirty", write(json!(false))).await;
    assert!(legacy.is_number());
    assert!(
        fixture
            .ok(
                "file_mirror_dirty",
                json!({
                    "projectId":fixture.project,"path":path,"content":DRAFT,
                })
            )
            .await
            .is_number()
    );
    assert_eq!(
        fixture
            .ok(
                "file_clear_mirror",
                json!({
                    "projectId":fixture.project,"path":path,
                })
            )
            .await,
        Value::Null
    );
    let outside = fixture.directory.join("outside-receipt.txt");
    std::fs::write(&outside, CONTENT).unwrap();
    assert!(
        fixture
            .call(
                "file_mirror_dirty",
                json!({
                    "projectId":fixture.project,"path":outside,"content":DRAFT,"receipt":true,
                })
            )
            .await
            .is_err()
    );
    fixture
        .services
        .state
        .projects
        .write()
        .remove(&fixture.project);
    assert!(
        fixture
            .call("file_mirror_dirty", write(json!(true)))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn 조건부_mirror_삭제는_새_초안과_입력_권한을_보존한다() {
    let fixture = Fixture::new();
    let path = fixture.root.join("conditional.txt");
    std::fs::write(&path, CONTENT).unwrap();
    let write = |content: &str| json!({"projectId":fixture.project,"path":path,"content":content});
    fixture.ok("file_mirror_dirty", write(DRAFT)).await;
    let old = fixture
        .ok("file_list_mirrors", json!({"projectId":fixture.project}))
        .await[0]
        .clone();
    fixture.ok("file_mirror_dirty", write(CONTENT)).await;
    let clear =
        |expected: Value| json!({"projectId":fixture.project,"path":path,"expected":expected});
    assert_eq!(
        fixture.ok("file_clear_mirror", clear(old)).await,
        json!(false)
    );
    let latest = fixture
        .ok("file_list_mirrors", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(latest[0]["content"], CONTENT);
    assert_eq!(
        fixture.ok("file_clear_mirror", clear(Value::Null)).await,
        json!(false)
    );
    assert!(
        fixture
            .call("file_clear_mirror", clear(json!({"content":"invalid"})))
            .await
            .is_err()
    );
    let outside = fixture.directory.join("outside.txt");
    std::fs::write(&outside, CONTENT).unwrap();
    let error = fixture
        .call(
            "file_clear_mirror",
            json!({"projectId":fixture.project,"path":outside,"expected":null}),
        )
        .await
        .unwrap_err();
    assert_eq!(error["message"]["kind"], "Forbidden");
    assert_eq!(
        fixture
            .ok("file_clear_mirror", clear(latest[0].clone()))
            .await,
        json!(true)
    );
    assert_eq!(
        fixture.ok("file_clear_mirror", clear(Value::Null)).await,
        json!(true)
    );
    fixture.ok("file_mirror_dirty", write(DRAFT)).await;
    assert_eq!(
        fixture
            .ok(
                "file_clear_mirror",
                json!({"projectId":fixture.project,"path":path})
            )
            .await,
        Value::Null
    );
    assert_eq!(
        fixture
            .ok("file_list_mirrors", json!({"projectId":fixture.project}))
            .await,
        json!([])
    );
    fixture
        .services
        .state
        .projects
        .write()
        .remove(&fixture.project);
    assert!(
        fixture
            .call("file_clear_mirror", clear(Value::Null))
            .await
            .is_err()
    );
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: taide_model::app_event::AppEvent) {
        panic!("file/tree commands do not emit application events");
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
            std::env::temp_dir().join(format!("taide-native-remote-files-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic remote files".into(),
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
            json: Arc::new(|_, name, _, _| {
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    Ok("null".into())
                })
            }),
            raw: Arc::new(|_, name, _| {
                Box::pin(async move { panic!("unexpected raw delegation: {name}") })
            }),
        };
        Self {
            directory,
            root,
            project,
            services,
            backend: remote_gateway::with_policy(extend_backend(remaining)),
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory = Arc::new(|_| Box::new(|_| panic!("files have no channels")));
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    async fn raw(&self, path: &std::path::Path) -> Result<Vec<u8>, Value> {
        (self.backend.raw)(
            self.services.clone(),
            "file_read_raw".into(),
            json!({"path":path}),
        )
        .await
    }

    async fn page(&self, name: &str, args: Value) -> TreeRowPage {
        serde_json::from_value(self.ok(name, args).await).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn catalog과_실제_arm_모드_경로_입력_plugin_선행_guard를_보존한다() {
    let declared: BTreeSet<_> = JSON_COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), JSON_COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    assert!(
        declared
            .iter()
            .all(|name| !crate::remote_preferences::COMMANDS.contains(name)
                && !crate::remote_ide::COMMANDS.contains(name))
    );
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-files.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    assert!(REMOTE_ALLOWED_COMMANDS.contains(&"file_read_raw"));
    let fixture = Fixture::new();
    assert_eq!(fixture.ok("layout_get", Value::Null).await, Value::Null);
    let outside = fixture.directory.join("outside.txt");
    std::fs::write(&outside, "synthetic outside").unwrap();
    let requests = [
        ("file_open", json!({"path":outside})),
        ("file_save", json!({"path":outside,"content":CONTENT})),
        ("file_create", json!({"path":outside,"isDir":false})),
        (
            "file_rename",
            json!({"from":outside,"to":fixture.root.join("moved.txt")}),
        ),
        (
            "file_copy",
            json!({"from":outside,"to":fixture.root.join("copy.txt")}),
        ),
        ("file_delete", json!({"path":outside})),
        (
            "file_mirror_dirty",
            json!({"projectId":fixture.project,"path":outside,"content":DRAFT}),
        ),
        (
            "file_clear_mirror",
            json!({"projectId":fixture.project,"path":outside}),
        ),
    ];
    for (name, args) in requests {
        let error = fixture.call(name, args).await.unwrap_err();
        assert_eq!(error["message"]["kind"], "Forbidden", "{name}: {error}");
    }
    assert!(fixture.services.plugin.0.read().is_none());
    assert_eq!(
        fixture.raw(&outside).await.unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "synthetic outside"
    );
    assert!(!fixture.root.join("copy.txt").exists());
    assert!(!fixture.root.join("moved.txt").exists());
    let valid = fixture.root.join("valid.txt");
    std::fs::write(&valid, CONTENT).unwrap();
    assert_eq!(
        fixture
            .call("file_read_raw", json!({"path":valid}))
            .await
            .unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_open".into(),
            json!({"path":valid})
        )
        .await
        .unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert_eq!(
        fixture
            .call("file_save", json!({"path":valid,"content":false}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    let error = fixture
        .call(
            "file_mirror_untitled",
            json!({"projectId":fixture.project,"tabId":"../escape","content":DRAFT}),
        )
        .await
        .unwrap_err();
    assert_eq!(error["message"]["kind"], "InvalidArgument");
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다() {
    let fixture = Fixture::new();
    let plugin_dir = fixture.services.state.paths.plugins_dir().join("synthetic");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::write(
        plugin_dir.join("taide-plugin.json"),
        json!({"manifestVersion":1,
        "id":"synthetic","name":"Synthetic","version":"1.0.0",
        "contributes":{"languages":[{"id":"synthetic-language","extensions":[".synremote"]}]}})
        .to_string(),
    )
    .unwrap();
    let folder = fixture.root.join("nested");
    fixture
        .ok("file_create", json!({"path":folder,"isDir":true}))
        .await;
    assert!(folder.is_dir());
    let file = folder.join("file.synremote");
    fixture
        .ok("file_create", json!({"path":file,"isDir":false}))
        .await;
    fixture
        .ok("file_save", json!({"path":file,"content":CONTENT}))
        .await;
    let opened = fixture.ok("file_open", json!({"path":file})).await;
    assert_eq!(opened["content"], CONTENT);
    assert_eq!(opened["languageId"], "synthetic-language");
    assert_eq!(fixture.raw(&file).await.unwrap(), CONTENT.as_bytes());
    let copy = folder.join("copy.synremote");
    fixture
        .ok("file_copy", json!({"from":file,"to":copy}))
        .await;
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), CONTENT);
    let moved = folder.join("moved.synremote");
    fixture
        .ok("file_rename", json!({"from":copy,"to":moved}))
        .await;
    assert!(!copy.exists());
    assert_eq!(std::fs::read_to_string(&moved).unwrap(), CONTENT);
    fixture.ok("file_delete", json!({"path":moved})).await;
    assert!(!moved.exists());
    fixture
        .ok(
            "file_mirror_dirty",
            json!({"projectId":fixture.project,"path":file,"content":DRAFT}),
        )
        .await;
    let mirrors = fixture
        .ok("file_list_mirrors", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(mirrors.as_array().unwrap().len(), 1);
    assert_eq!(mirrors[0]["content"], DRAFT);
    fixture
        .ok(
            "file_prune_mirrors",
            json!({"projectId":fixture.project,"keepPaths":[file]}),
        )
        .await;
    assert_eq!(
        fixture
            .ok("file_list_mirrors", json!({"projectId":fixture.project}))
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fixture
        .ok(
            "file_clear_mirror",
            json!({"projectId":fixture.project,"path":file}),
        )
        .await;
    assert!(
        fixture
            .ok("file_list_mirrors", json!({"projectId":fixture.project}))
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );
    fixture
        .ok(
            "file_mirror_dirty",
            json!({"projectId":fixture.project,"path":file,"content":DRAFT}),
        )
        .await;
    fixture
        .ok(
            "file_prune_mirrors",
            json!({"projectId":fixture.project,"keepPaths":[]}),
        )
        .await;
    assert!(
        fixture
            .ok("file_list_mirrors", json!({"projectId":fixture.project}))
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );

    let tab = TabId::new();
    let removed = TabId::new();
    for id in [&tab, &removed] {
        fixture
            .ok(
                "file_mirror_untitled",
                json!({"projectId":fixture.project,"tabId":id,"content":DRAFT}),
            )
            .await;
    }
    assert_eq!(
        fixture
            .ok(
                "file_list_untitled_mirrors",
                json!({"projectId":fixture.project})
            )
            .await
            .as_array()
            .unwrap()
            .len(),
        2
    );
    fixture
        .ok(
            "file_prune_untitled_mirrors",
            json!({"projectId":fixture.project,"keepTabIds":[tab]}),
        )
        .await;
    let drafts = fixture
        .ok(
            "file_list_untitled_mirrors",
            json!({"projectId":fixture.project}),
        )
        .await;
    assert_eq!(drafts.as_array().unwrap().len(), 1);
    assert_eq!(drafts[0]["tabId"], tab.as_str());
    assert_eq!(drafts[0]["content"], DRAFT);
    fixture
        .ok(
            "file_clear_untitled_mirror",
            json!({"projectId":fixture.project,"tabId":tab}),
        )
        .await;
    assert!(
        fixture
            .ok(
                "file_list_untitled_mirrors",
                json!({"projectId":fixture.project})
            )
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 실제_트리5명령은_페이지_expand_reveal_refresh_collapse와_감독자_거절을_보존한다() {
    let fixture = Fixture::new();
    let folder = fixture.root.join("nested");
    let file = folder.join("synthetic.txt");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(&file, CONTENT).unwrap();
    let args = json!({"projectId":fixture.project,"offset":0});
    let initial = fixture.page("tree_rows", args.clone()).await;
    assert_eq!(initial.rows.len(), 1);
    assert_eq!(initial.rows[0].path, folder.to_str().unwrap());
    let expanded = fixture
        .page(
            "tree_toggle",
            json!({"projectId":fixture.project,"path":folder}),
        )
        .await;
    assert_eq!(expanded.rows.len(), 2);
    assert!(expanded.rows[0].expanded);
    assert_eq!(expanded.rows[1].path, file.to_str().unwrap());
    let limited = fixture
        .page(
            "tree_rows",
            json!({"projectId":fixture.project,"offset":1,"limit":1}),
        )
        .await;
    assert_eq!(limited.rows.len(), 1);
    assert_eq!(limited.total, expanded.total);
    fixture
        .ok("tree_collapse_all", json!({"projectId":fixture.project}))
        .await;
    let revealed = fixture
        .page(
            "tree_reveal",
            json!({"projectId":fixture.project,"path":file}),
        )
        .await;
    assert_eq!(revealed.rows.len(), 2);
    let added = folder.join("added.txt");
    std::fs::write(&added, CONTENT).unwrap();
    let refreshed = fixture
        .page(
            "tree_refresh",
            json!({"projectId":fixture.project,"dir":folder}),
        )
        .await;
    assert!(
        refreshed
            .rows
            .iter()
            .any(|row| row.path == added.to_str().unwrap())
    );
    let collapsed = fixture
        .page("tree_collapse_all", json!({"projectId":fixture.project}))
        .await;
    assert_eq!(collapsed.rows.len(), 1);
    assert!(!collapsed.rows[0].expanded);
    assert_eq!(
        fixture
            .call(
                "tree_rows",
                json!({"projectId":ProjectId::new(),"offset":0})
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    fixture.services.tasks.shutdown().await;
    for (name, args) in [
        ("tree_rows", args),
        (
            "tree_toggle",
            json!({"projectId":fixture.project,"path":folder}),
        ),
        ("tree_collapse_all", json!({"projectId":fixture.project})),
        (
            "tree_reveal",
            json!({"projectId":fixture.project,"path":file}),
        ),
        (
            "tree_refresh",
            json!({"projectId":fixture.project,"dir":folder}),
        ),
        ("file_open", json!({"path":file})),
        ("file_save", json!({"path":file,"content":"must not write"})),
        (
            "file_copy",
            json!({"from":file,"to":folder.join("must-not-create")}),
        ),
        (
            "file_mirror_dirty",
            json!({"projectId":fixture.project,"path":file,"content":DRAFT}),
        ),
    ] {
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["code"],
            "Forbidden",
            "{name}"
        );
    }
    assert_eq!(std::fs::read_to_string(&file).unwrap(), CONTENT);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

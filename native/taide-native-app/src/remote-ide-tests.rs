use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use taide_ide::protocol::IdeSelectionSnapshot;
use taide_ide::store::{PendingDiff, PendingSave};
use taide_model::ide::{IdeDiagnostic, IdeDiffOutcome};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::oneshot;

use super::*;
use crate::remote_gateway;
use crate::remote_ws::{ChannelFactory, ResponseBody};

const DEADLINE: Duration = Duration::from_secs(5);
const ORIGINAL: &str = "synthetic original";
const SAVED: &str = "synthetic saved";

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: taide_model::app_event::AppEvent) {
        panic!("IDE commands do not emit application events");
    }
}

struct Fixture {
    directory: PathBuf,
    file: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
    backend: Dispatch,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-ide-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project");
        let file = root.join("synthetic.txt");
        std::fs::write(&file, ORIGINAL).unwrap();
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic remote IDE".into(),
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
                    channels("1".into())(ResponseBody::Json("null".into())).unwrap();
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(|_, name, args| {
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    assert_eq!(args["owner"], "remote");
                    Ok(vec![0])
                })
            }),
        };
        Self {
            directory,
            file,
            project,
            services,
            backend: remote_gateway::with_policy(extend_backend(remaining)),
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("IDE commands must not use channels")));
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    fn diff(&self, id: &str, path: PathBuf) -> oneshot::Receiver<(IdeDiffOutcome, Option<String>)> {
        let (sender, receiver) = oneshot::channel();
        self.services.ide.insert_pending_diff(
            id.into(),
            PendingDiff {
                project_id: self.project.clone(),
                new_path: path,
                responder: sender,
            },
        );
        receiver
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn 목록과_실제_arm_일치_선택_owner_진단_알림과_remaining_전달을_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    assert!(
        declared
            .iter()
            .all(|name| !crate::remote_preferences::COMMANDS.contains(name))
    );
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let source = include_str!("remote-ide.rs");
    let arms: BTreeSet<_> = pattern
        .captures_iter(source)
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);

    let fixture = Fixture::new();
    let status = fixture.ok("ide_get_status", Value::Null).await;
    assert_eq!(
        status,
        serde_json::to_value(fixture.services.ide.status()).unwrap()
    );
    let selection = IdeSelectionSnapshot {
        project_id: fixture.project.clone(),
        path: fixture.file.to_str().unwrap().into(),
        text: ORIGINAL.into(),
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: 1,
        is_empty: false,
    };
    fixture.services.ide.set_selection(selection.clone());
    let mut notifications = fixture.services.ide.subscribe();
    let input = json!({"owner":"main", "projectId":fixture.project, "path":fixture.file,
        "text":"remote must not overwrite", "startLine":0, "startCharacter":0,
        "endLine":1, "endCharacter":0, "isEmpty":false});
    assert_eq!(
        fixture
            .ok("ide_set_selection", json!({"input":input}))
            .await,
        Value::Null
    );
    fixture
        .ok("ide_clear_selection", json!({"owner":"main"}))
        .await;
    assert_eq!(
        fixture.services.ide.current_selection(),
        Some(selection.clone())
    );
    assert_eq!(fixture.services.ide.latest_selection(), Some(selection));
    assert!(matches!(
        notifications.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    let diagnostic = json!({"path":fixture.file,"severity":"warning", "startLine":0,
        "startCharacter":0,"endLine":1,"endCharacter":0,"message":"synthetic diagnostic"});
    fixture
        .ok(
            "ide_publish_diagnostics",
            json!({"projectId":fixture.project,"items":[diagnostic.clone()]}),
        )
        .await;
    let expected: IdeDiagnostic = serde_json::from_value(diagnostic).unwrap();
    assert_eq!(fixture.services.ide.diagnostics(None), Some(vec![expected]));
    fixture
        .ok(
            "ide_notify_at_mention",
            json!({"path":fixture.file,"lineStart":0,"lineEnd":1}),
        )
        .await;
    let notified: Value = serde_json::from_str(&notifications.try_recv().unwrap()).unwrap();
    assert_eq!(notified["method"], "at_mentioned");
    assert_eq!(
        notified["params"],
        json!({"filePath":fixture.file,"lineStart":0,"lineEnd":1})
    );

    let channels: ChannelFactory = Arc::new(|id| {
        assert_eq!(id, "1");
        Box::new(|body| {
            let ResponseBody::Json(text) = body else {
                panic!("JSON expected")
            };
            assert_eq!(text, "null");
            Ok(())
        })
    });
    let delegated = (fixture.backend.json)(
        fixture.services.clone(),
        "layout_get".into(),
        json!({"owner":"main"}),
        channels,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&delegated).unwrap()["owner"],
        "remote"
    );
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            json!({"owner":"main"})
        )
        .await
        .unwrap(),
        vec![0]
    );
    assert_eq!(
        fixture.call("ide_unknown", Value::Null).await.unwrap_err()["message"]["kind"],
        "Forbidden"
    );
    assert_eq!(
        fixture
            .call("ide_set_selection", json!({"input":{}}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn diff_saved는_실제_guarded_저장과_mirror_정리를_거치고_save_응답을_해소한다() {
    let fixture = Fixture::new();
    taide_file::service::mirror_dirty(
        &fixture.services.state.paths,
        &fixture.project,
        &fixture.file,
        fixture.file.to_str().unwrap(),
        "synthetic mirror",
    )
    .unwrap();
    assert!(
        taide_file::service::has_mirror(
            &fixture.services.state.paths,
            &fixture.project,
            &fixture.file
        )
        .unwrap()
    );
    let resolved = fixture.diff("saved", fixture.file.clone());
    let result = fixture
        .ok(
            "ide_resolve_diff",
            json!({"requestId":"saved","outcome":"saved","content":SAVED}),
        )
        .await;
    assert_eq!(result, Value::Null);
    assert_eq!(
        tokio::time::timeout(DEADLINE, resolved)
            .await
            .unwrap()
            .unwrap(),
        (IdeDiffOutcome::Saved, Some(SAVED.into()))
    );
    assert_eq!(std::fs::read_to_string(&fixture.file).unwrap(), SAVED);
    assert!(
        !taide_file::service::has_mirror(
            &fixture.services.state.paths,
            &fixture.project,
            &fixture.file
        )
        .unwrap()
    );
    assert_eq!(
        fixture
            .call(
                "ide_resolve_diff",
                json!({"requestId":"saved","outcome":"saved","content":SAVED})
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    for saved in [true, false] {
        let (sender, receiver) = oneshot::channel();
        fixture
            .services
            .ide
            .insert_pending_save("save".into(), PendingSave { responder: sender });
        assert_eq!(
            fixture
                .ok(
                    "ide_resolve_save",
                    json!({"requestId":"save","saved":saved})
                )
                .await,
            Value::Null
        );
        assert_eq!(
            tokio::time::timeout(DEADLINE, receiver)
                .await
                .unwrap()
                .unwrap(),
            saved
        );
    }
    assert_eq!(
        fixture
            .call("ide_resolve_save", json!({"requestId":"save","saved":true}))
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    fixture.services.tasks.shutdown().await;
}

#[tokio::test]
async fn diff_거부_누락_저장실패_닫힌_project와_취소는_원본_pending_수명을_유지한다() {
    let fixture = Fixture::new();
    for (wire, outcome) in [
        ("rejected", IdeDiffOutcome::Rejected),
        ("tabClosed", IdeDiffOutcome::TabClosed),
    ] {
        let response = fixture.diff(wire, fixture.file.clone());
        fixture
            .ok("ide_resolve_diff", json!({"requestId":wire,"outcome":wire}))
            .await;
        assert_eq!(response.await.unwrap(), (outcome, None));
        assert_eq!(std::fs::read_to_string(&fixture.file).unwrap(), ORIGINAL);
    }
    let missing_content = fixture.diff("missing-content", fixture.file.clone());
    assert_eq!(
        fixture
            .call(
                "ide_resolve_diff",
                json!({"requestId":"missing-content","outcome":"saved"})
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert!(missing_content.await.is_err());
    let directory = fixture.file.parent().unwrap().to_path_buf();
    let save_error = fixture.diff("directory", directory);
    assert!(
        fixture
            .call(
                "ide_resolve_diff",
                json!({"requestId":"directory","outcome":"saved","content":SAVED})
            )
            .await
            .is_err()
    );
    assert!(save_error.await.is_err());

    let guard = fixture.services.state.begin_owned_mutation().await;
    let cancelled = fixture.diff("cancelled", fixture.file.clone());
    let mut request = Box::pin(fixture.call(
        "ide_resolve_diff",
        json!({"requestId":"cancelled","outcome":"saved","content":SAVED}),
    ));
    assert!(futures_util::poll!(request.as_mut()).is_pending());
    drop(request);
    assert!(cancelled.await.is_err());
    drop(guard);
    assert_eq!(std::fs::read_to_string(&fixture.file).unwrap(), ORIGINAL);
    assert_eq!(
        fixture
            .call(
                "ide_resolve_diff",
                json!({"requestId":"cancelled","outcome":"saved","content":SAVED})
            )
            .await
            .unwrap_err()["code"],
        "NotFound"
    );

    let closed = fixture.diff("closed", fixture.file.clone());
    fixture
        .services
        .state
        .projects
        .write()
        .remove(&fixture.project);
    let error = fixture
        .call(
            "ide_resolve_diff",
            json!({"requestId":"closed","outcome":"saved","content":SAVED}),
        )
        .await
        .unwrap_err();
    assert_eq!(error["code"], "Localized");
    assert_eq!(error["message"]["kind"], "Forbidden");
    assert!(closed.await.is_err());
    assert_eq!(std::fs::read_to_string(&fixture.file).unwrap(), ORIGINAL);
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

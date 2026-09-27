use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::{task_actions, AppState, TaskSupervisor};
use uuid::Uuid;

struct Fixture(AppState);

impl Fixture {
    fn new() -> Self {
        Self(AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-task-actions-{}", Uuid::new_v4())),
        )))
    }

    fn project(&self) -> ProjectId {
        let root = self.0.paths.data_dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let project = Project {
            id: ProjectId::new(),
            root: root.to_string_lossy().into_owned(),
            name: "fixture".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        let id = project.id.clone();
        self.0.projects.write().insert(id.clone(), project);
        id
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.0.paths.data_dir).expect("자기 UUID task fixture만 정리");
        }
    }
}

#[tokio::test]
async fn 없는_프로젝트는_감독자_입장보다_먼저_거절된다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    tasks.stop_all();
    let error = task_actions::detect_tasks(&fixture.0, &tasks, ProjectId::new()).await.unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!fixture.0.paths.data_dir.exists());
}

#[tokio::test]
async fn scan은_같은_순서와_root_및_안전한_command_문자열만_반환한다() {
    let fixture = Fixture::new();
    let id = fixture.project();
    let root = fixture.0.paths.data_dir.join("root");
    std::fs::write(
        root.join("package.json"),
        r#"{"scripts":{"b":"never-run","a'fixture":"never-run"}}"#,
    )
    .unwrap();
    std::fs::write(root.join("bun.lock"), "").unwrap();
    std::fs::write(root.join("Makefile"), "fixture:\n\tnever-run\n").unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"fixture\"\n").unwrap();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let detected = task_actions::detect_tasks(&fixture.0, &tasks, id).await.unwrap();
    assert_eq!(detected, taide_task::service::detect_tasks(&root));
    assert!(detected.iter().all(|task| task.cwd == root.to_string_lossy()));
    assert_eq!(tasks.tracked_count(), 0);
    assert!(detected.iter().any(|task| task.command == "bun run 'b'"));
}

#[tokio::test]
async fn 종료한_감독자는_열린_프로젝트의_scan도_거절한다() {
    let fixture = Fixture::new();
    let id = fixture.project();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    tasks.stop_all();
    let error = task_actions::detect_tasks(&fixture.0, &tasks, id).await.unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn tauri와_원격은_같은_등록_감독자를_전달하고_직접_worker를_만들지_않는다() {
    let commands = include_str!("../src/domain/task/commands.rs");
    let remote = include_str!("../src/remote_gateway.rs");
    assert!(commands.contains("tasks: State<'_, TaskSupervisor>"));
    assert!(commands.contains("task_actions::detect_tasks(&state, &tasks, project_id).await"));
    assert!(!commands.contains("spawn_blocking"));
    assert!(remote.contains("task::detect_tasks(app.state(), app.state(), arg!(args, \"projectId\"))"));
}

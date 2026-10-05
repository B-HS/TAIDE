use std::future::Future;
use std::path::PathBuf;
use std::task::{Context, Poll, Waker};

use taide_model::file::REFUSED_FILE_BYTES;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_ui::document_admission::prepare_opened_document;
use taide_runtime::native_file_actions::open_document_file;
use taide_runtime::{AppState, TaskSupervisor};

const DOCUMENT_COUNT: usize = 4;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 2;

struct Fixture {
    dir: PathBuf,
    file: PathBuf,
    state: AppState,
    project: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir =
            std::env::temp_dir().join(format!("taide-document-admission-{}", ProjectId::new()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("main.rs");
        std::fs::write(&file, "한글\r\n日本語").unwrap();
        let state = AppState::new(AppPaths::new(dir.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "admission fixture".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        Self {
            dir,
            file,
            state,
            project,
        }
    }

    async fn opened(
        &self,
        tasks: &TaskSupervisor,
    ) -> taide_runtime::native_file_actions::NativeOpenedFile {
        open_document_file(
            &self.state,
            tasks,
            self.file.to_str().unwrap().into(),
            Vec::new,
        )
        .await
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: REFUSED_FILE_BYTES as usize,
    })
    .unwrap()
}

#[tokio::test]
async fn 실제_파일_admission은_commit까지_root와_작업_소유를_유지하고_drop에서_회수한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let prepared = prepare_opened_document(&fixture.state, &tasks, fixture.opened(&tasks).await)
        .await
        .unwrap();
    let mut waiting = Box::pin(fixture.state.begin_mutation());
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Pending));
    assert!(tasks.tracked_count() > 0);
    let mut store = store();
    let document = prepared.commit(&mut store).unwrap();
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "한글\r\n日本語"
    );
    assert!(matches!(
        waiting.as_mut().poll(&mut context),
        Poll::Ready(_)
    ));
    drop(waiting);
    let prepared = prepare_opened_document(&fixture.state, &tasks, fixture.opened(&tasks).await)
        .await
        .unwrap();
    assert_eq!(prepared.commit(&mut store).unwrap(), document);
    let prepared = prepare_opened_document(&fixture.state, &tasks, fixture.opened(&tasks).await)
        .await
        .unwrap();
    drop(prepared);
    assert_eq!(tasks.tracked_count(), 0);
    assert_eq!(store.documents().len(), 1);
}

#[tokio::test]
async fn 실제_파일_admission은_닫힌_root_외부_변경과_shutdown을_거절한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let opened = fixture.opened(&tasks).await;
    let removed = fixture
        .state
        .projects
        .write()
        .remove(&fixture.project)
        .unwrap();
    assert!(
        prepare_opened_document(&fixture.state, &tasks, opened)
            .await
            .is_err()
    );
    fixture
        .state
        .projects
        .write()
        .insert(fixture.project.clone(), removed);
    let opened = fixture.opened(&tasks).await;
    std::fs::write(&fixture.file, "changed synthetic file length").unwrap();
    assert!(
        prepare_opened_document(&fixture.state, &tasks, opened)
            .await
            .is_err()
    );
    let prepared = prepare_opened_document(&fixture.state, &tasks, fixture.opened(&tasks).await)
        .await
        .unwrap();
    fixture.state.begin_shutdown();
    let mut store = store();
    assert!(prepared.commit(&mut store).is_err());
    assert!(store.documents().is_empty());
    assert_eq!(tasks.tracked_count(), 0);
}

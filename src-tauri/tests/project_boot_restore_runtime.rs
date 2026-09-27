use std::path::{Path, PathBuf};

use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, WindowChrome};
use taide_project::service;
use taide_runtime::{project_actions, AppState};
use uuid::Uuid;

struct Fixture {
    base: PathBuf,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("taide-project-boot-{}", Uuid::new_v4()));
        Self {
            state: AppState::new(AppPaths::new(base.join("data"))),
            base,
        }
    }

    fn seed(&self, name: &str) -> Project {
        let root = self.base.join(name);
        std::fs::create_dir_all(&root).unwrap();
        let mut session = self.state.session.read().clone();
        let mut projects = self.state.projects.read().clone();
        let opened = service::open_project(&self.state.paths, &mut session, &mut projects, &root, true, |_| Vec::new()).unwrap();
        *self.state.session.write() = session;
        *self.state.projects.write() = projects;
        opened.project
    }

    fn restored(&self) -> AppState {
        AppState::new(AppPaths::new(self.state.paths.data_dir.clone()))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.base.exists() {
            std::fs::remove_dir_all(&self.base).expect("자기 UUID fixture만 정리");
        }
    }
}

#[test]
fn 빈_복원은_기본_state만_반영하고_저장_디렉터리를_생성하지_않는다() {
    let fixture = Fixture::new();
    assert!(project_actions::restore_state(&fixture.state).is_empty());
    assert!(fixture.state.projects.read().is_empty());
    assert!(fixture.state.layouts.read().is_empty());
    assert!(!fixture.base.exists());
}

#[test]
fn 부팅은_같은_session_layout_settings와_missing_root를_복원한다() {
    let fixture = Fixture::new();
    let first = fixture.seed("first");
    let missing = fixture.seed("missing");
    std::fs::remove_dir(Path::new(&missing.root)).unwrap();
    let layout = taide_layout::service::default_layout();
    taide_layout::service::save_layout(&fixture.state.paths, &first.id, &layout).unwrap();
    let mut settings = fixture.state.settings.read().clone();
    settings.follow_system_theme = true;
    taide_settings::service::save_settings(&fixture.state.paths, &settings).unwrap();
    let session_before = std::fs::read(fixture.state.paths.session_file()).unwrap();
    let restored = fixture.restored();
    assert!(project_actions::restore_state(&restored).is_empty());
    assert_eq!(*restored.settings.read(), settings);
    assert_eq!(restored.layouts.read().get(&first.id), Some(&layout));
    assert!(restored.projects.read().get(&missing.id).unwrap().root_missing);
    let pending = project_actions::projects_pending_watcher_restore(&restored.projects.read(), &restored.session.read());
    assert_eq!(pending, [(first.id, first.root)]);
    assert!(restored.dirty_layouts.read().is_empty());
    assert_eq!(std::fs::read(restored.paths.session_file()).unwrap(), session_before);
}

#[test]
fn legacy_chrome은_live_layout을_비우고_dirty와_session_승격을_저장한다() {
    let fixture = Fixture::new();
    let project = fixture.seed("legacy");
    let mut layout = taide_layout::service::default_layout();
    layout.shell_view.zen = true;
    layout.shell_view.sidebar_collapsed = true;
    taide_layout::service::save_layout(&fixture.state.paths, &project.id, &layout).unwrap();
    let layout_before = std::fs::read(fixture.state.paths.layout_file(&project.id)).unwrap();
    let restored = fixture.restored();
    assert!(project_actions::restore_state(&restored).is_empty());
    assert_eq!(
        restored.session.read().window_chrome,
        WindowChrome {
            zen: true,
            sidebar_rail_collapsed: true
        }
    );
    assert_eq!(service::load_session(&restored.paths).unwrap().0, *restored.session.read());
    assert_eq!(restored.layouts.read().get(&project.id).unwrap().shell_view, Default::default());
    assert!(restored.dirty_layouts.read().contains(&project.id));
    assert_eq!(std::fs::read(restored.paths.layout_file(&project.id)).unwrap(), layout_before);
}

#[test]
fn session_읽기_실패는_기존_live_state를_유지하되_settings를_복원한다() {
    let fixture = Fixture::new();
    let project = fixture.seed("existing");
    let before = fixture.state.session.read().clone();
    std::fs::remove_file(fixture.state.paths.session_file()).unwrap();
    std::fs::create_dir(fixture.state.paths.session_file()).unwrap();
    let mut settings = fixture.state.settings.read().clone();
    settings.follow_system_theme = true;
    taide_settings::service::save_settings(&fixture.state.paths, &settings).unwrap();
    let warnings = project_actions::restore_state(&fixture.state);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].starts_with("세션 복원 실패:"));
    assert_eq!(*fixture.state.session.read(), before);
    assert!(fixture.state.projects.read().contains_key(&project.id));
    assert_eq!(*fixture.state.settings.read(), settings);
}

#[test]
fn 선택은_active_session_drift_순서와_missing_제외를_유지한다() {
    let fixture = Fixture::new();
    let first = fixture.seed("first");
    let second = fixture.seed("second");
    let drift_id = ProjectId::new();
    let mut drift = first.clone();
    drift.id = drift_id.clone();
    fixture.state.projects.write().insert(drift_id.clone(), drift);
    let pending = project_actions::projects_pending_watcher_restore(&fixture.state.projects.read(), &fixture.state.session.read());
    assert_eq!(
        pending.iter().map(|(id, _)| id).collect::<Vec<_>>(),
        [&second.id, &first.id, &drift_id]
    );
    fixture.state.projects.write().get_mut(&second.id).unwrap().root_missing = true;
    let pending = project_actions::projects_pending_watcher_restore(&fixture.state.projects.read(), &fixture.state.session.read());
    assert_eq!(pending.iter().map(|(id, _)| id).collect::<Vec<_>>(), [&first.id, &drift_id]);
}

#[test]
fn setup의_동기_복원과_선택은_같은_runtime_정책을_호출한다() {
    let source = include_str!("../src/domain/project/commands.rs");
    assert!(source.contains("project_actions::restore_state(state)"));
    assert!(source.contains("project_actions::projects_pending_watcher_restore(projects, session)"));
    let setup = include_str!("../src/lib.rs");
    assert!(setup.find("commands::restore_state(&state)").unwrap() < setup.find("let services = Arc::new(AppServices::new(").unwrap());
    assert!(
        setup.find("commands::projects_pending_watcher_restore(").unwrap()
            < setup.find("let services = Arc::new(AppServices::new(").unwrap()
    );
}

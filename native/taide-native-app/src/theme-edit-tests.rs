use super::*;
use std::{path::PathBuf, sync::Mutex, time::Duration};
use taide_model::{
    error::AppErrorKind,
    ids::{PaneId, ProjectId, TabId},
    layout::{PaneNode, Tab, TabKind},
    paths::AppPaths,
};
use taide_runtime::{EventSink, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(5);
const EDITED_THEME_ID: &str = "work-light";

struct Emission {
    event: AppEvent,
    source_exists: bool,
}

struct Events {
    source: PathBuf,
    emitted: Mutex<Vec<Emission>>,
}

impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        self.emitted.lock().unwrap().push(Emission {
            event,
            source_exists: self.source.exists(),
        });
    }
}

struct Fixture {
    directory: PathBuf,
    runtime: tokio::runtime::Runtime,
    services: Arc<AppServices>,
    owner: Owner,
    events: Arc<Events>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-theme-edit-{}", ProjectId::new()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        let owner = Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: owner.pane.clone(),
            tabs: vec![Tab {
                id: owner.tab.clone(),
                kind: TabKind::Settings,
                title: "Settings".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: Some(owner.tab.clone()),
        };
        layout.focused_pane = owner.pane.clone();
        state.layouts.write().insert(owner.project.clone(), layout);
        let events = Arc::new(Events {
            source: state
                .paths
                .themes_dir()
                .join(format!("{EDITED_THEME_ID}.json")),
            emitted: Mutex::new(Vec::new()),
        });
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let tasks = TaskSupervisor::new(runtime.handle().clone());
        let services = crate::bootstrap::services(state, tasks, events.clone());
        Self {
            directory,
            runtime,
            services,
            owner,
            events,
        }
    }

    fn custom_light(&self) {
        let duplicate = Draft::load(
            &self.services.state,
            "taide-light",
            Mode::Create,
            "Synthetic light".into(),
        )
        .unwrap();
        let mut theme = duplicate.build().unwrap();
        theme.id = EDITED_THEME_ID.into();
        theme_actions::theme_save(&self.services.state, theme).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.runtime.block_on(async {
            tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
                .await
                .unwrap();
        });
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn native_theme_edit는_저장_중복_소유자와_폐기된_mount를_거절한다() {
    let fixture = Fixture::new();
    fixture.runtime.block_on(async {
        let session =
            Session::new(fixture.owner.clone(), "taide-dark".into(), Mode::Create).unwrap();
        let load = session.load_request("Synthetic duplicate".into());
        assert!(session.owns_load(&load));
        let mut draft = load.execute(&fixture.services).await.unwrap();
        draft.rename("Saved name".into());
        let save = session.save_request(&draft).unwrap();
        assert!(session.owns_save(&save));
        let summary = save.execute(&fixture.services).await.unwrap();
        assert_eq!(summary.id, draft.current().id);
        assert_eq!(
            theme_actions::theme_get(&fixture.services.state, summary.id.clone())
                .unwrap()
                .name,
            "Saved name"
        );
        assert_eq!(fixture.events.emitted.lock().unwrap().len(), 1);
        assert_eq!(
            save.execute(&fixture.services).await.unwrap_err().kind(),
            AppErrorKind::InvalidArgument
        );
        assert_eq!(fixture.events.emitted.lock().unwrap().len(), 1);
        assert!(session.delete_request().is_err());
        let edited = Session::new(fixture.owner.clone(), summary.id.clone(), Mode::Edit).unwrap();
        assert!(!edited.owns_load(&load));
        assert!(!edited.owns_save(&save));
        assert!(edited.save_request(&draft).is_err());
        let mut edit_draft = edited
            .load_request(String::new())
            .execute(&fixture.services)
            .await
            .unwrap();
        edit_draft.rename("Edited".into());
        edited
            .save_request(&edit_draft)
            .unwrap()
            .execute(&fixture.services)
            .await
            .unwrap();
        assert_eq!(
            theme_actions::theme_get(&fixture.services.state, summary.id.clone())
                .unwrap()
                .name,
            "Edited"
        );
        let delete = edited.delete_request().unwrap();
        assert!(edited.owns_delete(&delete));
        let remounted =
            Session::new(fixture.owner.clone(), summary.id.clone(), Mode::Edit).unwrap();
        assert!(!remounted.owns_delete(&delete));
        drop(edited);
        assert_eq!(
            delete.execute(&fixture.services).await.unwrap_err().kind(),
            AppErrorKind::Forbidden
        );
        let queued = remounted.save_request(&edit_draft).unwrap();
        let guard = fixture.services.state.begin_owned_mutation().await;
        let services = fixture.services.clone();
        let (entered, started) = tokio::sync::oneshot::channel();
        let waiter = tokio::spawn(async move {
            entered.send(()).unwrap();
            queued.execute(&services).await
        });
        started.await.unwrap();
        drop(remounted);
        drop(guard);
        let error = tokio::time::timeout(DEADLINE, waiter)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        drop(session);
        assert_eq!(
            load.execute(&fixture.services).await.unwrap_err().kind(),
            AppErrorKind::Forbidden
        );
        assert_eq!(
            save.execute(&fixture.services).await.unwrap_err().kind(),
            AppErrorKind::Forbidden
        );
        let mut wrong = fixture.owner.clone();
        wrong.project = ProjectId::new();
        let wrong = Session::new(wrong, "taide-dark".into(), Mode::Create).unwrap();
        assert_eq!(
            wrong
                .load_request("name".into())
                .execute(&fixture.services)
                .await
                .unwrap_err()
                .kind(),
            AppErrorKind::Forbidden
        );
        let hidden = Session::new(fixture.owner.clone(), summary.id, Mode::Edit).unwrap();
        fixture
            .services
            .state
            .layouts
            .write()
            .remove(&fixture.owner.project);
        assert_eq!(
            hidden
                .load_request(String::new())
                .execute(&fixture.services)
                .await
                .unwrap_err()
                .kind(),
            AppErrorKind::Forbidden
        );
        assert!(Session::new(fixture.owner.clone(), "../outside".into(), Mode::Edit).is_err());
    });
}

#[test]
fn native_theme_edit는_삭제전_fallback과_system_follow를_보존한다() {
    let fixture = Fixture::new();
    fixture.runtime.block_on(async {
        for follow in [false, true] {
            fixture.custom_light();
            {
                let mut settings = fixture.services.state.settings.write();
                settings.theme_id = EDITED_THEME_ID.into();
                settings.follow_system_theme = follow;
            }
            fixture.events.emitted.lock().unwrap().clear();
            let session = Session::new(fixture.owner.clone(), EDITED_THEME_ID.into(), Mode::Edit).unwrap();
            session.delete_request().unwrap().execute(&fixture.services).await.unwrap();
            assert!(!fixture.events.source.exists());
            let settings = fixture.services.state.settings.read().clone();
            assert_eq!(settings.theme_id, "taide-light");
            assert_eq!(settings.follow_system_theme, follow);
            let stored: taide_model::settings::Settings = taide_infra::persist::read_json(&fixture.services.state.paths.settings_file()).unwrap().unwrap();
            assert_eq!(stored, settings);
            let events = fixture.events.emitted.lock().unwrap();
            let Emission { event: AppEvent::SettingsChanged { settings }, source_exists } = &events[0] else { panic!("fallback event must precede deletion") };
            assert!(*source_exists);
            assert_eq!(settings.theme_id, "taide-light");
            assert_eq!(settings.follow_system_theme, follow);
            let theme_events: Vec<_> = events.iter().filter(|emission| matches!(emission.event, AppEvent::ThemeChanged { .. })).collect();
            assert_eq!(theme_events.len(), if follow { 1 } else { 2 });
            assert!(!theme_events.last().unwrap().source_exists);
            for emission in theme_events {
                assert!(matches!(&emission.event, AppEvent::ThemeChanged { theme_id } if theme_id == "taide-light"));
            }
        }
        fixture.custom_light();
        fixture.services.state.settings.write().theme_id = "taide-dark".into();
        fixture.events.emitted.lock().unwrap().clear();
        let session = Session::new(fixture.owner.clone(), EDITED_THEME_ID.into(), Mode::Edit).unwrap();
        session.delete_request().unwrap().execute(&fixture.services).await.unwrap();
        assert_eq!(fixture.services.state.settings.read().theme_id, "taide-dark");
        assert_eq!(fixture.events.emitted.lock().unwrap().len(), 1);
        let builtin = Session::new(fixture.owner.clone(), "taide-dark".into(), Mode::Edit).unwrap();
        assert_eq!(builtin.delete_request().unwrap().execute(&fixture.services).await.unwrap_err().kind(), AppErrorKind::InvalidArgument);
    });
}

#[test]
fn native_theme_edit는_fallback_저장실패시_테마삭제를_막는다() {
    let fixture = Fixture::new();
    fixture.custom_light();
    fixture.services.state.settings.write().theme_id = EDITED_THEME_ID.into();
    let before = fixture.services.state.settings.read().clone();
    let settings_path = fixture.services.state.paths.settings_file();
    std::fs::create_dir(&settings_path).unwrap();
    fixture.runtime.block_on(async {
        let session =
            Session::new(fixture.owner.clone(), EDITED_THEME_ID.into(), Mode::Edit).unwrap();
        let error = session
            .delete_request()
            .unwrap()
            .execute(&fixture.services)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Io);
        assert!(fixture.events.source.exists());
        assert_eq!(*fixture.services.state.settings.read(), before);
        assert!(fixture.events.emitted.lock().unwrap().is_empty());
        assert!(
            std::fs::read_dir(&fixture.directory)
                .unwrap()
                .all(|entry| !taide_infra::persist::is_temp_sibling(&entry.unwrap().path()))
        );
        fixture.services.state.begin_shutdown();
        assert_eq!(
            session
                .load_request(String::new())
                .execute(&fixture.services)
                .await
                .unwrap_err()
                .kind(),
            AppErrorKind::Forbidden
        );
        assert_eq!(
            session
                .delete_request()
                .unwrap()
                .execute(&fixture.services)
                .await
                .unwrap_err()
                .kind(),
            AppErrorKind::Forbidden
        );
    });
}

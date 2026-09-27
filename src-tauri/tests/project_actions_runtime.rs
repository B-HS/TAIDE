use std::sync::Mutex;

use futures_util::FutureExt;
use taide_model::app_event::AppEvent;
use taide_model::error::AppErrorKind;
use taide_model::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::layout::SplitDir;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay, ProjectDisplayPatch, ProjectGroup, ProjectRef, ShellSlotTree, WindowChromePatch};
use taide_runtime::{project_actions, AppState, EventSink};
use uuid::Uuid;

const PROJECT_COMMANDS: &[&str] = &[
    "project_list",
    "project_list_recent",
    "project_forget_recent",
    "project_get",
    "project_get_active",
    "session_get_shell_state",
    "session_focus_shell_slot",
    "session_set_shell_slot_sizes",
    "shell_slot_close",
    "session_set_window_chrome",
    "project_activate",
    "project_reorder",
    "project_set_display",
    "project_group_list",
    "project_group_create",
    "project_group_rename",
    "project_group_set_color",
    "project_group_set_collapsed",
    "project_group_set_members",
    "project_group_delete",
    "project_group_reorder",
];

const INITIAL_SLOT_SIZES: [f32; 2] = [50.0, 50.0];
const UPDATED_SLOT_SIZES: [f32; 2] = [30.0, 70.0];

struct Fixture(AppState);

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-project-actions-{}", Uuid::new_v4()));
        Self(AppState::new(AppPaths::new(dir)))
    }

    fn insert_project(&self, name: &str) -> Project {
        let project = Project {
            id: ProjectId::new(),
            root: self.0.paths.data_dir.join(name).to_string_lossy().into_owned(),
            name: name.to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        self.0.session.write().projects.push(ProjectRef {
            id: project.id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
            display: project.display.clone(),
            root_missing: false,
        });
        self.0.projects.write().insert(project.id.clone(), project.clone());
        project
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.0.paths.data_dir).expect("직접 만든 fixture만 정리");
        }
    }
}

struct InspectingSink {
    state: AppState,
    can_begin_mutation_on_publish: bool,
    events: Mutex<Vec<AppEvent>>,
}

impl InspectingSink {
    fn new(state: &AppState, can_begin_mutation_on_publish: bool) -> Self {
        Self {
            state: state.clone(),
            can_begin_mutation_on_publish,
            events: Mutex::new(Vec::new()),
        }
    }
}

impl EventSink for InspectingSink {
    fn publish(&self, event: AppEvent) {
        let matches_snapshot = {
            let session = self.state.session.read();
            match &event {
                AppEvent::ProjectListChanged { projects } => projects == &session.projects,
                AppEvent::ProjectGroupsChanged { groups } => groups == &session.groups,
                AppEvent::SessionShellSlotsChanged { tree, focused } => {
                    tree == &session.shell_slots && focused == &session.focused_shell_slot
                }
                AppEvent::WindowChromeChanged { chrome } => chrome == &session.window_chrome,
                AppEvent::ProjectActivated { project_id } => project_id == &session.active_project,
                AppEvent::ProjectRecentCleared { .. } => true,
                _ => false,
            }
        };
        assert!(matches_snapshot, "발행 전에 live state가 반영되어야 한다");
        assert!(
            self.state.session.try_write().is_some(),
            "snapshot read lock은 발행 전에 해제되어야 한다"
        );
        assert_eq!(
            self.state.begin_mutation().now_or_never().is_some(),
            self.can_begin_mutation_on_publish
        );
        if self.state.paths.session_file().exists() {
            assert_eq!(
                taide_project::service::load_session(&self.state.paths).unwrap().0,
                *self.state.session.read(),
                "발행 전에 live session과 같은 상태가 저장되어야 한다",
            );
        }
        self.events.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn 여섯_조회는_현재_공유_state를_읽고_빈_경로를_생성하지_않는다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    assert!(project_actions::project_list(state).await.unwrap().is_empty());
    assert!(project_actions::project_list_recent(state).await.unwrap().is_empty());
    assert!(project_actions::project_get_active(state).await.unwrap().is_none());
    assert!(project_actions::project_group_list(state).await.unwrap().is_empty());
    let shell = project_actions::session_get_shell_state(state).await.unwrap();
    assert_eq!(shell, taide_project::service::shell_state(&state.session.read()));
    let error = project_actions::project_get(state, ProjectId::from("prj-missing".to_string()))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 그룹_일곱_변경은_저장과_state_반영_뒤_guard를_해제하고_snapshot을_발행한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let sink = InspectingSink::new(state, true);
    let first = project_actions::project_group_create(&sink, state, "first".to_string(), None, None)
        .await
        .unwrap();
    let second = project_actions::project_group_create(&sink, state, "second".to_string(), None, None)
        .await
        .unwrap();
    project_actions::project_group_rename(&sink, state, first.id.clone(), "renamed".to_string())
        .await
        .unwrap();
    project_actions::project_group_set_color(&sink, state, first.id.clone(), Some("lane1".to_string()))
        .await
        .unwrap();
    project_actions::project_group_set_collapsed(&sink, state, first.id.clone(), true)
        .await
        .unwrap();
    project_actions::project_group_set_members(&sink, state, first.id.clone(), Vec::new())
        .await
        .unwrap();
    project_actions::project_group_reorder(&sink, state, vec![second.id.clone(), first.id.clone()])
        .await
        .unwrap();
    let groups = project_actions::project_group_list(state).await.unwrap();
    assert_eq!(
        groups.iter().map(|group| group.id.clone()).collect::<Vec<_>>(),
        [second.id.clone(), first.id.clone()]
    );
    let updated = groups.iter().find(|group| group.id == first.id).unwrap();
    assert_eq!(updated.name, "renamed");
    assert_eq!(updated.color.as_deref(), Some("lane1"));
    assert!(updated.collapsed);
    project_actions::project_group_delete(&sink, state, second.id).await.unwrap();
    let persisted = taide_project::service::load_session(&state.paths).unwrap().0;
    assert_eq!(persisted.groups, state.session.read().groups);
    assert!(sink
        .events
        .lock()
        .unwrap()
        .iter()
        .all(|event| matches!(event, AppEvent::ProjectGroupsChanged { .. })));
}

#[tokio::test]
async fn 프로젝트_정렬은_원래_mutation_guard를_이벤트까지_유지한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let sink = InspectingSink::new(state, false);
    project_actions::project_reorder(&sink, state, Vec::new()).await.unwrap();
    assert_eq!(
        *sink.events.lock().unwrap(),
        [AppEvent::ProjectListChanged { projects: Vec::new() }]
    );
    assert!(taide_project::service::load_session(&state.paths).unwrap().0.projects.is_empty());
}

#[tokio::test]
async fn 활성화와_display는_저장과_state를_반영하고_기존_guard를_이벤트까지_유지한다() {
    let fixture = Fixture::new();
    let project = fixture.insert_project("owned-root");
    let state = &fixture.0;
    let sink = InspectingSink::new(state, false);
    project_actions::project_activate(&sink, state, project.id.clone()).await.unwrap();
    project_actions::project_set_display(
        &sink,
        state,
        project.id.clone(),
        ProjectDisplayPatch {
            icon: None,
            label: Some("test".to_string()),
            color: None,
        },
    )
    .await
    .unwrap();
    let live = project_actions::project_get(state, project.id.clone()).await.unwrap();
    assert_eq!(live.display.label.as_deref(), Some("test"));
    assert!(live.last_opened_at > project.last_opened_at);
    assert_eq!(
        taide_project::service::load_project(&state.paths, &project.id).unwrap().0,
        Some(live)
    );
    assert_eq!(taide_project::service::load_session(&state.paths).unwrap().0, *state.session.read());
    let events = sink.events.lock().unwrap();
    assert!(matches!(
        &events[..],
        [
            AppEvent::ProjectActivated { .. },
            AppEvent::SessionShellSlotsChanged { .. },
            AppEvent::ProjectListChanged { .. }
        ]
    ));
    assert!(
        !std::path::Path::new(&project.root).exists(),
        "실제 프로젝트나 capability를 생성하지 않는다"
    );
}

#[tokio::test]
async fn 슬롯_focus_resize_close는_저장_뒤_guard를_해제하고_기존_이벤트_순서를_유지한다() {
    let fixture = Fixture::new();
    let first = fixture.insert_project("first-root");
    let second = fixture.insert_project("second-root");
    let first_slot = ShellSlotId::new();
    let second_slot = ShellSlotId::new();
    let state = &fixture.0;
    {
        let mut session = state.session.write();
        session.shell_slots = Some(ShellSlotTree::Split {
            dir: SplitDir::Horizontal,
            children: vec![
                ShellSlotTree::Leaf {
                    slot_id: first_slot.clone(),
                    project_id: first.id.clone(),
                },
                ShellSlotTree::Leaf {
                    slot_id: second_slot.clone(),
                    project_id: second.id.clone(),
                },
            ],
            sizes: INITIAL_SLOT_SIZES.to_vec(),
        });
        session.focused_shell_slot = Some(first_slot.clone());
        session.active_project = Some(first.id.clone());
    }
    let sink = InspectingSink::new(state, true);
    project_actions::session_focus_shell_slot(&sink, state, second_slot.clone())
        .await
        .unwrap();
    assert_eq!(state.session.read().active_project.as_ref(), Some(&second.id));
    project_actions::session_set_shell_slot_sizes(&sink, state, Vec::new(), UPDATED_SLOT_SIZES.to_vec())
        .await
        .unwrap();
    assert!(matches!(state.session.read().shell_slots.as_ref(), Some(ShellSlotTree::Split { sizes, .. }) if sizes == &UPDATED_SLOT_SIZES));
    project_actions::shell_slot_close(&sink, state, second_slot).await.unwrap();
    assert_eq!(state.session.read().focused_shell_slot.as_ref(), Some(&first_slot));
    assert_eq!(state.session.read().active_project.as_ref(), Some(&first.id));
    assert_eq!(state.projects.read().len(), 2, "슬롯 닫기는 프로젝트를 닫지 않는다");
    assert_eq!(taide_project::service::load_session(&state.paths).unwrap().0, *state.session.read());
    let events = sink.events.lock().unwrap();
    assert!(matches!(
        &events[..],
        [
            AppEvent::ProjectActivated { .. },
            AppEvent::SessionShellSlotsChanged { .. },
            AppEvent::SessionShellSlotsChanged { .. },
            AppEvent::ProjectActivated { .. },
            AppEvent::SessionShellSlotsChanged { .. }
        ]
    ));
}

#[tokio::test]
async fn chrome은_저장과_live_state_뒤_guard를_해제하고_반환과_같은_payload를_발행한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let sink = InspectingSink::new(state, true);
    let chrome = project_actions::session_set_window_chrome(
        &sink,
        state,
        WindowChromePatch {
            zen: Some(true),
            sidebar_rail_collapsed: None,
        },
    )
    .await
    .unwrap();
    assert!(chrome.zen);
    assert!(!chrome.sidebar_rail_collapsed);
    assert_eq!(*sink.events.lock().unwrap(), [AppEvent::WindowChromeChanged { chrome }]);
    assert_eq!(taide_project::service::load_session(&state.paths).unwrap().0.window_chrome, chrome);
}

#[tokio::test]
async fn 빈_recent_정리는_guard_해제_뒤_list와_결과만_발행하고_그룹을_유지한다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let group = ProjectGroup {
        id: ProjectGroupId::new(),
        name: "fixture".to_string(),
        color: None,
        members: vec![ProjectId::from("prj-missing-record".to_string())],
        collapsed: false,
    };
    state.session.write().groups = vec![group.clone()];
    let sink = InspectingSink::new(state, true);
    let result = project_actions::project_forget_recent(&sink, state).await.unwrap();
    assert_eq!(result.removed, 0);
    assert_eq!(result.skipped_with_drafts, 0);
    assert!(!result.groups_changed);
    assert_eq!(state.session.read().groups, [group]);
    assert_eq!(
        *sink.events.lock().unwrap(),
        [
            AppEvent::ProjectListChanged { projects: Vec::new() },
            AppEvent::ProjectRecentCleared {
                removed: 0,
                skipped_with_drafts: 0
            },
        ]
    );
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 미개방_프로젝트와_슬롯_오류는_저장과_이벤트를_시작하지_않는다() {
    let fixture = Fixture::new();
    let state = &fixture.0;
    let sink = InspectingSink::new(state, true);
    let project = ProjectId::from("prj-missing".to_string());
    let slot = ShellSlotId::new();
    let errors = [
        project_actions::project_activate(&sink, state, project.clone()).await.unwrap_err(),
        project_actions::project_set_display(&sink, state, project, ProjectDisplayPatch::default())
            .await
            .unwrap_err(),
        project_actions::session_focus_shell_slot(&sink, state, slot.clone())
            .await
            .unwrap_err(),
        project_actions::session_set_shell_slot_sizes(&sink, state, Vec::new(), Vec::new())
            .await
            .unwrap_err(),
        project_actions::shell_slot_close(&sink, state, slot).await.unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.kind(), AppErrorKind::NotFound);
    }
    let group_error = project_actions::project_group_create(&sink, state, String::new(), None, None)
        .await
        .unwrap_err();
    assert_eq!(group_error.kind(), AppErrorKind::InvalidArgument);
    assert!(state.session.read().groups.is_empty());
    assert!(sink.events.lock().unwrap().is_empty());
    assert!(!state.paths.data_dir.exists());
}

#[test]
fn 선택한_스물한_command는_기존_tauri_경로에서_runtime으로_위임한다() {
    let source = include_str!("../src/domain/project/commands.rs");
    for command in PROJECT_COMMANDS {
        let body = source
            .split_once(&format!("pub async fn {command}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains(&format!("project_actions::{command}(")), "{command}");
        assert!(!body.contains("state.begin_mutation()"));
    }
}

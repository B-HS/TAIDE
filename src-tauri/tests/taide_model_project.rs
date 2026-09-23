use taide_lib::domain::layout::types::{DropEdge as FacadeDropEdge, SplitDir as FacadeSplitDir};
use taide_lib::domain::project::types::{Project, ProjectGroup, SessionState, ShellSlotTree};
use taide_model::layout::{DropEdge as ModelDropEdge, SplitDir as ModelSplitDir};
use taide_model::project::{Project as ModelProject, SessionState as ModelSessionState, ShellSlotTree as ModelShellSlotTree};

fn model_session(session: SessionState) -> ModelSessionState {
    session
}

fn facade_session(session: ModelSessionState) -> SessionState {
    session
}

#[test]
fn 구버전_프로젝트와_세션은_기본값을_유지하고_공개_타입이_같다() {
    let project: ModelProject = serde_json::from_value(serde_json::json!({
        "id": "prj-old", "root": "/workspace", "name": "Legacy"
    }))
    .expect("기존 project.json");
    let facade: Project = project.clone();
    assert!(facade.capabilities.is_empty());
    assert!(!facade.root_missing);
    assert_eq!(facade.last_opened_at, 0.0);
    assert_eq!(facade.display.icon, None);

    let legacy = serde_json::json!({
        "version": 1,
        "projects": [{ "id": "prj-old", "root": "/workspace", "name": "Legacy" }],
        "activeProject": "prj-old"
    });
    let session: ModelSessionState = serde_json::from_value(legacy).expect("기존 session.json");
    let facade = facade_session(model_session(session));
    assert_eq!(facade.projects.len(), 1);
    assert_eq!(facade.projects[0].display.icon, None);
    assert!(!facade.projects[0].root_missing);
    assert!(facade.shell_slots.is_none());
    assert!(facade.focused_shell_slot.is_none());
    assert!(!facade.window_chrome.zen);
    assert!(facade.groups.is_empty());
    assert_eq!(ModelSessionState::default().version, 1);
}

#[test]
fn 셸_슬롯의_방향은_layout과_model에서_동일하다() {
    let dir: ModelSplitDir = FacadeSplitDir::Horizontal;
    let _: FacadeSplitDir = dir;
    let edge: ModelDropEdge = FacadeDropEdge::Center;
    let _: FacadeDropEdge = edge;

    let wire = serde_json::json!({
        "node": "split", "dir": "horizontal", "sizes": [50.0, 50.0],
        "children": [
            { "node": "leaf", "slotId": "slot-1", "projectId": "prj-first" },
            { "node": "leaf", "slotId": "slot-2", "projectId": "prj-second" }
        ]
    });
    let tree: ModelShellSlotTree = serde_json::from_value(wire.clone()).expect("기존 셸 슬롯 트리");
    let _: ShellSlotTree = tree.clone();
    assert_eq!(serde_json::to_value(tree).expect("셸 슬롯 트리 직렬화"), wire);
}

#[test]
fn 구버전_프로젝트_그룹은_선택_필드를_기본값으로_읽는다() {
    let group: taide_model::project::ProjectGroup = serde_json::from_value(serde_json::json!({
        "id": "grp-old", "name": "Legacy"
    }))
    .expect("기존 프로젝트 그룹");
    let facade: ProjectGroup = group;
    assert!(facade.members.is_empty());
    assert!(!facade.collapsed);
    assert!(facade.color.is_none());
}

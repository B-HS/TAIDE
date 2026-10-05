use serde_json::{Value, json};
use taide_model::ids::{PaneId, ProjectGroupId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{DropEdge, LAYOUT_SCHEMA_VERSION};
use taide_model::project::WindowChromePatch;
use taide_model::settings::Settings;
use taide_native_ui::commands::ShellMutation;
use taide_remote_web::shell::{Failure, Read, ShellState, mutation_call};
use taide_remote_web::{InvokeError, ResponsePayload};

const PROJECT: &str = "prj-synthetic";
const PANE: &str = "pane-synthetic";
const SLOT: &str = "slot-synthetic";
const BOOT_SEQ: u32 = 1;
const GROUPS_SEQ: u32 = 2;
const SESSION_SEQ: u32 = 3;
const SETTINGS_SEQ: u32 = 4;
const LAYOUT_SEQ: u32 = 5;
const LATE_SEQ: u32 = 6;
const CURRENT_SEQ: u32 = 7;
const FIRST_REVISION: u32 = 1;
const NEXT_REVISION: u32 = 2;
const FRESH_REVISION: u32 = 3;
const COALESCED_EVENT_COUNT: u32 = 3;
const BOOT_READ_COUNT: usize = 4;

fn project() -> ProjectId {
    ProjectId(PROJECT.into())
}

fn layout(revision: u32) -> Value {
    json!({"version": LAYOUT_SCHEMA_VERSION, "root": {"node": "leaf", "id": PANE, "tabs": [], "active": null}, "focusedPane": PANE, "revision": revision})
}

fn complete(state: &mut ShellState, read: Read, seq: u32, value: Value) {
    state.sent(read, seq);
    assert!(state.response(seq, &Ok(ResponsePayload::Json(value))));
    assert!(state.failures().is_empty(), "{:?}", state.failures());
}

fn ready() -> ShellState {
    let mut state = ShellState::default();
    state.refresh();
    complete(
        &mut state,
        Read::Projects,
        BOOT_SEQ,
        json!([{"id": PROJECT, "root": "/synthetic", "name": "Synthetic"}]),
    );
    complete(&mut state, Read::Groups, GROUPS_SEQ, json!([]));
    complete(
        &mut state,
        Read::Session,
        SESSION_SEQ,
        json!({"tree": {"node": "leaf", "slotId": SLOT, "projectId": PROJECT}, "focused": SLOT, "windowChrome": {"zen": false, "sidebarRailCollapsed": false}}),
    );
    complete(
        &mut state,
        Read::Settings,
        SETTINGS_SEQ,
        serde_json::to_value(Settings::default()).unwrap(),
    );
    assert!(state.snapshot().is_none());
    complete(
        &mut state,
        Read::Layout(project()),
        LAYOUT_SEQ,
        layout(FIRST_REVISION),
    );
    assert!(state.snapshot().is_some());
    assert!(state.failures().is_empty());
    state
}

#[test]
fn 설정파일_열기_ack는_제거된_project와_과거_layout을_덮어쓰지_않는다() {
    let mut state = ready();
    let fresh = serde_json::from_value(layout(FRESH_REVISION)).unwrap();
    assert!(state.layout_updated(project(), fresh));
    assert_eq!(
        state.snapshot().unwrap().layouts[&project()].revision,
        FRESH_REVISION
    );
    let old = serde_json::from_value(layout(FIRST_REVISION)).unwrap();
    assert!(!state.layout_updated(project(), old));
    let foreign = serde_json::from_value(layout(FRESH_REVISION)).unwrap();
    assert!(!state.layout_updated(ProjectId("removed-project".into()), foreign));
    assert_eq!(
        state.snapshot().unwrap().layouts[&project()].revision,
        FRESH_REVISION
    );
    complete(&mut state, Read::Projects, CURRENT_SEQ, json!([]));
    let removed = serde_json::from_value(layout(FRESH_REVISION)).unwrap();
    assert!(!state.layout_updated(project(), removed));
    assert!(!state.snapshot().unwrap().layouts.contains_key(&project()));
}

#[test]
fn 실제_model_응답은_공유_snapshot과_기존_read_명령에_연결된다() {
    let mut state = ShellState::default();
    state.refresh();
    assert_eq!(
        state.next_reads(),
        vec![Read::Projects, Read::Groups, Read::Session, Read::Settings]
    );
    for (read, command) in [
        (Read::Projects, "project_list"),
        (Read::Groups, "project_group_list"),
        (Read::Session, "session_get_shell_state"),
        (Read::Settings, "settings_get"),
    ] {
        let call = read.call();
        assert_eq!(call.command, command);
        assert_eq!(call.args, Value::Null);
    }
    let call = Read::Layout(project()).call();
    assert_eq!(call.command, "layout_get");
    assert_eq!(call.args, json!({"projectId": PROJECT}));
    let state = ready();
    let snapshot = state.snapshot().unwrap();
    assert_eq!(snapshot.focused_project(), Some(&project()));
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.layouts[&project()].revision, FIRST_REVISION);
    assert_eq!(
        snapshot.resizer_thickness,
        Settings::default().resizer_thickness as f32
    );
    assert_eq!(
        snapshot.hide_status_in_zen,
        Settings::default().zen_hide_status_bar
    );
    assert!(state.next_reads().is_empty());
}

#[test]
fn 이벤트는_inflight_갱신을_합치고_늦은_settings와_구_revision을_막는다() {
    let mut state = ready();
    let read = Read::Layout(project());
    state.event(
        "layout:changed",
        &json!({"projectId": PROJECT, "revision": FIRST_REVISION}).to_string(),
    );
    assert!(state.next_reads().is_empty());
    state.event(
        "layout:changed",
        &json!({"projectId": PROJECT, "revision": NEXT_REVISION}).to_string(),
    );
    assert_eq!(state.next_reads(), vec![read.clone()]);
    state.sent(read.clone(), LATE_SEQ);
    for _ in 0..COALESCED_EVENT_COUNT {
        state.event(
            "layout:changed",
            &json!({"projectId": PROJECT, "revision": FRESH_REVISION}).to_string(),
        );
    }
    assert!(state.next_reads().is_empty());
    assert!(state.response(LATE_SEQ, &Ok(ResponsePayload::Json(layout(NEXT_REVISION)))));
    assert_eq!(state.next_reads(), vec![read.clone()]);
    complete(
        &mut state,
        read.clone(),
        CURRENT_SEQ,
        layout(FRESH_REVISION),
    );
    complete(&mut state, read, LATE_SEQ, layout(FIRST_REVISION));
    assert_eq!(
        state.snapshot().unwrap().layouts[&project()].revision,
        FRESH_REVISION
    );

    state.sent(Read::Settings, LATE_SEQ);
    let settings = Settings {
        zen_hide_status_bar: !Settings::default().zen_hide_status_bar,
        ..Settings::default()
    };
    state.event(
        "settings:changed",
        &json!({"settings": settings}).to_string(),
    );
    assert!(!state.response(
        LATE_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(Settings::default()).unwrap()
        ))
    ));
    assert_eq!(
        state.snapshot().unwrap().hide_status_in_zen,
        settings.zen_hide_status_bar
    );
    state.event("layout:changed", "{broken");
    state.event(
        "layout:changed",
        &json!({"projectId": "prj-not-open", "revision": FRESH_REVISION}).to_string(),
    );
    assert!(state.next_reads().is_empty());
    state.event("project:groups-changed", "{}");
    state.event("session:window-chrome-changed", "{}");
    state.event("session:shell-slots-changed", "{}");
    assert_eq!(state.next_reads(), vec![Read::Groups, Read::Session]);
}

#[test]
fn 닫힌_project와_끊긴_연결의_응답은_폐기하고_오류는_자동_재실행하지_않는다() {
    let mut state = ready();
    state.sent(Read::Layout(project()), LATE_SEQ);
    complete(&mut state, Read::Projects, CURRENT_SEQ, json!([]));
    assert!(!state.response(LATE_SEQ, &Ok(ResponsePayload::Json(layout(FRESH_REVISION)))));
    assert!(state.snapshot().unwrap().layouts.is_empty());
    assert!(state.next_reads().is_empty());

    state.sent(Read::Groups, LATE_SEQ);
    state.disconnected();
    assert!(!state.response(LATE_SEQ, &Ok(ResponsePayload::Json(json!([])))));
    assert!(state.next_reads().is_empty());
    state.refresh();
    assert_eq!(state.next_reads().len(), BOOT_READ_COUNT);
    state.sent(Read::Groups, CURRENT_SEQ);
    assert!(state.response(CURRENT_SEQ, &Ok(ResponsePayload::Binary(vec![]))));
    assert_eq!(
        state.failures().get(&Read::Groups),
        Some(&Failure::MalformedResponse)
    );
    assert!(!state.next_reads().contains(&Read::Groups));
    state.invocation_failed(Read::Settings, InvokeError::Closed);
    assert_eq!(
        state.failures().get(&Read::Settings),
        Some(&Failure::Invocation(InvokeError::Closed))
    );
    state.sent(Read::Session, LATE_SEQ);
    let error = json!({"code": "Forbidden", "message": "synthetic"});
    assert!(state.response(LATE_SEQ, &Err(error.clone())));
    assert_eq!(
        state.failures().get(&Read::Session),
        Some(&Failure::Remote(error))
    );
    assert!(state.snapshot().is_some());
}

#[test]
fn 셸_mutation_15종은_기존_backend_인자와_null_patch를_유지한다() {
    let pane = PaneId(PANE.into());
    let tab = TabId("tab-synthetic".into());
    let slot = ShellSlotId(SLOT.into());
    let group = ProjectGroupId("group-synthetic".into());
    let cases = vec![
        (
            ShellMutation::ActivateProject(project()),
            "project_activate",
            json!({"projectId": PROJECT}),
        ),
        (
            ShellMutation::FocusSlot(slot.clone()),
            "session_focus_shell_slot",
            json!({"slotId": SLOT}),
        ),
        (
            ShellMutation::CloseSlot(slot),
            "shell_slot_close",
            json!({"slotId": SLOT}),
        ),
        (
            ShellMutation::SetGroupCollapsed {
                group,
                collapsed: true,
            },
            "project_group_set_collapsed",
            json!({"groupId": "group-synthetic", "collapsed": true}),
        ),
        (
            ShellMutation::ResizeSlots {
                path: vec![0],
                sizes: vec![1.0],
            },
            "session_set_shell_slot_sizes",
            json!({"path": [0], "sizes": [1.0]}),
        ),
        (
            ShellMutation::SetWindowChrome(WindowChromePatch {
                zen: Some(true),
                sidebar_rail_collapsed: None,
            }),
            "session_set_window_chrome",
            json!({"patch": {"zen": true, "sidebarRailCollapsed": null}}),
        ),
        (
            ShellMutation::SetSidebarCollapsed {
                project: project(),
                collapsed: true,
            },
            "layout_set_shell_view",
            json!({"projectId": PROJECT, "patch": {"zen": null, "sidebarCollapsed": true}}),
        ),
        (
            ShellMutation::ActivateTab(tab.clone()),
            "layout_activate_tab",
            json!({"tabId": tab}),
        ),
        (
            ShellMutation::ReopenClosed(project()),
            "layout_reopen_closed",
            json!({"projectId": PROJECT}),
        ),
        (
            ShellMutation::FocusPane(pane.clone()),
            "layout_focus_pane",
            json!({"paneId": PANE}),
        ),
        (
            ShellMutation::MoveTab {
                tab: tab.clone(),
                pane: pane.clone(),
                index: 0,
            },
            "layout_move_tab",
            json!({"tabId": tab, "paneId": PANE, "index": 0}),
        ),
        (
            ShellMutation::PinTab {
                tab: tab.clone(),
                pinned: true,
            },
            "layout_pin_tab",
            json!({"tabId": tab, "pinned": true}),
        ),
        (
            ShellMutation::KeepTab(tab.clone()),
            "layout_set_preview",
            json!({"tabId": tab, "preview": false}),
        ),
        (
            ShellMutation::SplitTab {
                pane: pane.clone(),
                edge: DropEdge::Right,
                tab: tab.clone(),
            },
            "layout_split",
            json!({"paneId": PANE, "edge": "right", "tabId": tab}),
        ),
        (
            ShellMutation::ResizePane {
                pane,
                sizes: vec![1.0],
            },
            "layout_resize",
            json!({"paneId": PANE, "sizes": [1.0]}),
        ),
    ];
    for (mutation, command, args) in cases {
        let call = mutation_call(&mutation);
        assert_eq!(call.command, command);
        assert_eq!(call.args, args);
    }
}

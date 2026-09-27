use std::collections::HashSet;

use taide_infra::perf::{self, SpanSlot};
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::project::{
    ForgetRecentOutcome, Project, ProjectDisplayPatch, ProjectGroup, ProjectRef, SessionShellState, WindowChrome, WindowChromePatch,
};
use taide_project::service;

use crate::{AppState, EventSink};

/// Publishes the current project list after releasing its snapshot lock.
pub fn emit_list_changed(events: &dyn EventSink, state: &AppState) {
    let projects = service::list_projects(&state.session.read());
    events.publish(AppEvent::ProjectListChanged { projects });
}

/// Publishes the current shell-slot snapshot after releasing the session lock.
pub fn emit_shell_slots_changed(events: &dyn EventSink, state: &AppState) {
    let payload = {
        let session = state.session.read();
        AppEvent::SessionShellSlotsChanged {
            tree: session.shell_slots.clone(),
            focused: session.focused_shell_slot.clone(),
        }
    };
    events.publish(payload);
}

/// Publishes the current project groups after releasing their snapshot lock.
pub fn emit_groups_changed(events: &dyn EventSink, state: &AppState) {
    let groups = service::list_groups(&state.session.read());
    events.publish(AppEvent::ProjectGroupsChanged { groups });
}

/// Applies the shared project list policy.
pub async fn project_list(state: &AppState) -> AppResult<Vec<ProjectRef>> {
    Ok(service::list_projects(&state.session.read()))
}

/// Applies the shared project list recent policy.
pub async fn project_list_recent(state: &AppState) -> AppResult<Vec<Project>> {
    service::list_recent_projects(&state.paths)
}

/// Applies the shared project forget recent policy.
pub async fn project_forget_recent(events: &dyn EventSink, state: &AppState) -> AppResult<ForgetRecentOutcome> {
    let outcome = {
        let _guard = state.begin_mutation().await;
        let open_ids: HashSet<ProjectId> = state.projects.read().keys().cloned().collect();
        let mut session = state.session.read().clone();
        let outcome = service::forget_recent_projects(&state.paths, &mut session, &open_ids)?;
        *state.session.write() = session;
        outcome
    };

    emit_list_changed(events, state);
    if outcome.groups_changed {
        emit_groups_changed(events, state);
    }
    events.publish(AppEvent::ProjectRecentCleared {
        removed: outcome.removed,
        skipped_with_drafts: outcome.skipped_with_drafts,
    });

    Ok(outcome)
}

/// Applies the shared project get policy.
pub async fn project_get(state: &AppState, project_id: ProjectId) -> AppResult<Project> {
    service::get_project(&state.projects.read(), &project_id)
}

/// Applies the shared project get active policy.
pub async fn project_get_active(state: &AppState) -> AppResult<Option<ProjectId>> {
    Ok(state.session.read().active_project.clone())
}

/// Applies the shared session get shell state policy.
pub async fn session_get_shell_state(state: &AppState) -> AppResult<SessionShellState> {
    Ok(service::shell_state(&state.session.read()))
}

/// Applies the shared session focus shell slot policy.
pub async fn session_focus_shell_slot(events: &dyn EventSink, state: &AppState, slot_id: ShellSlotId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::focus_shell_slot(&state.paths, &mut session, &mut projects, &slot_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    *state.projects.write() = projects;
    drop(_guard);

    events.publish(AppEvent::ProjectActivated {
        project_id: active_project,
    });
    emit_shell_slots_changed(events, state);

    Ok(())
}

/// Applies the shared session set shell slot sizes policy.
pub async fn session_set_shell_slot_sizes(events: &dyn EventSink, state: &AppState, path: Vec<u32>, sizes: Vec<f32>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_shell_slot_sizes(&state.paths, &mut session, &path, sizes)?;

    *state.session.write() = session;
    drop(_guard);

    emit_shell_slots_changed(events, state);

    Ok(())
}

/// Applies the shared shell slot close policy.
pub async fn shell_slot_close(events: &dyn EventSink, state: &AppState, slot_id: ShellSlotId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::close_shell_slot(&state.paths, &mut session, &slot_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    drop(_guard);

    events.publish(AppEvent::ProjectActivated {
        project_id: active_project,
    });
    emit_shell_slots_changed(events, state);

    Ok(())
}

/// Applies the shared session set window chrome policy.
pub async fn session_set_window_chrome(events: &dyn EventSink, state: &AppState, patch: WindowChromePatch) -> AppResult<WindowChrome> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    let chrome = service::set_window_chrome(&state.paths, &mut session, &patch)?;

    *state.session.write() = session;
    drop(_guard);

    events.publish(AppEvent::WindowChromeChanged { chrome });

    Ok(chrome)
}

/// Applies the shared project activate policy.
pub async fn project_activate(events: &dyn EventSink, state: &AppState, project_id: ProjectId) -> AppResult<()> {
    let _span = perf::span(SpanSlot::ProjectActivate);
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::activate_project(&state.paths, &mut session, &mut projects, &project_id)?;

    *state.session.write() = session;
    *state.projects.write() = projects;

    events.publish(AppEvent::ProjectActivated {
        project_id: Some(project_id),
    });
    emit_shell_slots_changed(events, state);

    Ok(())
}

/// Applies the shared project reorder policy.
pub async fn project_reorder(events: &dyn EventSink, state: &AppState, ids: Vec<ProjectId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::reorder_projects(&state.paths, &mut session, &ids)?;

    *state.session.write() = session;
    emit_list_changed(events, state);

    Ok(())
}

/// Applies the shared project set display policy.
pub async fn project_set_display(
    events: &dyn EventSink,
    state: &AppState,
    project_id: ProjectId,
    patch: ProjectDisplayPatch,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::set_project_display(&state.paths, &mut session, &mut projects, &project_id, &patch)?;

    *state.session.write() = session;
    *state.projects.write() = projects;
    emit_list_changed(events, state);

    Ok(())
}

/// Applies the shared project group list policy.
pub async fn project_group_list(state: &AppState) -> AppResult<Vec<ProjectGroup>> {
    Ok(service::list_groups(&state.session.read()))
}

/// Applies the shared project group create policy.
pub async fn project_group_create(
    events: &dyn EventSink,
    state: &AppState,
    name: String,
    color: Option<String>,
    members: Option<Vec<ProjectId>>,
) -> AppResult<ProjectGroup> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    let group = service::create_group(&state.paths, &mut session, &name, color.as_deref(), members)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(group)
}

/// Applies the shared project group rename policy.
pub async fn project_group_rename(events: &dyn EventSink, state: &AppState, group_id: ProjectGroupId, name: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::rename_group(&state.paths, &mut session, &group_id, &name)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(())
}

/// Applies the shared project group set color policy.
pub async fn project_group_set_color(
    events: &dyn EventSink,
    state: &AppState,
    group_id: ProjectGroupId,
    color: Option<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_group_color(&state.paths, &mut session, &group_id, color.as_deref())?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(())
}

/// Applies the shared project group set collapsed policy.
pub async fn project_group_set_collapsed(
    events: &dyn EventSink,
    state: &AppState,
    group_id: ProjectGroupId,
    collapsed: bool,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_group_collapsed(&state.paths, &mut session, &group_id, collapsed)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(())
}

/// Applies the shared project group set members policy.
pub async fn project_group_set_members(
    events: &dyn EventSink,
    state: &AppState,
    group_id: ProjectGroupId,
    members: Vec<ProjectId>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_group_members(&state.paths, &mut session, &group_id, members)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(())
}

/// Applies the shared project group delete policy.
pub async fn project_group_delete(events: &dyn EventSink, state: &AppState, group_id: ProjectGroupId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::delete_group(&state.paths, &mut session, &group_id)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(())
}

/// Applies the shared project group reorder policy.
pub async fn project_group_reorder(events: &dyn EventSink, state: &AppState, ids: Vec<ProjectGroupId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::reorder_groups(&state.paths, &mut session, &ids)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(events, state);

    Ok(())
}

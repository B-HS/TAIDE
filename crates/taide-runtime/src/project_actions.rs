use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::Path;

use taide_infra::perf::{self, SpanSlot};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::project::{
    CapabilityKind, ForgetRecentOutcome, OpenProjectInSlotRequest, Project, ProjectDisplayPatch, ProjectGroup, ProjectGroupOpenResult,
    ProjectRef, SessionShellState, SessionState, WindowChrome, WindowChromePatch,
};
use taide_project::groups;
use taide_project::service;

use crate::{AppState, EventSink};

/// Restores boot session, layouts, legacy chrome and settings into the shared state.
pub fn restore_state(state: &AppState) -> Vec<String> {
    let mut warnings = Vec::new();

    match service::restore_session(&state.paths) {
        Ok((mut session, projects, session_warnings)) => {
            let mut layouts = state.layouts.write();
            for project in &projects {
                layouts.insert(project.id.clone(), taide_layout::service::load_layout(&state.paths, &project.id));
            }

            let mut shell_views = layouts
                .iter()
                .map(|(project_id, layout)| (project_id.clone(), layout.shell_view))
                .collect();
            let promoted = service::promote_legacy_window_chrome(&mut session, &mut shell_views);
            for project_id in &promoted {
                if let (Some(layout), Some(view)) = (layouts.get_mut(project_id), shell_views.get(project_id)) {
                    layout.shell_view = *view;
                }
            }
            drop(layouts);

            if !promoted.is_empty() {
                state.dirty_layouts.write().extend(promoted.iter().cloned());
                if let Err(error) = service::save_session(&state.paths, &session) {
                    warnings.push(format!("창 크롬 상태 승격 후 세션 저장 실패: {error}"));
                }
            }

            *state.session.write() = session;
            *state.projects.write() = projects.into_iter().map(|project| (project.id.clone(), project)).collect();
            warnings.extend(session_warnings);
        }
        Err(error) => warnings.push(format!("세션 복원 실패: {error}")),
    }

    *state.settings.write() = taide_settings::service::load_settings(&state.paths);

    warnings
}

/// Selects present-root projects in active, session and remaining-map order.
pub fn projects_pending_watcher_restore(projects: &HashMap<ProjectId, Project>, session: &SessionState) -> Vec<(ProjectId, String)> {
    let mut ordered_ids: Vec<ProjectId> = session.active_project.iter().cloned().collect();
    ordered_ids.extend(
        session
            .projects
            .iter()
            .map(|project_ref| project_ref.id.clone())
            .filter(|id| Some(id) != session.active_project.as_ref()),
    );

    let mut seen: HashSet<ProjectId> = ordered_ids.iter().cloned().collect();
    for project_id in projects.keys() {
        if seen.insert(project_id.clone()) {
            ordered_ids.push(project_id.clone());
        }
    }

    ordered_ids
        .into_iter()
        .filter_map(|project_id| {
            let project = projects.get(&project_id)?;
            (!project.root_missing).then(|| (project.id.clone(), project.root.clone()))
        })
        .collect()
}

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

/// Supplies the existing toolkit-owned capability and window-flush boundaries.
pub trait ProjectLifecyclePort: Send + Sync {
    fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind>;
    fn attach_project_capabilities(&self, project: &Project) -> impl Future<Output = AppResult<()>> + Send;
    fn await_project_flush(&self, project_id: &ProjectId) -> impl Future<Output = ()> + Send;
    fn detach_all(&self, project_id: &ProjectId);
}

/// Applies the existing project lifecycle policy through explicit host ports.
pub async fn project_open(
    events: &dyn EventSink,
    state: &AppState,
    ports: &impl ProjectLifecyclePort,
    path: String,
) -> AppResult<service::ProjectOpenResult> {
    let result = {
        let _guard = state.begin_mutation().await;
        let mut session = state.session.read().clone();
        let mut projects = state.projects.read().clone();

        let result = service::open_project(&state.paths, &mut session, &mut projects, Path::new(&path), true, |canonical| {
            ports.detected_kinds(canonical)
        })?;

        *state.session.write() = session;
        *state.projects.write() = projects;
        result
    };

    if !result.already_open {
        if let Err(error) = ports.attach_project_capabilities(&result.project).await {
            if let Err(rollback) = project_close(events, state, ports, result.project.id.clone()).await {
                log::warn!(
                    "capability attach 실패 후 프로젝트 되돌리기도 실패했습니다 (projectId={}): {rollback}",
                    result.project.id
                );
            }
            return Err(error);
        }

        events.publish(AppEvent::ProjectOpened {
            project: Box::new(result.project.clone()),
        });
        emit_list_changed(events, state);
    }

    events.publish(AppEvent::ProjectActivated {
        project_id: Some(result.project.id.clone()),
    });
    emit_shell_slots_changed(events, state);

    Ok(result)
}

/// Applies the existing project lifecycle policy through explicit host ports.
pub async fn project_open_in_slot(
    events: &dyn EventSink,
    state: &AppState,
    ports: &impl ProjectLifecyclePort,
    request: OpenProjectInSlotRequest,
) -> AppResult<SessionShellState> {
    let opened = {
        let _guard = state.begin_mutation().await;
        let mut session = state.session.read().clone();
        let mut projects = state.projects.read().clone();

        let existing = match (&request.path, &request.project_id) {
            (Some(path), None) => service::find_open_project_by_root(&projects, Path::new(path)),
            (None, Some(project_id)) => {
                service::get_project(&projects, project_id)?;
                Some(project_id.clone())
            }
            _ => {
                return Err(AppError::InvalidArgument(
                    "project_open_in_slot needs exactly one of path or projectId".to_string(),
                ));
            }
        };

        service::ensure_slot_placement_allowed(&session, existing.as_ref(), &request.target_slot, request.edge)?;

        let opened = match (existing, &request.path) {
            (Some(project_id), _) => {
                let project = service::get_project(&projects, &project_id)?;
                service::ProjectOpenResult {
                    project,
                    already_open: true,
                }
            }
            (None, Some(path)) => service::open_project(&state.paths, &mut session, &mut projects, Path::new(path), false, |canonical| {
                ports.detected_kinds(canonical)
            })?,
            (None, None) => return Err(AppError::Internal("project_open_in_slot resolved no project".to_string())),
        };

        service::place_project_in_slot(
            &state.paths,
            &mut session,
            &mut projects,
            &opened.project.id,
            &request.target_slot,
            request.edge,
        )?;

        *state.session.write() = session;
        *state.projects.write() = projects;
        opened
    };

    if !opened.already_open {
        if let Err(error) = ports.attach_project_capabilities(&opened.project).await {
            if let Err(rollback) = project_close(events, state, ports, opened.project.id.clone()).await {
                log::warn!(
                    "capability attach 실패 후 슬롯 프로젝트 되돌리기도 실패했습니다 (projectId={}): {rollback}",
                    opened.project.id
                );
            }
            return Err(error);
        }

        events.publish(AppEvent::ProjectOpened {
            project: Box::new(opened.project.clone()),
        });
        emit_list_changed(events, state);
    }

    events.publish(AppEvent::ProjectActivated {
        project_id: Some(opened.project.id.clone()),
    });
    emit_shell_slots_changed(events, state);

    Ok(service::shell_state(&state.session.read()))
}

/// Applies the existing project lifecycle policy through explicit host ports.
pub async fn project_close(
    events: &dyn EventSink,
    state: &AppState,
    ports: &impl ProjectLifecyclePort,
    project_id: ProjectId,
) -> AppResult<()> {
    if !state.projects.read().contains_key(&project_id) {
        return Err(AppError::NotFound(format!("project not open: {project_id}")));
    }

    ports.await_project_flush(&project_id).await;

    let _guard = state.begin_mutation().await;
    if !state.projects.read().contains_key(&project_id) {
        return Ok(());
    }

    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::close_project(&state.paths, &mut session, &mut projects, &project_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    *state.projects.write() = projects;

    ports.detach_all(&project_id);

    events.publish(AppEvent::ProjectClosed {
        project_id: project_id.clone(),
    });
    events.publish(AppEvent::ProjectActivated {
        project_id: active_project,
    });
    emit_shell_slots_changed(events, state);
    emit_list_changed(events, state);

    Ok(())
}

/// Applies the existing project lifecycle policy through explicit host ports.
pub async fn project_group_open(
    events: &dyn EventSink,
    state: &AppState,
    ports: &impl ProjectLifecyclePort,
    group_id: ProjectGroupId,
) -> AppResult<ProjectGroupOpenResult> {
    let members = service::group_members(&state.session.read(), &group_id)?;
    let open_ids: HashSet<ProjectId> = state.projects.read().keys().cloned().collect();
    let plan = groups::plan_open(&members, &open_ids, |project_id| {
        service::group_member_root(&state.paths, project_id)
    });

    for (project_id, reason) in &plan.skipped {
        if reason == &groups::GroupSkipReason::Unavailable {
            log::warn!("그룹 멤버의 레코드나 루트가 없어 건너뜁니다 (groupId={group_id}, projectId={project_id})");
        }
    }

    let mut result = ProjectGroupOpenResult {
        opened: Vec::new(),
        skipped: plan.skipped.into_iter().map(|(project_id, _)| project_id).collect(),
    };

    let app_state = state;
    let opening_group = &group_id;
    run_group_open_plan(
        plan.steps,
        &mut result,
        || state.is_shutting_down(),
        |step, activate| async move {
            match open_group_member(events, app_state, ports, &step.root, activate).await {
                Ok(project) => Some(project.id),
                Err(error) => {
                    log::warn!(
                        "그룹 멤버를 열지 못했습니다 (groupId={opening_group}, projectId={}): {error}",
                        step.project_id
                    );
                    None
                }
            }
        },
    )
    .await;

    Ok(result)
}
async fn open_group_member(
    events: &dyn EventSink,
    state: &AppState,
    ports: &impl ProjectLifecyclePort,
    root: &str,
    activate: bool,
) -> AppResult<Project> {
    let opened = {
        let _guard = state.begin_mutation().await;
        let mut session = state.session.read().clone();
        let mut projects = state.projects.read().clone();

        let opened = service::open_project(&state.paths, &mut session, &mut projects, Path::new(root), activate, |canonical| {
            ports.detected_kinds(canonical)
        })?;

        *state.session.write() = session;
        *state.projects.write() = projects;
        opened
    };

    if !opened.already_open {
        if let Err(error) = ports.attach_project_capabilities(&opened.project).await {
            if let Err(rollback) = project_close(events, state, ports, opened.project.id.clone()).await {
                log::warn!(
                    "capability attach 실패 후 그룹 멤버 되돌리기도 실패했습니다 (projectId={}): {rollback}",
                    opened.project.id
                );
            }
            return Err(error);
        }

        events.publish(AppEvent::ProjectOpened {
            project: Box::new(opened.project.clone()),
        });
        emit_list_changed(events, state);
    }

    if activate {
        events.publish(AppEvent::ProjectActivated {
            project_id: Some(opened.project.id.clone()),
        });
        emit_shell_slots_changed(events, state);
    }

    Ok(opened.project)
}

/// Walks the ordered group queue and transfers activation to the first successful member.
pub async fn run_group_open_plan<Open, Fut>(
    steps: Vec<groups::GroupOpenStep>,
    result: &mut ProjectGroupOpenResult,
    mut stop: impl FnMut() -> bool,
    mut open_member: Open,
) where
    Open: FnMut(groups::GroupOpenStep, bool) -> Fut,
    Fut: std::future::Future<Output = Option<ProjectId>>,
{
    let mut pending_activation = steps.iter().any(|step| step.activate);

    for step in steps {
        if stop() {
            result.skipped.push(step.project_id);
            continue;
        }

        let activate = pending_activation;
        let project_id = step.project_id.clone();
        match open_member(step, activate).await {
            Some(opened_id) => {
                if activate {
                    pending_activation = false;
                }
                result.opened.push(opened_id);
            }
            None => result.skipped.push(project_id),
        }
    }
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

use std::sync::Arc;

use serde_json::Value;
use taide_infra::perf::{self, SpanSlot};
use taide_model::error::AppError;
use taide_runtime::project_actions::ProjectLifecyclePort;
use taide_runtime::{AppServices, project_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &[
    "project_list",
    "project_get",
    "project_get_active",
    "project_open",
    "project_close",
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
    "project_group_open",
    "project_open_in_slot",
    "shell_slot_close",
    "session_get_shell_state",
    "session_focus_shell_slot",
    "session_set_shell_slot_sizes",
    "session_set_window_chrome",
];

pub fn extend_backend<P, F>(create_lifecycle: F, remaining: Dispatch) -> Dispatch
where
    P: ProjectLifecyclePort + 'static,
    F: Fn(Arc<AppServices>) -> P + Send + Sync + 'static,
{
    let create_lifecycle = Arc::new(create_lifecycle);
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let create_lifecycle = create_lifecycle.clone();
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(services, create_lifecycle, &name, &args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch<P, F>(
    services: Arc<AppServices>,
    create_lifecycle: Arc<F>,
    name: &str,
    args: &Value,
) -> Result<String, Value>
where
    P: ProjectLifecyclePort + 'static,
    F: Fn(Arc<AppServices>) -> P + Send + Sync + 'static,
{
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    let state = &services.state;
    let events = services.events.as_ref();
    match name {
        "project_list" => respond(project_actions::project_list(state).await),
        "project_get" => respond(project_actions::project_get(state, arg!("projectId")).await),
        "project_get_active" => respond(project_actions::project_get_active(state).await),
        "project_open" => {
            let path = arg!("path");
            let worker = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-project-open", async move {
                        let _span = perf::span(SpanSlot::ProjectOpen);
                        let ports = create_lifecycle(worker.clone());
                        project_actions::project_open(
                            worker.events.as_ref(),
                            &worker.state,
                            &ports,
                            path,
                        )
                        .await
                    })
                    .await,
            )
        }
        "project_open_in_slot" => {
            let request = arg!("request");
            let worker = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-project-open-in-slot", async move {
                        let _span = perf::span(SpanSlot::ProjectOpen);
                        let ports = create_lifecycle(worker.clone());
                        project_actions::project_open_in_slot(
                            worker.events.as_ref(),
                            &worker.state,
                            &ports,
                            request,
                        )
                        .await
                    })
                    .await,
            )
        }
        "project_group_open" => {
            let group = arg!("groupId");
            let worker = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-project-group-open", async move {
                        let ports = create_lifecycle(worker.clone());
                        project_actions::project_group_open(
                            worker.events.as_ref(),
                            &worker.state,
                            &ports,
                            group,
                        )
                        .await
                    })
                    .await,
            )
        }
        "project_close" => {
            let project = arg!("projectId");
            let ports = create_lifecycle(services.clone());
            respond(project_actions::project_close(events, state, &ports, project).await)
        }
        "project_activate" => {
            respond(project_actions::project_activate(events, state, arg!("projectId")).await)
        }
        "project_reorder" => {
            respond(project_actions::project_reorder(events, state, arg!("ids")).await)
        }
        "project_set_display" => respond(
            project_actions::project_set_display(events, state, arg!("projectId"), arg!("patch"))
                .await,
        ),
        "project_group_list" => respond(project_actions::project_group_list(state).await),
        "project_group_create" => respond(
            project_actions::project_group_create(
                events,
                state,
                arg!("name"),
                arg!("color"),
                arg!("members"),
            )
            .await,
        ),
        "project_group_rename" => respond(
            project_actions::project_group_rename(events, state, arg!("groupId"), arg!("name"))
                .await,
        ),
        "project_group_set_color" => respond(
            project_actions::project_group_set_color(events, state, arg!("groupId"), arg!("color"))
                .await,
        ),
        "project_group_set_collapsed" => respond(
            project_actions::project_group_set_collapsed(
                events,
                state,
                arg!("groupId"),
                arg!("collapsed"),
            )
            .await,
        ),
        "project_group_set_members" => respond(
            project_actions::project_group_set_members(
                events,
                state,
                arg!("groupId"),
                arg!("members"),
            )
            .await,
        ),
        "project_group_delete" => {
            respond(project_actions::project_group_delete(events, state, arg!("groupId")).await)
        }
        "project_group_reorder" => {
            respond(project_actions::project_group_reorder(events, state, arg!("ids")).await)
        }
        "shell_slot_close" => {
            respond(project_actions::shell_slot_close(events, state, arg!("slotId")).await)
        }
        "session_get_shell_state" => respond(project_actions::session_get_shell_state(state).await),
        "session_focus_shell_slot" => {
            respond(project_actions::session_focus_shell_slot(events, state, arg!("slotId")).await)
        }
        "session_set_shell_slot_sizes" => respond(
            project_actions::session_set_shell_slot_sizes(
                events,
                state,
                arg!("path"),
                arg!("sizes"),
            )
            .await,
        ),
        "session_set_window_chrome" => {
            respond(project_actions::session_set_window_chrome(events, state, arg!("patch")).await)
        }
        _ => Err(error_value(AppError::Internal(format!(
            "native remote project/session routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-projects-tests.rs"]
mod tests;

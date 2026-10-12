use std::sync::Arc;

use serde_json::Value;
use taide_model::error::AppError;
use taide_model::layout::{OpenTabInSplitRequest, TabKind};
use taide_runtime::{AppServices, layout_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &[
    "layout_get",
    "layout_open_tab",
    "layout_close_tab",
    "layout_activate_tab",
    "layout_move_tab",
    "layout_split",
    "layout_open_tab_in_split",
    "layout_resize",
    "layout_focus_pane",
    "layout_pin_tab",
    "layout_set_preview",
    "layout_reopen_closed",
    "layout_set_view_state",
    "layout_set_dirty",
    "layout_set_terminal_session",
    "layout_open_untitled",
    "layout_convert_untitled",
    "layout_apply_path_change",
    "layout_set_shell_view",
];

pub struct Ports {
    pub discard_terminal: Arc<dyn Fn(&str) + Send + Sync>,
    pub(crate) view_states: Arc<crate::editor_view_state_sync::Shared>,
}

impl Ports {
    pub fn new(terminals: Arc<crate::terminal_host::Hub>) -> Self {
        Self {
            discard_terminal: Arc::new(move |id| terminals.discard(id)),
            view_states: Arc::default(),
        }
    }

    pub(crate) fn with_view_states(
        mut self,
        view_states: Arc<crate::editor_view_state_sync::Shared>,
    ) -> Self {
        self.view_states = view_states;
        self
    }
}

pub fn extend_backend(ports: Ports, remaining: Dispatch) -> Dispatch {
    let ports = Arc::new(ports);
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let ports = ports.clone();
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(&services, &ports, &name, &args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch(
    services: &AppServices,
    ports: &Ports,
    name: &str,
    args: &Value,
) -> Result<String, Value> {
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    macro_rules! mutate {
        ($mutation:expr) => {{
            let mutation = $mutation;
            ports
                .view_states
                .flush(services)
                .await
                .map_err(error_value)?;
            respond(mutation.await)
        }};
    }
    let state = &services.state;
    let events = services.events.as_ref();
    match name {
        "layout_get" => respond(layout_actions::layout_get(state, arg!("projectId")).await),
        "layout_open_tab" => {
            let project = arg!("projectId");
            let kind = arg!("kind");
            let title = arg!("title");
            let target = arg!("target");
            let preview = arg!("preview");
            layout_actions::ensure_file_tab_target_exists(
                &state.projects.read(),
                &state.cli_opened_paths.read(),
                &kind,
            )
            .map_err(error_value)?;
            mutate!(layout_actions::layout_open_tab(
                events, state, project, kind, title, target, preview
            ))
        }
        "layout_close_tab" => {
            let id = arg!("tabId");
            taide_layout::service::locate_project_with_tab(&state.layouts.read(), &id)
                .map_err(error_value)?;
            ports
                .view_states
                .flush(services)
                .await
                .map_err(error_value)?;
            let result = layout_actions::close_tab_and_finish(events, state, &id, |tab| {
                services.ide.reconcile_closed_tab(tab);
                if let TabKind::Terminal { session_id, .. } = &tab.kind {
                    services.terminal.kill_session(session_id);
                    (ports.discard_terminal)(session_id);
                }
            })
            .await
            .map(|(_, _, layout)| layout);
            respond(result)
        }
        "layout_activate_tab" => {
            mutate!(layout_actions::layout_activate_tab(
                events,
                state,
                arg!("tabId")
            ))
        }
        "layout_move_tab" => mutate!(layout_actions::layout_move_tab(
            events,
            state,
            arg!("tabId"),
            arg!("paneId"),
            arg!("index"),
        )),
        "layout_split" => mutate!(layout_actions::layout_split(
            events,
            state,
            arg!("paneId"),
            arg!("edge"),
            arg!("tabId"),
        )),
        "layout_open_tab_in_split" => {
            let request: OpenTabInSplitRequest = arg!("request");
            layout_actions::ensure_file_tab_target_exists(
                &state.projects.read(),
                &state.cli_opened_paths.read(),
                &request.kind,
            )
            .map_err(error_value)?;
            mutate!(layout_actions::layout_open_tab_in_split(
                events, state, request
            ))
        }
        "layout_resize" => respond(
            layout_actions::layout_resize(events, state, arg!("paneId"), arg!("sizes")).await,
        ),
        "layout_focus_pane" => {
            respond(layout_actions::layout_focus_pane(events, state, arg!("paneId")).await)
        }
        "layout_pin_tab" => respond(
            layout_actions::layout_pin_tab(events, state, arg!("tabId"), arg!("pinned")).await,
        ),
        "layout_set_preview" => respond(
            layout_actions::layout_set_preview(events, state, arg!("tabId"), arg!("preview")).await,
        ),
        "layout_reopen_closed" => {
            mutate!(layout_actions::layout_reopen_closed(
                events,
                state,
                arg!("projectId")
            ))
        }
        "layout_set_view_state" => respond(
            layout_actions::layout_set_view_state(events, state, arg!("tabId"), arg!("viewState"))
                .await,
        ),
        "layout_set_dirty" => respond(
            layout_actions::layout_set_dirty(events, state, arg!("tabId"), arg!("dirty")).await,
        ),
        "layout_set_terminal_session" => respond(
            layout_actions::layout_set_terminal_session(
                events,
                state,
                arg!("tabId"),
                arg!("sessionId"),
            )
            .await,
        ),
        "layout_open_untitled" => mutate!(layout_actions::layout_open_untitled(
            events,
            state,
            arg!("projectId"),
            arg!("target")
        )),
        "layout_convert_untitled" => mutate!(layout_actions::layout_convert_untitled(
            events,
            state,
            arg!("tabId"),
            arg!("path")
        )),
        "layout_apply_path_change" => mutate!(layout_actions::layout_apply_path_change(
            events,
            state,
            arg!("projectId"),
            arg!("change"),
        )),
        "layout_set_shell_view" => respond(
            layout_actions::layout_set_shell_view(events, state, arg!("projectId"), arg!("patch"))
                .await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote layout routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-layout-tests.rs"]
mod tests;

use std::sync::Arc;

use serde_json::Value;
use taide_infra::perf::{self, SpanSlot};
use taide_model::error::AppError;
use taide_runtime::{AppServices, file_actions, tree_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const JSON_COMMANDS: &[&str] = &[
    "file_open",
    "file_save",
    "file_create",
    "file_rename",
    "file_delete",
    "file_copy",
    "file_mirror_dirty",
    "file_list_mirrors",
    "file_clear_mirror",
    "file_prune_mirrors",
    "file_mirror_untitled",
    "file_list_untitled_mirrors",
    "file_clear_untitled_mirror",
    "file_prune_untitled_mirrors",
    "tree_rows",
    "tree_toggle",
    "tree_collapse_all",
    "tree_reveal",
    "tree_refresh",
];

pub fn extend_backend(remaining: Dispatch) -> Dispatch {
    let remaining_json = remaining.json;
    let remaining_raw = remaining.raw;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if JSON_COMMANDS.contains(&name.as_str()) {
                    return dispatch(&services, &name, &args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: Arc::new(move |services, name, args| {
            let remaining = remaining_raw.clone();
            Box::pin(async move {
                if name == "file_read_raw" {
                    return file_actions::file_read_raw(
                        &services.state,
                        argument(&args, "path").map_err(error_value)?,
                    )
                    .await
                    .map_err(error_value);
                }
                remaining(services, name, args).await
            })
        }),
    }
}

async fn dispatch(services: &AppServices, name: &str, args: &Value) -> Result<String, Value> {
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    let state = &services.state;
    let tasks = &services.tasks;
    match name {
        "file_open" => respond(
            file_actions::file_open(state, tasks, arg!("path"), || {
                taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(
                    &services.plugin,
                    &state.paths.plugins_dir(),
                ))
            })
            .await,
        ),
        "file_save" => {
            respond(file_actions::file_save(state, tasks, arg!("path"), arg!("content")).await)
        }
        "file_create" => {
            respond(file_actions::file_create(state, arg!("path"), arg!("isDir")).await)
        }
        "file_rename" => respond(file_actions::file_rename(state, arg!("from"), arg!("to")).await),
        "file_delete" => respond(file_actions::file_delete(state, arg!("path")).await),
        "file_copy" => {
            respond(file_actions::file_copy(state, tasks, arg!("from"), arg!("to")).await)
        }
        "file_mirror_dirty" => {
            let has_receipt: bool = if args.get("receipt").is_some() {
                arg!("receipt")
            } else {
                false
            };
            if has_receipt {
                return respond(
                    file_actions::file_mirror_dirty_with_receipt(
                        state,
                        tasks,
                        arg!("projectId"),
                        arg!("path"),
                        arg!("content"),
                    )
                    .await,
                );
            }
            respond(
                file_actions::file_mirror_dirty(
                    state,
                    tasks,
                    arg!("projectId"),
                    arg!("path"),
                    arg!("content"),
                )
                .await,
            )
        }
        "file_list_mirrors" => {
            respond(file_actions::file_list_mirrors(state, arg!("projectId")).await)
        }
        "file_clear_mirror" => {
            if args.get("expectedReceipt").is_some() {
                if args.get("expected").is_some() {
                    return respond::<()>(Err(AppError::InvalidArgument(
                        "conflicting mirror expectations".into(),
                    )));
                }
                return respond(
                    file_actions::file_clear_mirror_if_receipt(
                        state,
                        arg!("projectId"),
                        arg!("path"),
                        arg!("expectedReceipt"),
                    )
                    .await,
                );
            }
            if args.get("expected").is_some() {
                return respond(
                    file_actions::file_clear_mirror_if_current(
                        state,
                        arg!("projectId"),
                        arg!("path"),
                        arg!("expected"),
                    )
                    .await,
                );
            }
            respond(file_actions::file_clear_mirror(state, arg!("projectId"), arg!("path")).await)
        }
        "file_prune_mirrors" => respond(
            file_actions::file_prune_mirrors(state, arg!("projectId"), arg!("keepPaths")).await,
        ),
        "file_mirror_untitled" => respond(
            file_actions::file_mirror_untitled(
                state,
                arg!("projectId"),
                arg!("tabId"),
                arg!("content"),
            )
            .await,
        ),
        "file_list_untitled_mirrors" => {
            respond(file_actions::file_list_untitled_mirrors(state, arg!("projectId")).await)
        }
        "file_clear_untitled_mirror" => respond(
            file_actions::file_clear_untitled_mirror(state, arg!("projectId"), arg!("tabId")).await,
        ),
        "file_prune_untitled_mirrors" => respond(
            file_actions::file_prune_untitled_mirrors(state, arg!("projectId"), arg!("keepTabIds"))
                .await,
        ),
        "tree_rows" => respond(
            tree_actions::tree_rows(
                state,
                &services.tree,
                tasks,
                arg!("projectId"),
                arg!("offset"),
                arg!("limit"),
            )
            .await,
        ),
        "tree_toggle" => {
            let _span = perf::span(SpanSlot::TreeToggle);
            respond(
                tree_actions::tree_toggle(
                    state,
                    &services.tree,
                    tasks,
                    arg!("projectId"),
                    arg!("path"),
                )
                .await,
            )
        }
        "tree_collapse_all" => respond(
            tree_actions::tree_collapse_all(state, &services.tree, tasks, arg!("projectId")).await,
        ),
        "tree_reveal" => {
            let _span = perf::span(SpanSlot::TreeReveal);
            respond(
                tree_actions::tree_reveal(
                    state,
                    &services.tree,
                    tasks,
                    arg!("projectId"),
                    arg!("path"),
                )
                .await,
            )
        }
        "tree_refresh" => respond(
            tree_actions::tree_refresh(
                state,
                &services.tree,
                tasks,
                arg!("projectId"),
                arg!("dir"),
            )
            .await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote file/tree routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-files-tests.rs"]
mod tests;

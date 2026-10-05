use std::sync::Arc;

use serde_json::Value;
use taide_model::error::AppError;
use taide_runtime::AppServices;
use taide_runtime::git_actions::{self, GitActionContext};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &[
    "git_init",
    "git_status",
    "git_diff_file",
    "git_diff_staged_text",
    "git_show_file",
    "git_log",
    "git_ahead_behind",
    "git_remotes",
    "git_gutter",
    "git_blame_range",
    "git_stage",
    "git_unstage",
    "git_discard",
    "git_commit",
    "git_push",
    "git_pull",
    "git_fetch",
    "git_current_user",
    "git_branches",
    "git_branch_create",
    "git_branch_checkout",
    "git_branch_delete",
    "git_stash_list",
    "git_stash_push",
    "git_stash_apply",
    "git_stash_drop",
    "git_discard_hunk",
    "git_undo_last_commit",
    "git_conflict_sides",
    "git_resolve_conflict",
    "git_stage_hunk",
    "git_unstage_hunk",
    "git_stage_lines",
    "git_unstage_lines",
    "git_commit_files",
    "git_file_log",
    "git_revert_commit",
    "git_tags",
    "git_tag_create",
    "git_tag_delete",
    "git_checkout_remote_branch",
];

pub struct Ports {
    pub install_status_invalidation: Arc<dyn Fn(&AppServices) + Send + Sync>,
}

impl Ports {
    pub fn new(events: Arc<crate::event_relay::GitEvents>) -> Self {
        Self {
            install_status_invalidation: Arc::new(move |_| events.activate()),
        }
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
    let context = GitActionContext::new(&services.state, &services.git, &services.tasks);
    let events = services.events.as_ref();
    match name {
        "git_init" => respond(git_actions::git_init(events, context, arg!("projectId")).await),
        "git_status" => respond(
            git_actions::git_status(
                || {
                    services.git.ensure_invalidation_listeners(|| {
                        (ports.install_status_invalidation)(services)
                    })
                },
                context,
                arg!("projectId"),
            )
            .await,
        ),
        "git_diff_file" => respond(
            git_actions::git_diff_file(
                context,
                || {
                    let loaded = taide_plugin::service::ensure_loaded(
                        &services.plugin,
                        &services.state.paths.plugins_dir(),
                    );
                    taide_plugin::service::language_overlays(&loaded)
                },
                arg!("projectId"),
                arg!("path"),
                arg!("mode"),
                arg!("beforePath"),
            )
            .await,
        ),
        "git_diff_staged_text" => {
            respond(git_actions::git_diff_staged_text(context, arg!("projectId")).await)
        }
        "git_show_file" => respond(
            git_actions::git_show_file(context, arg!("projectId"), arg!("rev"), arg!("path")).await,
        ),
        "git_log" => respond(
            git_actions::git_log(context, arg!("projectId"), arg!("skip"), arg!("take")).await,
        ),
        "git_ahead_behind" => {
            respond(git_actions::git_ahead_behind(context, arg!("projectId")).await)
        }
        "git_remotes" => respond(git_actions::git_remotes(context, arg!("projectId")).await),
        "git_gutter" => {
            respond(git_actions::git_gutter(context, arg!("projectId"), arg!("path")).await)
        }
        "git_blame_range" => respond(
            git_actions::git_blame_range(
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("from"),
                arg!("to"),
            )
            .await,
        ),
        "git_stage" => {
            respond(git_actions::git_stage(events, context, arg!("projectId"), arg!("paths")).await)
        }
        "git_unstage" => respond(
            git_actions::git_unstage(events, context, arg!("projectId"), arg!("paths")).await,
        ),
        "git_discard" => respond(
            git_actions::git_discard(events, context, arg!("projectId"), arg!("paths")).await,
        ),
        "git_commit" => respond(
            git_actions::git_commit(
                events,
                context,
                arg!("projectId"),
                arg!("message"),
                arg!("opts"),
            )
            .await,
        ),
        "git_push" => respond(git_actions::git_push(events, context, arg!("projectId")).await),
        "git_pull" => respond(git_actions::git_pull(events, context, arg!("projectId")).await),
        "git_fetch" => respond(git_actions::git_fetch(events, context, arg!("projectId")).await),
        "git_current_user" => {
            respond(git_actions::git_current_user(context, arg!("projectId")).await)
        }
        "git_branches" => respond(git_actions::git_branches(context, arg!("projectId")).await),
        "git_branch_create" => respond(
            git_actions::git_branch_create(
                events,
                context,
                arg!("projectId"),
                arg!("name"),
                arg!("checkout"),
            )
            .await,
        ),
        "git_branch_checkout" => respond(
            git_actions::git_branch_checkout(events, context, arg!("projectId"), arg!("name"))
                .await,
        ),
        "git_branch_delete" => respond(
            git_actions::git_branch_delete(
                events,
                context,
                arg!("projectId"),
                arg!("name"),
                arg!("force"),
            )
            .await,
        ),
        "git_stash_list" => respond(git_actions::git_stash_list(context, arg!("projectId")).await),
        "git_stash_push" => respond(
            git_actions::git_stash_push(events, context, arg!("projectId"), arg!("message")).await,
        ),
        "git_stash_apply" => respond(
            git_actions::git_stash_apply(events, context, arg!("projectId"), arg!("index")).await,
        ),
        "git_stash_drop" => {
            respond(git_actions::git_stash_drop(context, arg!("projectId"), arg!("index")).await)
        }
        "git_discard_hunk" => respond(
            git_actions::git_discard_hunk(
                events,
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("hunkStart"),
                arg!("hunkEnd"),
            )
            .await,
        ),
        "git_undo_last_commit" => {
            respond(git_actions::git_undo_last_commit(events, context, arg!("projectId")).await)
        }
        "git_conflict_sides" => {
            respond(git_actions::git_conflict_sides(context, arg!("projectId"), arg!("path")).await)
        }
        "git_resolve_conflict" => respond(
            git_actions::git_resolve_conflict(
                events,
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("content"),
            )
            .await,
        ),
        "git_stage_hunk" => respond(
            git_actions::git_stage_hunk(
                events,
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("hunkStart"),
                arg!("hunkEnd"),
            )
            .await,
        ),
        "git_unstage_hunk" => respond(
            git_actions::git_unstage_hunk(
                events,
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("hunkStart"),
                arg!("hunkEnd"),
            )
            .await,
        ),
        "git_stage_lines" => respond(
            git_actions::git_stage_lines(
                events,
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("lineStart"),
                arg!("lineEnd"),
            )
            .await,
        ),
        "git_unstage_lines" => respond(
            git_actions::git_unstage_lines(
                events,
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("lineStart"),
                arg!("lineEnd"),
            )
            .await,
        ),
        "git_commit_files" => {
            respond(git_actions::git_commit_files(context, arg!("projectId"), arg!("rev")).await)
        }
        "git_file_log" => respond(
            git_actions::git_file_log(
                context,
                arg!("projectId"),
                arg!("path"),
                arg!("skip"),
                arg!("take"),
            )
            .await,
        ),
        "git_revert_commit" => respond(
            git_actions::git_revert_commit(events, context, arg!("projectId"), arg!("rev")).await,
        ),
        "git_tags" => respond(git_actions::git_tags(context, arg!("projectId")).await),
        "git_tag_create" => respond(
            git_actions::git_tag_create(
                events,
                context,
                arg!("projectId"),
                arg!("name"),
                arg!("target"),
                arg!("opts"),
            )
            .await,
        ),
        "git_tag_delete" => respond(
            git_actions::git_tag_delete(events, context, arg!("projectId"), arg!("name")).await,
        ),
        "git_checkout_remote_branch" => respond(
            git_actions::git_checkout_remote_branch(
                events,
                context,
                arg!("projectId"),
                arg!("remoteRef"),
            )
            .await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote git routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-git-tests.rs"]
mod tests;

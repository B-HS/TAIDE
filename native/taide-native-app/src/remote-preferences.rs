use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde_json::Value;
use taide_model::app::AppInfo;
use taide_model::error::{AppError, AppResult};
use taide_model::settings::Settings;
use taide_model::theme::ThemeEditorContext;
use taide_runtime::{
    AppServices, app_actions, locale_actions, remote_actions, settings_actions, snippet_actions,
    theme_actions,
};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;
use crate::settings_view::Owner;
use crate::theme_draft::Mode;
use crate::theme_edit::Session;

pub const COMMANDS: &[&str] = &[
    "app_get_info",
    "app_file_read",
    "app_file_write",
    "settings_get",
    "settings_update",
    "settings_set_theme",
    "theme_list",
    "theme_get",
    "theme_get_current",
    "theme_save",
    "theme_delete",
    "locale_list",
    "locale_get",
    "locale_get_current",
    "snippet_list",
    "snippet_save",
    "snippet_delete",
    "remote_status",
    "remote_revoke_sessions",
];

pub type Reconcile =
    Arc<dyn Fn(Arc<AppServices>, Settings, Settings) -> BoxFuture<'static, ()> + Send + Sync>;

pub struct Ports {
    pub info: AppInfo,
    pub reconcile: Reconcile,
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
                    return dispatch(services, ports, &name, args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn apply_settings(
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    next: Settings,
) -> AppResult<Settings> {
    settings_actions::apply_and_broadcast(
        &services.state,
        next,
        |current, updated| (ports.reconcile)(services.clone(), current, updated),
        services.events.as_ref(),
    )
    .await
}

async fn dispatch(
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    name: &str,
    args: Value,
) -> Result<String, Value> {
    macro_rules! arg {
        ($key:literal) => {
            argument(&args, $key).map_err(error_value)?
        };
    }
    match name {
        "app_get_info" => respond(Ok(&ports.info)),
        "app_file_read" => {
            respond(app_actions::app_file_read(&services.state, arg!("target")).await)
        }
        "app_file_write" => {
            let target = arg!("target");
            let content = arg!("content");
            let worker_services = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-app-file-write", async move {
                        let apply_services = worker_services.clone();
                        app_actions::app_file_write(
                            &worker_services.state,
                            target,
                            content,
                            |next| apply_settings(apply_services, ports, next),
                        )
                        .await
                    })
                    .await,
            )
        }
        "settings_get" => respond(settings_actions::settings_get(&services.state).await),
        "settings_update" => {
            let patch = arg!("patch");
            let worker_services = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-settings-update", async move {
                        settings_actions::settings_update(
                            &worker_services.state,
                            patch,
                            |current, updated| {
                                (ports.reconcile)(worker_services.clone(), current, updated)
                            },
                            worker_services.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        "settings_set_theme" => {
            let theme_id = arg!("themeId");
            let worker_services = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-settings-set-theme", async move {
                        settings_actions::settings_set_theme(
                            &worker_services.state,
                            theme_id,
                            |current, updated| {
                                (ports.reconcile)(worker_services.clone(), current, updated)
                            },
                            worker_services.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        "theme_list" => respond(theme_actions::theme_list(&services.state)),
        "theme_get" => respond(theme_actions::theme_get(&services.state, arg!("themeId"))),
        "theme_get_current" => {
            let system_theme: String = arg!("systemTheme");
            respond(theme_actions::theme_get_current(
                &services.state,
                &system_theme,
            ))
        }
        "theme_save" => {
            let theme = arg!("theme");
            if args.get("editor").is_none() {
                return respond(theme_actions::theme_save(&services.state, theme));
            }
            let editor: ThemeEditorContext = arg!("editor");
            let mode = if editor.is_create {
                Mode::Create
            } else {
                Mode::Edit
            };
            let session = Session::new(
                Owner {
                    project: editor.project_id,
                    pane: editor.pane_id,
                    tab: editor.tab_id,
                },
                editor.source_theme_id,
                mode,
            )
            .map_err(error_value)?;
            let request = session.save_theme_request(theme).map_err(error_value)?;
            respond(request.execute(&services).await)
        }
        "theme_delete" => {
            let theme_id: String = arg!("themeId");
            if args.get("editor").is_none() {
                return respond(theme_actions::theme_delete(&services.state, theme_id));
            }
            let editor: ThemeEditorContext = arg!("editor");
            if editor.is_create || editor.source_theme_id != theme_id {
                return Err(error_value(AppError::InvalidArgument(
                    "remote theme deletion does not match its editor".into(),
                )));
            }
            let session = Session::new(
                Owner {
                    project: editor.project_id,
                    pane: editor.pane_id,
                    tab: editor.tab_id,
                },
                editor.source_theme_id,
                Mode::Edit,
            )
            .map_err(error_value)?;
            let request = session.delete_request().map_err(error_value)?;
            respond(request.execute(&services).await)
        }
        "locale_list" => respond(locale_actions::locale_list(&services.state)),
        "locale_get" => respond(locale_actions::locale_get(
            &services.state,
            arg!("localeId"),
        )),
        "locale_get_current" => {
            let system_language: String = arg!("systemLanguage");
            respond(locale_actions::locale_get_current(
                &services.state,
                &system_language,
            ))
        }
        "snippet_list" => respond(snippet_actions::snippet_list(&services.state)),
        "snippet_save" => respond(snippet_actions::snippet_save(
            &services.state,
            arg!("fileName"),
            arg!("content"),
        )),
        "snippet_delete" => respond(snippet_actions::snippet_delete(
            &services.state,
            arg!("fileName"),
        )),
        "remote_status" => respond(remote_actions::remote_status(&services.remote).await),
        "remote_revoke_sessions" => {
            respond(remote_actions::remote_revoke_sessions(&services.remote).await)
        }
        _ => Err(error_value(AppError::Internal(
            "remote preferences routing mismatch".into(),
        ))),
    }
}

#[cfg(test)]
#[path = "remote-preferences-tests.rs"]
mod tests;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use taide_model::agent::ExternalOpenRequest;
use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_model::layout::{Tab, TabKind};
use taide_runtime::{AppServices, agent_actions, project_actions};

/// How one drained external-open request ended, for the UI thread's toasts.
#[derive(Debug)]
pub enum Outcome {
    ProjectOpened,
    Opened,
    WaitRegistered,
    OpenProjectFirst,
    Failed(AppError),
}

/// The original `agent-wait-marker-registry`: the `--wait` markers waiting on each opened path,
/// keyed by the path string the tab was opened with.
#[derive(Debug, Default)]
pub struct WaitMarkers(Mutex<HashMap<String, Vec<String>>>);

impl WaitMarkers {
    pub fn register(&self, path: String, marker: String) {
        self.lock().entry(path).or_default().push(marker);
    }

    pub fn take(&self, path: &str) -> Vec<String> {
        self.lock().remove(path).unwrap_or_default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Vec<String>>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

const PROGRAM_NAME: &str = "taide-native";

/// Original `parse_cli_payload` over the launch arguments that follow the data directory.
pub fn request_from(arguments: &[String]) -> Option<ExternalOpenRequest> {
    let argv = std::iter::once(PROGRAM_NAME.to_owned())
        .chain(arguments.iter().cloned())
        .collect::<Vec<_>>();
    taide_agent::service::parse_cli_payload(&argv)
}

/// Original `queue_external_open`: the marker is tracked for exit cleanup, the path is allowed past
/// the open-project boundary, and the request waits for the next drain.
pub fn queue(services: &AppServices, request: ExternalOpenRequest) {
    if let Some(marker) = &request.wait_marker {
        services.agents.register_wait_marker(marker.clone());
    }
    services
        .state
        .authorize_cli_opened_path(Path::new(&request.path));
    services.agents.push_pending_external_open(request);
}

/// Original `normalizeFsPath`: `\` becomes `/`, `.` segments vanish and `..` drops the segment
/// before it.
fn normalize(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let bytes = unified.as_bytes();
    let has_drive = bytes.len() >= 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes.len() == 2 || bytes[2] == b'/');
    let (anchor, rest) = if has_drive {
        unified.split_at(2)
    } else if unified.starts_with('/') {
        unified.split_at(1)
    } else {
        unified.split_at(0)
    };
    let mut segments: Vec<&str> = Vec::new();
    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
    let joined = segments.join("/");
    match anchor {
        "" => joined,
        "/" => format!("/{joined}"),
        drive => format!("{drive}/{joined}"),
    }
}

/// Original `isWithinRoot`.
pub fn is_within_root(path: &str, root: &str) -> bool {
    let (path, root) = (normalize(path), normalize(root));
    path == root || path.starts_with(&format!("{root}/"))
}

/// Original `resolveExternalOpenTarget`: the project with the longest root containing the path
/// (the first of equal lengths), else the active project, else the first project.
pub fn resolve_target(
    path: &str,
    projects: &[(ProjectId, String)],
    active: Option<&ProjectId>,
) -> Option<ProjectId> {
    let owner = projects
        .iter()
        .filter(|(_, root)| is_within_root(path, root))
        .fold(None::<&(ProjectId, String)>, |best, candidate| match best {
            Some(best) if best.1.encode_utf16().count() >= candidate.1.encode_utf16().count() => {
                Some(best)
            }
            _ => Some(candidate),
        });
    if let Some((project, _)) = owner {
        return Some(project.clone());
    }
    projects
        .iter()
        .find(|(project, _)| Some(project) == active)
        .or_else(|| projects.first())
        .map(|(project, _)| project.clone())
}

async fn release(services: &AppServices, marker: String) {
    if let Err(error) = agent_actions::agent_release_marker(
        &services.state,
        &services.agents,
        &services.tasks,
        marker,
    )
    .await
    {
        log::warn!("native wait marker release failed: {:?}", error.kind());
    }
}

async fn release_request(services: &AppServices, request: &ExternalOpenRequest) {
    if let Some(marker) = request.wait_marker.clone() {
        release(services, marker).await;
    }
}

/// Original `releaseClosedFileTabPath`: a closed file tab hands its path's markers back once no
/// tree of that project's layout still shows the same path.
pub async fn release_closed_tab(
    services: &AppServices,
    markers: &WaitMarkers,
    project: &ProjectId,
    tab: &Tab,
) {
    let TabKind::File { path } = &tab.kind else {
        return;
    };
    let remaining = services
        .state
        .layouts
        .read()
        .get(project)
        .is_some_and(|layout| crate::tabs::project_has_file_tab(layout, path));
    if remaining {
        return;
    }
    for marker in markers.take(path) {
        release(services, marker).await;
    }
}

fn open_projects(services: &AppServices) -> (Vec<(ProjectId, String)>, Option<ProjectId>) {
    let session = services.state.session.read();
    let open = services.state.projects.read();
    let projects = session
        .projects
        .iter()
        .filter_map(|project| {
            open.get(&project.id)
                .map(|open| (project.id.clone(), open.root.clone()))
        })
        .collect();
    (projects, session.active_project.clone())
}

/// Original `processExternalOpenRequest` for every queued request, in queue order.
pub(crate) async fn process(
    services: &Arc<AppServices>,
    integrations: &crate::host::HostIntegrations,
) -> Vec<Outcome> {
    let mut outcomes = Vec::new();
    for request in services.agents.drain_pending_external_opens() {
        let opened = async {
            project_actions::validate_open_root(Path::new(&request.path))?;
            integrations.flush_view_states(services).await?;
            crate::projects::NativeProjects::new(services.clone())
                .open(request.path.clone())
                .await
        }
        .await;
        if opened.is_ok() {
            release_request(services, &request).await;
            outcomes.push(Outcome::ProjectOpened);
            continue;
        }
        let (projects, active) = open_projects(services);
        let Some(target) = resolve_target(&request.path, &projects, active.as_ref()) else {
            release_request(services, &request).await;
            outcomes.push(Outcome::OpenProjectFirst);
            continue;
        };
        let result = async {
            integrations.flush_view_states(services).await?;
            project_actions::project_activate(
                services.events.as_ref(),
                &services.state,
                target.clone(),
            )
            .await?;
            crate::host::open_file_tab(
                services,
                target,
                None,
                request.path.clone(),
                request.wait_marker.is_none(),
            )
            .await
        }
        .await;
        match (result, request.wait_marker) {
            (Ok(()), Some(marker)) => {
                integrations
                    .wait_markers
                    .register(request.path.clone(), marker);
                outcomes.push(Outcome::WaitRegistered);
            }
            (Ok(()), None) => outcomes.push(Outcome::Opened),
            (Err(error), marker) => {
                if let Some(marker) = marker {
                    release(services, marker).await;
                }
                outcomes.push(Outcome::Failed(error));
            }
        }
    }
    outcomes
}

#[cfg(test)]
#[path = "external-open-tests.rs"]
mod tests;

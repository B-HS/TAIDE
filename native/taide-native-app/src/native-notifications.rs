use std::collections::HashMap;
use std::time::{Duration, Instant};

use taide_model::agent::{AgentActivity, BlockedReason, DetectedAgent};
use taide_model::ids::ProjectId;
use taide_model::layout::{ProjectLayout, TabKind};
use taide_model::locale::ResolvedLocale;
use taide_model::notification::NotificationCategory;
use taide_model::project::ProjectRef;

use crate::presentation::message;

/// Original `AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS`.
pub(crate) const AGENT_COMPLETION_NOTIFY_MIN_WORKING: Duration = Duration::from_secs(10);
/// Original `TASK_COMPLETION_NOTIFY_MIN_DURATION_MS`.
pub(crate) const TASK_COMPLETION_NOTIFY_MIN_DURATION_MS: u32 = 10_000;
/// Original `NOTIFICATION_TEXT_MAX_CODE_POINTS`.
pub(crate) const NOTIFICATION_TEXT_MAX_CODE_POINTS: usize = 200;
const NOTIFICATION_BODY_SEPARATOR: &str = " · ";
const TRUNCATION_MARK: char = '…';
const MS_PER_SECOND: u64 = 1_000;
const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;

/// One OS notification ready for the host's settings and focus gate.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub category: NotificationCategory,
    pub title: String,
    pub body: String,
    pub any_window_focused: bool,
}

/// An agent that handed control back after a long enough working stretch.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Completion {
    pub(crate) session: String,
    pub(crate) name: String,
    pub(crate) worked_for: Duration,
    pub(crate) awaiting_input: bool,
    pub(crate) blocked_reason: Option<BlockedReason>,
}

/// The original provider's per-project "working since" maps and its once-per-session hint flag.
#[derive(Debug, Default)]
pub(crate) struct Tracker {
    working_since: HashMap<ProjectId, HashMap<String, Instant>>,
    pub(crate) has_announced_delivery: bool,
}

impl Tracker {
    pub(crate) fn observe(
        &mut self,
        project: &ProjectId,
        agents: &[DetectedAgent],
        now: Instant,
    ) -> Vec<Completion> {
        let previous = self.working_since.remove(project).unwrap_or_default();
        let (working_since, completed) = evaluate(&previous, agents, now);
        self.working_since.insert(project.clone(), working_since);
        completed
    }

    pub(crate) fn forget(&mut self, project: &ProjectId) {
        self.working_since.remove(project);
    }
}

/// Original `evaluateAgentCompletions`: rebuilds the map from the roster and reports the agents
/// whose working stretch ended in `idle` or `awaitingInput` after the minimum.
pub(crate) fn evaluate(
    previous: &HashMap<String, Instant>,
    agents: &[DetectedAgent],
    now: Instant,
) -> (HashMap<String, Instant>, Vec<Completion>) {
    let mut working_since = HashMap::new();
    let mut completed = Vec::new();
    for agent in agents {
        let started = previous.get(&agent.session_id).copied();
        if agent.activity == AgentActivity::Working {
            working_since.insert(agent.session_id.clone(), started.unwrap_or(now));
            continue;
        }
        let Some(started) = started else {
            continue;
        };
        if !matches!(
            agent.activity,
            AgentActivity::Idle | AgentActivity::AwaitingInput
        ) {
            continue;
        }
        let worked_for = now.saturating_duration_since(started);
        if worked_for < AGENT_COMPLETION_NOTIFY_MIN_WORKING {
            continue;
        }
        completed.push(Completion {
            session: agent.session_id.clone(),
            name: agent.name.clone(),
            worked_for,
            awaiting_input: agent.activity == AgentActivity::AwaitingInput,
            blocked_reason: agent.blocked_reason,
        });
    }
    (working_since, completed)
}

pub(crate) fn agent_category(completion: &Completion) -> NotificationCategory {
    if completion.awaiting_input {
        NotificationCategory::AgentAwaitingInput
    } else {
        NotificationCategory::AgentCompleted
    }
}

fn is_present(text: &str) -> bool {
    !text.trim().is_empty()
}

/// Original `notificationBodyFragment`.
pub(crate) fn body_fragment(text: Option<&str>) -> String {
    text.filter(|text| is_present(text))
        .map(|text| format!("{NOTIFICATION_BODY_SEPARATOR}{text}"))
        .unwrap_or_default()
}

/// Original `joinNotificationBody`.
pub(crate) fn join_body(parts: &[Option<String>]) -> String {
    parts
        .iter()
        .flatten()
        .filter(|part| is_present(part))
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(NOTIFICATION_BODY_SEPARATOR)
}

/// Original `formatDurationShort`.
pub(crate) fn format_duration(locale: &ResolvedLocale, duration_ms: u64) -> String {
    let total_seconds = duration_ms / MS_PER_SECOND;
    let total_minutes = total_seconds / SECONDS_PER_MINUTE;
    let hours = total_minutes / MINUTES_PER_HOUR;
    let minutes = (total_minutes % MINUTES_PER_HOUR).to_string();
    let seconds = (total_seconds % SECONDS_PER_MINUTE).to_string();
    if hours > 0 {
        return message(
            locale,
            "common.durationHoursMinutes",
            &[("hours", &hours.to_string()), ("minutes", &minutes)],
        );
    }
    if total_minutes > 0 {
        return message(
            locale,
            "common.durationMinutesSeconds",
            &[("minutes", &minutes), ("seconds", &seconds)],
        );
    }
    message(locale, "common.durationSeconds", &[("seconds", &seconds)])
}

/// Original `buildAgentCompletionBody`.
pub(crate) fn agent_body(
    locale: &ResolvedLocale,
    completion: &Completion,
    tab_title: Option<&str>,
) -> String {
    let tab = body_fragment(tab_title);
    if completion.awaiting_input {
        let key = match completion.blocked_reason {
            Some(BlockedReason::Permission) => "notification.agentPermissionBody",
            Some(BlockedReason::Question) => "notification.agentQuestionBody",
            Some(BlockedReason::Dialog) | None => "notification.agentAwaitingInputBody",
        };
        return message(locale, key, &[("agent", &completion.name), ("tab", &tab)]);
    }
    let duration = format_duration(
        locale,
        u64::try_from(completion.worked_for.as_millis()).unwrap_or(u64::MAX),
    );
    message(
        locale,
        "notification.agentFinishedBody",
        &[
            ("agent", &completion.name),
            ("duration", &duration),
            ("tab", &tab),
        ],
    )
}

/// Original `buildTaskCompletionBody`.
pub(crate) fn task_body(
    locale: &ResolvedLocale,
    target: &str,
    exit_code: Option<i32>,
    duration_ms: u32,
) -> String {
    join_body(&[
        Some(target.to_owned()),
        exit_code.map(|code| {
            message(
                locale,
                "notification.exitCode",
                &[("code", &code.to_string())],
            )
        }),
        Some(format_duration(locale, u64::from(duration_ms))),
    ])
}

/// Original `truncateNotificationText`, counted in Unicode scalar values.
pub(crate) fn truncate(text: &str, max_code_points: usize) -> String {
    if text.chars().count() <= max_code_points {
        return text.to_owned();
    }
    text.chars()
        .take(max_code_points.saturating_sub(1))
        .chain(std::iter::once(TRUNCATION_MARK))
        .collect()
}

/// Original `resolveProjectNotificationTitle`: the display label, then the project name, then the
/// event's own title when the project is unknown.
pub(crate) fn project_title(
    projects: &[ProjectRef],
    project: Option<&ProjectId>,
    fallback: String,
) -> String {
    let Some(project) = project.and_then(|id| projects.iter().find(|item| &item.id == id)) else {
        return fallback;
    };
    match taide_native_ui::project_rail::resolve_project_display(&project.display).glyph {
        taide_native_ui::project_rail::ProjectGlyph::Label(label) => label,
        _ => project.name.clone(),
    }
}

/// Original `findTerminalTabTitle`, searching the main tree and every auxiliary window tree.
pub(crate) fn terminal_tab_title<'a>(layout: &'a ProjectLayout, session: &str) -> Option<&'a str> {
    if session.is_empty() {
        return None;
    }
    taide_layout::service::all_roots(layout)
        .flat_map(crate::tabs::tabs_in)
        .find(|tab| matches!(&tab.kind, TabKind::Terminal { session_id, .. } if session_id == session))
        .map(|tab| tab.title.as_str())
}

/// Original `findTerminalSessionOwner` over the open projects in list order.
pub(crate) fn session_owner<'a>(
    projects: &[ProjectRef],
    layouts: &'a HashMap<ProjectId, ProjectLayout>,
    session: &str,
) -> Option<(ProjectId, &'a str)> {
    projects.iter().find_map(|project| {
        let title = terminal_tab_title(layouts.get(&project.id)?, session)?;
        Some((project.id.clone(), title))
    })
}

#[cfg(test)]
#[path = "native-notifications-tests.rs"]
mod tests;

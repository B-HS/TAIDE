use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use taide_model::ai::{AiCommitMessageRequest, AiTextResponse};
use taide_model::error::AppResult;
use taide_model::git::{LogEntry, StagedDiffText};
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::locale::ResolvedLocale;
use taide_native_editor::language_configuration::is_js_whitespace;
use taide_runtime::AppServices;

use crate::presentation;

pub(crate) const DRAFT_LIMIT: usize = 8;
const RECENT_COMMITS: usize = 20;
const SHORT_HASH: usize = 7;
const NOTICE_SEPARATOR: &str = " · ";
const QUOTE_PAIRS: [(char, char); 5] = [
    ('"', '"'),
    ('\'', '\''),
    ('`', '`'),
    ('\u{201C}', '\u{201D}'),
    ('\u{2018}', '\u{2019}'),
];

pub type AiFuture = Pin<Box<dyn Future<Output = AppResult<AiTextResponse>> + Send>>;
pub type AiPort = Arc<dyn Fn(Arc<AppServices>, AiCommitMessageRequest) -> AiFuture + Send + Sync>;

fn runtime_ai() -> AiPort {
    Arc::new(|services, request| {
        Box::pin(async move {
            taide_runtime::ai_actions::ai_commit_message(
                &services.state,
                &services.ai_requests,
                &services.secrets,
                request,
            )
            .await
        })
    })
}

/// One window's unsent commit messages (`commit-message-memory.ts`): newest last, an emptied
/// message forgets its project, and only the newest [`DRAFT_LIMIT`] projects are kept.
#[derive(Default)]
pub(crate) struct Drafts {
    entries: Vec<(ProjectId, String)>,
}

impl Drafts {
    pub(crate) fn read(&self, project: &ProjectId) -> &str {
        self.entries
            .iter()
            .find(|(id, _)| id == project)
            .map_or("", |(_, message)| message)
    }

    pub(crate) fn write(&mut self, project: &ProjectId, message: String) {
        self.entries.retain(|(id, _)| id != project);
        if message.is_empty() {
            return;
        }
        self.entries.push((project.clone(), message));
        let excess = self.entries.len().saturating_sub(DRAFT_LIMIT);
        self.entries.drain(..excess);
    }
}

/// An AI commit message request a sidebar slot's panel is waiting for.
pub(crate) struct Request {
    pub project: ProjectId,
    pub id: String,
    pub owner: String,
}

/// The commit box state behind every source control panel: drafts per window (each original
/// window was its own JavaScript realm), commits in flight per project, and the latest AI
/// request per sidebar slot (`git-panel-container.tsx`).
pub(crate) struct State {
    drafts: HashMap<String, Drafts>,
    windows: HashMap<ShellSlotId, String>,
    pub(crate) committing: HashSet<ProjectId>,
    requests: HashMap<ShellSlotId, Request>,
    pub(crate) ai: AiPort,
}

impl Default for State {
    fn default() -> Self {
        Self {
            drafts: HashMap::new(),
            windows: HashMap::new(),
            committing: HashSet::new(),
            requests: HashMap::new(),
            ai: runtime_ai(),
        }
    }
}

impl State {
    /// The window whose drafts a sidebar slot uses: an auxiliary window's explorer, else `main`.
    pub(crate) fn window<'a>(&'a self, slot: &ShellSlotId, main: &'a str) -> &'a str {
        self.windows.get(slot).map_or(main, String::as_str)
    }

    /// Records which auxiliary window owns each explorer slot and forgets closed windows' drafts.
    pub(crate) fn set_windows(
        &mut self,
        main: &str,
        windows: impl Iterator<Item = (ShellSlotId, String)>,
    ) {
        self.windows = windows.collect();
        let open = &self.windows;
        self.drafts
            .retain(|window, _| window == main || open.values().any(|label| label == window));
    }

    pub(crate) fn draft(&self, window: &str, project: &ProjectId) -> &str {
        self.drafts
            .get(window)
            .map_or("", |drafts| drafts.read(project))
    }

    pub(crate) fn write(&mut self, window: &str, project: &ProjectId, message: String) {
        self.drafts
            .entry(window.to_owned())
            .or_default()
            .write(project, message);
    }

    pub(crate) fn is_generating(&self, project: &ProjectId, slot: &ShellSlotId) -> bool {
        self.requests
            .get(slot)
            .is_some_and(|request| request.project == *project)
    }

    pub(crate) fn begin(&mut self, slot: ShellSlotId, request: Request) {
        self.requests.insert(slot, request);
    }

    pub(crate) fn take(&mut self, slot: &ShellSlotId) -> Option<Request> {
        self.requests.remove(slot)
    }

    /// The slot still waiting for `id`, if no newer click, cancel or project switch replaced it.
    pub(crate) fn finish(&mut self, id: &str) -> Option<(ShellSlotId, Request)> {
        let slot = self
            .requests
            .iter()
            .find(|(_, request)| request.id == id)
            .map(|(slot, _)| slot.clone())?;
        self.requests.remove_entry(&slot)
    }

    /// Drops the requests whose panel no longer shows their project, for the caller to cancel.
    pub(crate) fn abandon(
        &mut self,
        is_shown: impl Fn(&ProjectId, &ShellSlotId) -> bool,
    ) -> Vec<Request> {
        let abandoned: Vec<_> = self
            .requests
            .iter()
            .filter(|(slot, request)| !is_shown(&request.project, slot))
            .map(|(slot, _)| slot.clone())
            .collect();
        abandoned
            .iter()
            .filter_map(|slot| self.requests.remove(slot))
            .collect()
    }

    pub(crate) fn clear_requests(&mut self) {
        self.requests.clear();
    }
}

/// `ai-commit-message.ts` `sanitizeAiCommitMessageResponse`: drops a code fence, trims, then
/// removes one surrounding quote pair and trims again.
pub(crate) fn sanitize(text: &str) -> String {
    let unfenced = crate::editor_ai_edit::strip_code_fence(text);
    let trimmed = unfenced.trim_matches(is_js_whitespace);
    QUOTE_PAIRS
        .iter()
        .find_map(|(open, close)| trimmed.strip_prefix(*open)?.strip_suffix(*close))
        .map_or(trimmed, |inner| inner.trim_matches(is_js_whitespace))
        .to_owned()
}

/// `buildRecentCommitsSummaryForAi`: the newest commits as `<short hash> <summary>` lines.
pub(crate) fn recent_commits(log: &[LogEntry]) -> String {
    log.iter()
        .take(RECENT_COMMITS)
        .map(|entry| {
            format!(
                "{} {}",
                entry.id.get(..SHORT_HASH).unwrap_or(&entry.id),
                entry.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The generated-message toast's description: what the staged diff had to fall back on or leave
/// out, in the original order.
pub(crate) fn notices(locale: &ResolvedLocale, diff: &StagedDiffText) -> Option<String> {
    let skipped = diff.skipped_files.len().to_string();
    let notices: Vec<_> = [
        diff.used_fallback
            .then(|| presentation::message(locale, "git.commitMessageUsedUnstaged", &[])),
        diff.truncated
            .then(|| presentation::message(locale, "git.commitMessageDiffTruncated", &[])),
        (!diff.skipped_files.is_empty()).then(|| {
            presentation::message(
                locale,
                "git.commitMessageFilesSkipped",
                &[("count", &skipped)],
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!notices.is_empty()).then(|| notices.join(NOTICE_SEPARATOR))
}

#[cfg(test)]
#[path = "git-commit-tests.rs"]
mod tests;

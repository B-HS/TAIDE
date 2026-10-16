use std::cell::Cell;
use std::sync::Arc;

use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::locale::ResolvedLocale;
use taide_model::project::Project;
use taide_native_ui::keymap::catalog::{Overrides, welcome_shortcuts};
use taide_native_ui::shell::{WelcomeContent, WelcomeRecent, WelcomeShortcutRow};
use taide_native_ui::snapshot::ShellSnapshot;
use taide_runtime::{AppState, TaskSupervisor};
use tokio::sync::oneshot;

const WORKER_NAME: &str = "native-welcome-recent";
/// Original `RECENT_PROJECT_DISPLAY_LIMIT`.
pub(crate) const RECENT_DISPLAY_LIMIT: usize = 8;
/// Original `RECENT_PROJECT_MENU_LIMIT`.
pub(crate) const RECENT_MENU_LIMIT: usize = 10;
/// Original `PROJECT_LABEL_MAX_CODEPOINTS`.
const LABEL_MAX_CODEPOINTS: usize = 4;

/// Original `resolveProjectDisplay(...).label`: a trimmed non-empty label capped in codepoints.
fn display_label(label: Option<&str>) -> Option<String> {
    let trimmed = label?.trim();
    (!trimmed.is_empty()).then(|| trimmed.chars().take(LABEL_MAX_CODEPOINTS).collect())
}

type Recent = Result<Arc<Vec<Project>>, ()>;

/// Recent project list cache for the Welcome surface. It loads on first use and reloads after
/// every shell snapshot change while a Welcome surface is shown, like the original query that is
/// refetched on mount and invalidated by project mutations.
#[derive(Default)]
pub(crate) struct State {
    wanted: Cell<bool>,
    loaded_for: Option<usize>,
    pending: Option<(usize, oneshot::Receiver<Recent>)>,
    recent: Option<Recent>,
    shortcuts: Option<(Option<String>, Arc<Vec<WelcomeShortcutRow>>)>,
}

impl State {
    pub(crate) fn tick(
        &mut self,
        snapshot: &Arc<ShellSnapshot>,
        state: &AppState,
        tasks: &TaskSupervisor,
        context: &eframe::egui::Context,
    ) {
        if let Some((generation, receiver)) = &mut self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.loaded_for = Some(*generation);
                    self.recent = Some(result);
                    self.pending = None;
                    context.request_repaint();
                }
                Err(oneshot::error::TryRecvError::Empty) => return,
                Err(oneshot::error::TryRecvError::Closed) => self.pending = None,
            }
        }
        let generation = Arc::as_ptr(snapshot) as usize;
        if !self.wanted.replace(false) || self.loaded_for == Some(generation) {
            return;
        }
        let (sender, receiver) = oneshot::channel();
        let state = state.clone();
        let repaint = context.clone();
        let started = tasks.spawn_transient(WORKER_NAME, async move {
            let result = taide_runtime::project_actions::project_list_recent(&state)
                .await
                .map(Arc::new)
                .map_err(|_| ());
            if sender.send(result).is_ok() {
                repaint.request_repaint();
            }
        });
        if started {
            self.pending = Some((generation, receiver));
        }
    }

    pub(crate) fn clear(&mut self) {
        self.pending = None;
    }

    pub(crate) fn shortcuts(
        &mut self,
        overrides: Option<&str>,
        locale: &ResolvedLocale,
        is_mac: bool,
    ) -> AppResult<Arc<Vec<WelcomeShortcutRow>>> {
        if let Some((source, rows)) = &self.shortcuts
            && source.as_deref() == overrides
        {
            return Ok(rows.clone());
        }
        let rows: Arc<Vec<WelcomeShortcutRow>> = Arc::new(
            welcome_shortcuts(&Overrides::parse(overrides), is_mac)?
                .into_iter()
                .map(|shortcut| WelcomeShortcutRow {
                    description: crate::presentation::message(
                        locale,
                        &shortcut.description_key,
                        &[],
                    ),
                    label: shortcut.label,
                })
                .collect(),
        );
        self.shortcuts = Some((overrides.map(str::to_owned), rows.clone()));
        Ok(rows)
    }

    /// Rail `+` menu rows: recent projects that are not open, limited like the original menu.
    pub(crate) fn rail(&self, open: &[taide_model::project::ProjectRef]) -> Vec<WelcomeRecent> {
        self.wanted.set(true);
        let Some(Ok(projects)) = &self.recent else {
            return Vec::new();
        };
        projects
            .iter()
            .filter(|candidate| open.iter().all(|project| project.id != candidate.id))
            .take(RECENT_MENU_LIMIT)
            .map(|candidate| WelcomeRecent {
                id: candidate.id.clone(),
                name: display_label(candidate.display.label.as_deref())
                    .unwrap_or_else(|| candidate.name.clone()),
                root: candidate.root.clone(),
                root_missing: candidate.root_missing,
            })
            .collect()
    }

    pub(crate) fn content(
        &self,
        project: Option<&ProjectId>,
        shortcuts: &[WelcomeShortcutRow],
    ) -> WelcomeContent {
        self.wanted.set(true);
        let recent = match &self.recent {
            None => Some(Vec::new()),
            Some(Err(())) => None,
            Some(Ok(projects)) => Some(
                projects
                    .iter()
                    .filter(|candidate| Some(&candidate.id) != project)
                    .take(RECENT_DISPLAY_LIMIT)
                    .map(|candidate| WelcomeRecent {
                        id: candidate.id.clone(),
                        name: candidate.name.clone(),
                        root: candidate.root.clone(),
                        root_missing: candidate.root_missing,
                    })
                    .collect(),
            ),
        };
        WelcomeContent {
            recent,
            shortcuts: shortcuts.to_vec(),
        }
    }
}

#[cfg(test)]
#[path = "welcome-tests.rs"]
mod tests;

use std::collections::HashSet;

use taide_model::ids::{ProjectId, TabId};
use taide_model::search::SearchQuery;

use crate::search_panel::Panel;
use crate::search_stream::Session;

/// How many search editor tabs that are not on screen keep their inputs and results
/// (`search-editor-memory.ts` `SEARCH_EDITOR_MEMORY_LIMIT`).
pub(crate) const MEMORY_LIMIT: usize = 16;

struct Entry {
    tab: TabId,
    project: ProjectId,
    panel: Panel,
    mounts: u64,
    is_mounted: bool,
}

/// A search editor tab body about to be drawn: its panel, a focus nonce that changes on every
/// mount, and whether it was just created from the tab's query (which triggers the one auto run).
pub(crate) struct Mounted<'a> {
    pub(crate) panel: &'a mut Panel,
    pub(crate) nonce: u64,
    pub(crate) is_created: bool,
}

/// Per-tab search editor bodies, remembered across the tab leaving the screen.
#[derive(Default)]
pub(crate) struct Editors {
    entries: Vec<Entry>,
}

impl Editors {
    /// The body to draw for `tab`: the remembered one, or a new one from the tab's query when
    /// nothing is remembered or the memory belongs to another project.
    pub(crate) fn mount(
        &mut self,
        tab: &TabId,
        project: &ProjectId,
        session: impl FnOnce() -> Session,
        query: &SearchQuery,
    ) -> Mounted<'_> {
        self.entries
            .retain(|entry| entry.tab != *tab || entry.project == *project);
        let (index, is_created) = match self.entries.iter().position(|entry| entry.tab == *tab) {
            Some(index) => (index, false),
            None => {
                self.entries.push(Entry {
                    tab: tab.clone(),
                    project: project.clone(),
                    panel: Panel::editor(session(), query),
                    mounts: 0,
                    is_mounted: false,
                });
                (self.entries.len() - 1, true)
            }
        };
        let entry = &mut self.entries[index];
        if !entry.is_mounted {
            entry.is_mounted = true;
            entry.mounts += 1;
        }
        Mounted {
            panel: &mut entry.panel,
            nonce: entry.mounts,
            is_created,
        }
    }

    /// Takes every body whose tab is no longer shown off the screen: its running search stops and
    /// comes back to be cancelled, it becomes the newest remembered entry, and the remembered
    /// entries beyond [`MEMORY_LIMIT`] are dropped oldest first.
    pub(crate) fn unmount_hidden(&mut self, shown: &HashSet<TabId>) -> Vec<Session> {
        let left = self
            .entries
            .extract_if(.., |entry| entry.is_mounted && !shown.contains(&entry.tab))
            .collect::<Vec<_>>();
        let mut cancelled = Vec::new();
        for mut entry in left {
            entry.is_mounted = false;
            cancelled.extend(entry.panel.stop());
            self.entries.push(entry);
        }
        while self
            .entries
            .iter()
            .filter(|entry| !entry.is_mounted)
            .count()
            > MEMORY_LIMIT
        {
            let Some(oldest) = self.entries.iter().position(|entry| !entry.is_mounted) else {
                break;
            };
            self.entries.remove(oldest);
        }
        cancelled
    }

    pub(crate) fn get_mut(&mut self, tab: &TabId) -> Option<&mut Panel> {
        self.entries
            .iter_mut()
            .find(|entry| entry.tab == *tab)
            .map(|entry| &mut entry.panel)
    }

    #[cfg(test)]
    pub(crate) fn get(&self, tab: &TabId) -> Option<&Panel> {
        self.entries
            .iter()
            .find(|entry| entry.tab == *tab)
            .map(|entry| &entry.panel)
    }
}

#[cfg(test)]
#[path = "search-editor-tests.rs"]
mod tests;

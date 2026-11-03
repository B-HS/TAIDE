use std::collections::{HashMap, HashSet};

use eframe::egui::{Color32, Event, Key, Ui};
use taide_model::error::AppResult;
use taide_model::git::StatusRow;
use taide_model::ids::TabId;
use taide_model::theme::{ResolvedTheme, ThemeType};
use taide_native_editor::document::{DocumentId, DocumentKey, EditorError, VirtualSide};
use taide_native_editor::store::EditorStore;
use taide_native_ui::diff_view::{Colors, Tokens};
use taide_native_ui::editor_surface::EditorTokens;

use crate::presentation;

const DIAGONAL_FILL_DARK: Color32 = Color32::from_rgba_unmultiplied_const(0xcc, 0xcc, 0xcc, 0x33);
const DIAGONAL_FILL_LIGHT: Color32 = Color32::from_rgba_unmultiplied_const(0x22, 0x22, 0x22, 0x33);
const UNCHANGED_CODE_DARK: Color32 = Color32::from_rgba_unmultiplied_const(0x74, 0x74, 0x74, 0x29);
const UNCHANGED_CODE_LIGHT: Color32 = Color32::from_rgba_unmultiplied_const(0xb8, 0xb8, 0xb8, 0x29);
pub(crate) const FAILED_TEXT_SIZE: f32 = 14.0;
pub(crate) const ORIGINAL_WINDOW: &str = "taide-diff-original";
pub(crate) const MODIFIED_WINDOW: &str = "taide-diff-modified";

/// Theme colors of the diff tab surface (`diff-pane.tsx`, monaco diff colors from `theme.ts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Appearance {
    pub(crate) colors: Colors,
    pub(crate) background: Color32,
    pub(crate) error: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| presentation::color(theme, key);
        Ok(Self {
            colors: Colors {
                inserted_line: color("diff.insertedLineBackground")?,
                removed_line: color("diff.removedLineBackground")?,
                diagonal_fill: match theme.theme_type {
                    ThemeType::Dark => DIAGONAL_FILL_DARK,
                    ThemeType::Light => DIAGONAL_FILL_LIGHT,
                },
                border: color("diff.border")?,
                stage: color("git.staged")?,
                foreground: color("editor.foreground")?,
                unchanged_code: match theme.theme_type {
                    ThemeType::Dark => UNCHANGED_CODE_DARK,
                    ThemeType::Light => UNCHANGED_CODE_LIGHT,
                },
                unchanged_region: color("appSidebar.background")?,
                unchanged_region_foreground: color("app.foreground")?,
            },
            background: color("editor.background")?,
            error: color("statusIndicator.error")?,
        })
    }
}

/// `diff-stageability.ts` `isDiffHunkStageable`: no file compare, and the path is not a conflict
/// row under either of its path forms.
pub(crate) fn is_stageable(compare_with: Option<&str>, path: &str, rows: &[StatusRow]) -> bool {
    compare_with.is_none()
        && !rows
            .iter()
            .any(|row| (row.abs_path == path || row.path == path) && row.is_conflicted)
}

/// Consumes Alt + the physical Backslash key (`event.code === 'Backslash'`).
pub(crate) fn take_side_by_side_toggle(ui: &Ui) -> bool {
    ui.input_mut(|input| {
        let before = input.events.len();
        input.events.retain(|event| {
            !matches!(
                event,
                Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.alt && physical_key.unwrap_or(*key) == Key::Backslash
            )
        });
        input.events.len() != before
    })
}

/// Per-tab render mode (`renderSideBySide`, true when the pane mounts) and which diff tabs drew in
/// the last frame; a tab that stops drawing loses its mode and its documents, like the unmounted
/// pane.
#[derive(Default)]
pub(crate) struct Tabs {
    side_by_side: HashMap<TabId, bool>,
    shown: HashSet<TabId>,
}

impl Tabs {
    pub(crate) fn is_side_by_side(&self, tab: &TabId) -> bool {
        self.side_by_side.get(tab).copied().unwrap_or(true)
    }

    pub(crate) fn shown(&mut self, tab: TabId) {
        self.side_by_side.entry(tab.clone()).or_insert(true);
        self.shown.insert(tab);
    }

    pub(crate) fn toggle(&mut self, tab: &TabId) {
        if let Some(side_by_side) = self.side_by_side.get_mut(tab) {
            *side_by_side = !*side_by_side;
        }
    }

    /// Ends a frame: forgets tabs that did not draw and returns the ones that did.
    pub(crate) fn finish_frame(&mut self) -> HashSet<TabId> {
        let shown = std::mem::take(&mut self.shown);
        self.side_by_side.retain(|tab, _| shown.contains(tab));
        shown
    }
}

/// Detaches and releases every diff document whose tab did not draw.
pub(crate) fn release_hidden(
    store: &mut EditorStore,
    shown: &HashSet<TabId>,
) -> Result<(), EditorError> {
    let hidden = store
        .documents()
        .snapshots()
        .filter_map(|snapshot| match &snapshot.key {
            DocumentKey::Virtual(key) if !shown.contains(&key.owner) => Some(snapshot.id),
            _ => None,
        })
        .collect::<Vec<DocumentId>>();
    for document in hidden {
        let views = store
            .views()
            .for_document(document)
            .map(|view| view.id)
            .collect::<Vec<_>>();
        for view in views {
            store.detach_view(view)?;
        }
        store.release_document(document)?;
    }
    Ok(())
}

pub(crate) fn window(side: VirtualSide) -> &'static str {
    match side {
        VirtualSide::Original => ORIGINAL_WINDOW,
        VirtualSide::Modified => MODIFIED_WINDOW,
    }
}

pub(crate) struct SyntaxTokens<'a>(pub(crate) &'a mut crate::editor_syntax::EditorSyntax);

impl Tokens for SyntaxTokens<'_> {
    fn tokens(&mut self, store: &EditorStore, document: DocumentId) -> Option<EditorTokens<'_>> {
        self.0.tokens(store, document)
    }
}

#[cfg(test)]
#[path = "git-diff-tests.rs"]
mod tests;

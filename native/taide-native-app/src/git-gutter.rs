use std::collections::{HashMap, HashSet};
use std::ops::Range;

use eframe::egui::{self, Color32};
use taide_model::error::AppResult;
use taide_model::git::{GutterHunk, HunkKind, StatusRow};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::theme::ResolvedTheme;
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, LaneMark, Stickiness,
};
use taide_native_editor::document::DocumentSnapshot;

use crate::git_conflict::Region;
use crate::presentation;

const DECORATION_Z_ORDER: u8 = 0;
const DIALOG_MAX_WIDTH: f32 = 512.0;

/// An `AlertDialog`: title, description and buttons in order with focus starting on the first
/// (Cancel); Escape answers the first and an outside click is ignored. Answers the pressed index.
pub(crate) fn alert(
    context: &egui::Context,
    id: egui::Id,
    (title, description): (&str, &str),
    buttons: &[String],
) -> Option<usize> {
    let response = egui::Modal::new(id).show(context, |ui| {
        ui.set_max_width(DIALOG_MAX_WIDTH);
        ui.heading(title);
        ui.label(description);
        ui.horizontal(|ui| {
            let pressed = buttons
                .iter()
                .map(|label| ui.button(label))
                .collect::<Vec<_>>();
            if let Some(first) = pressed.first()
                && !pressed.iter().any(egui::Response::has_focus)
            {
                first.request_focus();
            }
            pressed.iter().position(egui::Response::clicked)
        })
        .inner
    });
    let is_escaped = response.is_top_modal
        && !response.any_popup_open
        && context.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    response.inner.or(is_escaped.then_some(0))
}

/// Theme colors of the file editor's git surfaces (`global.css` gutter, conflict and blame
/// classes, `blame-footer-bar.tsx`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Appearance {
    pub(crate) added: Color32,
    pub(crate) modified: Color32,
    pub(crate) deleted: Color32,
    pub(crate) conflicted: Color32,
    pub(crate) current: Color32,
    pub(crate) incoming: Color32,
    pub(crate) blame_foreground: Color32,
    pub(crate) blame_background: Color32,
    pub(crate) border: Color32,
}

impl Appearance {
    pub(crate) fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        let color = |key| presentation::color(theme, key);
        Ok(Self {
            added: color("editorGutter.addedBackground")?,
            modified: color("editorGutter.modifiedBackground")?,
            deleted: color("editorGutter.deletedBackground")?,
            conflicted: color("git.conflicted")?,
            current: color("diff.insertedLineBackground")?,
            incoming: color("diff.removedLineBackground")?,
            blame_foreground: color("editorBlame.foreground")?,
            blame_background: color("editorBlame.background")?,
            border: color("app.border")?,
        })
    }
}

/// `conflict-status.ts` `isPathConflicted`: a conflicted status row at the tab's absolute path.
pub(crate) fn is_conflicted(rows: &[StatusRow], path: &str) -> bool {
    rows.iter()
        .any(|row| row.abs_path == path && row.is_conflicted)
}

/// The gutter hunk holding `line`, 1-based.
pub(crate) fn hunk_at(hunks: &[GutterHunk], line: u32) -> Option<(u32, u32)> {
    hunks
        .iter()
        .find(|hunk| (hunk.start..=hunk.end).contains(&line))
        .map(|hunk| (hunk.start, hunk.end))
}

/// `resolveSelectedLineRange`: a multi-line selection ending at the start of a line leaves that
/// line out.
pub(crate) fn selected_lines(start: u32, end: u32, ends_at_line_start: bool) -> (u32, u32) {
    if ends_at_line_start && end > start {
        (start, end - 1)
    } else {
        (start, end)
    }
}

fn layer(
    snapshot: &DocumentSnapshot,
    items: impl Iterator<Item = (Range<u32>, DecorationKind)>,
) -> DecorationLayer {
    let count = snapshot.rope.len_lines();
    DecorationLayer::new(
        snapshot.revision,
        DECORATION_Z_ORDER,
        items
            .filter_map(|(lines, kind)| {
                let first = lines.start.checked_sub(1)? as usize;
                let last = (lines.end.checked_sub(2)? as usize).min(count.checked_sub(1)?);
                (first <= last).then(|| Decoration {
                    bytes: snapshot.rope.line_to_byte(first)..snapshot.rope.line_to_byte(last),
                    kind,
                    stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                })
            })
            .collect(),
    )
}

/// The hunk marks of a clean file (`taide-gutter-*`): 3px bars for added and modified lines, a
/// triangle where lines were deleted.
pub(crate) fn hunk_layer(
    snapshot: &DocumentSnapshot,
    hunks: &[GutterHunk],
    appearance: Appearance,
) -> DecorationLayer {
    layer(
        snapshot,
        hunks.iter().map(|hunk| {
            let (mark, color) = match hunk.kind {
                HunkKind::Added => (LaneMark::Bar, appearance.added),
                HunkKind::Modified => (LaneMark::Bar, appearance.modified),
                HunkKind::Deleted => (LaneMark::DeletedTriangle, appearance.deleted),
            };
            (
                hunk.start..hunk.end + 1,
                DecorationKind::Lane {
                    mark,
                    color: color.to_srgba_unmultiplied(),
                },
            )
        }),
    )
}

/// A conflicted file's regions: the current side over the inserted line background, the incoming
/// side and its markers over the removed one, and a conflict bar at each start line.
pub(crate) fn conflict_layer(
    snapshot: &DocumentSnapshot,
    regions: &[Region],
    appearance: Appearance,
) -> DecorationLayer {
    layer(
        snapshot,
        regions.iter().flat_map(|region| {
            [
                (
                    region.start..region.base.unwrap_or(region.separator),
                    DecorationKind::LineBackground(appearance.current.to_srgba_unmultiplied()),
                ),
                (
                    region.separator..region.end + 1,
                    DecorationKind::LineBackground(appearance.incoming.to_srgba_unmultiplied()),
                ),
                (
                    region.start..region.start + 1,
                    DecorationKind::Lane {
                        mark: LaneMark::Bar,
                        color: appearance.conflicted.to_srgba_unmultiplied(),
                    },
                ),
            ]
        }),
    )
}

/// One pane's file editor git state (`EditorPane`): its file, blame, the hunk whose discard
/// dialog is open and its conflicts. A new file in the pane starts over.
pub(crate) struct Editor {
    pub(crate) project: ProjectId,
    pub(crate) tab: TabId,
    pub(crate) path: String,
    pub(crate) blame: crate::git_blame::Blame,
    pub(crate) pending_hunk: Option<(u32, u32)>,
    pub(crate) conflicts: crate::git_conflict::Conflicts,
}

/// The file editors drawn in the last frame, by pane.
#[derive(Default)]
pub(crate) struct Editors {
    panes: HashMap<PaneId, Editor>,
    shown: HashSet<PaneId>,
    pub(crate) appearance: Option<Appearance>,
}

impl Editors {
    /// The pane's editor for this frame, started over when the pane now shows another file.
    pub(crate) fn shown(
        &mut self,
        pane: &PaneId,
        project: &ProjectId,
        tab: &TabId,
        path: &str,
    ) -> &mut Editor {
        self.shown.insert(pane.clone());
        let editor = self.panes.entry(pane.clone()).or_insert_with(|| Editor {
            project: project.clone(),
            tab: tab.clone(),
            path: path.to_owned(),
            blame: Default::default(),
            pending_hunk: None,
            conflicts: Default::default(),
        });
        if editor.path != path || editor.project != *project {
            *editor = Editor {
                project: project.clone(),
                tab: tab.clone(),
                path: path.to_owned(),
                blame: Default::default(),
                pending_hunk: None,
                conflicts: Default::default(),
            };
        }
        editor.tab = tab.clone();
        editor
    }

    pub(crate) fn get(&self, pane: &PaneId) -> Option<&Editor> {
        self.panes.get(pane)
    }

    pub(crate) fn get_mut(&mut self, pane: &PaneId) -> Option<&mut Editor> {
        self.panes.get_mut(pane)
    }

    /// The editor drawing `tab` in `pane`, if that is the pane's current file.
    pub(crate) fn of_tab(&mut self, pane: &PaneId, tab: &TabId) -> Option<&mut Editor> {
        self.panes.get_mut(pane).filter(|editor| editor.tab == *tab)
    }

    /// Ends a frame: forgets editors that did not draw, like an unmounted pane.
    pub(crate) fn finish_frame(&mut self) {
        let shown = std::mem::take(&mut self.shown);
        self.panes.retain(|pane, _| shown.contains(pane));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }
}

#[cfg(test)]
#[path = "git-gutter-tests.rs"]
mod tests;

use std::time::Instant;

use eframe::egui::{self, Ui};
use taide_model::error::AppResult;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_native_editor::decoration::DecorationLayer;
use taide_native_editor::document::{DocumentSnapshot, Edit, UndoGroup};
use taide_native_editor::store::Transaction;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::command_registry::GitRun;
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::intents::git::{ConflictAction, Intent};

use super::{Command, Key, Parts, SideText, State, Surface, show_surface, take_fresh};
use crate::application::{AppSurfaces, NativeApplication, WINDOW_LABEL, editor_error};
use crate::git_cache::Invalidation;
use crate::git_conflict::{Compare, Resolution};
use crate::git_gutter::{hunk_at, selected_lines};
use crate::host::HostCommand;
use crate::presentation;

/// Share of the window height the conflict compare dialog takes (`h-[70vh]`).
const COMPARE_HEIGHT_SHARE: f32 = 0.7;
/// The compare dialog's widest size (`sm:max-w-4xl`).
const COMPARE_MAX_WIDTH: f32 = 896.0;
/// Room the dialog keeps on both sides (`max-w-[calc(100%-2rem)]`).
const COMPARE_MARGIN: f32 = 32.0;
const COMPARE_HEADER_HEIGHT: f32 = 32.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_STROKE: f32 = 1.5;
const CLOSE_INSET: f32 = 4.0;

/// One drawn file editor's parts the Git surfaces read (`EditorPane`).
pub(crate) struct EditorFrame<'f> {
    pub(crate) project: &'f ProjectId,
    pub(crate) pane: &'f PaneId,
    pub(crate) tab: &'f TabId,
    pub(crate) path: &'f str,
    pub(crate) view: ViewId,
    pub(crate) footer: Option<egui::Rect>,
    pub(crate) geometry: &'f EditorGeometry,
    pub(crate) response: &'f egui::Response,
    pub(crate) has_folding: bool,
    pub(crate) font: egui::FontId,
}

/// A file editor drawn this frame with its 1-based cursor line, keeping its gutter, blame and
/// conflict reads observed.
pub(crate) struct ShownEditor {
    project: ProjectId,
    pane: PaneId,
    tab: TabId,
    path: String,
    cursor_line: u32,
}

impl State {
    fn is_conflicted(&self, project: &ProjectId, path: &str) -> bool {
        self.cache
            .status_result(project)
            .and_then(Result::ok)
            .is_some_and(|status| crate::git_gutter::is_conflicted(&status.rows, path))
    }

    /// The file editor's gutter marks (`use-editor-git-gutter-and-conflicts.ts`): conflict regions
    /// while the file is conflicted, otherwise its working tree hunks.
    pub(crate) fn editor_layers(
        &self,
        project: &ProjectId,
        pane: &PaneId,
        path: &str,
        snapshot: &DocumentSnapshot,
    ) -> Vec<DecorationLayer> {
        let Some(appearance) = self.editors.appearance else {
            return Vec::new();
        };
        if self.is_conflicted(project, path) {
            let regions = self
                .editors
                .get(pane)
                .filter(|editor| editor.path == path)
                .map_or(&[][..], |editor| editor.conflicts.regions());
            return vec![crate::git_gutter::conflict_layer(
                snapshot, regions, appearance,
            )];
        }
        vec![crate::git_gutter::hunk_layer(
            snapshot,
            self.cache.gutter(project, path),
            appearance,
        )]
    }
}

fn view_key(pane: &PaneId, tab: &TabId) -> ViewKey {
    ViewKey {
        window: WINDOW_LABEL.into(),
        pane: pane.clone(),
        tab: tab.clone(),
    }
}

impl NativeApplication {
    pub(crate) fn observe_git_editors(&mut self, context: &egui::Context, shown: Vec<ShownEditor>) {
        for editor in shown {
            self.observe_git_editor(
                context,
                (&editor.project, &editor.pane, &editor.tab, &editor.path),
                editor.cursor_line,
            );
        }
    }

    pub(super) fn apply_git_editor_intent(&mut self, intent: Intent, context: &egui::Context) {
        match intent {
            Intent::EditorLanePress { pane, line } => {
                let Some(editor) = self.git_state.editors.get(&pane) else {
                    return;
                };
                let (project, path) = (editor.project.clone(), editor.path.clone());
                let is_conflicted = self.git_state.is_conflicted(&project, &path);
                let hunk = hunk_at(self.git_state.cache.gutter(&project, &path), line);
                let Some(editor) = self.git_state.editors.get_mut(&pane) else {
                    return;
                };
                if is_conflicted {
                    if let Some(region) = editor.conflicts.region_at(line) {
                        editor.conflicts.pending = Some(region);
                    }
                } else if hunk.is_some() {
                    editor.pending_hunk = hunk;
                }
                context.request_repaint();
            }
            Intent::ResolveHunkDiscard { pane, confirmed } => {
                let Some(editor) = self.git_state.editors.get_mut(&pane) else {
                    return;
                };
                let Some((start, end)) = editor.pending_hunk.take() else {
                    return;
                };
                if confirmed {
                    let (project, path) = (editor.project.clone(), editor.path.clone());
                    self.submit(HostCommand::Git(Command::DiscardHunk {
                        project,
                        path,
                        start,
                        end,
                    }));
                }
                context.request_repaint();
            }
            Intent::ResolveConflict { pane, action } => {
                self.resolve_git_conflict(&pane, action);
                context.request_repaint();
            }
            Intent::CloseConflictCompare { pane } => {
                if let Some(editor) = self.git_state.editors.get_mut(&pane) {
                    editor.conflicts.compare = None;
                }
                context.request_repaint();
            }
            Intent::ToggleBlame { pane, tab } => {
                self.toggle_git_blame(&pane, &tab);
                context.request_repaint();
            }
            Intent::StageSelection { pane, tab } => self.stage_git_selection(&pane, &tab),
            _ => {}
        }
    }

    /// `EditorPane`'s Git reads for this frame: status, gutter and user stay observed, the rested
    /// cursor line's blame is read and adopted, the overlay reads its lines, a conflicted file's
    /// markers are parsed and the compare dialog's sides are read while it is requested.
    fn observe_git_editor(
        &mut self,
        context: &egui::Context,
        (project, pane, tab, path): (&ProjectId, &PaneId, &TabId, &str),
        cursor_line: u32,
    ) {
        let now = Instant::now();
        let is_conflicted = self.git_state.is_conflicted(project, path);
        let snapshot = self
            .store
            .views()
            .find(&view_key(pane, tab))
            .and_then(|view| self.store.views().get(view))
            .and_then(|view| self.store.documents().snapshot(view.document).ok());
        let git = &mut self.git_state;
        for key in [
            Key::Status,
            Key::Gutter {
                path: path.to_owned(),
            },
            Key::CurrentUser,
        ] {
            git.cache.observe(project, key, now);
        }
        let editor = git.editors.shown(pane, project, tab, path);
        if let Some(wait) = editor.blame.cursor(cursor_line, now) {
            context.request_repaint_after(wait);
        }
        if let Some(line) = editor.blame.settled() {
            let key = Key::BlameLine {
                path: path.to_owned(),
                line,
            };
            git.cache.observe(project, key.clone(), now);
            editor.blame.adopt(
                git.cache.blame(project, &key),
                git.cache.fetched_at(project, &key),
            );
        }
        if let Some(lines) = editor.blame.overlay() {
            git.cache.observe(
                project,
                Key::BlameOverlay {
                    path: path.to_owned(),
                    lines,
                },
                now,
            );
        }
        let revision = snapshot.as_ref().map_or(0, |snapshot| snapshot.revision);
        match editor.conflicts.observe(is_conflicted, revision, now) {
            Ok(true) => {
                if let Some(snapshot) = &snapshot {
                    editor
                        .conflicts
                        .parsed(snapshot.revision, &snapshot.rope.to_string());
                    context.request_repaint();
                }
            }
            Ok(false) => {}
            Err(wait) => context.request_repaint_after(wait),
        }
        let Some(compare) = &mut editor.conflicts.compare else {
            return;
        };
        let key = Key::ConflictSides {
            path: path.to_owned(),
        };
        git.cache.observe(project, key.clone(), now);
        let fetched_at = git.cache.fetched_at(project, &key);
        match git.cache.diff(project, &key) {
            Some(Ok(_)) => {
                compare.seen = fetched_at;
                git.diffs.shown(compare.owner.clone());
            }
            Some(Err(error)) if fetched_at != compare.seen => {
                compare.seen = fetched_at;
                self.toasts.ipc_error(&self.locale, error, now);
            }
            _ => {}
        }
    }

    /// `git.toggleBlame` on the pane's file editor: on with the document's current line count, or
    /// off.
    pub(crate) fn toggle_git_blame(&mut self, pane: &PaneId, tab: &TabId) {
        let Some(line_count) = self
            .store
            .views()
            .find(&view_key(pane, tab))
            .and_then(|view| self.store.views().get(view))
            .and_then(|view| self.store.documents().snapshot(view.document).ok())
            .map(|snapshot| snapshot.rope.len_lines() as u32)
        else {
            return;
        };
        if let Some(editor) = self.git_state.editors.of_tab(pane, tab) {
            editor.blame.toggle_overlay(line_count);
        }
    }

    /// `taide.gitStageSelection`: a selection stages its lines, a bare cursor the gutter hunk it
    /// sits in; a conflicted file is left alone.
    pub(crate) fn stage_git_selection(&mut self, pane: &PaneId, tab: &TabId) {
        let Some(editor) = self.git_state.editors.of_tab(pane, tab) else {
            return;
        };
        let (project, path) = (editor.project.clone(), editor.path.clone());
        if self.git_state.is_conflicted(&project, &path) {
            return;
        }
        let Some(view) = self
            .store
            .views()
            .find(&view_key(pane, tab))
            .and_then(|view| self.store.views().get(view))
        else {
            return;
        };
        let Ok(snapshot) = self.store.documents().snapshot(view.document) else {
            return;
        };
        let selection = view.selection.selections[view.selection.primary];
        let (low, high) = (
            selection.anchor.min(selection.head),
            selection.anchor.max(selection.head),
        );
        let rope = &snapshot.rope;
        let command = if low == high {
            let line = rope.byte_to_line(selection.head) as u32 + 1;
            let Some((start, end)) = hunk_at(self.git_state.cache.gutter(&project, &path), line)
            else {
                return;
            };
            Command::StageHunk {
                project,
                path,
                start,
                end,
            }
        } else {
            let last = rope.byte_to_line(high);
            let (start, end) = selected_lines(
                rope.byte_to_line(low) as u32 + 1,
                last as u32 + 1,
                rope.line_to_byte(last) == high,
            );
            Command::StageLines {
                project,
                path,
                start,
                end,
            }
        };
        self.submit(HostCommand::Git(command));
    }

    /// A Git item of the file editor's context menu, run on the editor it was opened from.
    pub(crate) fn run_git_editor_command(
        &mut self,
        command: GitRun,
        source: ViewId,
        viewport: egui::ViewportId,
    ) -> bool {
        let Some(key) = self.store.views().get(source).map(|view| view.key.clone()) else {
            return false;
        };
        match command {
            GitRun::ToggleBlame => self.toggle_git_blame(&key.pane, &key.tab),
            GitRun::StageSelection => self.stage_git_selection(&key.pane, &key.tab),
            GitRun::OpenFileHistory => {
                let Some(editor) = self.git_state.editors.of_tab(&key.pane, &key.tab) else {
                    return false;
                };
                let (project, path) = (editor.project.clone(), editor.path.clone());
                self.git_state.histories.open(viewport, project, path);
            }
            GitRun::RevertHead | GitRun::CreateTagOnHead => return false,
        }
        true
    }

    /// `applyConflictResolution`: the chosen side replaces the region in one undoable edit, and
    /// once no markers remain the file is written and re-staged; Compare asks for the sides.
    fn resolve_git_conflict(&mut self, pane: &PaneId, action: ConflictAction) {
        let Some(editor) = self.git_state.editors.get_mut(pane) else {
            return;
        };
        let Some(region) = editor.conflicts.pending.take() else {
            return;
        };
        let (project, path, tab) = (
            editor.project.clone(),
            editor.path.clone(),
            editor.tab.clone(),
        );
        let resolution = match action {
            ConflictAction::Cancel => return,
            ConflictAction::Compare => {
                let key = Key::ConflictSides { path };
                let seen = self.git_state.cache.fetched_at(&project, &key);
                let owner = editor
                    .conflicts
                    .compare
                    .take()
                    .map_or_else(TabId::new, |compare| compare.owner);
                editor.conflicts.compare = Some(Compare { owner, seen });
                self.git_state.cache.refetch(&project, &key);
                return;
            }
            ConflictAction::AcceptBoth => Resolution::Both,
            ConflictAction::AcceptIncoming => Resolution::Incoming,
            ConflictAction::AcceptCurrent => Resolution::Current,
        };
        if let Err(error) =
            self.apply_conflict_resolution(project, path, &tab, pane, (region, resolution))
        {
            self.status = Some(editor_error(error).to_string());
        }
    }

    fn apply_conflict_resolution(
        &mut self,
        project: ProjectId,
        path: String,
        tab: &TabId,
        pane: &PaneId,
        (region, resolution): (crate::git_conflict::Region, Resolution),
    ) -> Result<(), taide_native_editor::document::EditorError> {
        let Some(view) = self.store.views().find(&view_key(pane, tab)) else {
            return Ok(());
        };
        let document = self
            .store
            .views()
            .get(view)
            .ok_or(taide_native_editor::document::EditorError::NotFound)?
            .document;
        let snapshot = self.store.documents().snapshot(document)?;
        let current = snapshot.rope.to_string();
        if !crate::git_conflict::parse(&current).contains(&region) {
            return Ok(());
        }
        let content = crate::git_conflict::resolve(&current, region, resolution);
        self.store.apply_separate(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text: content.clone(),
                }],
                group: UndoGroup(snapshot.revision),
                origin: Some(view),
                selection_after: None,
            },
        )?;
        self.editor_menu_changes.insert(document, view);
        if !crate::git_conflict::parse(&content).is_empty() {
            return Ok(());
        }
        let snapshot = self.store.save_snapshot(document)?;
        self.submit(HostCommand::Git(Command::ResolveConflict {
            project,
            path,
            content,
            snapshot,
        }));
        Ok(())
    }

    /// A resolved file settles like a save (`settleAfterDiskWrite`) and reports success.
    pub(super) fn finish_conflict_resolution(
        &mut self,
        context: &egui::Context,
        project: &ProjectId,
        snapshot: taide_native_editor::store::SaveSnapshot,
        result: AppResult<Option<taide_model::file::OpenedFile>>,
    ) {
        let now = Instant::now();
        let file = match result {
            Ok(file) => file,
            Err(error) => {
                self.toasts.ipc_error(&self.locale, &error, now);
                return;
            }
        };
        self.git_state
            .cache
            .invalidate(project, &Invalidation::repository());
        if let Some(file) = file {
            self.apply_editor_reply(
                context,
                crate::domains::editor::Reply::Saved {
                    snapshot,
                    result: Ok(file),
                },
            );
        }
        self.toasts.success(
            presentation::message(&self.locale, "git.conflictResolved", &[]),
            now,
        );
    }
}

impl AppSurfaces<'_> {
    /// Draws a file editor's Git surfaces after the editor: the blame footer and overlay, gutter
    /// lane presses and the hunk, conflict and compare dialogs.
    pub(crate) fn show_git_editor(
        &mut self,
        ui: &mut Ui,
        frame: EditorFrame<'_>,
        intents: &mut Vec<ShellIntent>,
    ) -> AppResult<()> {
        let git = self.git;
        let Some(appearance) = git.editors.appearance else {
            return Ok(());
        };
        let view =
            self.store.views().get(frame.view).ok_or_else(|| {
                editor_error(taide_native_editor::document::EditorError::NotFound)
            })?;
        let snapshot = self
            .store
            .documents()
            .snapshot(view.document)
            .map_err(editor_error)?;
        let head = view.selection.selections[view.selection.primary].head;
        let cursor_line = snapshot.rope.byte_to_line(head) as u32 + 1;
        let line_count = snapshot.rope.len_lines() as u32;
        let editor = git
            .editors
            .get(frame.pane)
            .filter(|editor| editor.path == frame.path && editor.tab == *frame.tab);
        let current_user = git.cache.current_user(frame.project);
        if let Some(rect) = frame.footer {
            crate::git_blame::paint_footer(
                ui,
                rect,
                editor.map_or_else(String::new, |editor| {
                    editor.blame.footer_text(line_count, current_user)
                }),
                (
                    appearance.blame_foreground,
                    appearance.blame_background,
                    appearance.border,
                ),
            );
        }
        if let Some(lines) = editor.and_then(|editor| editor.blame.overlay())
            && let Some(Ok(blame)) = git.cache.blame(
                frame.project,
                &Key::BlameOverlay {
                    path: frame.path.to_owned(),
                    lines,
                },
            )
        {
            crate::git_blame::paint_overlay(
                ui,
                frame.geometry,
                &snapshot,
                blame,
                (current_user, crate::git_blame::now_unix()),
                (frame.font.clone(), appearance.blame_foreground),
            );
        }
        let parts = (frame.geometry, frame.response);
        let pressed = if git.is_conflicted(frame.project, frame.path) {
            let regions = editor.map_or(&[][..], |editor| editor.conflicts.regions());
            taide_native_ui::diff_view::lane_line(ui, parts, frame.has_folding, true, |line| {
                regions.iter().any(|region| region.start == line)
            })
        } else {
            let hunks = git.cache.gutter(frame.project, frame.path);
            taide_native_ui::diff_view::lane_line(ui, parts, frame.has_folding, false, |line| {
                hunk_at(hunks, line).is_some()
            })
        };
        if let Some(line) = pressed {
            intents.push(ShellIntent::Git(Intent::EditorLanePress {
                pane: frame.pane.clone(),
                line,
            }));
        }
        self.git_editors.push(ShownEditor {
            project: frame.project.clone(),
            pane: frame.pane.clone(),
            tab: frame.tab.clone(),
            path: frame.path.to_owned(),
            cursor_line,
        });
        let Some(editor) = editor else {
            return Ok(());
        };
        let text = |key| presentation::message(self.locale, key, &[]);
        let range = |key, start: u32, end: u32| {
            let (start, end) = (start.to_string(), end.to_string());
            presentation::message(self.locale, key, &[("start", &start), ("end", &end)])
        };
        if let Some((start, end)) = editor.pending_hunk {
            let description = range("git.discardHunkDescription", start, end);
            if let Some(index) = crate::git_gutter::alert(
                ui.ctx(),
                egui::Id::new(("taide-git-discard-hunk", frame.pane)),
                (&text("git.discardHunkTitle"), &description),
                &[text("common.cancel"), text("git.discardConfirm")],
            ) {
                intents.push(ShellIntent::Git(Intent::ResolveHunkDiscard {
                    pane: frame.pane.clone(),
                    confirmed: index == 1,
                }));
            }
        }
        if let Some(region) = editor.conflicts.pending {
            let description = range("git.resolveConflictDescription", region.start, region.end);
            let actions = [
                ConflictAction::Cancel,
                ConflictAction::Compare,
                ConflictAction::AcceptBoth,
                ConflictAction::AcceptIncoming,
                ConflictAction::AcceptCurrent,
            ];
            if let Some(index) = crate::git_gutter::alert(
                ui.ctx(),
                egui::Id::new(("taide-git-resolve-conflict", frame.pane)),
                (&text("git.resolveConflictTitle"), &description),
                &[
                    text("common.cancel"),
                    text("git.compareChanges"),
                    text("git.acceptBothChanges"),
                    text("git.acceptIncomingChange"),
                    text("git.acceptCurrentChange"),
                ],
            ) {
                intents.push(ShellIntent::Git(Intent::ResolveConflict {
                    pane: frame.pane.clone(),
                    action: actions[index],
                }));
            }
        }
        if let Some(compare) = &editor.conflicts.compare
            && let Some(Ok(diff)) = git.cache.diff(
                frame.project,
                &Key::ConflictSides {
                    path: frame.path.to_owned(),
                },
            )
            && let Some(diff_appearance) = git.diff_appearance
        {
            let fetched_at = git.cache.fetched_at(
                frame.project,
                &Key::ConflictSides {
                    path: frame.path.to_owned(),
                },
            );
            let closed = self.show_conflict_compare(
                ui.ctx(),
                (frame.pane, &compare.owner, fetched_at),
                &snapshot.metadata.language_id,
                diff,
                diff_appearance,
            )?;
            if closed {
                intents.push(ShellIntent::Git(Intent::CloseConflictCompare {
                    pane: frame.pane.clone(),
                }));
            }
        }
        Ok(())
    }

    /// `ConflictCompareDialog`: ours against theirs side by side in a dialog 70% of the window
    /// tall; answers whether it was closed.
    fn show_conflict_compare(
        &mut self,
        context: &egui::Context,
        (pane, owner, fetched_at): (&PaneId, &TabId, Option<Instant>),
        language_id: &str,
        diff: &crate::git_cache::Diff,
        appearance: crate::git_diff::Appearance,
    ) -> AppResult<bool> {
        let content = context.content_rect();
        let size = egui::vec2(
            COMPARE_MAX_WIDTH
                .min(content.width() - COMPARE_MARGIN)
                .max(0.0),
            content.height() * COMPARE_HEIGHT_SHARE,
        );
        let hide_unchanged = self
            .services
            .state
            .settings
            .read()
            .editor_diff_hide_unchanged_regions;
        let title = presentation::message(self.locale, "git.compareChanges", &[]);
        let close_label = presentation::message(self.locale, "common.close", &[]);
        let parts = Parts {
            store: self.store,
            editor: self.editor,
            syntax: self.editor_syntax,
            services: self.services,
            display: self.editor_display_colors,
            brackets: self.editor_bracket_colors,
        };
        let id = egui::Id::new(("taide-git-conflict-compare", pane));
        let response = egui::Modal::new(id).show(context, |ui| -> AppResult<bool> {
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            let header = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width(), COMPARE_HEADER_HEIGHT),
            );
            ui.painter().text(
                header.left_center(),
                egui::Align2::LEFT_CENTER,
                &title,
                egui::TextStyle::Heading.resolve(ui.style()),
                ui.visuals().strong_text_color(),
            );
            let close = egui::Rect::from_center_size(
                egui::pos2(header.right() - CLOSE_SIZE / 2.0, header.center().y),
                egui::Vec2::splat(CLOSE_SIZE),
            );
            let close_response = ui.interact(close, id.with("close"), egui::Sense::click());
            close_response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &close_label)
            });
            let stroke = egui::Stroke::new(CLOSE_STROKE, ui.visuals().text_color());
            let mark = close.shrink(CLOSE_INSET);
            ui.painter()
                .line_segment([mark.left_top(), mark.right_bottom()], stroke);
            ui.painter()
                .line_segment([mark.right_top(), mark.left_bottom()], stroke);
            let body = egui::Rect::from_min_max(egui::pos2(rect.left(), header.bottom()), rect.max);
            let fresh = take_fresh(ui, owner, fetched_at);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
            child.set_clip_rect(body.intersect(ui.clip_rect()));
            show_surface(
                &mut child,
                parts,
                Surface {
                    owner,
                    pane,
                    language_id,
                    sides: [
                        SideText::Read {
                            text: &diff.sides.original,
                            fresh,
                        },
                        SideText::Read {
                            text: &diff.sides.modified,
                            fresh,
                        },
                    ],
                    hunks: &diff.hunks,
                    side_by_side: true,
                    stage_lanes: false,
                    hide_unchanged,
                },
                appearance.colors,
            )?;
            Ok(close_response.clicked())
        });
        let is_closing = response.backdrop_response.clicked()
            || taide_native_ui::modal::takes_escape(context, &response);
        Ok(response.inner? || is_closing)
    }
}

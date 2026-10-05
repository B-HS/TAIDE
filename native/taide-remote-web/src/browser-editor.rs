use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

use egui::Ui;
use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_model::project::ProjectRef;
use taide_native_editor::document::{DiskChoice, DocumentId, EditorError};
use taide_native_editor::indent::{IndentOptions, resolve};
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_surface::{EditorOutput, NativeEditor};

use crate::dirty::{DirtyFailure, DirtyState, FlushState};
use crate::files::{ChoiceRequest, FileEvent, FileFailure, FileState, FileSurface, FileViews};
use crate::mirror_runtime::MirrorRuntime;
use crate::mirror_writes::{
    ClearExpected, MirrorError, MirrorFailure, MirrorFlushStatus, MirrorKey, MirrorOutcome,
    MirrorWrites,
};
use crate::mirrors::{MirrorState, is_within_root};
use crate::{BrowserEvent, BrowserWorkbench, Delivery, InvokeError, WorkbenchError};

#[derive(Debug)]
pub enum EditorBrowserError {
    Workbench(WorkbenchError),
    Editor(EditorError),
    File(FileFailure),
    Presentation(AppError),
    Mirror(MirrorError),
}

pub struct BrowserEditor {
    workbench: BrowserWorkbench,
    files: FileViews,
    dirty: DirtyState,
    file_events: Vec<FileEvent>,
    mirrors: MirrorState,
    scopes: HashMap<ViewKey, ProjectScope>,
    mirror_restore_failures: Vec<(String, EditorError)>,
    mirror_writes: MirrorWrites,
    mirror_runtime: Option<MirrorRuntime>,
    mirror_time: Duration,
    mirror_runtime_error: Option<MirrorError>,
    pending_mirror_clears: BTreeSet<MirrorKey>,
    settings_views: taide_native_ui::settings_view::Views,
    theme_errors: Vec<AppError>,
    tooltips: taide_native_ui::tooltips::Provider,
    toasts: taide_native_ui::toast::Toasts,
    feedback: Vec<Feedback>,
    preview_theme: Option<taide_model::theme::ResolvedTheme>,
    app_files: crate::app_files::AppFiles,
    app_file_close_failures: Vec<crate::shell::Failure>,
    keybindings: Option<taide_native_ui::keybinding_editor::Editor>,
}

enum Feedback {
    Ipc(AppError),
    Settings(AppError),
    AppFile(crate::app_files::Error),
    Snippet(taide_native_ui::snippet_editor::Notice),
}

struct ProjectScope {
    project: ProjectId,
    path: String,
    document: Option<DocumentId>,
}

impl BrowserEditor {
    pub fn new(limits: EditorLimits, wake: Rc<dyn Fn()>) -> Result<Self, EditorBrowserError> {
        let files = FileViews::new(limits).map_err(EditorBrowserError::Editor)?;
        let mirror_runtime =
            MirrorRuntime::new(Rc::clone(&wake)).map_err(EditorBrowserError::Mirror)?;
        let mirror_time = mirror_runtime.now().map_err(EditorBrowserError::Mirror)?;
        let workbench = BrowserWorkbench::new(wake).map_err(EditorBrowserError::Workbench)?;
        let mut settings_views = taide_native_ui::settings_view::Views::default();
        settings_views.set_font_painter(crate::font_preview::Renderer::default());
        Ok(Self {
            workbench,
            files,
            dirty: DirtyState::default(),
            file_events: Vec::new(),
            mirrors: MirrorState::default(),
            scopes: HashMap::new(),
            mirror_restore_failures: Vec::new(),
            mirror_writes: MirrorWrites::default(),
            mirror_runtime: Some(mirror_runtime),
            mirror_time,
            mirror_runtime_error: None,
            pending_mirror_clears: BTreeSet::new(),
            settings_views,
            theme_errors: Vec::new(),
            tooltips: taide_native_ui::tooltips::Provider::default(),
            toasts: taide_native_ui::toast::Toasts::new()
                .map_err(EditorBrowserError::Presentation)?,
            feedback: Vec::new(),
            preview_theme: None,
            app_files: crate::app_files::AppFiles::default(),
            app_file_close_failures: Vec::new(),
            keybindings: None,
        })
    }

    pub fn workbench(&self) -> &BrowserWorkbench {
        &self.workbench
    }

    pub fn begin_ui_frame(&mut self, context: &egui::Context) {
        self.begin_settings_frame();
        self.tooltips.begin_frame(context);
    }

    pub fn finish_ui_frame(&mut self, context: &egui::Context) {
        self.finish_settings_frame();
        let preview = self
            .settings_views
            .preview_where(context.viewport_id(), |owner| {
                self.settings_owner_active(owner)
            })
            .cloned();
        if preview != self.preview_theme {
            self.preview_theme = preview;
            context.request_repaint();
        }
    }

    pub fn keybindings_open(&self) -> bool {
        self.keybindings
            .as_ref()
            .is_some_and(|editor| editor.is_open())
    }

    #[cfg(feature = "inspection")]
    pub fn keybindings_inspection(
        &self,
        context: &egui::Context,
    ) -> Option<taide_native_ui::keybinding_editor::KeybindingInspection> {
        self.keybindings
            .as_ref()
            .map(|editor| editor.inspection(context))
    }

    pub fn open_keybindings(&mut self, context: &egui::Context) -> Result<(), EditorBrowserError> {
        if self.keybindings.is_none() {
            let theme = self
                .theme_for_viewport(context.viewport_id())
                .ok_or_else(|| {
                    EditorBrowserError::Presentation(AppError::Internal(
                        "remote Keybinding theme is unavailable".into(),
                    ))
                })?;
            self.keybindings = Some(
                taide_native_ui::keybinding_editor::Editor::new(
                    theme,
                    &self.workbench.collation_locale(),
                    context.os().is_mac(),
                )
                .map_err(EditorBrowserError::Presentation)?,
            );
        }
        if let Some(editor) = self.keybindings.as_mut() {
            editor.open(context);
        }
        Ok(())
    }

    fn show_keybindings(
        &mut self,
        context: &egui::Context,
        enabled: bool,
    ) -> Result<(), EditorBrowserError> {
        if self.keybindings.is_none() {
            return Ok(());
        }
        let scope = self
            .workbench
            .snapshot()
            .and_then(|snapshot| snapshot.focused_tab())
            .map(|tab| taide_native_ui::keymap::Context {
                editor: matches!(
                    tab.kind,
                    taide_model::layout::TabKind::File { .. }
                        | taide_model::layout::TabKind::Untitled { .. }
                        | taide_model::layout::TabKind::AppFile { .. }
                ),
                terminal: matches!(tab.kind, taide_model::layout::TabKind::Terminal { .. }),
            })
            .unwrap_or_default();
        let theme = if enabled {
            self.theme_for_viewport(context.viewport_id())
        } else {
            self.workbench.presentation().theme()
        };
        let Some(theme) = theme else {
            return Ok(());
        };
        let appearance = taide_native_ui::keybinding_editor::Appearance::new(theme)
            .map_err(EditorBrowserError::Presentation)?;
        let tooltip_appearance = taide_native_ui::tooltips::Appearance::new(theme)
            .map_err(EditorBrowserError::Presentation)?;
        let (Some(locale), Some(settings), Some(editor)) = (
            self.workbench.presentation().locale(),
            self.workbench.shell().state().settings(),
            self.keybindings.as_mut(),
        ) else {
            return Ok(());
        };
        editor.set_appearance(appearance);
        editor.observe_closed(scope, taide_native_ui::toast::Instant::now());
        let output = editor
            .show(
                context,
                locale,
                settings.keymap_overrides.as_deref(),
                enabled,
            )
            .map_err(EditorBrowserError::Presentation)?;
        output.show_tooltips(locale, &self.tooltips, &tooltip_appearance);
        for warning in output.warnings {
            self.toasts
                .warning(warning, taide_native_ui::toast::Instant::now());
        }
        for overrides in output.saves {
            let result = overrides.json().map_err(EditorBrowserError::Presentation)?;
            if let Err(error) = self.workbench.change_preference(
                taide_native_ui::settings_controls::Change::KeymapOverrides(result),
            ) {
                self.feedback.push(Feedback::Settings(
                    crate::settings_catalog::invocation_error(error),
                ));
            }
        }
        Ok(())
    }

    pub fn theme_for_viewport(
        &self,
        viewport: egui::ViewportId,
    ) -> Option<&taide_model::theme::ResolvedTheme> {
        self.settings_views
            .preview_where(viewport, |owner| self.settings_owner_active(owner))
            .or_else(|| self.workbench.presentation().theme())
    }

    fn settings_owner_active(&self, owner: &taide_native_ui::settings_owner::Owner) -> bool {
        self.workbench.snapshot().is_some_and(|snapshot| {
            snapshot.project(&owner.project).is_some()
                && snapshot
                    .layouts
                    .get(&owner.project)
                    .is_some_and(|layout| owner.is_active_in(layout))
        })
    }

    pub fn tooltips(&self) -> &taide_native_ui::tooltips::Provider {
        &self.tooltips
    }

    pub fn toasts(&self) -> &taide_native_ui::toast::Toasts {
        &self.toasts
    }

    pub(crate) fn show_feedback(
        &mut self,
        context: &egui::Context,
        enabled: bool,
    ) -> Result<(), EditorBrowserError> {
        self.show_keybindings(context, enabled)?;
        self.tooltips.finish_frame(context);
        let theme = if enabled {
            self.theme_for_viewport(context.viewport_id())
        } else {
            self.workbench.presentation().theme()
        };
        let Some(theme_type) = theme.map(|theme| theme.theme_type) else {
            return Ok(());
        };
        let (Some(locale), Some(settings)) = (
            self.workbench.presentation().locale(),
            self.workbench.shell().state().settings(),
        ) else {
            return Ok(());
        };
        let now = taide_native_ui::toast::Instant::now();
        for feedback in std::mem::take(&mut self.feedback) {
            match feedback {
                Feedback::Ipc(error) => self.toasts.ipc_error(locale, &error, now),
                Feedback::Settings(error) => self.toasts.settings_failed(locale, &error, now),
                Feedback::Snippet(notice) => self.toasts.snippet(locale, notice, now),
                Feedback::AppFile(error) => {
                    self.toasts
                        .app_file_failed(locale, error.target, &error.error, now)
                }
            }
        }
        self.toasts
            .set_reduced_motion(self.workbench.reduced_motion());
        self.toasts
            .set_position(context, &settings.toast_position, now, enabled);
        self.toasts.tick(context, now, enabled);
        self.toasts
            .show(context, theme_type, &settings.toast_position, now, enabled)
            .map_err(EditorBrowserError::Presentation)
    }

    pub fn begin_settings_frame(&mut self) {
        self.settings_views.begin_frame();
    }

    pub fn finish_settings_frame(&mut self) {
        self.settings_views.finish_frame();
    }

    pub fn settings_views(&self) -> &taide_native_ui::settings_view::Views {
        &self.settings_views
    }

    pub fn show_settings(
        &mut self,
        ui: &mut Ui,
        owner: taide_native_ui::settings_owner::Owner,
    ) -> Result<taide_native_ui::settings_view::Output, EditorBrowserError> {
        if !self.settings_owner_active(&owner) {
            return Err(EditorBrowserError::Presentation(AppError::Forbidden(
                "remote Settings owner is no longer active".into(),
            )));
        }
        let presentation = self.workbench.presentation();
        let theme = self
            .theme_for_viewport(ui.ctx().viewport_id())
            .ok_or_else(|| {
                EditorBrowserError::Presentation(AppError::Internal(
                    "remote Settings theme is unavailable".into(),
                ))
            })?;
        let locale = presentation.locale().ok_or_else(|| {
            EditorBrowserError::Presentation(AppError::Internal(
                "remote Settings locale is unavailable".into(),
            ))
        })?;
        let settings = self.workbench.shell().state().settings().ok_or_else(|| {
            EditorBrowserError::Presentation(AppError::Internal(
                "remote Settings are unavailable".into(),
            ))
        })?;
        let appearance = taide_native_ui::settings_view::Appearance::new(theme)
            .map_err(EditorBrowserError::Presentation)?;
        let tooltip_appearance = taide_native_ui::tooltips::Appearance::new(theme)
            .map_err(EditorBrowserError::Presentation)?;
        let mut output = self
            .settings_views
            .show(ui, owner.clone(), settings, locale, &appearance);
        if let Some(error) = output.error.take() {
            self.feedback.push(Feedback::Settings(error));
        }
        if let Some(error) = output.theme_error.take() {
            self.feedback.push(Feedback::Ipc(error.clone()));
            self.theme_errors.push(error);
        }
        self.tooltips
            .show_triggers(&output.take_tooltips(), &tooltip_appearance);
        if let Some(request) = output.load.take() {
            self.workbench.request_settings_catalog(request);
            self.accept_settings_catalogs();
        }
        for request in std::mem::take(&mut output.resources) {
            self.workbench.request_settings_resource(request);
        }
        self.accept_settings_resources();
        for command in std::mem::take(&mut output.themes) {
            self.workbench.submit_theme(command);
        }
        for request in std::mem::take(&mut output.snippets) {
            if matches!(request.kind(), taide_native_ui::snippet_edit::Kind::List) {
                self.settings_views
                    .observe_snippet_list(request, taide_native_ui::toast::Instant::now());
            } else {
                self.workbench.submit_snippet(request);
            }
        }
        self.feedback.extend(
            std::mem::take(&mut output.snippet_notices)
                .into_iter()
                .map(Feedback::Snippet),
        );
        self.accept_snippet_replies();
        for change in std::mem::take(&mut output.changes) {
            if let Err(error) = self.workbench.change_preference(change) {
                self.feedback.push(Feedback::Settings(
                    crate::settings_catalog::invocation_error(error),
                ));
            }
        }
        for kind in std::mem::take(&mut output.folders) {
            self.workbench.open_settings_folder(kind);
        }
        if std::mem::take(&mut output.open_settings_file) {
            self.workbench.open_settings_file(&owner);
        }
        self.collect_app_file_feedback();
        self.feedback.extend(
            self.workbench
                .take_folder_errors()
                .into_iter()
                .map(Feedback::Ipc),
        );
        self.accept_theme_replies();
        if std::mem::take(&mut output.open_keybindings) {
            if let Err(error) = self.open_keybindings(ui.ctx()) {
                if let EditorBrowserError::Presentation(error) = error {
                    self.feedback.push(Feedback::Settings(error));
                }
            }
        }
        Ok(output)
    }

    fn accept_settings_catalogs(&mut self) {
        for finished in self.workbench.take_settings_catalogs() {
            self.settings_views
                .accept(&finished.request, Ok(finished.catalog));
        }
    }

    fn accept_settings_resources(&mut self) {
        for reply in self.workbench.take_settings_resources() {
            self.settings_views.accept_resource(reply);
        }
    }

    fn accept_theme_replies(&mut self) {
        for reply in self.workbench.take_theme_replies() {
            let mutation_error = match &reply {
                taide_native_ui::theme_edit::Reply::Loaded { .. } => None,
                _ => reply.error().cloned(),
            };
            if let Some(error) = self.settings_views.accept_theme(reply).or(mutation_error) {
                self.feedback.push(Feedback::Ipc(error.clone()));
                self.theme_errors.push(error);
            }
        }
    }

    fn accept_snippet_replies(&mut self) {
        for reply in self.workbench.take_snippet_replies() {
            self.settings_views.accept_snippet(reply);
        }
        for reply in self.workbench.take_snippet_catalogs() {
            self.settings_views
                .accept_snippet_catalog(reply, taide_native_ui::toast::Instant::now());
        }
        self.feedback.extend(
            self.settings_views
                .take_snippet_notices()
                .into_iter()
                .map(Feedback::Snippet),
        );
    }

    pub fn has_pending_snippet_mutations(&self) -> bool {
        self.workbench.has_pending_snippet_mutations()
    }

    pub fn snippet_failures(&self) -> &[crate::shell::Failure] {
        self.workbench.snippet_failures()
    }

    pub fn take_snippet_failures(&mut self) -> Vec<crate::shell::Failure> {
        self.workbench.take_snippet_failures()
    }

    pub fn take_theme_errors(&mut self) -> Vec<AppError> {
        let _ = self.workbench.take_theme_failures();
        std::mem::take(&mut self.theme_errors)
    }

    pub fn theme_failures(&self) -> &[crate::shell::Failure] {
        self.workbench.theme_failures()
    }

    pub fn has_pending_theme_mutations(&self) -> bool {
        self.workbench.has_pending_theme_mutations()
    }

    pub fn change_preference(
        &mut self,
        change: taide_native_ui::settings_controls::Change,
    ) -> Result<u32, InvokeError> {
        self.workbench.change_preference(change)
    }

    pub fn take_preference_results(&mut self) -> Vec<crate::preferences::Finished> {
        self.workbench.take_preference_results()
    }

    pub fn preference_results(&self) -> &[crate::preferences::Finished] {
        self.workbench.preference_results()
    }

    pub(crate) fn take_preference_close_failures(&mut self) -> Vec<crate::shell::Failure> {
        self.workbench.take_preference_close_failures()
    }

    pub fn has_pending_preferences(&self) -> bool {
        self.workbench.has_pending_preferences()
    }

    pub fn files(&self) -> &FileViews {
        &self.files
    }

    pub fn files_mut(&mut self) -> &mut FileViews {
        &mut self.files
    }

    pub fn file_events(&self) -> &[FileEvent] {
        &self.file_events
    }

    pub fn bind_project_file(
        &mut self,
        key: ViewKey,
        path: &str,
        project: &ProjectRef,
    ) -> Result<Option<ViewId>, EditorBrowserError> {
        if project.root.is_empty() || project.root.contains('\0') {
            return Err(EditorBrowserError::Editor(EditorError::InvalidIdentity));
        }
        let view = self
            .files
            .bind(key.clone(), path)
            .map_err(EditorBrowserError::Editor)?;
        if is_within_root(path, &project.root) {
            self.scopes.insert(
                key,
                ProjectScope {
                    project: project.id.clone(),
                    path: path.into(),
                    document: match self.files.state(path) {
                        FileState::Ready(document) => Some(document),
                        _ => None,
                    },
                },
            );
            self.mirrors.request(project.id.clone());
        } else {
            self.scopes.remove(&key);
        }
        if let Some(view) = view
            && let Some(owner) = self.files.store().views().get(view)
        {
            self.observe_dirty(owner.document);
        }
        Ok(view)
    }

    pub fn mirrors(&self) -> &MirrorState {
        &self.mirrors
    }

    pub fn retry_mirrors(&mut self, project: &ProjectId) {
        self.mirrors.refresh(project);
    }

    pub fn retry_mirror_restore(&mut self, key: &ViewKey) {
        if let Some(scope) = self.scopes.get(key) {
            self.files.retry_mirror_restore(&scope.path);
            self.mirrors.refresh(&scope.project);
        }
    }

    pub fn take_mirror_restore_failures(&mut self) -> Vec<(String, EditorError)> {
        std::mem::take(&mut self.mirror_restore_failures)
    }

    fn reconcile_mirror_scopes(&mut self) {
        let snapshot = self.workbench.snapshot();
        self.scopes.retain(|key, scope| {
            self.files.bound_path(key) == Some(scope.path.as_str())
                && snapshot.is_none_or(|snapshot| snapshot.project(&scope.project).is_some())
        });
        let active = self
            .scopes
            .values()
            .map(|scope| MirrorKey {
                project: scope.project.clone(),
                path: scope.path.clone(),
            })
            .collect::<BTreeSet<_>>();
        self.mirror_writes
            .retain_active(|key| active.contains(key), self.mirror_time);
        self.mirrors.retain(|project| {
            active.iter().any(|key| &key.project == project)
                || self.mirror_writes.has_project(project)
                || self
                    .pending_mirror_clears
                    .iter()
                    .any(|key| &key.project == project)
        });
    }

    fn update_mirror_clock(&mut self) {
        if let Some(runtime) = &self.mirror_runtime {
            match runtime.now() {
                Ok(now) => self.mirror_time = now,
                Err(error) => self.mirror_runtime_error = Some(error),
            }
        }
    }

    fn mirror_keys(&self, document: DocumentId) -> BTreeSet<MirrorKey> {
        let mut keys = self
            .mirror_writes
            .keys_for_document(document)
            .into_iter()
            .collect::<BTreeSet<_>>();
        keys.extend(
            self.scopes
                .values()
                .filter(|scope| scope.document == Some(document))
                .map(|scope| MirrorKey {
                    project: scope.project.clone(),
                    path: scope.path.clone(),
                }),
        );
        keys
    }

    fn observe_mirror_edit(&mut self, document: DocumentId) {
        let Ok(snapshot) = self.files.store().documents().snapshot(document) else {
            return;
        };
        if !snapshot.dirty {
            return;
        }
        for key in self.mirror_keys(document) {
            self.pending_mirror_clears.remove(&key);
            self.mirror_writes
                .observe(key, document, snapshot.revision, self.mirror_time);
        }
    }

    fn settle_mirrors(&mut self, document: DocumentId, dirty: bool) {
        for key in self.mirror_keys(document) {
            self.mirror_writes.settle(&key, dirty, self.mirror_time);
            self.mirrors.settle_path(&key.project, &key.path);
        }
    }

    fn queue_disk_mirror_clear(&mut self, key: MirrorKey) {
        let Some(mirrors) = self.mirrors.mirrors(&key.project) else {
            self.mirrors.request(key.project.clone());
            self.pending_mirror_clears.insert(key);
            return;
        };
        let expected = mirrors.iter().find(|entry| entry.path == key.path).cloned();
        self.mirror_writes
            .queue_clear(key.clone(), ClearExpected::Listed(expected));
        self.mirrors.settle_path(&key.project, &key.path);
        self.pending_mirror_clears.remove(&key);
    }

    fn resolve_mirror_clears(&mut self) {
        for key in self
            .pending_mirror_clears
            .iter()
            .cloned()
            .collect::<Vec<_>>()
        {
            if self.mirrors.mirrors(&key.project).is_some() {
                self.queue_disk_mirror_clear(key);
            }
        }
    }

    fn apply_mirror_outcomes(&mut self) {
        for outcome in self.mirror_writes.take_outcomes() {
            match outcome {
                MirrorOutcome::Committed { key, receipt } => {
                    self.mirrors.record_write(&key.project, receipt.entry)
                }
                MirrorOutcome::Invalidated(key) => self.mirrors.refresh(&key.project),
            }
        }
    }

    pub fn mirror_flush_status(&self) -> MirrorFlushStatus {
        if let Some(error) = &self.mirror_runtime_error {
            return MirrorFlushStatus::Failed(error.clone());
        }
        let status = self.mirror_writes.flush_status();
        if matches!(status, MirrorFlushStatus::Failed(_)) {
            return status;
        }
        for key in &self.pending_mirror_clears {
            if let Some(error) = self.mirrors.failure(&key.project) {
                return MirrorFlushStatus::Failed(MirrorError::Rpc(error.clone()));
            }
        }
        if !self.pending_mirror_clears.is_empty() {
            return MirrorFlushStatus::Pending;
        }
        status
    }

    pub fn mirror_timer_armed(&self) -> bool {
        self.mirror_runtime
            .as_ref()
            .is_some_and(MirrorRuntime::is_armed)
    }
    pub fn take_mirror_failures(&mut self) -> Vec<MirrorFailure> {
        self.mirror_writes.take_failures()
    }

    pub fn retry_mirror_writes(&mut self) {
        self.update_mirror_clock();
        if self
            .mirror_runtime
            .as_ref()
            .is_some_and(|runtime| runtime.now().is_ok())
        {
            self.mirror_runtime_error = None;
        }
        self.mirror_writes.retry(self.mirror_time);
        for key in &self.pending_mirror_clears {
            self.mirrors.refresh(&key.project);
        }
    }

    pub fn flush_mirrors(&mut self) {
        self.update_mirror_clock();
        self.collect_file_events();
        self.mirror_writes.flush(self.mirror_time);
        if self.workbench.is_connected() {
            self.flush_mirror_writes();
        }
        self.arm_mirror_timer();
    }

    pub fn unbind_file(&mut self, key: &ViewKey) -> Result<(), EditorBrowserError> {
        self.update_mirror_clock();
        self.collect_file_events();
        if let Some(scope) = self.scopes.get(key) {
            self.mirror_writes.flush_key(
                &MirrorKey {
                    project: scope.project.clone(),
                    path: scope.path.clone(),
                },
                self.mirror_time,
            );
        }
        self.files.unbind(key).map_err(EditorBrowserError::Editor)?;
        self.reconcile_mirror_scopes();
        if self.workbench.is_connected() {
            self.flush_mirror_writes();
        }
        self.arm_mirror_timer();
        Ok(())
    }

    fn flush_mirror_writes(&mut self) {
        if self.mirror_runtime_error.is_some() || self.mirror_runtime.is_none() {
            return;
        }
        for request in self.mirror_writes.next_clears() {
            if !self.workbench.is_connected() {
                break;
            }
            let call = request.call();
            match self.workbench.invoke(call.command, call.args) {
                Ok(seq) => self.mirror_writes.clear_sent(request, seq),
                Err(error) => self.mirror_writes.clear_failed(request, error),
            }
        }
        for key in self.mirror_writes.ready(self.mirror_time) {
            if !self.workbench.is_connected() {
                break;
            }
            let snapshot = self
                .mirror_writes
                .document(&key)
                .ok_or(EditorError::NotFound)
                .and_then(|document| self.files.store().documents().snapshot(document));
            let snapshot = match snapshot {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.mirror_writes.fail(key, MirrorError::Editor(error));
                    continue;
                }
            };
            if !snapshot.dirty {
                self.mirror_writes.settle(&key, false, self.mirror_time);
                continue;
            }
            let Some(request) =
                self.mirror_writes
                    .prepare(&key, snapshot.revision, snapshot.rope.to_string())
            else {
                continue;
            };
            let call = request.call();
            match self.workbench.invoke(call.command, call.args) {
                Ok(seq) => self.mirror_writes.sent(request, seq),
                Err(error) => self.mirror_writes.invocation_failed(request, error),
            }
        }
    }

    fn arm_mirror_timer(&mut self) {
        let deadline = if self.mirror_runtime_error.is_some() {
            None
        } else {
            self.mirror_writes.next_deadline()
        };
        if let Some(runtime) = &mut self.mirror_runtime
            && let Err(error) = runtime.arm(deadline)
        {
            self.mirror_runtime_error = Some(error);
        }
    }

    fn restore_mirrors(&mut self) {
        for scope in self.scopes.values() {
            if let Some(mirrors) = self.mirrors.mirrors(&scope.project) {
                let mirror = mirrors.iter().find(|mirror| mirror.path == scope.path);
                if let Err(error) = self.files.restore_mirror(&scope.path, mirror) {
                    self.mirror_restore_failures
                        .push((scope.path.clone(), error));
                }
            }
        }
    }

    pub fn take_file_events(&mut self) -> Vec<FileEvent> {
        std::mem::take(&mut self.file_events)
    }

    pub fn take_dirty_failures(&mut self) -> Vec<DirtyFailure> {
        self.dirty.take_failures()
    }

    pub fn retry_dirty(&mut self) {
        self.dirty.retry();
    }

    pub fn dirty_flush_state(&self) -> FlushState<'_> {
        self.dirty.flush_state()
    }

    pub fn request_disk_choice(
        &mut self,
        view: ViewId,
        choice: DiskChoice,
    ) -> Result<bool, EditorBrowserError> {
        let Some(request) = self
            .files
            .prepare_choice(view, choice)
            .map_err(EditorBrowserError::Editor)?
        else {
            return Ok(false);
        };
        self.queue_choice(request)
            .map_err(EditorBrowserError::Editor)
    }

    fn queue_choice(&mut self, request: ChoiceRequest) -> Result<bool, EditorError> {
        let document = request.document();
        if !self.files.queue_choice(request)? {
            return Ok(false);
        }
        self.observe_dirty(document);
        Ok(true)
    }

    fn observe_dirty(&mut self, document: DocumentId) {
        if let Ok(snapshot) = self.files.store().documents().snapshot(document) {
            for view in self.files.store().views().for_document(document) {
                self.dirty.observe(view.key.tab.clone(), snapshot.dirty);
            }
        }
    }

    fn collect_file_events(&mut self) {
        for event in self.files.take_events() {
            for scope in self.scopes.values_mut() {
                if let FileState::Ready(document) = self.files.state(&scope.path) {
                    scope.document = Some(document);
                }
            }
            match &event {
                FileEvent::Edited { document, .. } => self.observe_mirror_edit(*document),
                FileEvent::SaveFinished {
                    document,
                    result: Ok(dirty),
                    ..
                } => self.settle_mirrors(*document, *dirty),
                FileEvent::ChoiceFinished {
                    request,
                    result: Ok(_),
                    ..
                } if request.choice() == DiskChoice::ViewDisk => {
                    for key in self.mirror_keys(request.document()) {
                        self.mirror_writes.settle(&key, false, self.mirror_time);
                        self.queue_disk_mirror_clear(key);
                    }
                }
                FileEvent::ChoiceFinished {
                    request,
                    result: Ok(_),
                    ..
                } => {
                    for key in self.mirror_keys(request.document()) {
                        if let Some(document) = self.mirror_writes.document(&key) {
                            let dirty = self
                                .files
                                .store()
                                .documents()
                                .snapshot(document)
                                .is_ok_and(|snapshot| snapshot.dirty);
                            if !dirty {
                                self.mirror_writes.settle(&key, false, self.mirror_time);
                            }
                        }
                        self.mirror_writes.flush_key(&key, self.mirror_time);
                    }
                }
                _ => {}
            }
            match &event {
                FileEvent::Loaded { document, .. }
                | FileEvent::Restored { document, .. }
                | FileEvent::Edited { document, .. }
                | FileEvent::SaveFinished {
                    document,
                    result: Ok(_),
                    ..
                } => self.observe_dirty(*document),
                FileEvent::ChoiceFinished {
                    request,
                    result: Ok(_),
                    ..
                } => self.observe_dirty(request.document()),
                FileEvent::ChoiceRequested(request) => {
                    if let Err(error) = self.queue_choice(request.clone()) {
                        self.files
                            .choice_failed(request.clone(), FileFailure::Editor(error));
                    }
                }
                _ => {}
            }
            self.file_events.push(event);
        }
        let mut active = self.files.bound_tabs();
        active.extend(self.app_files.owners().map(|owner| owner.key.tab.clone()));
        self.dirty.retain(|tab| active.contains(tab));
    }

    fn flush_dirty_and_choice(&mut self) {
        for update in self.dirty.next_updates() {
            let call = update.call();
            match self.workbench.invoke(call.command, call.args) {
                Ok(seq) => self.dirty.sent(update, seq),
                Err(error) => self.dirty.invocation_failed(update, error),
            }
        }
        let Some(request) = self.files.waiting_choice().cloned() else {
            return;
        };
        match self.dirty.flush_state() {
            FlushState::Pending => {}
            FlushState::Failed(error) => self
                .files
                .choice_failed(request, FileFailure::Rpc(error.clone())),
            FlushState::Ready => {
                if let Err(EditorBrowserError::Editor(error)) =
                    self.choose_disk_after_dirty_flush(request.clone())
                {
                    self.files
                        .choice_failed(request, FileFailure::Editor(error));
                }
            }
        }
    }

    pub fn has_pending_app_files(&self) -> bool {
        self.app_files.has_pending_writes() || self.workbench.has_pending_app_file_opens()
    }

    pub(crate) fn take_app_file_close_failures(&mut self) -> Vec<crate::shell::Failure> {
        std::mem::take(&mut self.app_file_close_failures)
    }

    fn collect_app_file_feedback(&mut self) {
        let close_failure = |error: &AppError| {
            serde_json::to_value(error)
                .map(crate::shell::Failure::Remote)
                .unwrap_or(crate::shell::Failure::MalformedResponse)
        };
        for error in self.workbench.take_app_file_open_errors() {
            self.app_file_close_failures.push(close_failure(&error));
            self.feedback.push(Feedback::Ipc(error));
        }
        for error in self.app_files.take_errors() {
            if error.is_write {
                self.app_file_close_failures
                    .push(close_failure(&error.error));
                self.feedback.push(Feedback::AppFile(error));
                continue;
            }
            self.feedback.push(Feedback::Ipc(error.error));
        }
    }

    fn reconcile_app_files(&mut self) {
        self.app_files
            .settings_updated(self.workbench.shell().state().settings_generation());
        let Some(snapshot) = self.workbench.snapshot() else {
            return;
        };
        let stale = self
            .app_files
            .owners()
            .filter(|owner| !owner.exists_in(snapshot))
            .cloned()
            .collect::<Vec<_>>();
        for owner in stale {
            if let Err(error) = self.app_files.unbind(&owner, self.files.store_mut()) {
                self.feedback.push(Feedback::Ipc(error));
            }
        }
        let documents = self
            .app_files
            .owners()
            .filter_map(|owner| {
                self.files.store().documents().find(
                    &taide_native_editor::document::DocumentKey::AppFile(owner.target),
                )
            })
            .collect::<HashSet<_>>();
        for document in documents {
            self.observe_dirty(document);
        }
    }

    fn send_app_file_requests(&mut self) {
        for request in self.app_files.next_requests() {
            if !self.app_files.is_active(&request) {
                continue;
            }
            if !self
                .workbench
                .snapshot()
                .is_some_and(|snapshot| request.owner().exists_in(snapshot))
            {
                if let Err(error) = self
                    .app_files
                    .unbind(request.owner(), self.files.store_mut())
                {
                    self.feedback.push(Feedback::Ipc(error));
                }
                continue;
            }
            if !self.workbench.is_connected() {
                self.app_files.failed(request, InvokeError::Closed);
                continue;
            }
            let call = request.call();
            match self.workbench.invoke(call.command, call.args) {
                Ok(seq) => self.app_files.sent(request, seq),
                Err(error) => self.app_files.failed(request, error),
            }
        }
        self.collect_app_file_feedback();
    }

    pub fn show_app_file(
        &mut self,
        ui: &mut Ui,
        owner: crate::app_files::Owner,
        request_focus: bool,
    ) -> Result<Option<EditorOutput>, EditorBrowserError> {
        if !self
            .workbench
            .snapshot()
            .is_some_and(|snapshot| owner.exists_in(snapshot))
        {
            return Err(EditorBrowserError::Presentation(AppError::Forbidden(
                "remote app file owner is no longer available".into(),
            )));
        }
        let view = self
            .app_files
            .bind(owner.clone(), self.files.store_mut())
            .map_err(EditorBrowserError::Presentation)?;
        self.send_app_file_requests();
        let (Some(settings), Some(theme), Some(locale)) = (
            self.workbench.shell().state().settings(),
            self.theme_for_viewport(ui.ctx().viewport_id()),
            self.workbench.presentation().locale(),
        ) else {
            return Ok(None);
        };
        let mut editor = NativeEditor {
            appearance: taide_native_ui::presentation::editor_appearance(theme, settings)
                .map_err(EditorBrowserError::Presentation)?,
        };
        let locale = locale.clone();
        let Some(view) = view else {
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0,
                editor.appearance.background,
            );
            if self.app_files.error(&owner).is_some() {
                ui.centered_and_justified(|ui| {
                    ui.label(taide_native_ui::presentation::message(
                        &locale,
                        "editor.openFailed",
                        &[],
                    ));
                });
            }
            return Ok(None);
        };
        let document = self
            .files
            .store()
            .views()
            .get(view)
            .ok_or(EditorBrowserError::Editor(EditorError::NotFound))?
            .document;
        let metadata = self
            .files
            .store()
            .documents()
            .snapshot(document)
            .map_err(EditorBrowserError::Editor)?
            .metadata;
        editor = editor.with_indent(resolve(
            &metadata.editor_config,
            IndentOptions {
                tab_size: settings.editor_tab_size,
                insert_spaces: settings.editor_insert_spaces,
            },
        ));
        let output = editor
            .show(ui, self.files.store_mut(), view, request_focus)
            .map_err(EditorBrowserError::Editor)?;
        if output.changed {
            self.observe_dirty(document);
        }
        if output.save_requested {
            self.save_app_file(&owner)?;
        }
        Ok(Some(output))
    }

    pub fn save_app_file(
        &mut self,
        owner: &crate::app_files::Owner,
    ) -> Result<bool, EditorBrowserError> {
        if !self
            .workbench
            .snapshot()
            .is_some_and(|snapshot| owner.exists_in(snapshot))
        {
            return Err(EditorBrowserError::Presentation(AppError::Forbidden(
                "remote app file save owner is no longer available".into(),
            )));
        }
        let prepared = self
            .app_files
            .prepare_save(owner, self.files.store_mut())
            .map_err(EditorBrowserError::Presentation)?;
        self.send_app_file_requests();
        Ok(prepared)
    }

    pub fn show_file(
        &mut self,
        ui: &mut Ui,
        key: &ViewKey,
        request_focus: bool,
    ) -> Result<Option<EditorOutput>, EditorBrowserError> {
        let Some(settings) = self.workbench.shell().state().settings() else {
            return Ok(None);
        };
        let Some(theme) = self.theme_for_viewport(ui.ctx().viewport_id()) else {
            return Ok(None);
        };
        let appearance = taide_native_ui::presentation::editor_appearance(theme, settings);
        let banner = taide_native_ui::presentation::banner_appearance(theme)
            .map_err(EditorBrowserError::Presentation)?;
        let editor = NativeEditor {
            appearance: appearance.map_err(EditorBrowserError::Presentation)?,
        };
        let editor = match self.files.view(key) {
            Some(view) => {
                let document = self
                    .files
                    .store()
                    .views()
                    .get(view)
                    .ok_or(EditorBrowserError::Editor(EditorError::NotFound))?
                    .document;
                let snapshot = self
                    .files
                    .store()
                    .documents()
                    .snapshot(document)
                    .map_err(EditorBrowserError::Editor)?;
                editor.with_indent(resolve(
                    &snapshot.metadata.editor_config,
                    IndentOptions {
                        tab_size: settings.editor_tab_size,
                        insert_spaces: settings.editor_insert_spaces,
                    },
                ))
            }
            None => editor,
        };
        let presentation = self.workbench.presentation();
        let Some(locale) = presentation.locale() else {
            return Ok(None);
        };
        self.files
            .show_surface(
                ui,
                key,
                FileSurface {
                    editor: &editor,
                    locale,
                    banner: &banner,
                },
                request_focus,
            )
            .map(|output| output.editor)
            .map_err(EditorBrowserError::File)
    }

    pub fn choose_disk_after_dirty_flush(
        &mut self,
        request: ChoiceRequest,
    ) -> Result<Option<u32>, EditorBrowserError> {
        if !self
            .files
            .validate_choice(&request)
            .map_err(EditorBrowserError::Editor)?
        {
            return Ok(None);
        }
        let call = request.call();
        match self.workbench.invoke(call.command, call.args) {
            Ok(seq) => {
                self.files.choice_sent(request, seq);
                Ok(Some(seq))
            }
            Err(error) => {
                let failure = FileFailure::Rpc(crate::shell::Failure::Invocation(error));
                self.files.choice_failed(request, failure.clone());
                Err(EditorBrowserError::File(failure))
            }
        }
    }

    pub fn save_prepared(&mut self, view: ViewId) -> Result<Option<u32>, EditorBrowserError> {
        let settings = self
            .workbench
            .shell()
            .state()
            .settings()
            .ok_or(EditorBrowserError::Editor(EditorError::NotFound))?;
        let Some(request) = self
            .files
            .prepare_save_after_format(
                view,
                CleanupFlags {
                    trim_trailing_whitespace: settings.trim_trailing_whitespace_on_save,
                    insert_final_newline: settings.insert_final_newline_on_save,
                },
                false,
            )
            .map_err(EditorBrowserError::Editor)?
        else {
            return Ok(None);
        };
        let call = request.call();
        match self.workbench.invoke(call.command, call.args) {
            Ok(seq) => {
                self.files.save_sent(request, seq);
                Ok(Some(seq))
            }
            Err(error) => {
                self.files.save_failed(request, error.clone());
                Err(EditorBrowserError::File(FileFailure::Rpc(
                    crate::shell::Failure::Invocation(error),
                )))
            }
        }
    }

    pub fn refresh(&mut self) {
        self.workbench.refresh();
        self.app_files.refresh();
        self.files.refresh();
        self.mirrors.refresh_all();
        self.settings_views.invalidate_catalog();
        self.settings_views.refresh_snippets();
    }

    pub fn poll(&mut self) -> Vec<BrowserEvent> {
        self.update_mirror_clock();
        self.collect_file_events();
        self.reconcile_mirror_scopes();
        if self
            .mirror_runtime
            .as_ref()
            .is_some_and(MirrorRuntime::take_flush)
        {
            self.mirror_writes.flush(self.mirror_time);
        }
        let mut events = Vec::new();
        let mut was_disconnected = false;
        for event in self.workbench.poll() {
            match &event {
                BrowserEvent::Connected { .. } => {
                    self.app_files.refresh();
                    self.files.refresh();
                    self.mirrors.refresh_all();
                    self.settings_views.invalidate_catalog();
                    self.settings_views.refresh_resources();
                    self.settings_views
                        .reconnect_snippets(taide_native_ui::toast::Instant::now());
                }
                BrowserEvent::Disconnected { .. } | BrowserEvent::AuthenticationRequired => {
                    self.settings_views.disconnect_snippets();
                    self.app_files.disconnected();
                    self.files.disconnected();
                    self.dirty.disconnected();
                    self.mirrors.disconnected();
                    self.mirror_writes.disconnected();
                    was_disconnected = true;
                }
                BrowserEvent::Frame(Delivery::Response { seq, result }) => {
                    if self
                        .app_files
                        .response(*seq, result, self.files.store_mut())
                    {
                        continue;
                    }
                    let files = &self.files;
                    if self
                        .mirror_writes
                        .response(*seq, result, self.mirror_time, |document| {
                            files
                                .store()
                                .documents()
                                .snapshot(document)
                                .ok()
                                .map(|snapshot| snapshot.rope.to_string())
                        })
                    {
                        self.apply_mirror_outcomes();
                        continue;
                    }
                    if self.mirrors.response(*seq, result) || self.dirty.response(*seq, result) {
                        continue;
                    }
                    if self.files.response(*seq, result) {
                        self.collect_file_events();
                        continue;
                    }
                }
                BrowserEvent::Frame(Delivery::Event { event, payload }) => {
                    self.files.event(event, payload);
                    self.mirrors.event(event);
                    if event == "theme:changed" || event == "locale:changed" {
                        self.settings_views.invalidate_catalog();
                    }
                }
                _ => {}
            }
            events.push(event);
        }
        self.accept_settings_catalogs();
        self.accept_settings_resources();
        self.accept_theme_replies();
        self.accept_snippet_replies();
        self.reconcile_app_files();
        if self.workbench.is_connected()
            && let Some(request) = self.settings_views.next_snippet_read()
        {
            self.workbench.request_snippet_catalog(request);
            self.accept_snippet_replies();
        }
        self.collect_app_file_feedback();
        self.feedback.extend(
            self.workbench
                .take_preference_errors()
                .into_iter()
                .map(Feedback::Settings),
        );
        self.feedback.extend(
            self.workbench
                .take_folder_errors()
                .into_iter()
                .map(Feedback::Ipc),
        );
        self.reconcile_mirror_scopes();
        if let Some(snapshot) = self.workbench.snapshot() {
            self.mirrors.reconcile_layouts(&snapshot.layouts);
        }
        self.restore_mirrors();
        self.collect_file_events();
        self.resolve_mirror_clears();
        if was_disconnected {
            self.dirty.disconnected();
            self.mirror_writes.disconnected();
        }
        if self.workbench.is_connected() {
            self.send_app_file_requests();
            self.flush_dirty_and_choice();
            self.flush_mirror_writes();
            for project in self.mirrors.next_reads() {
                if !self.workbench.is_connected() {
                    self.mirrors.disconnected();
                    break;
                }
                let call = MirrorState::read_call(&project);
                match self.workbench.invoke(call.command, call.args) {
                    Ok(seq) => self.mirrors.sent(project, seq),
                    Err(error) => self.mirrors.invocation_failed(project, error),
                }
            }
            for path in self.files.next_reads() {
                if !self.workbench.is_connected() {
                    self.files.disconnected();
                    break;
                }
                let call = FileViews::read_call(&path);
                match self.workbench.invoke(call.command, call.args) {
                    Ok(seq) => self.files.read_sent(path, seq),
                    Err(error) => self.files.read_failed(path, error),
                }
            }
        }
        self.arm_mirror_timer();
        events
    }

    pub fn dispose(&mut self) {
        self.app_files.disconnected();
        self.settings_views.clear();
        self.collect_file_events();
        self.files.disconnected();
        self.dirty.disconnected();
        self.mirrors.disconnected();
        self.mirror_writes.disconnected();
        self.mirror_runtime = None;
        self.collect_file_events();
        self.mirror_writes.disconnected();
        self.workbench.dispose();
    }
}

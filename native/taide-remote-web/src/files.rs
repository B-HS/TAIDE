use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use egui::Ui;
use serde::Deserialize;
use serde_json::{Value, json};
use taide_model::file::{FsChange, MirrorEntry, OpenedFile};
use taide_model::ids::TabId;
use taide_model::locale::ResolvedLocale;
use taide_native_editor::document::{DiskChoice, DocumentId, DocumentKey, EditorError};
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::{EditorLimits, EditorStore, SaveSnapshot};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::conflict_banner::{
    BannerAction, BannerAppearance, BannerOutput, BannerVariant,
};
use taide_native_ui::editor_surface::{EditorOutput, NativeEditor};

use crate::shell::{Call, Failure};
use crate::{InvokeError, ResponsePayload};

#[derive(Debug, Clone, PartialEq)]
pub enum FileFailure {
    Rpc(Failure),
    Editor(EditorError),
}

pub enum FileState<'a> {
    Loading,
    Ready(DocumentId),
    Failed(&'a FileFailure),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileEvent {
    Restored {
        path: String,
        document: DocumentId,
    },
    ChoiceRequested(ChoiceRequest),
    ChoiceFinished {
        request: ChoiceRequest,
        result: Result<bool, FileFailure>,
    },
    Loaded {
        path: String,
        document: DocumentId,
    },
    Edited {
        document: DocumentId,
        view: ViewId,
    },
    SaveFinished {
        path: String,
        document: DocumentId,
        result: Result<bool, FileFailure>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceRequest {
    path: String,
    view: ViewId,
    document: DocumentId,
    revision: u64,
    choice: DiskChoice,
}

impl ChoiceRequest {
    pub fn call(&self) -> Call {
        FileViews::read_call(&self.path)
    }
    pub fn document(&self) -> DocumentId {
        self.document
    }
    pub fn choice(&self) -> DiskChoice {
        self.choice
    }
}

pub struct FileSurface<'a> {
    pub editor: &'a NativeEditor,
    pub locale: &'a ResolvedLocale,
    pub banner: &'a BannerAppearance,
}

pub struct FileSurfaceOutput {
    pub editor: Option<EditorOutput>,
    pub banner: Option<BannerOutput>,
}

pub struct SaveRequest {
    path: String,
    snapshot: SaveSnapshot,
}

impl SaveRequest {
    pub fn call(&self) -> Call {
        Call {
            command: "file_save",
            args: json!({"path": self.path, "content": self.snapshot.rope().to_string()}),
        }
    }
}

#[derive(Default)]
struct Entry {
    document: Option<DocumentId>,
    failure: Option<FileFailure>,
    restore_revision: Option<u64>,
    restore_finished: bool,
}

#[derive(Deserialize)]
struct FsEvent {
    change: FsChange,
}

pub struct FileViews {
    store: EditorStore,
    bindings: HashMap<ViewKey, String>,
    entries: BTreeMap<String, Entry>,
    dirty: BTreeSet<String>,
    reads: BTreeMap<u32, String>,
    saves: BTreeMap<u32, SaveRequest>,
    choices: BTreeMap<u32, ChoiceRequest>,
    waiting_choice: Option<ChoiceRequest>,
    notices: HashMap<ViewKey, BannerVariant>,
    events: Vec<FileEvent>,
}

impl FileViews {
    pub fn new(limits: EditorLimits) -> Result<Self, EditorError> {
        Ok(Self {
            store: EditorStore::new(limits)?,
            bindings: HashMap::new(),
            entries: BTreeMap::new(),
            dirty: BTreeSet::new(),
            reads: BTreeMap::new(),
            saves: BTreeMap::new(),
            choices: BTreeMap::new(),
            waiting_choice: None,
            notices: HashMap::new(),
            events: Vec::new(),
        })
    }

    pub fn store(&self) -> &EditorStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut EditorStore {
        &mut self.store
    }

    pub fn bind(&mut self, key: ViewKey, path: &str) -> Result<Option<ViewId>, EditorError> {
        if path.is_empty() || path.contains('\0') {
            return Err(EditorError::InvalidIdentity);
        }
        if self.bindings.get(&key).is_some_and(|old| old != path) {
            self.unbind(&key)?;
        }
        self.bindings.insert(key.clone(), path.into());
        if !self.entries.contains_key(path) {
            self.entries.insert(path.into(), Entry::default());
            self.dirty.insert(path.into());
        }
        match self.entries.get(path).and_then(|entry| entry.document) {
            Some(document) => self.store.attach_view(key, document).map(Some),
            None => Ok(None),
        }
    }

    pub fn unbind(&mut self, key: &ViewKey) -> Result<(), EditorError> {
        self.notices.remove(key);
        if let Some(view) = self.store.views().find(key) {
            self.store.detach_view(view)?;
        }
        let Some(path) = self.bindings.remove(key) else {
            return Ok(());
        };
        if self.bindings.values().any(|bound| bound == &path) {
            return Ok(());
        }
        self.dirty.remove(&path);
        self.reads.retain(|_, reading| reading != &path);
        self.release_unused();
        Ok(())
    }

    pub fn state(&self, path: &str) -> FileState<'_> {
        let Some(entry) = self.entries.get(path) else {
            return FileState::Loading;
        };
        if let Some(failure) = &entry.failure {
            return FileState::Failed(failure);
        }
        match entry.document {
            Some(document) => FileState::Ready(document),
            None => FileState::Loading,
        }
    }

    pub fn view(&self, key: &ViewKey) -> Option<ViewId> {
        self.store.views().find(key)
    }

    pub fn bound_tabs(&self) -> BTreeSet<TabId> {
        self.bindings.keys().map(|key| key.tab.clone()).collect()
    }

    pub fn bound_path(&self, key: &ViewKey) -> Option<&str> {
        self.bindings.get(key).map(String::as_str)
    }

    pub fn restore_mirror(
        &mut self,
        path: &str,
        mirror: Option<&MirrorEntry>,
    ) -> Result<bool, EditorError> {
        let Some(entry) = self.entries.get_mut(path) else {
            return Ok(false);
        };
        let Some(document) = entry.document else {
            return Ok(false);
        };
        if entry.restore_finished {
            return Ok(false);
        }
        if self
            .saves
            .values()
            .any(|saving| saving.snapshot.document() == document)
            || self
                .choices
                .values()
                .chain(self.waiting_choice.iter())
                .any(|choice| choice.document == document)
        {
            return Ok(false);
        }
        if mirror.is_some_and(|mirror| mirror.path != path) {
            return Err(EditorError::InvalidIdentity);
        }
        entry.restore_finished = true;
        let (Some(revision), Some(mirror)) = (entry.restore_revision, mirror) else {
            return Ok(false);
        };
        let restored =
            self.store
                .restore_file_draft_if_unchanged(document, revision, &mirror.content)?;
        if !restored {
            return Ok(false);
        }
        let notice = match mirror.conflict {
            true => BannerVariant::MirrorRestoredConflict,
            false => BannerVariant::MirrorRestored,
        };
        for (key, _) in self
            .bindings
            .iter()
            .filter(|(_, bound)| bound.as_str() == path)
        {
            self.notices.insert(key.clone(), notice);
        }
        self.events.push(FileEvent::Restored {
            path: path.into(),
            document,
        });
        Ok(true)
    }

    pub fn retry_mirror_restore(&mut self, path: &str) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.restore_finished = false;
        }
    }

    pub fn refresh(&mut self) {
        self.dirty.extend(self.bindings.values().cloned());
    }

    pub fn retry(&mut self, path: &str) {
        if self.bindings.values().any(|bound| bound == path) {
            self.dirty.insert(path.into());
        }
    }

    pub fn next_reads(&self) -> Vec<String> {
        self.dirty
            .iter()
            .filter(|path| {
                !self.reads.values().any(|pending| pending == *path)
                    && !self
                        .choices
                        .values()
                        .chain(self.waiting_choice.iter())
                        .any(|choice| {
                            self.entries
                                .get(*path)
                                .is_some_and(|entry| entry.document == Some(choice.document))
                        })
                    && !self.saves.values().any(|saving| {
                        self.entries
                            .get(*path)
                            .is_some_and(|entry| entry.document == Some(saving.snapshot.document()))
                    })
            })
            .cloned()
            .collect()
    }

    pub fn read_call(path: &str) -> Call {
        Call {
            command: "file_open",
            args: json!({"path": path}),
        }
    }

    pub fn read_sent(&mut self, path: String, seq: u32) {
        self.dirty.remove(&path);
        if let Some(entry) = self.entries.get_mut(&path) {
            entry.failure = None;
        }
        self.reads.insert(seq, path);
    }

    pub fn read_failed(&mut self, path: String, error: InvokeError) {
        self.dirty.remove(&path);
        if let Some(entry) = self.entries.get_mut(&path) {
            entry.failure = Some(FileFailure::Rpc(Failure::Invocation(error)));
        }
    }

    pub fn prepare_save(&mut self, view: ViewId) -> Result<Option<SaveRequest>, EditorError> {
        if self.has_pending_choice() {
            return Ok(None);
        }
        let owner = self.store.views().get(view).ok_or(EditorError::NotFound)?;
        if self
            .saves
            .values()
            .any(|saving| saving.snapshot.document() == owner.document)
        {
            return Ok(None);
        }
        let path = self
            .bindings
            .get(&owner.key)
            .ok_or(EditorError::InvalidIdentity)?
            .clone();
        let document = owner.document;
        Ok(Some(SaveRequest {
            path,
            snapshot: self.store.save_snapshot(document)?,
        }))
    }

    pub fn save_sent(&mut self, request: SaveRequest, seq: u32) {
        self.saves.insert(seq, request);
    }

    pub fn prepare_save_after_format(
        &mut self,
        view: ViewId,
        flags: CleanupFlags,
        auto_save: bool,
    ) -> Result<Option<SaveRequest>, EditorError> {
        let Some(request) = self.prepare_save(view)? else {
            return Ok(None);
        };
        let document = request.snapshot.document();
        let Some(prepared) = taide_native_editor::save_preparation::prepare(
            &mut self.store,
            request.snapshot,
            Some(view),
            flags,
            auto_save,
        )?
        else {
            return Ok(None);
        };
        if prepared.changed {
            self.events.push(FileEvent::Edited { document, view });
        }
        Ok(Some(SaveRequest {
            path: request.path,
            snapshot: prepared.snapshot,
        }))
    }

    pub fn save_failed(&mut self, request: SaveRequest, error: InvokeError) {
        self.events.push(FileEvent::SaveFinished {
            document: request.snapshot.document(),
            path: request.path,
            result: Err(FileFailure::Rpc(Failure::Invocation(error))),
        });
    }

    pub fn event(&mut self, name: &str, payload: &str) {
        match name {
            "fs:changed" => {
                if let Ok(event) = serde_json::from_str::<FsEvent>(payload) {
                    for path in event.change.paths {
                        self.retry(&path);
                    }
                }
            }
            "fs:rescan-required" => self.refresh(),
            _ => {}
        }
    }

    pub fn restore_notice(
        &mut self,
        key: ViewKey,
        notice: BannerVariant,
    ) -> Result<(), EditorError> {
        if !self.bindings.contains_key(&key) {
            return Err(EditorError::InvalidIdentity);
        }
        self.notices.insert(key, notice);
        Ok(())
    }

    pub fn prepare_choice(
        &self,
        view: ViewId,
        choice: DiskChoice,
    ) -> Result<Option<ChoiceRequest>, EditorError> {
        let owner = self.store.views().get(view).ok_or(EditorError::NotFound)?;
        if self.has_pending_choice()
            || self
                .saves
                .values()
                .any(|save| save.snapshot.document() == owner.document)
        {
            return Ok(None);
        }
        let snapshot = self.store.documents().snapshot(owner.document)?;
        Ok(Some(ChoiceRequest {
            path: self
                .bindings
                .get(&owner.key)
                .ok_or(EditorError::InvalidIdentity)?
                .clone(),
            view,
            document: owner.document,
            revision: snapshot.revision,
            choice,
        }))
    }

    pub fn validate_choice(&self, request: &ChoiceRequest) -> Result<bool, EditorError> {
        let owner = self
            .store
            .views()
            .get(request.view)
            .ok_or(EditorError::NotFound)?;
        if owner.document != request.document
            || self.bindings.get(&owner.key) != Some(&request.path)
        {
            return Err(EditorError::InvalidIdentity);
        }
        if self.store.documents().snapshot(request.document)?.revision != request.revision {
            return Err(EditorError::StaleRevision);
        }
        Ok(self.choices.is_empty()
            && self
                .waiting_choice
                .as_ref()
                .is_none_or(|waiting| waiting == request)
            && !self
                .saves
                .values()
                .any(|save| save.snapshot.document() == request.document))
    }

    pub fn choice_sent(&mut self, request: ChoiceRequest, seq: u32) {
        self.waiting_choice = None;
        let paths = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.document == Some(request.document))
            .map(|(path, _)| path.clone())
            .collect::<BTreeSet<_>>();
        self.reads.retain(|_, path| !paths.contains(path));
        self.dirty.retain(|path| !paths.contains(path));
        self.choices.insert(seq, request);
    }

    pub fn choice_failed(&mut self, request: ChoiceRequest, failure: FileFailure) {
        if self.waiting_choice.as_ref() == Some(&request) {
            self.waiting_choice = None;
        }
        self.events.push(FileEvent::ChoiceFinished {
            request,
            result: Err(failure),
        });
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        if let Some(request) = self.choices.remove(&seq) {
            let applied = self
                .validate_choice(&request)
                .map_err(FileFailure::Editor)
                .and_then(|available| {
                    if !available || self.dirty.contains(&request.path) {
                        return Err(FileFailure::Editor(EditorError::StaleRevision));
                    }
                    let file = serde_json::from_value::<OpenedFile>(json_payload(result)?)
                        .map_err(|_| FileFailure::Rpc(Failure::MalformedResponse))?;
                    let canonical = file.path.clone();
                    self.store
                        .choose_disk(
                            request.document,
                            request.revision,
                            Path::new(&canonical),
                            file,
                            request.choice,
                        )
                        .map_err(FileFailure::Editor)
                });
            if applied.is_ok() {
                self.clear_notices(request.document);
            }
            self.events.push(FileEvent::ChoiceFinished {
                request,
                result: applied,
            });
            self.release_unused();
            return true;
        }
        if let Some(request) = self.saves.remove(&seq) {
            let document = request.snapshot.document();
            let applied = json_payload(result)
                .and_then(|value| {
                    serde_json::from_value::<()>(value)
                        .map_err(|_| FileFailure::Rpc(Failure::MalformedResponse))
                })
                .and_then(|()| {
                    self.store
                        .mark_saved(request.snapshot, None)
                        .map(|clean| !clean)
                        .map_err(FileFailure::Editor)
                });
            if applied.is_ok() {
                self.clear_notices(document);
                for entry in self
                    .entries
                    .values_mut()
                    .filter(|entry| entry.document == Some(document))
                {
                    entry.restore_finished = true;
                }
                self.dirty.extend(
                    self.entries
                        .iter()
                        .filter(|(path, entry)| {
                            entry.document == Some(document)
                                && self.bindings.values().any(|bound| bound == *path)
                        })
                        .map(|(path, _)| path.clone()),
                );
            }
            self.events.push(FileEvent::SaveFinished {
                path: request.path,
                document,
                result: applied,
            });
            self.release_unused();
            return true;
        }
        let Some(path) = self.reads.remove(&seq) else {
            return false;
        };
        if self.dirty.contains(&path) || !self.bindings.values().any(|bound| bound == &path) {
            return true;
        }
        let applied = json_payload(result)
            .and_then(|value| {
                serde_json::from_value::<OpenedFile>(value)
                    .map_err(|_| FileFailure::Rpc(Failure::MalformedResponse))
            })
            .and_then(|file| self.apply_file(&path, file).map_err(FileFailure::Editor));
        if let Some(entry) = self.entries.get_mut(&path) {
            entry.failure = applied.err();
        }
        true
    }

    pub fn disconnected(&mut self) {
        self.reads.clear();
        self.dirty.clear();
        for (_, request) in std::mem::take(&mut self.saves) {
            self.save_failed(request, InvokeError::Closed);
        }
        for (_, request) in std::mem::take(&mut self.choices) {
            self.choice_failed(
                request,
                FileFailure::Rpc(Failure::Invocation(InvokeError::Closed)),
            );
        }
        if let Some(request) = self.waiting_choice.take() {
            self.choice_failed(
                request,
                FileFailure::Rpc(Failure::Invocation(InvokeError::Closed)),
            );
        }
        self.release_unused();
    }

    pub fn take_events(&mut self) -> Vec<FileEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn has_pending_operations(&self) -> bool {
        !self.saves.is_empty() || self.has_pending_choice()
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        editor: &NativeEditor,
        key: &ViewKey,
        request_focus: bool,
    ) -> Result<Option<EditorOutput>, FileFailure> {
        let path = self
            .bindings
            .get(key)
            .ok_or(FileFailure::Editor(EditorError::InvalidIdentity))?;
        match self.state(path) {
            FileState::Failed(error) => return Err(error.clone()),
            FileState::Loading => return Ok(None),
            FileState::Ready(_) => {}
        }
        let view = self
            .store
            .views()
            .find(key)
            .ok_or(FileFailure::Editor(EditorError::NotFound))?;
        let output = editor
            .show(ui, &mut self.store, view, request_focus)
            .map_err(FileFailure::Editor)?;
        if output.changed {
            let document = self
                .store
                .views()
                .get(view)
                .ok_or(FileFailure::Editor(EditorError::NotFound))?
                .document;
            self.events.push(FileEvent::Edited { document, view });
        }
        Ok(Some(output))
    }

    pub fn show_surface(
        &mut self,
        ui: &mut Ui,
        key: &ViewKey,
        surface: FileSurface<'_>,
        request_focus: bool,
    ) -> Result<FileSurfaceOutput, FileFailure> {
        ui.add_enabled_ui(!self.has_pending_choice(), |ui| {
            let path = self
                .bindings
                .get(key)
                .ok_or(FileFailure::Editor(EditorError::InvalidIdentity))?;
            match self.state(path) {
                FileState::Loading | FileState::Failed(_) => {
                    let failure = match self.state(path) {
                        FileState::Failed(error) => Some(error.clone()),
                        _ => None,
                    };
                    let rect = ui.available_rect_before_wrap();
                    ui.painter()
                        .rect_filled(rect, 0.0, surface.editor.appearance.background);
                    if let Some(error) = failure {
                        ui.allocate_ui_with_layout(
                            rect.size(),
                            egui::Layout::centered_and_justified(egui::Direction::TopDown),
                            |ui| {
                                ui.colored_label(
                                    surface.banner.error,
                                    taide_native_ui::presentation::message(
                                        surface.locale,
                                        "editor.openFailed",
                                        &[],
                                    ),
                                );
                            },
                        );
                        return Err(error);
                    }
                    ui.allocate_rect(rect, egui::Sense::hover());
                    return Ok(FileSurfaceOutput {
                        editor: None,
                        banner: None,
                    });
                }
                FileState::Ready(_) => {}
            }
            let view = self
                .store
                .views()
                .find(key)
                .ok_or(FileFailure::Editor(EditorError::NotFound))?;
            let document = self
                .store
                .views()
                .get(view)
                .ok_or(FileFailure::Editor(EditorError::NotFound))?
                .document;
            let conflict = self
                .store
                .has_disk_conflict(document)
                .map_err(FileFailure::Editor)?;
            let variant = if conflict {
                Some(BannerVariant::ChangedOnDisk)
            } else {
                self.notices.get(key).copied()
            };
            let banner = variant.map(|variant| {
                taide_native_ui::conflict_banner::show(ui, surface.locale, surface.banner, variant)
            });
            if let Some(action) = banner.as_ref().and_then(|banner| banner.action) {
                match action {
                    BannerAction::Dismiss => {
                        self.notices.remove(key);
                    }
                    BannerAction::KeepMine | BannerAction::ViewDisk => {
                        let choice = if action == BannerAction::KeepMine {
                            DiskChoice::KeepMine
                        } else {
                            DiskChoice::ViewDisk
                        };
                        if let Some(request) = self
                            .prepare_choice(view, choice)
                            .map_err(FileFailure::Editor)?
                        {
                            self.events.push(FileEvent::ChoiceRequested(request));
                        }
                    }
                }
            }
            let snapshot = self
                .store
                .documents()
                .snapshot(document)
                .map_err(FileFailure::Editor)?;
            if snapshot.metadata.read_only {
                let message = if snapshot.metadata.lossy {
                    "editor.readOnlyLossyEncoding"
                } else {
                    "editor.readOnlyLargeFile"
                };
                ui.colored_label(
                    surface.banner.warning,
                    taide_native_ui::presentation::message(surface.locale, message, &[]),
                );
            }
            Ok(FileSurfaceOutput {
                editor: self.show(ui, surface.editor, key, request_focus)?,
                banner,
            })
        })
        .inner
    }

    fn clear_notices(&mut self, document: DocumentId) {
        self.notices.retain(|key, _| {
            self.store
                .views()
                .find(key)
                .and_then(|view| self.store.views().get(view))
                .is_none_or(|view| view.document != document)
        });
    }

    pub fn queue_choice(&mut self, request: ChoiceRequest) -> Result<bool, EditorError> {
        if self.has_pending_choice() || !self.validate_choice(&request)? {
            return Ok(false);
        }
        self.waiting_choice = Some(request);
        Ok(true)
    }

    pub fn waiting_choice(&self) -> Option<&ChoiceRequest> {
        self.waiting_choice.as_ref()
    }

    fn has_pending_choice(&self) -> bool {
        self.waiting_choice.is_some() || !self.choices.is_empty()
    }

    fn apply_file(&mut self, path: &str, file: OpenedFile) -> Result<(), EditorError> {
        let fresh = self
            .entries
            .get(path)
            .is_some_and(|entry| entry.document.is_none())
            && self
                .store
                .documents()
                .find(&DocumentKey::File(file.path.clone().into()))
                .is_none();
        let document = match self.entries.get(path).and_then(|entry| entry.document) {
            Some(document) => {
                let canonical = file.path.clone();
                self.store
                    .observe_file(document, Path::new(&canonical), file)?;
                document
            }
            None => self.store.open_remote_file(file, None)?,
        };
        if let Some(entry) = self.entries.get_mut(path) {
            entry.document = Some(document);
            if fresh {
                entry.restore_revision = Some(self.store.documents().snapshot(document)?.revision);
            }
        }
        for (key, _) in self
            .bindings
            .iter()
            .filter(|(_, bound)| bound.as_str() == path)
        {
            self.store.attach_view(key.clone(), document)?;
        }
        self.events.push(FileEvent::Loaded {
            path: path.into(),
            document,
        });
        Ok(())
    }

    fn release_unused(&mut self) {
        let unused = self
            .entries
            .iter()
            .filter_map(|(path, entry)| {
                if self.bindings.values().any(|bound| bound == path) {
                    return None;
                }
                if let Some(document) = entry.document {
                    if self
                        .saves
                        .values()
                        .any(|saving| saving.snapshot.document() == document)
                    {
                        return None;
                    }
                    if self
                        .choices
                        .values()
                        .chain(self.waiting_choice.iter())
                        .any(|choice| choice.document == document)
                    {
                        return None;
                    }
                    if self.store.views().for_document(document).next().is_none()
                        && self
                            .store
                            .release_document(document)
                            .is_err_and(|error| error != EditorError::NotFound)
                    {
                        return None;
                    }
                }
                Some(path.clone())
            })
            .collect::<Vec<_>>();
        for path in unused {
            self.entries.remove(&path);
        }
    }
}

fn json_payload(result: &Result<ResponsePayload, Value>) -> Result<Value, FileFailure> {
    match result {
        Ok(ResponsePayload::Json(value)) => Ok(value.clone()),
        Ok(ResponsePayload::Binary(_)) => Err(FileFailure::Rpc(Failure::MalformedResponse)),
        Err(error) => Err(FileFailure::Rpc(Failure::Remote(error.clone()))),
    }
}

use std::collections::{HashMap, VecDeque};
use std::path::{Component, PathBuf};
use std::sync::Arc;

use ropey::Rope;
use taide_model::app::AppFileTarget;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::TabId;

use crate::display_map::DisplayMap;
use crate::document::{
    DiskChoice, DiskSnapshot, DocumentId, DocumentKey, DocumentMetadata, DocumentSnapshot, Edit,
    EditorError, UndoGroup, apply_edits, byte_to_char,
};
use crate::view::{
    Composition, EditRun, GoalColumns, ScrollPosition, SelectionSet, ViewId, ViewKey, ViewState,
    WrapAffinities,
};

#[derive(Debug, Clone, Copy)]
pub struct EditorLimits {
    pub max_documents: usize,
    pub max_views: usize,
    pub max_undo_groups: usize,
    pub max_document_bytes: usize,
}

pub struct Transaction {
    pub revision: u64,
    pub edits: Vec<Edit>,
    pub group: UndoGroup,
    pub origin: Option<ViewId>,
    pub selection_after: Option<SelectionSet>,
}

#[derive(Clone)]
pub struct SaveSnapshot {
    document: DocumentId,
    key: DocumentKey,
    revision: u64,
    rope: Rope,
}

impl SaveSnapshot {
    pub fn document(&self) -> DocumentId {
        self.document
    }
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

struct HistoryEntry {
    before: Rope,
    after: Rope,
    before_selections: HashMap<ViewId, SelectionSet>,
    after_selections: HashMap<ViewId, SelectionSet>,
    group: UndoGroup,
    origin: Option<ViewId>,
}

struct Document {
    id: DocumentId,
    key: DocumentKey,
    rope: Rope,
    baseline: Rope,
    revision: u64,
    saved_revision: u64,
    metadata: DocumentMetadata,
    undo: VecDeque<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    can_merge: bool,
    requires_save: bool,
    syntax_tokens: Option<crate::syntax::SyntaxSnapshot>,
    observed_disk: Option<DiskSnapshot>,
}

impl Document {
    fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            id: self.id,
            key: self.key.clone(),
            revision: self.revision,
            rope: self.rope.clone(),
            metadata: self.metadata.clone(),
            dirty: self.requires_save || self.rope != self.baseline,
        }
    }
}

#[derive(Default)]
pub struct DocumentStore {
    documents: HashMap<DocumentId, Document>,
    by_key: HashMap<DocumentKey, DocumentId>,
    next_id: u64,
    pending_disposals: Option<Vec<DocumentKey>>,
}

impl DocumentStore {
    fn record_disposal(&mut self, key: DocumentKey) {
        if let Some(pending) = &mut self.pending_disposals {
            pending.push(key);
        }
    }

    fn remove(&mut self, id: DocumentId) -> Option<Document> {
        let document = self.documents.remove(&id)?;
        self.record_disposal(document.key.clone());
        Some(document)
    }

    pub fn snapshot(&self, id: DocumentId) -> Result<DocumentSnapshot, EditorError> {
        Ok(self
            .documents
            .get(&id)
            .ok_or(EditorError::NotFound)?
            .snapshot())
    }

    pub fn find(&self, key: &DocumentKey) -> Option<DocumentId> {
        self.by_key.get(key).copied()
    }
    pub fn snapshots(&self) -> impl Iterator<Item = DocumentSnapshot> + '_ {
        self.documents.values().map(Document::snapshot)
    }
    pub fn len(&self) -> usize {
        self.documents.len()
    }
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

#[derive(Default)]
pub struct ViewStore {
    views: HashMap<ViewId, ViewState>,
    by_key: HashMap<ViewKey, ViewId>,
    next_id: u64,
}

impl ViewStore {
    pub fn get(&self, view: ViewId) -> Option<&ViewState> {
        self.views.get(&view)
    }
    pub fn find(&self, key: &ViewKey) -> Option<ViewId> {
        self.by_key.get(key).copied()
    }
    pub fn for_document(&self, document: DocumentId) -> impl Iterator<Item = &ViewState> {
        self.views
            .values()
            .filter(move |view| view.document == document)
    }
    pub fn len(&self) -> usize {
        self.views.len()
    }
    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }
}

pub struct EditorStore {
    documents: DocumentStore,
    views: ViewStore,
    limits: EditorLimits,
}

impl EditorStore {
    pub fn new(limits: EditorLimits) -> Result<Self, EditorError> {
        if limits.max_documents == 0
            || limits.max_views == 0
            || limits.max_undo_groups == 0
            || limits.max_document_bytes == 0
        {
            return Err(EditorError::Capacity);
        }
        Ok(Self {
            documents: DocumentStore::default(),
            views: ViewStore::default(),
            limits,
        })
    }

    pub fn documents(&self) -> &DocumentStore {
        &self.documents
    }
    pub fn views(&self) -> &ViewStore {
        &self.views
    }

    pub fn track_document_disposals(&mut self) {
        self.documents
            .pending_disposals
            .get_or_insert_with(Vec::new);
    }

    pub fn pending_document_disposals(&self) -> &[DocumentKey] {
        self.documents.pending_disposals.as_deref().unwrap_or(&[])
    }

    pub fn acknowledge_document_disposals(&mut self) {
        if let Some(pending) = &mut self.documents.pending_disposals {
            pending.clear();
        }
    }

    pub fn break_undo_group(&mut self, document: DocumentId) -> Result<(), EditorError> {
        self.documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?
            .can_merge = false;
        Ok(())
    }

    pub fn syntax(
        &self,
        document: DocumentId,
    ) -> Result<Option<&crate::syntax::SyntaxSnapshot>, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        Ok(owner.syntax_tokens.as_ref().filter(|syntax| {
            syntax.revision == owner.revision && syntax.language_id == owner.metadata.language_id
        }))
    }

    pub fn install_syntax(
        &mut self,
        document: DocumentId,
        syntax: crate::syntax::SyntaxSnapshot,
    ) -> Result<(), EditorError> {
        let snapshot = self.documents.snapshot(document)?;
        if syntax.revision != snapshot.revision {
            return Err(EditorError::StaleRevision);
        }
        if syntax.language_id != snapshot.metadata.language_id {
            return Err(EditorError::InvalidIdentity);
        }
        let limit = self
            .limits
            .max_document_bytes
            .checked_add(snapshot.rope.len_lines())
            .ok_or(EditorError::Capacity)?;
        let mut count = 0usize;
        let mut previous_line = None;
        for line in &syntax.lines {
            if line.line >= snapshot.rope.len_lines()
                || previous_line.is_some_and(|previous| previous >= line.line)
            {
                return Err(EditorError::InvalidBoundary);
            }
            previous_line = Some(line.line);
            let range = crate::editing::line_content_range(&snapshot, line.line);
            let mut previous_byte = None;
            for token in &line.tokens {
                count = count.checked_add(1).ok_or(EditorError::Capacity)?;
                if count > limit {
                    return Err(EditorError::Capacity);
                }
                if token.start_byte > range.len()
                    || previous_byte.is_some_and(|previous| previous >= token.start_byte)
                    || previous_byte.is_none() && token.start_byte != 0
                {
                    return Err(EditorError::InvalidBoundary);
                }
                byte_to_char(&snapshot.rope, range.start + token.start_byte)?;
                previous_byte = Some(token.start_byte);
            }
        }
        self.documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?
            .syntax_tokens = Some(syntax);
        Ok(())
    }

    pub fn open_file(
        &mut self,
        canonical_path: PathBuf,
        file: OpenedFile,
    ) -> Result<DocumentId, EditorError> {
        self.open_file_with_draft(canonical_path, file, None)
    }

    pub fn open_file_with_draft(
        &mut self,
        canonical_path: PathBuf,
        file: OpenedFile,
        draft: Option<&str>,
    ) -> Result<DocumentId, EditorError> {
        if !canonical_path.is_absolute()
            || canonical_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(EditorError::InvalidIdentity);
        }
        self.admit_file_document(DocumentKey::File(canonical_path), file, draft)
    }

    pub fn open_remote_file(
        &mut self,
        file: OpenedFile,
        draft: Option<&str>,
    ) -> Result<DocumentId, EditorError> {
        if file.path.is_empty() || file.path.contains('\0') {
            return Err(EditorError::InvalidIdentity);
        }
        let key = DocumentKey::File(PathBuf::from(&file.path));
        self.admit_file_document(key, file, draft)
    }

    pub fn restore_file_draft_if_unchanged(
        &mut self,
        document: DocumentId,
        expected_revision: u64,
        draft: &str,
    ) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if !matches!(owner.key, DocumentKey::File(_)) {
            return Err(EditorError::InvalidIdentity);
        }
        if owner.revision != expected_revision
            || owner.requires_save
            || owner.rope != owner.baseline
        {
            return Ok(false);
        }
        if draft.len() > self.limits.max_document_bytes {
            return Err(EditorError::Capacity);
        }
        let revision = owner
            .revision
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let rope = Rope::from_str(draft);
        for view in self
            .views
            .views
            .values_mut()
            .filter(|view| view.document == document)
        {
            view.selection = view.selection.clamped(&rope);
            view.composition = None;
            view.folds.clear();
        }
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        owner.rope = rope;
        owner.revision = revision;
        owner.requires_save = true;
        owner.syntax_tokens = None;
        owner.can_merge = false;
        owner.undo.clear();
        owner.redo.clear();
        Ok(true)
    }

    fn admit_file_document(
        &mut self,
        key: DocumentKey,
        file: OpenedFile,
        draft: Option<&str>,
    ) -> Result<DocumentId, EditorError> {
        let metadata = DocumentMetadata::from_opened(&file);
        let existed = self.documents.by_key.contains_key(&key);
        if !existed && draft.is_some_and(|draft| draft.len() > self.limits.max_document_bytes) {
            return Err(EditorError::Capacity);
        }
        let document = self.admit(key, &file.content, metadata)?;
        if !existed && let Some(draft) = draft {
            let owner = self
                .documents
                .documents
                .get_mut(&document)
                .ok_or(EditorError::NotFound)?;
            owner.rope = Rope::from_str(draft);
            owner.requires_save = true;
        }
        Ok(document)
    }

    pub fn open_untitled(
        &mut self,
        tab: TabId,
        content: &str,
        language_id: String,
    ) -> Result<DocumentId, EditorError> {
        let metadata = DocumentMetadata {
            language_id,
            tier: FileSizeTier::Normal,
            read_only: false,
            lossy: false,
            editor_config: EditorConfigOptions::default(),
            disk_modified_ms: None,
            line_ending: crate::document::LineEnding::Lf,
        };
        self.admit(DocumentKey::Untitled(tab), content, metadata)
    }

    pub fn open_app_file(
        &mut self,
        target: AppFileTarget,
        content: &str,
    ) -> Result<DocumentId, EditorError> {
        self.admit(
            DocumentKey::AppFile(target),
            content,
            DocumentMetadata {
                language_id: "json".into(),
                tier: FileSizeTier::Normal,
                read_only: false,
                lossy: false,
                editor_config: EditorConfigOptions::default(),
                disk_modified_ms: None,
                line_ending: crate::document::LineEnding::from_content(content),
            },
        )
    }

    pub fn refresh_app_file(
        &mut self,
        document: DocumentId,
        target: AppFileTarget,
        content: &str,
    ) -> Result<bool, EditorError> {
        let current = self.documents.snapshot(document)?;
        if current.key != DocumentKey::AppFile(target) {
            return Err(EditorError::InvalidIdentity);
        }
        if current.dirty {
            return Ok(false);
        }
        let snapshot = self.save_snapshot(document)?;
        self.mark_app_file_saved(snapshot, target, content)
    }

    pub fn mark_app_file_saved(
        &mut self,
        snapshot: SaveSnapshot,
        target: AppFileTarget,
        canonical_content: &str,
    ) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&snapshot.document)
            .ok_or(EditorError::NotFound)?;
        if snapshot.key != DocumentKey::AppFile(target) || owner.key != snapshot.key {
            return Err(EditorError::InvalidIdentity);
        }
        if snapshot.revision < owner.saved_revision || snapshot.revision > owner.revision {
            return Err(EditorError::StaleSave);
        }
        if canonical_content.len() > self.limits.max_document_bytes {
            return Err(EditorError::Capacity);
        }
        let is_unchanged = owner.rope == snapshot.rope;
        let canonical = Rope::from_str(canonical_content);
        let replace = is_unchanged && owner.rope != canonical;
        let revision = if replace {
            owner
                .revision
                .checked_add(1)
                .ok_or(EditorError::RevisionOverflow)?
        } else {
            owner.revision
        };
        if replace {
            for view in self
                .views
                .views
                .values_mut()
                .filter(|view| view.document == snapshot.document)
            {
                view.selection = view.selection.clamped(&canonical);
                view.composition = None;
                view.folds.clear();
            }
        }
        let owner = self
            .documents
            .documents
            .get_mut(&snapshot.document)
            .ok_or(EditorError::NotFound)?;
        owner.baseline = canonical.clone();
        owner.saved_revision = snapshot.revision;
        owner.requires_save = !is_unchanged;
        owner.can_merge = false;
        if replace {
            owner.rope = canonical;
            owner.revision = revision;
            owner.saved_revision = revision;
            owner.metadata.line_ending =
                crate::document::LineEnding::from_content(canonical_content);
            owner.syntax_tokens = None;
            owner.undo.clear();
            owner.redo.clear();
        }
        Ok(is_unchanged)
    }

    pub fn restore_untitled(
        &mut self,
        tab: TabId,
        draft: Option<&str>,
    ) -> Result<DocumentId, EditorError> {
        let existed = self
            .documents
            .find(&DocumentKey::Untitled(tab.clone()))
            .is_some();
        if !existed && draft.is_some_and(|draft| draft.len() > self.limits.max_document_bytes) {
            return Err(EditorError::Capacity);
        }
        let document = self.open_untitled(tab, "", "plaintext".into())?;
        if !existed && let Some(draft) = draft {
            let owner = self
                .documents
                .documents
                .get_mut(&document)
                .ok_or(EditorError::NotFound)?;
            owner.rope = Rope::from_str(draft);
            owner.requires_save = true;
        }
        Ok(document)
    }

    pub fn convert_untitled_save(
        &mut self,
        snapshot: SaveSnapshot,
        canonical_path: PathBuf,
        file: OpenedFile,
        file_tab: TabId,
    ) -> Result<Option<DocumentId>, EditorError> {
        let DocumentKey::Untitled(source_tab) = &snapshot.key else {
            return Err(EditorError::InvalidIdentity);
        };
        if !canonical_path.is_absolute()
            || canonical_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(EditorError::InvalidIdentity);
        }
        let source = self
            .documents
            .documents
            .get(&snapshot.document)
            .ok_or(EditorError::NotFound)?;
        if source.key != snapshot.key {
            return Err(EditorError::InvalidIdentity);
        }
        if snapshot.revision < source.saved_revision || snapshot.revision > source.revision {
            return Err(EditorError::StaleSave);
        }
        if !snapshot.rope.chars().eq(file.content.chars()) {
            return Err(EditorError::StaleSave);
        }
        if file.tier == FileSizeTier::Refused {
            return Err(EditorError::Refused);
        }
        if file.content.len() > self.limits.max_document_bytes {
            return Err(EditorError::Capacity);
        }
        let key = DocumentKey::File(canonical_path);
        let previous = self.documents.find(&key);
        if let Some(previous) = previous {
            let target = self
                .documents
                .documents
                .get(&previous)
                .ok_or(EditorError::NotFound)?;
            if target.requires_save || target.rope != target.baseline {
                return Err(EditorError::UnsavedChanges);
            }
        }
        let mut views = self.views.views.clone();
        let mut by_key = self.views.by_key.clone();
        let mut removed = Vec::new();
        for view in views
            .values_mut()
            .filter(|view| view.document == snapshot.document || Some(view.document) == previous)
        {
            if &view.key.tab == source_tab && view.document == snapshot.document {
                by_key.remove(&view.key);
                view.key.tab = file_tab.clone();
                if by_key.get(&view.key).is_some_and(|other| *other != view.id) {
                    removed.push(view.id);
                    continue;
                }
                by_key.insert(view.key.clone(), view.id);
            }
            view.document = snapshot.document;
            view.selection = view.selection.clamped(&source.rope);
            view.composition = None;
            view.folds.clear();
        }
        for view in removed {
            views.remove(&view);
        }
        if let Some(previous) = previous {
            self.documents.remove(previous);
        }
        self.documents.record_disposal(snapshot.key.clone());
        self.documents.by_key.remove(&snapshot.key);
        self.documents.by_key.insert(key.clone(), snapshot.document);
        let source = self
            .documents
            .documents
            .get_mut(&snapshot.document)
            .ok_or(EditorError::NotFound)?;
        source.key = key;
        source.baseline = snapshot.rope;
        source.saved_revision = snapshot.revision;
        source.metadata = DocumentMetadata::from_opened(&file);
        source.observed_disk = Some(DiskSnapshot {
            rope: source.baseline.clone(),
            metadata: source.metadata.clone(),
        });
        source.can_merge = false;
        source.requires_save = false;
        self.views.views = views;
        self.views.by_key = by_key;
        Ok(previous)
    }

    pub fn retarget_file(
        &mut self,
        requested: &DocumentSnapshot,
        canonical_path: PathBuf,
        mut metadata: DocumentMetadata,
    ) -> Result<Option<DocumentId>, EditorError> {
        if !matches!(&requested.key, DocumentKey::File(_))
            || !canonical_path.is_absolute()
            || canonical_path
                .components()
                .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(EditorError::InvalidIdentity);
        }
        let source = self
            .documents
            .documents
            .get(&requested.id)
            .ok_or(EditorError::NotFound)?;
        if source.key != requested.key {
            return Err(EditorError::InvalidIdentity);
        }
        if source.revision != requested.revision {
            return Err(EditorError::StaleRevision);
        }
        let revision = source
            .revision
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let key = DocumentKey::File(canonical_path);
        let displaced = self.documents.find(&key).filter(|id| *id != requested.id);
        metadata.line_ending = source.metadata.line_ending;
        let rope = source.rope.clone();
        if let Some(displaced) = displaced {
            self.documents.remove(displaced);
        }
        if key != requested.key {
            self.documents.record_disposal(requested.key.clone());
        }
        self.documents.by_key.remove(&requested.key);
        self.documents.by_key.insert(key.clone(), requested.id);
        let source = self
            .documents
            .documents
            .get_mut(&requested.id)
            .ok_or(EditorError::NotFound)?;
        source.key = key;
        source.revision = revision;
        source.saved_revision = revision;
        source.metadata = metadata;
        if let Some(disk) = &mut source.observed_disk {
            disk.metadata = source.metadata.clone();
        }
        source.syntax_tokens = None;
        source.undo.clear();
        source.redo.clear();
        source.can_merge = false;
        for view in self
            .views
            .views
            .values_mut()
            .filter(|view| view.document == requested.id || Some(view.document) == displaced)
        {
            if Some(view.document) == displaced {
                view.folds.clear();
            }
            view.document = requested.id;
            view.selection = view.selection.clamped(&rope);
            view.composition = None;
        }
        Ok(displaced)
    }

    pub fn refresh_clean_file(
        &mut self,
        document: DocumentId,
        canonical_path: &std::path::Path,
        file: OpenedFile,
    ) -> Result<(), EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.key != DocumentKey::File(canonical_path.to_owned()) {
            return Err(EditorError::InvalidIdentity);
        }
        if owner.requires_save || owner.rope != owner.baseline {
            return Err(EditorError::UnsavedChanges);
        }
        if file.tier == FileSizeTier::Refused {
            return Err(EditorError::Refused);
        }
        if file.content.len() > self.limits.max_document_bytes {
            return Err(EditorError::Capacity);
        }
        let revision = owner
            .revision
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let metadata = DocumentMetadata::from_opened(&file);
        let rope = Rope::from_str(&file.content);
        for view in self
            .views
            .views
            .values_mut()
            .filter(|view| view.document == document)
        {
            view.selection = view.selection.clamped(&rope);
            view.composition = None;
            view.folds.clear();
        }
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        owner.rope = rope.clone();
        owner.baseline = rope;
        owner.metadata = metadata;
        owner.observed_disk = Some(DiskSnapshot {
            rope: owner.baseline.clone(),
            metadata: owner.metadata.clone(),
        });
        owner.revision = revision;
        owner.syntax_tokens = None;
        owner.saved_revision = revision;
        owner.undo.clear();
        owner.redo.clear();
        owner.can_merge = false;
        owner.requires_save = false;
        Ok(())
    }

    pub fn has_disk_conflict(&self, document: DocumentId) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        Ok(owner.snapshot().dirty
            && owner
                .observed_disk
                .as_ref()
                .is_some_and(|disk| disk.rope != owner.baseline))
    }

    pub fn observe_file(
        &mut self,
        document: DocumentId,
        canonical: &std::path::Path,
        file: OpenedFile,
    ) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.key != DocumentKey::File(canonical.to_owned()) {
            return Err(EditorError::InvalidIdentity);
        }
        if file.tier == FileSizeTier::Refused {
            return Err(EditorError::Refused);
        }
        if file.content.len() > self.limits.max_document_bytes {
            return Err(EditorError::Capacity);
        }
        if !owner.snapshot().dirty {
            if owner.rope.chars().eq(file.content.chars()) {
                let owner = self
                    .documents
                    .documents
                    .get_mut(&document)
                    .ok_or(EditorError::NotFound)?;
                let line_ending = owner.metadata.line_ending;
                owner.metadata = DocumentMetadata::from_opened(&file);
                owner.metadata.line_ending = line_ending;
                owner.observed_disk = Some(DiskSnapshot {
                    rope: owner.rope.clone(),
                    metadata: owner.metadata.clone(),
                });
                return Ok(false);
            }
            self.refresh_clean_file(document, canonical, file)?;
            return Ok(false);
        }
        let disk = DiskSnapshot {
            rope: Rope::from_str(&file.content),
            metadata: DocumentMetadata::from_opened(&file),
        };
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        let line_ending = owner.metadata.line_ending;
        owner.metadata = disk.metadata.clone();
        owner.metadata.line_ending = line_ending;
        owner.observed_disk = Some(disk);
        self.has_disk_conflict(document)
    }

    pub fn choose_disk(
        &mut self,
        document: DocumentId,
        expected_revision: u64,
        canonical: &std::path::Path,
        file: OpenedFile,
        choice: DiskChoice,
    ) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.key != DocumentKey::File(canonical.to_owned()) {
            return Err(EditorError::InvalidIdentity);
        }
        if owner.revision != expected_revision {
            return Err(EditorError::StaleRevision);
        }
        if file.tier == FileSizeTier::Refused {
            return Err(EditorError::Refused);
        }
        if file.content.len() > self.limits.max_document_bytes {
            return Err(EditorError::Capacity);
        }
        let dirty = owner.snapshot().dirty;
        let revision = match choice {
            DiskChoice::ViewDisk => owner
                .revision
                .checked_add(1)
                .ok_or(EditorError::RevisionOverflow)?,
            DiskChoice::KeepMine => owner.revision,
        };
        let disk = DiskSnapshot {
            rope: Rope::from_str(&file.content),
            metadata: DocumentMetadata::from_opened(&file),
        };
        if choice == DiskChoice::ViewDisk {
            for view in self
                .views
                .views
                .values_mut()
                .filter(|view| view.document == document)
            {
                view.selection = view.selection.clamped(&disk.rope);
                view.composition = None;
                view.folds.clear();
            }
        }
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        owner.baseline = disk.rope.clone();
        let line_ending = owner.metadata.line_ending;
        owner.metadata = disk.metadata.clone();
        owner.observed_disk = Some(disk.clone());
        owner.can_merge = false;
        if choice == DiskChoice::ViewDisk {
            owner.rope = disk.rope;
            owner.revision = revision;
            owner.syntax_tokens = None;
            owner.saved_revision = revision;
            owner.requires_save = false;
            owner.undo.clear();
            owner.redo.clear();
        } else {
            owner.metadata.line_ending = line_ending;
            owner.requires_save = dirty;
        }
        Ok(owner.snapshot().dirty)
    }

    fn admit(
        &mut self,
        key: DocumentKey,
        content: &str,
        metadata: DocumentMetadata,
    ) -> Result<DocumentId, EditorError> {
        if metadata.tier == FileSizeTier::Refused {
            return Err(EditorError::Refused);
        }
        if let Some(id) = self.documents.by_key.get(&key) {
            return Ok(*id);
        }
        if self.documents.len() >= self.limits.max_documents
            || content.len() > self.limits.max_document_bytes
        {
            return Err(EditorError::Capacity);
        }
        let next_id = self
            .documents
            .next_id
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let id = DocumentId(next_id);
        let rope = Rope::from_str(content);
        let baseline = match &key {
            DocumentKey::File(_) | DocumentKey::AppFile(_) => rope.clone(),
            DocumentKey::Untitled(_) => Rope::new(),
        };
        let observed_disk = matches!(&key, DocumentKey::File(_)).then(|| DiskSnapshot {
            rope: rope.clone(),
            metadata: metadata.clone(),
        });
        self.documents.documents.insert(
            id,
            Document {
                id,
                key: key.clone(),
                baseline,
                rope,
                revision: 0,
                saved_revision: 0,
                metadata,
                undo: VecDeque::new(),
                redo: Vec::new(),
                can_merge: false,
                requires_save: false,
                syntax_tokens: None,
                observed_disk,
            },
        );
        self.documents.by_key.insert(key, id);
        self.documents.next_id = next_id;
        Ok(id)
    }

    pub fn attach_view(
        &mut self,
        key: ViewKey,
        document: DocumentId,
    ) -> Result<ViewId, EditorError> {
        if let Some(view) = self.views.by_key.get(&key) {
            if self.views.views[view].document != document {
                return Err(EditorError::InvalidIdentity);
            }
            return Ok(*view);
        }
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        if self.views.len() >= self.limits.max_views {
            return Err(EditorError::Capacity);
        }
        let next_id = self
            .views
            .next_id
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let id = ViewId(next_id);
        self.views.views.insert(
            id,
            ViewState {
                id,
                key: key.clone(),
                document,
                selection: SelectionSet::default(),
                scroll: ScrollPosition::default(),
                folds: Vec::new(),
                composition: None,
                edit_run: None,
                goal_columns: None,
                wrap_affinities: None,
                display: None,
            },
        );
        self.views.by_key.insert(key, id);
        self.views.next_id = next_id;
        owner.can_merge = false;
        Ok(id)
    }

    pub fn set_view_state(
        &mut self,
        view: ViewId,
        selection: SelectionSet,
        scroll: ScrollPosition,
        folds: Vec<std::ops::Range<usize>>,
    ) -> Result<(), EditorError> {
        let current = self.views.views.get(&view).ok_or(EditorError::NotFound)?;
        let document = self
            .documents
            .documents
            .get(&current.document)
            .ok_or(EditorError::NotFound)?;
        selection.validate(&document.rope)?;
        if !scroll.x.is_finite() || !scroll.y.is_finite() || scroll.x < 0.0 || scroll.y < 0.0 {
            return Err(EditorError::InvalidBoundary);
        }
        for fold in &folds {
            if fold.start >= fold.end {
                return Err(EditorError::InvalidBoundary);
            }
            byte_to_char(&document.rope, fold.start)?;
            byte_to_char(&document.rope, fold.end)?;
        }
        let current = self
            .views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?;
        if current.selection != selection {
            current.goal_columns = None;
            current.wrap_affinities = None;
        }
        current.selection = selection;
        current.scroll = scroll;
        current.folds = folds;
        Ok(())
    }

    pub fn set_edit_run(&mut self, view: ViewId, run: Option<EditRun>) -> Result<(), EditorError> {
        self.views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?
            .edit_run = run;
        Ok(())
    }

    pub fn set_goal_columns(
        &mut self,
        view: ViewId,
        goal: Option<GoalColumns>,
    ) -> Result<(), EditorError> {
        let current = self
            .views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?;
        if goal.as_ref().is_some_and(|goal| {
            goal.leftover_visible_columns.len() != current.selection.selections.len()
        }) {
            return Err(EditorError::InvalidBoundary);
        }
        current.goal_columns = goal;
        Ok(())
    }

    pub fn set_wrap_affinities(
        &mut self,
        view: ViewId,
        affinities: Option<WrapAffinities>,
    ) -> Result<(), EditorError> {
        let current = self
            .views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?;
        if affinities.as_ref().is_some_and(|affinities| {
            affinities.heads_at_row_end.len() != current.selection.selections.len()
        }) {
            return Err(EditorError::InvalidBoundary);
        }
        current.wrap_affinities = affinities;
        Ok(())
    }

    pub fn take_display(&mut self, view: ViewId) -> Result<Option<Arc<DisplayMap>>, EditorError> {
        Ok(self
            .views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?
            .display
            .take())
    }

    pub fn set_display(
        &mut self,
        view: ViewId,
        display: Option<Arc<DisplayMap>>,
    ) -> Result<(), EditorError> {
        self.views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?
            .display = display;
        Ok(())
    }

    pub fn set_composition(
        &mut self,
        view: ViewId,
        composition: Option<Composition>,
    ) -> Result<(), EditorError> {
        let current = self.views.views.get(&view).ok_or(EditorError::NotFound)?;
        let document = self
            .documents
            .documents
            .get(&current.document)
            .ok_or(EditorError::NotFound)?;
        if let Some(composition) = &composition {
            if document.metadata.read_only {
                return Err(EditorError::ReadOnly);
            }
            if composition.revision != document.revision {
                return Err(EditorError::StaleRevision);
            }
            if composition.replace.start > composition.replace.end {
                return Err(EditorError::InvalidBoundary);
            }
            byte_to_char(&document.rope, composition.replace.start)?;
            byte_to_char(&document.rope, composition.replace.end)?;
        }
        self.views
            .views
            .get_mut(&view)
            .ok_or(EditorError::NotFound)?
            .composition = composition;
        Ok(())
    }

    pub fn apply(
        &mut self,
        document: DocumentId,
        transaction: Transaction,
    ) -> Result<u64, EditorError> {
        self.apply_transaction(document, transaction, true)
    }

    pub fn apply_separate(
        &mut self,
        document: DocumentId,
        transaction: Transaction,
    ) -> Result<u64, EditorError> {
        self.apply_transaction(document, transaction, false)
    }

    fn apply_transaction(
        &mut self,
        document: DocumentId,
        transaction: Transaction,
        allow_merge: bool,
    ) -> Result<u64, EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.revision != transaction.revision {
            return Err(EditorError::StaleRevision);
        }
        if owner.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if let Some(origin) = transaction.origin {
            if self
                .views
                .get(origin)
                .is_none_or(|view| view.document != document)
            {
                return Err(EditorError::InvalidIdentity);
            }
        } else if transaction.selection_after.is_some() {
            return Err(EditorError::InvalidIdentity);
        }
        let (after, edits) = apply_edits(
            &owner.rope,
            &transaction.edits,
            self.limits.max_document_bytes,
        )?;
        if let Some(selection) = &transaction.selection_after {
            selection.validate(&after)?;
        }
        if edits.is_empty() {
            return Ok(owner.revision);
        }
        let next_revision = owner
            .revision
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let before_selections: HashMap<_, _> = self
            .views
            .for_document(document)
            .map(|view| (view.id, view.selection.clone()))
            .collect();
        let mut after_selections: HashMap<_, _> = before_selections
            .iter()
            .map(|(view, selection)| (*view, selection.mapped(&edits)))
            .collect();
        if let (Some(origin), Some(selection)) = (transaction.origin, transaction.selection_after) {
            after_selections.insert(origin, selection);
        }
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        let entry = HistoryEntry {
            before: owner.rope.clone(),
            after: after.clone(),
            before_selections,
            after_selections: after_selections.clone(),
            group: transaction.group,
            origin: transaction.origin,
        };
        if allow_merge
            && owner.can_merge
            && owner
                .undo
                .back()
                .is_some_and(|last| last.group == entry.group && last.origin == entry.origin)
        {
            let last = owner.undo.back_mut().ok_or(EditorError::NotFound)?;
            last.after = entry.after;
            last.after_selections = entry.after_selections;
        } else {
            owner.undo.push_back(entry);
            if owner.undo.len() > self.limits.max_undo_groups {
                owner.undo.pop_front();
            }
        }
        owner.rope = after;
        owner.revision = next_revision;
        owner.requires_save = true;
        owner.syntax_tokens = None;
        owner.redo.clear();
        owner.can_merge = allow_merge;
        for view in self
            .views
            .views
            .values_mut()
            .filter(|view| view.document == document)
        {
            view.selection = after_selections
                .remove(&view.id)
                .ok_or(EditorError::NotFound)?;
            view.composition = None;
            let fold_set = SelectionSet {
                primary: 0,
                selections: view
                    .folds
                    .iter()
                    .map(|fold| crate::view::Selection {
                        anchor: fold.start,
                        head: fold.end,
                    })
                    .collect(),
            }
            .mapped(&edits);
            view.folds = fold_set
                .selections
                .into_iter()
                .filter(|selection| selection.anchor < selection.head)
                .map(|selection| selection.anchor..selection.head)
                .collect();
        }
        Ok(next_revision)
    }

    pub fn undo(&mut self, document: DocumentId) -> Result<bool, EditorError> {
        self.restore_history(document, false)
    }
    pub fn redo(&mut self, document: DocumentId) -> Result<bool, EditorError> {
        self.restore_history(document, true)
    }

    fn restore_history(&mut self, document: DocumentId, redo: bool) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if (redo && owner.redo.is_empty()) || (!redo && owner.undo.is_empty()) {
            return Ok(false);
        }
        let revision = owner
            .revision
            .checked_add(1)
            .ok_or(EditorError::RevisionOverflow)?;
        let entry = if redo {
            owner.redo.pop()
        } else {
            owner.undo.pop_back()
        }
        .ok_or(EditorError::NotFound)?;
        let (text, selections) = if redo {
            (&entry.after, &entry.after_selections)
        } else {
            (&entry.before, &entry.before_selections)
        };
        for view in self
            .views
            .views
            .values_mut()
            .filter(|view| view.document == document)
        {
            view.selection = selections
                .get(&view.id)
                .cloned()
                .unwrap_or_else(|| view.selection.clamped(text));
            view.composition = None;
            view.folds.clear();
        }
        owner.rope = text.clone();
        owner.revision = revision;
        owner.requires_save = true;
        owner.syntax_tokens = None;
        owner.can_merge = false;
        if redo {
            owner.undo.push_back(entry);
        } else {
            owner.redo.push(entry);
        }
        Ok(true)
    }

    pub fn save_snapshot(&mut self, document: DocumentId) -> Result<SaveSnapshot, EditorError> {
        let owner = self
            .documents
            .documents
            .get_mut(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        owner.can_merge = false;
        Ok(SaveSnapshot {
            document,
            key: owner.key.clone(),
            revision: owner.revision,
            rope: owner.rope.clone(),
        })
    }

    pub fn mark_saved(
        &mut self,
        snapshot: SaveSnapshot,
        disk_modified_ms: Option<f64>,
    ) -> Result<bool, EditorError> {
        let owner = self
            .documents
            .documents
            .get_mut(&snapshot.document)
            .ok_or(EditorError::NotFound)?;
        if owner.key != snapshot.key || matches!(owner.key, DocumentKey::AppFile(_)) {
            return Err(EditorError::InvalidIdentity);
        }
        if snapshot.revision < owner.saved_revision || snapshot.revision > owner.revision {
            return Err(EditorError::StaleSave);
        }
        owner.baseline = snapshot.rope;
        owner.saved_revision = snapshot.revision;
        owner.metadata.disk_modified_ms = disk_modified_ms;
        owner.observed_disk = Some(DiskSnapshot {
            rope: owner.baseline.clone(),
            metadata: owner.metadata.clone(),
        });
        owner.can_merge = false;
        owner.requires_save = false;
        Ok(owner.rope == owner.baseline)
    }

    pub fn detach_view(&mut self, view: ViewId) -> Result<ViewState, EditorError> {
        let removed = self
            .views
            .views
            .remove(&view)
            .ok_or(EditorError::NotFound)?;
        self.views.by_key.remove(&removed.key);
        if let Some(owner) = self.documents.documents.get_mut(&removed.document) {
            owner.can_merge = false;
        }
        Ok(removed)
    }

    pub fn release_document(&mut self, document: DocumentId) -> Result<(), EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if self.views.for_document(document).next().is_some() {
            return Err(EditorError::AttachedViews);
        }
        if owner.requires_save || owner.rope != owner.baseline {
            return Err(EditorError::UnsavedChanges);
        }
        let owner = self
            .documents
            .remove(document)
            .ok_or(EditorError::NotFound)?;
        self.documents.by_key.remove(&owner.key);
        Ok(())
    }

    pub fn discard_document(
        &mut self,
        document: DocumentId,
        revision: u64,
    ) -> Result<(), EditorError> {
        let owner = self
            .documents
            .documents
            .get(&document)
            .ok_or(EditorError::NotFound)?;
        if owner.revision != revision {
            return Err(EditorError::StaleRevision);
        }
        if self.views.for_document(document).next().is_some() {
            return Err(EditorError::AttachedViews);
        }
        let owner = self
            .documents
            .remove(document)
            .ok_or(EditorError::NotFound)?;
        self.documents.by_key.remove(&owner.key);
        Ok(())
    }
}

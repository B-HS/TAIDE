use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use taide_infra::lsp_frame::FrameLimits;
use taide_infra::lsp_proc::LspProcConfig;
use taide_infra::lsp_writer::WriterLimits;
use taide_infra::root_guard;
use taide_lsp::native::protocol::{
    Rejection, ServerNotification, ServerReply, ServerRequestKind, lsp_types,
};
use taide_lsp::native::session::{
    SessionClient, SessionNotice, SessionNoticeKind, SessionOptions, SessionSnapshot,
};
use taide_lsp::native::{DocumentMirror, Failure, MAX_MIRROR_BYTES, MAX_PENDING_REQUESTS, Phase};
use taide_model::error::{AppError, AppResult};
use taide_model::file::FileSizeTier;
use taide_model::ids::ProjectId;
use taide_model::layout::{PaneNode, TabKind};
use taide_model::lsp::{LanguageServerSpec, LspServerId};
use taide_model::project::ShellSlotTree;
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot};
use taide_native_ui::shell::WindowScope;
use taide_native_ui::snapshot::ShellSnapshot;
use taide_runtime::{AppServices, native_lsp_actions};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

const COMMAND_CAPACITY: usize = 64;
const HEADER_BYTES: usize = 4 * 1024;
const REQUEST_TIMEOUT_MS: u64 = 15_000;
const EXIT_GRACE: Duration = Duration::from_secs(2);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const SAVE_ACTION_TIMEOUT: Duration = Duration::from_secs(5);

#[path = "lsp-recovery.rs"]
mod recovery;

#[path = "lsp-status.rs"]
pub mod status;

#[path = "lsp-diagnostics.rs"]
mod raw_diagnostics;

#[path = "lsp-idle.rs"]
mod idle;

#[path = "lsp-document-symbols.rs"]
mod document_symbols;
#[path = "lsp-editor-completion.rs"]
mod editor_completion;
#[path = "lsp-editor-documentation.rs"]
mod editor_documentation;
#[path = "lsp-symbol-locations.rs"]
mod symbol_locations;
#[path = "lsp-syntax-folding.rs"]
mod syntax_folding;

#[path = "lsp-workspace-symbols.rs"]
mod workspace_symbols;

#[cfg(all(test, unix))]
#[path = "lsp-diagnostics-tests.rs"]
mod diagnostics_tests;

#[cfg(all(test, unix))]
#[path = "lsp-document-symbols-tests.rs"]
mod document_symbol_tests;
#[cfg(all(test, unix))]
#[path = "lsp-editor-completion-tests.rs"]
mod editor_completion_tests;
#[cfg(all(test, unix))]
#[path = "lsp-editor-documentation-tests.rs"]
mod editor_documentation_tests;

#[cfg(all(test, unix))]
#[path = "lsp-symbol-locations-tests.rs"]
mod symbol_location_tests;
#[cfg(all(test, unix))]
#[path = "lsp-syntax-folding-tests.rs"]
mod syntax_folding_tests;
#[cfg(all(test, unix))]
#[path = "lsp-workspace-symbols-tests.rs"]
mod workspace_symbol_tests;

#[derive(Clone, Copy, Default)]
pub struct SaveActionFlags {
    pub fix_all: bool,
    pub organize_imports: bool,
}

#[derive(Clone)]
pub struct ProtocolDocument {
    pub snapshot: DocumentSnapshot,
    pub uri: String,
    pub revision: Option<u64>,
}

pub struct WorkspaceEditEvent {
    pub edit: lsp_types::WorkspaceEdit,
    pub documents: Vec<ProtocolDocument>,
    pub completion: oneshot::Sender<bool>,
}

pub fn visible_files<'a>(
    snapshot: &'a ShellSnapshot,
    scope: &WindowScope,
) -> Vec<(&'a ProjectId, &'a str)> {
    let mut files = Vec::new();
    match scope {
        WindowScope::Main => {
            if !snapshot.projects.is_empty()
                && let Some(tree) = &snapshot.shell.tree
            {
                visible_slots(tree, snapshot, &mut files);
            }
        }
        WindowScope::Auxiliary { project, slot } => {
            if let Some((project, layout)) = snapshot.layouts.get_key_value(project)
                && let Some(window) = layout
                    .auxiliary_windows
                    .iter()
                    .find(|window| window.slot == *slot)
            {
                visible_panes(&window.root, project, &mut files);
            }
        }
    }
    files
}

fn visible_slots<'a>(
    tree: &'a ShellSlotTree,
    snapshot: &'a ShellSnapshot,
    files: &mut Vec<(&'a ProjectId, &'a str)>,
) {
    if snapshot.shell.window_chrome.zen
        && snapshot
            .shell
            .focused
            .as_ref()
            .is_none_or(|focused| taide_native_ui::snapshot::slot_project(tree, focused).is_none())
    {
        return;
    }
    match tree {
        ShellSlotTree::Leaf { project_id, .. } => {
            if let Some(layout) = snapshot.layouts.get(project_id) {
                visible_panes(&layout.root, project_id, files);
            }
        }
        ShellSlotTree::Split { children, .. } => {
            for child in children {
                visible_slots(child, snapshot, files);
            }
        }
    }
}

fn visible_panes<'a>(
    node: &'a PaneNode,
    project: &'a ProjectId,
    files: &mut Vec<(&'a ProjectId, &'a str)>,
) {
    match node {
        PaneNode::Leaf { tabs, active, .. } => {
            if let Some(tab) = tabs.iter().find(|tab| Some(&tab.id) == active.as_ref())
                && let TabKind::File { path } = &tab.kind
            {
                files.push((project, path));
            }
        }
        PaneNode::Split { children, .. } => {
            for child in children {
                visible_panes(child, project, files);
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct SessionKey {
    project: ProjectId,
    server: LspServerId,
    root: String,
}

struct Plan {
    key: SessionKey,
    spec: LanguageServerSpec,
    config: LspProcConfig,
}

#[derive(Clone)]
struct Document {
    snapshot: DocumentSnapshot,
    uri: String,
    protocol_revision: u64,
}

#[derive(Clone)]
struct Session {
    client: SessionClient,
    order: usize,
    owner: crate::diagnostics::Owner,
    name: String,
    roots: Vec<String>,
    documents: HashMap<DocumentId, Document>,
    raw_diagnostics: raw_diagnostics::Store,
    marker_documents: HashMap<DocumentId, String>,
    idle: idle::Idle,
}

enum Command {
    Completion(crate::editor_completion::Request),
    Documentation(crate::editor_documentation::Request),
    SymbolLocations(crate::editor_locations::Request),
    SyntaxFolding(crate::editor_folding::Request),
    DocumentSymbols(crate::editor_symbols::Request),
    WorkspaceSymbols(crate::workspace_symbols::Request),
    ExplorerPaste {
        project: ProjectId,
        request: crate::explorer_clipboard::Request,
    },
    ExplorerDelete(crate::explorer_delete::Request),
    ExplorerCreate {
        project: ProjectId,
        request: crate::explorer::CreateRequest,
    },
    ExplorerRename {
        project: ProjectId,
        request: crate::explorer::RenameRequest,
    },
    Sync {
        project: ProjectId,
        snapshot: DocumentSnapshot,
    },
    Close(DocumentId),
    CloseBinding(ProjectId, DocumentId),
    DisposeModels(Vec<String>),
    Models {
        changed: Vec<DocumentSnapshot>,
        removed: HashSet<DocumentId>,
    },
    RetainProjects(HashSet<ProjectId>),
    Saved(DocumentId),
    Format {
        snapshot: DocumentSnapshot,
        options: Option<lsp_types::FormattingOptions>,
        actions: SaveActionFlags,
    },
    Bindings {
        document: DocumentId,
        completion: oneshot::Sender<HashMap<SessionKey, Session>>,
    },
    Notice {
        key: SessionKey,
        source: watch::Receiver<SessionSnapshot>,
        notice: SessionNotice,
    },
    StateChanged {
        key: SessionKey,
        source: watch::Receiver<SessionSnapshot>,
        state: SessionSnapshot,
    },
}

pub enum Reply {
    Completion {
        request: crate::editor_completion::Request,
        result: Result<crate::editor_completion::Response, Failure>,
    },
    Documentation {
        request: crate::editor_documentation::Request,
        result: Result<crate::editor_documentation::Response, Failure>,
    },
    SymbolLocations {
        request: crate::editor_locations::Request,
        result: Result<crate::editor_locations::Response, Failure>,
    },
    SyntaxFolding {
        request: crate::editor_folding::Request,
        result: Result<crate::editor_folding::Response, Failure>,
    },
    WorkspaceSymbols {
        request: crate::workspace_symbols::Request,
        result: Result<crate::workspace_symbols::Response, Failure>,
    },
    DocumentSymbols {
        request: crate::editor_symbols::Request,
        result: Result<crate::editor_symbols::Response, Failure>,
    },
    ExplorerMovePrepare(crate::explorer_source_missing::Prepare),
    ExplorerMoved(crate::explorer_source_missing::Changed),
    ExplorerPasted(crate::explorer_clipboard::Reply),
    ExplorerDeletePrepare(crate::explorer_delete::Prepare),
    ExplorerDeleted(crate::explorer_delete::Deleted),
    ExplorerDeleteFinished {
        request: crate::explorer_delete::Request,
        result: AppResult<Vec<DocumentId>>,
    },
    ExplorerCreated(crate::explorer::CreateReply),
    ExplorerRenamed(crate::explorer::RenameReply),
    RenamePrepare(crate::workspace_rename::Prepare),
    Renamed(crate::workspace_rename::Renamed),
    DeletePrepare(crate::workspace_delete::Prepare),
    Deleted(crate::workspace_delete::Deleted),
    DocumentQuery {
        path: std::path::PathBuf,
        completion: oneshot::Sender<Option<DocumentSnapshot>>,
    },
    Synced {
        document: DocumentId,
        revision: u64,
        sessions: Vec<SessionSnapshot>,
    },
    Formatted {
        snapshot: DocumentSnapshot,
        result: Result<Option<Vec<lsp_types::TextEdit>>, Failure>,
    },
    Diagnostics {
        owner: crate::diagnostics::Owner,
        document: DocumentId,
        revision: u64,
        diagnostics: lsp_types::PublishDiagnosticsParams,
    },
    Failed {
        document: Option<DocumentId>,
        error: AppError,
    },
    WorkspaceEdit(WorkspaceEditEvent),
}

pub struct LspBridge {
    commands: mpsc::Sender<Command>,
    replies: mpsc::Receiver<Reply>,
    stop: watch::Sender<bool>,
    worker: Option<JoinHandle<()>>,
    synced: HashMap<(ProjectId, DocumentId), (u64, String, FileSizeTier)>,
    models: HashMap<DocumentId, (DocumentKey, u64)>,
    projects: Option<HashSet<ProjectId>>,
    states: watch::Receiver<Vec<RegistryState>>,
}

struct RegistryState {
    project: ProjectId,
    name: String,
    snapshot: SessionSnapshot,
    owner: crate::diagnostics::Owner,
    documents: HashSet<DocumentId>,
    open_documents: HashSet<DocumentId>,
}

struct Publishers {
    replies: mpsc::Sender<Reply>,
    states: watch::Sender<Vec<RegistryState>>,
}

impl LspBridge {
    pub fn connect(
        services: Arc<AppServices>,
        path_var: OsString,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> AppResult<Self> {
        let (commands, receiver) = mpsc::channel(COMMAND_CAPACITY);
        let (sender, replies) = mpsc::channel(COMMAND_CAPACITY);
        let (states, snapshots) = watch::channel(Vec::new());
        let (stop, stopping) = watch::channel(false);
        let worker = services
            .tasks
            .clone()
            .spawn_transient_handle(
                "native-lsp-documents",
                run(
                    services,
                    path_var,
                    commands.clone(),
                    receiver,
                    Publishers {
                        replies: sender,
                        states,
                    },
                    stopping,
                    repaint,
                ),
            )
            .ok_or_else(|| AppError::Forbidden("native language server host is stopping".into()))?;
        Ok(Self {
            commands,
            replies,
            stop,
            worker: Some(worker),
            synced: HashMap::new(),
            models: HashMap::new(),
            projects: None,
            states: snapshots,
        })
    }

    pub fn sync(&mut self, project: ProjectId, snapshot: DocumentSnapshot) -> AppResult<()> {
        let identity = (
            snapshot.revision,
            snapshot.metadata.language_id.clone(),
            snapshot.metadata.tier,
        );
        let key = (project.clone(), snapshot.id);
        if self.synced.get(&key) == Some(&identity) {
            return Ok(());
        }
        self.submit(Command::Sync { project, snapshot })?;
        self.synced.insert(key, identity);
        Ok(())
    }

    pub fn summary(&self, project: &ProjectId) -> Option<status::Summary> {
        if self.states.has_changed().is_err() {
            return None;
        }
        status::summarize(
            project,
            self.states
                .borrow()
                .iter()
                .map(|state| (&state.project, &state.snapshot)),
        )
    }

    pub fn diagnostic_bindings(&mut self) -> Option<crate::diagnostics::Bindings> {
        match self.states.has_changed() {
            Ok(false) => None,
            Err(_) => Some(HashMap::new()),
            Ok(true) => Some(
                self.states
                    .borrow_and_update()
                    .iter()
                    .map(|state| (state.owner, state.documents.clone()))
                    .collect(),
            ),
        }
    }

    pub fn usage_labels(&self) -> crate::remote_utilities::LabelProvider {
        let pids = self.states.clone();
        Arc::new(move |services| {
            if pids.has_changed().is_err() {
                return crate::remote_utilities::UsageLabels::new();
            }
            let snapshots = pids.borrow();
            let projects = services.state.projects.read();
            snapshots
                .iter()
                .filter_map(|state| {
                    let pid = state.snapshot.pid?;
                    let name = &state.name;
                    let label = projects
                        .get(&state.project)
                        .map(|project| format!("{name} · {}", project.name))
                        .unwrap_or_else(|| name.clone());
                    Some((
                        pid,
                        (taide_model::system::SystemUsageProcessKind::Lsp, label),
                    ))
                })
                .collect()
        })
    }

    pub fn retain(&mut self, documents: &HashSet<DocumentId>) -> AppResult<()> {
        let closed = self
            .synced
            .keys()
            .filter(|(_, document)| !documents.contains(document))
            .map(|(_, document)| *document)
            .collect::<HashSet<_>>();
        for document in closed {
            self.submit(Command::Close(document))?;
            self.synced.retain(|(_, id), _| *id != document);
        }
        Ok(())
    }

    pub(crate) fn retain_bindings(
        &mut self,
        bindings: &HashSet<(ProjectId, DocumentId)>,
    ) -> AppResult<()> {
        let closed = self
            .synced
            .keys()
            .filter(|key| !bindings.contains(*key))
            .cloned()
            .collect::<Vec<_>>();
        for (project, document) in closed {
            self.submit(Command::CloseBinding(project.clone(), document))?;
            self.synced.remove(&(project, document));
        }
        Ok(())
    }

    pub(crate) fn flush_model_disposals(
        &self,
        editor: &mut taide_native_editor::store::EditorStore,
    ) -> AppResult<()> {
        if editor.pending_document_disposals().is_empty() {
            return Ok(());
        }
        let uris = editor
            .pending_document_disposals()
            .iter()
            .filter_map(|key| match key {
                DocumentKey::File(path) => {
                    path.to_str().map(taide_lsp::service::workspace_folder_uri)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if !uris.is_empty() {
            self.submit(Command::DisposeModels(uris))?;
        }
        editor.acknowledge_document_disposals();
        Ok(())
    }

    pub(crate) fn reconcile_models(
        &mut self,
        editor: &mut taide_native_editor::store::EditorStore,
    ) -> AppResult<()> {
        self.flush_model_disposals(editor)?;
        let snapshots = editor
            .documents()
            .snapshots()
            .filter(
                |model| matches!(&model.key, DocumentKey::File(path) if path.to_str().is_some()),
            )
            .collect::<Vec<_>>();
        let identities = snapshots
            .iter()
            .map(|model| (model.id, (model.key.clone(), model.revision)))
            .collect::<HashMap<_, _>>();
        let changed = snapshots
            .into_iter()
            .filter(|model| self.models.get(&model.id) != identities.get(&model.id))
            .collect::<Vec<_>>();
        let removed = self
            .models
            .keys()
            .filter(|id| !identities.contains_key(id))
            .copied()
            .collect::<HashSet<_>>();
        if changed.is_empty() && removed.is_empty() {
            return Ok(());
        }
        self.submit(Command::Models {
            changed,
            removed: removed.clone(),
        })?;
        self.models = identities;
        self.synced.retain(|(_, id), _| !removed.contains(id));
        Ok(())
    }

    pub fn retain_projects(&mut self, projects: &HashSet<ProjectId>) -> AppResult<()> {
        if self.projects.as_ref() == Some(projects) {
            return Ok(());
        }
        self.submit(Command::RetainProjects(projects.clone()))?;
        self.projects = Some(projects.clone());
        self.synced
            .retain(|(project, _), _| projects.contains(project));
        Ok(())
    }

    pub fn saved(&self, document: DocumentId) -> AppResult<()> {
        self.submit(Command::Saved(document))
    }

    pub(crate) fn document_symbols(
        &self,
        request: crate::editor_symbols::Request,
    ) -> AppResult<()> {
        self.submit(Command::DocumentSymbols(request))
    }

    pub(crate) fn syntax_folding(&self, request: crate::editor_folding::Request) -> AppResult<()> {
        self.submit(Command::SyntaxFolding(request))
    }

    pub(crate) fn symbol_locations(
        &self,
        request: crate::editor_locations::Request,
    ) -> AppResult<()> {
        self.submit(Command::SymbolLocations(request))
    }

    pub(crate) fn documentation(
        &self,
        request: crate::editor_documentation::Request,
    ) -> AppResult<()> {
        self.submit(Command::Documentation(request))
    }

    pub(crate) fn completion(&self, request: crate::editor_completion::Request) -> AppResult<()> {
        self.submit(Command::Completion(request))
    }

    pub(crate) fn completion_providers(
        &self,
        project: &ProjectId,
        snapshot: &DocumentSnapshot,
    ) -> HashSet<crate::editor_symbols::ProviderIdentity> {
        self.feature_providers(project, snapshot, "textDocument/completion")
    }

    pub(crate) fn completion_options(
        &self,
        project: &ProjectId,
        snapshot: &DocumentSnapshot,
    ) -> HashMap<crate::editor_symbols::ProviderIdentity, Vec<lsp_types::CompletionOptions>> {
        let DocumentKey::File(path) = &snapshot.key else {
            return HashMap::new();
        };
        let Some(path) = path.to_str() else {
            return HashMap::new();
        };
        if self.states.has_changed().is_err() {
            return HashMap::new();
        }
        let uri = taide_lsp::service::workspace_folder_uri(path);
        self.states
            .borrow()
            .iter()
            .filter(|state| {
                state.project == *project
                    && state.open_documents.contains(&snapshot.id)
                    && state
                        .snapshot
                        .supports_document(&uri, "textDocument/completion")
            })
            .filter_map(|state| {
                let options = state.snapshot.document_completion_options.get(&uri)?;
                Some((
                    crate::editor_symbols::ProviderIdentity {
                        owner: state.owner,
                        generation: state.snapshot.generation,
                        capability_revision: state.snapshot.capability_revision,
                    },
                    options.clone(),
                ))
            })
            .collect()
    }

    pub(crate) fn documentation_providers(
        &self,
        project: &ProjectId,
        snapshot: &DocumentSnapshot,
        kind: crate::editor_documentation::Kind,
    ) -> HashSet<crate::editor_symbols::ProviderIdentity> {
        self.feature_providers(project, snapshot, kind.method())
    }

    pub(crate) fn signature_triggers(
        &self,
        project: &ProjectId,
        snapshot: &DocumentSnapshot,
    ) -> taide_native_editor::documentation::SignatureTriggers {
        let DocumentKey::File(path) = &snapshot.key else {
            return Default::default();
        };
        let Some(path) = path.to_str() else {
            return Default::default();
        };
        if self.states.has_changed().is_err() {
            return Default::default();
        }
        let uri = taide_lsp::service::workspace_folder_uri(path);
        let states = self.states.borrow();
        taide_native_editor::documentation::SignatureTriggers::from_options(
            states
                .iter()
                .filter(|state| {
                    state.project == *project
                        && state.open_documents.contains(&snapshot.id)
                        && state
                            .snapshot
                            .supports_document(&uri, "textDocument/signatureHelp")
                })
                .flat_map(|state| {
                    state
                        .snapshot
                        .document_signature_options
                        .get(&uri)
                        .into_iter()
                        .flatten()
                }),
        )
    }

    pub(crate) fn location_providers(
        &self,
        project: &ProjectId,
        snapshot: &DocumentSnapshot,
        kind: taide_native_editor::symbol_locations::Kind,
    ) -> HashSet<crate::editor_symbols::ProviderIdentity> {
        self.feature_providers(project, snapshot, kind.method())
    }

    fn feature_providers(
        &self,
        project: &ProjectId,
        snapshot: &DocumentSnapshot,
        method: &str,
    ) -> HashSet<crate::editor_symbols::ProviderIdentity> {
        let DocumentKey::File(path) = &snapshot.key else {
            return HashSet::new();
        };
        let Some(path) = path.to_str() else {
            return HashSet::new();
        };
        if self.states.has_changed().is_err() {
            return HashSet::new();
        }
        let uri = taide_lsp::service::workspace_folder_uri(path);
        self.states
            .borrow()
            .iter()
            .filter(|state| {
                state.project == *project
                    && state.open_documents.contains(&snapshot.id)
                    && state.snapshot.supports_document(&uri, method)
            })
            .map(|state| crate::editor_symbols::ProviderIdentity {
                owner: state.owner,
                generation: state.snapshot.generation,
                capability_revision: state.snapshot.capability_revision,
            })
            .collect()
    }

    pub(crate) fn workspace_symbols(
        &self,
        request: crate::workspace_symbols::Request,
    ) -> AppResult<()> {
        self.submit(Command::WorkspaceSymbols(request))
    }

    pub(crate) fn workspace_providers(
        &self,
        project: &ProjectId,
    ) -> HashSet<crate::editor_symbols::ProviderIdentity> {
        if self.states.has_changed().is_err() {
            return HashSet::new();
        }
        self.states
            .borrow()
            .iter()
            .filter(|state| {
                state.project == *project
                    && !matches!(state.snapshot.phase, Phase::Stopping | Phase::Stopped)
            })
            .map(|state| crate::editor_symbols::ProviderIdentity {
                owner: state.owner,
                generation: state.snapshot.generation,
                capability_revision: state.snapshot.capability_revision,
            })
            .collect()
    }

    pub(crate) fn symbol_providers(
        &self,
        project: &ProjectId,
        document: DocumentId,
    ) -> HashSet<crate::editor_symbols::ProviderIdentity> {
        if self.states.has_changed().is_err() {
            return HashSet::new();
        }
        self.states
            .borrow()
            .iter()
            .filter(|state| {
                state.project == *project
                    && state.open_documents.contains(&document)
                    && state.snapshot.phase == Phase::Running
            })
            .map(|state| crate::editor_symbols::ProviderIdentity {
                owner: state.owner,
                generation: state.snapshot.generation,
                capability_revision: state.snapshot.capability_revision,
            })
            .collect()
    }

    pub fn format(
        &self,
        snapshot: DocumentSnapshot,
        options: lsp_types::FormattingOptions,
    ) -> AppResult<()> {
        self.participate(snapshot, Some(options), SaveActionFlags::default())
    }

    pub fn participate(
        &self,
        snapshot: DocumentSnapshot,
        options: Option<lsp_types::FormattingOptions>,
        actions: SaveActionFlags,
    ) -> AppResult<()> {
        self.submit(Command::Format {
            snapshot,
            options,
            actions,
        })
    }

    pub fn poll(&mut self) -> Option<Reply> {
        self.replies.try_recv().ok()
    }

    pub fn rename_entry(
        &self,
        project: ProjectId,
        request: crate::explorer::RenameRequest,
    ) -> AppResult<()> {
        self.submit(Command::ExplorerRename { project, request })
    }

    pub fn paste_entry(
        &self,
        project: ProjectId,
        request: crate::explorer_clipboard::Request,
    ) -> AppResult<()> {
        self.submit(Command::ExplorerPaste { project, request })
    }

    pub fn create_entry(
        &self,
        project: ProjectId,
        request: crate::explorer::CreateRequest,
    ) -> AppResult<()> {
        self.submit(Command::ExplorerCreate { project, request })
    }

    pub fn delete_entry(&self, request: crate::explorer_delete::Request) -> AppResult<()> {
        self.submit(Command::ExplorerDelete(request))
    }

    pub fn disconnect(mut self) -> JoinHandle<()> {
        self.stop.send_replace(true);
        self.worker
            .take()
            .expect("native language server worker is owned")
    }

    fn submit(&self, command: Command) -> AppResult<()> {
        self.commands
            .try_send(command)
            .map_err(|_| AppError::Internal("native language server queue is unavailable".into()))
    }
}

impl Drop for LspBridge {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}

fn options() -> Result<SessionOptions, Failure> {
    Ok(SessionOptions {
        frame_limits: FrameLimits::new(HEADER_BYTES, MAX_MIRROR_BYTES)
            .map_err(|_| Failure::Capacity)?,
        writer_limits: WriterLimits::new(MAX_PENDING_REQUESTS, MAX_MIRROR_BYTES)
            .map_err(|_| Failure::Capacity)?,
        command_capacity: MAX_PENDING_REQUESTS,
        command_bytes: MAX_MIRROR_BYTES,
        incoming_capacity: MAX_PENDING_REQUESTS,
        incoming_bytes: MAX_MIRROR_BYTES,
        outgoing_capacity: MAX_PENDING_REQUESTS,
        outgoing_bytes: MAX_MIRROR_BYTES,
        request_timeout_ms: REQUEST_TIMEOUT_MS,
        write_timeout: WRITE_TIMEOUT,
        exit_grace: EXIT_GRACE,
    })
}

fn initialize(plan: &Plan) -> Value {
    let root = &plan.key.root;
    let mut params = json!({
        "processId":null, "clientInfo":{"name":"TAIDE"},
        "rootUri":taide_lsp::service::workspace_folder_uri(root), "rootPath":root,
        "workspaceFolders":[{"uri":taide_lsp::service::workspace_folder_uri(root),"name":Path::new(root).file_name().and_then(|name| name.to_str()).unwrap_or(root)}],
        "capabilities":{
            "general":{"positionEncodings":["utf-16"]},
            "workspace":{"workspaceFolders":true,"configuration":true,"applyEdit":true,"workspaceEdit":{"documentChanges":true},"symbol":{"dynamicRegistration":false,"symbolKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]}}},
            "textDocument":{"hover":{"contentFormat":["markdown","plaintext"]},"completion":{"completionItem":{"snippetSupport":true},"contextSupport":true,"dynamicRegistration":true},"signatureHelp":{},"definition":{"linkSupport":true},"declaration":{"linkSupport":true},"typeDefinition":{"linkSupport":true},"implementation":{"linkSupport":true},"references":{},"foldingRange":{"dynamicRegistration":false,"lineFoldingOnly":true,"rangeLimit":taide_native_editor::folding::MAX_FOLDING_REGIONS},"synchronization":{"dynamicRegistration":false,"didSave":true},"documentSymbol":{"hierarchicalDocumentSymbolSupport":true,"symbolKind":{"valueSet":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]}},"formatting":{},"codeAction":{"codeActionLiteralSupport":{"codeActionKind":{"valueSet":["source.fixAll","source.organizeImports"]}},"resolveSupport":{"properties":["edit","command"]},"dataSupport":true}}
        }
    });
    if let Some(initialization) = &plan.spec.initialization_options {
        params["initializationOptions"] = initialization.clone();
    }
    params
}

async fn plans(
    services: &Arc<AppServices>,
    project: ProjectId,
    snapshot: &DocumentSnapshot,
    path_var: OsString,
) -> AppResult<Vec<Plan>> {
    let DocumentKey::File(path) = &snapshot.key else {
        return Ok(Vec::new());
    };
    if snapshot.metadata.tier != FileSizeTier::Normal {
        return Ok(Vec::new());
    }
    let services = services.clone();
    let path = path.clone();
    let language = snapshot.metadata.language_id.clone();
    services
        .tasks
        .clone()
        .run_blocking_result("native-lsp-discovery", move || {
            if services.state.is_shutting_down() {
                return Err(AppError::Forbidden(
                    "native language server host is stopping".into(),
                ));
            }
            let root = root_guard::project_root(&services.state.projects.read(), &project)?;
            if root_guard::ensure_within_root(&root, &path)? != path {
                return Err(AppError::Forbidden(
                    "native language server document identity changed".into(),
                ));
            }
            let available =
                taide_lsp::service::detect_servers(&services.state.paths.lsp_dir(), &path_var);
            let mut plans = Vec::new();
            for spec in taide_lsp::manifest::servers()
                .into_iter()
                .filter(|spec| spec.language_ids.contains(&language))
            {
                if !available
                    .iter()
                    .any(|server| server.id == spec.id && server.available)
                {
                    continue;
                }
                let server_root =
                    taide_lsp::service::find_root(&spec, &path).unwrap_or_else(|| root.clone());
                let server_root = server_root
                    .to_str()
                    .ok_or_else(|| {
                        AppError::InvalidArgument(
                            "native language server root must be UTF-8".into(),
                        )
                    })?
                    .to_owned();
                let config = taide_lsp::process::resolve_process_config(
                    &services.state.paths,
                    &spec,
                    &server_root,
                    &path_var,
                )?;
                plans.push(Plan {
                    key: SessionKey {
                        project: project.clone(),
                        server: spec.id.clone(),
                        root: server_root,
                    },
                    spec,
                    config,
                });
            }
            Ok(plans)
        })
        .await
}

fn session_key(sessions: &HashMap<SessionKey, Session>, plan: &Plan) -> SessionKey {
    sessions
        .iter()
        .find_map(|(key, session)| {
            (key.project == plan.key.project
                && key.server == plan.key.server
                && !matches!(
                    session.client.snapshot().phase,
                    Phase::Stopping | Phase::Stopped
                )
                && taide_lsp::service::should_reuse_session(
                    &plan.spec,
                    &session.roots,
                    &plan.key.root,
                ))
            .then(|| key.clone())
        })
        .unwrap_or_else(|| plan.key.clone())
}

fn workspace_folders(session: &Session) -> Result<Vec<lsp_types::WorkspaceFolder>, Failure> {
    session
        .roots
        .iter()
        .map(|root| {
            Ok(lsp_types::WorkspaceFolder {
                uri: taide_lsp::service::workspace_folder_uri(root)
                    .parse()
                    .map_err(|_| Failure::MalformedRequest)?,
                name: Path::new(root)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(root)
                    .into(),
            })
        })
        .collect()
}

async fn sync_document(
    services: &Arc<AppServices>,
    sessions: &mut HashMap<SessionKey, Session>,
    commands: &mpsc::Sender<Command>,
    project: ProjectId,
    snapshot: DocumentSnapshot,
    path_var: OsString,
) -> AppResult<Vec<SessionSnapshot>> {
    let plans = plans(services, project.clone(), &snapshot, path_var).await?;
    let mut keys = plans
        .iter()
        .map(|plan| session_key(sessions, plan))
        .collect::<HashSet<_>>();
    keys.extend(
        sessions
            .keys()
            .filter(|key| key.project != project)
            .cloned(),
    );
    close_document(sessions, snapshot.id, Some(&keys), None).await;
    let DocumentKey::File(path) = &snapshot.key else {
        return Ok(Vec::new());
    };
    let path = path.to_str().ok_or_else(|| {
        AppError::InvalidArgument("native language server path must be UTF-8".into())
    })?;
    let uri = taide_lsp::service::workspace_folder_uri(path);
    let mut state = Vec::new();
    for plan in plans {
        let key = session_key(sessions, &plan);
        if !sessions.contains_key(&key) {
            let order = sessions
                .values()
                .map(|session| session.order)
                .max()
                .map_or(Ok(0), |order| {
                    order
                        .checked_add(1)
                        .ok_or_else(|| failure(Failure::Capacity))
                })?;
            let params = initialize(&plan);
            let client = native_lsp_actions::spawn_session(
                &services.tasks,
                services.lsp.clone(),
                recovery::copy_config(&plan.config),
                params,
                options().map_err(failure)?,
            )
            .map_err(failure)?;
            let mut notices = client
                .take_notifications()
                .ok_or_else(|| failure(Failure::TransportClosed))?;
            let sender = commands.clone();
            let notice_key = key.clone();
            let source = client.subscribe();
            if !services
                .tasks
                .spawn_transient("native-lsp-notifications", async move {
                    while let Some(notice) = notices.recv().await {
                        if sender
                            .send(Command::Notice {
                                key: notice_key.clone(),
                                source: source.clone(),
                                notice,
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                })
            {
                return Err(failure(Failure::TransportClosed));
            }
            sessions.insert(
                key.clone(),
                Session {
                    client,
                    order,
                    owner: crate::diagnostics::Owner::new(),
                    name: plan.spec.name.clone(),
                    roots: vec![plan.key.root.clone()],
                    documents: HashMap::new(),
                    raw_diagnostics: raw_diagnostics::Store::default(),
                    marker_documents: HashMap::new(),
                    idle: idle::Idle::default(),
                },
            );
            if !recovery::attach(
                services,
                sessions[&key].client.clone(),
                plan.config,
                key.clone(),
                commands.clone(),
            ) {
                if let Some(session) = sessions.remove(&key) {
                    let _ = session.client.stop().await;
                }
                return Err(failure(Failure::TransportClosed));
            }
        }
        let session = sessions
            .get_mut(&key)
            .ok_or_else(|| failure(Failure::TransportClosed))?;
        if !session.roots.contains(&plan.key.root) {
            let mut client = session.client.clone();
            client
                .wait_for_phase(Phase::Running)
                .await
                .map_err(failure)?;
            client
                .add_workspace_root(plan.key.root.clone())
                .await
                .map_err(failure)?;
            session.roots.push(plan.key.root);
        }
        sync_session_document(session, &snapshot, &uri).await?;
        state.push(session.client.snapshot());
    }
    Ok(state)
}

async fn sync_session_document(
    session: &mut Session,
    snapshot: &DocumentSnapshot,
    uri: &str,
) -> AppResult<()> {
    if let Some(previous) = session.documents.get(&snapshot.id)
        && (previous.uri != uri
            || previous.snapshot.metadata.language_id != snapshot.metadata.language_id)
    {
        session
            .client
            .close(previous.uri.to_owned())
            .await
            .map_err(failure)?;
        if previous.uri != uri {
            session.raw_diagnostics.remove(&previous.uri);
            session.marker_documents.remove(&snapshot.id);
        }
        session.documents.remove(&snapshot.id);
    }
    let protocol_revision = if let Some(previous) = session.documents.get(&snapshot.id) {
        if previous.snapshot.revision == snapshot.revision {
            return Ok(());
        }
        let next = previous
            .protocol_revision
            .checked_add(1)
            .ok_or_else(|| failure(Failure::CounterOverflow))?;
        session
            .client
            .change(
                uri.to_owned(),
                previous.protocol_revision,
                next,
                snapshot.rope.to_string(),
            )
            .await
            .map_err(failure)?;
        next
    } else {
        session
            .client
            .open(DocumentMirror {
                uri: uri.to_owned(),
                language_id: snapshot.metadata.language_id.clone(),
                revision: 0,
                version: 0,
                text: snapshot.rope.to_string(),
            })
            .await
            .map_err(failure)?;
        0
    };
    session.documents.insert(
        snapshot.id,
        Document {
            snapshot: snapshot.clone(),
            uri: uri.to_owned(),
            protocol_revision,
        },
    );
    session.idle.acquire();
    Ok(())
}

async fn dispose_models(
    sessions: &mut HashMap<SessionKey, Session>,
    models: &mut HashMap<DocumentId, DocumentSnapshot>,
    uris: Vec<String>,
) {
    for uri in uris {
        let Ok(parsed) = uri.parse::<lsp_types::Uri>() else {
            continue;
        };
        let removed = models
            .iter()
            .filter_map(|(id, model)| {
                matches!(&model.key, DocumentKey::File(path) if path.to_str().is_some_and(|path| {
                raw_diagnostics::matches(&taide_lsp::service::workspace_folder_uri(path), &parsed)
            })).then_some(*id)
            })
            .collect::<HashSet<_>>();
        models.retain(|id, _| !removed.contains(id));
        let documents = sessions
            .values_mut()
            .flat_map(|session| {
                session.raw_diagnostics.remove(&uri);
                session.marker_documents.retain(|id, model_uri| {
                    !removed.contains(id) && !raw_diagnostics::matches(model_uri, &parsed)
                });
                session
                    .documents
                    .iter()
                    .filter_map(|(id, document)| {
                        raw_diagnostics::matches(&document.uri, &parsed).then_some(*id)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<_>>();
        for document in documents {
            close_document(sessions, document, None, Some(&parsed)).await;
        }
    }
}

async fn close_document(
    sessions: &mut HashMap<SessionKey, Session>,
    document: DocumentId,
    retain: Option<&HashSet<SessionKey>>,
    uri: Option<&lsp_types::Uri>,
) {
    let keys = sessions
        .keys()
        .filter(|key| retain.is_none_or(|retained| !retained.contains(*key)))
        .cloned()
        .collect::<Vec<_>>();
    for key in keys {
        if let Some(session) = sessions.get_mut(&key) {
            if uri.is_some_and(|uri| {
                session
                    .documents
                    .get(&document)
                    .is_none_or(|document| !raw_diagnostics::matches(&document.uri, uri))
            }) {
                continue;
            }
            if let Some(document) = session.documents.remove(&document) {
                let _ = session.client.close(document.uri).await;
            }
            if session.documents.is_empty() {
                session.idle.release(tokio::time::Instant::now());
            }
        }
    }
}

fn failure(error: Failure) -> AppError {
    AppError::Internal(format!("native language server: {error:?}"))
}

async fn notice(
    sessions: &mut HashMap<SessionKey, Session>,
    models: &HashMap<DocumentId, DocumentSnapshot>,
    key: SessionKey,
    notice: SessionNotice,
    services: &Arc<AppServices>,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> Option<Reply> {
    let session = sessions.get_mut(&key)?;
    if notice.generation != session.client.snapshot().generation {
        return None;
    }
    match notice.kind {
        SessionNoticeKind::Request(request) => {
            if let ServerRequestKind::ApplyEdit(params) = request.kind.as_ref() {
                let documents = protocol_documents(session);
                let client = session.client.clone();
                let ticket = request.ticket;
                let edit = params.edit.clone();
                let services_for_edit = services.clone();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let roots = session.roots.clone();
                if !services
                    .tasks
                    .spawn_transient("native-lsp-apply-edit-reply", async move {
                        let applied = crate::lsp_workspace_worker::apply(
                            &services_for_edit,
                            edit,
                            documents,
                            Some(roots),
                            &sender,
                            &repaint,
                        )
                        .await
                        .is_ok();
                        let _result = client
                            .reply(
                                ticket,
                                ServerReply::ApplyEdit(lsp_types::ApplyWorkspaceEditResponse {
                                    applied,
                                    failure_reason: (!applied).then(|| "edit rejected".into()),
                                    failed_change: None,
                                }),
                            )
                            .await;
                    })
                {
                    return None;
                }
                return None;
            }
            let response = match request.kind.as_ref() {
                ServerRequestKind::Configuration(params) => {
                    ServerReply::Configuration(vec![Value::Null; params.items.len()])
                }
                ServerRequestKind::WorkspaceFolders => {
                    ServerReply::WorkspaceFolders(Some(workspace_folders(session).ok()?))
                }
                ServerRequestKind::CreateProgress(_) => ServerReply::Acknowledged,
                _ => ServerReply::Rejected(Rejection::Unavailable),
            };
            if let Err(error) = session.client.reply(request.ticket, response).await {
                return Some(Reply::Failed {
                    document: None,
                    error: failure(error),
                });
            }
        }
        SessionNoticeKind::Notification(ServerNotification::Diagnostics(diagnostics)) => {
            session
                .raw_diagnostics
                .publish(&diagnostics.uri, &diagnostics.diagnostics);
            let document = session
                .documents
                .values()
                .find(|document| raw_diagnostics::matches(&document.uri, &diagnostics.uri));
            if document.is_some_and(|document| {
                diagnostics.version.is_some_and(|version| {
                    u64::try_from(version).ok() != Some(document.protocol_revision)
                })
            }) {
                return None;
            }
            let model = models.values().find(|model| {
                matches!(&model.key, DocumentKey::File(path) if path.to_str().is_some_and(|path| {
                    raw_diagnostics::matches(&taide_lsp::service::workspace_folder_uri(path), &diagnostics.uri)
                }))
            })?;
            session
                .marker_documents
                .insert(model.id, diagnostics.uri.as_str().to_owned());
            return Some(Reply::Diagnostics {
                owner: session.owner,
                document: model.id,
                revision: document.map_or(model.revision, |document| document.snapshot.revision),
                diagnostics,
            });
        }
        SessionNoticeKind::Notification(_) => {}
    }
    None
}

async fn format(
    sessions: &HashMap<SessionKey, Session>,
    snapshot: &DocumentSnapshot,
    options: lsp_types::FormattingOptions,
) -> Result<Option<Vec<lsp_types::TextEdit>>, Failure> {
    let mut candidates = sessions
        .iter()
        .filter_map(|(key, session)| {
            session
                .documents
                .get(&snapshot.id)
                .map(|document| (key, session, document))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.server.cmp(&right.0.server));
    for (_, session, document) in candidates {
        if document.snapshot.revision != snapshot.revision {
            return Err(Failure::StaleRevision);
        }
        let mut client = session.client.clone();
        if client.snapshot().phase != Phase::Running {
            client.wait_for_phase(Phase::Running).await?;
        }
        let params = lsp_types::DocumentFormattingParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: document
                    .uri
                    .parse()
                    .map_err(|_| Failure::MalformedRequest)?,
            },
            options: options.clone(),
            work_done_progress_params: Default::default(),
        };
        match client
            .request_typed::<lsp_types::request::Formatting>(
                params,
                Some((document.uri.clone(), document.protocol_revision)),
            )
            .await
        {
            Ok(reply) => return Ok(reply.value),
            Err(Failure::UnsupportedCapability) => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

fn protocol_documents(session: &Session) -> Vec<ProtocolDocument> {
    session
        .documents
        .values()
        .map(|document| ProtocolDocument {
            snapshot: document.snapshot.clone(),
            uri: document.uri.clone(),
            revision: Some(document.protocol_revision),
        })
        .collect()
}

async fn bindings(
    commands: &mpsc::Sender<Command>,
    document: DocumentId,
) -> Result<HashMap<SessionKey, Session>, Failure> {
    let (completion, receive) = oneshot::channel();
    commands
        .send(Command::Bindings {
            document,
            completion,
        })
        .await
        .map_err(|_| Failure::TransportClosed)?;
    receive.await.map_err(|_| Failure::TransportClosed)
}

async fn apply_action_edit(
    services: &Arc<AppServices>,
    session: &Session,
    edit: lsp_types::WorkspaceEdit,
    replies: &mpsc::Sender<Reply>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) -> Result<bool, Failure> {
    Ok(crate::lsp_workspace_worker::apply(
        services,
        edit,
        protocol_documents(session),
        None,
        replies,
        repaint,
    )
    .await
    .is_ok())
}

async fn execute(client: &SessionClient, command: lsp_types::Command) -> Result<(), Failure> {
    client
        .request_typed::<lsp_types::request::ExecuteCommand>(
            lsp_types::ExecuteCommandParams {
                command: command.command,
                arguments: command.arguments.unwrap_or_default(),
                work_done_progress_params: Default::default(),
            },
            None,
        )
        .await
        .map(|_| ())
}

async fn save_actions(
    services: Arc<AppServices>,
    document: DocumentId,
    actions: SaveActionFlags,
    commands: mpsc::Sender<Command>,
    replies: mpsc::Sender<Reply>,
    repaint: Arc<dyn Fn() + Send + Sync>,
) -> Result<(), Failure> {
    let mut keys = bindings(&commands, document)
        .await?
        .into_keys()
        .collect::<Vec<_>>();
    keys.sort_by(|left, right| left.server.cmp(&right.server));
    let mut kinds = Vec::new();
    if actions.fix_all {
        kinds.push(lsp_types::CodeActionKind::SOURCE_FIX_ALL);
    }
    if actions.organize_imports {
        kinds.push(lsp_types::CodeActionKind::SOURCE_ORGANIZE_IMPORTS);
    }
    for key in keys {
        for kind in &kinds {
            let mut selected = bindings(&commands, document).await?;
            let Some(mut session) = selected.remove(&key) else {
                continue;
            };
            let Some(current) = session.documents.get(&document) else {
                continue;
            };
            let mut client = session.client.clone();
            if client.snapshot().phase != Phase::Running
                && client.wait_for_phase(Phase::Running).await.is_err()
            {
                continue;
            }
            let end = taide_native_editor::lsp::byte_to_position(
                &current.snapshot,
                current.snapshot.rope.len_bytes(),
            )
            .map_err(|_| Failure::MalformedRequest)?;
            let params = lsp_types::CodeActionParams {
                text_document: lsp_types::TextDocumentIdentifier {
                    uri: current.uri.parse().map_err(|_| Failure::MalformedRequest)?,
                },
                range: lsp_types::Range {
                    start: lsp_types::Position::default(),
                    end,
                },
                context: lsp_types::CodeActionContext {
                    diagnostics: session.raw_diagnostics.get(&current.uri).to_vec(),
                    only: Some(vec![kind.clone()]),
                    trigger_kind: Some(lsp_types::CodeActionTriggerKind::AUTOMATIC),
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            let Ok(response) = client
                .request_typed::<lsp_types::request::CodeActionRequest>(
                    params,
                    Some((current.uri.clone(), current.protocol_revision)),
                )
                .await
            else {
                continue;
            };
            for action in response.value.unwrap_or_default() {
                match action {
                    lsp_types::CodeActionOrCommand::Command(command) => {
                        let _result = execute(&client, command).await;
                    }
                    lsp_types::CodeActionOrCommand::CodeAction(mut action) => {
                        if action.kind.as_ref().is_some_and(|actual| {
                            actual.as_str() != kind.as_str()
                                && !actual
                                    .as_str()
                                    .strip_prefix(kind.as_str())
                                    .is_some_and(|suffix| suffix.starts_with('.'))
                        }) {
                            continue;
                        }
                        if action.edit.is_none()
                            && let Ok(resolved) = client
                                .request_typed::<lsp_types::request::CodeActionResolveRequest>(
                                    action.clone(),
                                    session.documents.get(&document).map(|document| {
                                        (document.uri.clone(), document.protocol_revision)
                                    }),
                                )
                                .await
                        {
                            action = resolved.value;
                        }
                        if let Some(edit) = action.edit {
                            if !apply_action_edit(&services, &session, edit, &replies, &repaint)
                                .await?
                            {
                                continue;
                            }
                            if let Some(current) = bindings(&commands, document).await?.remove(&key)
                            {
                                session = current;
                            }
                        }
                        if let Some(command) = action.command {
                            let _result = execute(&client, command).await;
                        }
                    }
                }
                if let Some(current) = bindings(&commands, document).await?.remove(&key) {
                    session = current;
                }
            }
        }
    }
    Ok(())
}

async fn run(
    services: Arc<AppServices>,
    path_var: OsString,
    commands: mpsc::Sender<Command>,
    mut receiver: mpsc::Receiver<Command>,
    publishers: Publishers,
    mut stopping: watch::Receiver<bool>,
    repaint: Arc<dyn Fn() + Send + Sync>,
) {
    let Publishers { replies, states } = publishers;
    let mut sessions = HashMap::<SessionKey, Session>::new();
    let mut models = HashMap::<DocumentId, DocumentSnapshot>::new();
    loop {
        let deadline = sessions
            .values()
            .filter_map(|session| session.idle.deadline())
            .min();
        let command = tokio::select! {
            biased;
            _ = stopping.changed() => break,
            _ = idle::wait(deadline) => {
                let expired = sessions.iter().filter(|(_, session)| {
                    session.idle.ready(tokio::time::Instant::now())
                }).map(|(key, _)| key.clone()).collect::<Vec<_>>();
                for key in expired {
                    if let Some(session) = sessions.remove(&key) {
                        let _ = session.client.stop().await;
                    }
                }
                publish_status(&sessions, &states, &repaint);
                continue;
            },
            command = receiver.recv() => command,
        };
        let Some(command) = command else {
            break;
        };
        let reply = match command {
            Command::Completion(request) => {
                if request.is_cancelled() {
                    continue;
                }
                let selected = sessions
                    .iter()
                    .filter(|(key, session)| {
                        key.project == request.project
                            && session.documents.contains_key(&request.snapshot.id)
                    })
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect::<HashMap<_, _>>();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let mut cancelled = request.cancelled.clone();
                let mut stopping = stopping.clone();
                let rejected = request.clone();
                if !services
                    .tasks
                    .spawn_transient("native-lsp-editor-completion", async move {
                        let result = tokio::select! {
                            biased;
                            _ = stopping.changed() => return,
                            _ = cancelled.changed() => return,
                            result = editor_completion::request(&selected, &request) => result,
                        };
                        if !request.is_cancelled()
                            && sender
                                .send(Reply::Completion { request, result })
                                .await
                                .is_ok()
                        {
                            repaint();
                        }
                    })
                {
                    Some(Reply::Completion {
                        request: rejected,
                        result: Err(Failure::TransportClosed),
                    })
                } else {
                    None
                }
            }
            Command::Documentation(request) => {
                if request.is_cancelled() {
                    continue;
                }
                let selected = sessions
                    .iter()
                    .filter(|(key, session)| {
                        key.project == request.project
                            && session.documents.contains_key(&request.snapshot.id)
                    })
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect::<HashMap<_, _>>();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let mut cancelled = request.cancelled.clone();
                let mut stopping = stopping.clone();
                let rejected = request.clone();
                if !services.tasks.spawn_transient("native-lsp-editor-documentation", async move {
                    let result = tokio::select! {
                        biased;
                        _ = stopping.changed() => return,
                        _ = cancelled.changed() => return,
                        result = editor_documentation::request(&selected, &request, &sender, &repaint) => result,
                    };
                    if !request.is_cancelled()
                        && sender.send(Reply::Documentation { request, result }).await.is_ok()
                    {
                        repaint();
                    }
                }) {
                    Some(Reply::Documentation { request: rejected, result: Err(Failure::TransportClosed) })
                } else {
                    None
                }
            }
            Command::CloseBinding(project, document) => {
                let retained = sessions
                    .keys()
                    .filter(|key| key.project != project)
                    .cloned()
                    .collect::<HashSet<_>>();
                close_document(&mut sessions, document, Some(&retained), None).await;
                publish_status(&sessions, &states, &repaint);
                None
            }
            Command::WorkspaceSymbols(request) => {
                if request.is_cancelled() {
                    continue;
                }
                let selected = workspace_symbols::select(&sessions, &request.project);
                let sender = replies.clone();
                let repaint = repaint.clone();
                let mut cancelled = request.cancelled.clone();
                let mut stopping = stopping.clone();
                let rejected = request.clone();
                if !services
                    .tasks
                    .spawn_transient("native-lsp-workspace-symbols", async move {
                        let result = tokio::select! {
                            biased;
                            _ = stopping.changed() => return,
                            _ = cancelled.changed() => return,
                            result = workspace_symbols::request(&selected, &request) => result,
                        };
                        if !request.is_cancelled()
                            && sender
                                .send(Reply::WorkspaceSymbols { request, result })
                                .await
                                .is_ok()
                        {
                            repaint();
                        }
                    })
                {
                    Some(Reply::WorkspaceSymbols {
                        request: rejected,
                        result: Err(Failure::TransportClosed),
                    })
                } else {
                    None
                }
            }
            Command::SymbolLocations(request) => {
                if request.is_cancelled() {
                    continue;
                }
                let selected = sessions
                    .iter()
                    .filter(|(key, session)| {
                        key.project == request.project
                            && session.documents.contains_key(&request.snapshot.id)
                    })
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect::<HashMap<_, _>>();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let mut cancelled = request.cancelled.clone();
                let mut stopping = stopping.clone();
                let rejected = request.clone();
                if !services
                    .tasks
                    .spawn_transient("native-lsp-symbol-locations", async move {
                        let result = tokio::select! {
                            biased;
                            _ = stopping.changed() => return,
                            _ = cancelled.changed() => return,
                            result = symbol_locations::request(&selected, &request) => result,
                        };
                        if !request.is_cancelled()
                            && sender
                                .send(Reply::SymbolLocations { request, result })
                                .await
                                .is_ok()
                        {
                            repaint();
                        }
                    })
                {
                    Some(Reply::SymbolLocations {
                        request: rejected,
                        result: Err(Failure::TransportClosed),
                    })
                } else {
                    None
                }
            }
            Command::SyntaxFolding(request) => {
                if request.is_cancelled() {
                    continue;
                }
                let selected = sessions
                    .iter()
                    .filter(|(key, session)| {
                        key.project == request.project
                            && session.documents.contains_key(&request.snapshot.id)
                    })
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect::<HashMap<_, _>>();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let mut cancelled = request.cancelled.clone();
                let mut stopping = stopping.clone();
                let rejected = request.clone();
                if !services
                    .tasks
                    .spawn_transient("native-lsp-syntax-folding", async move {
                        let result = tokio::select! {
                            biased;
                            _ = stopping.changed() => return,
                            _ = cancelled.changed() => return,
                            result = syntax_folding::request(&selected, &request) => result,
                        };
                        if !request.is_cancelled()
                            && sender
                                .send(Reply::SyntaxFolding { request, result })
                                .await
                                .is_ok()
                        {
                            repaint();
                        }
                    })
                {
                    Some(Reply::SyntaxFolding {
                        request: rejected,
                        result: Err(Failure::TransportClosed),
                    })
                } else {
                    None
                }
            }
            Command::DocumentSymbols(request) => {
                if request.is_cancelled() {
                    continue;
                }
                let selected = sessions
                    .iter()
                    .filter(|(key, session)| {
                        key.project == request.project
                            && session.documents.contains_key(&request.snapshot.id)
                    })
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect::<HashMap<_, _>>();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let mut cancelled = request.cancelled.clone();
                let mut stopping = stopping.clone();
                let rejected = request.clone();
                if !services
                    .tasks
                    .spawn_transient("native-lsp-document-symbols", async move {
                        let result = tokio::select! {
                            biased;
                            _ = stopping.changed() => return,
                            _ = cancelled.changed() => return,
                            result = document_symbols::request(&selected, &request) => result,
                        };
                        if !request.is_cancelled()
                            && sender
                                .send(Reply::DocumentSymbols { request, result })
                                .await
                                .is_ok()
                        {
                            repaint();
                        }
                    })
                {
                    Some(Reply::DocumentSymbols {
                        request: rejected,
                        result: Err(Failure::TransportClosed),
                    })
                } else {
                    None
                }
            }
            Command::StateChanged { key, source, state } => {
                publish_status(&sessions, &states, &repaint);
                if state.failure == Some(Failure::ReinitializeExhausted)
                    && sessions.get(&key).is_some_and(|session| {
                        let current = session.client.snapshot();
                        source.same_channel(&session.client.subscribe())
                            && current.generation == state.generation
                            && current.phase == state.phase
                            && current.failure == state.failure
                    })
                {
                    if let Some(session) = sessions.remove(&key) {
                        let _ = session.client.stop().await;
                    }
                    publish_status(&sessions, &states, &repaint);
                    if replies
                        .send(Reply::Failed {
                            document: None,
                            error: failure(Failure::ReinitializeExhausted),
                        })
                        .await
                        .is_err()
                    {
                        break;
                    }
                    repaint();
                    continue;
                }
                if let Some(session) = sessions.get(&key)
                    && source.same_channel(&session.client.subscribe())
                    && (state.generation > 0 || state.phase == Phase::Degraded)
                {
                    let current = session.client.snapshot();
                    if current.generation != state.generation || current.phase != state.phase {
                        continue;
                    }
                    for document in session.documents.values() {
                        if replies
                            .send(Reply::Synced {
                                document: document.snapshot.id,
                                revision: document.snapshot.revision,
                                sessions: vec![current.clone()],
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                        repaint();
                    }
                }
                None
            }
            Command::ExplorerPaste { project, request } => {
                let work_services = services.clone();
                let sender = replies.clone();
                let work_repaint = repaint.clone();
                let rejected_project = project.clone();
                let rejected_request = request.clone();
                if services
                    .tasks
                    .spawn_transient("native-explorer-paste", async move {
                        let event = crate::explorer_clipboard::apply(
                            &work_services,
                            project,
                            request,
                            &sender,
                            &work_repaint,
                        )
                        .await;
                        if sender.send(Reply::ExplorerPasted(event)).await.is_ok() {
                            work_repaint();
                        }
                    })
                {
                    None
                } else {
                    Some(Reply::ExplorerPasted(crate::explorer_clipboard::Reply {
                        project: rejected_project,
                        request: rejected_request,
                        clear_cut: false,
                        result: Err(AppError::Forbidden(
                            "native explorer paste is stopping".into(),
                        )),
                    }))
                }
            }
            Command::ExplorerDelete(request) => {
                let work_services = services.clone();
                let sender = replies.clone();
                let work_repaint = repaint.clone();
                let rejected = request.clone();
                if services
                    .tasks
                    .spawn_transient("native-explorer-delete", async move {
                        let result = crate::explorer_delete::apply(
                            &work_services,
                            request.clone(),
                            &sender,
                            &work_repaint,
                        )
                        .await;
                        if sender
                            .send(Reply::ExplorerDeleteFinished { request, result })
                            .await
                            .is_ok()
                        {
                            work_repaint();
                        }
                    })
                {
                    None
                } else {
                    Some(Reply::ExplorerDeleteFinished {
                        request: rejected,
                        result: Err(AppError::Forbidden(
                            "native explorer deletion is stopping".into(),
                        )),
                    })
                }
            }
            Command::ExplorerCreate { project, request } => {
                let work_services = services.clone();
                let sender = replies.clone();
                let work_repaint = repaint.clone();
                let rejected_project = project.clone();
                let rejected_request = request.clone();
                if services
                    .tasks
                    .spawn_transient("native-explorer-create", async move {
                        let result =
                            crate::explorer::create_entry(&work_services, &project, &request).await;
                        if sender
                            .send(Reply::ExplorerCreated(crate::explorer::CreateReply {
                                project,
                                request,
                                result,
                            }))
                            .await
                            .is_ok()
                        {
                            work_repaint();
                        }
                    })
                {
                    None
                } else {
                    Some(Reply::ExplorerCreated(crate::explorer::CreateReply {
                        project: rejected_project,
                        request: rejected_request,
                        result: Err(AppError::Forbidden(
                            "native explorer creation is stopping".into(),
                        )),
                    }))
                }
            }
            Command::ExplorerRename { project, request } => {
                let work_services = services.clone();
                let sender = replies.clone();
                let work_repaint = repaint.clone();
                let rejected_project = project.clone();
                let rejected_request = request.clone();
                if services
                    .tasks
                    .spawn_transient("native-explorer-rename", async move {
                        let result = crate::explorer::rename_entry(
                            &work_services,
                            &project,
                            &request,
                            &sender,
                            &work_repaint,
                        )
                        .await;
                        if sender
                            .send(Reply::ExplorerRenamed(crate::explorer::RenameReply {
                                project,
                                request,
                                result,
                            }))
                            .await
                            .is_ok()
                        {
                            work_repaint();
                        }
                    })
                {
                    None
                } else {
                    Some(Reply::ExplorerRenamed(crate::explorer::RenameReply {
                        project: rejected_project,
                        request: rejected_request,
                        result: Err(AppError::Forbidden(
                            "native explorer rename is stopping".into(),
                        )),
                    }))
                }
            }
            Command::Sync { project, snapshot } => {
                let document = snapshot.id;
                let revision = snapshot.revision;
                models.insert(document, snapshot.clone());
                match sync_document(
                    &services,
                    &mut sessions,
                    &commands,
                    project,
                    snapshot,
                    path_var.clone(),
                )
                .await
                {
                    Ok(sessions) => Some(Reply::Synced {
                        document,
                        revision,
                        sessions,
                    }),
                    Err(error) => {
                        for session in sessions
                            .values_mut()
                            .filter(|session| session.documents.is_empty())
                        {
                            session.idle.release(tokio::time::Instant::now());
                        }
                        Some(Reply::Failed {
                            document: Some(document),
                            error,
                        })
                    }
                }
            }
            Command::Close(document) => {
                close_document(&mut sessions, document, None, None).await;
                None
            }
            Command::DisposeModels(uris) => {
                dispose_models(&mut sessions, &mut models, uris).await;
                None
            }
            Command::Models { changed, removed } => {
                models.retain(|id, _| !removed.contains(id));
                models.extend(changed.into_iter().map(|model| (model.id, model)));
                for session in sessions.values_mut() {
                    session
                        .marker_documents
                        .retain(|id, _| !removed.contains(id));
                }
                None
            }
            Command::RetainProjects(projects) => {
                let closed = sessions
                    .keys()
                    .filter(|key| !projects.contains(&key.project))
                    .cloned()
                    .collect::<Vec<_>>();
                for key in closed {
                    if let Some(session) = sessions.remove(&key) {
                        let _ = session.client.stop().await;
                    }
                }
                None
            }
            Command::Saved(document) => {
                let mut error = None;
                for session in sessions
                    .values()
                    .filter(|session| session.documents.contains_key(&document))
                {
                    if session.client.snapshot().phase != Phase::Running {
                        let mut client = session.client.clone();
                        let uri = session.documents[&document].uri.clone();
                        let sender = replies.clone();
                        let repaint = repaint.clone();
                        if !services.tasks.spawn_transient(
                            "native-lsp-save-notification",
                            async move {
                                let result = async {
                                    client.wait_for_phase(Phase::Running).await?;
                                    client.saved(uri).await
                                }
                                .await;
                                if let Err(error) = result
                                    && sender
                                        .send(Reply::Failed {
                                            document: Some(document),
                                            error: failure(error),
                                        })
                                        .await
                                        .is_ok()
                                {
                                    repaint();
                                }
                            },
                        ) {
                            error = Some(failure(Failure::TransportClosed));
                        }
                        continue;
                    }
                    if let Err(failed) = session
                        .client
                        .saved(session.documents[&document].uri.clone())
                        .await
                    {
                        error = Some(failure(failed));
                    }
                }
                error.map(|error| Reply::Failed {
                    document: Some(document),
                    error,
                })
            }
            Command::Notice {
                key,
                source,
                notice: message,
            } => {
                if sessions
                    .get(&key)
                    .is_some_and(|session| source.same_channel(&session.client.subscribe()))
                {
                    notice(
                        &mut sessions,
                        &models,
                        key,
                        message,
                        &services,
                        &replies,
                        &repaint,
                    )
                    .await
                } else {
                    None
                }
            }
            Command::Bindings {
                document,
                completion,
            } => {
                let selected = sessions
                    .iter()
                    .filter(|(_, session)| session.documents.contains_key(&document))
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect();
                drop(completion.send(selected));
                None
            }
            Command::Format {
                snapshot,
                options,
                actions,
            } => {
                let selected = sessions
                    .iter()
                    .filter(|(_, session)| session.documents.contains_key(&snapshot.id))
                    .map(|(key, session)| (key.clone(), session.clone()))
                    .collect::<HashMap<_, _>>();
                let sender = replies.clone();
                let repaint = repaint.clone();
                let commands = commands.clone();
                let tasks = services.tasks.clone();
                let action_services = services.clone();
                let rejected = snapshot.clone();
                if services
                    .tasks
                    .spawn_transient("native-lsp-format", async move {
                        let mut selected = selected;
                        let mut snapshot = snapshot;
                        if actions.fix_all || actions.organize_imports {
                            let (complete, receive) = oneshot::channel();
                            let actions_commands = commands.clone();
                            let actions_replies = sender.clone();
                            let actions_repaint = repaint.clone();
                            let document = snapshot.id;
                            let admitted =
                                tasks.spawn_transient("native-lsp-save-actions", async move {
                                    drop(
                                        complete.send(
                                            save_actions(
                                                action_services,
                                                document,
                                                actions,
                                                actions_commands,
                                                actions_replies,
                                                actions_repaint,
                                            )
                                            .await,
                                        ),
                                    );
                                });
                            let error = if admitted {
                                match tokio::time::timeout(SAVE_ACTION_TIMEOUT, receive).await {
                                    Ok(Ok(Ok(()))) => None,
                                    Ok(Ok(Err(error))) => Some(error),
                                    Ok(Err(_)) => Some(Failure::TransportClosed),
                                    Err(_) => Some(Failure::TimedOut),
                                }
                            } else {
                                Some(Failure::TransportClosed)
                            };
                            if let Some(error) = error
                                && sender
                                    .send(Reply::Failed {
                                        document: Some(document),
                                        error: failure(error),
                                    })
                                    .await
                                    .is_ok()
                            {
                                repaint();
                            }
                            if let Ok(latest) = bindings(&commands, document).await {
                                if let Some(current) = latest
                                    .values()
                                    .find_map(|session| session.documents.get(&document))
                                {
                                    snapshot = current.snapshot.clone();
                                }
                                selected = latest;
                            }
                        }
                        let result = match options {
                            Some(options) => format(&selected, &snapshot, options).await,
                            None => Ok(None),
                        };
                        if sender
                            .send(Reply::Formatted { snapshot, result })
                            .await
                            .is_ok()
                        {
                            repaint();
                        }
                    })
                {
                    None
                } else {
                    Some(Reply::Formatted {
                        snapshot: rejected,
                        result: Err(Failure::TransportClosed),
                    })
                }
            }
        };
        publish_status(&sessions, &states, &repaint);
        if let Some(reply) = reply {
            if replies.send(reply).await.is_err() {
                break;
            }
            repaint();
        }
    }
    for session in sessions.into_values() {
        let _ = session.client.stop().await;
    }
    states.send_replace(Vec::new());
    repaint();
}

fn publish_status(
    sessions: &HashMap<SessionKey, Session>,
    states: &watch::Sender<Vec<RegistryState>>,
    repaint: &Arc<dyn Fn() + Send + Sync>,
) {
    states.send_replace(
        sessions
            .iter()
            .map(|(key, session)| RegistryState {
                project: key.project.clone(),
                name: session.name.clone(),
                snapshot: session.client.snapshot(),
                owner: session.owner,
                documents: session
                    .marker_documents
                    .keys()
                    .copied()
                    .chain(session.documents.keys().copied())
                    .collect(),
                open_documents: session.documents.keys().copied().collect(),
            })
            .collect(),
    );
    repaint();
}

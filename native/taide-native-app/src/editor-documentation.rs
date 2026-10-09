use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::documentation::{Content, HoverPart, RichDocument, Signatures};
use taide_native_editor::lsp::{LspRange, Position, byte_to_position};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;
pub(crate) use taide_native_editor::documentation::Kind;

pub(crate) struct Context {
    pub project: ProjectId,
    pub source: ViewId,
    pub owner: ViewId,
    pub kind: Kind,
    pub byte: usize,
    pub fallback: Range<usize>,
    pub viewport: eframe::egui::ViewportId,
    pub keyboard: bool,
}

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) source: ViewId,
    pub(crate) source_key: ViewKey,
    pub(crate) owner: ViewId,
    pub(crate) owner_key: ViewKey,
    pub(crate) owner_document: DocumentId,
    pub(crate) selection: SelectionSet,
    pub(crate) kind: Kind,
    pub(crate) byte: usize,
    pub(crate) position: Position,
    pub(crate) fallback: LspRange,
    pub(crate) token: Uuid,
    pub(crate) cancelled: watch::Receiver<bool>,
    pub(crate) viewport: eframe::egui::ViewportId,
    pub(crate) keyboard: bool,
}

impl Request {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }

    pub(crate) fn describes(&self, store: &EditorStore) -> bool {
        !self.is_cancelled()
            && store.views().get(self.owner).is_some_and(|owner| {
                owner.key == self.owner_key && owner.document == self.owner_document
            })
            && store.views().get(self.source).is_some_and(|source| {
                source.key == self.source_key
                    && source.document == self.snapshot.id
                    && source.selection == self.selection
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|snapshot| {
                            snapshot.key == self.snapshot.key
                                && snapshot.revision == self.snapshot.revision
                                && snapshot.metadata.language_id
                                    == self.snapshot.metadata.language_id
                        })
            })
    }

    pub(crate) fn is_active(
        &self,
        layout: &taide_model::layout::ProjectLayout,
        scope: &taide_native_ui::shell::WindowScope,
    ) -> bool {
        crate::symbol_sidebar::window_tree(&self.project, layout, scope).is_some_and(|(root, _)| {
            taide_native_ui::snapshot::active_tab(root, &self.owner_key.pane)
                .is_some_and(|tab| tab.id == self.owner_key.tab)
        })
    }
}

pub struct HoverGroup {
    pub provider: ProviderIdentity,
    pub part: HoverPart,
}

pub struct SignatureGroup {
    pub provider: ProviderIdentity,
    pub help: lsp_types::SignatureHelp,
}

pub enum Response {
    Hover {
        part: Option<(usize, HoverGroup)>,
        complete: bool,
    },
    Signature(Option<SignatureGroup>),
}

#[derive(Clone)]
pub(crate) struct PreparedHover {
    pub range: LspRange,
    pub documents: Vec<Arc<RichDocument>>,
}

#[derive(Clone)]
pub(crate) struct PreparedSignature {
    pub model: Arc<Signatures>,
    pub documentation: Vec<Option<Arc<RichDocument>>>,
    pub parameters: Vec<Vec<Option<Arc<RichDocument>>>>,
}

impl PreparedSignature {
    fn new(help: lsp_types::SignatureHelp) -> Option<Self> {
        let model = Signatures::new(help)?;
        let documentation = model
            .value()
            .signatures
            .iter()
            .map(|signature| prepare_docs(signature.documentation.as_ref()))
            .collect();
        let parameters = model
            .value()
            .signatures
            .iter()
            .map(|signature| {
                signature
                    .parameters
                    .iter()
                    .flatten()
                    .map(|parameter| prepare_docs(parameter.documentation.as_ref()))
                    .collect()
            })
            .collect();
        Some(Self {
            model: Arc::new(model),
            documentation,
            parameters,
        })
    }

    pub(crate) fn parameter_documentation(&self) -> Option<&Arc<RichDocument>> {
        let index = self
            .model
            .active()
            .active_parameter
            .or(self.model.value().active_parameter)
            .unwrap_or_default() as usize;
        self.parameters
            .get(self.model.index())?
            .get(index)?
            .as_ref()
    }
}

fn prepare_docs(documentation: Option<&lsp_types::Documentation>) -> Option<Arc<RichDocument>> {
    let content = Content::from(documentation?.clone());
    if content.is_empty() {
        return None;
    }
    Some(Arc::new(crate::editor_markup::parse(&content)))
}

pub(crate) enum Payload {
    Hover(Arc<Vec<Arc<PreparedHover>>>),
    Signature(Arc<PreparedSignature>),
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    cancel: watch::Sender<bool>,
    complete: bool,
    payload: Option<Payload>,
    hovers: BTreeMap<usize, Arc<PreparedHover>>,
}

impl Entry {
    fn documents(&self) -> Vec<&RichDocument> {
        match &self.payload {
            Some(Payload::Hover(parts)) => parts
                .iter()
                .flat_map(|part| part.documents.iter().map(Arc::as_ref))
                .collect(),
            Some(Payload::Signature(signature)) => signature
                .parameter_documentation()
                .into_iter()
                .chain(
                    signature
                        .documentation
                        .get(signature.model.index())
                        .and_then(Option::as_ref),
                )
                .map(Arc::as_ref)
                .collect(),
            None => Vec::new(),
        }
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<(ViewId, Kind), Entry>,
    code: crate::editor_documentation_code::Cache,
    images: crate::editor_documentation_images::Cache,
}

impl State {
    pub(crate) fn clear(
        &mut self,
        store: &mut EditorStore,
        syntax: &mut crate::editor_syntax::EditorSyntax,
    ) -> Result<(), EditorError> {
        self.entries.clear();
        self.images = crate::editor_documentation_images::Cache::default();
        self.code.prepare(store, syntax, std::iter::empty(), &[])
    }

    pub(crate) fn prepare_images(
        &mut self,
        store: &EditorStore,
        context: &eframe::egui::Context,
        services: &taide_runtime::AppServices,
    ) {
        let documents = self
            .entries
            .values()
            .filter(|entry| entry.request.describes(store))
            .flat_map(|entry| {
                entry
                    .documents()
                    .into_iter()
                    .map(|document| (&entry.request.project, document))
            })
            .collect::<Vec<_>>();
        self.images
            .prepare(context, services, documents.into_iter());
    }

    pub(crate) fn prepare_code(
        &mut self,
        store: &mut EditorStore,
        syntax: &mut crate::editor_syntax::EditorSyntax,
        plugins: &[taide_model::plugin::LoadedPlugin],
    ) -> Result<(), EditorError> {
        let documents = self
            .entries
            .values()
            .filter(|entry| entry.request.describes(store))
            .flat_map(|entry| {
                entry.documents().into_iter().map(|document| {
                    (
                        document,
                        entry.request.snapshot.metadata.language_id.as_str(),
                    )
                })
            })
            .collect::<Vec<_>>();
        self.code
            .prepare(store, syntax, documents.into_iter(), plugins)
    }

    pub(crate) fn begin(
        &mut self,
        store: &EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
    ) -> Result<Request, EditorError> {
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?;
        let owner = store
            .views()
            .get(context.owner)
            .ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        let position = byte_to_position(&snapshot, context.byte)?;
        if context.fallback.start > context.fallback.end {
            return Err(EditorError::InvalidBoundary);
        }
        let fallback = LspRange::new(
            byte_to_position(&snapshot, context.fallback.start)?,
            byte_to_position(&snapshot, context.fallback.end)?,
        );
        let (cancel, cancelled) = watch::channel(false);
        let request = Request {
            project: context.project,
            snapshot,
            source: context.source,
            source_key: source.key.clone(),
            owner: context.owner,
            owner_key: owner.key.clone(),
            owner_document: owner.document,
            selection: source.selection.clone(),
            kind: context.kind,
            byte: context.byte,
            position,
            fallback,
            token: Uuid::new_v4(),
            cancelled,
            viewport: context.viewport,
            keyboard: context.keyboard,
        };
        let mut previous = self.entries.remove(&(request.source, request.kind));
        let payload = previous.as_mut().and_then(|entry| {
            (request.kind == Kind::Signature
                && entry.providers == providers
                && entry.request.snapshot.id == request.snapshot.id
                && entry.request.snapshot.key == request.snapshot.key
                && entry.request.snapshot.metadata.language_id
                    == request.snapshot.metadata.language_id
                && entry.request.owner == request.owner
                && entry.request.owner_key == request.owner_key)
                .then(|| entry.payload.take())
                .flatten()
        });
        self.entries.insert(
            (request.source, request.kind),
            Entry {
                request: request.clone(),
                providers,
                cancel,
                complete: false,
                payload,
                hovers: BTreeMap::new(),
            },
        );
        Ok(request)
    }

    pub(crate) fn is_current(&self, request: &Request) -> bool {
        !request.is_cancelled()
            && self
                .entries
                .get(&(request.source, request.kind))
                .is_some_and(|entry| entry.request.token == request.token)
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> Result<bool, Failure> {
        if !self.is_current(request) || !request.describes(store) {
            return Ok(false);
        }
        let entry = self
            .entries
            .get_mut(&(request.source, request.kind))
            .unwrap();
        if entry.providers != providers {
            self.entries.remove(&(request.source, request.kind));
            return Ok(false);
        }
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                entry.complete = true;
                entry.payload = None;
                return Err(error);
            }
        };
        entry.payload = match result {
            Response::Hover { part, complete } if request.kind == Kind::Hover => {
                entry.complete = complete;
                if let Some((ordinal, group)) = part
                    && providers.contains(&group.provider)
                    && taide_native_editor::lsp::range_to_bytes(&request.snapshot, group.part.range)
                        .is_ok_and(|range| {
                            byte_to_position(&request.snapshot, range.start).ok()
                                == Some(group.part.range.start)
                                && byte_to_position(&request.snapshot, range.end).ok()
                                    == Some(group.part.range.end)
                        })
                {
                    entry.hovers.insert(
                        ordinal,
                        Arc::new(PreparedHover {
                            range: group.part.range,
                            documents: group
                                .part
                                .contents
                                .iter()
                                .map(|content| Arc::new(crate::editor_markup::parse(content)))
                                .collect(),
                        }),
                    );
                }
                Some(Payload::Hover(Arc::new(
                    entry.hovers.values().cloned().collect(),
                )))
            }
            Response::Signature(group) if request.kind == Kind::Signature => {
                entry.complete = true;
                group
                    .filter(|group| providers.contains(&group.provider))
                    .and_then(|group| PreparedSignature::new(group.help))
                    .map(|signature| Payload::Signature(Arc::new(signature)))
            }
            _ => return Err(Failure::MalformedResponse),
        };
        Ok(true)
    }

    pub(crate) fn request(
        &self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
    ) -> Option<&Request> {
        let request = &self.entries.get(&(view, kind))?.request;
        request.describes(store).then_some(request)
    }

    pub(crate) fn payload(
        &self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
    ) -> Option<&Payload> {
        self.request(store, view, kind)?;
        self.entries.get(&(view, kind))?.payload.as_ref()
    }

    pub(crate) fn pending(&self, store: &EditorStore, view: ViewId, kind: Kind) -> bool {
        self.request(store, view, kind).is_some()
            && self
                .entries
                .get(&(view, kind))
                .is_some_and(|entry| !entry.complete)
    }

    pub(crate) fn cycle(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        forward: bool,
        cycle: bool,
    ) -> bool {
        if self.request(store, view, Kind::Signature).is_none() {
            return false;
        }
        let Some(Payload::Signature(signature)) = self
            .entries
            .get_mut(&(view, Kind::Signature))
            .and_then(|entry| entry.payload.as_mut())
        else {
            return false;
        };
        Arc::make_mut(&mut Arc::make_mut(signature).model).next(forward, cycle)
    }

    pub(crate) fn close(&mut self, view: ViewId, kind: Kind) {
        self.entries.remove(&(view, kind));
        self.images
            .retain_documents(self.entries.values().flat_map(|entry| {
                entry
                    .documents()
                    .into_iter()
                    .map(|document| (&entry.request.project, document))
            }));
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        projects: &HashSet<ProjectId>,
        mut active: impl FnMut(&Request) -> bool,
        mut providers: impl FnMut(&ProjectId, &DocumentSnapshot, Kind) -> HashSet<ProviderIdentity>,
    ) {
        self.entries.retain(|_, entry| {
            projects.contains(&entry.request.project)
                && entry.request.describes(store)
                && active(&entry.request)
                && providers(
                    &entry.request.project,
                    &entry.request.snapshot,
                    entry.request.kind,
                ) == entry.providers
        });
    }
}

pub(crate) struct Provider<'a, 'state> {
    pub state: std::rc::Rc<std::cell::RefCell<&'state mut State>>,
    pub project: Option<ProjectId>,
    pub owner: ViewId,
    pub owner_key: ViewKey,
    pub lsp: Option<&'a crate::lsp::LspBridge>,
    pub viewport: eframe::egui::ViewportId,
    pub commands: &'a mut Vec<crate::host::HostCommand>,
    pub appearance: &'a taide_native_ui::editor_surface::EditorAppearance,
    pub language: String,
}

#[derive(Clone)]
pub struct FileRequest {
    project: ProjectId,
    owner: ViewKey,
    path: String,
    line: u64,
    column: u64,
    viewport: eframe::egui::ViewportId,
}

impl FileRequest {
    pub(crate) fn new(
        project: ProjectId,
        owner: ViewKey,
        uri: &url::Url,
        viewport: eframe::egui::ViewportId,
    ) -> Option<Self> {
        if uri.scheme() != "file" {
            return None;
        }
        let mut uri = uri.clone();
        let fragment = uri.fragment().unwrap_or_default();
        let fragment = fragment.strip_prefix('L').unwrap_or(fragment);
        let line_end = fragment.bytes().take_while(u8::is_ascii_digit).count();
        let mut line = 1;
        let mut column = 1;
        if line_end != 0 {
            line = fragment[..line_end].parse::<u64>().ok()?.max(1);
            if let Some(column_text) = fragment[line_end..].strip_prefix(',') {
                let column_end = column_text.bytes().take_while(u8::is_ascii_digit).count();
                if column_end != 0 {
                    column = column_text[..column_end].parse::<u64>().ok()?.max(1);
                }
            }
        }
        let maximum = u64::from(u32::MAX) + 1;
        if line > maximum || column > maximum {
            return None;
        }
        uri.set_fragment(None);
        let path = uri.to_file_path().ok()?.to_str()?.to_owned();
        Some(Self {
            project,
            owner,
            path,
            line,
            column,
            viewport,
        })
    }

    pub(crate) async fn open(
        &self,
        services: &taide_runtime::AppServices,
    ) -> taide_model::error::AppResult<crate::terminal_tabs::OpenedFileLink> {
        use taide_model::error::AppError;
        use taide_model::layout::{Tab, TabKind};
        let _guard = services.state.begin_mutation().await;
        if services.state.is_shutting_down() {
            return Err(AppError::Forbidden(
                "documentation file link is shutting down".into(),
            ));
        }
        let projects = services.state.projects.read().clone();
        if !projects.contains_key(&self.project) {
            return Err(AppError::NotFound(
                "documentation file link project closed".into(),
            ));
        }
        let (_, path) = taide_infra::root_guard::resolve_owning_project_or_cli_opened(
            &projects,
            &services.state.cli_opened_paths.read(),
            std::path::Path::new(&self.path),
        )?;
        taide_infra::root_guard::ensure_existing_file(&path, &self.path)?;
        let mut layouts = services.state.layouts.read().clone();
        let layout = layouts
            .get_mut(&self.project)
            .filter(|layout| crate::editor_problems::view_key_is_active(layout, &self.owner))
            .ok_or_else(|| AppError::NotFound("documentation file link source changed".into()))?;
        let title = std::path::Path::new(&self.path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&self.path)
            .to_owned();
        let preview = services.state.settings.read().enable_preview_tabs;
        let tab = taide_layout::service::open_tab(
            layout,
            &self.owner.pane,
            Tab {
                id: taide_model::ids::TabId::new(),
                kind: TabKind::File {
                    path: self.path.clone(),
                },
                title,
                pinned: false,
                preview,
                dirty: false,
                view_state: None,
            },
            preview,
        )?;
        taide_layout::service::focus_pane(layout, &self.owner.pane)?;
        let updated = taide_runtime::layout_actions::finish_mutation(
            services.events.as_ref(),
            &services.state,
            &self.project,
            layout,
        );
        *services.state.layouts.write() = layouts;
        Ok(crate::terminal_tabs::OpenedFileLink {
            project: self.project.clone(),
            pane: self.owner.pane.clone(),
            tab,
            path: self.path.clone(),
            line: self.line as f64,
            column: self.column as f64,
            viewport: self.viewport,
            layout: updated,
        })
    }
}

impl taide_native_ui::editor_documentation::Provider for Provider<'_, '_> {
    fn available(&self, store: &EditorStore, view: ViewId, kind: Kind) -> bool {
        self.project
            .as_ref()
            .zip(self.lsp)
            .is_some_and(|(project, lsp)| {
                store
                    .views()
                    .get(view)
                    .and_then(|view| store.documents().snapshot(view.document).ok())
                    .is_some_and(|snapshot| {
                        !lsp.documentation_providers(project, &snapshot, kind)
                            .is_empty()
                    })
            })
    }

    fn version(&self, store: &EditorStore, view: ViewId) -> String {
        let Some((project, lsp)) = self.project.as_ref().zip(self.lsp) else {
            return String::new();
        };
        let Some(snapshot) = store
            .views()
            .get(view)
            .and_then(|view| store.documents().snapshot(view.document).ok())
        else {
            return String::new();
        };
        let mut providers = [Kind::Hover, Kind::Signature]
            .into_iter()
            .flat_map(|kind| {
                lsp.documentation_providers(project, &snapshot, kind)
                    .into_iter()
                    .map(move |provider| format!("{kind:?}/{provider:?}"))
            })
            .collect::<Vec<_>>();
        providers.sort_unstable();
        providers.join(";")
    }

    fn triggers(
        &self,
        store: &EditorStore,
        view: ViewId,
    ) -> taide_native_editor::documentation::SignatureTriggers {
        self.project
            .as_ref()
            .zip(self.lsp)
            .and_then(|(project, lsp)| {
                Some(
                    lsp.signature_triggers(
                        project,
                        &store
                            .documents()
                            .snapshot(store.views().get(view)?.document)
                            .ok()?,
                    ),
                )
            })
            .unwrap_or_default()
    }

    fn word(&self, store: &EditorStore, view: ViewId, byte: usize) -> Option<Range<usize>> {
        let snapshot = store
            .documents()
            .snapshot(store.views().get(view)?.document)
            .ok()?;
        let position = byte_to_position(&snapshot, byte).ok()?;
        let line =
            taide_native_editor::editing::line_content_range(&snapshot, position.line as usize);
        let text = snapshot.rope.byte_slice(line.clone()).to_string();
        let rules = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
            .or_else(|| crate::editor_syntax::language_rules("plaintext"))?;
        let word = rules.word_range(&text, byte - line.start)?;
        Some(line.start + word.start..line.start + word.end)
    }

    fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
        byte: usize,
        keyboard: bool,
    ) -> Result<bool, EditorError> {
        if !self.available(store, view, kind) {
            return Ok(false);
        }
        let Some((project, lsp)) = self.project.clone().zip(self.lsp) else {
            return Ok(false);
        };
        let snapshot = store.documents().snapshot(
            store
                .views()
                .get(view)
                .ok_or(EditorError::NotFound)?
                .document,
        )?;
        let fallback = self.word(store, view, byte).unwrap_or(byte..byte);
        let request = self.state.borrow_mut().begin(
            store,
            Context {
                project: project.clone(),
                source: view,
                owner: self.owner,
                kind,
                byte,
                fallback,
                viewport: self.viewport,
                keyboard,
            },
            lsp.documentation_providers(&project, &snapshot, kind),
        )?;
        if lsp.documentation(request).is_err() {
            self.state.borrow_mut().close(view, kind);
            return Err(EditorError::InvalidBoundary);
        }
        Ok(true)
    }

    fn current(
        &self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
    ) -> Option<taide_native_ui::editor_documentation::Widget> {
        use taide_native_ui::editor_documentation::{Content, Part, Widget};
        let state = self.state.borrow();
        let request = state.request(store, view, kind)?;
        if request.owner != self.owner || request.viewport != self.viewport {
            return None;
        }
        let content = state
            .payload(store, view, kind)
            .map(|payload| match payload {
                Payload::Hover(parts) => Content::Hover(
                    parts
                        .iter()
                        .filter_map(|part| {
                            Some(Part {
                                range: taide_native_editor::lsp::range_to_bytes(
                                    &request.snapshot,
                                    part.range,
                                )
                                .ok()?,
                                documents: part.documents.clone(),
                            })
                        })
                        .collect(),
                ),
                Payload::Signature(signature) => Content::Signature {
                    model: signature.model.clone(),
                    parameter: signature.parameter_documentation().cloned(),
                    documentation: signature
                        .documentation
                        .get(signature.model.index())
                        .cloned()
                        .flatten(),
                },
            });
        Some(Widget {
            token: request.token.to_string(),
            byte: request.byte,
            keyboard: request.keyboard,
            pending: state.pending(store, view, kind),
            content,
        })
    }

    fn close(&mut self, view: ViewId, kind: Kind) {
        self.state.borrow_mut().close(view, kind);
    }

    fn cycle(&mut self, store: &EditorStore, view: ViewId, forward: bool) -> bool {
        self.state.borrow_mut().cycle(store, view, forward, true)
    }

    fn code(
        &mut self,
        _ui: &eframe::egui::Ui,
        language: &str,
        text: &str,
    ) -> Option<eframe::egui::text::LayoutJob> {
        self.state
            .borrow()
            .code
            .job(language, text, &self.language, self.appearance)
    }

    fn image(
        &mut self,
        ui: &mut eframe::egui::Ui,
        source: &str,
        alt: &str,
        dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<eframe::egui::Response> {
        self.state
            .borrow_mut()
            .images
            .show(ui, self.project.as_ref()?, source, alt, dimensions)
    }

    fn open_link(&mut self, ui: &eframe::egui::Ui, target: &str) -> bool {
        let Ok(uri) = url::Url::parse(target) else {
            return false;
        };
        match uri.scheme() {
            "http" | "https" | "mailto" => {
                ui.ctx()
                    .open_url(eframe::egui::OpenUrl::new_tab(uri.as_str()));
                true
            }
            "file" => {
                let Some(project) = self.project.clone() else {
                    return false;
                };
                let Some(request) =
                    FileRequest::new(project, self.owner_key.clone(), &uri, self.viewport)
                else {
                    return false;
                };
                self.commands
                    .push(crate::host::HostCommand::OpenDocumentationFile(request));
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "editor-documentation-tests.rs"]
mod tests;

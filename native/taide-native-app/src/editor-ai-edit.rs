use std::collections::HashMap;
use std::future::Future;
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;

use eframe::egui;
use taide_model::ai::{AiInlineEditRequest, AiProviderId, AiTextResponse};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentKey, DocumentSnapshot, Edit, EditorError, UndoGroup};
use taide_native_editor::language_configuration::is_js_whitespace;
use taide_native_editor::store::{EditorStore, Transaction};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_runtime::AppServices;
use tokio::sync::{oneshot, watch};
use uuid::Uuid;

const CONTEXT_UNITS: usize = 2000;
const SELECTION_BYTES: usize = 100 * 1024;
const INSTRUCTION_BYTES: usize = 4 * 1024;
const FENCE: &str = "```";

type EditFuture = Pin<Box<dyn Future<Output = AppResult<AiTextResponse>> + Send>>;
type Complete = Arc<dyn Fn(&AppServices, AiInlineEditRequest) -> EditFuture + Send + Sync>;

#[derive(Clone, PartialEq, Eq)]
struct Config {
    provider: Option<AiProviderId>,
    model: Option<String>,
    base_url: Option<String>,
    credential: Option<Uuid>,
}

impl Config {
    fn new(services: &AppServices) -> Self {
        let settings = services.state.settings.read();
        Self {
            provider: settings.ai_provider,
            model: settings.ai_model.clone(),
            base_url: settings.ai_omlx_base_url.clone(),
            credential: settings.ai_provider.and_then(|provider| {
                let account = match provider {
                    AiProviderId::OllamaCloud => taide_infra::secret::SecretAccount::AiOllamaCloud,
                    AiProviderId::Codex => taide_infra::secret::SecretAccount::AiCodex,
                    AiProviderId::Omlx => taide_infra::secret::SecretAccount::AiOmlx,
                };
                services.secrets.0.revision(account)
            }),
        }
    }
}

pub(crate) struct Origin {
    pub project: ProjectId,
    pub owner: ViewId,
    pub owner_key: ViewKey,
    pub source: ViewId,
    pub source_key: ViewKey,
    pub snapshot: DocumentSnapshot,
    pub target: Range<usize>,
    owner_document: taide_native_editor::document::DocumentId,
    file: String,
}

impl Origin {
    fn describes(&self, store: &EditorStore) -> bool {
        store
            .views()
            .get(self.owner)
            .is_some_and(|view| view.key == self.owner_key && view.document == self.owner_document)
            && store.views().get(self.source).is_some_and(|view| {
                view.key == self.source_key
                    && view.document == self.snapshot.id
                    && view.composition.is_none()
                    && store
                        .documents()
                        .snapshot(view.document)
                        .is_ok_and(|snapshot| {
                            snapshot.key == self.snapshot.key
                                && snapshot.revision == self.snapshot.revision
                                && snapshot.metadata.language_id
                                    == self.snapshot.metadata.language_id
                                && !snapshot.metadata.read_only
                        })
            })
    }
}

struct Pending {
    id: String,
    cancel: watch::Sender<bool>,
    reply: oneshot::Receiver<AppResult<AiTextResponse>>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

enum Stage {
    Idle,
    Loading(Pending),
    Preview(String),
}

pub(crate) struct Session {
    pub token: Uuid,
    pub focus: Uuid,
    pub origin: Origin,
    pub instruction: String,
    config: Option<Config>,
    stage: Stage,
}

impl Session {
    pub(crate) fn is_loading(&self) -> bool {
        matches!(self.stage, Stage::Loading(_))
    }

    pub(crate) fn preview(&self) -> Option<&str> {
        match &self.stage {
            Stage::Preview(text) => Some(text),
            _ => None,
        }
    }
}

pub(crate) struct State {
    sessions: HashMap<ViewId, Session>,
    complete: Complete,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Poll {
    Waiting,
    Preview,
    Empty,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            complete: Arc::new(|services, request| {
                let state = services.state.clone();
                let requests = services.ai_requests.clone();
                let secrets = services.secrets.clone();
                Box::pin(async move {
                    taide_runtime::ai_actions::ai_inline_edit(&state, &requests, &secrets, request)
                        .await
                })
            }),
        }
    }
}

impl State {
    pub(crate) fn open(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        file: String,
    ) -> Result<(), EditorError> {
        if let Some(session) = self
            .sessions
            .get_mut(&source)
            .filter(|session| session.origin.describes(store))
        {
            session.focus = Uuid::new_v4();
            return Ok(());
        }
        self.close(source);
        let view = store.views().get(source).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(view.document)?;
        if snapshot.metadata.read_only {
            return Err(EditorError::ReadOnly);
        }
        if view.composition.is_some() {
            return Err(EditorError::InvalidIdentity);
        }
        let selection = view.selection.selections[view.selection.primary];
        let target = if selection.anchor == selection.head {
            let line = snapshot.rope.byte_to_line(selection.head);
            taide_native_editor::editing::line_content_range(&snapshot, line)
        } else {
            selection.anchor.min(selection.head)..selection.anchor.max(selection.head)
        };
        if target.len() > SELECTION_BYTES {
            return Err(EditorError::Capacity);
        }
        let owner_view = store.views().get(owner).ok_or(EditorError::NotFound)?;
        let file = match &snapshot.key {
            DocumentKey::File(path) => path.to_string_lossy().into_owned(),
            _ => file,
        };
        self.sessions.insert(
            source,
            Session {
                token: Uuid::new_v4(),
                focus: Uuid::new_v4(),
                origin: Origin {
                    project,
                    owner,
                    owner_key: owner_view.key.clone(),
                    source,
                    source_key: view.key.clone(),
                    snapshot,
                    target,
                    owner_document: owner_view.document,
                    file,
                },
                instruction: String::new(),
                config: None,
                stage: Stage::Idle,
            },
        );
        Ok(())
    }

    pub(crate) fn session(
        &mut self,
        store: &EditorStore,
        services: &AppServices,
        view: ViewId,
    ) -> Option<&mut Session> {
        if !self.is_open(store, services, view) {
            self.close(view);
            return None;
        }
        self.sessions.get_mut(&view)
    }

    pub(crate) fn is_open(
        &self,
        store: &EditorStore,
        services: &AppServices,
        view: ViewId,
    ) -> bool {
        self.sessions.get(&view).is_some_and(|session| {
            session.origin.describes(store)
                && session
                    .config
                    .as_ref()
                    .is_none_or(|config| config == &Config::new(services))
        })
    }

    pub(crate) fn submit(
        &mut self,
        store: &EditorStore,
        services: &AppServices,
        view: ViewId,
        context: &egui::Context,
    ) -> AppResult<bool> {
        let complete = self.complete.clone();
        let Some(session) = self.session(store, services, view) else {
            return Ok(false);
        };
        let instruction = session.instruction.trim_matches(is_js_whitespace);
        if instruction.is_empty() || !matches!(session.stage, Stage::Idle) {
            return Ok(false);
        }
        if instruction.len() > INSTRUCTION_BYTES {
            return Err(AppError::InvalidArgument(
                "AI inline edit instruction exceeds byte limit".into(),
            ));
        }
        let (prefix, suffix) = context_window(&session.origin.snapshot, &session.origin.target)
            .map_err(|_| AppError::InvalidArgument("AI inline edit target is invalid".into()))?;
        let config = Config::new(services);
        let id = Uuid::new_v4().to_string();
        let request = AiInlineEditRequest {
            request_id: id.clone(),
            owner: session.origin.owner_key.window.clone(),
            provider: config.provider,
            model: config.model.clone(),
            selection: session
                .origin
                .snapshot
                .rope
                .byte_slice(session.origin.target.clone())
                .to_string(),
            instruction: instruction.into(),
            language: session.origin.snapshot.metadata.language_id.clone(),
            file_path: session.origin.file.clone(),
            prefix,
            suffix,
        };
        let future = complete(services, request);
        let (cancel, mut cancelled) = watch::channel(false);
        let (send, reply) = oneshot::channel();
        let repaint = context.clone();
        if !services
            .tasks
            .spawn_transient("native-ai-inline-edit", async move {
                let result = tokio::select! {
                    biased;
                    _ = cancelled.changed() => return,
                    result = future => result,
                };
                let _ = send.send(result);
                repaint.request_repaint();
            })
        {
            return Err(AppError::Internal(
                "AI inline edit task supervisor is stopped".into(),
            ));
        }
        session.config = Some(config);
        session.stage = Stage::Loading(Pending { id, cancel, reply });
        Ok(true)
    }

    pub(crate) fn poll(
        &mut self,
        store: &EditorStore,
        services: &AppServices,
        view: ViewId,
    ) -> AppResult<Poll> {
        let Some(session) = self.session(store, services, view) else {
            return Ok(Poll::Waiting);
        };
        let Stage::Loading(pending) = &mut session.stage else {
            return Ok(Poll::Waiting);
        };
        let result = match pending.reply.try_recv() {
            Ok(result) => result,
            Err(oneshot::error::TryRecvError::Empty) => return Ok(Poll::Waiting),
            Err(oneshot::error::TryRecvError::Closed) => Err(AppError::Internal(
                "AI inline edit task ended without a response".into(),
            )),
        };
        let result = result.and_then(|response| {
            if response.request_id != pending.id {
                return Err(AppError::InvalidArgument(
                    "AI inline edit response identity mismatch".into(),
                ));
            }
            let text = response
                .text
                .map(|text| strip_code_fence(&text))
                .filter(|text| !text.is_empty());
            Ok(text)
        });
        match result {
            Ok(Some(text)) => {
                session.stage = Stage::Preview(text);
                Ok(Poll::Preview)
            }
            Ok(None) => {
                self.close(view);
                Ok(Poll::Empty)
            }
            Err(error) => {
                self.close(view);
                Err(error)
            }
        }
    }

    pub(crate) fn accept(
        &mut self,
        store: &mut EditorStore,
        services: &AppServices,
        view: ViewId,
    ) -> Result<bool, EditorError> {
        let Some(session) = self.session(store, services, view) else {
            return Ok(false);
        };
        let Some(text) = session.preview() else {
            return Ok(false);
        };
        let snapshot = &session.origin.snapshot;
        let text = taide_native_editor::editing::normalize_line_breaks(
            text,
            snapshot.metadata.line_ending.as_str(),
        );
        let end = session.origin.target.start + text.len();
        store.apply_separate(
            snapshot.id,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: session.origin.target.clone(),
                    text,
                }],
                group: UndoGroup(snapshot.revision),
                origin: Some(view),
                selection_after: Some(SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: end,
                        head: end,
                    }],
                }),
            },
        )?;
        self.close(view);
        Ok(true)
    }

    pub(crate) fn close(&mut self, view: ViewId) {
        self.sessions.remove(&view);
    }

    pub(crate) fn clear(&mut self) {
        self.sessions.clear();
    }

    pub(crate) fn is_loading(&self, view: ViewId) -> bool {
        self.sessions.get(&view).is_some_and(Session::is_loading)
    }

    pub(crate) fn set_instruction(&mut self, view: ViewId, instruction: String) {
        if let Some(session) = self.sessions.get_mut(&view) {
            session.instruction = instruction
        }
    }

    pub(crate) fn previews(&self, store: &EditorStore) -> impl Iterator<Item = (&str, &str)> {
        self.sessions
            .values()
            .filter(|session| session.origin.describes(store))
            .filter_map(|session| {
                session
                    .preview()
                    .map(|text| (session.origin.snapshot.metadata.language_id.as_str(), text))
            })
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        services: &AppServices,
        mut active: impl FnMut(&Origin) -> bool,
    ) -> usize {
        let mut cancelled = 0;
        self.sessions.retain(|_, session| {
            let retained = session.origin.describes(store)
                && session
                    .config
                    .as_ref()
                    .is_none_or(|config| config == &Config::new(services))
                && active(&session.origin);
            if !retained && session.is_loading() {
                cancelled += 1;
            }
            retained
        });
        cancelled
    }

    #[cfg(test)]
    pub(crate) fn with_complete(complete: Complete) -> Self {
        Self {
            complete,
            ..Self::default()
        }
    }
}

fn context_window(
    snapshot: &DocumentSnapshot,
    target: &Range<usize>,
) -> Result<(String, String), EditorError> {
    let start = snapshot
        .rope
        .try_byte_to_char(target.start)
        .map_err(|_| EditorError::InvalidBoundary)?;
    let end = snapshot
        .rope
        .try_byte_to_char(target.end)
        .map_err(|_| EditorError::InvalidBoundary)?;
    if start > end
        || snapshot.rope.char_to_byte(start) != target.start
        || snapshot.rope.char_to_byte(end) != target.end
    {
        return Err(EditorError::InvalidBoundary);
    }
    let first = snapshot
        .rope
        .char_to_utf16_cu(start)
        .saturating_sub(CONTEXT_UNITS);
    let mut prefix = snapshot.rope.utf16_cu_to_char(first);
    if snapshot.rope.char_to_utf16_cu(prefix) < first {
        prefix += 1
    }
    let last = snapshot
        .rope
        .char_to_utf16_cu(end)
        .saturating_add(CONTEXT_UNITS)
        .min(snapshot.rope.len_utf16_cu());
    let suffix = snapshot.rope.utf16_cu_to_char(last);
    Ok((
        snapshot.rope.slice(prefix..start).to_string(),
        snapshot.rope.slice(end..suffix).to_string(),
    ))
}

fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim_matches(is_js_whitespace);
    let Some(open) = trimmed.find(FENCE) else {
        return trimmed.into();
    };
    let Some(close) = trimmed.rfind(FENCE).filter(|close| *close > open) else {
        return trimmed.into();
    };
    let start = trimmed[open..]
        .find('\n')
        .map(|offset| open + offset)
        .filter(|line| *line < close)
        .map_or(open + FENCE.len(), |line| line + 1);
    let content = trimmed[start..close].replace("\r\n", "\n");
    content.strip_suffix('\n').unwrap_or(&content).into()
}

#[cfg(test)]
#[path = "editor-ai-edit-tests.rs"]
mod tests;

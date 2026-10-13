use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Event, Key};
use taide_model::ai::{AiInlineCompleteRequest, AiProviderId, AiTextResponse};
use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::settings::Settings;
use taide_native_editor::completion_preview::{GhostText, Part};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use taide_runtime::AppServices;
use tokio::sync::{oneshot, watch};
use uuid::Uuid;

const DEBOUNCE: Duration = Duration::from_millis(300);
const PREFIX_UNITS: usize = 4000;
const SUFFIX_UNITS: usize = 2000;
const CACHE_ENTRIES: usize = 50;
const COMPLETION_BYTES: usize = 32 * 1024;

type CompletionFuture = Pin<Box<dyn Future<Output = AppResult<AiTextResponse>> + Send>>;
type Complete = Arc<
    dyn Fn(&AppServices, AiInlineCompleteRequest, Option<String>) -> CompletionFuture + Send + Sync,
>;

#[derive(Clone, PartialEq, Eq)]
struct Config {
    provider: AiProviderId,
    model: String,
    base_url: Option<String>,
}

impl Config {
    fn new(settings: &Settings) -> Option<Self> {
        if !settings.ai_auto_tab_enabled {
            return None;
        }
        let provider = settings.ai_provider?;
        let model = settings
            .ai_model
            .clone()
            .filter(|model| !model.is_empty())?;
        Some(Self {
            provider,
            model,
            base_url: settings.ai_omlx_base_url.clone(),
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
struct CacheKey {
    config: Config,
    credential: Option<Uuid>,
    file: String,
    language: String,
    prefix: String,
    suffix: String,
}

pub(crate) struct Origin {
    pub project: ProjectId,
    pub owner: ViewId,
    pub owner_key: ViewKey,
    snapshot: DocumentSnapshot,
    source: ViewId,
    source_key: ViewKey,
    owner_document: taide_native_editor::document::DocumentId,
    selection: SelectionSet,
    key: CacheKey,
}

impl Origin {
    fn describes(&self, store: &EditorStore) -> bool {
        store.views().get(self.owner).is_some_and(|owner| {
            owner.key == self.owner_key && owner.document == self.owner_document
        }) && store.views().get(self.source).is_some_and(|source| {
            source.key == self.source_key
                && source.document == self.snapshot.id
                && source.selection == self.selection
                && source.composition.is_none()
                && store
                    .documents()
                    .snapshot(source.document)
                    .is_ok_and(|snapshot| {
                        snapshot.key == self.snapshot.key
                            && snapshot.revision == self.snapshot.revision
                            && snapshot.metadata.language_id == self.snapshot.metadata.language_id
                            && !snapshot.metadata.read_only
                    })
        })
    }
}

struct Entry {
    origin: Origin,
    id: String,
    cancel: watch::Sender<bool>,
    reply: oneshot::Receiver<AppResult<AiTextResponse>>,
    text: Option<String>,
    finished: bool,
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

impl Entry {
    fn reuse(&mut self, store: &EditorStore) -> bool {
        let Some(source) = store.views().get(self.origin.source) else {
            return false;
        };
        let Ok(snapshot) = store.documents().snapshot(source.document) else {
            return false;
        };
        let Some(text) = self.text.as_ref().filter(|_| self.finished) else {
            return false;
        };
        if source.key != self.origin.source_key
            || source.document != self.origin.snapshot.id
            || source.composition.is_some()
            || source.selection.primary != self.origin.selection.primary
            || source.selection.selections.len() != self.origin.selection.selections.len()
            || snapshot.key != self.origin.snapshot.key
            || snapshot.metadata.read_only
            || snapshot.metadata.language_id != self.origin.snapshot.metadata.language_id
            || !store.views().get(self.origin.owner).is_some_and(|owner| {
                owner.key == self.origin.owner_key && owner.document == self.origin.owner_document
            })
        {
            return false;
        }
        let Ok(taide_native_editor::change_journal::ChangesSince::Tracked(changes)) =
            store.changes_since(snapshot.id, self.origin.snapshot.revision)
        else {
            return false;
        };
        let changes = changes.collect::<Vec<_>>();
        let mut positions = self
            .origin
            .selection
            .selections
            .iter()
            .map(|selection| (selection.head, 0usize))
            .collect::<Vec<_>>();
        for change in &changes {
            if change.spans.len() != positions.len()
                || change.spans.iter().any(|span| {
                    span.new_end_byte <= span.old_end_byte
                        || span.start_byte != span.old_end_byte && changes.len() != 1
                })
            {
                return false;
            }
            for (head, size) in &mut positions {
                let Some(span) = change.spans.iter().find(|span| span.old_end_byte == *head) else {
                    return false;
                };
                let old_size = span.old_end_byte - span.start_byte;
                let before_delta = change
                    .spans
                    .iter()
                    .filter(|other| other.start_byte < span.start_byte)
                    .map(|other| other.new_end_byte - other.old_end_byte)
                    .sum::<usize>();
                let start = span.start_byte + before_delta;
                if old_size > 0
                    && self
                        .origin
                        .snapshot
                        .rope
                        .byte_slice(span.start_byte..span.old_end_byte)
                        != snapshot.rope.byte_slice(start..start + old_size)
                {
                    return false;
                }
                *size += span.new_end_byte - span.old_end_byte;
                *head += change
                    .spans
                    .iter()
                    .filter(|span| span.start_byte <= *head)
                    .map(|span| span.new_end_byte - span.old_end_byte)
                    .sum::<usize>();
            }
        }
        let mut inserted = None;
        for ((head, size), after) in positions.iter().zip(&source.selection.selections) {
            if *size == 0 || after.anchor != after.head || after.head != *head {
                return false;
            }
            let value = snapshot
                .rope
                .byte_slice(after.head - size..after.head)
                .to_string();
            if !text.starts_with(&value)
                || inserted.as_ref().is_some_and(|previous| previous != &value)
            {
                return false;
            }
            inserted = Some(value);
        }
        let Some(inserted) = inserted else {
            return false;
        };
        let remainder = text[inserted.len()..].to_owned();
        self.text = (!remainder.is_empty()).then_some(remainder);
        self.origin.snapshot = snapshot;
        self.origin.selection = source.selection.clone();
        true
    }
}

pub(crate) struct State {
    entries: HashMap<ViewId, Entry>,
    cache: VecDeque<(CacheKey, String)>,
    complete: Complete,
}

impl Default for State {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            cache: VecDeque::new(),
            complete: Arc::new(|services, request, cached| {
                let state = services.state.clone();
                let requests = services.ai_requests.clone();
                let secrets = services.secrets.clone();
                Box::pin(async move {
                    let status =
                        taide_runtime::ai_actions::ai_token_status(&state, &secrets).await?;
                    let configured = match request.provider {
                        AiProviderId::OllamaCloud => status.ollama_cloud,
                        AiProviderId::Codex => status.codex,
                        AiProviderId::Omlx => status.omlx,
                    };
                    if !configured {
                        return Ok(AiTextResponse {
                            request_id: request.request_id,
                            text: None,
                        });
                    }
                    if cached.is_some() {
                        return Ok(AiTextResponse {
                            request_id: request.request_id,
                            text: cached,
                        });
                    }
                    taide_runtime::ai_actions::ai_inline_complete(
                        &state, &requests, &secrets, request,
                    )
                    .await
                })
            }),
        }
    }
}

impl State {
    #[cfg(test)]
    pub(crate) fn with_complete(complete: Complete) -> Self {
        Self {
            complete,
            ..Self::default()
        }
    }
    pub(crate) fn observe(
        &mut self,
        store: &EditorStore,
        services: &AppServices,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        file: String,
        context: &egui::Context,
    ) -> Result<(), EditorError> {
        self.request(store, services, project, source, owner, file, context, true)
    }

    pub(crate) fn request(
        &mut self,
        store: &EditorStore,
        services: &AppServices,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        file: String,
        context: &egui::Context,
        automatic: bool,
    ) -> Result<(), EditorError> {
        let Some(config) = Config::new(&services.state.settings.read()) else {
            self.entries.remove(&source);
            return Ok(());
        };
        let credential = services.secrets.0.revision(account(config.provider));
        if automatic
            && self.entries.get(&source).is_some_and(|entry| {
                entry.origin.describes(store)
                    && entry.origin.key.config == config
                    && entry.origin.key.credential == credential
            })
        {
            return Ok(());
        }
        if automatic
            && self.entries.get_mut(&source).is_some_and(|entry| {
                entry.origin.key.config == config
                    && entry.origin.key.credential == credential
                    && entry.reuse(store)
            })
        {
            return Ok(());
        }
        self.entries.remove(&source);
        let current = store.views().get(source).ok_or(EditorError::NotFound)?;
        if current.composition.is_some()
            || current
                .selection
                .selections
                .iter()
                .any(|selection| selection.anchor != selection.head)
        {
            return Ok(());
        }
        let snapshot = store.documents().snapshot(current.document)?;
        if snapshot.metadata.read_only {
            return Ok(());
        }
        let file = match &snapshot.key {
            taide_native_editor::document::DocumentKey::File(path) => {
                path.to_string_lossy().into_owned()
            }
            _ => file,
        };
        let owner_state = store.views().get(owner).ok_or(EditorError::NotFound)?;
        let byte = current.selection.selections[current.selection.primary].head;
        let (prefix, suffix) = context_window(&snapshot, byte)?;
        let key = CacheKey {
            config: config.clone(),
            credential,
            file: file.clone(),
            language: snapshot.metadata.language_id.clone(),
            prefix,
            suffix,
        };
        let cached = self
            .cache
            .iter()
            .find(|(stored, _)| stored == &key)
            .map(|(_, text)| text.clone());
        let id = Uuid::new_v4().to_string();
        let request = AiInlineCompleteRequest {
            request_id: id.clone(),
            owner: owner_state.key.window.clone(),
            provider: config.provider,
            model: config.model,
            prefix: key.prefix.clone(),
            suffix: key.suffix.clone(),
            language: key.language.clone(),
            file_path: file,
        };
        let is_cached = cached.is_some();
        let future = (self.complete)(services, request, cached);
        let (cancel, mut cancelled) = watch::channel(false);
        let (send, reply) = oneshot::channel();
        let repaint = context.clone();
        if !services
            .tasks
            .spawn_transient("native-ai-inline-completion", async move {
                let result = tokio::select! {
                    biased;
                    _ = cancelled.changed() => return,
                    result = async move {
                        if automatic && !is_cached {
                            tokio::time::sleep(DEBOUNCE).await;
                        }
                        future.await
                    } => result,
                };
                let _ = send.send(result);
                repaint.request_repaint();
            })
        {
            return Ok(());
        }
        self.entries.insert(
            source,
            Entry {
                origin: Origin {
                    project,
                    owner,
                    owner_key: owner_state.key.clone(),
                    snapshot,
                    source,
                    source_key: current.key.clone(),
                    owner_document: owner_state.document,
                    selection: current.selection.clone(),
                    key,
                },
                id,
                cancel,
                reply,
                text: None,
                finished: false,
            },
        );
        Ok(())
    }

    pub(crate) fn poll(&mut self, store: &EditorStore, view: ViewId) -> Vec<GhostText> {
        let Some(entry) = self
            .entries
            .get_mut(&view)
            .filter(|entry| entry.origin.describes(store))
        else {
            self.entries.remove(&view);
            return Vec::new();
        };
        if !entry.finished {
            match entry.reply.try_recv() {
                Ok(result) => {
                    entry.finished = true;
                    entry.text = result
                        .ok()
                        .filter(|response| response.request_id == entry.id)
                        .and_then(|response| response.text)
                        .filter(|text| !text.is_empty() && text.len() <= COMPLETION_BYTES);
                    if let Some(text) = &entry.text {
                        self.cache.retain(|(key, _)| key != &entry.origin.key);
                        if self.cache.len() >= CACHE_ENTRIES {
                            self.cache.pop_front();
                        }
                        self.cache
                            .push_back((entry.origin.key.clone(), text.clone()));
                    }
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    entry.finished = true;
                    entry.text = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
            }
        }
        let Some(text) = &entry.text else {
            return Vec::new();
        };
        entry
            .origin
            .selection
            .selections
            .iter()
            .map(|selection| GhostText {
                line: entry.origin.snapshot.rope.byte_to_line(selection.head),
                parts: vec![Part {
                    byte: selection.head,
                    text: text.clone(),
                    is_preview: false,
                }],
            })
            .collect()
    }

    pub(crate) fn defers(&self, view: ViewId, event: &Event) -> bool {
        self.entries
            .get(&view)
            .is_some_and(|entry| entry.text.is_some())
            && matches!(
                event,
                Event::Key { key: Key::Tab | Key::Escape, pressed: true, modifiers, .. }
                    if modifiers.is_none()
            )
    }

    pub(crate) fn event(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        event: &Event,
    ) -> Result<bool, EditorError> {
        if !self.defers(view, event) || self.poll(store, view).is_empty() {
            return Ok(false);
        }
        let entry = self.entries.remove(&view).ok_or(EditorError::NotFound)?;
        if matches!(
            event,
            Event::Key {
                key: Key::Escape,
                ..
            }
        ) {
            return Ok(true);
        }
        taide_native_editor::editing::replace_selections(
            store,
            view,
            entry.text.as_deref().unwrap_or_default(),
            None,
        )
    }

    pub(crate) fn close(&mut self, view: ViewId) {
        self.entries.remove(&view);
    }

    pub(crate) fn cancel_changed(&mut self, store: &EditorStore, view: ViewId) {
        if self
            .entries
            .get(&view)
            .is_some_and(|entry| !entry.origin.describes(store))
        {
            self.close(view);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.cache.clear();
    }

    pub(crate) fn reconcile_credentials(&mut self, services: &AppServices) {
        self.entries.retain(|_, entry| {
            services
                .secrets
                .0
                .revision(account(entry.origin.key.config.provider))
                == entry.origin.key.credential
        });
        self.cache.retain(|(key, _)| {
            services.secrets.0.revision(account(key.config.provider)) == key.credential
        });
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        settings: &Settings,
        mut active: impl FnMut(&Origin) -> bool,
    ) {
        let config = Config::new(settings);
        self.entries.retain(|_, entry| {
            config.as_ref() == Some(&entry.origin.key.config)
                && entry.origin.describes(store)
                && active(&entry.origin)
        });
    }
}

fn account(provider: AiProviderId) -> taide_infra::secret::SecretAccount {
    match provider {
        AiProviderId::OllamaCloud => taide_infra::secret::SecretAccount::AiOllamaCloud,
        AiProviderId::Codex => taide_infra::secret::SecretAccount::AiCodex,
        AiProviderId::Omlx => taide_infra::secret::SecretAccount::AiOmlx,
    }
}

fn context_window(
    snapshot: &DocumentSnapshot,
    byte: usize,
) -> Result<(String, String), EditorError> {
    let character = snapshot
        .rope
        .try_byte_to_char(byte)
        .map_err(|_| EditorError::InvalidBoundary)?;
    if snapshot.rope.char_to_byte(character) != byte {
        return Err(EditorError::InvalidBoundary);
    }
    let units = snapshot.rope.char_to_utf16_cu(character);
    let prefix_unit = units.saturating_sub(PREFIX_UNITS);
    let mut start = snapshot.rope.utf16_cu_to_char(prefix_unit);
    if snapshot.rope.char_to_utf16_cu(start) < prefix_unit {
        start += 1;
    }
    let end_unit = units
        .saturating_add(SUFFIX_UNITS)
        .min(snapshot.rope.len_utf16_cu());
    let end = snapshot.rope.utf16_cu_to_char(end_unit);
    Ok((
        snapshot.rope.slice(start..character).to_string(),
        snapshot.rope.slice(character..end).to_string(),
    ))
}

#[cfg(test)]
#[path = "editor-ai-completion-tests.rs"]
mod tests;

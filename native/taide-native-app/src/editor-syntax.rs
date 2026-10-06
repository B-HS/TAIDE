use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use taide_model::error::{AppError, AppResult};
use taide_model::plugin::LoadedPlugin;
use taide_model::theme::ResolvedTheme;
use taide_native_editor::change_journal::ChangesSince;
use taide_native_editor::document::DocumentId;
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::{DocumentVersion, EditorStore};
use taide_native_editor::syntax::SyntaxSnapshot;
use taide_native_syntax::{
    LeadingTrailingDebounce, PluginGrammar, THEME_REAPPLY_DEBOUNCE, TokenPipeline, TokenTheme,
    WorkerClient, token_worker,
};
use taide_native_ui::editor_surface::EditorTokens;
use taide_plugin::service::PluginStore;
use taide_runtime::{AppState, TaskSupervisor, plugin_actions};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const WORKER_NAME: &str = "native-editor-syntax";
const PLUGIN_GRAMMAR_REQUEST_NAME: &str = "native-editor-plugin-grammar-request";
const PLUGIN_GRAMMAR_READ_NAME: &str = "native-editor-plugin-grammar-read";

type Wake = Arc<dyn Fn() + Send + Sync>;
type PluginList = Option<Vec<LoadedPlugin>>;

struct TrackedDocument {
    revision: u64,
    language_id: String,
    shown_lines: Option<Range<usize>>,
}

struct PluginGrammarRead {
    plugins: Vec<LoadedPlugin>,
    grammars: Vec<PluginGrammar>,
}

async fn plugin_grammars(state: &AppState, store: &PluginStore) -> AppResult<PluginGrammarRead> {
    let plugins = plugin_actions::plugin_list(state, store).await?;
    let mut grammars = Vec::new();
    for plugin in plugins.iter().filter(|plugin| plugin.enabled) {
        for language in &plugin.manifest.contributes.languages {
            if language.grammar.as_deref().is_none_or(str::is_empty) {
                continue;
            }
            let Ok(grammar_json) = plugin_actions::plugin_read_grammar(
                state,
                store,
                plugin.manifest.id.clone(),
                language.id.clone(),
            )
            .await
            else {
                continue;
            };
            grammars.extend(PluginGrammar::from_contribution(
                &language.id,
                language.embedded_languages.as_deref().unwrap_or_default(),
                &grammar_json,
            ));
        }
    }
    Ok(PluginGrammarRead { plugins, grammars })
}

pub(crate) async fn finished(worker: Option<JoinHandle<()>>) {
    if let Some(worker) = worker
        && let Err(error) = worker.await
    {
        log::warn!("native editor syntax worker did not finish cleanly: {error}");
    }
}

pub struct EditorSyntax {
    pipeline: TokenPipeline,
    worker: Option<JoinHandle<()>>,
    wake: Wake,
    documents: HashMap<DocumentId, TrackedDocument>,
    theme: Option<ResolvedTheme>,
    theme_debounce: LeadingTrailingDebounce,
    followed_plugins: Option<PluginList>,
    plugin_grammar_read: Option<oneshot::Receiver<AppResult<PluginGrammarRead>>>,
}

impl EditorSyntax {
    pub fn connect(
        tasks: &TaskSupervisor,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> AppResult<Self> {
        let (client, task) = token_worker(repaint.clone());
        let worker = tasks
            .spawn_blocking_transient_handle(WORKER_NAME, move || task.run())
            .ok_or_else(|| AppError::Forbidden("native editor syntax host is stopping".into()))?;
        Ok(Self::with_wake(client, Some(worker), repaint))
    }

    #[cfg(test)]
    fn new(client: WorkerClient, worker: Option<JoinHandle<()>>) -> Self {
        Self::with_wake(client, worker, Arc::new(|| {}))
    }

    fn with_wake(client: WorkerClient, worker: Option<JoinHandle<()>>, wake: Wake) -> Self {
        Self {
            pipeline: TokenPipeline::new(client),
            worker,
            wake,
            documents: HashMap::new(),
            theme: None,
            theme_debounce: LeadingTrailingDebounce::new(THEME_REAPPLY_DEBOUNCE),
            followed_plugins: None,
            plugin_grammar_read: None,
        }
    }

    pub fn follow_plugins(
        &mut self,
        state: &AppState,
        store: &PluginStore,
        tasks: &TaskSupervisor,
    ) {
        if let Some(read) = &mut self.plugin_grammar_read {
            let result = match read.try_recv() {
                Ok(result) => result,
                Err(oneshot::error::TryRecvError::Empty) => return,
                Err(oneshot::error::TryRecvError::Closed) => Err(AppError::Internal(
                    "native editor plugin grammar worker stopped".into(),
                )),
            };
            self.plugin_grammar_read = None;
            match result {
                Ok(read) => {
                    self.followed_plugins = Some(Some(read.plugins));
                    self.install_plugin_grammars(read.grammars);
                }
                Err(error) => {
                    log::warn!("native editor plugin grammars were not read: {error}");
                }
            }
        }
        if self.followed_plugins.as_ref() == Some(&*store.0.read()) {
            return;
        }
        let (sender, read) = oneshot::channel();
        let wake = self.wake.clone();
        let supervisor = tasks.clone();
        let read_state = state.clone();
        let read_store = store.clone();
        if !tasks.spawn_transient(PLUGIN_GRAMMAR_REQUEST_NAME, async move {
            let runtime = tokio::runtime::Handle::current();
            let result = supervisor
                .run_blocking_result(PLUGIN_GRAMMAR_READ_NAME, move || {
                    runtime.block_on(plugin_grammars(&read_state, &read_store))
                })
                .await;
            drop(sender.send(result));
            wake();
        }) {
            return;
        }
        self.followed_plugins = Some(store.0.read().clone());
        self.plugin_grammar_read = Some(read);
    }

    fn install_plugin_grammars(&mut self, grammars: Vec<PluginGrammar>) {
        self.pipeline.set_plugin_grammars(grammars);
        let pipeline = &mut self.pipeline;
        self.documents.retain(|document, tracked| {
            let is_accepted = pipeline.accepts_language(&tracked.language_id);
            if !is_accepted {
                pipeline.close(*document);
            }
            is_accepted
        });
    }

    pub fn tick(
        &mut self,
        store: &EditorStore,
        theme: &ResolvedTheme,
        now: Instant,
    ) -> Option<Duration> {
        self.follow_theme(theme, now);
        self.follow_documents(store);
        for (document, tracked) in &mut self.documents {
            if let Some(lines) = tracked.shown_lines.take() {
                self.pipeline.set_visible_lines(*document, lines);
            }
        }
        self.pipeline.poll();
        self.theme_debounce.trailing_delay(now)
    }

    pub fn disconnect(&mut self) -> Option<JoinHandle<()>> {
        self.pipeline.disconnect();
        self.worker.take()
    }

    pub fn tokens(
        &mut self,
        store: &EditorStore,
        document: DocumentId,
    ) -> Option<EditorTokens<'_>> {
        self.catch_up(store, document);
        Some(EditorTokens {
            revision: self.documents.get(&document)?.revision,
            lines: self.pipeline.tokens(document)?,
            styles: self.pipeline.style_table()?,
        })
    }

    pub fn show_lines(&mut self, document: DocumentId, lines: Range<usize>) {
        let Some(tracked) = self.documents.get_mut(&document) else {
            return;
        };
        tracked.shown_lines = Some(match tracked.shown_lines.take() {
            Some(shown) => shown.start.min(lines.start)..shown.end.max(lines.end),
            None => lines,
        });
    }

    pub fn supply_save_cleanup(
        &mut self,
        store: &mut EditorStore,
        document: DocumentId,
        flags: CleanupFlags,
    ) {
        let Ok(snapshot) = store.documents().snapshot(document) else {
            return;
        };
        let trims_trailing_whitespace = snapshot
            .metadata
            .editor_config
            .trim_trailing_whitespace
            .unwrap_or(flags.trim_trailing_whitespace);
        if !trims_trailing_whitespace {
            return;
        }
        self.pipeline.poll();
        self.catch_up(store, document);
        let is_current = self
            .documents
            .get(&document)
            .is_some_and(|tracked| tracked.revision == snapshot.revision);
        let Some(tokens) = self.pipeline.tokens(document).filter(|_| is_current) else {
            return;
        };
        let language_id = snapshot.metadata.language_id.clone();
        let is_exempt_from_tokenization = tokens.line_count() == 0;
        let syntax = if is_exempt_from_tokenization {
            SyntaxSnapshot::without_tokenizer(
                snapshot.revision,
                language_id,
                snapshot.rope.len_lines(),
            )
        } else {
            let Some(styles) = self.pipeline.style_table() else {
                return;
            };
            SyntaxSnapshot::from_accurate_lines(snapshot.revision, language_id, tokens, styles)
        };
        if let Err(error) = store.install_syntax(document, syntax) {
            log::warn!("native editor save syntax was rejected: {error:?}");
        }
    }

    fn catch_up(&mut self, store: &EditorStore, document: DocumentId) {
        let Some(version) = store
            .documents()
            .versions()
            .find(|version| version.id == document)
        else {
            if self.documents.remove(&document).is_some() {
                self.pipeline.close(document);
            }
            return;
        };
        let is_current = self.documents.get(&document).is_some_and(|tracked| {
            tracked.revision == version.revision && tracked.language_id == version.language_id
        });
        if is_current {
            return;
        }
        self.follow_document(store, version);
        if self.documents.contains_key(&document) {
            self.pipeline.poll();
        }
    }

    fn follow_theme(&mut self, theme: &ResolvedTheme, now: Instant) {
        let is_due = if self.theme.as_ref() == Some(theme) {
            self.theme_debounce.poll(now)
        } else {
            self.theme = Some(theme.clone());
            self.theme_debounce.trigger(now)
        };
        if !is_due {
            return;
        }
        let Some(theme) = &self.theme else {
            return;
        };
        match TokenTheme::from_resolved(theme) {
            Ok(token_theme) => self.pipeline.set_theme(token_theme),
            Err(error) => log::warn!("native editor token theme was rejected: {error:?}"),
        }
    }

    fn follow_documents(&mut self, store: &EditorStore) {
        let tracked_count = self.documents.len();
        let mut open_tracked_count = 0;
        for version in store.documents().versions() {
            open_tracked_count += usize::from(self.follow_document(store, version));
        }
        if open_tracked_count == tracked_count {
            return;
        }
        let pipeline = &mut self.pipeline;
        self.documents.retain(|document, tracked| {
            let is_open = store.changes_since(*document, tracked.revision).is_ok();
            if !is_open {
                pipeline.close(*document);
            }
            is_open
        });
    }

    fn follow_document(&mut self, store: &EditorStore, version: DocumentVersion<'_>) -> bool {
        let was_tracked = self.documents.contains_key(&version.id);
        if was_tracked {
            self.follow_tracked(store, version);
        } else {
            self.track(store, version);
        }
        was_tracked
    }

    fn track(&mut self, store: &EditorStore, version: DocumentVersion<'_>) {
        if !self.pipeline.accepts_language(version.language_id)
            || store.views().for_document(version.id).next().is_none()
        {
            return;
        }
        let Ok(snapshot) = store.documents().snapshot(version.id) else {
            return;
        };
        self.documents.insert(
            version.id,
            TrackedDocument {
                revision: snapshot.revision,
                language_id: snapshot.metadata.language_id.clone(),
                shown_lines: None,
            },
        );
        self.pipeline.open(snapshot);
    }

    fn follow_tracked(&mut self, store: &EditorStore, version: DocumentVersion<'_>) {
        let Some(tracked) = self.documents.get_mut(&version.id) else {
            return;
        };
        let is_same_language = tracked.language_id == version.language_id;
        if is_same_language && tracked.revision == version.revision {
            return;
        }
        if !self.pipeline.accepts_language(version.language_id) {
            self.documents.remove(&version.id);
            self.pipeline.close(version.id);
            return;
        }
        let Ok(snapshot) = store.documents().snapshot(version.id) else {
            return;
        };
        let changes = store.changes_since(version.id, tracked.revision);
        tracked.revision = snapshot.revision;
        tracked.language_id = snapshot.metadata.language_id.clone();
        match changes {
            Ok(ChangesSince::Tracked(changes)) if is_same_language => {
                self.pipeline.edit(snapshot, changes);
            }
            _ => self.pipeline.replace(snapshot),
        }
    }
}

#[cfg(test)]
#[path = "editor-syntax-tests.rs"]
mod tests;

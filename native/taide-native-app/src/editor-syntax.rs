use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use taide_model::error::{AppError, AppResult};
use taide_model::plugin::LoadedPlugin;
use taide_model::theme::ResolvedTheme;
use taide_native_editor::change_journal::ChangesSince;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::language_configuration::{
    LanguageRules, LineSyntax, PreparedTokens, UntokenizedLines, token_kind_at,
};
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::{DocumentVersion, EditorStore};
use taide_native_editor::syntax::{SyntaxSnapshot, Token, TokenKind};
use taide_native_syntax::{
    LeadingTrailingDebounce, PluginGrammar, SPAN_FIELDS, THEME_REAPPLY_DEBOUNCE, TokenPipeline,
    TokenPreparer, TokenTheme, WorkerClient, monaco_language, token_worker,
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

pub(crate) struct PeekTokens {
    pub(crate) document: DocumentId,
    pub(crate) revision: u64,
    pub(crate) language_id: String,
    lines: taide_native_editor::line_tokens::LineTokens,
    styles: Option<taide_native_editor::line_tokens::TokenStyleTable>,
    preparer: Option<TokenPreparer>,
}

impl PeekTokens {
    pub(crate) fn frame(&self) -> Option<EditorTokens<'_>> {
        Some(EditorTokens {
            revision: self.revision,
            lines: &self.lines,
            styles: self.styles.as_ref()?,
        })
    }
}

impl LineSyntax for PeekTokens {
    fn prepare_tokens(
        &self,
        document: &DocumentSnapshot,
        lines: Range<usize>,
    ) -> Result<Option<PreparedTokens>, EditorError> {
        if document.id != self.document || document.metadata.language_id != self.language_id {
            return Err(EditorError::StaleRevision);
        }
        prepare_tokens(
            document,
            lines,
            &self.lines,
            document.revision == self.revision,
            self.preparer.as_ref(),
        )
    }

    fn tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        if document.id != self.document
            || document.revision != self.revision
            || document.metadata.language_id != self.language_id
        {
            return None;
        }
        if self.lines.line_count() == 0 {
            return UntokenizedLines.tokens(document, line);
        }
        if !self.lines.has_accurate_tokens(line) {
            return None;
        }
        let mut kinds: Vec<Token> = Vec::new();
        for [start_byte, style_id] in self.lines.spans(line).as_chunks::<SPAN_FIELDS>().0 {
            let kind = self.styles.as_ref()?.style(*style_id).kind;
            if kinds.last().is_none_or(|previous| previous.kind != kind) {
                kinds.push(Token {
                    start_byte: *start_byte as usize,
                    kind,
                });
            }
        }
        Some(kinds)
    }

    fn kind_if_inserting(
        &self,
        document: &DocumentSnapshot,
        line: usize,
        byte_in_line: usize,
        _character: char,
    ) -> TokenKind {
        self.tokens(document, line)
            .filter(|tokens| {
                byte_in_line > 0
                    && byte_in_line < line_content_range(document, line).len()
                    && tokens.iter().all(|token| token.start_byte != byte_in_line)
            })
            .map_or(TokenKind::Other, |tokens| {
                token_kind_at(&tokens, byte_in_line)
            })
    }
}

fn prepare_tokens(
    document: &DocumentSnapshot,
    lines: Range<usize>,
    tokens: &taide_native_editor::line_tokens::LineTokens,
    is_current: bool,
    preparer: Option<&TokenPreparer>,
) -> Result<Option<PreparedTokens>, EditorError> {
    if taide_native_syntax::is_too_large_for_tokenization(
        document.rope.len_utf16_cu(),
        document.rope.len_lines(),
    ) || (is_current && lines.clone().all(|line| tokens.has_accurate_tokens(line)))
    {
        return Ok(None);
    }
    preparer
        .ok_or(EditorError::Refused)?
        .prepare(document, lines)
        .map(Some)
        .map_err(|error| {
            log::warn!("native editor token preparation failed: {error:?}");
            EditorError::Refused
        })
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

pub fn language_rules(language_id: &str) -> Option<&'static dyn LanguageRules> {
    static IS_FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);
    match monaco_language(language_id) {
        Ok(language) => language.map(|language| language as &dyn LanguageRules),
        Err(error) => {
            if !IS_FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
                log::warn!("native editor language configuration was rejected: {error:?}");
            }
            None
        }
    }
}

pub struct SyntaxLease<'a> {
    syntax: Cell<Option<&'a mut EditorSyntax>>,
    document: DocumentId,
}

impl<'a> SyntaxLease<'a> {
    pub fn new(syntax: &'a mut EditorSyntax, document: DocumentId) -> Self {
        Self {
            syntax: Cell::new(Some(syntax)),
            document,
        }
    }

    pub fn frame_tokens(&self, store: &EditorStore) -> Option<EditorTokens<'a>> {
        self.syntax.take()?.tokens(store, self.document)
    }

    fn read<T>(&self, read: impl FnOnce(&mut EditorSyntax) -> T) -> Option<T> {
        let syntax = self.syntax.take()?;
        let value = read(syntax);
        self.syntax.set(Some(syntax));
        Some(value)
    }
}

impl LineSyntax for SyntaxLease<'_> {
    fn prepare_tokens(
        &self,
        document: &DocumentSnapshot,
        lines: Range<usize>,
    ) -> Result<Option<PreparedTokens>, EditorError> {
        self.read(|syntax| {
            syntax.pipeline.poll();
            let Some(tokens) = syntax.pipeline.tokens(document.id) else {
                return Ok(None);
            };
            prepare_tokens(
                document,
                lines,
                tokens,
                true,
                syntax.pipeline.token_preparer().as_ref(),
            )
        })
        .ok_or(EditorError::Refused)?
    }

    fn follow_edits(&self, store: &EditorStore) {
        self.read(|syntax| syntax.catch_up(store, self.document));
    }

    fn tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        self.read(|syntax| syntax.line_kinds(document, line))
            .flatten()
    }

    fn accurate_tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        self.read(|syntax| {
            if let Some(tokens) = syntax.pipeline.tokens(document.id)
                && tokens.line_count() > 0
                && !tokens.has_accurate_tokens(line)
            {
                return None;
            }
            syntax.line_kinds(document, line)
        })
        .flatten()
    }

    fn kind_if_inserting(
        &self,
        document: &DocumentSnapshot,
        line: usize,
        byte_in_line: usize,
        _character: char,
    ) -> TokenKind {
        let line_length = line_content_range(document, line).len();
        self.read(|syntax| syntax.line_kinds(document, line))
            .flatten()
            .filter(|tokens| {
                byte_in_line > 0
                    && byte_in_line < line_length
                    && tokens.iter().all(|token| token.start_byte != byte_in_line)
            })
            .map_or(TokenKind::Other, |tokens| {
                token_kind_at(&tokens, byte_in_line)
            })
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
    peek_tokens: HashMap<DocumentId, Arc<PeekTokens>>,
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
            peek_tokens: HashMap::new(),
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
        self.peek_tokens
            .retain(|document, _| self.documents.contains_key(document));
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

    pub(crate) fn peek_tokens(
        &mut self,
        store: &EditorStore,
        document: DocumentId,
    ) -> Option<Arc<PeekTokens>> {
        self.peek_tokens
            .retain(|document, _| store.documents().snapshot(*document).is_ok());
        self.catch_up(store, document);
        if !self.documents.contains_key(&document) {
            let snapshot = store.documents().snapshot(document).ok()?;
            if !self
                .pipeline
                .accepts_language(&snapshot.metadata.language_id)
            {
                self.peek_tokens.remove(&document);
                return None;
            }
            self.documents.insert(
                document,
                TrackedDocument {
                    revision: snapshot.revision,
                    language_id: snapshot.metadata.language_id.clone(),
                    shown_lines: None,
                },
            );
            self.pipeline.open(snapshot);
        }
        let revision = self.documents.get(&document)?.revision;
        let language_id = &self.documents.get(&document)?.language_id;
        let lines = self.pipeline.tokens(document)?;
        let styles = self.pipeline.style_table().cloned();
        let preparer = self.pipeline.token_preparer();
        if let Some(cached) = self.peek_tokens.get(&document).filter(|cached| {
            cached.revision == revision
                && cached.language_id == *language_id
                && cached.lines.generation() == lines.generation()
                && cached.styles == styles
                && cached.preparer.as_ref().map(TokenPreparer::configuration)
                    == preparer.as_ref().map(TokenPreparer::configuration)
        }) {
            return Some(cached.clone());
        }
        let tokens = Arc::new(PeekTokens {
            document,
            revision,
            language_id: language_id.clone(),
            lines: lines.clone(),
            styles,
            preparer,
        });
        self.peek_tokens.insert(document, tokens.clone());
        Some(tokens)
    }

    pub(crate) fn preview_tokens(
        &mut self,
        store: &EditorStore,
        document: DocumentId,
        line: usize,
        text: &Arc<[String]>,
    ) -> Option<Arc<taide_native_editor::line_tokens::PreviewTokens>> {
        self.catch_up(store, document);
        if !self.documents.contains_key(&document) {
            self.peek_tokens(store, document)?;
        }
        self.pipeline.preview(document, line, text)
    }

    pub(crate) fn retain_previews(&mut self, active: &[(DocumentId, usize)]) {
        self.pipeline.retain_previews(active);
    }

    fn line_kinds(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        let Some(tracked) = self.documents.get(&document.id) else {
            return UntokenizedLines.tokens(document, line);
        };
        if tracked.revision != document.revision {
            return None;
        }
        let tokens = self.pipeline.tokens(document.id)?;
        let is_exempt_from_tokenization = tokens.line_count() == 0;
        if is_exempt_from_tokenization || line_content_range(document, line).is_empty() {
            return UntokenizedLines.tokens(document, line);
        }
        if line >= tokens.line_count() {
            return None;
        }
        let styles = self.pipeline.style_table()?;
        let mut kinds: Vec<Token> = Vec::new();
        for [start_byte, style_id] in tokens.spans(line).as_chunks::<SPAN_FIELDS>().0 {
            let kind = styles.style(*style_id).kind;
            if kinds.last().is_none_or(|previous| previous.kind != kind) {
                kinds.push(Token {
                    start_byte: *start_byte as usize,
                    kind,
                });
            }
        }
        Some(kinds)
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

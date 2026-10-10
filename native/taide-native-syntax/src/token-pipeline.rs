use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use taide_native_editor::change_journal::ChangeSet;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::line_tokens::{LineTokens, PreviewTokens, TokenStyleTable};

use crate::document_tokens::{DocumentTokens, is_too_large_for_tokenization};
use crate::plugin_grammars::PluginGrammar;
use crate::requested_languages::{RequestedLanguages, is_bundled_language};
use crate::textmate_tokenizer::TokenizerLimits;
use crate::token_theme::TokenTheme;
use crate::token_worker::{
    PreviewJob, TokenPreparer, TokenizationJob, WorkerClient, WorkerConfiguration, WorkerRequest,
    WorkerResponse,
};
use crate::tokenizer::SyntaxError;

struct PipelineDocument {
    snapshot: DocumentSnapshot,
    store: DocumentTokens,
    job: Option<u64>,
    visible_lines: Range<usize>,
    is_too_large: bool,
}

impl PipelineDocument {
    fn stored_line_count(&self) -> usize {
        if self.is_too_large {
            0
        } else {
            self.snapshot.rope.len_lines()
        }
    }

    fn reset(&mut self) {
        self.store.reset(self.stored_line_count());
    }
}

struct ActiveConfiguration {
    generation: u64,
    language_ids: Vec<String>,
    style_table: TokenStyleTable,
}

struct PendingConfiguration {
    generation: u64,
    language_ids: Vec<String>,
}

struct Preview {
    job: u64,
    revision: u64,
    generation: u64,
    text: Arc<[String]>,
    tokens: Option<Arc<PreviewTokens>>,
}

pub struct TokenPipeline {
    client: Option<WorkerClient>,
    limits: TokenizerLimits,
    languages: RequestedLanguages,
    plugin_grammars: Vec<PluginGrammar>,
    is_plugin_grammar_set_unsent: bool,
    theme: Option<TokenTheme>,
    is_configuration_stale: bool,
    pending: Option<PendingConfiguration>,
    active: Option<ActiveConfiguration>,
    configuration_error: Option<SyntaxError>,
    last_generation: u64,
    last_job: u64,
    documents: HashMap<DocumentId, PipelineDocument>,
    previews: HashMap<(DocumentId, usize), Preview>,
}

impl TokenPipeline {
    pub fn new(client: WorkerClient) -> Self {
        Self::with_limits(client, TokenizerLimits::default())
    }

    pub fn with_limits(client: WorkerClient, limits: TokenizerLimits) -> Self {
        Self {
            client: Some(client),
            limits,
            languages: RequestedLanguages::default(),
            plugin_grammars: Vec::new(),
            is_plugin_grammar_set_unsent: false,
            theme: None,
            is_configuration_stale: false,
            pending: None,
            active: None,
            configuration_error: None,
            last_generation: 0,
            last_job: 0,
            documents: HashMap::new(),
            previews: HashMap::new(),
        }
    }

    pub fn is_worker_running(&self) -> bool {
        self.client.is_some()
    }

    pub fn requested_language_ids(&self) -> &[String] {
        self.languages.ids()
    }

    pub fn configuration_error(&self) -> Option<&SyntaxError> {
        self.configuration_error.as_ref()
    }

    pub fn style_table(&self) -> Option<&TokenStyleTable> {
        self.active
            .as_ref()
            .map(|configuration| &configuration.style_table)
    }

    pub fn tokens(&self, document: DocumentId) -> Option<&LineTokens> {
        self.documents
            .get(&document)
            .map(|document| document.store.tokens())
    }

    pub fn token_preparer(&self) -> Option<TokenPreparer> {
        let generation = if self.is_configuration_stale {
            self.last_generation.checked_add(1)?
        } else {
            self.pending
                .as_ref()
                .map(|pending| pending.generation)
                .or_else(|| self.active.as_ref().map(|active| active.generation))?
        };
        Some(self.client.as_ref()?.token_preparer(
            WorkerConfiguration {
                generation,
                language_ids: self.languages.ids().to_vec(),
                theme: self.theme.clone()?,
                limits: self.limits,
            },
            self.plugin_grammars.clone(),
        ))
    }

    pub fn contains(&self, document: DocumentId) -> bool {
        self.documents.contains_key(&document)
    }

    pub fn preview(
        &mut self,
        document: DocumentId,
        line: usize,
        text: &Arc<[String]>,
    ) -> Option<Arc<PreviewTokens>> {
        let tracked = self.documents.get(&document)?;
        let active = self.active.as_ref()?;
        if !self.is_tokenizable(tracked)
            || line >= tracked.snapshot.rope.len_lines()
            || text.is_empty()
            || self.pending.is_some()
            || self.is_configuration_stale
        {
            return None;
        }
        let generation = active.generation;
        let revision = tracked.snapshot.revision;
        let key = (document, line);
        if let Some(preview) = self.previews.get(&key).filter(|preview| {
            preview.revision == revision
                && preview.generation == generation
                && (Arc::ptr_eq(&preview.text, text) || preview.text == *text)
        }) {
            return preview.tokens.clone();
        }
        let previous = tracked.store.state_before(line)?;
        let language_id = tracked.snapshot.metadata.language_id.clone();
        self.last_job += 1;
        let id = self.last_job;
        if !self.send(WorkerRequest::Preview(Box::new(PreviewJob {
            id,
            generation,
            language_id,
            previous,
            lines: text.to_vec(),
        }))) {
            return None;
        }
        self.previews.insert(
            key,
            Preview {
                job: id,
                revision,
                generation,
                text: text.clone(),
                tokens: None,
            },
        );
        None
    }

    pub fn retain_previews(&mut self, active: &[(DocumentId, usize)]) {
        self.previews.retain(|key, _| active.contains(key));
    }

    pub fn is_settled(&self) -> bool {
        !self.is_configuration_stale
            && self.pending.is_none()
            && self.documents.values().all(|document| {
                document.job.is_none()
                    && (document.store.tokens().first_invalid_line().is_none()
                        || !self.is_tokenizable(document))
            })
    }

    pub fn set_theme(&mut self, theme: TokenTheme) {
        if self.theme.as_ref() == Some(&theme) {
            return;
        }
        self.theme = Some(theme);
        self.is_configuration_stale = true;
    }

    pub fn accepts_language(&self, language_id: &str) -> bool {
        is_bundled_language(language_id)
            || self
                .plugin_grammars
                .iter()
                .any(|grammar| grammar.language_id() == language_id)
    }

    pub fn set_plugin_grammars(&mut self, plugin_grammars: Vec<PluginGrammar>) {
        for language_id in plugin_grammars
            .iter()
            .flat_map(PluginGrammar::embedded_languages)
        {
            if self.languages.request(language_id) {
                self.is_configuration_stale = true;
            }
        }
        if self.plugin_grammars == plugin_grammars {
            return;
        }
        self.plugin_grammars = plugin_grammars;
        self.is_plugin_grammar_set_unsent = true;
        self.is_configuration_stale = true;
    }

    pub fn open(&mut self, snapshot: DocumentSnapshot) {
        self.previews
            .retain(|(document, _), _| *document != snapshot.id);
        self.cancel_job(snapshot.id);
        self.request_language(&snapshot);
        let mut document = PipelineDocument {
            is_too_large: is_too_large_for_tokenization(
                snapshot.rope.len_utf16_cu(),
                snapshot.rope.len_lines(),
            ),
            store: DocumentTokens::new(0),
            job: None,
            visible_lines: 0..0,
            snapshot,
        };
        document.reset();
        self.documents.insert(document.snapshot.id, document);
    }

    pub fn replace(&mut self, snapshot: DocumentSnapshot) {
        self.cancel_job(snapshot.id);
        self.request_language(&snapshot);
        match self.documents.get_mut(&snapshot.id) {
            Some(document) => {
                document.snapshot = snapshot;
                document.reset();
            }
            None => self.open(snapshot),
        }
    }

    pub fn edit<'a>(
        &mut self,
        snapshot: DocumentSnapshot,
        changes: impl IntoIterator<Item = &'a ChangeSet>,
    ) {
        self.previews
            .retain(|(document, _), _| *document != snapshot.id);
        let is_same_language = self.documents.get(&snapshot.id).is_some_and(|document| {
            document.snapshot.metadata.language_id == snapshot.metadata.language_id
        });
        if !is_same_language {
            self.replace(snapshot);
            return;
        }
        self.cancel_job(snapshot.id);
        let Some(document) = self.documents.get_mut(&snapshot.id) else {
            return;
        };
        document.snapshot = snapshot;
        if document.is_too_large {
            return;
        }
        for change in changes {
            document.store.apply(change);
        }
        if document.store.tokens().line_count() != document.stored_line_count() {
            document.reset();
        }
    }

    pub fn close(&mut self, document: DocumentId) {
        self.previews.retain(|(id, _), _| *id != document);
        self.cancel_job(document);
        self.documents.remove(&document);
    }

    pub fn set_visible_lines(&mut self, document: DocumentId, visible_lines: Range<usize>) {
        let Some(tracked) = self.documents.get_mut(&document) else {
            return;
        };
        if tracked.visible_lines == visible_lines {
            return;
        }
        tracked.visible_lines = visible_lines.clone();
        if tracked.job.is_some() {
            self.send(WorkerRequest::SetVisibleLines {
                document,
                visible_lines,
            });
        }
    }

    pub fn disconnect(&mut self) {
        self.previews.clear();
        self.client = None;
        self.pending = None;
        for document in self.documents.values_mut() {
            document.job = None;
        }
    }

    pub fn poll(&mut self) -> bool {
        let mut has_changed = false;
        while let Some(response) = self.receive() {
            has_changed |= self.accept(response);
        }
        self.configure();
        self.start_jobs();
        has_changed
    }

    fn receive(&mut self) -> Option<WorkerResponse> {
        match self.client.as_ref()?.try_receive() {
            Ok(response) => response,
            Err(_) => {
                self.disconnect();
                None
            }
        }
    }

    fn send(&mut self, request: WorkerRequest) -> bool {
        let is_sent = self
            .client
            .as_ref()
            .is_some_and(|client| client.send(request).is_ok());
        if !is_sent {
            self.disconnect();
        }
        is_sent
    }

    fn request_language(&mut self, snapshot: &DocumentSnapshot) {
        if self.languages.request(&snapshot.metadata.language_id) {
            self.is_configuration_stale = true;
        }
    }

    fn cancel_job(&mut self, document: DocumentId) {
        let has_job = self
            .documents
            .get_mut(&document)
            .is_some_and(|tracked| tracked.job.take().is_some());
        if has_job {
            self.send(WorkerRequest::Cancel { document });
        }
    }

    fn is_tokenizable(&self, document: &PipelineDocument) -> bool {
        !document.is_too_large
            && self.active.as_ref().is_some_and(|configuration| {
                configuration
                    .language_ids
                    .contains(&document.snapshot.metadata.language_id)
            })
    }

    fn accept(&mut self, response: WorkerResponse) -> bool {
        match response {
            WorkerResponse::Previewed { job, lines } => {
                let Some(preview) = self
                    .previews
                    .values_mut()
                    .find(|preview| preview.job == job)
                else {
                    return false;
                };
                let Some(active) = self
                    .active
                    .as_ref()
                    .filter(|active| active.generation == preview.generation)
                else {
                    return false;
                };
                if lines.len() != preview.text.len() {
                    return false;
                }
                let mut tokens = LineTokens::new(lines.len());
                for (line, tokenized) in lines.into_iter().enumerate() {
                    tokens.set_line(line, tokenized.spans, false);
                }
                preview.tokens = Some(Arc::new(PreviewTokens {
                    lines: tokens,
                    styles: active.style_table.clone(),
                }));
                true
            }
            WorkerResponse::Configured { generation, result } => {
                let Some(pending) = self
                    .pending
                    .take_if(|pending| pending.generation == generation)
                else {
                    return false;
                };
                match result {
                    Ok(style_table) => {
                        self.previews.clear();
                        self.configuration_error = None;
                        self.active = Some(ActiveConfiguration {
                            generation,
                            language_ids: pending.language_ids,
                            style_table,
                        });
                        for document in self.documents.values_mut() {
                            document.reset();
                        }
                        true
                    }
                    Err(error) => {
                        self.configuration_error = Some(error);
                        false
                    }
                }
            }
            WorkerResponse::Tokenized {
                job,
                document,
                first_line,
                lines,
                is_finished,
            } => {
                let Some(tracked) = self
                    .documents
                    .get_mut(&document)
                    .filter(|tracked| tracked.job == Some(job))
                else {
                    return false;
                };
                let next_line = first_line + lines.len();
                let has_lines = !lines.is_empty();
                tracked.store.accept(first_line, lines);
                if is_finished {
                    tracked.job = None;
                } else if !tracked.store.is_awaiting(next_line) {
                    self.cancel_job(document);
                }
                has_lines
            }
        }
    }

    fn configure(&mut self) {
        if !self.is_configuration_stale || self.pending.is_some() {
            return;
        }
        let Some(theme) = self.theme.clone() else {
            return;
        };
        if self.is_plugin_grammar_set_unsent {
            let plugin_grammars = self.plugin_grammars.clone();
            if !self.send(WorkerRequest::SetPluginGrammars(plugin_grammars)) {
                return;
            }
            self.is_plugin_grammar_set_unsent = false;
        }
        let generation = self.last_generation + 1;
        let language_ids = self.languages.ids().to_vec();
        let is_sent = self.send(WorkerRequest::Configure(Box::new(WorkerConfiguration {
            generation,
            language_ids: language_ids.clone(),
            theme,
            limits: self.limits,
        })));
        if !is_sent {
            return;
        }
        self.last_generation = generation;
        self.is_configuration_stale = false;
        self.pending = Some(PendingConfiguration {
            generation,
            language_ids: language_ids
                .into_iter()
                .chain(
                    self.plugin_grammars
                        .iter()
                        .map(|grammar| grammar.language_id().to_owned()),
                )
                .collect(),
        });
        for document in self.documents.values_mut() {
            document.job = None;
        }
    }

    fn start_jobs(&mut self) {
        if self.pending.is_some() || self.client.is_none() {
            return;
        }
        let Some(generation) = self
            .active
            .as_ref()
            .map(|configuration| configuration.generation)
        else {
            return;
        };
        let waiting: Vec<DocumentId> = self
            .documents
            .iter()
            .filter(|(_, document)| document.job.is_none() && self.is_tokenizable(document))
            .map(|(id, _)| *id)
            .collect();
        for id in waiting {
            let Some(document) = self.documents.get_mut(&id) else {
                continue;
            };
            let Some(plan) = document.store.plan() else {
                continue;
            };
            let job = self.last_job + 1;
            let request = WorkerRequest::Tokenize(Box::new(TokenizationJob {
                id: job,
                generation,
                snapshot: document.snapshot.clone(),
                plan,
                visible_lines: document.visible_lines.clone(),
            }));
            document.job = Some(job);
            self.last_job = job;
            if !self.send(request) {
                return;
            }
        }
    }
}

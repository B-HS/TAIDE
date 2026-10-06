use std::collections::HashMap;
use std::ops::Range;

use taide_native_editor::change_journal::ChangeSet;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::line_tokens::{LineTokens, TokenStyleTable};

use crate::document_tokens::{DocumentTokens, is_too_large_for_tokenization};
use crate::requested_languages::RequestedLanguages;
use crate::textmate_tokenizer::TokenizerLimits;
use crate::token_theme::TokenTheme;
use crate::token_worker::{
    TokenizationJob, WorkerClient, WorkerConfiguration, WorkerRequest, WorkerResponse,
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

pub struct TokenPipeline {
    client: Option<WorkerClient>,
    limits: TokenizerLimits,
    languages: RequestedLanguages,
    theme: Option<TokenTheme>,
    is_configuration_stale: bool,
    pending: Option<PendingConfiguration>,
    active: Option<ActiveConfiguration>,
    configuration_error: Option<SyntaxError>,
    last_generation: u64,
    last_job: u64,
    documents: HashMap<DocumentId, PipelineDocument>,
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
            theme: None,
            is_configuration_stale: false,
            pending: None,
            active: None,
            configuration_error: None,
            last_generation: 0,
            last_job: 0,
            documents: HashMap::new(),
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

    pub fn contains(&self, document: DocumentId) -> bool {
        self.documents.contains_key(&document)
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

    pub fn open(&mut self, snapshot: DocumentSnapshot) {
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
            WorkerResponse::Configured { generation, result } => {
                let Some(pending) = self
                    .pending
                    .take_if(|pending| pending.generation == generation)
                else {
                    return false;
                };
                match result {
                    Ok(style_table) => {
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
            language_ids,
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

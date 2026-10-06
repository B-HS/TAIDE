use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::line_tokens::TokenStyleTable;
use taide_native_editor::syntax::{Token, TokenKind};

use crate::bundled_grammars::bundled_grammar_set;
use crate::document_tokens::TokenizationPlan;
use crate::textmate_tokenizer::{EngineState, TextmateTokenizer, TokenizerLimits};
use crate::token_theme::TokenTheme;
use crate::tokenizer::{LineState, SyntaxError, TokenizedLine, UNSTYLED_STYLE_ID};

const VISIBLE_SLICE: Duration = Duration::from_millis(4);
const BACKGROUND_SLICE: Duration = Duration::from_millis(12);
const MAX_SLICE_LINES: usize = 1024;

pub type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Debug, Clone)]
pub struct WorkerConfiguration {
    pub generation: u64,
    pub language_ids: Vec<String>,
    pub theme: TokenTheme,
    pub limits: TokenizerLimits,
}

#[derive(Clone)]
pub struct TokenizationJob {
    pub id: u64,
    pub generation: u64,
    pub snapshot: DocumentSnapshot,
    pub plan: TokenizationPlan,
    pub visible_lines: Range<usize>,
}

pub enum WorkerRequest {
    Configure(Box<WorkerConfiguration>),
    Tokenize(Box<TokenizationJob>),
    Cancel {
        document: DocumentId,
    },
    SetVisibleLines {
        document: DocumentId,
        visible_lines: Range<usize>,
    },
}

#[derive(Debug)]
pub enum WorkerResponse {
    Configured {
        generation: u64,
        result: Result<TokenStyleTable, SyntaxError>,
    },
    Tokenized {
        job: u64,
        document: DocumentId,
        first_line: usize,
        lines: Vec<TokenizedLine>,
        is_finished: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerStopped;

pub struct WorkerClient {
    requests: Sender<WorkerRequest>,
    responses: Receiver<WorkerResponse>,
}

impl WorkerClient {
    pub fn send(&self, request: WorkerRequest) -> Result<(), WorkerStopped> {
        self.requests.send(request).map_err(|_| WorkerStopped)
    }

    pub fn try_receive(&self) -> Result<Option<WorkerResponse>, WorkerStopped> {
        match self.responses.try_recv() {
            Ok(response) => Ok(Some(response)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(WorkerStopped),
        }
    }
}

pub struct WorkerTask {
    requests: Receiver<WorkerRequest>,
    responses: Sender<WorkerResponse>,
    wake: Wake,
}

pub fn token_worker(wake: Wake) -> (WorkerClient, WorkerTask) {
    let (request_sender, request_receiver) = mpsc::channel();
    let (response_sender, response_receiver) = mpsc::channel();
    (
        WorkerClient {
            requests: request_sender,
            responses: response_receiver,
        },
        WorkerTask {
            requests: request_receiver,
            responses: response_sender,
            wake,
        },
    )
}

impl WorkerTask {
    pub fn run(self) {
        let mut state = WorkerState::default();
        loop {
            if state.jobs.is_empty() {
                let Ok(request) = self.requests.recv() else {
                    return;
                };
                if !self.answer(state.accept(request)) {
                    return;
                }
            }
            loop {
                match self.requests.try_recv() {
                    Ok(request) => {
                        if !self.answer(state.accept(request)) {
                            return;
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            if !self.answer(state.run_slice()) {
                return;
            }
        }
    }

    fn answer(&self, response: Option<WorkerResponse>) -> bool {
        let Some(response) = response else {
            return true;
        };
        let is_delivered = self.responses.send(response).is_ok();
        if is_delivered {
            (self.wake)();
        }
        is_delivered
    }
}

struct Engine {
    generation: u64,
    tokenizer: TextmateTokenizer,
}

impl Engine {
    fn new(configuration: &WorkerConfiguration) -> Result<(Self, TokenStyleTable), SyntaxError> {
        let language_ids: Vec<&str> = configuration
            .language_ids
            .iter()
            .map(String::as_str)
            .collect();
        let grammars = bundled_grammar_set(&language_ids)?;
        let tokenizer = TextmateTokenizer::new(
            &grammars,
            configuration.theme.settings(),
            configuration.limits,
        )?;
        let style_table = configuration
            .theme
            .style_table(&tokenizer)
            .map_err(|error| SyntaxError::InvalidTheme(format!("{error:?}")))?;
        Ok((
            Self {
                generation: configuration.generation,
                tokenizer,
            },
            style_table,
        ))
    }

    fn tokenize(
        &mut self,
        snapshot: &DocumentSnapshot,
        line: usize,
        previous: Option<&LineState>,
        text: &mut String,
    ) -> TokenizedLine {
        text.clear();
        text.extend(
            snapshot
                .rope
                .byte_slice(line_content_range(snapshot, line))
                .chunks(),
        );
        self.tokenizer
            .try_tokenize_line(&snapshot.metadata.language_id, text, previous)
            .unwrap_or_else(|_| TokenizedLine {
                spans: vec![0, UNSTYLED_STYLE_ID],
                kinds: vec![Token {
                    start_byte: 0,
                    kind: TokenKind::Other,
                }],
                end_state: previous
                    .cloned()
                    .unwrap_or_else(|| LineState(EngineState::initial())),
                is_stopped_early: false,
            })
    }
}

struct RunningJob {
    job: Box<TokenizationJob>,
    next_line: usize,
    state: Option<LineState>,
    line_count: usize,
}

impl RunningJob {
    fn is_visible_work(&self) -> bool {
        self.next_line < self.job.visible_lines.end.min(self.line_count)
    }
}

#[derive(Default)]
struct WorkerState {
    engine: Option<Engine>,
    jobs: VecDeque<RunningJob>,
    line_text: String,
}

impl WorkerState {
    fn accept(&mut self, request: WorkerRequest) -> Option<WorkerResponse> {
        match request {
            WorkerRequest::Configure(configuration) => {
                self.jobs.clear();
                let result = Engine::new(&configuration).map(|(engine, style_table)| {
                    self.engine = Some(engine);
                    style_table
                });
                Some(WorkerResponse::Configured {
                    generation: configuration.generation,
                    result,
                })
            }
            WorkerRequest::Tokenize(job) => {
                self.jobs
                    .retain(|running| running.job.snapshot.id != job.snapshot.id);
                let is_current = self
                    .engine
                    .as_ref()
                    .is_some_and(|engine| engine.generation == job.generation);
                if !is_current {
                    return Some(WorkerResponse::Tokenized {
                        job: job.id,
                        document: job.snapshot.id,
                        first_line: job.plan.start_line,
                        lines: Vec::new(),
                        is_finished: true,
                    });
                }
                self.jobs.push_back(RunningJob {
                    next_line: job.plan.start_line,
                    state: job.plan.start_state.clone(),
                    line_count: job.snapshot.rope.len_lines(),
                    job,
                });
                None
            }
            WorkerRequest::Cancel { document } => {
                self.jobs
                    .retain(|running| running.job.snapshot.id != document);
                None
            }
            WorkerRequest::SetVisibleLines {
                document,
                visible_lines,
            } => {
                if let Some(running) = self
                    .jobs
                    .iter_mut()
                    .find(|running| running.job.snapshot.id == document)
                {
                    running.job.visible_lines = visible_lines;
                }
                None
            }
        }
    }

    fn run_slice(&mut self) -> Option<WorkerResponse> {
        let Some(engine) = self.engine.as_mut() else {
            self.jobs.clear();
            return None;
        };
        let index = self
            .jobs
            .iter()
            .position(RunningJob::is_visible_work)
            .unwrap_or(0);
        let mut running = self.jobs.remove(index)?;
        let (stop_line, budget) = if running.is_visible_work() {
            (
                running.job.visible_lines.end.min(running.line_count),
                VISIBLE_SLICE,
            )
        } else {
            (running.line_count, BACKGROUND_SLICE)
        };
        let started = Instant::now();
        let first_line = running.next_line;
        let mut lines = Vec::new();
        let mut is_finished = running.next_line >= running.line_count;
        while !is_finished && running.next_line < stop_line && lines.len() < MAX_SLICE_LINES {
            let line = running.next_line;
            let tokenized = engine.tokenize(
                &running.job.snapshot,
                line,
                running.state.as_ref(),
                &mut self.line_text,
            );
            let is_converged = running
                .job
                .plan
                .known_end_state(line)
                .is_some_and(|known| known.is_same(&tokenized.end_state));
            running.state = Some(tokenized.end_state.clone());
            running.next_line += 1;
            lines.push(tokenized);
            is_finished = is_converged || running.next_line >= running.line_count;
            if started.elapsed() >= budget {
                break;
            }
        }
        let response = WorkerResponse::Tokenized {
            job: running.job.id,
            document: running.job.snapshot.id,
            first_line,
            lines,
            is_finished,
        };
        if !is_finished {
            self.jobs.push_back(running);
        }
        Some(response)
    }
}

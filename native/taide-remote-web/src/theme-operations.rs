use std::collections::BTreeMap;

use serde_json::{Value, json};
use taide_model::{
    error::{AppError, AppResult},
    theme::{ResolvedTheme, ThemeEditorContext, ThemeSummary},
};
use taide_native_ui::{
    settings_owner::Owner,
    theme_draft::{Mode, builtin_id},
    theme_edit::{Command, DeleteRequest, LoadRequest, Reply, SaveRequest},
};

use crate::{
    InvokeError, ResponsePayload,
    settings_catalog::{decode, invocation_error, json_payload},
    shell::{Call, Failure},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    List,
    Source,
    Base,
    Save,
    Delete,
}

enum Operation {
    Load {
        request: LoadRequest,
        themes: Option<Vec<ThemeSummary>>,
        source: Option<Box<ResolvedTheme>>,
        base: Option<Box<ResolvedTheme>>,
    },
    Save(Box<SaveRequest>),
    Delete(DeleteRequest),
}

impl Operation {
    fn owner(&self) -> &Owner {
        match self {
            Self::Load { request, .. } => request.owner(),
            Self::Save(request) => request.owner(),
            Self::Delete(request) => request.owner(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            Self::Load { request, .. } => request.is_active(),
            Self::Save(request) => request.is_active(),
            Self::Delete(request) => request.is_active(),
        }
    }

    fn is_mutation(&self) -> bool {
        !matches!(self, Self::Load { .. })
    }

    fn matches(&self, command: &Command) -> bool {
        match (self, command) {
            (Self::Load { request, .. }, Command::Load(other)) => request.same_request(other),
            (Self::Save(request), Command::Save(other)) => request.same_request(other),
            (Self::Delete(request), Command::Delete(other)) => request.same_request(other),
            _ => false,
        }
    }

    fn failed(self, error: AppError) -> Reply {
        let command = match self {
            Self::Load { request, .. } => Command::Load(request),
            Self::Save(request) => Command::Save(request),
            Self::Delete(request) => Command::Delete(request),
        };
        command.failed(error)
    }
}

pub struct Invocation {
    job: u64,
    stage: Stage,
    pub call: Call,
    pub owner: Owner,
}

#[derive(Default)]
pub struct ThemeOperations {
    sequence: u64,
    jobs: BTreeMap<u64, Operation>,
    pending: BTreeMap<u32, (u64, Stage)>,
    replies: Vec<Reply>,
    failures: Vec<Failure>,
}

impl ThemeOperations {
    pub fn submit(&mut self, command: Command) {
        if self
            .jobs
            .values()
            .any(|operation| operation.matches(&command))
        {
            return;
        }
        let Some(job) = self.sequence.checked_add(1) else {
            let error = invocation_error(InvokeError::SequenceExhausted);
            if !matches!(command, Command::Load(_)) {
                self.record_failure(&error);
            }
            self.replies.push(command.failed(error));
            return;
        };
        self.sequence = job;
        let operation = match command {
            Command::Load(request) => Operation::Load {
                request,
                themes: None,
                source: None,
                base: None,
            },
            Command::Save(request) => Operation::Save(request),
            Command::Delete(request) => Operation::Delete(request),
        };
        self.jobs.insert(job, operation);
    }

    pub fn next_calls(&self) -> Vec<Invocation> {
        let mut calls = Vec::new();
        for (job, operation) in &self.jobs {
            let stages = match operation {
                Operation::Load {
                    request,
                    themes,
                    source,
                    base,
                } => {
                    let mut stages = Vec::new();
                    if themes.is_none() {
                        stages.push((
                            Stage::List,
                            Call {
                                command: "theme_list",
                                args: Value::Null,
                            },
                        ));
                    }
                    if source.is_none() {
                        stages.push((
                            Stage::Source,
                            Call {
                                command: "theme_get",
                                args: json!({"themeId":request.source_id()}),
                            },
                        ));
                    }
                    if base.is_none()
                        && themes.is_some()
                        && let Some(source) = source
                    {
                        stages.push((
                            Stage::Base,
                            Call {
                                command: "theme_get",
                                args: json!({"themeId":builtin_id(source.theme_type)}),
                            },
                        ));
                    }
                    stages
                }
                Operation::Save(request) => vec![(
                    Stage::Save,
                    Call {
                        command: "theme_save",
                        args: json!({"theme":request.theme(),"editor":context(request.owner(), request.source_id(), request.mode())}),
                    },
                )],
                Operation::Delete(request) => vec![(
                    Stage::Delete,
                    Call {
                        command: "theme_delete",
                        args: json!({"themeId":request.source_id(),"editor":context(request.owner(), request.source_id(), Mode::Edit)}),
                    },
                )],
            };
            for (stage, call) in stages {
                if self
                    .pending
                    .values()
                    .any(|pending| *pending == (*job, stage))
                {
                    continue;
                }
                calls.push(Invocation {
                    job: *job,
                    stage,
                    call,
                    owner: operation.owner().clone(),
                });
            }
        }
        calls
    }

    pub fn is_active(&self, invocation: &Invocation) -> bool {
        self.jobs
            .get(&invocation.job)
            .is_some_and(Operation::is_active)
    }

    pub fn sent(&mut self, invocation: Invocation, seq: u32) {
        self.pending.insert(seq, (invocation.job, invocation.stage));
    }

    pub fn failed(&mut self, invocation: Invocation, error: AppError) {
        self.fail(invocation.job, error);
    }

    fn fail(&mut self, job: u64, error: AppError) {
        let Some(operation) = self.jobs.remove(&job) else {
            return;
        };
        if operation.is_mutation() {
            self.record_failure(&error);
        }
        self.replies.push(operation.failed(error));
    }

    fn record_failure(&mut self, error: &AppError) {
        self.failures.push(match serde_json::to_value(error) {
            Ok(value) => Failure::Remote(value),
            Err(_) => Failure::MalformedResponse,
        });
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        let Some((job, stage)) = self.pending.remove(&seq) else {
            return false;
        };
        if let Err(error) = self.apply(job, stage, json_payload(result)) {
            self.fail(job, error);
        }
        true
    }

    fn apply(&mut self, job: u64, stage: Stage, result: AppResult<Value>) -> AppResult<()> {
        let Some(operation) = self.jobs.get_mut(&job) else {
            return Ok(());
        };
        let value = result?;
        match (operation, stage) {
            (Operation::Load { themes, .. }, Stage::List) => *themes = Some(decode(value)?),
            (Operation::Load { source, .. }, Stage::Source) => {
                let resolved: ResolvedTheme = decode(value)?;
                *source = Some(Box::new(resolved));
            }
            (Operation::Load { base, .. }, Stage::Base) => *base = Some(Box::new(decode(value)?)),
            (Operation::Save(request), Stage::Save) => {
                let summary: ThemeSummary = decode(value)?;
                if summary.id != request.theme().id
                    || summary.theme_type != request.theme().theme_type
                    || summary.builtin
                {
                    return Err(AppError::Internal(
                        "remote saved theme does not match its request".into(),
                    ));
                }
                let Some(Operation::Save(request)) = self.jobs.remove(&job) else {
                    unreachable!()
                };
                self.replies.push(Reply::Saved {
                    request,
                    result: Ok(summary),
                });
                return Ok(());
            }
            (Operation::Delete(_), Stage::Delete) => {
                decode::<()>(value)?;
                let Some(Operation::Delete(request)) = self.jobs.remove(&job) else {
                    unreachable!()
                };
                self.replies.push(Reply::Deleted {
                    request,
                    result: Ok(()),
                });
                return Ok(());
            }
            _ => {
                return Err(AppError::Internal(
                    "remote theme response stage does not match its request".into(),
                ));
            }
        }
        if let Some(Operation::Load {
            request,
            themes: Some(themes),
            source: Some(source),
            base,
        }) = self.jobs.get_mut(&job)
            && themes
                .iter()
                .any(|theme| theme.id == request.source_id() && theme.builtin)
        {
            *base = Some(source.clone());
        }
        let ready = matches!(
            self.jobs.get(&job),
            Some(Operation::Load {
                themes: Some(_),
                source: Some(_),
                base: Some(_),
                ..
            })
        );
        if !ready {
            return Ok(());
        }
        let Some(Operation::Load {
            request,
            themes: Some(themes),
            source: Some(source),
            base: Some(base),
        }) = self.jobs.remove(&job)
        else {
            unreachable!()
        };
        let result = request.resolve(&themes, *source, *base).map(Box::new);
        self.replies.push(Reply::Loaded { request, result });
        Ok(())
    }

    pub fn has_pending_mutations(&self) -> bool {
        self.jobs.values().any(Operation::is_mutation)
    }

    pub fn failures(&self) -> &[Failure] {
        &self.failures
    }

    pub fn take_failures(&mut self) -> Vec<Failure> {
        std::mem::take(&mut self.failures)
    }

    pub fn take_replies(&mut self) -> Vec<Reply> {
        std::mem::take(&mut self.replies)
    }

    pub fn disconnected(&mut self) {
        self.pending.clear();
        for job in self.jobs.keys().copied().collect::<Vec<_>>() {
            self.fail(job, invocation_error(InvokeError::Closed));
        }
    }
}

fn context(owner: &Owner, source: &str, mode: Mode) -> ThemeEditorContext {
    ThemeEditorContext {
        project_id: owner.project.clone(),
        pane_id: owner.pane.clone(),
        tab_id: owner.tab.clone(),
        source_theme_id: source.into(),
        is_create: mode == Mode::Create,
    }
}

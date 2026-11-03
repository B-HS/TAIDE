use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use taide_model::ide::{IdeDiagnostic, IdeDiagnosticSeverity};
use taide_model::ids::ProjectId;
use taide_native_editor::document::{DocumentId, DocumentKey};

/// Original `IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS`.
pub(crate) const IDE_DIAGNOSTICS_PUSH_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone)]
struct Signature {
    revision: Arc<()>,
    project: Option<ProjectId>,
    running: bool,
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.revision, &other.revision)
            && self.project == other.project
            && self.running == other.running
    }
}

/// What the diagnostics push wants from this tick.
#[derive(Debug, PartialEq)]
pub(crate) enum Push {
    Idle,
    Wait(Duration),
    Due(ProjectId),
}

/// The IDE sync provider's state: save requests waiting for their host reply and the debounced
/// diagnostics push.
#[derive(Default)]
pub(crate) struct State {
    saves: HashMap<DocumentId, Vec<String>>,
    signature: Option<Signature>,
    deadline: Option<Instant>,
}

impl State {
    pub(crate) fn wait_for_save(&mut self, document: DocumentId, request: String) {
        self.saves.entry(document).or_default().push(request);
    }

    pub(crate) fn take_saves(&mut self, document: DocumentId) -> Vec<String> {
        self.saves.remove(&document).unwrap_or_default()
    }

    /// Original effect over `[markers, activeProjectId, ideStatus.running]`: every change restarts
    /// the debounce while a focused project exists and the IDE server runs, and clears it
    /// otherwise.
    pub(crate) fn diagnostics_push(
        &mut self,
        revision: &Arc<()>,
        project: Option<&ProjectId>,
        running: bool,
        now: Instant,
    ) -> Push {
        let signature = Signature {
            revision: revision.clone(),
            project: project.cloned(),
            running,
        };
        if self.signature.as_ref() != Some(&signature) {
            self.deadline =
                (project.is_some() && running).then(|| now + IDE_DIAGNOSTICS_PUSH_DEBOUNCE);
            self.signature = Some(signature);
        }
        let Some(deadline) = self.deadline else {
            return Push::Idle;
        };
        if now < deadline {
            return Push::Wait(deadline - now);
        }
        self.deadline = None;
        project.cloned().map_or(Push::Idle, Push::Due)
    }
}

/// The marker list the original sends, built from every diagnostics batch of a file document;
/// positions stay LSP zero-based and a missing severity reads as an error like the problems view.
pub(crate) fn diagnostics(store: &crate::diagnostics::Store) -> Vec<IdeDiagnostic> {
    store
        .batches()
        .filter_map(|batch| match &batch.key {
            DocumentKey::File(path) => Some((path.to_string_lossy().into_owned(), batch)),
            DocumentKey::Untitled(_) | DocumentKey::AppFile(_) | DocumentKey::Virtual(_) => None,
        })
        .flat_map(|(path, batch)| {
            batch
                .diagnostics
                .iter()
                .map(move |diagnostic| IdeDiagnostic {
                    path: path.clone(),
                    severity: match crate::diagnostics::severity(diagnostic) {
                        crate::diagnostics::WARNING => IdeDiagnosticSeverity::Warning,
                        crate::diagnostics::INFO => IdeDiagnosticSeverity::Info,
                        crate::diagnostics::HINT => IdeDiagnosticSeverity::Hint,
                        _ => IdeDiagnosticSeverity::Error,
                    },
                    start_line: diagnostic.range.start.line,
                    start_character: diagnostic.range.start.character,
                    end_line: diagnostic.range.end.line,
                    end_character: diagnostic.range.end.character,
                    message: diagnostic.message.clone(),
                    source: diagnostic.source.clone(),
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ide_진단_push는_변경마다_300ms를_다시_세고_조건이_빠지면_멈춘다() {
        let mut state = State::default();
        let start = Instant::now();
        let project = ProjectId::new();
        let first = Arc::new(());
        assert_eq!(
            state.diagnostics_push(&first, Some(&project), true, start),
            Push::Wait(IDE_DIAGNOSTICS_PUSH_DEBOUNCE)
        );
        let second = Arc::new(());
        let later = start + IDE_DIAGNOSTICS_PUSH_DEBOUNCE / 2;
        assert_eq!(
            state.diagnostics_push(&second, Some(&project), true, later),
            Push::Wait(IDE_DIAGNOSTICS_PUSH_DEBOUNCE)
        );
        assert_eq!(
            state.diagnostics_push(
                &second,
                Some(&project),
                true,
                later + IDE_DIAGNOSTICS_PUSH_DEBOUNCE
            ),
            Push::Due(project.clone())
        );
        assert_eq!(
            state.diagnostics_push(
                &second,
                Some(&project),
                true,
                later + IDE_DIAGNOSTICS_PUSH_DEBOUNCE * 2
            ),
            Push::Idle
        );
        assert_eq!(
            state.diagnostics_push(&second, Some(&project), false, later),
            Push::Idle
        );
        assert_eq!(
            state.diagnostics_push(&second, None, true, later),
            Push::Idle
        );
    }

    #[test]
    fn ide_진단_push는_포커스_프로젝트가_바뀌면_새_프로젝트로_다시_센다() {
        let mut state = State::default();
        let start = Instant::now();
        let revision = Arc::new(());
        let first = ProjectId::new();
        let second = ProjectId::new();
        assert_eq!(
            state.diagnostics_push(&revision, Some(&first), true, start),
            Push::Wait(IDE_DIAGNOSTICS_PUSH_DEBOUNCE)
        );
        let due = start + IDE_DIAGNOSTICS_PUSH_DEBOUNCE;
        assert_eq!(
            state.diagnostics_push(&revision, Some(&first), true, due),
            Push::Due(first.clone())
        );
        assert_eq!(
            state.diagnostics_push(&revision, Some(&second), true, due),
            Push::Wait(IDE_DIAGNOSTICS_PUSH_DEBOUNCE)
        );
        assert_eq!(
            state.diagnostics_push(
                &revision,
                Some(&second),
                true,
                due + IDE_DIAGNOSTICS_PUSH_DEBOUNCE
            ),
            Push::Due(second.clone())
        );
    }

    #[test]
    fn ide_저장요청의_탭찾기는_보조창_트리와_정확한_경로_문자열을_본다() {
        use crate::tabs::project_has_file_tab;
        use taide_model::ids::{PaneId, TabId};
        use taide_model::layout::{AuxWindowLayout, PaneNode, Tab, TabKind};
        let tab = |path: &str| Tab {
            id: TabId::new(),
            kind: TabKind::File { path: path.into() },
            title: path.into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        };
        let mut layout = taide_layout::service::default_layout();
        assert!(!project_has_file_tab(&layout, "/work/aux.md"));
        let pane = PaneId::new();
        layout.auxiliary_windows.push(AuxWindowLayout {
            slot: 1,
            root: PaneNode::Leaf {
                id: pane.clone(),
                tabs: vec![tab("/work/aux.md")],
                active: None,
            },
            focused_pane: pane,
        });
        assert!(project_has_file_tab(&layout, "/work/aux.md"));
        assert!(!project_has_file_tab(&layout, "/work/./aux.md"));
    }
}

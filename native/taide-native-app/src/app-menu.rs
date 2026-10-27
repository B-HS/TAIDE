use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use eframe::egui;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::locale::ResolvedLocale;
use taide_model::project::Project;
use taide_runtime::{AppState, TaskSupervisor};
use tokio::sync::oneshot;

use crate::presentation::message;

pub(crate) const FILE_ID: &str = "taide-file";
pub(crate) const OPEN_RECENT_ID: &str = "taide-open-recent";
pub(crate) const CLEAR_RECENT_ID: &str = "taide-clear-recent";
pub(crate) const QUIT_ID: &str = "taide-quit";
pub(crate) const NO_RECENT_ID: &str = "taide-no-recent";
const RECENT_ID_PREFIX: &str = "taide-recent:";
pub(crate) const UNDO_ID: &str = "taide-edit-undo";
pub(crate) const REDO_ID: &str = "taide-edit-redo";
pub(crate) const CUT_ID: &str = "taide-edit-cut";
pub(crate) const COPY_ID: &str = "taide-edit-copy";
pub(crate) const PASTE_ID: &str = "taide-edit-paste";
pub(crate) const SELECT_ALL_ID: &str = "taide-edit-select-all";
const APP_TITLE: &str = "TAIDE";
const EDIT_TITLE: &str = "Edit";
const WINDOW_TITLE: &str = "Window";
const QUIT_TEXT: &str = "Quit";
const UNDO_TEXT: &str = "Undo";
const REDO_TEXT: &str = "Redo";
const CUT_TEXT: &str = "Cut";
const COPY_TEXT: &str = "Copy";
const PASTE_TEXT: &str = "Paste";
const SELECT_ALL_TEXT: &str = "Select All";
const MNEMONIC: char = '&';
const WORKER_NAME: &str = "native-app-menu-recent";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Quit,
    OpenRecent(ProjectId),
    ClearRecent,
    Edit(Edit),
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Request(egui::ViewportCommand),
    Chord(egui::Key, egui::Modifiers),
}

impl Edit {
    pub(crate) fn input(self) -> Input {
        let command = command_modifiers();
        match self {
            Self::Undo => Input::Chord(egui::Key::Z, command),
            Self::Redo => Input::Chord(egui::Key::Z, command | egui::Modifiers::SHIFT),
            Self::Cut => Input::Request(egui::ViewportCommand::RequestCut),
            Self::Copy => Input::Request(egui::ViewportCommand::RequestCopy),
            Self::Paste => Input::Request(egui::ViewportCommand::RequestPaste),
            Self::SelectAll => Input::Chord(egui::Key::A, command),
        }
    }
}

fn command_modifiers() -> egui::Modifiers {
    if cfg!(target_os = "macos") {
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
    } else {
        egui::Modifiers::CTRL | egui::Modifiers::COMMAND
    }
}

pub(crate) fn action(id: &str) -> Action {
    match id {
        QUIT_ID => Action::Quit,
        CLEAR_RECENT_ID => Action::ClearRecent,
        UNDO_ID => Action::Edit(Edit::Undo),
        REDO_ID => Action::Edit(Edit::Redo),
        CUT_ID => Action::Edit(Edit::Cut),
        COPY_ID => Action::Edit(Edit::Copy),
        PASTE_ID => Action::Edit(Edit::Paste),
        SELECT_ALL_ID => Action::Edit(Edit::SelectAll),
        _ => recent_project(id).map_or(Action::Ignored, Action::OpenRecent),
    }
}

pub(crate) fn recent_id(project: &ProjectId) -> String {
    format!("{RECENT_ID_PREFIX}{project}")
}

fn recent_project(id: &str) -> Option<ProjectId> {
    id.strip_prefix(RECENT_ID_PREFIX)
        .filter(|rest| !rest.is_empty())
        .map(|rest| ProjectId::from(rest.to_owned()))
}

pub(crate) fn literal_text(text: &str) -> String {
    text.replace(MNEMONIC, "&&")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Standard {
    About,
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Minimize,
    Maximize,
    Fullscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shortcut {
    CommandQ,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Item {
        id: String,
        text: String,
        enabled: bool,
        shortcut: Option<Shortcut>,
    },
    Separator,
    Standard(Standard),
    Submenu {
        id: &'static str,
        title: String,
        rows: Vec<Row>,
    },
}

impl Row {
    fn item(id: &str, text: &str, enabled: bool) -> Self {
        Self::Item {
            id: id.to_owned(),
            text: text.to_owned(),
            enabled,
            shortcut: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Submenu {
    pub(crate) id: Option<&'static str>,
    pub(crate) title: String,
    pub(crate) rows: Vec<Row>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Model {
    pub(crate) file: String,
    pub(crate) open_recent: String,
    pub(crate) recent: Vec<Row>,
}

impl Model {
    pub(crate) fn new(locale: &ResolvedLocale, projects: &[Project]) -> Self {
        let mut recent: Vec<Row> = projects
            .iter()
            .take(crate::welcome::RECENT_MENU_LIMIT)
            .map(|project| {
                Row::item(
                    &recent_id(&project.id),
                    label(project),
                    !project.root_missing,
                )
            })
            .collect();
        if recent.is_empty() {
            recent.push(Row::item(
                NO_RECENT_ID,
                &message(locale, "menu.noRecentProjects", &[]),
                false,
            ));
        }
        recent.push(Row::Separator);
        recent.push(Row::item(
            CLEAR_RECENT_ID,
            &message(locale, "menu.clearRecent", &[]),
            !projects.is_empty(),
        ));
        Self {
            file: message(locale, "menu.file", &[]),
            open_recent: message(locale, "menu.openRecent", &[]),
            recent,
        }
    }

    pub(crate) fn bar(&self) -> Vec<Submenu> {
        let edit = [
            (UNDO_ID, UNDO_TEXT),
            (REDO_ID, REDO_TEXT),
            (CUT_ID, CUT_TEXT),
            (COPY_ID, COPY_TEXT),
            (PASTE_ID, PASTE_TEXT),
            (SELECT_ALL_ID, SELECT_ALL_TEXT),
        ]
        .map(|(id, text)| Row::item(id, text, true));
        let [undo, redo, cut, copy, paste, select_all] = edit;
        vec![
            Submenu {
                id: None,
                title: APP_TITLE.into(),
                rows: vec![
                    Row::Standard(Standard::About),
                    Row::Separator,
                    Row::Standard(Standard::Services),
                    Row::Separator,
                    Row::Standard(Standard::Hide),
                    Row::Standard(Standard::HideOthers),
                    Row::Standard(Standard::ShowAll),
                    Row::Separator,
                    Row::Item {
                        id: QUIT_ID.into(),
                        text: QUIT_TEXT.into(),
                        enabled: true,
                        shortcut: Some(Shortcut::CommandQ),
                    },
                ],
            },
            Submenu {
                id: Some(FILE_ID),
                title: self.file.clone(),
                rows: vec![Row::Submenu {
                    id: OPEN_RECENT_ID,
                    title: self.open_recent.clone(),
                    rows: self.recent.clone(),
                }],
            },
            Submenu {
                id: None,
                title: EDIT_TITLE.into(),
                rows: vec![undo, redo, Row::Separator, cut, copy, paste, select_all],
            },
            Submenu {
                id: None,
                title: WINDOW_TITLE.into(),
                rows: vec![
                    Row::Standard(Standard::Minimize),
                    Row::Standard(Standard::Maximize),
                    Row::Separator,
                    Row::Standard(Standard::Fullscreen),
                ],
            },
        ]
    }
}

fn label(project: &Project) -> &str {
    project
        .display
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or(&project.name)
}

pub(crate) trait Bar {
    fn apply(&mut self, model: &Model);
}

#[derive(Default)]
pub(crate) struct Changes {
    revision: AtomicU64,
    kept_drafts: Mutex<Vec<u32>>,
}

impl Changes {
    pub(crate) fn record(&self, event: &AppEvent) {
        match event {
            AppEvent::ProjectListChanged { .. } | AppEvent::ProjectActivated { .. } => {
                self.revision.fetch_add(1, Ordering::AcqRel);
            }
            AppEvent::ProjectRecentCleared {
                skipped_with_drafts,
                ..
            } if *skipped_with_drafts > 0 => self
                .kept_drafts
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(*skipped_with_drafts),
            _ => {}
        }
    }

    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    fn take_kept_drafts(&self) -> Vec<u32> {
        std::mem::take(
            &mut *self
                .kept_drafts
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        )
    }
}

type Recent = AppResult<Vec<Project>>;

pub(crate) struct State {
    changes: Arc<Changes>,
    bar: Option<Box<dyn Bar>>,
    events: Option<mpsc::Receiver<String>>,
    loaded: Option<u64>,
    pending: Option<(u64, oneshot::Receiver<Recent>)>,
    recent: Vec<Project>,
    drawn: Option<Model>,
    chord: Option<(egui::ViewportId, egui::Key, egui::Modifiers)>,
}

impl State {
    pub(crate) fn new(changes: Arc<Changes>) -> Self {
        Self {
            changes,
            bar: None,
            events: None,
            loaded: None,
            pending: None,
            recent: Vec::new(),
            drawn: None,
            chord: None,
        }
    }

    pub(crate) fn connect(&mut self, bar: Box<dyn Bar>, events: mpsc::Receiver<String>) {
        self.bar = Some(bar);
        self.events = Some(events);
        self.loaded = None;
        self.drawn = None;
    }

    pub(crate) fn tick(
        &mut self,
        locale: &ResolvedLocale,
        state: &AppState,
        tasks: &TaskSupervisor,
        context: &egui::Context,
    ) -> Vec<Action> {
        let Some(bar) = self.bar.as_mut() else {
            return Vec::new();
        };
        if let Some((revision, receiver)) = &mut self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.recent = result.unwrap_or_else(|error| {
                        log::warn!("native recent menu read failed: {:?}", error.kind());
                        Vec::new()
                    });
                    self.loaded = Some(*revision);
                    self.pending = None;
                }
                Err(oneshot::error::TryRecvError::Empty) => {}
                Err(oneshot::error::TryRecvError::Closed) => self.pending = None,
            }
        }
        let revision = self.changes.revision();
        if self.pending.is_none() && self.loaded != Some(revision) {
            let (sender, receiver) = oneshot::channel();
            let state = state.clone();
            let repaint = context.clone();
            let started = tasks.spawn_transient(WORKER_NAME, async move {
                let result = taide_runtime::project_actions::project_list_recent(&state).await;
                if sender.send(result).is_ok() {
                    repaint.request_repaint();
                }
            });
            if started {
                self.pending = Some((revision, receiver));
            }
        }
        if self.loaded.is_some() {
            let model = Model::new(locale, &self.recent);
            if self.drawn.as_ref() != Some(&model) {
                bar.apply(&model);
                self.drawn = Some(model);
            }
        }
        self.events
            .as_ref()
            .map(|events| events.try_iter().map(|id| action(&id)).collect())
            .unwrap_or_default()
    }

    pub(crate) fn take_kept_drafts(&self) -> Vec<u32> {
        self.changes.take_kept_drafts()
    }

    pub(crate) fn queue_chord(
        &mut self,
        viewport: egui::ViewportId,
        key: egui::Key,
        modifiers: egui::Modifiers,
    ) {
        self.chord = Some((viewport, key, modifiers));
    }

    pub(crate) fn inject(&mut self, viewport: egui::ViewportId, events: &mut Vec<egui::Event>) {
        if self.chord.is_none_or(|(target, _, _)| target != viewport) {
            return;
        }
        let Some((_, key, modifiers)) = self.chord.take() else {
            return;
        };
        events.extend([true, false].map(|pressed| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        }));
    }

    pub(crate) fn finish_child_passes(&mut self) {
        if self
            .chord
            .is_some_and(|(target, _, _)| target != egui::ViewportId::ROOT)
        {
            self.chord = None;
        }
    }
}

#[cfg(test)]
#[path = "app-menu-tests.rs"]
mod tests;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use taide_lsp::native::Failure;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_native_editor::document::EditorError;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;
use taide_native_ui::editor_code_actions::{Command, Session};
use tokio::sync::oneshot;

use crate::editor_code_actions::{
    Candidate, Chosen, Finished, Invocation, Navigation, Request, Resolved, State,
};
use crate::lsp::LspBridge;

const AUTOMATIC_DELAY: Duration = Duration::from_millis(250);
const BULB_WIDTH: f32 = 22.0;

fn whitespace(character: char) -> bool {
    character == '\u{feff}' || (character.is_whitespace() && character != '\u{85}')
}

fn on_code(store: &EditorStore, view: ViewId) -> bool {
    let Some(view) = store.views().get(view) else {
        return false;
    };
    let Some(selection) = view.selection.selections.get(view.selection.primary) else {
        return false;
    };
    if selection.anchor != selection.head {
        return true;
    }
    let Ok(snapshot) = store.documents().snapshot(view.document) else {
        return false;
    };
    let line = snapshot.rope.byte_to_line(selection.head);
    let start = snapshot.rope.line_to_byte(line);
    let content = snapshot.rope.line(line).to_string();
    let content = content.trim_end_matches(['\r', '\n']);
    let offset = selection.head.saturating_sub(start).min(content.len());
    let left = content
        .get(..offset)
        .and_then(|text| text.chars().next_back());
    let right = content.get(offset..).and_then(|text| text.chars().next());
    left.is_some_and(|character| !whitespace(character))
        || right.is_some_and(|character| !whitespace(character))
}

fn bulb_position(
    store: &EditorStore,
    view: ViewId,
    request: &Request,
    tab_size: u32,
    space_width: f32,
) -> Option<taide_native_ui::editor_code_actions::Bulb> {
    let current = store.views().get(view)?;
    let snapshot = &request.snapshot;
    let line = request.range.start.line as usize;
    let line_count = snapshot.rope.len_lines();
    if line >= line_count {
        return None;
    }
    let content = |line| {
        snapshot
            .rope
            .line(line)
            .to_string()
            .trim_end_matches(['\r', '\n'])
            .to_owned()
    };
    let current_content = content(line);
    let indent = current_content
        .chars()
        .take_while(|character| matches!(character, ' ' | '\t'))
        .fold(0, |column, character| {
            if character == '\t' {
                column + tab_size - column % tab_size
            } else {
                column + 1
            }
        });
    let folded = |line| {
        let start = snapshot.rope.line_to_byte(line);
        current
            .folds
            .iter()
            .any(|fold| fold.start < start && start < fold.end)
    };
    let empty_or_indented = |line| {
        let text = content(line);
        text.chars().next().is_none_or(whitespace) || text.encode_utf16().count() <= 1
    };
    let mut effective = line;
    let mut gutter = false;
    if indent as f32 * space_width <= BULB_WIDTH {
        if line > 0 && !folded(line - 1) {
            let above = empty_or_indented(line - 1);
            let below = line + 1 < line_count && !folded(line + 1) && empty_or_indented(line + 1);
            if !above && !below {
                gutter = true;
            } else if above || line + 1 == line_count {
                effective -= 1;
            } else {
                effective += 1;
            }
        } else if line == 0
            && (line_count == 1 || (!empty_or_indented(line + 1) && !empty_or_indented(line)))
        {
            gutter = true;
        } else if line + 1 < line_count && !folded(line + 1) {
            effective += 1;
        } else if (request.range.start.character + 1) as f32 * space_width < BULB_WIDTH {
            return None;
        }
    }
    let text = content(effective);
    let mut characters = text.chars();
    let first = characters.next();
    let after_single = !gutter
        && indent as f32 * space_width <= BULB_WIDTH
        && first.is_some_and(|character| !whitespace(character))
        && characters.all(whitespace);
    let anchor =
        snapshot.rope.line_to_byte(effective) + if after_single { first?.len_utf8() } else { 0 };
    Some(taide_native_ui::editor_code_actions::Bulb { anchor, gutter })
}

struct Observed {
    request: Request,
    providers: HashSet<crate::editor_symbols::ProviderIdentity>,
    diagnostics: HashMap<crate::diagnostics::Owner, u64>,
}

const LIGHTBULB_COLOR: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(0xff, 0xcc, 0x00);
const AUTO_FIX_COLOR: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(0x75, 0xbe, 0xff);
const LIGHTBULB_LIGHT_COLOR: eframe::egui::Color32 =
    eframe::egui::Color32::from_rgb(0xdd, 0xb1, 0x00);
const AUTO_FIX_LIGHT_COLOR: eframe::egui::Color32 =
    eframe::egui::Color32::from_rgb(0x00, 0x7a, 0xcc);
const DISABLED_DARK: [u8; 4] = [0xcc, 0xcc, 0xcc, 0x80];
const DISABLED_LIGHT: [u8; 4] = [0x61, 0x61, 0x61, 0x80];

pub(crate) fn colors(
    theme: &taide_model::theme::ResolvedTheme,
) -> AppResult<taide_native_ui::editor_code_actions::Colors> {
    let color = |key: &str| taide_native_ui::presentation::color(theme, key);
    let dark = theme.theme_type == taide_model::theme::ThemeType::Dark;
    let [red, green, blue, alpha] = if dark { DISABLED_DARK } else { DISABLED_LIGHT };
    Ok(taide_native_ui::editor_code_actions::Colors {
        background: color("menu.background")?,
        foreground: color("app.foreground")?,
        disabled: eframe::egui::Color32::from_rgba_unmultiplied(red, green, blue, alpha),
        selected_background: color("menu.itemHover")?,
        selected_foreground: color("app.foreground")?,
        hover_background: color("menu.itemHover")?,
        border: color("menu.border")?,
        shadow: color("app.shadow")?,
        lightbulb: if dark {
            LIGHTBULB_COLOR
        } else {
            LIGHTBULB_LIGHT_COLOR
        },
        lightbulb_auto_fix: if dark {
            AUTO_FIX_COLOR
        } else {
            AUTO_FIX_LIGHT_COLOR
        },
    })
}

struct Pending {
    request: Request,
    receive: oneshot::Receiver<Result<Vec<Candidate>, Failure>>,
}

struct Resolving {
    chosen: Chosen,
    receive: oneshot::Receiver<Result<Resolved, Failure>>,
}

struct Applying {
    chosen: Chosen,
    receive: oneshot::Receiver<AppResult<Option<Navigation>>>,
}

fn operation_error(detail: impl Into<String>) -> AppError {
    AppError::localized(
        AppErrorKind::Internal,
        "editor.workspaceEditApplyFailed",
        detail,
    )
}

fn silent(error: &Failure) -> bool {
    matches!(
        error,
        Failure::Cancelled
            | Failure::StaleRevision
            | Failure::StaleGeneration
            | Failure::Restarted
            | Failure::Stopped
    )
}

#[derive(Default)]
pub(crate) struct Controller {
    pub(crate) state: State,
    pending: Option<Pending>,
    resolving: Option<Resolving>,
    applying: Option<Applying>,
    error: Option<AppError>,
    observed: Option<Observed>,
    scheduled: Option<(Request, Instant)>,
}

impl Controller {
    pub(crate) fn clear(&mut self) {
        self.state.clear();
        self.pending = None;
        self.resolving = None;
        self.applying = None;
        self.error = None;
        self.observed = None;
        self.scheduled = None;
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        lsp: &LspBridge,
        active: impl FnOnce(&Request) -> bool,
    ) {
        self.state.reconcile(store, active, |request| {
            lsp.code_action_providers(
                &request.project,
                &request.snapshot,
                request.invocation.command,
            )
        });
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.request.is_cancelled())
        {
            self.pending = None;
        }
        if self
            .resolving
            .as_ref()
            .is_some_and(|pending| pending.chosen.request.is_cancelled())
        {
            self.resolving = None;
        }
        if self
            .scheduled
            .as_ref()
            .is_some_and(|(request, _)| request.is_cancelled())
        {
            self.scheduled = None;
        }
    }

    pub(crate) fn next_refresh(&self, now: Instant) -> Option<Duration> {
        self.scheduled
            .as_ref()
            .map(|(_, deadline)| deadline.saturating_duration_since(now))
    }

    pub(crate) fn dispatch(&mut self, lsp: &LspBridge, now: Instant) {
        if !self
            .scheduled
            .as_ref()
            .is_some_and(|(_, deadline)| *deadline <= now)
        {
            return;
        }
        let (request, _) = self.scheduled.take().unwrap();
        if request.is_cancelled() {
            return;
        }
        match lsp.code_actions(request.clone()) {
            Ok(receive) => self.pending = Some(Pending { request, receive }),
            Err(error) => {
                self.state.finish(&request);
                self.error = Some(operation_error(error.to_string()));
            }
        }
    }

    fn observe(
        &mut self,
        store: &EditorStore,
        project: Option<&ProjectId>,
        source: ViewId,
        owner: ViewId,
        lsp: Option<&LspBridge>,
        now: Instant,
    ) {
        if self.applying.is_some() || self.resolving.is_some() {
            return;
        }
        if self
            .state
            .request()
            .is_some_and(|request| !request.invocation.automatic && request.describes(store))
        {
            return;
        }
        let Some((project, lsp)) = project.zip(lsp) else {
            return;
        };
        let Some(view) = store.views().get(source) else {
            return;
        };
        let Ok(snapshot) = store.documents().snapshot(view.document) else {
            return;
        };
        if !on_code(store, source) {
            self.state.clear();
            self.pending = None;
            self.scheduled = None;
            return;
        }
        let providers = lsp.code_action_providers(project, &snapshot, Command::QuickFix);
        let diagnostics = lsp.code_action_diagnostics(project, snapshot.id);
        if self.observed.as_ref().is_some_and(|observed| {
            observed.request.project == *project
                && observed.request.source == source
                && observed.request.owner == owner
                && observed.request.matches(store)
                && observed.providers == providers
                && observed.diagnostics == diagnostics
        }) {
            return;
        }
        self.pending = None;
        self.scheduled = None;
        let Ok(Some(request)) = self.state.begin(
            store,
            project.clone(),
            source,
            owner,
            providers.clone(),
            Invocation {
                command: Command::QuickFix,
                automatic: true,
            },
        ) else {
            return;
        };
        self.observed = Some(Observed {
            request: request.clone(),
            providers,
            diagnostics,
        });
        self.scheduled = Some((request, now + AUTOMATIC_DELAY));
    }

    pub(crate) fn begin(
        &mut self,
        store: &EditorStore,
        project: ProjectId,
        source: ViewId,
        owner: ViewId,
        invocation: Invocation,
        lsp: &LspBridge,
    ) -> Result<bool, EditorError> {
        if self.applying.is_some() {
            return Ok(false);
        }
        self.clear();
        let view = store.views().get(source).ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(view.document)?;
        let providers = lsp.code_action_providers(&project, &snapshot, invocation.command);
        let Some(request) = self
            .state
            .begin(store, project, source, owner, providers, invocation)?
        else {
            return Ok(false);
        };
        match lsp.code_actions(request.clone()) {
            Ok(receive) => self.pending = Some(Pending { request, receive }),
            Err(error) => {
                self.state.finish(&request);
                self.error = Some(operation_error(error.to_string()));
            }
        }
        Ok(true)
    }

    fn choose(&mut self, chosen: Chosen, lsp: &LspBridge) {
        match lsp.resolve_code_action(chosen.clone()) {
            Ok(receive) => self.resolving = Some(Resolving { chosen, receive }),
            Err(error) => {
                self.state.finish(&chosen.request);
                self.error = Some(operation_error(error.to_string()));
            }
        }
    }

    pub(crate) fn poll(
        &mut self,
        store: &EditorStore,
        lsp: &LspBridge,
    ) -> (Option<AppError>, Option<Finished>) {
        if let Some(mut pending) = self.pending.take() {
            if !pending.request.is_cancelled() {
                let response = match pending.receive.try_recv() {
                    Ok(response) => Some(response),
                    Err(oneshot::error::TryRecvError::Empty) => None,
                    Err(oneshot::error::TryRecvError::Closed) => {
                        Some(Err(Failure::TransportClosed))
                    }
                };
                if let Some(response) = response {
                    let providers = lsp.code_action_providers(
                        &pending.request.project,
                        &pending.request.snapshot,
                        pending.request.invocation.command,
                    );
                    match self
                        .state
                        .accept(store, &pending.request, providers, response)
                    {
                        Ok(Some(chosen)) => self.choose(chosen, lsp),
                        Ok(None) => (),
                        Err(error) if silent(&error) => (),
                        Err(error) => {
                            self.error =
                                Some(operation_error(format!("native code actions: {error:?}")))
                        }
                    }
                } else {
                    self.pending = Some(pending);
                }
            }
        }
        if let Some(mut resolving) = self.resolving.take() {
            if !resolving.chosen.request.is_cancelled() {
                let response = match resolving.receive.try_recv() {
                    Ok(response) => Some(response),
                    Err(oneshot::error::TryRecvError::Empty) => None,
                    Err(oneshot::error::TryRecvError::Closed) => {
                        Some(Err(Failure::TransportClosed))
                    }
                };
                if let Some(response) = response {
                    let request = &resolving.chosen.request;
                    match response {
                        Ok(resolved) => {
                            let providers = lsp.code_action_providers(
                                &request.project,
                                &request.snapshot,
                                request.invocation.command,
                            );
                            if resolved.provider == resolving.chosen.candidate.provider
                                && self.state.begin_apply(store, &resolving.chosen, providers)
                            {
                                match lsp.apply_code_action(resolving.chosen.clone(), resolved) {
                                    Ok(receive) => {
                                        self.applying = Some(Applying {
                                            chosen: resolving.chosen,
                                            receive,
                                        })
                                    }
                                    Err(error) => {
                                        self.state.finish(request);
                                        self.error = Some(operation_error(error.to_string()));
                                    }
                                }
                            } else {
                                self.state.finish(request);
                            }
                        }
                        Err(error) => {
                            self.state.finish(request);
                            if !silent(&error) {
                                self.error = Some(operation_error(format!(
                                    "native code action resolve: {error:?}"
                                )));
                            }
                        }
                    }
                } else {
                    self.resolving = Some(resolving);
                }
            }
        }
        let mut finished = None;
        if let Some(mut applying) = self.applying.take() {
            match applying.receive.try_recv() {
                Ok(result) => {
                    if !applying.chosen.request.is_cancelled() {
                        match result {
                            Ok(Some(navigation)) => {
                                finished = Some(Finished {
                                    request: applying.chosen.request.clone(),
                                    provider: applying.chosen.candidate.provider,
                                    navigation,
                                })
                            }
                            Ok(None) => (),
                            Err(error) => self.error = Some(operation_error(error.to_string())),
                        }
                    }
                    self.state.finish(&applying.chosen.request);
                }
                Err(oneshot::error::TryRecvError::Empty) => self.applying = Some(applying),
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.state.finish(&applying.chosen.request);
                    self.error = Some(operation_error("native code action application stopped"));
                }
            }
        }
        (self.error.take(), finished)
    }

    fn session(&self, store: &EditorStore, view: ViewId) -> Option<Session> {
        let (request, actions, menu_open) = self.state.listed(store, view)?;
        if actions.is_empty() {
            return None;
        }
        let selection = request
            .selection
            .selections
            .get(request.selection.primary)?;
        Some(Session {
            token: request.token.as_u128(),
            anchor: selection.anchor.min(selection.head),
            menu_open,
            include_disabled: !request.invocation.automatic
                && request.invocation.command != Command::QuickFix,
            show_headers: true,
            actions,
            bulb: None,
            labels: Default::default(),
        })
    }

    pub(crate) fn local_key(
        &self,
        store: &EditorStore,
        source: ViewId,
        event: &eframe::egui::Event,
    ) -> bool {
        self.session(store, source)
            .is_some_and(|session| session.menu_open)
            && matches!(
                event,
                eframe::egui::Event::Key {
                    key: eframe::egui::Key::Enter
                        | eframe::egui::Key::Escape
                        | eframe::egui::Key::ArrowUp
                        | eframe::egui::Key::ArrowDown
                        | eframe::egui::Key::Space
                        | eframe::egui::Key::Period
                        | eframe::egui::Key::P
                        | eframe::egui::Key::N,
                    ..
                }
            )
    }
}

#[derive(Clone)]
pub(crate) struct Consumer<'a>(pub(crate) Rc<RefCell<&'a mut Controller>>);

impl Consumer<'_> {
    pub(crate) fn observe(
        &self,
        store: &EditorStore,
        project: Option<&ProjectId>,
        source: ViewId,
        owner: ViewId,
        lsp: Option<&LspBridge>,
        now: Instant,
    ) {
        self.0
            .borrow_mut()
            .observe(store, project, source, owner, lsp, now);
    }
}

pub(crate) struct Provider<'a, 'state> {
    pub(crate) consumer: Consumer<'state>,
    pub(crate) lsp: Option<&'a LspBridge>,
    pub(crate) context: eframe::egui::Context,
    pub(crate) font: eframe::egui::FontId,
    pub(crate) tab_size: u32,
}

impl taide_native_ui::editor_code_actions::Provider for Provider<'_, '_> {
    fn current(&self, store: &EditorStore, view: ViewId) -> Option<Session> {
        let controller = self.consumer.0.borrow();
        let mut session = controller.session(store, view)?;
        if session
            .actions
            .iter()
            .any(|action| action.disabled.is_none())
        {
            let space_width = self.context.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(" ".into(), self.font.clone(), eframe::egui::Color32::WHITE)
                    .size()
                    .x
            });
            session.bulb = bulb_position(
                store,
                view,
                controller.state.request()?,
                self.tab_size,
                space_width,
            );
        }
        Some(session)
    }
    fn open(&mut self, store: &EditorStore, view: ViewId, token: u128) {
        self.consumer.0.borrow_mut().state.open(store, view, token);
    }
    fn select(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: u128,
        index: usize,
        preview: bool,
    ) {
        let mut controller = self.consumer.0.borrow_mut();
        let Some(chosen) = controller.state.select(store, view, token, index, preview) else {
            return;
        };
        if let Some(lsp) = self.lsp {
            controller.choose(chosen, lsp);
        } else {
            controller.state.finish(&chosen.request);
        }
    }
    fn close(&mut self, view: ViewId, token: u128) {
        self.consumer.0.borrow_mut().state.close(view, token);
    }
}

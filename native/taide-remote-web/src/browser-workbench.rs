use std::rc::Rc;

use serde_json::Value;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::settings_controls::Change;
use taide_native_ui::snapshot::ShellSnapshot;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, MediaQueryList};

use crate::preferences::{Finished, PreferenceWrites, change_call};
use crate::presentation::{PresentationState, Read, SystemInputs};
use crate::settings_catalog::{CatalogReads, invocation_error};
use crate::theme_operations::ThemeOperations;
use crate::{BrowserError, BrowserEvent, BrowserShell, Delivery, InvokeError};

const DARK_MEDIA_QUERY: &str = "(prefers-color-scheme: dark)";
const MOTION_MEDIA_QUERY: &str = "(prefers-reduced-motion: reduce)";
const MEDIA_CHANGE_EVENT: &str = "change";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchError {
    Browser(BrowserError),
    SystemInputsUnavailable,
}

struct ThemeListener {
    query: MediaQueryList,
    callback: Closure<dyn FnMut(Event)>,
}

impl Drop for ThemeListener {
    fn drop(&mut self) {
        let _ = self.query.remove_event_listener_with_callback(
            MEDIA_CHANGE_EVENT,
            self.callback.as_ref().unchecked_ref(),
        );
    }
}

struct SystemAppearance {
    listener: Option<ThemeListener>,
    motion_listener: Option<ThemeListener>,
    language: String,
}

impl SystemAppearance {
    fn new(wake: Rc<dyn Fn()>) -> Result<Self, WorkbenchError> {
        let window =
            web_sys::window().ok_or(WorkbenchError::Browser(BrowserError::WindowUnavailable))?;
        let method = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("matchMedia"))
            .map_err(|_| WorkbenchError::SystemInputsUnavailable)?;
        let language = window.navigator().language().unwrap_or_default();
        if method.is_undefined() || method.is_null() {
            return Ok(Self {
                listener: None,
                motion_listener: None,
                language,
            });
        }
        let query = window
            .match_media(DARK_MEDIA_QUERY)
            .map_err(|_| WorkbenchError::SystemInputsUnavailable)?
            .ok_or(WorkbenchError::SystemInputsUnavailable)?;
        let theme_wake = Rc::clone(&wake);
        let callback = Closure::new(move |_: Event| theme_wake());
        query
            .add_event_listener_with_callback(MEDIA_CHANGE_EVENT, callback.as_ref().unchecked_ref())
            .map_err(|_| WorkbenchError::SystemInputsUnavailable)?;
        let listener = ThemeListener { query, callback };
        let query = window
            .match_media(MOTION_MEDIA_QUERY)
            .map_err(|_| WorkbenchError::SystemInputsUnavailable)?
            .ok_or(WorkbenchError::SystemInputsUnavailable)?;
        let callback = Closure::new(move |_: Event| wake());
        query
            .add_event_listener_with_callback(MEDIA_CHANGE_EVENT, callback.as_ref().unchecked_ref())
            .map_err(|_| WorkbenchError::SystemInputsUnavailable)?;
        Ok(Self {
            listener: Some(listener),
            motion_listener: Some(ThemeListener { query, callback }),
            language,
        })
    }

    fn theme(&self) -> &'static str {
        match &self.listener {
            None => "dark",
            Some(listener) if listener.query.matches() => "dark",
            Some(_) => "light",
        }
    }

    fn inputs(&self) -> SystemInputs {
        SystemInputs {
            theme: self.theme().into(),
            language: self.language.clone(),
        }
    }
}

pub struct BrowserWorkbench {
    shell: BrowserShell,
    appearance: Option<SystemAppearance>,
    presentation: PresentationState,
    preferences: PreferenceWrites,
    preference_results: Vec<Finished>,
    catalogs: CatalogReads,
    resources: crate::settings_resources::ResourceReads,
    themes: ThemeOperations,
    snippets: crate::snippet_operations::SnippetOperations,
    folders: crate::settings_folders::FolderActions,
    app_file_opens: crate::app_file_opens::AppFileOpens,
}

impl BrowserWorkbench {
    pub fn new(wake: Rc<dyn Fn()>) -> Result<Self, WorkbenchError> {
        let appearance = SystemAppearance::new(Rc::clone(&wake))?;
        let presentation = PresentationState::new(appearance.inputs());
        let shell = BrowserShell::new(wake).map_err(WorkbenchError::Browser)?;
        Ok(Self {
            shell,
            appearance: Some(appearance),
            presentation,
            preferences: PreferenceWrites::default(),
            preference_results: Vec::new(),
            catalogs: CatalogReads::default(),
            resources: crate::settings_resources::ResourceReads::default(),
            themes: ThemeOperations::default(),
            snippets: crate::snippet_operations::SnippetOperations::default(),
            folders: crate::settings_folders::FolderActions::default(),
            app_file_opens: crate::app_file_opens::AppFileOpens::default(),
        })
    }

    pub fn shell(&self) -> &BrowserShell {
        &self.shell
    }

    pub fn snapshot(&self) -> Option<&ShellSnapshot> {
        self.shell.snapshot()
    }

    pub fn presentation(&self) -> &PresentationState {
        &self.presentation
    }

    pub fn collation_locale(&self) -> String {
        self.appearance
            .as_ref()
            .map(|appearance| appearance.language.as_str())
            .filter(|language| !language.is_empty())
            .unwrap_or("en-US")
            .replace('_', "-")
    }

    pub fn reduced_motion(&self) -> bool {
        self.appearance.as_ref().is_some_and(|appearance| {
            appearance
                .motion_listener
                .as_ref()
                .is_some_and(|listener| listener.query.matches())
        })
    }

    pub fn is_connected(&self) -> bool {
        self.shell.is_connected()
    }

    pub fn invoke(&self, command: &str, args: Value) -> Result<u32, InvokeError> {
        self.shell.invoke(command, args)
    }

    pub fn mutate(&self, mutation: &ShellMutation) -> Result<u32, InvokeError> {
        self.shell.mutate(mutation)
    }

    pub fn change_preference(&mut self, change: Change) -> Result<u32, InvokeError> {
        if !self.shell.is_connected() {
            return Err(InvokeError::Closed);
        }
        let call = change_call(&change);
        let seq = self.shell.invoke(call.command, call.args)?;
        self.preferences
            .sent(seq, change, self.shell.state().settings_generation());
        Ok(seq)
    }

    pub fn take_preference_results(&mut self) -> Vec<Finished> {
        std::mem::take(&mut self.preference_results)
    }

    pub fn preference_results(&self) -> &[Finished] {
        &self.preference_results
    }

    pub(crate) fn take_preference_errors(&mut self) -> Vec<taide_model::error::AppError> {
        self.preferences.take_errors()
    }

    pub(crate) fn take_preference_close_failures(&mut self) -> Vec<crate::shell::Failure> {
        self.preferences.take_close_failures()
    }

    pub fn has_pending_preferences(&self) -> bool {
        self.preferences.is_pending()
    }

    pub(crate) fn open_settings_folder(&mut self, kind: taide_model::system::AppDataPathKind) {
        if !self.is_connected() {
            self.folders.failed(InvokeError::Closed);
            return;
        }
        let call = crate::settings_folders::FolderActions::call(kind);
        match self.invoke(call.command, call.args) {
            Ok(seq) => self.folders.sent(seq),
            Err(error) => self.folders.failed(error),
        }
    }

    pub(crate) fn take_folder_errors(&mut self) -> Vec<taide_model::error::AppError> {
        self.folders.take_errors()
    }

    pub(crate) fn open_settings_file(&mut self, owner: &taide_native_ui::settings_owner::Owner) {
        if !self.is_connected() {
            self.app_file_opens.failed(InvokeError::Closed);
            return;
        }
        let call = crate::app_file_opens::AppFileOpens::settings_call(owner);
        match self.invoke(call.command, call.args) {
            Ok(seq) => self.app_file_opens.sent(seq, owner.project.clone()),
            Err(error) => self.app_file_opens.failed(error),
        }
    }

    pub(crate) fn take_app_file_open_errors(&mut self) -> Vec<taide_model::error::AppError> {
        self.app_file_opens.take_errors()
    }

    pub(crate) fn has_pending_app_file_opens(&self) -> bool {
        self.app_file_opens.is_pending()
    }

    pub fn request_settings_catalog(&mut self, request: taide_native_ui::settings_view::Request) {
        self.catalogs.request(request);
        self.send_settings_catalog_reads();
    }

    pub fn take_settings_catalogs(&mut self) -> Vec<crate::settings_catalog::Finished> {
        self.catalogs.take_finished()
    }

    pub fn request_settings_resource(
        &mut self,
        request: taide_native_ui::settings_resources::Request,
    ) {
        let active = self.snapshot().is_some_and(|snapshot| {
            snapshot.project(&request.owner().project).is_some()
                && snapshot
                    .layouts
                    .get(&request.owner().project)
                    .is_some_and(|layout| request.owner().is_active_in(layout))
        });
        if !active {
            self.resources.failed(
                request,
                taide_model::error::AppError::Forbidden(
                    "remote Settings resource owner is no longer active".into(),
                ),
            );
            return;
        }
        if !self.is_connected() {
            self.resources
                .failed(request, invocation_error(InvokeError::Closed));
            return;
        }
        let call = crate::settings_resources::ResourceReads::call(&request);
        match self.invoke(call.command, call.args) {
            Ok(seq) => self.resources.sent(request, seq),
            Err(error) => self.resources.failed(request, invocation_error(error)),
        }
    }

    pub fn take_settings_resources(&mut self) -> Vec<taide_native_ui::settings_resources::Reply> {
        self.resources.take_finished()
    }

    pub fn submit_theme(&mut self, command: taide_native_ui::theme_edit::Command) {
        self.themes.submit(command);
        self.send_theme_calls();
    }

    pub fn submit_snippet(&mut self, request: taide_native_ui::snippet_edit::Request) {
        if self.snippets.is_pending(&request) {
            return;
        }
        let active = request.is_active()
            && self.snapshot().is_some_and(|snapshot| {
                snapshot.project(&request.owner().project).is_some()
                    && snapshot
                        .layouts
                        .get(&request.owner().project)
                        .is_some_and(|layout| request.owner().is_active_in(layout))
            });
        if !active {
            self.snippets.failed(
                request,
                taide_model::error::AppError::Forbidden(
                    "remote snippet editor owner is no longer active".into(),
                ),
            );
            return;
        }
        if !self.is_connected() {
            self.snippets
                .failed(request, invocation_error(InvokeError::Closed));
            return;
        }
        let call = crate::snippet_operations::SnippetOperations::call(&request);
        match self.invoke(call.command, call.args) {
            Ok(seq) => self.snippets.sent(request, seq),
            Err(error) => self.snippets.failed(request, invocation_error(error)),
        }
    }

    pub fn take_snippet_replies(&mut self) -> Vec<taide_native_ui::snippet_edit::Reply> {
        self.snippets.take_replies()
    }

    pub fn request_snippet_catalog(&mut self, request: taide_native_ui::snippet_catalog::Request) {
        if !request.is_active() {
            self.snippets.catalog_failed(
                request,
                taide_model::error::AppError::Forbidden(
                    "remote snippet catalog is no longer active".into(),
                ),
            );
            return;
        }
        if !self.is_connected() {
            self.snippets
                .catalog_failed(request, invocation_error(InvokeError::Closed));
            return;
        }
        match self.invoke("snippet_list", Value::Null) {
            Ok(seq) => self.snippets.catalog_sent(request, seq),
            Err(error) => self
                .snippets
                .catalog_failed(request, invocation_error(error)),
        }
    }

    pub fn take_snippet_catalogs(&mut self) -> Vec<taide_native_ui::snippet_catalog::Reply> {
        self.snippets.take_catalog_replies()
    }

    pub fn has_pending_snippet_mutations(&self) -> bool {
        self.snippets.has_pending_mutations()
    }

    pub fn snippet_failures(&self) -> &[crate::shell::Failure] {
        self.snippets.failures()
    }

    pub fn take_snippet_failures(&mut self) -> Vec<crate::shell::Failure> {
        self.snippets.take_failures()
    }

    pub fn take_theme_replies(&mut self) -> Vec<taide_native_ui::theme_edit::Reply> {
        self.themes.take_replies()
    }

    pub fn has_pending_theme_mutations(&self) -> bool {
        self.themes.has_pending_mutations()
    }

    pub fn theme_failures(&self) -> &[crate::shell::Failure] {
        self.themes.failures()
    }

    pub fn take_theme_failures(&mut self) -> Vec<crate::shell::Failure> {
        self.themes.take_failures()
    }

    fn send_theme_calls(&mut self) {
        for invocation in self.themes.next_calls() {
            let active = self.themes.is_active(&invocation)
                && self.snapshot().is_some_and(|snapshot| {
                    snapshot.project(&invocation.owner.project).is_some()
                        && snapshot
                            .layouts
                            .get(&invocation.owner.project)
                            .is_some_and(|layout| invocation.owner.is_active_in(layout))
                });
            if !active {
                self.themes.failed(
                    invocation,
                    taide_model::error::AppError::Forbidden(
                        "remote theme editor owner is no longer active".into(),
                    ),
                );
                continue;
            }
            if !self.is_connected() {
                self.themes
                    .failed(invocation, invocation_error(InvokeError::Closed));
                continue;
            }
            match self.invoke(invocation.call.command, invocation.call.args.clone()) {
                Ok(seq) => self.themes.sent(invocation, seq),
                Err(error) => self.themes.failed(invocation, invocation_error(error)),
            }
        }
    }

    fn send_settings_catalog_reads(&mut self) {
        for (request, read) in self.catalogs.next_reads() {
            let active = self.snapshot().is_some_and(|snapshot| {
                snapshot.project(&request.owner().project).is_some()
                    && snapshot
                        .layouts
                        .get(&request.owner().project)
                        .is_some_and(|layout| request.owner().is_active_in(layout))
            });
            if !active {
                self.catalogs.failed(
                    &request,
                    read,
                    taide_model::error::AppError::Forbidden(
                        "remote Settings owner is no longer active".into(),
                    ),
                );
                continue;
            }
            if !self.is_connected() {
                self.catalogs
                    .failed(&request, read, invocation_error(InvokeError::Closed));
                continue;
            }
            let call = read.call();
            match self.invoke(call.command, call.args) {
                Ok(seq) => self.catalogs.sent(request, read, seq),
                Err(error) => self
                    .catalogs
                    .failed(&request, read, invocation_error(error)),
            }
        }
    }

    pub fn refresh(&mut self) {
        self.shell.refresh();
        self.presentation.refresh();
        self.catalogs.invalidate();
    }

    pub fn retry_presentation(&mut self, read: Read) {
        self.presentation.retry(read);
    }

    pub fn poll(&mut self) -> Vec<BrowserEvent> {
        if let Some(appearance) = &self.appearance
            && self.presentation.inputs().theme != appearance.theme()
        {
            self.presentation.set_inputs(appearance.inputs());
        }
        let mut events = Vec::new();
        for event in self.shell.poll() {
            match &event {
                BrowserEvent::Connected { .. } => self.presentation.refresh(),
                BrowserEvent::Disconnected { .. } | BrowserEvent::AuthenticationRequired => {
                    self.presentation.disconnected();
                    self.preference_results
                        .extend(self.preferences.disconnected());
                    self.catalogs.disconnected();
                    self.resources.disconnected();
                    self.themes.disconnected();
                    self.snippets.disconnected();
                    self.folders.disconnected();
                    self.app_file_opens.disconnected();
                }
                BrowserEvent::Frame(Delivery::Response { seq, result }) => {
                    if let Some(opened) = self.app_file_opens.response(*seq, result) {
                        if let Some(opened) = opened {
                            self.shell.layout_updated(opened);
                        }
                        continue;
                    }
                    if self.folders.response(*seq, result) {
                        continue;
                    }
                    if self.themes.response(*seq, result) {
                        continue;
                    }
                    if self.snippets.response(*seq, result) {
                        continue;
                    }
                    if self.catalogs.response(*seq, result) {
                        continue;
                    }
                    if self.resources.response(*seq, result) {
                        continue;
                    }
                    if let Some(finished) = self.preferences.response(*seq, result) {
                        if let Ok(settings) = &finished.result {
                            self.shell
                                .settings_updated(settings.clone(), finished.settings_generation);
                        }
                        self.preference_results.push(finished);
                        continue;
                    }
                    if self.presentation.response(*seq, result) {
                        continue;
                    }
                }
                BrowserEvent::Frame(Delivery::Event { event, payload }) => {
                    self.presentation.event(event, payload);
                    if event == "theme:changed" || event == "locale:changed" {
                        self.catalogs.invalidate();
                    }
                }
                _ => {}
            }
            events.push(event);
        }
        if let Some(settings) = self.shell.state().settings() {
            self.presentation.settings(settings);
        }
        self.send_theme_calls();
        if self.shell.is_connected() && self.shell.state().settings().is_some() {
            for read in self.presentation.next_reads() {
                if !self.shell.is_connected() {
                    self.presentation.disconnected();
                    break;
                }
                let call = read.call(self.presentation.inputs());
                match self.shell.invoke(call.command, call.args) {
                    Ok(seq) => self.presentation.sent(read, seq),
                    Err(error) => self.presentation.invocation_failed(read, error),
                }
            }
        }
        events
    }

    pub fn dispose(&mut self) {
        self.appearance = None;
        self.presentation.disconnected();
        self.preference_results
            .extend(self.preferences.disconnected());
        self.catalogs.disconnected();
        self.resources.disconnected();
        self.themes.disconnected();
        self.snippets.disconnected();
        self.folders.disconnected();
        self.app_file_opens.disconnected();
        self.shell.dispose();
    }
}

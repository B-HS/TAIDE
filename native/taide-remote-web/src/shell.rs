use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Deserialize;
use serde_json::{Value, json};
use taide_model::ids::ProjectId;
use taide_model::layout::{ProjectLayout, ShellViewPatch};
use taide_model::project::{ProjectGroup, ProjectRef, SessionShellState};
use taide_model::settings::Settings;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::snapshot::ShellSnapshot;
use taide_remote_wire::client::{InvokeError, ResponsePayload};

pub struct Call {
    pub command: &'static str,
    pub args: Value,
}

pub fn mutation_call(mutation: &ShellMutation) -> Call {
    let (command, args) = match mutation {
        ShellMutation::ActivateProject(project) => {
            ("project_activate", json!({"projectId": project}))
        }
        ShellMutation::FocusSlot(slot) => ("session_focus_shell_slot", json!({"slotId": slot})),
        ShellMutation::CloseSlot(slot) => ("shell_slot_close", json!({"slotId": slot})),
        ShellMutation::SetGroupCollapsed { group, collapsed } => (
            "project_group_set_collapsed",
            json!({"groupId": group, "collapsed": collapsed}),
        ),
        ShellMutation::ResizeSlots { path, sizes } => (
            "session_set_shell_slot_sizes",
            json!({"path": path, "sizes": sizes}),
        ),
        ShellMutation::SetWindowChrome(patch) => {
            ("session_set_window_chrome", json!({"patch": patch}))
        }
        ShellMutation::SetSidebarCollapsed { project, collapsed } => (
            "layout_set_shell_view",
            json!({"projectId": project, "patch": ShellViewPatch { zen: None, sidebar_collapsed: Some(*collapsed) }}),
        ),
        ShellMutation::ActivateTab(tab) => ("layout_activate_tab", json!({"tabId": tab})),
        ShellMutation::ReopenClosed(project) => {
            ("layout_reopen_closed", json!({"projectId": project}))
        }
        ShellMutation::FocusPane(pane) => ("layout_focus_pane", json!({"paneId": pane})),
        ShellMutation::MoveTab { tab, pane, index } => (
            "layout_move_tab",
            json!({"tabId": tab, "paneId": pane, "index": index}),
        ),
        ShellMutation::PinTab { tab, pinned } => {
            ("layout_pin_tab", json!({"tabId": tab, "pinned": pinned}))
        }
        ShellMutation::KeepTab(tab) => (
            "layout_set_preview",
            json!({"tabId": tab, "preview": false}),
        ),
        ShellMutation::SplitTab { pane, edge, tab } => (
            "layout_split",
            json!({"paneId": pane, "edge": edge, "tabId": tab}),
        ),
        ShellMutation::ResizePane { pane, sizes } => {
            ("layout_resize", json!({"paneId": pane, "sizes": sizes}))
        }
    };
    Call { command, args }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Read {
    Projects,
    Groups,
    Session,
    Settings,
    Layout(ProjectId),
}

impl Read {
    pub fn call(&self) -> Call {
        let (command, args) = match self {
            Self::Projects => ("project_list", Value::Null),
            Self::Groups => ("project_group_list", Value::Null),
            Self::Session => ("session_get_shell_state", Value::Null),
            Self::Settings => ("settings_get", Value::Null),
            Self::Layout(project) => ("layout_get", json!({"projectId": project})),
        };
        Call { command, args }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    Invocation(InvokeError),
    Remote(Value),
    MalformedResponse,
}

#[derive(Default)]
pub struct ShellState {
    projects: Option<Vec<ProjectRef>>,
    groups: Option<Vec<ProjectGroup>>,
    session: Option<SessionShellState>,
    settings: Option<Settings>,
    settings_generation: u64,
    layouts: HashMap<ProjectId, ProjectLayout>,
    pending: BTreeMap<u32, Read>,
    dirty: BTreeSet<Read>,
    failures: BTreeMap<Read, Failure>,
    snapshot: Option<ShellSnapshot>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LayoutEvent {
    project_id: ProjectId,
    revision: u32,
}

#[derive(Deserialize)]
struct SettingsEvent {
    settings: Settings,
}

impl ShellState {
    pub fn snapshot(&self) -> Option<&ShellSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    pub fn settings_generation(&self) -> u64 {
        self.settings_generation
    }

    pub fn settings_updated(&mut self, settings: Settings, expected_generation: u64) -> bool {
        if self.settings_generation != expected_generation {
            return false;
        }
        self.settings = Some(settings);
        self.pending.retain(|_, read| read != &Read::Settings);
        self.dirty.remove(&Read::Settings);
        self.failures.remove(&Read::Settings);
        self.rebuild_snapshot();
        true
    }

    pub fn failures(&self) -> &BTreeMap<Read, Failure> {
        &self.failures
    }

    pub fn layout_updated(&mut self, project: ProjectId, layout: ProjectLayout) -> bool {
        if !self.contains_project(&project)
            || self
                .layouts
                .get(&project)
                .is_some_and(|current| current.revision > layout.revision)
        {
            return false;
        }
        self.layouts.insert(project, layout);
        self.rebuild_snapshot();
        true
    }

    pub fn refresh(&mut self) {
        self.dirty
            .extend([Read::Projects, Read::Groups, Read::Session, Read::Settings]);
        if let Some(projects) = &self.projects {
            self.dirty.extend(
                projects
                    .iter()
                    .map(|project| Read::Layout(project.id.clone())),
            );
        }
    }

    pub fn next_reads(&self) -> Vec<Read> {
        self.dirty
            .iter()
            .filter(|read| !self.pending.values().any(|pending| pending == *read))
            .cloned()
            .collect()
    }

    pub fn sent(&mut self, read: Read, seq: u32) {
        self.dirty.remove(&read);
        self.failures.remove(&read);
        self.pending.insert(seq, read);
    }

    pub fn invocation_failed(&mut self, read: Read, error: InvokeError) {
        self.dirty.remove(&read);
        self.failures.insert(read, Failure::Invocation(error));
    }

    pub fn disconnected(&mut self) {
        self.pending.clear();
        self.dirty.clear();
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        let Some(read) = self.pending.remove(&seq) else {
            return false;
        };
        let applied = match result {
            Ok(ResponsePayload::Json(value)) => self.apply(&read, value.clone()),
            Ok(ResponsePayload::Binary(_)) => Err(Failure::MalformedResponse),
            Err(error) => Err(Failure::Remote(error.clone())),
        };
        match applied {
            Ok(()) => {
                self.failures.remove(&read);
                self.rebuild_snapshot();
            }
            Err(error) => {
                self.failures.insert(read, error);
            }
        }
        true
    }

    pub fn event(&mut self, name: &str, payload: &str) {
        match name {
            "project:list-changed" | "project:opened" | "project:closed" | "project:activated" => {
                self.dirty.insert(Read::Projects);
            }
            "project:groups-changed" => {
                self.dirty.insert(Read::Groups);
            }
            "session:shell-slots-changed" | "session:window-chrome-changed" => {
                self.dirty.insert(Read::Session);
            }
            "settings:changed" => {
                if let Ok(event) = serde_json::from_str::<SettingsEvent>(payload) {
                    self.settings_generation = self.settings_generation.saturating_add(1);
                    self.settings_updated(event.settings, self.settings_generation);
                }
            }
            "layout:changed" => {
                if let Ok(event) = serde_json::from_str::<LayoutEvent>(payload)
                    && self.contains_project(&event.project_id)
                    && self
                        .layouts
                        .get(&event.project_id)
                        .is_none_or(|layout| layout.revision < event.revision)
                {
                    self.dirty.insert(Read::Layout(event.project_id));
                }
            }
            _ => {}
        }
    }

    fn contains_project(&self, id: &ProjectId) -> bool {
        self.projects
            .as_ref()
            .is_some_and(|projects| projects.iter().any(|project| &project.id == id))
    }

    fn apply(&mut self, read: &Read, value: Value) -> Result<(), Failure> {
        match read {
            Read::Projects => {
                let projects: Vec<ProjectRef> =
                    serde_json::from_value(value).map_err(|_| Failure::MalformedResponse)?;
                let retained: BTreeSet<_> =
                    projects.iter().map(|project| project.id.clone()).collect();
                self.layouts.retain(|id, _| retained.contains(id));
                self.pending
                    .retain(|_, read| !matches!(read, Read::Layout(id) if !retained.contains(id)));
                self.dirty
                    .retain(|read| !matches!(read, Read::Layout(id) if !retained.contains(id)));
                self.failures
                    .retain(|read, _| !matches!(read, Read::Layout(id) if !retained.contains(id)));
                self.dirty.extend(
                    projects
                        .iter()
                        .filter(|project| !self.layouts.contains_key(&project.id))
                        .map(|project| Read::Layout(project.id.clone())),
                );
                self.projects = Some(projects);
            }
            Read::Groups => {
                self.groups =
                    Some(serde_json::from_value(value).map_err(|_| Failure::MalformedResponse)?)
            }
            Read::Session => {
                self.session =
                    Some(serde_json::from_value(value).map_err(|_| Failure::MalformedResponse)?)
            }
            Read::Settings => {
                self.settings =
                    Some(serde_json::from_value(value).map_err(|_| Failure::MalformedResponse)?)
            }
            Read::Layout(project) => {
                if !self.contains_project(project) {
                    return Ok(());
                }
                let layout: ProjectLayout =
                    serde_json::from_value(value).map_err(|_| Failure::MalformedResponse)?;
                self.layout_updated(project.clone(), layout);
            }
        }
        Ok(())
    }

    fn rebuild_snapshot(&mut self) {
        let (Some(projects), Some(groups), Some(shell), Some(settings)) =
            (&self.projects, &self.groups, &self.session, &self.settings)
        else {
            return;
        };
        if projects
            .iter()
            .any(|project| !self.layouts.contains_key(&project.id))
        {
            self.snapshot = None;
            return;
        }
        self.snapshot = Some(ShellSnapshot {
            projects: projects.clone(),
            groups: groups.clone(),
            shell: shell.clone(),
            layouts: self.layouts.clone(),
            hide_status_in_zen: settings.zen_hide_status_bar,
            resizer_thickness: settings.resizer_thickness as f32,
        });
    }
}

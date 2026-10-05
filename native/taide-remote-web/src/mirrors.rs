use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::{Value, json};
use taide_model::file::MirrorEntry;
use taide_model::ids::ProjectId;
use taide_model::layout::ProjectLayout;

use crate::shell::{Call, Failure};
use crate::{InvokeError, ResponsePayload};

const DRIVE_PREFIX_BYTES: usize = 2;

#[derive(Default)]
struct Entry {
    mirrors: Option<Vec<MirrorEntry>>,
    failure: Option<Failure>,
}

struct Pending {
    project: ProjectId,
    valid: bool,
}

#[derive(Default)]
pub struct MirrorState {
    entries: BTreeMap<ProjectId, Entry>,
    dirty: BTreeSet<ProjectId>,
    pending: BTreeMap<u32, Pending>,
    auxiliary_slots: BTreeMap<ProjectId, BTreeSet<u32>>,
}

impl MirrorState {
    pub fn request(&mut self, project: ProjectId) {
        if !self.entries.contains_key(&project) {
            self.entries.insert(project.clone(), Entry::default());
            self.dirty.insert(project);
        }
    }

    pub fn mirrors(&self, project: &ProjectId) -> Option<&[MirrorEntry]> {
        let entry = self.entries.get(project)?;
        if self.dirty.contains(project) || entry.failure.is_some() {
            return None;
        }
        entry.mirrors.as_deref()
    }

    pub fn failure(&self, project: &ProjectId) -> Option<&Failure> {
        self.entries.get(project)?.failure.as_ref()
    }

    pub fn record_write(&mut self, project: &ProjectId, mirror: MirrorEntry) {
        self.invalidate_pending(project);
        if let Some(mirrors) = self
            .entries
            .get_mut(project)
            .and_then(|entry| entry.mirrors.as_mut())
        {
            mirrors.retain(|entry| entry.path != mirror.path);
            mirrors.push(mirror);
        }
    }

    pub fn settle_path(&mut self, project: &ProjectId, path: &str) {
        self.invalidate_pending(project);
        if let Some(mirrors) = self
            .entries
            .get_mut(project)
            .and_then(|entry| entry.mirrors.as_mut())
        {
            mirrors.retain(|entry| entry.path != path);
        }
    }

    fn invalidate_pending(&mut self, project: &ProjectId) {
        if self
            .pending
            .values()
            .any(|pending| &pending.project == project)
        {
            self.refresh(project);
        }
    }

    pub fn refresh(&mut self, project: &ProjectId) {
        if let Some(entry) = self.entries.get_mut(project) {
            entry.failure = None;
            self.dirty.insert(project.clone());
            for pending in self
                .pending
                .values_mut()
                .filter(|pending| &pending.project == project)
            {
                pending.valid = false;
            }
        }
    }

    pub fn refresh_all(&mut self) {
        for project in self.entries.keys().cloned().collect::<Vec<_>>() {
            self.refresh(&project);
        }
    }

    pub fn event(&mut self, name: &str) {
        if name == "fs:rescan-required" {
            self.refresh_all();
        }
    }

    pub fn reconcile_layouts(&mut self, layouts: &HashMap<ProjectId, ProjectLayout>) {
        for project in self.entries.keys().cloned().collect::<Vec<_>>() {
            let Some(layout) = layouts.get(&project) else {
                continue;
            };
            let slots = layout
                .auxiliary_windows
                .iter()
                .map(|window| window.slot)
                .collect::<BTreeSet<_>>();
            if self
                .auxiliary_slots
                .get(&project)
                .is_some_and(|previous| previous.iter().any(|slot| !slots.contains(slot)))
            {
                self.refresh(&project);
            }
            self.auxiliary_slots.insert(project, slots);
        }
    }

    pub fn retain(&mut self, mut active: impl FnMut(&ProjectId) -> bool) {
        self.entries.retain(|project, _| active(project));
        self.auxiliary_slots
            .retain(|project, _| self.entries.contains_key(project));
        self.dirty
            .retain(|project| self.entries.contains_key(project));
        for pending in self.pending.values_mut() {
            if !self.entries.contains_key(&pending.project) {
                pending.valid = false;
            }
        }
    }

    pub fn next_reads(&self) -> Vec<ProjectId> {
        self.dirty
            .iter()
            .filter(|project| {
                !self
                    .pending
                    .values()
                    .any(|pending| &pending.project == *project)
            })
            .cloned()
            .collect()
    }

    pub fn read_call(project: &ProjectId) -> Call {
        Call {
            command: "file_list_mirrors",
            args: json!({"projectId": project}),
        }
    }

    pub fn sent(&mut self, project: ProjectId, seq: u32) {
        self.dirty.remove(&project);
        if let Some(entry) = self.entries.get_mut(&project) {
            entry.failure = None;
            entry.mirrors = None;
        }
        self.pending.insert(
            seq,
            Pending {
                project,
                valid: true,
            },
        );
    }

    pub fn invocation_failed(&mut self, project: ProjectId, error: InvokeError) {
        self.dirty.remove(&project);
        if let Some(entry) = self.entries.get_mut(&project) {
            entry.failure = Some(Failure::Invocation(error));
        }
    }

    pub fn response(&mut self, seq: u32, result: &Result<ResponsePayload, Value>) -> bool {
        let Some(pending) = self.pending.remove(&seq) else {
            return false;
        };
        if !pending.valid || self.dirty.contains(&pending.project) {
            return true;
        }
        let Some(entry) = self.entries.get_mut(&pending.project) else {
            return true;
        };
        let parsed = match result {
            Ok(ResponsePayload::Json(value)) => {
                serde_json::from_value::<Vec<MirrorEntry>>(value.clone())
                    .map_err(|_| Failure::MalformedResponse)
            }
            Ok(ResponsePayload::Binary(_)) => Err(Failure::MalformedResponse),
            Err(error) => Err(Failure::Remote(error.clone())),
        };
        match parsed {
            Ok(mirrors) => {
                entry.mirrors = Some(mirrors);
                entry.failure = None;
            }
            Err(error) => {
                entry.failure = Some(error);
            }
        }
        true
    }

    pub fn disconnected(&mut self) {
        self.pending.clear();
        self.dirty.clear();
        for entry in self.entries.values_mut() {
            entry.failure = Some(Failure::Invocation(InvokeError::Closed));
        }
    }
}

pub fn is_within_root(path: &str, root: &str) -> bool {
    let path = normalized_path(path);
    let root = normalized_path(root);
    path == root || path.starts_with(&format!("{root}/"))
}

fn normalized_path(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let bytes = unified.as_bytes();
    let drive = bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1) == Some(&b':')
        && (bytes.len() == DRIVE_PREFIX_BYTES || bytes.get(DRIVE_PREFIX_BYTES) == Some(&b'/'));
    let (anchor, rest) = if drive {
        unified.split_at(DRIVE_PREFIX_BYTES)
    } else if unified.starts_with('/') {
        unified.split_at(1)
    } else {
        ("", unified.as_str())
    };
    let mut segments = Vec::new();
    for segment in rest.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
    match anchor {
        "" => segments.join("/"),
        "/" => format!("/{}", segments.join("/")),
        _ => format!("{anchor}/{}", segments.join("/")),
    }
}

use std::collections::{HashMap, HashSet};

use eframe::egui::ViewportId;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ShellSlotId;
use taide_model::project::ShellSlotTree;

use crate::explorer_clipboard::Entry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owner {
    window: ViewportId,
    generation: u64,
}

struct Mount {
    owner: Owner,
    clipboard: Option<Entry>,
    reveal: Option<String>,
}

pub struct ClipboardOwners {
    window: ViewportId,
    serial: u64,
    mounts: HashMap<ShellSlotId, Mount>,
}

impl ClipboardOwners {
    pub fn new(window: ViewportId) -> Self {
        Self {
            window,
            serial: 0,
            mounts: HashMap::new(),
        }
    }

    pub fn reconcile(&mut self, tree: Option<&ShellSlotTree>) {
        let mut slots = HashSet::new();
        if let Some(tree) = tree {
            collect_slots(tree, &mut slots);
        }
        self.mounts.retain(|slot, _| slots.contains(slot));
    }

    pub fn mount(&mut self, slot: &ShellSlotId) -> AppResult<Owner> {
        if let Some(mount) = self.mounts.get(slot) {
            return Ok(mount.owner);
        }
        self.serial = self.serial.checked_add(1).ok_or_else(|| {
            AppError::Internal("native explorer clipboard owner exhausted".into())
        })?;
        let owner = Owner {
            window: self.window,
            generation: self.serial,
        };
        self.mounts.insert(
            slot.clone(),
            Mount {
                owner,
                clipboard: None,
                reveal: None,
            },
        );
        Ok(owner)
    }

    pub fn clipboard(&self, owner: Owner) -> Option<&Entry> {
        self.mounts
            .values()
            .find(|mount| mount.owner == owner)
            .and_then(|mount| mount.clipboard.as_ref())
    }

    pub fn replace(&mut self, owner: Owner, clipboard: Option<Entry>) -> bool {
        let Some(mount) = self.mounts.values_mut().find(|mount| mount.owner == owner) else {
            return false;
        };
        mount.clipboard = clipboard;
        true
    }

    pub fn paste_finished(&mut self, owner: Option<Owner>, clear_cut: bool, path: Option<String>) {
        let Some(owner) = owner else {
            return;
        };
        let Some(mount) = self.mounts.values_mut().find(|mount| mount.owner == owner) else {
            return;
        };
        if clear_cut {
            mount.clipboard = None;
        }
        if path.is_some() {
            mount.reveal = path;
        }
    }

    pub fn take_reveal(&mut self, owner: Owner) -> Option<String> {
        self.mounts
            .values_mut()
            .find(|mount| mount.owner == owner)
            .and_then(|mount| mount.reveal.take())
    }
}

fn collect_slots(tree: &ShellSlotTree, slots: &mut HashSet<ShellSlotId>) {
    match tree {
        ShellSlotTree::Leaf { slot_id, .. } => {
            slots.insert(slot_id.clone());
        }
        ShellSlotTree::Split { children, .. } => {
            for child in children {
                collect_slots(child, slots);
            }
        }
    }
}

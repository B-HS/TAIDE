use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use eframe::egui::ViewportId;
use taide_model::error::AppResult;

use crate::toast::{Action, Toasts};

/// One toaster per window, like `AppToaster` mounted in every window (`app-providers.tsx` 15-26).
/// Dereferencing reaches the layer of the window that had OS focus when the frame began, so a
/// plain `toasts.error(..)` for a user action lands in the window the user is acting in.
/// Providers only the main window mounts (external open, notifications, drops) use
/// [`ToastLayers::main`].
pub(crate) struct ToastLayers {
    main: Toasts,
    auxiliary: HashMap<ViewportId, Toasts>,
    focused: ViewportId,
}

impl ToastLayers {
    pub(crate) fn new() -> AppResult<Self> {
        Ok(Self {
            main: Toasts::with_actions()?,
            auxiliary: HashMap::new(),
            focused: ViewportId::ROOT,
        })
    }

    pub(crate) fn main(&mut self) -> &mut Toasts {
        &mut self.main
    }

    /// The toaster an auxiliary window draws, if that window has one.
    pub(crate) fn auxiliary(&mut self, viewport: ViewportId) -> Option<&mut Toasts> {
        self.auxiliary.get_mut(&viewport)
    }

    /// Mounts a toaster for an auxiliary window before its first pass.
    pub(crate) fn mount(&mut self, viewport: ViewportId) -> AppResult<()> {
        if viewport != ViewportId::ROOT && !self.auxiliary.contains_key(&viewport) {
            self.auxiliary.insert(viewport, Toasts::with_actions()?);
        }
        Ok(())
    }

    /// Drops the toasts of windows that closed, as the original toaster unmounts with its window.
    pub(crate) fn retain(&mut self, is_open: impl Fn(ViewportId) -> bool) {
        self.auxiliary.retain(|viewport, _| is_open(*viewport));
        if !self.auxiliary.contains_key(&self.focused) {
            self.focused = ViewportId::ROOT;
        }
    }

    /// Records the window with OS focus as the layer dereferencing reaches.
    pub(crate) fn focus(&mut self, viewport: ViewportId) {
        self.focused = if self.auxiliary.contains_key(&viewport) {
            viewport
        } else {
            ViewportId::ROOT
        };
    }

    pub(crate) fn set_reduced_motion(&mut self, is_reduced_motion: bool) {
        self.main.set_reduced_motion(is_reduced_motion);
        for layer in self.auxiliary.values_mut() {
            layer.set_reduced_motion(is_reduced_motion);
        }
    }

    pub(crate) fn take_actions(&mut self) -> Vec<Action> {
        let mut actions = self.main.take_actions();
        for layer in self.auxiliary.values_mut() {
            actions.extend(layer.take_actions());
        }
        actions
    }
}

impl Deref for ToastLayers {
    type Target = Toasts;

    fn deref(&self) -> &Toasts {
        self.auxiliary.get(&self.focused).unwrap_or(&self.main)
    }
}

impl DerefMut for ToastLayers {
    fn deref_mut(&mut self) -> &mut Toasts {
        match self.auxiliary.get_mut(&self.focused) {
            Some(layer) => layer,
            None => &mut self.main,
        }
    }
}

#[cfg(test)]
#[path = "toast-layers-tests.rs"]
mod tests;

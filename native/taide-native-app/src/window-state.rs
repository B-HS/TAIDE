use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use serde::{Deserialize, Serialize};
use taide_infra::persist;
use taide_model::error::AppResult;

pub const FILE_NAME: &str = "native-window-state.json";
pub const MAIN_LABEL: &str = "main";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl WindowState {
    pub fn apply(self, builder: egui::ViewportBuilder) -> egui::ViewportBuilder {
        builder
            .with_inner_size([self.width, self.height])
            .with_maximized(self.maximized)
            .with_fullscreen(self.fullscreen)
    }
}

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load(data_dir: &Path) -> Option<WindowState> {
    match persist::read_json::<BTreeMap<String, WindowState>>(&path(data_dir)) {
        Ok(states) => states?.remove(MAIN_LABEL),
        Err(error) => {
            log::warn!("native window state load failed: {:?}", error.kind());
            None
        }
    }
}

pub fn main_viewport(builder: egui::ViewportBuilder, data_dir: &Path) -> egui::ViewportBuilder {
    match load(data_dir) {
        Some(state) => state.apply(builder),
        None => builder,
    }
}

pub(crate) struct Tracker {
    path: PathBuf,
    current: Option<WindowState>,
}

impl Tracker {
    pub(crate) fn new(data_dir: &Path) -> Self {
        Self {
            path: path(data_dir),
            current: load(data_dir),
        }
    }

    pub(crate) fn observe(&mut self, info: &egui::ViewportInfo, zoom_factor: f32) {
        if info.minimized == Some(true) {
            return;
        }
        let Some(inner) = info.inner_rect else {
            return;
        };
        let size = inner.size() * zoom_factor;
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let maximized = info.maximized == Some(true);
        let fullscreen = info.fullscreen == Some(true);
        let state = self.current.get_or_insert(WindowState {
            width: size.x,
            height: size.y,
            maximized,
            fullscreen,
        });
        state.maximized = maximized;
        state.fullscreen = fullscreen;
        if !maximized && !fullscreen {
            state.width = size.x;
            state.height = size.y;
        }
    }

    pub(crate) fn save(&self) -> AppResult<()> {
        let Some(state) = self.current else {
            return Ok(());
        };
        persist::write_json(
            &self.path,
            &BTreeMap::from([(MAIN_LABEL.to_owned(), state)]),
        )
    }
}

#[cfg(test)]
#[path = "window-state-tests.rs"]
mod tests;

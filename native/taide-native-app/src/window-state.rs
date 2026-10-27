use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use eframe::egui;
use serde::{Deserialize, Serialize};
use taide_infra::persist;
use taide_model::error::AppResult;
use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;

pub const FILE_NAME: &str = "native-window-state.json";
pub const MAIN_LABEL: &str = "main";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub position: Option<[f32; 2]>,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl WindowState {
    pub fn apply(
        self,
        builder: egui::ViewportBuilder,
        monitors: &[Monitor],
    ) -> egui::ViewportBuilder {
        let builder = builder
            .with_inner_size([self.width, self.height])
            .with_maximized(self.maximized)
            .with_fullscreen(self.fullscreen);
        match self.position {
            Some(position)
                if monitors
                    .iter()
                    .any(|monitor| self.intersects(*monitor, position)) =>
            {
                builder.with_position(position)
            }
            _ => builder,
        }
    }

    fn intersects(self, monitor: Monitor, [x, y]: [f32; 2]) -> bool {
        let right = x + self.width;
        let bottom = y + self.height;
        [(x, y), (right, y), (x, bottom), (right, bottom)]
            .into_iter()
            .any(|corner| monitor.contains(corner))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Monitor {
    pub position: [f32; 2],
    pub size: [f32; 2],
}

impl Monitor {
    fn contains(self, (x, y): (f32, f32)) -> bool {
        let [left, top] = self.position;
        let [width, height] = self.size;
        x >= left && x < left + width && y >= top && y < top + height
    }

    fn from_physical(
        position: winit::dpi::PhysicalPosition<i32>,
        size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        let position = position.to_logical::<f32>(scale_factor);
        let size = size.to_logical::<f32>(scale_factor);
        Self {
            position: [position.x, position.y],
            size: [size.width, size.height],
        }
    }
}

impl From<winit::monitor::MonitorHandle> for Monitor {
    fn from(handle: winit::monitor::MonitorHandle) -> Self {
        Self::from_physical(handle.position(), handle.size(), handle.scale_factor())
    }
}

pub type Monitors = Rc<OnceCell<Vec<Monitor>>>;

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

pub fn main_viewport(
    builder: egui::ViewportBuilder,
    data_dir: &Path,
    monitors: &[Monitor],
) -> egui::ViewportBuilder {
    match load(data_dir) {
        Some(state) => state.apply(builder, monitors),
        None => builder,
    }
}

pub fn main_window_hook(data_dir: PathBuf, monitors: Monitors) -> eframe::WindowBuilderHook {
    Box::new(move |builder| {
        main_viewport(
            builder,
            &data_dir,
            monitors.get().map_or(&[], Vec::as_slice),
        )
    })
}

pub struct MonitorProbe<'a> {
    application: eframe::EframeWinitApplication<'a>,
    monitors: Monitors,
}

impl<'a> MonitorProbe<'a> {
    pub fn new(application: eframe::EframeWinitApplication<'a>, monitors: Monitors) -> Self {
        Self {
            application,
            monitors,
        }
    }
}

impl ApplicationHandler<eframe::UserEvent> for MonitorProbe<'_> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        self.application.new_events(event_loop, cause);
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.monitors
            .get_or_init(|| event_loop.available_monitors().map(Monitor::from).collect());
        self.application.resumed(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: eframe::UserEvent) {
        self.application.user_event(event_loop, event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        self.application.window_event(event_loop, window_id, event);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        self.application.device_event(event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.application.about_to_wait(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.application.suspended(event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.application.exiting(event_loop);
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.application.memory_warning(event_loop);
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
        let position = info
            .outer_rect
            .map(|outer| outer.min.to_vec2() * zoom_factor)
            .map(|position| [position.x, position.y]);
        let maximized = info.maximized == Some(true);
        let fullscreen = info.fullscreen == Some(true);
        let state = self.current.get_or_insert(WindowState {
            width: size.x,
            height: size.y,
            position,
            maximized,
            fullscreen,
        });
        state.maximized = maximized;
        state.fullscreen = fullscreen;
        if !maximized && !fullscreen {
            state.width = size.x;
            state.height = size.y;
            if position.is_some() {
                state.position = position;
            }
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

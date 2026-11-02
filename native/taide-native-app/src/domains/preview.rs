use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{self, Ui};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_native_ui::commands::{ShellIntent, ShellMutation};
use taide_native_ui::intents::preview::Intent;
use taide_native_ui::surfaces::preview::PreviewSurfaces;
use taide_runtime::AppServices;

use crate::application::{AppSurfaces, MAX_DOCUMENTS, NativeApplication};
use crate::host::{HostCommand, HostReply, HostWorker};
use crate::presentation;

const WEB_CONNECTIONS: usize = 16;
const WEB_HEADER_TIMEOUT: Duration = Duration::from_secs(5);
const WEB_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const WEB_HELPER_TIMEOUT: Duration = Duration::from_secs(30);

pub enum Command {
    ReadHwpPreview(crate::preview_hwp::Request),
    ReadPresentationPreview(crate::preview_presentation::Request),
    ReadSpreadsheetPreview(crate::preview_spreadsheet::Request),
    ReadPreview(crate::preview::Request),
    ReadPdfPreview(crate::preview_pdf::Request),
}

pub enum Reply {
    HwpSourceReady(crate::preview_hwp::Request),
    HwpPreview {
        request: crate::preview_hwp::Request,
        result: Result<crate::preview_hwp::Page, crate::preview::Failure>,
    },
    SpreadsheetSourceReady(crate::preview_spreadsheet::Request),
    SpreadsheetPreview {
        request: crate::preview_spreadsheet::Request,
        result: Result<crate::preview_spreadsheet::Workbook, crate::preview::Failure>,
    },
    PresentationSourceReady(crate::preview_presentation::Request),
    PresentationPreview {
        request: crate::preview_presentation::Request,
        result: Result<crate::preview_presentation::Outline, crate::preview::Failure>,
    },
    PdfSourceReady(crate::preview_pdf::Request),
    PdfPreview {
        request: crate::preview_pdf::Request,
        result: Result<crate::preview_pdf::Page, crate::preview_pdf::Failure>,
    },
    Preview {
        request: crate::preview::Request,
        result: AppResult<crate::preview::Raster>,
    },
}

pub(crate) struct State {
    pub(crate) web_bridge: Option<crate::preview_web_host::Bridge>,
    pub(crate) helper_executable: PathBuf,
    pub(crate) web_previews: crate::preview_web_cache::Cache,
    pub(crate) web_views: crate::preview_web_view::Views,
    pub(crate) open_with: crate::open_with::Registry,
    pub(crate) previews: crate::preview::Cache,
    pub(crate) hwp_previews: crate::preview_hwp::Cache,
    pub(crate) pdf_previews: crate::preview_pdf::Cache,
    pub(crate) pdf_appearance: crate::preview_pdf_surface::Appearance,
    pub(crate) presentation_previews: crate::preview_presentation_cache::Cache,
    pub(crate) presentation_appearance: crate::preview_presentation_surface::Appearance,
    pub(crate) spreadsheet_previews: crate::preview_spreadsheet_cache::Cache,
    pub(crate) spreadsheet_appearance: crate::preview_spreadsheet_surface::Appearance,
}

#[derive(Default)]
pub(crate) struct FrameOutput {
    load_previews: Vec<(String, usize)>,
    load_pdf_previews: Vec<(TabId, String, usize)>,
    load_hwp_previews: Vec<(TabId, String, usize)>,
    load_presentation_previews: Vec<(TabId, String)>,
    load_spreadsheet_previews: Vec<(TabId, String)>,
    load_web_previews: Vec<String>,
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let sender = worker.sender;
    let repaint = worker.repaint;
    match command {
        Command::ReadHwpPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_hwp::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::Preview(Reply::HwpSourceReady(progress)))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::Preview(Reply::HwpPreview { request, result }))
        }
        Command::ReadSpreadsheetPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_spreadsheet::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::Preview(Reply::SpreadsheetSourceReady(progress)))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::Preview(Reply::SpreadsheetPreview {
                request,
                result,
            }))
        }
        Command::ReadPresentationPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_presentation::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::Preview(Reply::PresentationSourceReady(progress)))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::Preview(Reply::PresentationPreview {
                request,
                result,
            }))
        }
        Command::ReadPreview(request) => {
            let result = crate::preview::read(services, &request).await;
            Some(HostReply::Preview(Reply::Preview { request, result }))
        }
        Command::ReadPdfPreview(request) => {
            let progress = request.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let result = crate::preview_pdf::read(services, &request, move || {
                if sender
                    .blocking_send(HostReply::Preview(Reply::PdfSourceReady(progress)))
                    .is_ok()
                {
                    repaint();
                }
            })
            .await;
            Some(HostReply::Preview(Reply::PdfPreview { request, result }))
        }
    }
}

pub(crate) fn connect_web(
    services: Arc<AppServices>,
    context: &egui::Context,
    executable: PathBuf,
    appearance: &crate::preview_pdf_surface::Appearance,
) -> AppResult<crate::preview_web_host::Bridge> {
    let repaint = context.clone();
    let bridge = crate::preview_web_host::Bridge::connect_lazy(
        services,
        Arc::new(move || repaint.request_repaint()),
        executable,
        WEB_HELPER_TIMEOUT,
        crate::preview_web_http::Limits {
            connections: WEB_CONNECTIONS,
            sources: MAX_DOCUMENTS,
            header_timeout: WEB_HEADER_TIMEOUT,
            idle_timeout: WEB_IDLE_TIMEOUT,
        },
    )?;
    bridge.set_media_appearance(media_appearance(appearance));
    Ok(bridge)
}

fn media_appearance(
    appearance: &crate::preview_pdf_surface::Appearance,
) -> crate::preview_web_media::Appearance {
    crate::preview_web_media::Appearance {
        background: appearance.background.to_srgba_unmultiplied(),
        foreground: appearance.foreground.to_srgba_unmultiplied(),
        muted: appearance.muted.to_srgba_unmultiplied(),
    }
}

impl NativeApplication {
    pub(crate) fn poll_preview_web(&mut self) {
        while let Some(reply) = self
            .preview_state
            .web_bridge
            .as_mut()
            .and_then(crate::preview_web_host::Bridge::poll)
        {
            self.reconcile_previews();
            match reply {
                crate::preview_web_host::Reply::SourceReady(request) => {
                    self.preview_state.web_previews.source_ready(&request)
                }
                crate::preview_web_host::Reply::Prepared { request, result } => {
                    if self.closing.is_some() || self.services.state.is_shutting_down() {
                        self.preview_state.web_previews.cancelled(&request);
                    } else {
                        let other_bytes = self
                            .preview_state
                            .previews
                            .retained_bytes()
                            .saturating_add(self.preview_state.hwp_previews.retained_bytes())
                            .saturating_add(self.preview_state.pdf_previews.retained_bytes())
                            .saturating_add(
                                self.preview_state.presentation_previews.retained_bytes(),
                            )
                            .saturating_add(
                                self.preview_state.spreadsheet_previews.retained_bytes(),
                            );
                        self.preview_state
                            .web_previews
                            .accept(request, result, other_bytes);
                    }
                }
            }
        }
    }

    pub(crate) fn apply_preview_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::HwpSourceReady(request) => {
                self.reconcile_previews();
                if self.closing.is_none() && !self.services.state.is_shutting_down() {
                    self.preview_state.hwp_previews.source_ready(&request);
                }
            }
            Reply::HwpPreview { request, result } => {
                self.reconcile_previews();
                if self.closing.is_some() || self.services.state.is_shutting_down() {
                    self.preview_state.hwp_previews.cancelled(&request);
                } else {
                    self.preview_state.hwp_previews.accept(
                        context,
                        request,
                        result,
                        self.preview_state
                            .previews
                            .retained_bytes()
                            .saturating_add(self.preview_state.pdf_previews.retained_bytes())
                            .saturating_add(
                                self.preview_state.presentation_previews.retained_bytes(),
                            )
                            .saturating_add(
                                self.preview_state.spreadsheet_previews.retained_bytes(),
                            )
                            .saturating_add(self.preview_state.web_previews.retained_bytes()),
                    );
                }
            }
            Reply::SpreadsheetSourceReady(request) => {
                self.reconcile_previews();
                if self.closing.is_none() && !self.services.state.is_shutting_down() {
                    self.preview_state
                        .spreadsheet_previews
                        .source_ready(&request);
                }
            }
            Reply::SpreadsheetPreview { request, result } => {
                self.reconcile_previews();
                if self.closing.is_some() || self.services.state.is_shutting_down() {
                    self.preview_state.spreadsheet_previews.cancelled(&request);
                } else {
                    self.preview_state.spreadsheet_previews.accept(
                        request,
                        result,
                        self.preview_state
                            .previews
                            .retained_bytes()
                            .saturating_add(self.preview_state.pdf_previews.retained_bytes())
                            .saturating_add(
                                self.preview_state.presentation_previews.retained_bytes(),
                            )
                            .saturating_add(self.preview_state.hwp_previews.retained_bytes())
                            .saturating_add(self.preview_state.web_previews.retained_bytes()),
                    );
                }
            }
            Reply::PresentationSourceReady(request) => {
                self.reconcile_previews();
                if self.closing.is_none() && !self.services.state.is_shutting_down() {
                    self.preview_state
                        .presentation_previews
                        .source_ready(&request);
                }
            }
            Reply::PresentationPreview { request, result } => {
                self.reconcile_previews();
                if self.closing.is_some() || self.services.state.is_shutting_down() {
                    self.preview_state.presentation_previews.cancelled(&request);
                } else {
                    self.preview_state.presentation_previews.accept(
                        request,
                        result,
                        self.preview_state
                            .previews
                            .retained_bytes()
                            .saturating_add(self.preview_state.pdf_previews.retained_bytes())
                            .saturating_add(
                                self.preview_state.spreadsheet_previews.retained_bytes(),
                            )
                            .saturating_add(self.preview_state.hwp_previews.retained_bytes())
                            .saturating_add(self.preview_state.web_previews.retained_bytes()),
                    );
                }
            }
            Reply::PdfSourceReady(request) => {
                self.reconcile_previews();
                if self.closing.is_none() && !self.services.state.is_shutting_down() {
                    self.preview_state.pdf_previews.source_ready(&request);
                }
            }
            Reply::PdfPreview { request, result } => {
                self.reconcile_previews();
                if self.closing.is_some() || self.services.state.is_shutting_down() {
                    self.preview_state.pdf_previews.cancelled(&request);
                } else {
                    self.preview_state.pdf_previews.accept(
                        context,
                        request,
                        result,
                        self.preview_state
                            .previews
                            .retained_bytes()
                            .saturating_add(
                                self.preview_state.presentation_previews.retained_bytes(),
                            )
                            .saturating_add(
                                self.preview_state.spreadsheet_previews.retained_bytes(),
                            )
                            .saturating_add(self.preview_state.hwp_previews.retained_bytes())
                            .saturating_add(self.preview_state.web_previews.retained_bytes()),
                    );
                }
            }
            Reply::Preview { request, result } => {
                self.reconcile_previews();
                if self.closing.is_some() || self.services.state.is_shutting_down() {
                    self.preview_state.previews.cancelled(&request);
                } else {
                    self.preview_state.previews.accept_with_other_bytes(
                        context,
                        request,
                        result,
                        self.preview_state
                            .pdf_previews
                            .retained_bytes()
                            .saturating_add(
                                self.preview_state.presentation_previews.retained_bytes(),
                            )
                            .saturating_add(
                                self.preview_state.spreadsheet_previews.retained_bytes(),
                            )
                            .saturating_add(self.preview_state.hwp_previews.retained_bytes())
                            .saturating_add(self.preview_state.web_previews.retained_bytes()),
                    );
                }
            }
        }
    }

    pub(crate) fn apply_preview_intent(&mut self, intent: Intent, context: &egui::Context) {
        match intent {
            Intent::ReopenWith { path, preview } => {
                self.preview_state.open_with.set(
                    path,
                    if preview {
                        crate::open_with::Mode::Preview
                    } else {
                        crate::open_with::Mode::Editor
                    },
                );
                context.request_repaint();
            }
        }
    }

    pub(crate) fn reconnect_preview(&mut self, context: &egui::Context) {
        self.preview_state.previews.reset_pending();
        self.preview_state.hwp_previews.reset_pending();
        self.preview_state.pdf_previews.reset_pending();
        self.preview_state.presentation_previews.reset_pending();
        self.preview_state.spreadsheet_previews.reset_pending();
        match connect_web(
            self.services.clone(),
            context,
            self.preview_state.helper_executable.clone(),
            &self.preview_state.pdf_appearance,
        ) {
            Ok(bridge) => self.preview_state.web_bridge = Some(bridge),
            Err(error) => self.status = Some(error.to_string()),
        }
    }

    pub(crate) fn release_preview_tab(
        &mut self,
        closed: &crate::tabs::ClosedNativeTab,
        path: &str,
    ) {
        if let Some(layout) = self.services.state.layouts.read().get(&closed.project) {
            self.preview_state.open_with.close_path(path, layout);
        }
        if !closed.has_remaining_file {
            self.preview_state.previews.invalidate(path);
            self.cancel_web(Some(std::path::Path::new(path)));
            self.preview_state.web_previews.invalidate(path);
        }
        self.preview_state
            .pdf_previews
            .reconcile(&self.pdf_preview_paths());
        self.preview_state
            .presentation_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Presentation));
        self.preview_state
            .spreadsheet_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Spreadsheet));
    }

    pub(crate) fn reconcile_previews(&mut self) {
        let invalidation = self.explorer_search_state.tree_changes.take_previews();
        if invalidation.all {
            self.failed.clear();
            self.refused.clear();
            self.cancel_web(None);
            self.preview_state.previews.invalidate_all();
            self.preview_state.hwp_previews.invalidate_all();
            self.preview_state.pdf_previews.invalidate_all();
            self.preview_state.presentation_previews.invalidate_all();
            self.preview_state.spreadsheet_previews.invalidate_all();
            self.preview_state.web_previews.invalidate_all();
        } else {
            for path in invalidation.paths {
                self.failed.remove(&path);
                self.refused.remove(&path);
                self.cancel_web(Some(std::path::Path::new(&path)));
                self.preview_state.previews.invalidate(&path);
                self.preview_state.hwp_previews.invalidate(&path);
                self.preview_state.pdf_previews.invalidate(&path);
                self.preview_state.presentation_previews.invalidate(&path);
                self.preview_state.spreadsheet_previews.invalidate(&path);
                self.preview_state.web_previews.invalidate(&path);
            }
            let projects = self.services.state.projects.read();
            for project in invalidation.projects {
                if let Some(project) = projects.get(&project) {
                    self.failed
                        .retain(|path, _| !std::path::Path::new(path).starts_with(&project.root));
                    self.refused
                        .retain(|path| !std::path::Path::new(path).starts_with(&project.root));
                    self.cancel_web(Some(std::path::Path::new(&project.root)));
                    self.preview_state.previews.invalidate_root(&project.root);
                    self.preview_state
                        .hwp_previews
                        .invalidate_root(&project.root);
                    self.preview_state
                        .pdf_previews
                        .invalidate_root(&project.root);
                    self.preview_state
                        .presentation_previews
                        .invalidate_root(&project.root);
                    self.preview_state
                        .spreadsheet_previews
                        .invalidate_root(&project.root);
                    self.preview_state
                        .web_previews
                        .invalidate_root(&project.root);
                }
            }
        }
        let layouts = self.services.state.layouts.read().clone();
        if !self.failed.is_empty() || !self.refused.is_empty() {
            let live = layouts
                .values()
                .flat_map(taide_layout::service::all_roots)
                .flat_map(crate::tabs::tabs_in)
                .filter_map(|tab| match &tab.kind {
                    TabKind::File { path } => Some(path),
                    _ => None,
                })
                .collect::<HashSet<_>>();
            self.failed.retain(|path, _| live.contains(path));
            self.refused.retain(|path| live.contains(path));
        }
        self.preview_state.open_with.reconcile(&layouts);
        let projects = self.services.state.projects.read();
        let paths = layouts
            .iter()
            .filter(|(project, _)| projects.contains_key(*project))
            .flat_map(|(_, layout)| taide_layout::service::all_roots(layout))
            .flat_map(crate::tabs::tabs_in)
            .filter_map(|tab| match &tab.kind {
                TabKind::File { path }
                    if self.preview_state.open_with.surface(path)
                        == crate::open_with::Surface::Preview(
                            crate::open_with::PreviewKind::Image,
                        ) =>
                {
                    Some(path.clone())
                }
                _ => None,
            })
            .collect();
        self.preview_state.previews.retain(&paths);
        drop(projects);
        self.preview_state
            .pdf_previews
            .reconcile(&self.pdf_preview_paths());
        self.preview_state
            .hwp_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Hwp));
        self.preview_state
            .presentation_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Presentation));
        self.preview_state
            .spreadsheet_previews
            .reconcile(&self.preview_tab_paths(crate::open_with::PreviewKind::Spreadsheet));
        let paths = self.web_preview_paths();
        if self
            .preview_state
            .web_previews
            .active_request()
            .is_some_and(|request| !paths.values().any(|path| path == &request.path))
        {
            self.cancel_web(None);
        }
        self.preview_state.web_previews.reconcile(&paths);
    }

    pub(crate) fn cancel_web(&self, changed: Option<&std::path::Path>) {
        if let Some(request) = self.preview_state.web_previews.active_request()
            && changed.is_none_or(|path| std::path::Path::new(&request.path).starts_with(path))
            && let Some(bridge) = &self.preview_state.web_bridge
        {
            bridge.cancel_through(request.token);
        }
    }

    pub(crate) fn pdf_preview_paths(&self) -> HashMap<TabId, String> {
        self.preview_tab_paths(crate::open_with::PreviewKind::Pdf)
    }

    fn web_preview_paths(&self) -> HashMap<TabId, String> {
        [
            crate::open_with::PreviewKind::Html,
            crate::open_with::PreviewKind::Audio,
            crate::open_with::PreviewKind::Video,
        ]
        .into_iter()
        .flat_map(|kind| self.preview_tab_paths(kind))
        .collect()
    }

    pub(crate) fn preview_tab_paths(
        &self,
        kind: crate::open_with::PreviewKind,
    ) -> HashMap<TabId, String> {
        let layouts = self.services.state.layouts.read().clone();
        let projects = self.services.state.projects.read();
        layouts
            .iter()
            .filter(|(project, _)| projects.contains_key(*project))
            .flat_map(|(_, layout)| taide_layout::service::all_roots(layout))
            .flat_map(crate::tabs::tabs_in)
            .filter_map(|tab| match &tab.kind {
                TabKind::File { path }
                    if self.preview_state.open_with.surface(path)
                        == crate::open_with::Surface::Preview(kind) =>
                {
                    Some((tab.id.clone(), path.clone()))
                }
                _ => None,
            })
            .collect()
    }

    pub(crate) fn tick_preview_media(&mut self) {
        if let Some(bridge) = &self.preview_state.web_bridge
            && bridge.set_media_appearance(media_appearance(&self.preview_state.pdf_appearance))
        {
            if let Some(request) = self.preview_state.web_previews.active_request()
                && matches!(
                    crate::open_with::preview_kind(&request.path),
                    Some(
                        crate::open_with::PreviewKind::Audio | crate::open_with::PreviewKind::Video
                    )
                )
            {
                bridge.cancel_through(request.token);
            }
            self.preview_state.web_previews.invalidate_media();
        }
    }

    pub(crate) fn show_preview_web_views(
        &mut self,
        context: &egui::Context,
        frame: &mut eframe::Frame,
        web_placements: Vec<crate::preview_web_view::Placement>,
        is_enabled: bool,
    ) {
        let live = self
            .web_preview_paths()
            .into_iter()
            .filter_map(|(tab, path)| {
                self.preview_state
                    .web_previews
                    .source(&path)
                    .cloned()
                    .map(|source| (tab, source))
            })
            .collect();
        let web_enabled = is_enabled
            && !self.commands_state.keybindings.is_open()
            && !self.commands_state.palette.is_open()
            && self.pending_tab_close.is_none()
            && self.explorer_search_state.pending_entry_delete.is_none()
            && !egui::Popup::is_any_open(context)
            && !egui::DragAndDrop::has_any_payload(context)
            && !taide_native_ui::project_drag::is_dragging(context)
            && !taide_native_ui::tab_drag::is_dragging(context)
            && !crate::file_drop::is_hovering(context)
            && !self.preview_state.web_views.drags().is_hovering();
        if let Err(error) =
            self.preview_state
                .web_views
                .sync(frame, context, &live, &web_placements, web_enabled)
        {
            self.preview_state.web_views.clear();
            for placement in web_placements {
                if let Some(path) = self.web_preview_paths().get(&placement.tab) {
                    self.preview_state.web_previews.renderer_failed(path);
                }
            }
            self.status = Some(error.to_string());
        }
    }

    pub(crate) fn load_preview_requests(&mut self, context: &egui::Context, output: FrameOutput) {
        for (path, max_side) in output.load_previews {
            if let Some(request) = self.preview_state.previews.begin(&path, max_side)
                && !self.submit(HostCommand::Preview(Command::ReadPreview(request.clone())))
            {
                self.preview_state.previews.cancelled(&request);
                context.request_repaint();
            }
        }
        for (tab, path, max_side) in output.load_pdf_previews {
            if let Some(request) = self.preview_state.pdf_previews.begin(&tab, &path, max_side)
                && !self.submit(HostCommand::Preview(Command::ReadPdfPreview(
                    request.clone(),
                )))
            {
                self.preview_state.pdf_previews.cancelled(&request);
                context.request_repaint();
            }
        }
        for (tab, path, max_side) in output.load_hwp_previews {
            if let Some(request) = self.preview_state.hwp_previews.begin(&tab, &path, max_side)
                && !self.submit(HostCommand::Preview(Command::ReadHwpPreview(
                    request.clone(),
                )))
            {
                self.preview_state.hwp_previews.cancelled(&request);
                context.request_repaint();
            }
        }
        for (tab, path) in output.load_presentation_previews {
            if let Some(request) = self.preview_state.presentation_previews.begin(&tab, &path)
                && !self.submit(HostCommand::Preview(Command::ReadPresentationPreview(
                    request.clone(),
                )))
            {
                self.preview_state.presentation_previews.cancelled(&request);
                context.request_repaint();
            }
        }
        for (tab, path) in output.load_spreadsheet_previews {
            if let Some(request) = self.preview_state.spreadsheet_previews.begin(&tab, &path)
                && !self.submit(HostCommand::Preview(Command::ReadSpreadsheetPreview(
                    request.clone(),
                )))
            {
                self.preview_state.spreadsheet_previews.cancelled(&request);
                break;
            }
        }
        for path in output.load_web_previews {
            if let Some(request) = self.preview_state.web_previews.begin(&path) {
                let result = self
                    .preview_state
                    .web_bridge
                    .as_ref()
                    .ok_or_else(|| {
                        AppError::Forbidden("native web preview host is disconnected".into())
                    })
                    .and_then(|bridge| bridge.submit(request.clone()));
                if let Err(error) = result {
                    self.preview_state
                        .web_previews
                        .accept(request, Err(error), 0);
                }
            }
        }
    }
}

impl PreviewSurfaces for AppSurfaces<'_> {
    fn can_reopen_with(&self, path: &str) -> bool {
        crate::open_with::preview_kind(path.rsplit('/').next().unwrap_or_default()).is_some()
    }
}

impl AppSurfaces<'_> {
    pub(crate) fn show_preview_tab(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        tab: &Tab,
        path: &String,
        kind: crate::open_with::PreviewKind,
        intents: &mut Vec<ShellIntent>,
    ) {
        if self.target == Some(pane) {
            *self.focused = None;
        }
        let response = ui
            .allocate_ui(ui.available_size(), |ui| {
                if kind == crate::open_with::PreviewKind::Hwp {
                    let output = crate::preview_hwp_surface::show_with_tooltips(
                        ui,
                        self.hwp_previews,
                        &tab.id,
                        path,
                        &tab.title,
                        self.locale,
                        self.pdf_appearance,
                    );
                    self.tooltips
                        .show_triggers(&output.tooltips, self.tooltip_appearance);
                    if output.external {
                        self.commands.push(HostCommand::Shell(
                            crate::domains::shell::Command::OpenPath(path.clone()),
                        ));
                    }
                    self.preview_output.load_hwp_previews.push((
                        tab.id.clone(),
                        path.clone(),
                        ui.input(|input| input.max_texture_side),
                    ));
                    return;
                }
                if kind == crate::open_with::PreviewKind::Pdf {
                    let output = crate::preview_pdf_surface::show_with_tooltips(
                        ui,
                        self.pdf_previews,
                        &tab.id,
                        path,
                        &tab.title,
                        self.locale,
                        self.pdf_appearance,
                    );
                    self.tooltips
                        .show_triggers(&output.tooltips, self.tooltip_appearance);
                    if output.external {
                        self.commands.push(HostCommand::Shell(
                            crate::domains::shell::Command::OpenPath(path.clone()),
                        ));
                    }
                    self.preview_output.load_pdf_previews.push((
                        tab.id.clone(),
                        path.clone(),
                        ui.input(|input| input.max_texture_side),
                    ));
                    return;
                }
                if kind == crate::open_with::PreviewKind::Presentation {
                    if crate::preview_presentation_surface::show(
                        ui,
                        self.presentation_previews,
                        &tab.id,
                        path,
                        &tab.title,
                        self.locale,
                        self.presentation_appearance,
                    ) {
                        self.commands.push(HostCommand::Shell(
                            crate::domains::shell::Command::OpenPath(path.clone()),
                        ));
                    }
                    self.preview_output
                        .load_presentation_previews
                        .push((tab.id.clone(), path.clone()));
                    return;
                }
                if kind == crate::open_with::PreviewKind::Spreadsheet {
                    if crate::preview_spreadsheet_surface::show(
                        ui,
                        self.spreadsheet_previews,
                        &tab.id,
                        path,
                        &tab.title,
                        self.locale,
                        self.spreadsheet_appearance,
                    ) {
                        self.commands.push(HostCommand::Shell(
                            crate::domains::shell::Command::OpenPath(path.clone()),
                        ));
                    }
                    self.preview_output
                        .load_spreadsheet_previews
                        .push((tab.id.clone(), path.clone()));
                    return;
                }
                if kind != crate::open_with::PreviewKind::Image {
                    if matches!(
                        kind,
                        crate::open_with::PreviewKind::Html
                            | crate::open_with::PreviewKind::Audio
                            | crate::open_with::PreviewKind::Video
                    ) {
                        if let Some(source) = self.web_previews.source(path) {
                            let bounds = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                            self.web_placements
                                .push(crate::preview_web_view::Placement {
                                    tab: tab.id.clone(),
                                    source: source.clone(),
                                    bounds,
                                    background: self
                                        .pdf_appearance
                                        .background
                                        .to_srgba_unmultiplied(),
                                    focused: self.target == Some(pane),
                                });
                            ui.allocate_rect(bounds, egui::Sense::hover());
                        } else {
                            let appearance = crate::preview_status::Appearance {
                                background: self.pdf_appearance.background,
                                border: self.pdf_appearance.border,
                                foreground: self.pdf_appearance.foreground,
                                muted: self.pdf_appearance.muted,
                            };
                            if crate::preview_status::show(
                                ui,
                                &tab.id,
                                self.web_previews.error(path),
                                &tab.title,
                                self.locale,
                                &appearance,
                                "preview.notSupported",
                            ) {
                                self.commands.push(HostCommand::Shell(
                                    crate::domains::shell::Command::OpenPath(path.clone()),
                                ));
                            }
                            self.preview_output.load_web_previews.push(path.clone());
                        }
                        return;
                    }
                    ui.label(format!(
                        "native preview provider is not connected: {kind:?}"
                    ));
                    return;
                }
                if let Some(error) = self.previews.error(path) {
                    ui.label(presentation::message(
                        self.locale,
                        "preview.notSupported",
                        &[],
                    ));
                    ui.label(&tab.title);
                    ui.label(error.to_string());
                    if ui
                        .button(presentation::message(
                            self.locale,
                            "preview.openExternally",
                            &[],
                        ))
                        .clicked()
                    {
                        self.commands.push(HostCommand::Shell(
                            crate::domains::shell::Command::OpenPath(path.clone()),
                        ));
                    }
                    return;
                }
                if let Some(texture) = self.previews.advance(path, ui.ctx()) {
                    crate::preview::show_image(
                        ui,
                        texture,
                        path.rsplit('/').next().unwrap_or_default(),
                    );
                    return;
                }
                self.preview_output
                    .load_previews
                    .push((path.clone(), ui.input(|input| input.max_texture_side)));
            })
            .response;
        if response.contains_pointer() && ui.input(|input| input.pointer.any_pressed()) {
            intents.push(ShellIntent::Mutate(ShellMutation::FocusPane(pane.clone())));
        }
    }
}

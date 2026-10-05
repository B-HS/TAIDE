use std::collections::HashMap;

use eframe::egui;
use taide_model::{
    error::{AppError, AppResult},
    ids::TabId,
};
use url::Url;
#[cfg(target_os = "macos")]
use wry::WebViewBuilderExtDarwin;

pub struct Placement {
    pub tab: TabId,
    pub source: Url,
    pub bounds: egui::Rect,
    pub background: [u8; 4],
    pub focused: bool,
}

#[cfg(any(target_os = "macos", test))]
fn eligible(placement: &Placement, live: &HashMap<TabId, Url>) -> bool {
    live.get(&placement.tab) == Some(&placement.source)
        && placement.bounds.is_finite()
        && placement.bounds.is_positive()
}

#[cfg(any(target_os = "macos", test))]
fn allowed_navigation(source: &Url, candidate: &str) -> bool {
    Url::parse(candidate).is_ok_and(|candidate| {
        candidate.scheme() == source.scheme()
            && candidate.host_str() == source.host_str()
            && candidate.port() == source.port()
            && candidate.username().is_empty()
            && candidate.password().is_none()
            && candidate.path() == source.path()
            && candidate.query() == source.query()
    })
}

#[cfg(target_os = "macos")]
struct Child {
    source: Url,
    view: wry::WebView,
    visible: bool,
    focused: bool,
    background: [u8; 4],
    crashed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Default)]
pub struct Views {
    #[cfg(target_os = "macos")]
    children: HashMap<TabId, Child>,
}

impl Views {
    pub fn clear(&mut self) {
        #[cfg(target_os = "macos")]
        self.children.clear();
    }

    #[cfg(target_os = "macos")]
    pub fn sync(
        &mut self,
        frame: &eframe::Frame,
        context: &egui::Context,
        live: &HashMap<TabId, Url>,
        placements: &[Placement],
        enabled: bool,
    ) -> AppResult<()> {
        self.children.retain(|tab, child| {
            live.get(tab) == Some(&child.source)
                && placements
                    .iter()
                    .any(|placement| &placement.tab == tab && eligible(placement, live))
        });
        if self
            .children
            .values()
            .any(|child| child.crashed.load(std::sync::atomic::Ordering::Acquire))
        {
            return Err(AppError::Internal(
                "preview web content process terminated".into(),
            ));
        }
        let has_web_focus = enabled
            && placements
                .iter()
                .any(|placement| placement.focused && eligible(placement, live));
        if !has_web_focus {
            for child in self.children.values_mut().filter(|child| child.focused) {
                child.view.focus_parent().map_err(|_| {
                    AppError::Internal("preview parent focus could not be restored".into())
                })?;
                child.focused = false;
            }
        }
        let scale = f64::from(context.pixels_per_point());
        for placement in placements.iter().filter(|_| enabled) {
            if !eligible(placement, live) {
                continue;
            }
            crate::preview_web_document::validate_document_source(&placement.source)?;
            if placement.source.scheme() != "http" {
                return Err(crate::preview::invalid(
                    "web view requires the approved HTTP transport",
                ));
            }
            let bounds = wry::Rect {
                position: wry::dpi::PhysicalPosition::new(
                    f64::from(placement.bounds.min.x) * scale,
                    f64::from(placement.bounds.min.y) * scale,
                )
                .into(),
                size: wry::dpi::PhysicalSize::new(
                    f64::from(placement.bounds.width()) * scale,
                    f64::from(placement.bounds.height()) * scale,
                )
                .into(),
            };
            if !self.children.contains_key(&placement.tab) {
                let source = placement.source.clone();
                let [red, green, blue, alpha] = placement.background;
                let crashed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let report_crash = crashed.clone();
                let repaint = context.clone();
                let view = wry::WebViewBuilder::new()
                    .with_url(placement.source.as_str())
                    .with_bounds(bounds)
                    .with_background_color((red, green, blue, alpha))
                    .with_javascript_disabled()
                    .with_incognito(true)
                    .with_devtools(false)
                    .with_clipboard(false)
                    .with_general_autofill_enabled(false)
                    .with_allow_link_preview(false)
                    .with_back_forward_navigation_gestures(false)
                    .with_on_web_content_process_terminate_handler(move || {
                        report_crash.store(true, std::sync::atomic::Ordering::Release);
                        repaint.request_repaint();
                    })
                    .with_focused(false)
                    .with_visible(false)
                    .with_navigation_handler(move |candidate| {
                        allowed_navigation(&source, &candidate)
                    })
                    .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
                    .with_download_started_handler(|_, _| false)
                    .build_as_child(frame)
                    .map_err(|_| {
                        AppError::Internal("isolated preview web view could not be created".into())
                    })?;
                self.children.insert(
                    placement.tab.clone(),
                    Child {
                        source: placement.source.clone(),
                        view,
                        visible: false,
                        focused: false,
                        background: placement.background,
                        crashed,
                    },
                );
            }
            let Some(child) = self.children.get_mut(&placement.tab) else {
                continue;
            };
            if child.background != placement.background {
                let [red, green, blue, alpha] = placement.background;
                child
                    .view
                    .set_background_color((red, green, blue, alpha))
                    .map_err(|_| {
                        AppError::Internal("preview background could not be updated".into())
                    })?;
                child.background = placement.background;
            }
            child
                .view
                .set_bounds(bounds)
                .map_err(|_| AppError::Internal("preview bounds could not be updated".into()))?;
            if !child.visible {
                child
                    .view
                    .set_visible(true)
                    .map_err(|_| AppError::Internal("preview view could not be shown".into()))?;
                child.visible = true;
            }
            if placement.focused && !child.focused {
                child.view.focus().map_err(|_| {
                    AppError::Internal("preview focus could not be acquired".into())
                })?;
            }
            child.focused = placement.focused;
        }
        for (tab, child) in &mut self.children {
            let is_visible = enabled
                && placements
                    .iter()
                    .any(|placement| &placement.tab == tab && eligible(placement, live));
            if is_visible || !child.visible {
                continue;
            }
            child
                .view
                .set_visible(false)
                .map_err(|_| AppError::Internal("preview view could not be hidden".into()))?;
            if child.focused {
                child.view.focus_parent().map_err(|_| {
                    AppError::Internal("preview parent focus could not be restored".into())
                })?;
            }
            child.visible = false;
            child.focused = false;
        }
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn sync(
        &mut self,
        _: &eframe::Frame,
        _: &egui::Context,
        _: &HashMap<TabId, Url>,
        placements: &[Placement],
        _: bool,
    ) -> AppResult<()> {
        if placements.is_empty() {
            return Ok(());
        }
        Err(AppError::Internal(
            "isolated preview renderer is not implemented on this platform".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_view_placementは_閉じた_tab_古い_source_非有限と空のboundsを拒否する() {
        let tab = TabId::new();
        let source = Url::parse("http://127.0.0.1:32101/prj-synthetic/root/index.html").unwrap();
        let live = HashMap::from([(tab.clone(), source.clone())]);
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1.0, 1.0));
        let mut placement = Placement {
            tab,
            source,
            bounds,
            background: [0; 4],
            focused: false,
        };
        assert!(eligible(&placement, &live));
        assert!(!eligible(&placement, &HashMap::new()));
        placement.source = Url::parse("http://127.0.0.1:32101/prj-new/root/index.html").unwrap();
        assert!(!eligible(&placement, &live));
        placement.source = live.get(&placement.tab).unwrap().clone();
        for bounds in [
            egui::Rect::NOTHING,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::ZERO),
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(f32::NAN, 1.0)),
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(f32::INFINITY, 1.0)),
        ] {
            placement.bounds = bounds;
            assert!(!eligible(&placement, &live));
        }
    }

    #[test]
    fn web_view_navigationは_同じ_documentの_fragment以外を拒否する() {
        let source = Url::parse("http://127.0.0.1:32101/prj-synthetic/root/index.html").unwrap();
        assert!(allowed_navigation(&source, source.as_str()));
        assert!(allowed_navigation(&source, &format!("{source}#section")));
        for candidate in [
            "https://external.invalid",
            "file:///root/index.html",
            "javascript:synthetic()",
            "data:text/html,synthetic",
            "http://127.0.0.1:32102/prj-synthetic/root/index.html",
            "http://127.0.0.1:32101/prj-other/root/index.html",
            "http://user@127.0.0.1:32101/prj-synthetic/root/index.html",
            "http://127.0.0.1:32101/prj-synthetic/root/index.html?q=1",
        ] {
            assert!(!allowed_navigation(&source, candidate));
        }
    }
}

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui::{self, Rect, Ui};
use taide_model::ids::{PaneId, ProjectId};
use taide_model::layout::ProjectLayout;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::documentation::{Content, RichDocument};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewKey;
use taide_native_ui::editor_markup::{Colors, Provider};
use taide_native_ui::editor_surface::EditorAppearance;
use taide_runtime::AppServices;

const UPDATE_DELAY: Duration = Duration::from_millis(200);
const INITIAL_FRACTION: f32 = 0.5;
const MIN_FRACTION: f32 = 0.2;
const MAX_FRACTION: f32 = 1.0 - MIN_FRACTION;
const TOOLBAR_HEIGHT: f32 = 32.0;
const PREVIEW_PADDING: i8 = 16;
const PREVIEW_FONT_SIZE: f32 = 14.0;
const BUTTON_SIZE: f32 = 24.0;
const ICON_SIZE: f32 = 16.0;
const ICON_RADIUS: f32 = 2.0;
const ICON_STROKE: f32 = 1.5;

type Key = (egui::ViewportId, PaneId);

struct Source {
    document: DocumentId,
    revision: u64,
    observed: Instant,
    pending: Option<DocumentSnapshot>,
    rendered: RichDocument,
}

struct Entry {
    enabled: bool,
    fraction: f32,
    source: Option<Source>,
    project: Option<ProjectId>,
    toggle: Rect,
    editor: Rect,
    preview: Option<Rect>,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            enabled: false,
            fraction: INITIAL_FRACTION,
            source: None,
            project: None,
            toggle: Rect::NOTHING,
            editor: Rect::NOTHING,
            preview: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<Key, Entry>,
    images: crate::editor_documentation_images::Cache,
}

impl State {
    #[cfg(test)]
    pub(crate) fn geometry(
        &self,
        viewport: egui::ViewportId,
        pane: &PaneId,
    ) -> (Rect, Rect, Option<Rect>) {
        let entry = &self.entries[&(viewport, pane.clone())];
        (entry.toggle, entry.editor, entry.preview)
    }

    #[cfg(test)]
    pub(crate) fn expire_update(&mut self, viewport: egui::ViewportId, pane: &PaneId) {
        let source = self
            .entries
            .get_mut(&(viewport, pane.clone()))
            .unwrap()
            .source
            .as_mut()
            .unwrap();
        assert!(source.pending.is_some());
        source.observed -= UPDATE_DELAY;
    }

    pub(crate) fn reconcile(
        &mut self,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        store: &EditorStore,
    ) {
        self.entries.retain(|(_, pane), entry| {
            let has_pane = layouts.values().any(|layout| {
                taide_layout::service::all_roots(layout)
                    .any(|root| taide_layout::service::find_leaf(root, pane).is_some())
            });
            if entry.source.as_ref().is_some_and(|source| {
                !store.views().for_document(source.document).any(|view| {
                    &view.key.pane == pane
                        && layouts.values().any(|layout| {
                            taide_layout::service::all_roots(layout).any(|root| {
                                taide_native_ui::snapshot::active_tab(root, pane)
                                    .is_some_and(|tab| tab.id == view.key.tab)
                            })
                        })
                })
            }) {
                entry.source = None;
                entry.preview = None;
            }
            has_pane
        });
        self.images.retain_documents(
            self.entries.values().filter_map(|entry| {
                Some((entry.project.as_ref()?, &entry.source.as_ref()?.rendered))
            }),
        );
    }

    pub(crate) fn toolbar(&mut self, ui: &mut Ui, pane: &PaneId, language: &str, label: &str) {
        let key = (ui.ctx().viewport_id(), pane.clone());
        if language != "markdown" {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.source = None;
                entry.preview = None;
                entry.toggle = Rect::NOTHING;
            }
            return;
        }
        let entry = self.entries.entry(key).or_default();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), TOOLBAR_HEIGHT),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                let response = ui.add(
                    egui::Button::new("")
                        .selected(entry.enabled)
                        .min_size(egui::vec2(BUTTON_SIZE, BUTTON_SIZE)),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        entry.enabled,
                        label,
                    )
                });
                let icon = Rect::from_center_size(
                    response.rect.center(),
                    egui::vec2(ICON_SIZE, ICON_SIZE),
                );
                let stroke = egui::Stroke::new(ICON_STROKE, ui.visuals().text_color());
                ui.painter().rect_stroke(
                    icon,
                    egui::CornerRadius::from(ICON_RADIUS),
                    stroke,
                    egui::StrokeKind::Inside,
                );
                ui.painter().vline(icon.center().x, icon.y_range(), stroke);
                entry.toggle = response.rect;
                if response.clicked() {
                    entry.enabled = !entry.enabled;
                    if !entry.enabled {
                        entry.source = None;
                    }
                }
                response.on_hover_text(label);
            },
        );
    }

    pub(crate) fn regions(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        language: &str,
        thickness: f32,
        border: egui::Color32,
    ) -> (Rect, Option<Rect>) {
        let rect = ui.available_rect_before_wrap();
        let key = (ui.ctx().viewport_id(), pane.clone());
        let Some(entry) = self.entries.get_mut(&key) else {
            return (rect, None);
        };
        if !entry.enabled || language != "markdown" {
            entry.editor = rect;
            entry.preview = None;
            return (rect, None);
        }
        let gap = thickness.max(0.0).min(rect.width());
        let extent = (rect.width() - gap).max(0.0);
        let center = rect.left() + extent * entry.fraction + gap * INITIAL_FRACTION;
        let response = ui.interact(
            Rect::from_center_size(
                egui::pos2(center, rect.center().y),
                egui::vec2(taide_native_ui::split::RESIZE_HIT_SIZE, rect.height()),
            ),
            egui::Id::new(("markdown-divider", &key)),
            egui::Sense::click_and_drag(),
        );
        if response.dragged() && extent > 0.0 {
            entry.fraction = (entry.fraction + response.drag_delta().x / extent)
                .clamp(MIN_FRACTION, MAX_FRACTION);
        }
        if response.has_focus() && extent > 0.0 {
            let delta = ui.input_mut(|input| {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                    -taide_native_ui::split::KEYBOARD_RESIZE_STEP
                } else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                    taide_native_ui::split::KEYBOARD_RESIZE_STEP
                } else {
                    0.0
                }
            });
            entry.fraction = (entry.fraction + delta / extent).clamp(MIN_FRACTION, MAX_FRACTION);
        }
        let edge = rect.left() + extent * entry.fraction;
        ui.painter().vline(
            edge + gap * INITIAL_FRACTION,
            rect.y_range(),
            egui::Stroke::new(gap, border),
        );
        entry.editor = Rect::from_min_max(rect.min, egui::pos2(edge, rect.bottom()));
        entry.preview = Some(Rect::from_min_max(
            egui::pos2(edge + gap, rect.top()),
            rect.max,
        ));
        (entry.editor, entry.preview)
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        pane: &PaneId,
        owner: &ViewKey,
        project: Option<&ProjectId>,
        snapshot: &DocumentSnapshot,
        appearance: &EditorAppearance,
        colors: Colors,
        services: &AppServices,
        commands: &mut Vec<crate::host::HostCommand>,
    ) {
        let key = (ui.ctx().viewport_id(), pane.clone());
        let Some(entry) = self.entries.get_mut(&key) else {
            return;
        };
        let Some(rect) = entry.preview else { return };
        let now = Instant::now();
        entry.project = project.cloned();
        if entry
            .source
            .as_ref()
            .is_none_or(|source| source.document != snapshot.id)
        {
            entry.source = Some(Source {
                document: snapshot.id,
                revision: snapshot.revision,
                observed: now,
                pending: None,
                rendered: crate::editor_markup::parse(&Content::Markdown(
                    snapshot.rope.to_string(),
                )),
            });
        }
        let source = entry
            .source
            .as_mut()
            .expect("preview source was initialized");
        if source.revision != snapshot.revision {
            source.revision = snapshot.revision;
            source.observed = now;
            source.pending = Some(snapshot.clone());
        }
        if let Some(pending) = &source.pending {
            let elapsed = now.saturating_duration_since(source.observed);
            if elapsed >= UPDATE_DELAY {
                source.rendered =
                    crate::editor_markup::parse(&Content::Markdown(pending.rope.to_string()));
                source.pending = None;
            } else {
                ui.ctx().request_repaint_after(UPDATE_DELAY - elapsed);
            }
        }
        self.images.prepare(
            ui.ctx(),
            services,
            self.entries.values().filter_map(|entry| {
                Some((entry.project.as_ref()?, &entry.source.as_ref()?.rendered))
            }),
        );
        let entry = &self.entries[&key];
        let source = entry
            .source
            .as_ref()
            .expect("preview source remains initialized");
        let mut preview_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("markdown-preview", &key))
                .max_rect(rect),
        );
        preview_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        preview_ui
            .painter()
            .rect_filled(rect, egui::CornerRadius::ZERO, appearance.background);
        let mut preview_appearance = appearance.clone();
        preview_appearance.font.size = PREVIEW_FONT_SIZE;
        let colors = Colors {
            background: appearance.background,
            foreground: appearance.foreground,
            ..colors
        };
        let mut provider = PreviewProvider {
            images: &mut self.images,
            project,
            owner,
            viewport: key.0,
            commands,
        };
        egui::Frame::new()
            .inner_margin(PREVIEW_PADDING)
            .show(&mut preview_ui, |ui| {
                egui::ScrollArea::both()
                    .id_salt(("markdown-scroll", &key))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        taide_native_ui::editor_markup::show(
                            ui,
                            &source.rendered,
                            &preview_appearance,
                            colors,
                            &mut provider,
                        );
                    });
            });
    }
}

struct PreviewProvider<'a> {
    images: &'a mut crate::editor_documentation_images::Cache,
    project: Option<&'a ProjectId>,
    owner: &'a ViewKey,
    viewport: egui::ViewportId,
    commands: &'a mut Vec<crate::host::HostCommand>,
}

impl Provider for PreviewProvider<'_> {
    fn open_link(&mut self, ui: &Ui, target: &str) -> bool {
        crate::editor_documentation::open_link(
            ui,
            target,
            self.project,
            self.owner,
            self.viewport,
            self.commands,
        )
    }

    fn image(
        &mut self,
        ui: &mut Ui,
        source: &str,
        alt: &str,
        dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<egui::Response> {
        self.images.show(ui, self.project?, source, alt, dimensions)
    }
}

#[cfg(test)]
#[path = "editor-markdown-tests.rs"]
mod tests;

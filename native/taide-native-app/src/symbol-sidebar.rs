use std::collections::HashMap;

use eframe::egui::{self, Id, Rect, Response, Sense, Ui, vec2};
use taide_model::{
    error::AppResult,
    ids::{ProjectId, ShellSlotId},
    layout::{PaneNode, ProjectLayout},
    locale::ResolvedLocale,
    project::ShellSlotTree,
};
use taide_native_ui::shell::WindowScope;

use crate::navigation_icons::{Icon, Icons};
use crate::symbol_outline::{Appearance, Panel};

const HEADER_HEIGHT: f32 = 36.0;
const BUTTON_SIZE: f32 = 24.0;
const ICON_SIZE: f32 = 16.0;
const PADDING: f32 = 8.0;
const GAP: f32 = 4.0;
const BUTTON_RADIUS: u8 = 2;
const BORDER_WIDTH: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum View {
    #[default]
    Files,
    Outline,
}

impl View {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Files => "explorer.title",
            Self::Outline => "outline.title",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Files => Icon::FolderTree,
            Self::Outline => Icon::ListTree,
        }
    }
}

pub(crate) struct Sidebar {
    project: ProjectId,
    pub view: View,
    pub outline: Panel,
    composing: bool,
}

#[derive(Default)]
pub(crate) struct Views {
    entries: HashMap<ShellSlotId, Sidebar>,
}

impl Views {
    pub(crate) fn reconcile(&mut self, tree: Option<&ShellSlotTree>) {
        self.entries.retain(|slot, entry| {
            tree.and_then(|tree| taide_native_ui::snapshot::slot_project(tree, slot))
                == Some(&entry.project)
        });
    }

    pub(crate) fn entry(&mut self, project: &ProjectId, slot: &ShellSlotId) -> &mut Sidebar {
        let entry = self.entries.entry(slot.clone()).or_insert_with(|| Sidebar {
            project: project.clone(),
            view: View::Files,
            outline: Panel::default(),
            composing: false,
        });
        if entry.project != *project {
            *entry = Sidebar {
                project: project.clone(),
                view: View::Files,
                outline: Panel::default(),
                composing: false,
            };
        }
        entry
    }
}

pub(crate) fn window_tree<'a>(
    project: &ProjectId,
    layout: &'a ProjectLayout,
    scope: &WindowScope,
) -> Option<(&'a PaneNode, &'a taide_model::ids::PaneId)> {
    match scope {
        WindowScope::Main => Some((&layout.root, &layout.focused_pane)),
        WindowScope::Auxiliary {
            project: owner,
            slot,
        } if owner == project => layout
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == *slot)
            .map(|window| (&window.root, &window.focused_pane)),
        WindowScope::Auxiliary { .. } => None,
    }
}

impl Sidebar {
    pub(crate) fn switch(
        &mut self,
        ui: &mut Ui,
        id: Id,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &mut Icons,
    ) -> AppResult<Vec<(View, Response)>> {
        let ime_frame = ui.input(|input| {
            let mut ime_frame = false;
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    ime_frame = true;
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
            ime_frame
        });
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, HEADER_HEIGHT), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, appearance.sidebar);
        ui.painter().hline(
            rect.x_range(),
            rect.bottom(),
            egui::Stroke::new(BORDER_WIDTH, appearance.sidebar_border),
        );
        let responses = [View::Files, View::Outline]
            .into_iter()
            .enumerate()
            .map(|(index, view)| {
                let button = Rect::from_center_size(
                    egui::pos2(
                        rect.left()
                            + PADDING
                            + BUTTON_SIZE / 2.0
                            + index as f32 * (BUTTON_SIZE + GAP),
                        rect.center().y,
                    ),
                    vec2(BUTTON_SIZE, BUTTON_SIZE),
                );
                let response = ui.interact(button, id.with(view.label()), Sense::click());
                let activated = ui.is_enabled()
                    && !ime_frame
                    && !self.composing
                    && response.has_focus()
                    && ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            || input.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                    });
                if response.clicked_by(egui::PointerButton::Primary)
                    || (!ime_frame && !self.composing && response.clicked())
                    || activated
                {
                    self.view = view;
                    response.request_focus();
                }
                (view, response)
            })
            .collect::<Vec<_>>();
        for (view, response) in &responses {
            let button = response.rect;
            let active = self.view == *view;
            if active || response.hovered() {
                ui.painter().rect_filled(
                    button,
                    BUTTON_RADIUS,
                    if active {
                        appearance.selected
                    } else {
                        appearance.hover
                    },
                );
            }
            icons.paint(
                ui,
                Rect::from_center_size(button.center(), vec2(ICON_SIZE, ICON_SIZE)),
                view.icon(),
                if active {
                    appearance.foreground
                } else {
                    appearance.muted
                },
                0.0,
            )?;
            response.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    ui.is_enabled(),
                    crate::presentation::message(locale, view.label(), &[]),
                )
            });
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(egui::accesskit::Role::Tab);
                node.set_selected(self.view == *view);
            });
        }
        Ok(responses)
    }
}

#[cfg(test)]
#[path = "symbol-sidebar-tests.rs"]
mod tests;

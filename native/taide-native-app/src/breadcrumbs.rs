use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Align, Id, Layout, Popup, Rect, Response, Sense, Ui, UiBuilder, vec2};
use taide_model::{
    error::AppResult,
    ids::{PaneId, ProjectId, TabId},
    layout::{ProjectLayout, TabKind},
    locale::ResolvedLocale,
    project::ShellSlotTree,
    tree::{TreeEntryKind, TreeRow},
};
use taide_native_editor::document::{DocumentId, DocumentKey, DocumentSnapshot};
use taide_native_ui::command_palette::SymbolIndex;

use crate::{
    breadcrumb_menu::{Entry, Menu, Target},
    navigation_icons::{Icon, Icons},
    symbol_navigation::{Tree, direct_children, path_segments},
    symbol_outline::Appearance,
};

const HEIGHT: f32 = 32.0;
const PADDING: f32 = 8.0;
const GAP: f32 = 2.0;
const TEXT_SIZE: f32 = 12.0;
const SEPARATOR_SIZE: f32 = 14.0;
const SEPARATOR_OPACITY: f32 = 0.6;
const BORDER_WIDTH: f32 = 1.0;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Source {
    pub(crate) project: ProjectId,
    pub(crate) pane: PaneId,
    pub(crate) tab: TabId,
    pub(crate) path: String,
    pub(crate) document: DocumentId,
    pub(crate) revision: u64,
    pub(crate) language: String,
    pub(crate) generation: u64,
}

impl Source {
    pub(crate) fn is_active(&self, layout: &ProjectLayout) -> bool {
        taide_layout::service::all_roots(layout).any(|root| {
            taide_native_ui::snapshot::active_tab(root, &self.pane).is_some_and(|tab| {
                tab.id == self.tab
                    && matches!(&tab.kind, TabKind::File { path } if path == &self.path)
            })
        })
    }

    pub(crate) fn describes(&self, snapshot: &DocumentSnapshot, generation: u64) -> bool {
        self.document == snapshot.id
            && self.revision == snapshot.revision
            && self.language == snapshot.metadata.language_id
            && self.generation == generation
            && matches!(&snapshot.key, DocumentKey::File(path) if path == std::path::Path::new(&self.path))
    }
}

pub(crate) enum Action {
    RevealTree(Source),
    OpenFile { source: Source, path: String },
    RevealSymbol { source: Source, index: usize },
}

pub(crate) struct Scope<'a> {
    pub source: Option<Source>,
    pub root: &'a str,
    pub caret: Option<usize>,
    pub symbols: SymbolIndex<'a>,
    pub rows: &'a [TreeRow],
}

#[derive(Default)]
pub(crate) struct Output {
    pub actions: Vec<Action>,
    pub segments: Vec<Response>,
    pub entries: Vec<Response>,
}

#[derive(Default)]
pub(crate) struct Bar {
    source: Option<Source>,
    tree: Option<Tree>,
    menus: HashMap<Id, Menu>,
    composing: bool,
}

#[derive(Default)]
pub(crate) struct Views {
    entries: HashMap<(egui::ViewportId, ProjectId, PaneId, TabId), Bar>,
}

impl Views {
    pub(crate) fn entry(&mut self, viewport: egui::ViewportId, source: &Source) -> &mut Bar {
        self.entries
            .entry((
                viewport,
                source.project.clone(),
                source.pane.clone(),
                source.tab.clone(),
            ))
            .or_default()
    }

    pub(crate) fn reconcile(
        &mut self,
        ctx: &egui::Context,
        layouts: &HashMap<ProjectId, ProjectLayout>,
        tree: Option<&ShellSlotTree>,
        scope: &taide_native_ui::shell::WindowScope,
    ) {
        self.entries.retain(|(viewport, project, _, _), bar| {
            let live = bar.source.as_ref().is_some_and(|source| {
                layouts.get(project).is_some_and(|layout| {
                    source.is_active(layout)
                        && (*viewport != ctx.viewport_id()
                            || (crate::symbol_sidebar::window_tree(project, layout, scope)
                                .is_some_and(|(root, _)| {
                                    taide_layout::service::find_leaf(root, &source.pane).is_some()
                                })
                                && match scope {
                                    taide_native_ui::shell::WindowScope::Main => {
                                        tree.is_some_and(|tree| contains_project(tree, project))
                                    }
                                    taide_native_ui::shell::WindowScope::Auxiliary {
                                        project: owner,
                                        ..
                                    } => owner == project,
                                }))
                })
            });
            if !live && *viewport == ctx.viewport_id() {
                bar.close(ctx);
            }
            live
        });
    }
}

fn contains_project(tree: &ShellSlotTree, project: &ProjectId) -> bool {
    match tree {
        ShellSlotTree::Leaf { project_id, .. } => project_id == project,
        ShellSlotTree::Split { children, .. } => children
            .iter()
            .any(|child| contains_project(child, project)),
    }
}

impl Bar {
    fn close(&mut self, ctx: &egui::Context) {
        for id in self.menus.keys() {
            Popup::close_id(ctx, *id);
        }
        self.menus.clear();
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut Ui,
        id: Id,
        scope: Scope<'_>,
        locale: &ResolvedLocale,
        appearance: &Appearance,
        icons: &mut Icons,
    ) -> AppResult<Output> {
        if self.source != scope.source {
            self.close(ui.ctx());
            self.tree = scope
                .source
                .as_ref()
                .and(scope.symbols.entries)
                .map(Tree::new);
            self.source = scope.source.clone();
        } else if self.tree.is_none() {
            self.tree = scope
                .source
                .as_ref()
                .and(scope.symbols.entries)
                .map(Tree::new);
        }
        let ime_frame = ui.input(|input| {
            let mut found = false;
            for event in &input.events {
                if let egui::Event::Ime(event) = event {
                    found = true;
                    match event {
                        egui::ImeEvent::Preedit { text, .. } => self.composing = !text.is_empty(),
                        egui::ImeEvent::Commit(_) => self.composing = false,
                        _ => {}
                    }
                }
            }
            found
        });
        let keyboard = !self.composing && !ime_frame;
        let paths = scope
            .source
            .as_ref()
            .map_or_else(Vec::new, |source| path_segments(scope.root, &source.path));
        let chain = self
            .tree
            .as_ref()
            .zip(scope.symbols.entries)
            .zip(scope.caret)
            .map_or_else(Vec::new, |((tree, symbols), caret)| {
                tree.enclosing(symbols, caret)
            });
        let live_ids: HashSet<_> = paths
            .iter()
            .map(|path| id.with(("path", &path.path)))
            .chain(chain.iter().map(|index| {
                id.with((
                    "symbol",
                    self.tree.as_ref().and_then(|tree| tree.id(*index)),
                ))
            }))
            .collect();
        self.menus.retain(|menu_id, _| {
            let live = live_ids.contains(menu_id) && ui.is_enabled();
            if !live {
                Popup::close_id(ui.ctx(), *menu_id);
            }
            live
        });
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Other,
                ui.is_enabled(),
                crate::presentation::message(locale, "breadcrumbs.title", &[]),
            )
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Navigation)
        });
        ui.painter().rect_filled(rect, 0.0, appearance.editor);
        ui.painter().hline(
            rect.x_range(),
            rect.bottom(),
            egui::Stroke::new(BORDER_WIDTH, appearance.border),
        );
        let inner = rect.shrink2(vec2(PADDING, 0.0));
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(id)
                .max_rect(inner)
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        let mut output = Output::default();
        egui::ScrollArea::horizontal()
            .id_salt(id.with("scroll"))
            .max_height(HEIGHT)
            .show(&mut child, |ui| -> AppResult<()> {
                ui.spacing_mut().item_spacing.x = GAP;
                ui.set_height(HEIGHT);
                if scope.source.is_none() {
                    ui.label(
                        egui::RichText::new(crate::presentation::message(
                            locale,
                            "breadcrumbs.noActiveFile",
                            &[],
                        ))
                        .size(TEXT_SIZE)
                        .color(appearance.muted),
                    );
                    return Ok(());
                }
                let source = scope.source.as_ref().unwrap();
                for (index, path) in paths.iter().enumerate() {
                    if index > 0 {
                        separator(ui, appearance, icons)?;
                    }
                    let entries = || {
                        direct_children(scope.rows, &path.parent)
                            .into_iter()
                            .map(|row| Entry {
                                key: row.path.clone(),
                                label: row.name.clone(),
                                target: (row.kind == TreeEntryKind::File)
                                    .then(|| Target::File(row.path.clone())),
                            })
                            .collect::<Vec<_>>()
                    };
                    let menu_id = id.with(("path", &path.path));
                    let menu = self.menus.entry(menu_id).or_default();
                    let shown = menu.show(
                        ui,
                        menu_id,
                        &path.label,
                        index + 1 == paths.len() && chain.is_empty(),
                        true,
                        entries,
                        keyboard,
                        locale,
                        appearance,
                    );
                    if shown.opened {
                        output.actions.push(Action::RevealTree(source.clone()));
                    }
                    if let Some(Target::File(path)) = shown.selected {
                        output.actions.push(Action::OpenFile {
                            source: source.clone(),
                            path,
                        });
                    }
                    output.segments.push(shown.trigger);
                    output.entries.extend(shown.entries);
                }
                if let Some(symbols) = scope.symbols.entries
                    && let Some(tree) = &self.tree
                {
                    for (level, index) in chain.iter().enumerate() {
                        if !paths.is_empty() || level > 0 {
                            separator(ui, appearance, icons)?;
                        }
                        let siblings = tree.children(tree.parent(*index));
                        let entries = || {
                            siblings
                                .iter()
                                .map(|sibling| Entry {
                                    key: format!("symbol-{sibling}"),
                                    label: symbols[*sibling].name.clone(),
                                    target: Some(Target::Symbol(*sibling)),
                                })
                                .collect::<Vec<_>>()
                        };
                        let menu_id = id.with(("symbol", tree.id(*index)));
                        let menu = self.menus.entry(menu_id).or_default();
                        let shown = menu.show(
                            ui,
                            menu_id,
                            &symbols[*index].name,
                            level + 1 == chain.len(),
                            siblings.len() > 1,
                            entries,
                            keyboard,
                            locale,
                            appearance,
                        );
                        if let Some(Target::Symbol(index)) = shown.selected {
                            output.actions.push(Action::RevealSymbol {
                                source: source.clone(),
                                index,
                            });
                        }
                        output.segments.push(shown.trigger);
                        output.entries.extend(shown.entries);
                    }
                }
                Ok(())
            })
            .inner?;
        Ok(output)
    }
}

fn separator(ui: &mut Ui, appearance: &Appearance, icons: &mut Icons) -> AppResult<()> {
    let (rect, _) = ui.allocate_exact_size(vec2(SEPARATOR_SIZE, SEPARATOR_SIZE), Sense::hover());
    icons.paint(
        ui,
        Rect::from_center_size(rect.center(), vec2(SEPARATOR_SIZE, SEPARATOR_SIZE)),
        Icon::ChevronRight,
        appearance.muted.gamma_multiply(SEPARATOR_OPACITY),
        0.0,
    )
}

#[cfg(test)]
#[path = "breadcrumbs-tests.rs"]
mod tests;

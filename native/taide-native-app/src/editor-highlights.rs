use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32};
use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::theme::{ResolvedTheme, ThemeType};
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, OverviewLane, Stickiness,
};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::lsp::{LspRange, Position, byte_to_position, range_to_bytes};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{SelectionSet, ViewId, ViewKey};
use taide_native_ui::command_registry::HighlightCommand;
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

const REQUEST_DEBOUNCE: Duration = Duration::from_millis(50);
const TRIGGER_DELAY: Duration = Duration::from_millis(250);
const BORDER_WIDTH: f32 = 1.0;
const READ_DARK: Color32 = Color32::from_rgba_unmultiplied_const(0x57, 0x57, 0x57, 0xb8);
const READ_LIGHT: Color32 = Color32::from_rgba_unmultiplied_const(0x57, 0x57, 0x57, 0x40);
const WRITE_DARK: Color32 = Color32::from_rgba_unmultiplied_const(0x00, 0x49, 0x72, 0xb8);
const WRITE_LIGHT: Color32 = Color32::from_rgba_unmultiplied_const(0x0e, 0x63, 0x9c, 0x40);
const OVERVIEW_READ: Color32 = Color32::from_rgba_unmultiplied_const(0xa0, 0xa0, 0xa0, 0xcc);
const OVERVIEW_WRITE: Color32 = Color32::from_rgba_unmultiplied_const(0xc0, 0xa0, 0xc0, 0xcc);
const HIGHLIGHT_KIND_COUNT: usize = 3;
const WRITE_KIND: usize = 2;
const SELECTION_DARK: Color32 = Color32::from_rgb(0x26, 0x4f, 0x78);
const SELECTION_LIGHT: Color32 = Color32::from_rgb(0xad, 0xd6, 0xff);
const BACKGROUND_DARK: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x1e);
const SELECTION_PROMINENCE: f32 = 0.3;
const SELECTION_OPACITY: f32 = 0.6;
const RGB_MAX: f32 = 255.0;
const RGB_MIDPOINT: f32 = 0.5;
const RGB_DOUBLE: f32 = 2.0;
const SRGB_LINEAR_THRESHOLD: f32 = 0.03928;
const SRGB_LINEAR_DIVISOR: f32 = 12.92;
const SRGB_OFFSET: f32 = 0.055;
const SRGB_SCALE: f32 = 1.055;
const SRGB_EXPONENT: f32 = 2.4;
const LUMINANCE_WEIGHTS: [f32; 3] = [0.2126, 0.7152, 0.0722];
const LUMINANCE_PRECISION: f32 = 10_000.0;

fn luminance(color: Color32) -> f32 {
    let rgba = color.to_srgba_unmultiplied();
    let value = rgba[..LUMINANCE_WEIGHTS.len()]
        .iter()
        .zip(LUMINANCE_WEIGHTS)
        .map(|(channel, weight)| {
            let gamma = f32::from(*channel) / RGB_MAX;
            let linear = if gamma <= SRGB_LINEAR_THRESHOLD {
                gamma / SRGB_LINEAR_DIVISOR
            } else {
                ((gamma + SRGB_OFFSET) / SRGB_SCALE).powf(SRGB_EXPONENT)
            };
            linear * weight
        })
        .sum::<f32>();
    (value * LUMINANCE_PRECISION).round() / LUMINANCE_PRECISION
}

fn selection_highlight(selection: Color32, background: Color32) -> Color32 {
    let source_luminance = luminance(selection);
    let target_luminance = luminance(background);
    let lighter = source_luminance < target_luminance;
    let divisor = if lighter {
        target_luminance
    } else {
        source_luminance
    };
    let factor = if divisor > 0.0 {
        SELECTION_PROMINENCE * (source_luminance - target_luminance).abs() / divisor
    } else {
        0.0
    };
    let rgba = selection.to_srgba_unmultiplied();
    let rgb = rgba[..LUMINANCE_WEIGHTS.len()]
        .iter()
        .map(|value| f32::from(*value) / RGB_MAX)
        .collect::<Vec<_>>();
    let low = rgb.iter().copied().fold(f32::INFINITY, f32::min);
    let high = rgb.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let lightness = (low + high) / RGB_DOUBLE;
    let chroma = high - low;
    let saturation = if chroma == 0.0 {
        0.0
    } else if lightness <= RGB_MIDPOINT {
        chroma / (RGB_DOUBLE * lightness)
    } else {
        chroma / (RGB_DOUBLE - RGB_DOUBLE * lightness)
    };
    let lightness = if lighter {
        lightness + lightness * factor
    } else {
        lightness - lightness * factor
    };
    let lightness = lightness.clamp(0.0, 1.0);
    let value = lightness + saturation * lightness.min(1.0 - lightness);
    let mut hsv = egui::ecolor::Hsva::from_rgb([rgb[0], rgb[1], rgb[2]]);
    hsv.v = value;
    hsv.s = if value > 0.0 {
        RGB_DOUBLE * (1.0 - lightness / value)
    } else {
        0.0
    };
    let rgb = hsv
        .to_rgb()
        .map(|channel| (channel * RGB_MAX).round().clamp(0.0, RGB_MAX) as u8);
    let alpha = (f32::from(rgba[3]) * SELECTION_OPACITY).round() as u8;
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], alpha)
}

#[derive(Clone, Copy)]
pub(crate) struct Colors {
    background: [Color32; HIGHLIGHT_KIND_COUNT],
    border: [Color32; HIGHLIGHT_KIND_COUNT],
    overview: [Color32; HIGHLIGHT_KIND_COUNT],
    minimap: Color32,
}

impl Colors {
    pub(crate) fn from_theme(theme: &ResolvedTheme) -> AppResult<Self> {
        let value = |key: &str, fallback| {
            theme.colors.get(key).map_or(Ok(fallback), |value| {
                taide_native_ui::presentation::parse_color(value, key)
            })
        };
        let dark = theme.theme_type == ThemeType::Dark;
        let read = value(
            "editor.wordHighlightBackground",
            if dark { READ_DARK } else { READ_LIGHT },
        )?;
        let read_border = value("editor.wordHighlightBorder", Color32::TRANSPARENT)?;
        let selected = value(
            "editor.selectionBackground",
            value(
                "editor.selection",
                if dark {
                    SELECTION_DARK
                } else {
                    SELECTION_LIGHT
                },
            )?,
        )?;
        let background = value(
            "editor.background",
            if dark {
                BACKGROUND_DARK
            } else {
                Color32::WHITE
            },
        )?;
        let selection = value(
            "editor.selectionHighlightBackground",
            selection_highlight(selected, background),
        )?;
        let text_overview = value(
            "editorOverviewRuler.selectionHighlightForeground",
            OVERVIEW_READ,
        )?;
        Ok(Self {
            background: [
                value("editor.wordHighlightTextBackground", read)?,
                read,
                value(
                    "editor.wordHighlightStrongBackground",
                    if dark { WRITE_DARK } else { WRITE_LIGHT },
                )?,
            ],
            border: [
                value("editor.wordHighlightTextBorder", read_border)?,
                read_border,
                value("editor.wordHighlightStrongBorder", Color32::TRANSPARENT)?,
            ],
            overview: [
                value(
                    "editorOverviewRuler.wordHighlightTextForeground",
                    text_overview,
                )?,
                value("editorOverviewRuler.wordHighlightForeground", OVERVIEW_READ)?,
                value(
                    "editorOverviewRuler.wordHighlightStrongForeground",
                    OVERVIEW_WRITE,
                )?,
            ],
            minimap: value("minimap.selectionOccurrenceHighlight", selection)?,
        })
    }
}

pub(crate) struct Context {
    pub project: ProjectId,
    pub source: ViewId,
    pub owner: ViewId,
    pub viewport: egui::ViewportId,
}

#[derive(Clone)]
pub struct Request {
    pub(crate) project: ProjectId,
    pub(crate) snapshot: DocumentSnapshot,
    pub(crate) source: ViewId,
    pub(crate) source_key: ViewKey,
    pub(crate) owner: ViewId,
    pub(crate) owner_key: ViewKey,
    pub(crate) owner_document: taide_native_editor::document::DocumentId,
    pub(crate) selection: SelectionSet,
    pub(crate) position: Position,
    pub(crate) viewport: egui::ViewportId,
    pub(crate) token: Uuid,
    pub(crate) cancelled: watch::Receiver<bool>,
}

impl Request {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }

    pub(crate) fn describes(&self, store: &EditorStore) -> bool {
        self.describes_document(store)
            && store
                .views()
                .get(self.source)
                .is_some_and(|source| source.selection == self.selection)
    }

    fn describes_document(&self, store: &EditorStore) -> bool {
        !self.is_cancelled()
            && store.views().get(self.owner).is_some_and(|owner| {
                owner.key == self.owner_key && owner.document == self.owner_document
            })
            && store.views().get(self.source).is_some_and(|source| {
                source.key == self.source_key
                    && source.document == self.snapshot.id
                    && source.composition.is_none()
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|snapshot| {
                            snapshot.key == self.snapshot.key
                                && snapshot.revision == self.snapshot.revision
                                && snapshot.metadata.language_id
                                    == self.snapshot.metadata.language_id
                        })
            })
    }

    pub(crate) fn is_active(
        &self,
        layout: &taide_model::layout::ProjectLayout,
        scope: &taide_native_ui::shell::WindowScope,
    ) -> bool {
        crate::symbol_sidebar::window_tree(&self.project, layout, scope).is_some_and(|(root, _)| {
            taide_native_ui::snapshot::active_tab(root, &self.owner_key.pane)
                .is_some_and(|tab| tab.id == self.owner_key.tab)
        })
    }
}

pub struct Response {
    pub(crate) provider: Option<ProviderIdentity>,
    pub(crate) highlights: Vec<lsp_types::DocumentHighlight>,
}

struct Highlight {
    bytes: Range<usize>,
    kind: usize,
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    cancel: watch::Sender<bool>,
    due: Instant,
    submitted: bool,
    render_after: Instant,
    highlights: Vec<Highlight>,
}

fn has_word_selection(snapshot: &DocumentSnapshot, selection: &SelectionSet) -> bool {
    let selected = selection.selections[selection.primary];
    let selected_bytes = selected.anchor.min(selected.head)..selected.anchor.max(selected.head);
    let rules = crate::editor_syntax::language_rules(&snapshot.metadata.language_id)
        .or_else(|| crate::editor_syntax::language_rules("plaintext"));
    taide_native_editor::cursor_commands::word_range(snapshot, selected.head, rules)
        .is_some_and(|word| word.start <= selected_bytes.start && selected_bytes.end <= word.end)
}

impl Entry {
    fn refresh_selection(&mut self, store: &EditorStore) -> bool {
        if self.request.describes(store) {
            return true;
        }
        if !self.request.describes_document(store) {
            return false;
        }
        let Some(source) = store.views().get(self.request.source) else {
            return false;
        };
        let selected = source.selection.selections[source.selection.primary];
        if !has_word_selection(&self.request.snapshot, &source.selection)
            || !self.highlights.iter().any(|highlight| {
                highlight.bytes.start <= selected.head && selected.head <= highlight.bytes.end
            })
        {
            return false;
        }
        self.request.selection = source.selection.clone();
        true
    }
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<egui::ViewportId, Entry>,
    sources: HashMap<egui::ViewportId, (ViewKey, ViewKey)>,
}

pub(crate) struct Display {
    pub layer: DecorationLayer,
    borders: Vec<(Range<usize>, Color32)>,
}

impl Display {
    pub(crate) fn paint(
        &self,
        ui: &egui::Ui,
        geometry: &taide_native_ui::editor_geometry::EditorGeometry,
    ) {
        let painter = ui.painter().with_clip_rect(geometry.content_rect);
        for (bytes, color) in &self.borders {
            for rect in geometry.range_rects(bytes.clone()) {
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(BORDER_WIDTH, *color),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
}

impl State {
    pub(crate) fn observe(
        &mut self,
        store: &EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
        now: Instant,
    ) -> Result<Option<Request>, EditorError> {
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?;
        let owner = store
            .views()
            .get(context.owner)
            .ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        self.sources
            .insert(context.viewport, (owner.key.clone(), source.key.clone()));
        let selected = source.selection.selections[source.selection.primary];
        let has_unchanged_highlights = self.entries.get(&context.viewport).is_some_and(|entry| {
            entry.request.describes(store)
                && entry.request.project == context.project
                && entry.request.owner == context.owner
                && entry.request.source == context.source
                && entry.providers == providers
                && !entry.highlights.is_empty()
        });
        if providers.is_empty()
            || source.composition.is_some()
            || (!has_word_selection(&snapshot, &source.selection) && !has_unchanged_highlights)
        {
            self.close(context.viewport);
            return Ok(None);
        }
        let changed = self.entries.get_mut(&context.viewport).is_none_or(|entry| {
            !entry.refresh_selection(store)
                || entry.request.project != context.project
                || entry.request.source != context.source
                || entry.request.owner != context.owner
                || entry.providers != providers
        });
        if changed {
            self.close(context.viewport);
            let (cancel, cancelled) = watch::channel(false);
            self.entries.insert(
                context.viewport,
                Entry {
                    request: Request {
                        project: context.project,
                        snapshot: snapshot.clone(),
                        source: context.source,
                        source_key: source.key.clone(),
                        owner: context.owner,
                        owner_key: owner.key.clone(),
                        owner_document: owner.document,
                        selection: source.selection.clone(),
                        position: byte_to_position(&snapshot, selected.head)?,
                        viewport: context.viewport,
                        token: Uuid::new_v4(),
                        cancelled,
                    },
                    providers,
                    cancel,
                    due: now + REQUEST_DEBOUNCE,
                    submitted: false,
                    render_after: now,
                    highlights: Vec::new(),
                },
            );
        }
        let entry = self
            .entries
            .get_mut(&context.viewport)
            .ok_or(EditorError::NotFound)?;
        if entry.submitted || now < entry.due {
            return Ok(None);
        }
        entry.submitted = true;
        Ok(Some(entry.request.clone()))
    }

    fn trigger(
        &mut self,
        store: &EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
        now: Instant,
    ) -> Result<Option<Request>, EditorError> {
        if self
            .entries
            .get_mut(&context.viewport)
            .is_some_and(|entry| {
                entry.refresh_selection(store)
                    && entry.request.project == context.project
                    && entry.request.owner == context.owner
                    && entry.request.source == context.source
                    && entry.providers == providers
                    && !entry.highlights.is_empty()
            })
        {
            return Ok(None);
        }
        let viewport = context.viewport;
        self.close(viewport);
        self.observe(store, context, providers, now)?;
        let Some(entry) = self.entries.get_mut(&viewport) else {
            return Ok(None);
        };
        entry.submitted = true;
        entry.due = now;
        entry.render_after = now + TRIGGER_DELAY;
        Ok(Some(entry.request.clone()))
    }

    pub(crate) fn source_for_owner(
        &self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        owner: ViewId,
    ) -> ViewId {
        let Some(target) = store.views().get(owner) else {
            return owner;
        };
        self.sources
            .get(&viewport)
            .filter(|(key, _)| key == &target.key)
            .and_then(|(_, source)| store.views().find(source))
            .unwrap_or(owner)
    }

    #[cfg(test)]
    pub(crate) fn rendering_deadline(&self, viewport: egui::ViewportId) -> Option<Instant> {
        self.entries.get(&viewport).map(|entry| entry.render_after)
    }

    pub(crate) fn has_highlights(
        &self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        view: ViewId,
    ) -> bool {
        self.entries.get(&viewport).is_some_and(|entry| {
            entry.request.describes(store)
                && !entry.highlights.is_empty()
                && Instant::now() >= entry.render_after
                && store.views().get(view).is_some_and(|target| {
                    target.document == entry.request.snapshot.id && target.composition.is_none()
                })
        })
    }

    fn navigate(
        &mut self,
        store: &mut EditorStore,
        context: Context,
        command: HighlightCommand,
    ) -> Result<bool, EditorError> {
        if !self.has_highlights(store, context.viewport, context.source) {
            return Ok(false);
        }
        let Some(entry) = self.entries.get_mut(&context.viewport).filter(|entry| {
            entry.request.owner == context.owner && entry.request.project == context.project
        }) else {
            return Ok(false);
        };
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?
            .clone();
        let caret = source.selection.selections[source.selection.primary].head;
        let mut ranges = entry
            .highlights
            .iter()
            .map(|highlight| highlight.bytes.clone())
            .collect::<Vec<_>>();
        ranges.sort_by_key(|bytes| (bytes.start, bytes.end));
        ranges.dedup();
        let index = ranges
            .iter()
            .position(|bytes| bytes.start <= caret && caret <= bytes.end);
        let target = match command {
            HighlightCommand::Next => index.map_or(0, |index| (index + 1) % ranges.len()),
            HighlightCommand::Previous => index.map_or(ranges.len() - 1, |index| {
                (index + ranges.len() - 1) % ranges.len()
            }),
            HighlightCommand::Trigger => return Ok(false),
        };
        let bytes = ranges[target].clone();
        let selection = SelectionSet {
            primary: 0,
            selections: vec![taide_native_editor::view::Selection {
                anchor: bytes.start,
                head: bytes.start,
            }],
        };
        store.set_view_state(
            context.source,
            selection.clone(),
            source.scroll,
            source.folds,
        )?;
        store.request_selection_reveal(context.source, bytes.clone(), true)?;
        entry.request.source = context.source;
        entry.request.source_key = source.key;
        entry.request.selection = selection;
        entry.request.position = byte_to_position(&entry.request.snapshot, bytes.start)?;
        Ok(true)
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        result: Result<Response, Failure>,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&request.viewport).filter(|entry| {
            entry.request.token == request.token
                && request.describes(store)
                && entry.providers == providers
        }) else {
            return false;
        };
        entry.highlights.clear();
        if let Ok(response) = result
            && response
                .provider
                .is_none_or(|provider| providers.contains(&provider))
        {
            entry.highlights = response
                .highlights
                .into_iter()
                .filter_map(|highlight| {
                    let bytes = range_to_bytes(
                        &request.snapshot,
                        LspRange {
                            start: Position {
                                line: highlight.range.start.line,
                                character: highlight.range.start.character,
                            },
                            end: Position {
                                line: highlight.range.end.line,
                                character: highlight.range.end.character,
                            },
                        },
                    )
                    .ok()?;
                    if bytes.is_empty() {
                        return None;
                    }
                    let kind = match highlight
                        .kind
                        .unwrap_or(lsp_types::DocumentHighlightKind::TEXT)
                    {
                        lsp_types::DocumentHighlightKind::TEXT => 0,
                        lsp_types::DocumentHighlightKind::WRITE => WRITE_KIND,
                        _ => 1,
                    };
                    Some(Highlight { bytes, kind })
                })
                .collect();
        }
        true
    }

    pub(crate) fn close(&mut self, viewport: egui::ViewportId) {
        if let Some(entry) = self.entries.remove(&viewport) {
            entry.cancel.send_replace(true);
        }
    }

    pub(crate) fn reject(&mut self, request: &Request) {
        if self
            .entries
            .get(&request.viewport)
            .is_some_and(|entry| entry.request.token == request.token)
        {
            self.close(request.viewport);
        }
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        active: impl Fn(&Request) -> bool,
        providers: impl Fn(&ProjectId, &DocumentSnapshot) -> HashSet<ProviderIdentity>,
    ) {
        self.entries.retain(|_, entry| {
            let keep = entry.refresh_selection(store)
                && active(&entry.request)
                && entry.providers == providers(&entry.request.project, &entry.request.snapshot);
            if !keep {
                entry.cancel.send_replace(true);
            }
            keep
        });
    }

    pub(crate) fn display(
        &self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        view: ViewId,
        colors: Colors,
    ) -> Option<Display> {
        let entry = self.entries.get(&viewport)?;
        let target = store.views().get(view)?;
        if !entry.request.describes(store)
            || target.document != entry.request.snapshot.id
            || Instant::now() < entry.render_after
        {
            return None;
        }
        let mut borders = Vec::new();
        let items = entry
            .highlights
            .iter()
            .flat_map(|highlight| {
                let kind = highlight.kind;
                if colors.border[kind] != Color32::TRANSPARENT {
                    borders.push((highlight.bytes.clone(), colors.border[kind]));
                }
                [
                    Decoration {
                        bytes: highlight.bytes.clone(),
                        kind: DecorationKind::Inline(InlineStyle {
                            background: Some(colors.background[kind].to_srgba_unmultiplied()),
                            ..Default::default()
                        }),
                        stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                    },
                    Decoration {
                        bytes: highlight.bytes.clone(),
                        kind: DecorationKind::Overview {
                            lane: OverviewLane::Center,
                            color: colors.overview[kind].to_srgba_unmultiplied(),
                            minimap: Some(colors.minimap.to_srgba_unmultiplied()),
                        },
                        stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                    },
                ]
            })
            .collect();
        Some(Display {
            layer: DecorationLayer::new(entry.request.snapshot.revision, 0, items),
            borders,
        })
    }
}

#[derive(Clone)]
pub(crate) struct Consumer<'a, 'state> {
    pub state: Rc<RefCell<&'state mut State>>,
    pub lsp: Option<&'a crate::lsp::LspBridge>,
    pub context: egui::Context,
    pub colors: Colors,
}

impl Consumer<'_, '_> {
    fn providers(
        &self,
        store: &EditorStore,
        project: &ProjectId,
        source: ViewId,
    ) -> HashSet<ProviderIdentity> {
        store
            .views()
            .get(source)
            .and_then(|view| store.documents().snapshot(view.document).ok())
            .map_or_else(HashSet::new, |snapshot| {
                self.lsp.map_or_else(HashSet::new, |lsp| {
                    lsp.highlight_providers(project, &snapshot)
                })
            })
    }

    pub(crate) fn execute(
        &self,
        store: &mut EditorStore,
        project: Option<&ProjectId>,
        owner: ViewId,
        source: ViewId,
        command: HighlightCommand,
    ) -> Result<bool, EditorError> {
        let Some(project) = project else {
            return Ok(false);
        };
        let context = Context {
            project: project.clone(),
            owner,
            source,
            viewport: self.context.viewport_id(),
        };
        if command != HighlightCommand::Trigger {
            let changed = self.state.borrow_mut().navigate(store, context, command)?;
            if changed {
                self.context.request_repaint();
            }
            return Ok(changed);
        }
        let request = self.state.borrow_mut().trigger(
            store,
            context,
            self.providers(store, project, source),
            Instant::now(),
        )?;
        if let Some(request) = request
            && self.lsp.is_none_or(|lsp| lsp.highlights(request).is_err())
        {
            self.state.borrow_mut().close(self.context.viewport_id());
            return Ok(false);
        }
        self.context.request_repaint_after(TRIGGER_DELAY);
        Ok(true)
    }

    pub(crate) fn observe(
        &self,
        store: &EditorStore,
        project: Option<&ProjectId>,
        owner: ViewId,
        source: ViewId,
    ) {
        let Some(project) = project else {
            self.state.borrow_mut().close(self.context.viewport_id());
            return;
        };
        let providers = self.providers(store, project, source);
        let now = Instant::now();
        let before = self
            .state
            .borrow()
            .entries
            .get(&self.context.viewport_id())
            .map(|entry| entry.request.token);
        let request = self.state.borrow_mut().observe(
            store,
            Context {
                project: project.clone(),
                owner,
                source,
                viewport: self.context.viewport_id(),
            },
            providers,
            now,
        );
        match request {
            Ok(Some(request)) => {
                if self.lsp.is_none_or(|lsp| lsp.highlights(request).is_err()) {
                    self.state.borrow_mut().close(self.context.viewport_id());
                }
            }
            Err(_) => self.state.borrow_mut().close(self.context.viewport_id()),
            _ => {}
        }
        let state = self.state.borrow();
        if before
            != state
                .entries
                .get(&self.context.viewport_id())
                .map(|entry| entry.request.token)
        {
            self.context.request_repaint();
        }
        if let Some(entry) = state.entries.get(&self.context.viewport_id())
            && !entry.submitted
        {
            self.context
                .request_repaint_after(entry.due.saturating_duration_since(now));
        }
        if let Some(entry) = state.entries.get(&self.context.viewport_id())
            && !entry.highlights.is_empty()
            && now < entry.render_after
        {
            self.context
                .request_repaint_after(entry.render_after.saturating_duration_since(now));
        }
    }

    pub(crate) fn display(&self, store: &EditorStore, view: ViewId) -> Option<Display> {
        self.state
            .borrow()
            .display(store, self.context.viewport_id(), view, self.colors)
    }
}

#[cfg(test)]
#[path = "editor-highlights-tests.rs"]
mod tests;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::rc::Rc;
use std::time::{Duration, Instant};

use eframe::egui;
use taide_lsp::native::Failure;
use taide_lsp::native::protocol::lsp_types;
use taide_model::error::AppResult;
use taide_model::file::FileSizeTier;
use taide_model::ids::ProjectId;
use taide_model::theme::{ResolvedTheme, ThemeType};
use taide_native_editor::document::{DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::inlay_hints::{Hint, Kind};
use taide_native_editor::lsp::{LspRange, Position, byte_to_position, position_to_byte};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_inlay_hints::{Colors, Presentation, Style};
use tokio::sync::watch;
use uuid::Uuid;

use crate::editor_symbols::ProviderIdentity;

const REQUEST_DEBOUNCE: Duration = Duration::from_millis(25);
const RANGE_MARGIN: usize = 30;
const FOREGROUND: egui::Color32 = egui::Color32::from_rgb(0x96, 0x96, 0x96);
const BADGE_DARK: egui::Color32 = egui::Color32::from_rgb(0x4d, 0x4d, 0x4d);
const BADGE_LIGHT: egui::Color32 = egui::Color32::from_rgb(0xc4, 0xc4, 0xc4);
const BACKGROUND_OPACITY: f32 = 0.1;

pub(crate) fn colors(theme: &ResolvedTheme) -> AppResult<Colors> {
    let value = |key: &str, fallback| {
        theme.colors.get(key).map_or(Ok(fallback), |value| {
            taide_native_ui::presentation::parse_color(value, key)
        })
    };
    let default = Style {
        foreground: value("editorInlayHint.foreground", FOREGROUND)?,
        background: value(
            "editorInlayHint.background",
            value(
                "badge.background",
                if theme.theme_type == ThemeType::Dark {
                    BADGE_DARK
                } else {
                    BADGE_LIGHT
                },
            )?
            .gamma_multiply(BACKGROUND_OPACITY),
        )?,
    };
    Ok(Colors {
        default,
        type_hint: Style {
            foreground: value("editorInlayHint.typeForeground", default.foreground)?,
            background: value("editorInlayHint.typeBackground", default.background)?,
        },
        parameter: Style {
            foreground: value("editorInlayHint.parameterForeground", default.foreground)?,
            background: value("editorInlayHint.parameterBackground", default.background)?,
        },
    })
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
    pub(crate) viewport: egui::ViewportId,
    pub(crate) range: LspRange,
    pub(crate) token: Uuid,
    pub(crate) cancelled: watch::Receiver<bool>,
}

impl Request {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }

    pub(crate) fn describes(&self, store: &EditorStore) -> bool {
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
                        .is_ok_and(|current| {
                            current.key == self.snapshot.key
                                && current.revision == self.snapshot.revision
                                && current.metadata.language_id
                                    == self.snapshot.metadata.language_id
                                && current.metadata.tier == FileSizeTier::Normal
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
    pub(crate) groups: Vec<(ProviderIdentity, Vec<lsp_types::InlayHint>)>,
}

struct Entry {
    request: Request,
    providers: HashSet<ProviderIdentity>,
    cancel: watch::Sender<bool>,
    due: Instant,
    submitted: bool,
    items: std::sync::Arc<[Hint]>,
}

#[derive(Default)]
pub(crate) struct State {
    entries: HashMap<(egui::ViewportId, ViewId), Entry>,
    seen: HashSet<(egui::ViewportId, ViewId)>,
}

pub(crate) struct Context {
    pub project: ProjectId,
    pub owner: ViewId,
    pub source: ViewId,
    pub viewport: egui::ViewportId,
    pub lines: Range<usize>,
}

fn requested_range(
    snapshot: &DocumentSnapshot,
    lines: Range<usize>,
) -> Result<LspRange, EditorError> {
    let start = lines
        .start
        .saturating_sub(RANGE_MARGIN)
        .min(snapshot.rope.len_lines().saturating_sub(1));
    let end = lines
        .end
        .saturating_add(RANGE_MARGIN)
        .min(snapshot.rope.len_lines())
        .saturating_sub(1);
    Ok(LspRange {
        start: Position {
            line: u32::try_from(start).map_err(|_| EditorError::Capacity)?,
            character: 0,
        },
        end: byte_to_position(snapshot, line_content_range(snapshot, end).end)?,
    })
}

fn convert(
    snapshot: &DocumentSnapshot,
    range: LspRange,
    hint: lsp_types::InlayHint,
) -> Option<Hint> {
    let position = hint.position;
    if position < range.start
        || position > range.end
        || position.line as usize >= snapshot.rope.len_lines()
    {
        return None;
    }
    let byte = position_to_byte(
        snapshot,
        Position {
            line: position.line,
            character: position.character,
        },
    )
    .ok()?;
    if byte_to_position(snapshot, byte).ok()? != position {
        return None;
    }
    let label = match hint.label {
        lsp_types::InlayHintLabel::String(label) => label,
        lsp_types::InlayHintLabel::LabelParts(parts) => {
            parts.into_iter().map(|part| part.value).collect()
        }
    };
    if label.is_empty() || label.contains(['\r', '\n']) {
        return None;
    }
    Some(Hint {
        byte,
        label,
        kind: match hint.kind {
            Some(lsp_types::InlayHintKind::TYPE) => Some(Kind::Type),
            Some(lsp_types::InlayHintKind::PARAMETER) => Some(Kind::Parameter),
            _ => None,
        },
        tooltip: hint.tooltip.map(|tooltip| match tooltip {
            lsp_types::InlayHintTooltip::String(text) => text,
            lsp_types::InlayHintTooltip::MarkupContent(content) => content.value,
        }),
        padding_left: hint.padding_left.unwrap_or(false),
        padding_right: hint.padding_right.unwrap_or(false),
    })
}

impl State {
    #[cfg(test)]
    pub(crate) fn request(&self, viewport: egui::ViewportId, view: ViewId) -> Option<&Request> {
        self.entries
            .get(&(viewport, view))
            .map(|entry| &entry.request)
    }
    pub(crate) fn observe(
        &mut self,
        store: &EditorStore,
        context: Context,
        providers: HashSet<ProviderIdentity>,
        now: Instant,
    ) -> Result<Option<Request>, EditorError> {
        let key = (context.viewport, context.source);
        let source = store
            .views()
            .get(context.source)
            .ok_or(EditorError::NotFound)?;
        let owner = store
            .views()
            .get(context.owner)
            .ok_or(EditorError::NotFound)?;
        let snapshot = store.documents().snapshot(source.document)?;
        if context.lines.is_empty()
            || source.composition.is_some()
            || snapshot.metadata.tier != FileSizeTier::Normal
            || !matches!(snapshot.key, DocumentKey::File(_))
            || providers.is_empty()
        {
            self.remove(key);
            return Ok(None);
        }
        let range = requested_range(&snapshot, context.lines)?;
        let unchanged = self.entries.get(&key).is_some_and(|entry| {
            entry.request.describes(store)
                && entry.request.project == context.project
                && entry.request.owner == context.owner
                && entry.request.range == range
                && entry.providers == providers
        });
        if !unchanged {
            let items = self
                .entries
                .get(&key)
                .filter(|entry| {
                    entry.request.describes(store)
                        && entry.request.project == context.project
                        && entry.request.owner == context.owner
                        && entry.providers == providers
                })
                .map_or_else(|| std::sync::Arc::from([]), |entry| entry.items.clone());
            self.remove(key);
            let due = if self.seen.insert(key) {
                now
            } else {
                now + REQUEST_DEBOUNCE
            };
            let (cancel, cancelled) = watch::channel(false);
            self.entries.insert(
                key,
                Entry {
                    request: Request {
                        project: context.project,
                        snapshot,
                        source: context.source,
                        source_key: source.key.clone(),
                        owner: context.owner,
                        owner_key: owner.key.clone(),
                        owner_document: owner.document,
                        viewport: context.viewport,
                        range,
                        token: Uuid::new_v4(),
                        cancelled,
                    },
                    providers,
                    cancel,
                    due,
                    submitted: false,
                    items,
                },
            );
        }
        let entry = self.entries.get_mut(&key).ok_or(EditorError::NotFound)?;
        if entry.submitted || now < entry.due {
            return Ok(None);
        }
        entry.submitted = true;
        Ok(Some(entry.request.clone()))
    }

    fn remove(&mut self, key: (egui::ViewportId, ViewId)) {
        if let Some(entry) = self.entries.remove(&key) {
            entry.cancel.send_replace(true);
        }
    }

    pub(crate) fn reject(&mut self, request: &Request) {
        let key = (request.viewport, request.source);
        if self
            .entries
            .get(&key)
            .is_some_and(|entry| entry.request.token == request.token)
        {
            self.remove(key);
        }
    }

    pub(crate) fn accept(
        &mut self,
        store: &EditorStore,
        request: &Request,
        providers: HashSet<ProviderIdentity>,
        response: Result<Response, Failure>,
    ) -> bool {
        let Some(entry) = self.entries.get_mut(&(request.viewport, request.source)) else {
            return false;
        };
        if entry.request.token != request.token
            || !request.describes(store)
            || entry.providers != providers
        {
            return false;
        }
        let mut items = entry
            .items
            .iter()
            .filter(|hint| {
                byte_to_position(&request.snapshot, hint.byte).is_ok_and(|position| {
                    position < request.range.start || position > request.range.end
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        items.extend(
            response
                .ok()
                .into_iter()
                .flat_map(|response| response.groups)
                .filter(|(provider, _)| providers.contains(provider))
                .flat_map(|(_, hints)| {
                    hints
                        .into_iter()
                        .filter_map(|hint| convert(&request.snapshot, request.range, hint))
                })
                .collect::<Vec<_>>(),
        );
        items.sort_by_key(|item| item.byte);
        let mut seen = HashSet::new();
        items.retain(|hint| seen.insert(hint.clone()));
        entry.items = items.into();
        true
    }

    pub(crate) fn display(
        &self,
        store: &EditorStore,
        viewport: egui::ViewportId,
        view: ViewId,
        colors: Colors,
    ) -> Option<Presentation> {
        let entry = self.entries.get(&(viewport, view))?;
        entry.request.describes(store).then(|| Presentation {
            document: entry.request.snapshot.id,
            revision: entry.request.snapshot.revision,
            items: entry.items.clone(),
            colors,
        })
    }

    pub(crate) fn reconcile(
        &mut self,
        store: &EditorStore,
        active: impl Fn(&Request) -> bool,
        providers: impl Fn(&ProjectId, &DocumentSnapshot) -> HashSet<ProviderIdentity>,
    ) {
        self.entries.retain(|_, entry| {
            let keep = entry.request.describes(store)
                && active(&entry.request)
                && entry.providers == providers(&entry.request.project, &entry.request.snapshot);
            if !keep {
                entry.cancel.send_replace(true);
            }
            keep
        });
        self.seen
            .retain(|(_, view)| store.views().get(*view).is_some());
    }

    pub(crate) fn clear(&mut self) {
        for entry in self.entries.values() {
            entry.cancel.send_replace(true);
        }
        self.entries.clear();
        self.seen.clear();
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
    pub(crate) fn display(&self, store: &EditorStore, view: ViewId) -> Option<Presentation> {
        self.state
            .borrow()
            .display(store, self.context.viewport_id(), view, self.colors)
    }

    pub(crate) fn observe(
        &self,
        store: &EditorStore,
        project: Option<&ProjectId>,
        owner: ViewId,
        source: ViewId,
        lines: Range<usize>,
    ) {
        let key = (self.context.viewport_id(), source);
        let Some(project) = project else {
            self.state.borrow_mut().remove(key);
            return;
        };
        let providers = store
            .views()
            .get(source)
            .and_then(|view| store.documents().snapshot(view.document).ok())
            .zip(self.lsp)
            .map_or_else(HashSet::new, |(snapshot, lsp)| {
                lsp.inlay_hint_providers(project, &snapshot)
            });
        let now = Instant::now();
        let result = self.state.borrow_mut().observe(
            store,
            Context {
                project: project.clone(),
                owner,
                source,
                viewport: self.context.viewport_id(),
                lines,
            },
            providers.clone(),
            now,
        );
        match result {
            Ok(Some(request)) => {
                if self
                    .lsp
                    .is_none_or(|lsp| lsp.inlay_hints(request.clone()).is_err())
                {
                    self.state.borrow_mut().accept(
                        store,
                        &request,
                        providers,
                        Err(Failure::TransportClosed),
                    );
                }
                self.context.request_repaint();
            }
            Err(_) => self.state.borrow_mut().remove(key),
            _ => {}
        }
        if let Some(entry) = self.state.borrow().entries.get(&key)
            && !entry.submitted
        {
            self.context
                .request_repaint_after(entry.due.saturating_duration_since(now));
        }
    }
}

#[cfg(test)]
#[path = "editor-inlay-hints-tests.rs"]
mod tests;

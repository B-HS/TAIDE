use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use eframe::egui::{self, Color32, Event, Key, Ui};
use taide_lsp::native::protocol::lsp_types;
use taide_model::ids::ProjectId;
use taide_model::snippet::SnippetFile;
use taide_native_editor::completion::{CompletionItemKind, PreparationOptions};
use taide_native_editor::document::EditorError;
use taide_native_editor::documentation::{ImageDimensions, RichDocument};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::language_configuration::{LineSyntax, token_kind_at};
use taide_native_editor::snippet_session::Session;
use taide_native_editor::snippet_syntax::ParseLimits;
use taide_native_editor::store::EditorStore;
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_syntax::MonacoSnippetTransforms;
use taide_native_ui::editor_completion::{Trigger, Widget};
use taide_native_ui::editor_surface::NativeEditor;
use taide_runtime::AppServices;
use uuid::Uuid;

use super::{Context, PreparationContext, Request, SnippetClock, State, colors, supply};

const SNIPPET_NESTING_LIMIT: usize = 128;
const SNIPPET_MARKER_LIMIT: usize = 65_536;
const CLASS_DARK: Color32 = Color32::from_rgb(0xee, 0x9d, 0x28);
const CLASS_LIGHT: Color32 = Color32::from_rgb(0xd6, 0x7e, 0x00);
const FUNCTION_DARK: Color32 = Color32::from_rgb(0xb1, 0x80, 0xd7);
const FUNCTION_LIGHT: Color32 = Color32::from_rgb(0x65, 0x2d, 0x90);
const FIELD_DARK: Color32 = Color32::from_rgb(0x75, 0xbe, 0xff);
const FIELD_LIGHT: Color32 = Color32::from_rgb(0x00, 0x7a, 0xcc);

#[derive(Clone)]
pub(crate) struct Consumer<'a, 'state> {
    pub state: Rc<RefCell<&'state mut State>>,
    pub services: &'a AppServices,
    pub files: Arc<[SnippetFile]>,
    pub context: egui::Context,
}

pub(crate) struct Provider<'a, 'state> {
    pub consumer: Consumer<'a, 'state>,
    pub project: Option<ProjectId>,
    pub owner: ViewId,
    pub owner_key: ViewKey,
    pub lsp: Option<&'a crate::lsp::LspBridge>,
    pub viewport: egui::ViewportId,
    pub commands: &'a mut Vec<crate::host::HostCommand>,
    pub editor: &'a NativeEditor,
    pub syntax: &'a dyn LineSyntax,
    pub model_path: String,
    pub language: String,
}

pub(super) struct Snippet {
    pub origin: Request,
    pub session: Session,
    pub transforms: MonacoSnippetTransforms,
    shown_choice: Option<taide_native_editor::snippet_syntax::Index>,
}

pub(super) struct Preview {
    candidate: taide_native_editor::completion::Candidate,
    selection: taide_native_editor::view::SelectionSet,
    indent: taide_native_editor::indent::IndentOptions,
    alternate: bool,
    ghosts: Vec<taide_native_editor::completion_preview::GhostText>,
    styles: std::collections::HashMap<usize, Arc<taide_native_editor::line_tokens::PreviewTokens>>,
    text: std::collections::BTreeMap<usize, Arc<[String]>>,
}

impl Consumer<'_, '_> {
    pub(crate) fn defers(&self, view: ViewId, event: &Event) -> bool {
        let state = self.state.borrow();
        taide_native_ui::editor_completion::shortcut(
            event,
            self.context.os().is_mac(),
            state.entries.contains_key(&view),
        )
        .is_some()
            || state.snippet_sessions.contains_key(&view)
                && matches!(event, Event::Key { key: Key::Tab | Key::Escape, pressed: true, modifiers, .. } if !modifiers.ctrl && !modifiers.alt && !modifiers.mac_cmd && !modifiers.command)
    }

    pub(crate) fn queue_command(
        &self,
        view: ViewId,
        command: taide_native_editor::completion::Command,
    ) {
        self.state
            .borrow_mut()
            .commands
            .entry(view)
            .or_default()
            .push(command);
        self.context.request_repaint();
    }
}

impl State {
    pub(crate) fn prepare_code(
        &mut self,
        store: &mut EditorStore,
        syntax: &mut crate::editor_syntax::EditorSyntax,
        plugins: &[taide_model::plugin::LoadedPlugin],
    ) -> Result<(), EditorError> {
        let mut active = Vec::new();
        for entry in self
            .entries
            .values_mut()
            .filter(|entry| entry.request.describes(store))
        {
            let Some(preview) = entry.preview.as_mut() else {
                continue;
            };
            preview.styles.clear();
            let document = &entry.request.snapshot;
            if preview.text.is_empty() {
                let mut grouped = std::collections::BTreeMap::<
                    usize,
                    Vec<taide_native_editor::completion_preview_view::ViewData>,
                >::new();
                for ghost in &preview.ghosts {
                    if let Ok(view) =
                        taide_native_editor::completion_preview_view::ViewData::new(document, ghost)
                    {
                        grouped.entry(view.line).or_default().push(view);
                    }
                }
                for (line, views) in grouped {
                    let source = line_content_range(document, line);
                    let mut first = String::new();
                    let mut previous = source.start;
                    let mut inline = views
                        .iter()
                        .flat_map(|view| &view.inline)
                        .collect::<Vec<_>>();
                    inline.sort_by_key(|part| part.byte);
                    for part in inline {
                        first.extend(document.rope.byte_slice(previous..part.byte).chunks());
                        first.push_str(&part.text);
                        previous = part.byte;
                    }
                    first.extend(document.rope.byte_slice(previous..source.end).chunks());
                    let lines =
                        std::iter::once(first)
                            .chain(views.iter().flat_map(|view| {
                                view.additional.iter().map(|line| line.text.clone())
                            }))
                            .collect::<Arc<[String]>>();
                    preview.text.insert(line, lines);
                }
            }
            for (line, lines) in &preview.text {
                active.push((document.id, *line));
                if let Some(tokens) = syntax.preview_tokens(store, document.id, *line, lines) {
                    preview.styles.insert(*line, tokens);
                }
            }
        }
        syntax.retain_previews(&active);
        let documents = self
            .entries
            .values()
            .filter(|entry| entry.request.describes(store))
            .flat_map(|entry| {
                entry.documents.values().map(|document| {
                    (
                        document.as_ref(),
                        entry.request.snapshot.metadata.language_id.as_str(),
                    )
                })
            })
            .collect::<Vec<_>>();
        self.code
            .prepare(store, syntax, documents.into_iter(), plugins)
    }

    pub(crate) fn prepare_images(
        &mut self,
        store: &EditorStore,
        context: &egui::Context,
        services: &AppServices,
    ) {
        let documents = self
            .entries
            .values()
            .filter(|entry| entry.request.describes(store))
            .flat_map(|entry| {
                entry
                    .documents
                    .values()
                    .map(|document| (&entry.request.project, document.as_ref()))
            })
            .collect::<Vec<_>>();
        self.images
            .prepare(context, services, documents.into_iter());
    }
}

impl Provider<'_, '_> {
    fn choice(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        force: bool,
    ) -> Result<bool, EditorError> {
        if !taide_native_ui::editor_completion::Provider::available(self, store, view) {
            return Ok(false);
        }
        let mut state = self.consumer.state.borrow_mut();
        let Some(snippet) = state.snippet_sessions.get_mut(&view) else {
            return Ok(false);
        };
        if !snippet.session.synchronize(store) {
            state.snippet_sessions.remove(&view);
            return Ok(false);
        }
        let Some(choice) = snippet.session.active_choice(store)? else {
            snippet.shown_choice = None;
            return Ok(false);
        };
        if !force && snippet.shown_choice == Some(choice.index) {
            return Ok(false);
        }
        let index = choice.index;
        let bytes = choice.bytes.clone();
        let options = choice.options.to_vec();
        snippet.shown_choice = Some(index);
        let origin = snippet.origin.clone();
        let document = store.documents().snapshot(origin.snapshot.id)?;
        let word = document.rope.byte_slice(bytes.clone()).to_string();
        let matches_word = options.iter().any(|value| value == &word);
        let width = options.len().to_string().len();
        let range = taide_native_editor::lsp::LspRange::new(
            taide_native_editor::lsp::byte_to_position(&document, bytes.start)?,
            taide_native_editor::lsp::byte_to_position(&document, bytes.end)?,
        );
        let items = options
            .into_iter()
            .enumerate()
            .map(|(index, value)| lsp_types::CompletionItem {
                label: value.clone(),
                kind: Some(CompletionItemKind::VALUE),
                insert_text: Some(value.clone()),
                text_edit: Some(lsp_types::CompletionTextEdit::Edit(lsp_types::TextEdit {
                    range,
                    new_text: value.clone(),
                })),
                sort_text: Some(format!("{index:0width$}")),
                filter_text: matches_word.then(|| format!("{word}_{value}")),
                ..Default::default()
            })
            .collect();
        let request = state.begin(
            store,
            Context {
                project: origin.project,
                source: view,
                owner: origin.owner,
                word: bytes,
                viewport: origin.viewport,
                automatic: false,
            },
            HashSet::new(),
        )?;
        let entry = state.entries.get_mut(&view).ok_or(EditorError::NotFound)?;
        entry.snippets = taide_native_editor::completion::Candidates::new(
            &request.snapshot,
            request.position,
            request.word,
            Some(lsp_types::CompletionResponse::Array(items)),
        )
        .items;
        entry.supplied = true;
        entry.choice = Some(index);
        self.consumer.context.request_repaint();
        Ok(true)
    }

    fn supply(&mut self, store: &EditorStore, view: ViewId) {
        let mut state = self.consumer.state.borrow_mut();
        let Some(request) = state.request(store, view).cloned() else {
            return;
        };
        let services = &self.consumer.services;
        let repaint = self.consumer.context.clone();
        state.supply(
            store,
            &request,
            &self.consumer.files,
            &services.tasks,
            Arc::new(move || repaint.request_repaint()),
        );
    }
}

impl taide_native_ui::editor_completion::Provider for Provider<'_, '_> {
    fn is_embedded(&self, view: ViewId) -> bool {
        view != self.owner
    }
    fn available(&self, store: &EditorStore, view: ViewId) -> bool {
        self.project.is_some()
            && store
                .views()
                .get(self.owner)
                .is_some_and(|owner| owner.key == self.owner_key)
            && store.views().get(view).is_some_and(|source| {
                source.composition.is_none()
                    && store
                        .documents()
                        .snapshot(source.document)
                        .is_ok_and(|document| !document.metadata.read_only)
            })
    }

    fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        trigger: Trigger,
    ) -> Result<bool, EditorError> {
        if !self.available(store, view)
            || (trigger == Trigger::Automatic && !self.should_auto_trigger(store, view))
        {
            return Ok(false);
        }
        if trigger == Trigger::Manual && self.choice(store, view, true)? {
            return Ok(true);
        }
        let project = self.project.clone().ok_or(EditorError::InvalidIdentity)?;
        let source = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(source.document)?;
        let byte = source.selection.selections[source.selection.primary].head;
        let providers = self.lsp.map_or_else(HashSet::new, |lsp| {
            lsp.completion_providers(&project, &document)
        });
        let request = self.consumer.state.borrow_mut().begin(
            store,
            Context {
                project,
                source: view,
                owner: self.owner,
                word: supply::word_at(&document, byte)?,
                viewport: self.viewport,
                automatic: trigger != Trigger::Manual,
            },
            providers,
        )?;
        self.supply(store, view);
        self.consumer.state.borrow_mut().queue(request.source);
        Ok(true)
    }

    fn current(&self, store: &EditorStore, view: ViewId) -> Option<Widget> {
        let mut state = self.consumer.state.borrow_mut();
        if !state.refresh_view(store, view).ok()? {
            return None;
        }
        let model = state.model(store, view).ok()?;
        let request = state.request(store, view)?;
        let byte = store.views().get(view)?.selection.selections[request.selection.primary].head;
        let automatic = request.automatic;
        let pending = model.is_none() && state.pending(store, view);
        let (token, leading, delta) = state.display(store, view)?;
        Some(Widget {
            token: token.to_string(),
            byte,
            automatic,
            pending,
            model,
            leading: leading.into(),
            delta,
        })
    }

    fn close(&mut self, view: ViewId) {
        self.consumer.state.borrow_mut().close(view);
    }

    fn preview(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
        alternate: bool,
    ) -> Vec<taide_native_editor::completion_preview::GhostText> {
        let Some(source) = store
            .views()
            .get(view)
            .filter(|view| view.composition.is_none())
        else {
            return Vec::new();
        };
        let Ok(document) = store.documents().snapshot(source.document) else {
            return Vec::new();
        };
        let mut state = self.consumer.state.borrow_mut();
        let Some(entry) = state.entries.get_mut(&view).filter(|entry| {
            entry.widget_token.to_string() == token
                && entry.choice.is_none()
                && entry.request.describes(store)
        }) else {
            return Vec::new();
        };
        let Some(candidate) = entry
            .model
            .as_ref()
            .and_then(|model| model.borrow().candidate(candidate).cloned())
        else {
            return Vec::new();
        };
        let indent = self.editor.indent_options(&document);
        if let Some(preview) = &entry.preview
            && preview.candidate == candidate
            && preview.selection == source.selection
            && preview.indent == indent
            && preview.alternate == alternate
        {
            return preview.ghosts.clone();
        }
        let primary = source.selection.selections[source.selection.primary].head;
        let limits = ParseLimits {
            max_bytes: taide_model::file::REFUSED_FILE_BYTES as usize,
            max_nesting: SNIPPET_NESTING_LIMIT,
            max_markers: SNIPPET_MARKER_LIMIT,
        };
        let mut transforms = MonacoSnippetTransforms::new(limits);
        let Ok(text) =
            candidate.preview_text(&document, primary, indent, limits, |pattern, options| {
                transforms.compile(pattern, options)
            })
        else {
            entry.preview = None;
            return Vec::new();
        };
        let Ok(ranges) = candidate.replacement_ranges(&document, &source.selection, alternate)
        else {
            entry.preview = None;
            return Vec::new();
        };
        let ghosts = ranges
            .into_iter()
            .zip(&source.selection.selections)
            .map(|(range, selection)| {
                taide_native_editor::completion_preview::compute(
                    &document,
                    range,
                    &text,
                    selection.head,
                    taide_native_editor::completion_preview::Options {
                        mode: taide_native_editor::completion_preview::Mode::SubwordSmart,
                        preview_suffix_utf16: 0,
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>();
        let Ok(ghosts) = ghosts else {
            entry.preview = None;
            return Vec::new();
        };
        let ghosts = ghosts
            .into_iter()
            .flatten()
            .filter(|ghost| !ghost.parts.is_empty())
            .collect::<Vec<_>>();
        entry.preview = Some(Preview {
            candidate,
            selection: source.selection.clone(),
            indent,
            alternate,
            ghosts: ghosts.clone(),
            styles: std::collections::HashMap::new(),
            text: std::collections::BTreeMap::new(),
        });
        ghosts
    }

    fn preview_tokens(
        &self,
        view: ViewId,
        line: usize,
    ) -> Option<Arc<taide_native_editor::line_tokens::PreviewTokens>> {
        self.consumer
            .state
            .borrow()
            .entries
            .get(&view)?
            .preview
            .as_ref()?
            .styles
            .get(&line)
            .cloned()
    }

    fn triggers(&self, store: &EditorStore, view: ViewId) -> Vec<String> {
        if self
            .consumer
            .state
            .borrow()
            .entries
            .get(&view)
            .is_some_and(|entry| entry.choice.is_some())
        {
            return Vec::new();
        }
        let Some((project, lsp)) = self.project.as_ref().zip(self.lsp) else {
            return Vec::new();
        };
        let Some(document) = store
            .views()
            .get(view)
            .and_then(|source| store.documents().snapshot(source.document).ok())
        else {
            return Vec::new();
        };
        let mut triggers = lsp
            .completion_options(project, &document)
            .into_values()
            .flatten()
            .flat_map(|options| options.trigger_characters.unwrap_or_default())
            .collect::<Vec<_>>();
        triggers.sort();
        triggers.dedup();
        triggers
    }

    fn should_auto_trigger(&self, store: &EditorStore, view: ViewId) -> bool {
        if !self.available(store, view) {
            return false;
        }
        let Some(source) = store.views().get(view) else {
            return false;
        };
        if source
            .selection
            .selections
            .iter()
            .any(|selection| selection.anchor != selection.head)
        {
            return false;
        }
        let Ok(document) = store.documents().snapshot(source.document) else {
            return false;
        };
        let byte = source.selection.selections[source.selection.primary].head;
        let Ok(word) = supply::word_at(&document, byte) else {
            return false;
        };
        if word.is_empty() || supply::is_number(&document.rope.byte_slice(word.clone()).to_string())
        {
            return false;
        }
        if word.end != byte && document.rope.byte_slice(word.start..byte).len_utf16_cu() != 1 {
            return false;
        }
        self.syntax.follow_edits(store);
        let line = document.rope.byte_to_line(byte);
        let range = line_content_range(&document, line);
        let previous = document
            .rope
            .char_to_byte(document.rope.byte_to_char(byte).saturating_sub(1))
            .max(range.start);
        !self
            .syntax
            .accurate_tokens(&document, line)
            .is_some_and(|tokens| {
                matches!(
                    token_kind_at(&tokens, previous - range.start),
                    TokenKind::Comment | TokenKind::String
                )
            })
    }

    fn after_event(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        _event: &Event,
    ) -> Result<(), EditorError> {
        let mut state = self.consumer.state.borrow_mut();
        if let Some(snippet) = state.snippet_sessions.get_mut(&view)
            && !snippet.session.synchronize(store)
        {
            state.snippet_sessions.remove(&view);
        }
        let _ = state.refresh_view(store, view);
        drop(state);
        self.supply(store, view);
        self.choice(store, view, false).map(|_| ())
    }

    fn take_commands(&mut self, view: ViewId) -> Vec<taide_native_editor::completion::Command> {
        self.consumer
            .state
            .borrow_mut()
            .commands
            .remove(&view)
            .unwrap_or_default()
    }

    fn accept(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
        alternate: bool,
    ) -> Result<bool, EditorError> {
        let token = Uuid::parse_str(token).map_err(|_| EditorError::InvalidIdentity)?;
        let origin = self
            .consumer
            .state
            .borrow()
            .request(store, view)
            .cloned()
            .ok_or(EditorError::StaleRevision)?;
        let choice = {
            let state = self.consumer.state.borrow();
            let entry = state.entries.get(&view).ok_or(EditorError::NotFound)?;
            if entry.widget_token != token {
                return Err(EditorError::StaleRevision);
            }
            if entry.choice.is_some() {
                Some(
                    entry
                        .model
                        .as_ref()
                        .ok_or(EditorError::NotFound)?
                        .borrow()
                        .candidate(candidate)
                        .ok_or(EditorError::NotFound)?
                        .item
                        .label
                        .clone(),
                )
            } else {
                None
            }
        };
        if let Some(text) = choice {
            let mut state = self.consumer.state.borrow_mut();
            let snippet = state
                .snippet_sessions
                .get_mut(&view)
                .ok_or(EditorError::NotFound)?;
            if !snippet.session.synchronize(store) {
                return Err(EditorError::StaleRevision);
            }
            let before = store
                .views()
                .get(view)
                .ok_or(EditorError::NotFound)?
                .clone();
            snippet.session.select_active(store)?;
            if let Err(error) = snippet.session.replace(store, &text, None) {
                store.set_view_state(view, before.selection, before.scroll, before.folds)?;
                return Err(error);
            }
            snippet.session.step(store, true, |request| {
                snippet
                    .transforms
                    .evaluate(request.transform, request.value)
            })?;
            if !snippet.session.is_active() {
                state.snippet_sessions.remove(&view);
            }
            state.close(view);
            return Ok(true);
        }
        let document = store.documents().snapshot(origin.snapshot.id)?;
        let options = PreparationOptions {
            alternate,
            indent: self.editor.indent_options(&document),
            limits: ParseLimits {
                max_bytes: taide_model::file::REFUSED_FILE_BYTES as usize,
                max_nesting: SNIPPET_NESTING_LIMIT,
                max_markers: SNIPPET_MARKER_LIMIT,
            },
        };
        let indent = options.indent;
        let limits = options.limits;
        let language =
            supply::language(&document.metadata.language_id).ok_or(EditorError::InvalidBoundary)?;
        let clock = SnippetClock::now();
        let clipboard = self.consumer.state.borrow().clipboard(store, view);
        let prepared = self.consumer.state.borrow_mut().prepare_candidate(
            store,
            view,
            token,
            candidate,
            options,
            PreparationContext {
                model_path: &self.model_path,
                language,
                clipboard: clipboard.as_deref().map(String::as_str),
                clipboard_spread: true,
                clock: &clock,
            },
        )?;
        let owner = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let before = document.revision;
        let mut state = self.consumer.state.borrow_mut();
        if let Some(snippet) = state.snippet_sessions.get_mut(&view)
            && snippet.session.synchronize(store)
        {
            snippet
                .session
                .insert_nested(store, &owner, before, prepared.snippets)?;
            snippet.transforms = MonacoSnippetTransforms::new(limits);
            snippet.shown_choice = None;
            if !snippet.session.is_active() {
                state.snippet_sessions.remove(&view);
            }
        } else {
            let insertion = taide_native_editor::snippet_insertion::insert(
                store,
                &owner,
                before,
                prepared.snippets,
                limits,
            )?;
            let session = Session::new(store, insertion, indent, limits)?;
            state.snippet_sessions.remove(&view);
            if session.is_active() {
                state.snippet_sessions.insert(
                    view,
                    Snippet {
                        origin,
                        session,
                        transforms: prepared.transforms,
                        shown_choice: None,
                    },
                );
            }
        }
        state.close(view);
        Ok(store.documents().snapshot(document.id)?.revision != before)
    }

    fn documentation(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
    ) -> Option<Arc<RichDocument>> {
        let token = Uuid::parse_str(token).ok()?;
        let mut state = self.consumer.state.borrow_mut();
        state.request(store, view)?;
        let entry = state.entries.get_mut(&view)?;
        if entry.widget_token != token {
            return None;
        }
        if let Some(document) = entry.documents.get(&candidate) {
            return Some(document.clone());
        }
        let content = entry
            .model
            .as_ref()?
            .borrow()
            .candidate(candidate)?
            .documentation()?;
        if content.is_empty() {
            return None;
        }
        let document = Arc::new(crate::editor_markup::parse(&content));
        entry.documents.clear();
        entry.documents.insert(candidate, document.clone());
        self.consumer.context.request_repaint();
        Some(document)
    }

    fn detail(
        &self,
        store: &EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
    ) -> Option<String> {
        let token = Uuid::parse_str(token).ok()?;
        let state = self.consumer.state.borrow();
        state.request(store, view)?;
        let entry = state.entries.get(&view)?;
        if entry.widget_token != token {
            return None;
        }
        let detail = entry
            .model
            .as_ref()?
            .borrow()
            .candidate(candidate)?
            .item
            .detail
            .clone();
        detail.filter(|detail| !detail.is_empty())
    }

    fn icon_color(&self, ui: &Ui, kind: CompletionItemKind) -> Option<Color32> {
        let dark = ui.visuals().dark_mode;
        Some(match kind {
            CompletionItemKind::CLASS | CompletionItemKind::ENUM | CompletionItemKind::EVENT => {
                if dark {
                    CLASS_DARK
                } else {
                    CLASS_LIGHT
                }
            }
            CompletionItemKind::METHOD
            | CompletionItemKind::FUNCTION
            | CompletionItemKind::CONSTRUCTOR => {
                if dark {
                    FUNCTION_DARK
                } else {
                    FUNCTION_LIGHT
                }
            }
            CompletionItemKind::FIELD
            | CompletionItemKind::INTERFACE
            | CompletionItemKind::VARIABLE => {
                if dark {
                    FIELD_DARK
                } else {
                    FIELD_LIGHT
                }
            }
            _ => ui.visuals().text_color(),
        })
    }

    fn swatch(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
    ) -> Option<Color32> {
        let token = Uuid::parse_str(token).ok()?;
        let mut state = self.consumer.state.borrow_mut();
        state.request(store, view)?;
        let entry = state.entries.get_mut(&view)?;
        if entry.widget_token != token {
            return None;
        }
        if let Some(color) = entry.colors.get(&candidate) {
            return *color;
        }
        let color = colors::color(&entry.model.as_ref()?.borrow().candidate(candidate)?.item);
        entry.colors.insert(candidate, color);
        color
    }

    fn snippet_event(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        event: &Event,
    ) -> Result<bool, EditorError> {
        let Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        else {
            return Ok(false);
        };
        if modifiers.ctrl || modifiers.alt || modifiers.mac_cmd || modifiers.command {
            return Ok(false);
        }
        let mut state = self.consumer.state.borrow_mut();
        let Some(snippet) = state.snippet_sessions.get_mut(&view) else {
            return Ok(false);
        };
        if !snippet.session.synchronize(store) {
            state.snippet_sessions.remove(&view);
            return Ok(false);
        }
        match key {
            Key::Escape => snippet.session.cancel(),
            Key::Tab => {
                snippet.session.step(store, !modifiers.shift, |request| {
                    snippet
                        .transforms
                        .evaluate(request.transform, request.value)
                })?;
            }
            _ => return Ok(false),
        }
        if !snippet.session.is_active() {
            state.snippet_sessions.remove(&view);
        }
        Ok(true)
    }
}

impl taide_native_ui::editor_markup::Provider for Provider<'_, '_> {
    fn code(&mut self, _ui: &Ui, language: &str, text: &str) -> Option<egui::text::LayoutJob> {
        self.consumer.state.borrow().code.job(
            language,
            text,
            &self.language,
            &self.editor.appearance,
        )
    }

    fn image(
        &mut self,
        ui: &mut Ui,
        source: &str,
        alt: &str,
        dimensions: ImageDimensions,
    ) -> Option<egui::Response> {
        self.consumer.state.borrow_mut().images.show(
            ui,
            self.project.as_ref()?,
            source,
            alt,
            dimensions,
        )
    }

    fn open_link(&mut self, ui: &Ui, target: &str) -> bool {
        crate::editor_documentation::open_link(
            ui,
            target,
            self.project.as_ref(),
            &self.owner_key,
            self.viewport,
            self.commands,
        )
    }
}

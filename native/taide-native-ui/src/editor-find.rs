use std::fmt;
use std::ops::Range;

use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness,
};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, Edit, EditorError};
use taide_native_editor::editing::normalize_line_breaks;
use taide_native_editor::find::{
    FIND_MATCH_LIMIT, FIND_SEARCH_TIMEOUT, FindMatch, FindOptions, FindPatternCompiler,
    FindPatternError, FindQuery, FindResults, FindSeedOptions, seed_find_text,
};
use taide_native_editor::find_replacement::{
    ReplacePattern, apply_replacement_edits, replacement_edits,
};
use taide_native_editor::language_configuration::LanguageRules;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{Selection, SelectionSet, ViewId};

const SCOPE_LAYER: u8 = 1;
const MATCH_LAYER: u8 = 2;
const CURRENT_MATCH_LAYER: u8 = 3;
pub(crate) const OPTIONS_REVEAL_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindCommand {
    Open,
    OpenWithSelection,
    OpenWithArgs,
    OpenReplace,
    Next,
    Previous,
    NextSelection,
    PreviousSelection,
    Close,
    ToggleCase,
    ToggleWholeWord,
    ToggleRegex,
    ToggleScope,
    TogglePreserveCase,
    ReplaceOne,
    ReplaceAll,
    SelectAll,
    GoToMatch,
}

impl FindCommand {
    pub fn from_action(action: &str) -> Option<Self> {
        Some(match action {
            "actions.find" => Self::Open,
            "editor.actions.findWithArgs" => Self::OpenWithArgs,
            "actions.findWithSelection" => Self::OpenWithSelection,
            "editor.action.startFindReplaceAction" => Self::OpenReplace,
            "editor.action.nextMatchFindAction" => Self::Next,
            "editor.action.previousMatchFindAction" => Self::Previous,
            "editor.action.nextSelectionMatchFindAction" => Self::NextSelection,
            "editor.action.previousSelectionMatchFindAction" => Self::PreviousSelection,
            "closeFindWidget" => Self::Close,
            "toggleFindCaseSensitive" => Self::ToggleCase,
            "toggleFindWholeWord" => Self::ToggleWholeWord,
            "toggleFindRegex" => Self::ToggleRegex,
            "toggleFindInSelection" => Self::ToggleScope,
            "togglePreserveCase" => Self::TogglePreserveCase,
            "editor.action.replaceOne" => Self::ReplaceOne,
            "editor.action.replaceAll" => Self::ReplaceAll,
            "editor.action.selectAllMatches" => Self::SelectAll,
            "editor.action.goToMatchFindAction" => Self::GoToMatch,
            _ => return None,
        })
    }

    pub fn requires_write(self) -> bool {
        matches!(
            self,
            Self::OpenReplace | Self::ReplaceOne | Self::ReplaceAll
        )
    }
}

#[derive(Debug)]
pub enum FindError {
    Editor(EditorError),
    Pattern(FindPatternError),
}

impl From<EditorError> for FindError {
    fn from(error: EditorError) -> Self {
        Self::Editor(error)
    }
}

impl From<FindPatternError> for FindError {
    fn from(error: FindPatternError) -> Self {
        Self::Pattern(error)
    }
}

impl fmt::Display for FindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Editor(error) => write!(formatter, "{error:?}"),
            Self::Pattern(error) => write!(formatter, "{error}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindFocus {
    Find,
    Replace,
    Editor,
}

struct CachedQuery {
    source: String,
    options: FindOptions,
    query: Option<FindQuery>,
    error: Option<String>,
}

#[derive(Default)]
pub struct EditorFind {
    pub search: String,
    pub replacement: String,
    pub options: FindOptions,
    pub preserve_case: bool,
    pub visible: bool,
    pub replace_visible: bool,
    pub error: Option<String>,
    pub match_number: Option<String>,
    pub(crate) focus: Option<FindFocus>,
    pub(crate) width: Option<f32>,
    pub(crate) composing: bool,
    pub(crate) options_until: Option<crate::Instant>,
    pub(crate) search_history_draft: Option<String>,
    pub(crate) replacement_history_draft: Option<String>,
    pub(crate) match_focus: bool,
    pub(crate) input_focused: bool,
    document: Option<DocumentId>,
    revision: Option<u64>,
    cache: Option<CachedQuery>,
    results: FindResults,
    scopes: Option<DecorationLayer>,
    current_match: Option<(u64, Range<usize>)>,
}

impl EditorFind {
    pub fn results(&self) -> &FindResults {
        &self.results
    }

    pub fn scopes(&self) -> Vec<Range<usize>> {
        self.scopes.as_ref().map_or_else(Vec::new, |layer| {
            layer
                .items()
                .iter()
                .map(|item| item.bytes.clone())
                .collect()
        })
    }

    pub fn refresh(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        compiler: &dyn FindPatternCompiler,
    ) -> Result<bool, FindError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        if self.document != Some(document.id) {
            self.document = Some(document.id);
            self.revision = None;
            self.scopes = None;
            self.current_match = None;
        }
        if document.metadata.read_only {
            self.replace_visible = false;
        }
        let changed_query = self
            .cache
            .as_ref()
            .is_none_or(|cache| cache.source != self.search || cache.options != self.options);
        if !changed_query && self.revision == Some(document.revision) {
            return Ok(false);
        }
        if let Some(layer) = &self.scopes {
            let tracked = layer
                .tracking(store.changes_since(document.id, layer.revision())?)
                .map(|tracked| tracked.into_owned());
            let Some(tracked) = tracked else {
                self.error =
                    Some("The search scope could not be tracked. Select the range again.".into());
                self.results = FindResults::default();
                self.revision = Some(document.revision);
                return Ok(true);
            };
            self.scopes = Some(tracked);
        }
        self.error = None;
        self.current_match = None;
        if changed_query {
            let query = match FindQuery::new(&self.search, self.options.clone(), compiler) {
                Ok(query) => Some(query),
                Err(error) => {
                    self.error = Some(error.to_string());
                    None
                }
            };
            self.cache = Some(CachedQuery {
                source: self.search.clone(),
                options: self.options.clone(),
                query,
                error: self.error.clone(),
            });
        }
        self.results = FindResults::default();
        if let Some(query) = self.cache.as_ref().and_then(|cache| cache.query.as_ref()) {
            match query.find_matches(
                &document.rope,
                &self.scopes(),
                FIND_MATCH_LIMIT,
                Some(FIND_SEARCH_TIMEOUT),
            ) {
                Ok(results) => {
                    if results.timed_out {
                        self.error = Some("Search exceeded its time limit".into());
                    }
                    self.results = results;
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        } else if let Some(cache) = &self.cache {
            self.error = cache.error.clone();
        }
        self.revision = Some(document.revision);
        Ok(true)
    }

    pub fn execute(
        &mut self,
        command: FindCommand,
        store: &mut EditorStore,
        view: ViewId,
        compiler: &dyn FindPatternCompiler,
        rules: Option<&dyn LanguageRules>,
    ) -> Result<bool, FindError> {
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        if command.requires_write() && document.metadata.read_only {
            return Err(EditorError::ReadOnly.into());
        }
        self.refresh(store, view, compiler)?;
        if !self.visible
            && matches!(
                command,
                FindCommand::ToggleCase
                    | FindCommand::ToggleWholeWord
                    | FindCommand::ToggleRegex
                    | FindCommand::TogglePreserveCase
            )
        {
            self.options_until = Some(crate::Instant::now() + OPTIONS_REVEAL_DELAY);
        }
        match command {
            FindCommand::Close => {
                self.visible = false;
                self.scopes = None;
                self.match_number = None;
                self.focus = Some(FindFocus::Editor);
                self.revision = None;
                return Ok(false);
            }
            FindCommand::Open
            | FindCommand::OpenWithSelection
            | FindCommand::OpenWithArgs
            | FindCommand::OpenReplace => {
                let was_visible = self.visible;
                self.visible = true;
                if command == FindCommand::OpenReplace {
                    self.replace_visible = true;
                } else if !was_visible {
                    self.replace_visible = false;
                }
                let selection = current.selection.selections[current.selection.primary];
                if (command != FindCommand::OpenReplace
                    || selection.anchor != selection.head && !self.input_focused)
                    && (command != FindCommand::OpenWithArgs || self.search.is_empty())
                {
                    self.seed(
                        &document,
                        &current.selection,
                        rules,
                        command == FindCommand::OpenWithSelection,
                    );
                }
                if command != FindCommand::OpenWithSelection {
                    self.focus = Some(
                        if command == FindCommand::OpenReplace
                            && (self.input_focused || selection.anchor != selection.head)
                        {
                            FindFocus::Replace
                        } else {
                            FindFocus::Find
                        },
                    );
                }
            }
            FindCommand::NextSelection | FindCommand::PreviousSelection => {
                self.visible = true;
                self.seed(&document, &current.selection, rules, false);
            }
            FindCommand::Next | FindCommand::Previous if self.search.is_empty() => {
                self.visible = true;
                self.seed(&document, &current.selection, rules, false);
            }
            FindCommand::ToggleCase => self.options.match_case = !self.options.match_case,
            FindCommand::ToggleWholeWord => self.options.whole_word = !self.options.whole_word,
            FindCommand::ToggleRegex => self.options.is_regex = !self.options.is_regex,
            FindCommand::TogglePreserveCase => self.preserve_case = !self.preserve_case,
            FindCommand::ToggleScope => self.toggle_scope(&document, &current.selection),
            FindCommand::GoToMatch => {
                if self.results.matches.is_empty() {
                    self.error = Some("No matches. Try searching for something else.".into());
                } else {
                    self.match_number = Some(String::new());
                    self.match_focus = true;
                }
            }
            _ => {}
        }
        let query_changed = self.refresh(store, view, compiler)?;
        if query_changed
            && matches!(
                command,
                FindCommand::ToggleCase
                    | FindCommand::ToggleWholeWord
                    | FindCommand::ToggleRegex
                    | FindCommand::ToggleScope
            )
            && self.error.is_none()
        {
            self.navigate(store, view, true, true)?;
        }
        match command {
            FindCommand::Next | FindCommand::NextSelection => {
                self.navigate(store, view, true, false)?;
            }
            FindCommand::Previous | FindCommand::PreviousSelection => {
                self.navigate(store, view, false, false)?;
            }
            FindCommand::ReplaceOne => return self.replace_one(store, view, compiler),
            FindCommand::ReplaceAll => return self.replace_all(store, view, compiler),
            FindCommand::SelectAll => {
                let matches = self.all_matches(&document)?;
                if !matches.is_empty() {
                    let primary = current.selection.selections[current.selection.primary];
                    let selections = matches
                        .iter()
                        .map(|found| Selection {
                            anchor: found.range.start,
                            head: found.range.end,
                        })
                        .collect::<Vec<_>>();
                    let primary = selections
                        .iter()
                        .position(|selection| *selection == primary)
                        .unwrap_or(0);
                    store.set_view_state(
                        view,
                        SelectionSet {
                            primary,
                            selections,
                        },
                        current.scroll,
                        current.folds,
                    )?;
                    self.focus = Some(FindFocus::Editor);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn seed(
        &mut self,
        document: &DocumentSnapshot,
        selection: &SelectionSet,
        rules: Option<&dyn LanguageRules>,
        multiline: bool,
    ) {
        if let Some(source) = seed_find_text(
            document,
            selection,
            rules,
            FindSeedOptions {
                is_regex: self.options.is_regex,
                allow_multiline: multiline,
                require_selection: false,
            },
        ) {
            self.search = source;
        }
    }

    fn toggle_scope(&mut self, document: &DocumentSnapshot, selections: &SelectionSet) {
        if self.scopes.take().is_some() {
            self.revision = None;
            return;
        }
        let rope = &document.rope;
        let items = selections
            .selections
            .iter()
            .filter_map(|selection| {
                let mut start = selection.anchor.min(selection.head);
                let mut end = selection.anchor.max(selection.head);
                if start == end {
                    return None;
                }
                let first = rope.byte_to_line(start);
                let mut last = rope.byte_to_line(end);
                if first != last {
                    if rope.line_to_byte(last) == end {
                        last -= 1;
                    }
                    start = rope.line_to_byte(first);
                    let line = rope.line(last).to_string();
                    end = rope.line_to_byte(last) + line.trim_end_matches(['\r', '\n']).len();
                }
                Some(Decoration {
                    bytes: start..end,
                    kind: DecorationKind::Inline(InlineStyle::default()),
                    stickiness: Stickiness::AlwaysGrowsWhenTypingAtEdges,
                })
            })
            .collect::<Vec<_>>();
        if !items.is_empty() {
            self.scopes = Some(DecorationLayer::new(document.revision, SCOPE_LAYER, items));
            self.revision = None;
        }
    }

    fn all_matches(&self, document: &DocumentSnapshot) -> Result<Vec<FindMatch>, FindError> {
        if self.document != Some(document.id) || self.revision != Some(document.revision) {
            return Err(EditorError::StaleRevision.into());
        }
        if let Some(error) = &self.error {
            return Err(FindPatternError(error.clone()).into());
        }
        if self.results.timed_out {
            return Err(FindPatternError("Search exceeded its time limit".into()).into());
        }
        if !self.results.limit_reached {
            return Ok(self.results.matches.clone());
        }
        let Some(query) = self.cache.as_ref().and_then(|cache| cache.query.as_ref()) else {
            return Ok(Vec::new());
        };
        let results = query.find_matches(
            &document.rope,
            &self.scopes(),
            usize::MAX,
            Some(FIND_SEARCH_TIMEOUT),
        )?;
        if results.timed_out {
            return Err(FindPatternError("Search exceeded its time limit".into()).into());
        }
        Ok(results.matches)
    }

    pub fn navigate(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        forward: bool,
        inclusive: bool,
    ) -> Result<bool, FindError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let primary = current.selection.selections[current.selection.primary];
        let start = primary.anchor.min(primary.head);
        let end = primary.anchor.max(primary.head);
        let matches = self.all_matches(&document)?;
        let found = if forward {
            matches
                .iter()
                .find(|found| {
                    found.range.start >= if inclusive { start } else { end }
                        && (inclusive || found.range != (start..end))
                })
                .or_else(|| matches.first())
        } else {
            matches
                .iter()
                .rev()
                .find(|found| {
                    found.range.end <= start && (inclusive || found.range != (start..end))
                })
                .or_else(|| matches.last())
        };
        let Some(found) = found else {
            return Ok(false);
        };
        select_match(store, view, found.range.clone())?;
        self.current_match = Some((document.revision, found.range.clone()));
        Ok(true)
    }

    pub fn go_to_match(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        number: usize,
    ) -> Result<bool, FindError> {
        if !self.preview_match(store, view, number)? {
            return Ok(false);
        }
        self.match_number = None;
        self.focus = Some(FindFocus::Find);
        Ok(true)
    }

    pub(crate) fn preview_match(
        &self,
        store: &mut EditorStore,
        view: ViewId,
        number: usize,
    ) -> Result<bool, FindError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let matches = self.all_matches(&document)?;
        let Some(found) = number.checked_sub(1).and_then(|index| matches.get(index)) else {
            return Ok(false);
        };
        select_match(store, view, found.range.clone())?;
        Ok(true)
    }

    fn replace_one(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        compiler: &dyn FindPatternCompiler,
    ) -> Result<bool, FindError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        let selection = current.selection.selections[current.selection.primary];
        let range = selection.anchor.min(selection.head)..selection.anchor.max(selection.head);
        let matches = self.all_matches(&document)?;
        let Some(found) = matches.iter().find(|found| found.range == range) else {
            self.navigate(store, view, true, true)?;
            return Ok(false);
        };
        let replacement = ReplacePattern::new(&self.replacement, self.options.is_regex)
            .build(&found.captures, self.preserve_case);
        let text = normalize_line_breaks(&replacement, document.metadata.line_ending.as_str());
        let end = found.range.start + text.len();
        apply_replacement_edits(
            store,
            view,
            document.revision,
            vec![Edit {
                bytes: found.range.clone(),
                text,
            }],
        )?;
        select_match(store, view, end..end)?;
        self.refresh(store, view, compiler)?;
        self.navigate(store, view, true, found.range.start != found.range.end)?;
        Ok(true)
    }

    fn replace_all(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        compiler: &dyn FindPatternCompiler,
    ) -> Result<bool, FindError> {
        let current = store.views().get(view).ok_or(EditorError::NotFound)?;
        let document = store.documents().snapshot(current.document)?;
        if let Some(error) = &self.error {
            return Err(FindPatternError(error.clone()).into());
        }
        let Some(query) = self.cache.as_ref().and_then(|cache| cache.query.as_ref()) else {
            return Ok(false);
        };
        let pattern = ReplacePattern::new(&self.replacement, self.options.is_regex);
        let edits = replacement_edits(
            query,
            &document,
            &self.scopes(),
            &pattern,
            self.preserve_case,
            Some(FIND_SEARCH_TIMEOUT),
        )?;
        let changed = apply_replacement_edits(store, view, document.revision, edits)?;
        self.refresh(store, view, compiler)?;
        Ok(changed)
    }

    pub fn decorations(
        &self,
        selection: &SelectionSet,
        highlight: [u8; 4],
        current: [u8; 4],
        scope: [u8; 4],
    ) -> Vec<DecorationLayer> {
        if !self.visible {
            return Vec::new();
        }
        let Some(revision) = self.revision else {
            return Vec::new();
        };
        let primary = selection.selections[selection.primary];
        let selected = primary.anchor.min(primary.head)..primary.anchor.max(primary.head);
        let inline = |bytes, background| Decoration {
            bytes,
            kind: DecorationKind::Inline(InlineStyle {
                background: Some(background),
                ..Default::default()
            }),
            stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
        };
        let matches = self
            .results
            .matches
            .iter()
            .map(|found| inline(found.range.clone(), highlight))
            .collect();
        let current = if self
            .results
            .matches
            .iter()
            .any(|found| found.range == selected)
            || self
                .current_match
                .as_ref()
                .is_some_and(|(at, range)| *at == revision && *range == selected)
        {
            vec![inline(selected, current)]
        } else {
            Vec::new()
        };
        let scopes = self
            .scopes
            .as_ref()
            .filter(|layer| layer.revision() == revision)
            .map_or_else(Vec::new, |layer| {
                layer
                    .items()
                    .iter()
                    .map(|item| Decoration {
                        bytes: item.bytes.clone(),
                        kind: DecorationKind::LineBackground(scope),
                        stickiness: Stickiness::AlwaysGrowsWhenTypingAtEdges,
                    })
                    .collect()
            });
        vec![
            DecorationLayer::new(revision, SCOPE_LAYER, scopes),
            DecorationLayer::new(revision, MATCH_LAYER, matches),
            DecorationLayer::new(revision, CURRENT_MATCH_LAYER, current),
        ]
    }

    pub fn scroll_decorations(
        &self,
        colors: crate::editor_overview::OverviewColors,
    ) -> Option<DecorationLayer> {
        if !self.visible || self.error.is_some() {
            return None;
        }
        Some(DecorationLayer::new(
            self.revision?,
            MATCH_LAYER,
            self.results
                .matches
                .iter()
                .map(|found| Decoration {
                    bytes: found.range.clone(),
                    kind: DecorationKind::Overview {
                        lane: taide_native_editor::decoration::OverviewLane::Center,
                        color: colors.find.to_srgba_unmultiplied(),
                        minimap: Some(colors.minimap_find.to_srgba_unmultiplied()),
                    },
                    stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
                })
                .collect(),
        ))
    }
}

fn select_match(
    store: &mut EditorStore,
    view: ViewId,
    range: Range<usize>,
) -> Result<(), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let folds = current
        .folds
        .into_iter()
        .filter(|fold| {
            !(fold.start <= range.start && range.start < fold.end
                || fold.start < range.end && range.end <= fold.end
                || range.start <= fold.start && fold.end <= range.end)
        })
        .collect();
    store.set_view_state(
        view,
        SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: range.start,
                head: range.end,
            }],
        },
        current.scroll,
        folds,
    )?;
    store.set_edit_run(view, None)?;
    store.request_selection_reveal(view, range, true)
}

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::change_journal::ChangesSince;
use crate::document::{
    DocumentId, DocumentSnapshot, Edit, EditorError, LineEnding, UndoGroup, byte_to_char,
};
use crate::editing::replacement_transaction;
use crate::indent::IndentOptions;
use crate::snippet_insertion::{self, Insertion, PreparedSnippet};
use crate::snippet_normalization::own_cost;
use crate::snippet_syntax::{Index, Marker, ParseLimits, Transform};
use crate::snippet_tracking::map_utf16_range;
use crate::snippet_whitespace::normalize_transform;
use crate::store::{EditorStore, Transaction};
use crate::view::{Selection, SelectionSet, ViewId, ViewState};

pub struct TransformRequest<'a> {
    pub transform: &'a Transform,
    pub value: &'a str,
    pub cursor_index: usize,
    pub line_leading_whitespace: &'a str,
    pub indent: IndentOptions,
    pub line_ending: LineEnding,
}

#[derive(Clone)]
struct Placeholder {
    index: Index,
    units: Range<usize>,
    authored_nonempty: bool,
    enclosing: Vec<usize>,
    transform: Option<Transform>,
    choices: Option<Vec<String>>,
    context: usize,
    owned_bytes: usize,
    owned_markers: usize,
}

#[derive(Clone)]
struct Snippet {
    cursor_index: usize,
    placeholders: Vec<Placeholder>,
    groups: Vec<Vec<usize>>,
    active_group: usize,
    contexts: Vec<Context>,
}

#[derive(Clone)]
struct Context {
    cursor_index: usize,
    line_leading_whitespace: String,
}

#[derive(Clone)]
struct State {
    document: DocumentId,
    view: ViewId,
    revision: u64,
    edit_group: UndoGroup,
    primary_cursor: usize,
    snippets: Vec<Snippet>,
    indent: IndentOptions,
}

#[derive(Clone)]
pub struct Session {
    state: Option<State>,
    limits: ParseLimits,
}

pub struct ActiveChoice<'a> {
    pub index: Index,
    pub bytes: Range<usize>,
    pub options: &'a [String],
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlaceholderDecoration {
    pub bytes: Range<usize>,
    pub is_active: bool,
    pub is_final: bool,
}

impl Session {
    pub fn new(
        store: &EditorStore,
        insertion: Insertion,
        indent: IndentOptions,
        limits: ParseLimits,
    ) -> Result<Self, EditorError> {
        let view = store
            .views()
            .get(insertion.view)
            .ok_or(EditorError::NotFound)?;
        if view.document != insertion.document || view.selection != insertion.selection {
            return Err(EditorError::InvalidIdentity);
        }
        let document = store.documents().snapshot(insertion.document)?;
        if document.revision != insertion.revision {
            return Err(EditorError::StaleRevision);
        }
        let state = State::from_insertion(&document, &insertion, indent, limits)?;
        let mut session = Self {
            state: Some(state),
            limits,
        };
        if session.state.as_ref().is_none_or(|state| {
            state
                .snippets
                .first()
                .is_none_or(|snippet| snippet.groups.len() <= 1)
        }) {
            session.cancel();
        }
        Ok(session)
    }

    pub fn insert_nested(
        &mut self,
        store: &mut EditorStore,
        expected_view: &ViewState,
        revision: u64,
        snippets: Vec<PreparedSnippet>,
    ) -> Result<(), EditorError> {
        let document = self.validate(store)?;
        let state = self.state.as_ref().ok_or(EditorError::Refused)?;
        if expected_view.id != state.view || expected_view.document != state.document {
            return Err(EditorError::InvalidIdentity);
        }
        let mut plan =
            snippet_insertion::prepare(store, expected_view, revision, snippets, self.limits)?;
        let nested =
            State::from_insertion(&plan.document, &plan.insertion, state.indent, self.limits)?;
        let parents = state.nested_parents(&document, &expected_view.selection)?;
        let mut next = state.clone();
        next.map_edits(&document, &plan.transaction.edits)?;
        let nontrivial = nested.snippets.iter().any(|snippet| {
            snippet
                .placeholders
                .iter()
                .any(|placeholder| placeholder.index != Index::FINAL)
        });
        if nontrivial && let Some(parents) = parents {
            next.merge(nested, parents, self.limits)?;
            plan.insertion.selection = next.selection(&plan.document)?;
        } else {
            plan.insertion.selection = SelectionSet {
                primary: expected_view.selection.primary,
                selections: plan
                    .insertion
                    .snippets
                    .iter()
                    .map(|snippet| Selection {
                        anchor: snippet.bytes.end,
                        head: snippet.bytes.end,
                    })
                    .collect(),
            };
        }
        plan.insertion.selection.validate(&plan.document.rope)?;
        plan.transaction.selection_after = Some(plan.insertion.selection.clone());
        let insertion = snippet_insertion::apply(store, plan)?;
        next.revision = insertion.revision;
        next.edit_group = UndoGroup(insertion.revision);
        self.state = Some(next);
        self.synchronize(store);
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.state.is_some()
    }

    pub fn cancel(&mut self) {
        self.state = None;
    }

    fn validate(&mut self, store: &EditorStore) -> Result<DocumentSnapshot, EditorError> {
        let Some(state) = &self.state else {
            return Err(EditorError::Refused);
        };
        let result = (|| {
            let view = store.views().get(state.view).ok_or(EditorError::NotFound)?;
            if view.document != state.document {
                return Err(EditorError::InvalidIdentity);
            }
            let document = store.documents().snapshot(state.document)?;
            if document.revision != state.revision {
                return Err(EditorError::StaleRevision);
            }
            if document.metadata.read_only {
                return Err(EditorError::ReadOnly);
            }
            if !state.contains_selection(&document, &view.selection)? {
                return Err(EditorError::InvalidBoundary);
            }
            Ok(document)
        })();
        if result.is_err() {
            self.cancel();
        }
        result
    }

    pub fn synchronize(&mut self, store: &EditorStore) -> bool {
        let followed = (|| {
            let state = self.state.as_ref().ok_or(EditorError::Refused)?;
            let document = store.documents().snapshot(state.document)?;
            if document.revision == state.revision {
                return Ok(());
            }
            let ChangesSince::Tracked(changes) =
                store.changes_since(state.document, state.revision)?
            else {
                return Err(EditorError::StaleRevision);
            };
            let mut next = state.clone();
            for change in changes {
                if next.revision != change.revision_before {
                    return Err(EditorError::StaleRevision);
                }
                for span in change.spans.iter().rev() {
                    let changed = span.start_utf16..span.old_end_utf16;
                    let inside = next.snippets.iter().any(|snippet| {
                        snippet.groups[snippet.active_group].iter().any(|index| {
                            let placeholder = &snippet.placeholders[*index];
                            placeholder.index != Index::FINAL
                                && placeholder.units.start <= changed.start
                                && placeholder.units.end >= changed.end
                        })
                    });
                    if !inside {
                        return Err(EditorError::InvalidBoundary);
                    }
                    next.map_units(changed, span.new_end_utf16 - span.start_utf16)?;
                }
                next.revision = change.revision_after;
            }
            if next.revision != document.revision {
                return Err(EditorError::StaleRevision);
            }
            self.state = Some(next);
            Ok::<(), EditorError>(())
        })();
        if followed.is_err() {
            self.cancel();
            return false;
        }
        self.validate(store).is_ok()
    }

    pub fn decorations(
        &self,
        store: &EditorStore,
    ) -> Result<Vec<PlaceholderDecoration>, EditorError> {
        let Some(state) = &self.state else {
            return Ok(Vec::new());
        };
        let view = store.views().get(state.view).ok_or(EditorError::NotFound)?;
        if view.document != state.document {
            return Err(EditorError::InvalidIdentity);
        }
        let document = store.documents().snapshot(state.document)?;
        if document.revision != state.revision {
            return Err(EditorError::StaleRevision);
        }
        let mut result = Vec::new();
        for snippet in &state.snippets {
            let mut active = BTreeSet::new();
            for index in &snippet.groups[snippet.active_group] {
                active.insert(*index);
                active.extend(snippet.placeholders[*index].enclosing.iter().copied());
            }
            for (index, placeholder) in snippet.placeholders.iter().enumerate() {
                result.push(PlaceholderDecoration {
                    bytes: byte_range(&document, &placeholder.units)?,
                    is_active: active.contains(&index),
                    is_final: placeholder.index == Index::FINAL,
                });
            }
        }
        Ok(result)
    }

    pub fn active_choice(
        &self,
        store: &EditorStore,
    ) -> Result<Option<ActiveChoice<'_>>, EditorError> {
        let Some(state) = &self.state else {
            return Ok(None);
        };
        let document = store.documents().snapshot(state.document)?;
        let view = store.views().get(state.view).ok_or(EditorError::NotFound)?;
        if view.document != state.document {
            return Err(EditorError::InvalidIdentity);
        }
        if document.revision != state.revision {
            return Err(EditorError::StaleRevision);
        }
        if !state.contains_selection(&document, &view.selection)? {
            return Err(EditorError::InvalidIdentity);
        }
        let Some(snippet) = state
            .snippets
            .iter()
            .find(|snippet| snippet.cursor_index == state.primary_cursor)
        else {
            return Ok(None);
        };
        let Some(group) = snippet.groups.get(snippet.active_group) else {
            return Ok(None);
        };
        let Some(index) = group.first() else {
            return Ok(None);
        };
        let placeholder = &snippet.placeholders[*index];
        let Some(options) = &placeholder.choices else {
            return Ok(None);
        };
        Ok(Some(ActiveChoice {
            index: placeholder.index,
            bytes: byte_range(&document, &placeholder.units)?,
            options,
        }))
    }

    pub fn replace(
        &mut self,
        store: &mut EditorStore,
        text: &str,
        delete_forward: Option<bool>,
    ) -> Result<bool, EditorError> {
        let document = self.validate(store)?;
        let state = self.state.as_ref().ok_or(EditorError::Refused)?;
        if store
            .views()
            .get(state.view)
            .is_some_and(|view| view.composition.is_some())
        {
            return Err(EditorError::Refused);
        }
        let count = store
            .views()
            .get(state.view)
            .ok_or(EditorError::NotFound)?
            .selection
            .selections
            .len();
        if text
            .len()
            .checked_mul(count)
            .is_none_or(|bytes| bytes > self.limits.max_bytes)
        {
            return Err(EditorError::Capacity);
        }
        let Some(mut transaction) =
            replacement_transaction(store, state.view, text, delete_forward)?
        else {
            return Ok(false);
        };
        transaction.group = state.edit_group;
        if let Some(selection) = &mut transaction.selection_after {
            let primary = selection.selections[selection.primary];
            selection.selections.dedup();
            selection.primary = selection
                .selections
                .iter()
                .position(|selection| *selection == primary)
                .ok_or(EditorError::InvalidBoundary)?;
        }
        let mut next = state.clone();
        next.map_edits(&document, &transaction.edits)?;
        next.revision = store.apply(state.document, transaction)?;
        self.state = Some(next);
        if !self.synchronize(store) {
            store.break_undo_group(document.id)?;
        }
        Ok(true)
    }

    pub fn select_active(&mut self, store: &mut EditorStore) -> Result<(), EditorError> {
        let document = self.validate(store)?;
        let state = self.state.as_ref().ok_or(EditorError::Refused)?;
        let current = store.views().get(state.view).ok_or(EditorError::NotFound)?;
        store.set_view_state(
            state.view,
            state.selection(&document)?,
            current.scroll.clone(),
            current.folds.clone(),
        )
    }

    pub fn step(
        &mut self,
        store: &mut EditorStore,
        forward: bool,
        mut evaluate: impl FnMut(TransformRequest<'_>) -> Result<String, EditorError>,
    ) -> Result<bool, EditorError> {
        loop {
            let document = self.validate(store)?;
            let state = self.state.as_ref().ok_or(EditorError::Refused)?;
            let view = store.views().get(state.view).ok_or(EditorError::NotFound)?;
            if view.composition.is_some() {
                return Err(EditorError::Refused);
            }
            let mut edits = Vec::new();
            let mut bytes = 0usize;
            for snippet in &state.snippets {
                for index in &snippet.groups[snippet.active_group] {
                    let placeholder = &snippet.placeholders[*index];
                    let Some(transform) = &placeholder.transform else {
                        continue;
                    };
                    let range = byte_range(&document, &placeholder.units)?;
                    if range.len() > self.limits.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    let current = document.rope.byte_slice(range.clone()).to_string();
                    let context = snippet
                        .contexts
                        .get(placeholder.context)
                        .ok_or(EditorError::InvalidBoundary)?;
                    let text = evaluate(TransformRequest {
                        transform,
                        value: &current,
                        cursor_index: context.cursor_index,
                        line_leading_whitespace: &context.line_leading_whitespace,
                        indent: state.indent,
                        line_ending: document.metadata.line_ending,
                    })?;
                    let text = normalize_transform(
                        &text,
                        &context.line_leading_whitespace,
                        state.indent,
                        document.metadata.line_ending,
                        self.limits.max_bytes,
                    )?;
                    bytes = bytes.checked_add(text.len()).ok_or(EditorError::Capacity)?;
                    if bytes > self.limits.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    edits.push(Edit { bytes: range, text });
                }
            }
            edits.sort_by_key(|edit| (edit.bytes.start, edit.bytes.end));
            let mut next = state.clone();
            next.map_edits(&document, &edits)?;
            if !edits.is_empty() {
                next.revision = store.apply(
                    state.document,
                    Transaction {
                        revision: state.revision,
                        edits,
                        group: state.edit_group,
                        origin: Some(state.view),
                        selection_after: None,
                    },
                )?;
            }
            let mut skip = true;
            for snippet in &mut next.snippets {
                let previous = snippet.active_group;
                snippet.active_group = if forward {
                    (previous + 1).min(snippet.groups.len() - 1)
                } else {
                    previous.saturating_sub(1)
                };
                skip &= previous != snippet.active_group
                    && snippet.groups[snippet.active_group].iter().all(|index| {
                        let placeholder = &snippet.placeholders[*index];
                        std::iter::once(*index)
                            .chain(placeholder.enclosing.iter().copied())
                            .any(|index| {
                                let parent = &snippet.placeholders[index];
                                parent.authored_nonempty && parent.units.is_empty()
                            })
                    });
            }
            let after = store.documents().snapshot(next.document)?;
            let selection = next.selection(&after)?;
            let current_view = store
                .views()
                .get(next.view)
                .ok_or(EditorError::NotFound)?
                .clone();
            store.set_view_state(
                next.view,
                selection,
                current_view.scroll,
                current_view.folds,
            )?;
            let last = next
                .snippets
                .first()
                .is_none_or(|snippet| snippet.active_group == snippet.groups.len() - 1);
            self.state = Some(next);
            if last {
                store.break_undo_group(after.id)?;
                self.cancel();
                return Ok(true);
            }
            if !skip {
                self.synchronize(store);
                return Ok(true);
            }
        }
    }
}

fn byte_range(
    document: &DocumentSnapshot,
    units: &Range<usize>,
) -> Result<Range<usize>, EditorError> {
    let start = document
        .rope
        .try_utf16_cu_to_char(units.start)
        .map_err(|_| EditorError::InvalidBoundary)?;
    let end = document
        .rope
        .try_utf16_cu_to_char(units.end)
        .map_err(|_| EditorError::InvalidBoundary)?;
    Ok(document.rope.char_to_byte(start)..document.rope.char_to_byte(end))
}

impl State {
    fn from_insertion(
        document: &DocumentSnapshot,
        insertion: &Insertion,
        indent: IndentOptions,
        limits: ParseLimits,
    ) -> Result<Self, EditorError> {
        if indent.tab_size == 0 {
            return Err(EditorError::InvalidBoundary);
        }
        let primary_cursor = insertion.primary_cursor;
        if !insertion
            .snippets
            .iter()
            .any(|snippet| snippet.cursor_index == primary_cursor)
        {
            return Err(EditorError::InvalidBoundary);
        }
        let mut snippets = Vec::new();
        let mut allocated_bytes = 0usize;
        let mut allocated_markers = 0usize;
        for snippet in &insertion.snippets {
            allocated_bytes = allocated_bytes
                .checked_add(snippet.line_leading_whitespace.len())
                .ok_or(EditorError::Capacity)?;
            let mut placeholders = Vec::new();
            for span in &snippet.placeholders {
                let mut nodes = snippet.markers.as_slice();
                let mut marker = None;
                for index in &span.marker_path {
                    let node = nodes.get(*index).ok_or(EditorError::InvalidBoundary)?;
                    marker = Some(node);
                    nodes = match node {
                        Marker::Text(_) => &[],
                        Marker::Placeholder { children, .. }
                        | Marker::Variable { children, .. } => children,
                    };
                }
                let Some(Marker::Placeholder {
                    transform, choices, ..
                }) = marker
                else {
                    return Err(EditorError::InvalidBoundary);
                };
                let (bytes, markers) = own_cost(marker.ok_or(EditorError::InvalidBoundary)?)?;
                allocated_bytes = allocated_bytes
                    .checked_add(bytes)
                    .ok_or(EditorError::Capacity)?;
                allocated_markers = allocated_markers
                    .checked_add(markers)
                    .ok_or(EditorError::Capacity)?;
                if allocated_bytes > limits.max_bytes || allocated_markers > limits.max_markers {
                    return Err(EditorError::Capacity);
                }
                let start = document
                    .rope
                    .char_to_utf16_cu(byte_to_char(&document.rope, span.bytes.start)?);
                let end = document
                    .rope
                    .char_to_utf16_cu(byte_to_char(&document.rope, span.bytes.end)?);
                placeholders.push(Placeholder {
                    index: span.index,
                    units: start..end,
                    authored_nonempty: start < end,
                    enclosing: span.enclosing.clone(),
                    transform: transform.clone(),
                    choices: choices.clone(),
                    context: 0,
                    owned_bytes: bytes,
                    owned_markers: markers,
                });
            }
            let mut indices = placeholders
                .iter()
                .map(|placeholder| placeholder.index)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            indices.sort_by_key(|index| (*index == Index::FINAL, *index));
            let groups = indices
                .into_iter()
                .map(|index| {
                    placeholders
                        .iter()
                        .enumerate()
                        .filter(|(_, placeholder)| placeholder.index == index)
                        .map(|(index, _)| index)
                        .collect()
                })
                .collect();
            snippets.push(Snippet {
                cursor_index: snippet.cursor_index,
                placeholders,
                groups,
                active_group: 0,
                contexts: vec![Context {
                    cursor_index: snippet.cursor_index,
                    line_leading_whitespace: snippet.line_leading_whitespace.clone(),
                }],
            });
        }
        Ok(Self {
            document: insertion.document,
            view: insertion.view,
            revision: insertion.revision,
            edit_group: UndoGroup(insertion.revision),
            primary_cursor,
            snippets,
            indent,
        })
    }

    fn nested_parents(
        &self,
        document: &DocumentSnapshot,
        selection: &SelectionSet,
    ) -> Result<Option<Vec<Vec<(usize, usize)>>>, EditorError> {
        let count = self
            .snippets
            .iter()
            .map(|snippet| snippet.groups[snippet.active_group].len())
            .sum::<usize>();
        if count != selection.selections.len() {
            return Ok(None);
        }
        let mut used = BTreeSet::new();
        let mut parents = Vec::new();
        for snippet in &self.snippets {
            let mut matches = Vec::new();
            for parent in &snippet.groups[snippet.active_group] {
                let range = byte_range(document, &snippet.placeholders[*parent].units)?;
                let cursor = selection
                    .selections
                    .iter()
                    .enumerate()
                    .find(|(cursor, selection)| {
                        !used.contains(cursor)
                            && range.start <= selection.anchor.min(selection.head)
                            && selection.anchor.max(selection.head) <= range.end
                    });
                let Some((cursor, _)) = cursor else {
                    return Ok(None);
                };
                used.insert(cursor);
                matches.push((*parent, cursor));
            }
            parents.push(matches);
        }
        Ok(Some(parents))
    }

    fn merge(
        &mut self,
        nested: Self,
        parents: Vec<Vec<(usize, usize)>>,
        limits: ParseLimits,
    ) -> Result<(), EditorError> {
        let mut children = nested
            .snippets
            .into_iter()
            .map(|snippet| (snippet.cursor_index, snippet))
            .collect::<BTreeMap<_, _>>();
        let mut orders = Vec::new();
        let mut active_orders = Vec::new();
        for (snippet, parents) in self.snippets.iter_mut().zip(parents) {
            let removed = parents
                .iter()
                .map(|(parent, _)| *parent)
                .collect::<BTreeSet<_>>();
            let mut mapping = BTreeMap::new();
            let mut placeholders = Vec::new();
            let mut keys = Vec::new();
            for (index, placeholder) in snippet.placeholders.iter().enumerate() {
                if removed.contains(&index)
                    || placeholder
                        .enclosing
                        .iter()
                        .any(|parent| removed.contains(parent))
                {
                    continue;
                }
                mapping.insert(index, placeholders.len());
                keys.push((placeholder.index, 0usize, false, Index::FINAL));
                placeholders.push(placeholder.clone());
            }
            for placeholder in &mut placeholders {
                placeholder.enclosing = placeholder
                    .enclosing
                    .iter()
                    .map(|parent| {
                        mapping
                            .get(parent)
                            .copied()
                            .ok_or(EditorError::InvalidBoundary)
                    })
                    .collect::<Result<_, _>>()?;
            }
            let mut active_order = None;
            for (parent, cursor) in parents {
                let parent = &snippet.placeholders[parent];
                let child = children
                    .remove(&cursor)
                    .ok_or(EditorError::InvalidBoundary)?;
                let context_offset = snippet.contexts.len();
                snippet.contexts.extend(child.contexts);
                let placeholder_offset = placeholders.len();
                let enclosing = parent
                    .enclosing
                    .iter()
                    .map(|index| {
                        mapping
                            .get(index)
                            .copied()
                            .ok_or(EditorError::InvalidBoundary)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                for mut placeholder in child.placeholders {
                    let key = (
                        parent.index,
                        1usize,
                        placeholder.index == Index::FINAL,
                        placeholder.index,
                    );
                    if placeholder.index != Index::FINAL {
                        active_order = Some(active_order.map_or(key, |previous| key.min(previous)));
                    }
                    placeholder.enclosing = placeholder
                        .enclosing
                        .iter()
                        .map(|index| {
                            index
                                .checked_add(placeholder_offset)
                                .ok_or(EditorError::Capacity)
                        })
                        .chain(enclosing.iter().copied().map(Ok))
                        .collect::<Result<_, _>>()?;
                    placeholder.context = placeholder
                        .context
                        .checked_add(context_offset)
                        .ok_or(EditorError::Capacity)?;
                    keys.push(key);
                    placeholders.push(placeholder);
                }
            }
            snippet.placeholders = placeholders;
            snippet.compact_contexts()?;
            active_orders.push(active_order.ok_or(EditorError::InvalidBoundary)?);
            orders.push(keys);
        }
        if !children.is_empty() {
            return Err(EditorError::InvalidBoundary);
        }
        let unique = orders
            .iter()
            .flatten()
            .copied()
            .filter(|key| key.0 != Index::FINAL)
            .collect::<BTreeSet<_>>();
        let indices = unique
            .into_iter()
            .enumerate()
            .map(|(ordinal, key)| {
                Ok((
                    key,
                    Index::ordered(ordinal.checked_add(1).ok_or(EditorError::Capacity)?)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, EditorError>>()?;
        let mut bytes = 0usize;
        let mut markers = 0usize;
        for ((snippet, orders), active) in self.snippets.iter_mut().zip(orders).zip(active_orders) {
            for context in &snippet.contexts {
                bytes = bytes
                    .checked_add(context.line_leading_whitespace.len())
                    .ok_or(EditorError::Capacity)?;
            }
            for (placeholder, order) in snippet.placeholders.iter_mut().zip(orders) {
                placeholder.index = if order.0 == Index::FINAL {
                    Index::FINAL
                } else {
                    *indices.get(&order).ok_or(EditorError::InvalidBoundary)?
                };
                bytes = bytes
                    .checked_add(placeholder.owned_bytes)
                    .ok_or(EditorError::Capacity)?;
                markers = markers
                    .checked_add(placeholder.owned_markers)
                    .ok_or(EditorError::Capacity)?;
                if placeholder.enclosing.len() > limits.max_nesting {
                    return Err(EditorError::Capacity);
                }
            }
            let active = *indices.get(&active).ok_or(EditorError::InvalidBoundary)?;
            snippet.regroup(active)?;
        }
        if bytes > limits.max_bytes || markers > limits.max_markers {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn selection(&self, document: &DocumentSnapshot) -> Result<SelectionSet, EditorError> {
        let mut selections = Vec::new();
        let mut primary = 0;
        for snippet in &self.snippets {
            if snippet.cursor_index == self.primary_cursor {
                primary = selections.len();
            }
            for index in &snippet.groups[snippet.active_group] {
                let bytes = byte_range(document, &snippet.placeholders[*index].units)?;
                selections.push(Selection {
                    anchor: bytes.start,
                    head: bytes.end,
                });
            }
        }
        Ok(SelectionSet {
            primary,
            selections,
        })
    }

    fn contains_selection(
        &self,
        document: &DocumentSnapshot,
        selections: &SelectionSet,
    ) -> Result<bool, EditorError> {
        if selections.selections.len() < self.snippets.len() {
            return Ok(false);
        }
        let Some(first) = self.snippets.first() else {
            return Ok(false);
        };
        let mut selections = selections
            .selections
            .iter()
            .map(|selection| {
                selection.anchor.min(selection.head)..selection.anchor.max(selection.head)
            })
            .collect::<Vec<_>>();
        selections.sort_by_key(|range| (range.start, range.end));
        for group in &first.groups {
            let index = first.placeholders[group[0]].index;
            if index == Index::FINAL {
                continue;
            }
            let mut ranges = Vec::new();
            for snippet in &self.snippets {
                for placeholder in &snippet.placeholders {
                    if placeholder.index == index {
                        ranges.push(byte_range(document, &placeholder.units)?);
                    }
                }
            }
            if ranges.len() != selections.len() {
                continue;
            }
            ranges.sort_by_key(|range| (range.start, range.end));
            if ranges.iter().zip(&selections).all(|(range, selection)| {
                range.start <= selection.start && selection.end <= range.end
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn map_edits(
        &mut self,
        document: &DocumentSnapshot,
        edits: &[Edit],
    ) -> Result<(), EditorError> {
        let mut edits = edits.iter().collect::<Vec<_>>();
        edits.sort_by_key(|edit| (edit.bytes.start, edit.bytes.end));
        for pair in edits.windows(2) {
            if pair[0].bytes.end > pair[1].bytes.start {
                return Err(EditorError::Overlap);
            }
        }
        for edit in edits.into_iter().rev() {
            if edit.bytes.start > edit.bytes.end {
                return Err(EditorError::InvalidBoundary);
            }
            let start = document
                .rope
                .char_to_utf16_cu(byte_to_char(&document.rope, edit.bytes.start)?);
            let end = document
                .rope
                .char_to_utf16_cu(byte_to_char(&document.rope, edit.bytes.end)?);
            let inserted = edit.text.encode_utf16().count();
            self.map_units(start..end, inserted)?;
        }
        Ok(())
    }

    fn map_units(&mut self, changed: Range<usize>, inserted: usize) -> Result<(), EditorError> {
        for snippet in &mut self.snippets {
            let mut active = BTreeSet::new();
            for index in &snippet.groups[snippet.active_group] {
                active.insert(*index);
                active.extend(snippet.placeholders[*index].enclosing.iter().copied());
            }
            let ending = snippet.groups[snippet.active_group].iter().any(|index| {
                let placeholder = &snippet.placeholders[*index];
                !placeholder.units.is_empty() && placeholder.units.end == changed.start
            });
            for (index, placeholder) in snippet.placeholders.iter_mut().enumerate() {
                let adjoining = changed.is_empty()
                    && ending
                    && !placeholder.units.is_empty()
                    && placeholder.units.start == changed.start;
                let grow = placeholder.index != Index::FINAL
                    && active.contains(&index)
                    && placeholder.units.start <= changed.start
                    && placeholder.units.end >= changed.end
                    && !adjoining;
                placeholder.units = map_utf16_range(
                    placeholder.units.clone(),
                    changed.clone(),
                    inserted,
                    grow,
                    false,
                )?;
            }
        }
        Ok(())
    }
}

impl Snippet {
    fn compact_contexts(&mut self) -> Result<(), EditorError> {
        let used = self
            .placeholders
            .iter()
            .map(|placeholder| placeholder.context)
            .collect::<BTreeSet<_>>();
        let mut contexts = Vec::new();
        let mut mapping = BTreeMap::new();
        for index in used {
            mapping.insert(index, contexts.len());
            contexts.push(
                self.contexts
                    .get(index)
                    .ok_or(EditorError::InvalidBoundary)?
                    .clone(),
            );
        }
        for placeholder in &mut self.placeholders {
            placeholder.context = *mapping
                .get(&placeholder.context)
                .ok_or(EditorError::InvalidBoundary)?;
        }
        self.contexts = contexts;
        Ok(())
    }

    fn regroup(&mut self, active: Index) -> Result<(), EditorError> {
        let mut groups = BTreeMap::<Index, Vec<usize>>::new();
        for (index, placeholder) in self.placeholders.iter().enumerate() {
            groups.entry(placeholder.index).or_default().push(index);
        }
        let final_group = groups.remove(&Index::FINAL);
        self.active_group = groups
            .keys()
            .position(|index| *index == active)
            .ok_or(EditorError::InvalidBoundary)?;
        self.groups = groups.into_values().collect();
        self.groups.extend(final_group);
        Ok(())
    }
}

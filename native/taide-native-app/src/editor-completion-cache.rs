use std::cell::RefCell;
use std::rc::Rc;

use taide_native_editor::completion::Candidate;
use taide_native_editor::completion_model::Model;
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness,
};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::lsp::{byte_to_position, range_to_bytes};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;
use tokio::sync::watch;
use uuid::Uuid;

use super::{Entry, Request, State, supply};

pub(super) fn leading(document: &DocumentSnapshot, byte: usize) -> String {
    let line = line_content_range(document, document.rope.byte_to_line(byte));
    document.rope.byte_slice(line.start..byte).to_string()
}

fn whitespace(value: &str) -> &str {
    let end = value
        .char_indices()
        .find(|(_, character)| !matches!(character, ' ' | '\t'))
        .map_or(value.len(), |(byte, _)| byte);
    &value[..end]
}

pub(super) fn rebase_candidates(
    store: &EditorStore,
    document: &DocumentSnapshot,
    byte: usize,
    candidates: &mut Vec<Candidate>,
) -> Result<(), EditorError> {
    let rebased = candidates
        .iter()
        .map(|candidate| {
            candidate.rebased(
                document,
                byte,
                store.changes_since(candidate.document, candidate.revision)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    *candidates = rebased;
    Ok(())
}

pub(super) fn refresh(entry: &mut Entry, store: &EditorStore) -> Result<bool, EditorError> {
    let request = &entry.request;
    if request.is_cancelled() {
        return Ok(false);
    }
    let Some(owner) = store.views().get(request.owner) else {
        return Ok(false);
    };
    let Some(source) = store.views().get(request.source) else {
        return Ok(false);
    };
    let document = store.documents().snapshot(source.document)?;
    if owner.key != request.owner_key
        || owner.document != request.owner_document
        || source.key != request.source_key
        || source.document != request.snapshot.id
        || document.key != request.snapshot.key
        || document.metadata.read_only
        || document.metadata.language_id != request.snapshot.metadata.language_id
        || source.composition.is_some()
        || source.selection.primary != request.selection.primary
        || source.selection.selections.len() != request.selection.selections.len()
    {
        return Ok(false);
    }
    let byte = source.selection.selections[source.selection.primary].head;
    let position = byte_to_position(&document, byte)?;
    let current_leading = leading(&document, byte);
    if position.line != request.position.line
        || whitespace(&current_leading) != whitespace(&entry.leading)
    {
        return Ok(false);
    }
    let word = supply::word_at(&document, byte)?;
    if word.is_empty() {
        return Ok(false);
    }
    let old_word = range_to_bytes(&request.snapshot, request.replace_word)?;
    let mut tracked = DecorationLayer::new(
        request.snapshot.revision,
        0,
        vec![Decoration {
            bytes: old_word,
            kind: DecorationKind::Inline(InlineStyle::default()),
            stickiness: Stickiness::AlwaysGrowsWhenTypingAtEdges,
        }],
    );
    let taide_native_editor::change_journal::ChangesSince::Tracked(changes) =
        store.changes_since(document.id, request.snapshot.revision)?
    else {
        return Ok(false);
    };
    for change in changes {
        if !tracked.apply(change) {
            return Ok(false);
        }
    }
    if word.start != tracked.items()[0].bytes.start {
        return Ok(false);
    }
    if entry.model.is_none() && entry.complete {
        let mut candidates = entry
            .groups
            .values()
            .flat_map(|group| &group.candidates.items)
            .cloned()
            .collect::<Vec<_>>();
        candidates.extend(entry.snippets.iter().cloned());
        let fallback_pending = candidates.is_empty() && entry.word_result.is_some();
        if candidates.is_empty() && !fallback_pending {
            candidates.extend(entry.words.iter().cloned());
        }
        if !fallback_pending {
            entry.model = Some(Rc::new(RefCell::new(Model::new(
                &request.snapshot,
                candidates,
            )?)));
            entry.origin_position = request.position;
        }
    }
    if let Some(model) = &entry.model {
        model.borrow_mut().rebase(
            &document,
            byte,
            store.changes_since(document.id, request.snapshot.revision)?,
        )?;
    }
    let incomplete = entry
        .groups
        .values()
        .filter(|group| group.candidates.is_incomplete && !group.candidates.items.is_empty())
        .map(|group| group.provider)
        .collect::<std::collections::HashSet<_>>();
    let restart = entry.model.is_none() || position.character < entry.origin_position.character;
    let requery = restart
        || !entry.complete
        || (position.character > request.position.character && !incomplete.is_empty());
    let mut next = request.clone();
    next.snapshot = document;
    next.selection = source.selection.clone();
    next.position = position;
    next.word = taide_native_editor::lsp::LspRange::new(
        byte_to_position(&next.snapshot, word.start)?,
        position,
    );
    next.replace_word = taide_native_editor::lsp::LspRange::new(
        next.word.start,
        byte_to_position(&next.snapshot, word.end)?,
    );
    next.token = Uuid::new_v4();
    let (cancel, cancelled) = watch::channel(false);
    next.cancelled = cancelled;
    if requery {
        next.query_providers = if restart {
            next.providers.clone()
        } else if !entry.complete {
            request.query_providers.clone()
        } else {
            incomplete
        };
        entry.complete = false;
        entry.queued = true;
    }
    if entry.model.is_none() {
        entry.supplied = false;
        entry.words.clear();
    }
    entry.word_result = None;
    entry.cancel.send_replace(true);
    entry.clipboard_pending = false;
    entry.cancel = cancel;
    entry.request = next;
    entry.leading = current_leading;
    entry.delta = (i64::from(position.character) - i64::from(entry.origin_position.character))
        .try_into()
        .map_err(|_| EditorError::Capacity)?;
    Ok(entry.request.describes(store))
}

impl State {
    pub(crate) fn refresh_view(
        &mut self,
        store: &EditorStore,
        view: ViewId,
    ) -> Result<bool, EditorError> {
        if let Some(choice) = self.entries.get(&view).and_then(|entry| entry.choice) {
            let valid = self.snippet_sessions.get_mut(&view).is_some_and(|snippet| {
                snippet.session.synchronize(store)
                    && snippet
                        .session
                        .active_choice(store)
                        .is_ok_and(|active| active.is_some_and(|active| active.index == choice))
            });
            if !valid {
                if self
                    .snippet_sessions
                    .get(&view)
                    .is_some_and(|snippet| !snippet.session.is_active())
                {
                    self.snippet_sessions.remove(&view);
                }
                self.close(view);
                return Ok(false);
            }
        }
        let Some(entry) = self.entries.get_mut(&view) else {
            return Ok(false);
        };
        if entry.request.describes(store) {
            return Ok(true);
        }
        match refresh(entry, store) {
            Ok(true) => return Ok(true),
            Err(error) => {
                self.close(view);
                return Err(error);
            }
            Ok(false) => {}
        }
        self.close(view);
        Ok(false)
    }

    pub(crate) fn queue(&mut self, view: ViewId) {
        if let Some(entry) = self.entries.get_mut(&view) {
            entry.queued = true;
        }
    }

    pub(crate) fn take_queued(&mut self) -> Vec<Request> {
        self.entries
            .values_mut()
            .filter_map(|entry| std::mem::take(&mut entry.queued).then(|| entry.request.clone()))
            .collect()
    }

    pub(crate) fn display(&self, store: &EditorStore, view: ViewId) -> Option<(Uuid, &str, isize)> {
        self.request(store, view)?;
        let entry = self.entries.get(&view)?;
        Some((entry.widget_token, &entry.leading, entry.delta))
    }
}

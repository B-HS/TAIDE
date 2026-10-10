use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use taide_lsp::native::protocol::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, InsertReplaceEdit,
    InsertTextFormat,
};
use taide_model::snippet::SnippetFile;
use taide_native_editor::completion::Candidate;
use taide_native_editor::completion_model::Model;
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::language_configuration::LanguageRules;
use taide_native_editor::lsp::range_to_bytes;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

use super::{Request, State};

const WORD_LIMIT: usize = 10_000;
const MODEL_SYNC_UTF16_LIMIT: usize = 50 * 1024 * 1024;
const WORKER_NAME: &str = "native-editor-completion-words";

pub(super) fn needs_clipboard(candidate: &Candidate) -> bool {
    candidate.is_snippet()
        && (candidate.text().contains("$CLIPBOARD") || candidate.text().contains("${CLIPBOARD"))
}

pub(super) fn language(id: &str) -> Option<&'static taide_native_syntax::MonacoLanguage> {
    taide_native_syntax::monaco_language(id)
        .ok()
        .flatten()
        .or_else(|| {
            taide_native_syntax::monaco_language("plaintext")
                .ok()
                .flatten()
        })
}

pub(super) fn word_at(
    document: &DocumentSnapshot,
    byte: usize,
) -> Result<std::ops::Range<usize>, EditorError> {
    taide_native_editor::lsp::byte_to_position(document, byte)?;
    let line = line_content_range(document, document.rope.byte_to_line(byte));
    let text = document.rope.byte_slice(line.clone()).to_string();
    let word = language(&document.metadata.language_id)
        .and_then(|language| language.word_range(&text, byte - line.start));
    Ok(word.map_or(byte..byte, |word| {
        line.start + word.start..line.start + word.end
    }))
}

fn snippets(request: &Request, files: &[SnippetFile]) -> Vec<Candidate> {
    taide_native_ui::snippet_completion::collect(files, &request.snapshot.metadata.language_id)
        .into_iter()
        .filter_map(|snippet| {
            Candidate::new(
                &request.snapshot,
                request.position,
                request.word,
                CompletionItem {
                    label: snippet.prefix,
                    kind: Some(CompletionItemKind::SNIPPET),
                    detail: Some(snippet.name),
                    documentation: snippet.description.map(Documentation::String),
                    insert_text: Some(snippet.body),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                },
            )
        })
        .collect()
}

fn word_documents(store: &EditorStore, request: &Request) -> Vec<DocumentSnapshot> {
    let mut documents = store
        .documents()
        .versions()
        .filter(|version| {
            version.language_id == request.snapshot.metadata.language_id
                && store.views().for_document(version.id).next().is_some()
        })
        .filter_map(|version| store.documents().snapshot(version.id).ok())
        .collect::<Vec<_>>();
    documents.sort_unstable_by_key(|document| (document.id != request.snapshot.id, document.id));
    documents
}

fn words(
    request: &Request,
    documents: &[DocumentSnapshot],
    word_limit: usize,
    sync_utf16_limit: usize,
) -> Vec<Candidate> {
    if request.is_cancelled() || word_limit == 0 {
        return Vec::new();
    }
    let language = language(&request.snapshot.metadata.language_id);
    let Some(language) = language else {
        return Vec::new();
    };
    let Ok(range) = range_to_bytes(&request.snapshot, request.replace_word) else {
        return Vec::new();
    };
    let leading_word = request.snapshot.rope.byte_slice(range).to_string();
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for document in documents {
        if request.is_cancelled() {
            return Vec::new();
        }
        if document.rope.len_utf16_cu() > sync_utf16_limit {
            continue;
        }
        for line in 0..document.rope.len_lines() {
            if request.is_cancelled() {
                return Vec::new();
            }
            let text = document
                .rope
                .byte_slice(line_content_range(document, line))
                .to_string();
            language.visit_words(&text, |range| {
                if request.is_cancelled() || items.len() >= word_limit || range.is_empty() {
                    return false;
                }
                let word = &text[range];
                if word == leading_word || is_number(word) || !seen.insert(word.to_owned()) {
                    return true;
                }
                if let Some(candidate) = Candidate::new(
                    &request.snapshot,
                    request.position,
                    request.word,
                    CompletionItem {
                        label: word.to_owned(),
                        kind: Some(CompletionItemKind::TEXT),
                        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
                            new_text: word.to_owned(),
                            insert: request.word,
                            replace: request.replace_word,
                        })),
                        ..Default::default()
                    },
                ) {
                    items.push(candidate);
                }
                items.len() < word_limit
            });
            if items.len() >= word_limit {
                return items;
            }
        }
    }
    if request.is_cancelled() {
        return Vec::new();
    }
    items
}

pub(super) fn is_number(text: &str) -> bool {
    let text = text.trim_matches(|character: char| {
        character == '\u{feff}' || (character.is_whitespace() && character != '\u{85}')
    });
    if text.is_empty() {
        return true;
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
    ] {
        if let Some(digits) = text.strip_prefix(prefix) {
            return !digits.is_empty()
                && digits
                    .chars()
                    .all(|character| character.is_ascii() && character.is_digit(radix));
        }
    }
    let text = text
        .strip_prefix('+')
        .or_else(|| text.strip_prefix('-'))
        .unwrap_or(text);
    if text == "Infinity" {
        return true;
    }
    let bytes = text.as_bytes();
    let mut index = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    let mut has_digits = index > 0;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        has_digits |= index > start;
    }
    if !has_digits {
        return false;
    }
    if bytes
        .get(index)
        .is_some_and(|character| matches!(character, b'e' | b'E'))
    {
        index += 1;
        if bytes
            .get(index)
            .is_some_and(|character| matches!(character, b'+' | b'-'))
        {
            index += 1;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if start == index {
            return false;
        }
    }
    index == bytes.len()
}

impl State {
    pub(crate) fn supply(
        &mut self,
        store: &EditorStore,
        request: &Request,
        files: &[SnippetFile],
        tasks: &TaskSupervisor,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> bool {
        if !self.is_current(request) || !request.describes(store) {
            return false;
        }
        let entry = self.entries.get_mut(&request.source).unwrap();
        if entry.supplied {
            return false;
        }
        entry.supplied = true;
        entry.model = None;
        entry.documents.clear();
        entry.snippets = snippets(request, files);
        entry.update_clipboard_requirement();
        let documents = word_documents(store, request);
        let request = request.clone();
        let (sender, receiver) = oneshot::channel();
        let worker = tasks.spawn_blocking_transient_handle(WORKER_NAME, move || {
            let candidates = words(&request, &documents, WORD_LIMIT, MODEL_SYNC_UTF16_LIMIT);
            drop(sender.send(candidates));
            repaint();
        });
        if worker.is_some() {
            entry.word_result = Some(receiver);
        }
        true
    }

    pub(crate) fn poll_supply(&mut self, store: &EditorStore) -> bool {
        let mut changed = false;
        for entry in self.entries.values_mut() {
            let Some(receiver) = &mut entry.word_result else {
                continue;
            };
            let words = match receiver.try_recv() {
                Ok(words) => words,
                Err(oneshot::error::TryRecvError::Empty) => continue,
                Err(oneshot::error::TryRecvError::Closed) => Vec::new(),
            };
            entry.word_result = None;
            if entry.request.describes(store) {
                entry.words = words;
                if entry.snippets.is_empty()
                    && entry
                        .groups
                        .values()
                        .all(|group| group.candidates.items.is_empty())
                {
                    entry.model = None;
                }
                changed = true;
            }
        }
        changed
    }

    pub(crate) fn candidates(&self, store: &EditorStore, view: ViewId) -> Option<Vec<&Candidate>> {
        self.request(store, view)?;
        let entry = self.entries.get(&view)?;
        if !entry.complete {
            return Some(Vec::new());
        }
        let mut primary = entry
            .groups
            .values()
            .flat_map(|group| &group.candidates.items)
            .collect::<Vec<_>>();
        primary.extend(&entry.snippets);
        if primary.is_empty() {
            primary.extend(&entry.words);
        }
        Some(primary)
    }

    pub(crate) fn model(
        &mut self,
        store: &EditorStore,
        view: ViewId,
    ) -> Result<Option<Rc<RefCell<Model>>>, EditorError> {
        if self.request(store, view).is_none()
            || self
                .entries
                .get(&view)
                .is_some_and(|entry| entry.needs_clipboard && entry.clipboard.is_none())
            || (self.pending(store, view)
                && self
                    .entries
                    .get(&view)
                    .is_none_or(|entry| entry.model.is_none()))
        {
            return Ok(None);
        }
        if self
            .entries
            .get(&view)
            .is_some_and(|entry| entry.model.is_none())
        {
            let candidates = self
                .candidates(store, view)
                .unwrap()
                .into_iter()
                .cloned()
                .collect();
            let entry = self.entries.get_mut(&view).unwrap();
            let model = Model::new(&entry.request.snapshot, candidates)?;
            entry.model = Some(Rc::new(RefCell::new(model)));
            entry.origin_position = entry.request.position;
            entry.delta = 0;
        }
        Ok(self
            .entries
            .get(&view)
            .and_then(|entry| entry.model.clone()))
    }
}

#[cfg(test)]
#[path = "editor-completion-supply-tests.rs"]
mod tests;

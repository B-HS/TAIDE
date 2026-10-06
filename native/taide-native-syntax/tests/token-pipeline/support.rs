use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use serde_json::json;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::theme::ResolvedTheme;
use taide_native_editor::document::{DocumentId, DocumentSnapshot, Edit, UndoGroup};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_syntax::{
    TextmateTokenizer, TokenTheme, TokenizerLimits, WorkerClient, WorkerRequest,
    bundled_grammar_set, token_worker,
};

pub const TIMEOUT: Duration = Duration::from_secs(60);
pub const QUIET_PERIOD: Duration = Duration::from_millis(200);
pub const CORE_LANGUAGE_IDS: [&str; 3] = ["json", "jsonc", "markdown"];
const DOCUMENT_COUNT: usize = 8;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 8;
const DOCUMENT_BYTE_LIMIT: usize = 64 * 1024 * 1024;

pub struct Worker {
    pub wake: Receiver<()>,
    pub finished: Receiver<()>,
}

pub fn spawn_worker() -> (WorkerClient, Worker) {
    spawn_worker_with(Vec::new())
}

pub fn spawn_worker_with(queued_requests: Vec<WorkerRequest>) -> (WorkerClient, Worker) {
    let (wake_sender, wake) = mpsc::channel();
    let (finished_sender, finished) = mpsc::channel();
    let (client, task) = token_worker(Arc::new(move || {
        wake_sender.send(()).ok();
    }));
    for request in queued_requests {
        client.send(request).unwrap();
    }
    thread::spawn(move || {
        task.run();
        finished_sender.send(()).ok();
    });
    (client, Worker { wake, finished })
}

pub fn dark_theme() -> ResolvedTheme {
    serde_json::from_value(json!({
        "id": "synthetic-dark",
        "name": "Synthetic Dark",
        "type": "dark",
        "colors": { "editor.foreground": "#d4d4d4", "editor.background": "#1e1e1e" },
        "syntax": {},
        "terminal": {},
        "tokenColors": [
            { "scope": ["comment"], "settings": { "foreground": "#6a9955" } },
            { "scope": ["string"], "settings": { "foreground": "#ce9178" } },
            { "scope": ["keyword", "storage"], "settings": { "foreground": "#569cd6" } },
            { "scope": ["constant.numeric"], "settings": { "foreground": "#b5cea8" } },
            { "scope": ["entity.name.function"], "settings": { "foreground": "#dcdcaa" } },
        ],
    }))
    .unwrap()
}

pub fn light_theme() -> ResolvedTheme {
    serde_json::from_value(json!({
        "id": "synthetic-light",
        "name": "Synthetic Light",
        "type": "light",
        "colors": { "editor.foreground": "#24292e", "editor.background": "#ffffff" },
        "syntax": {},
        "terminal": {},
        "tokenColors": [
            { "scope": ["comment"], "settings": { "foreground": "#6a737d", "fontStyle": "italic" } },
            { "scope": ["string"], "settings": { "foreground": "#032f62" } },
            { "scope": ["keyword"], "settings": { "foreground": "#d73a49", "fontStyle": "bold" } },
        ],
    }))
    .unwrap()
}

pub fn token_theme(theme: &ResolvedTheme) -> TokenTheme {
    TokenTheme::from_resolved(theme).unwrap()
}

pub fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: DOCUMENT_BYTE_LIMIT,
    })
    .unwrap()
}

pub fn open(store: &mut EditorStore, path: &str, language_id: &str, content: &str) -> DocumentId {
    store
        .open_file(
            PathBuf::from(path),
            OpenedFile {
                path: path.into(),
                content: content.into(),
                language_id: language_id.into(),
                byte_size: content.len().try_into().unwrap(),
                line_count: content.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap()
}

pub fn snapshot(store: &EditorStore, document: DocumentId) -> DocumentSnapshot {
    store.documents().snapshot(document).unwrap()
}

pub fn text(store: &EditorStore, document: DocumentId) -> String {
    snapshot(store, document).rope.to_string()
}

pub fn apply(store: &mut EditorStore, document: DocumentId, edits: Vec<(Range<usize>, &str)>) {
    let revision = snapshot(store, document).revision;
    store
        .apply_separate(
            document,
            Transaction {
                revision,
                edits: edits
                    .into_iter()
                    .map(|(bytes, text)| Edit {
                        bytes,
                        text: text.into(),
                    })
                    .collect(),
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

pub fn tokenizer(
    requested_language_ids: &[&str],
    theme: &TokenTheme,
    limits: TokenizerLimits,
) -> TextmateTokenizer {
    TextmateTokenizer::new(
        &bundled_grammar_set(requested_language_ids).unwrap(),
        theme.settings(),
        limits,
    )
    .unwrap()
}

pub fn direct_spans(
    requested_language_ids: &[&str],
    theme: &TokenTheme,
    limits: TokenizerLimits,
    language_id: &str,
    content: &str,
) -> Vec<Vec<u32>> {
    let mut tokenizer = tokenizer(requested_language_ids, theme, limits);
    let mut state = None;
    content
        .split('\n')
        .map(|line| {
            let tokenized = tokenizer
                .try_tokenize_line(language_id, line, state.as_ref())
                .unwrap();
            state = Some(tokenized.end_state);
            tokenized.spans
        })
        .collect()
}

pub fn repeated_lines(line: &str, count: usize) -> String {
    vec![line; count].join("\n")
}

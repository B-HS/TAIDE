use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::paths::AppPaths;
use taide_model::theme::ResolvedTheme;
use taide_native_app::editor_syntax::EditorSyntax;
use taide_native_app::save;
use taide_native_editor::document::{DocumentId, Edit, UndoGroup};
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_syntax::MAX_TOKENIZED_DOCUMENT_LINES;
use taide_plugin::service::PluginStore;
use taide_runtime::{AppState, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(60);
const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 2 * 1024 * 1024;
const PATH: &str = "/synthetic/save-syntax.rs";
const SOURCE: &str = "let a = 1;  \n// note \t\nlet s = \"open  \nclosed\"; \nlet b = 2; \t";
const TRIMMED: &str = "let a = 1;\n// note\nlet s = \"open  \nclosed\";\nlet b = 2;";
const EDITED_LINE: &str = "// note";
const EDITED_PREFIX: &str = "/";
const FIRST_LINE_TRIMMED: &str =
    "let a = 1;\n/// note \t\nlet s = \"open  \nclosed\"; \nlet b = 2; \t";
const EDITED_TRIMMED: &str = "let a = 1;\n/// note\nlet s = \"open  \nclosed\";\nlet b = 2;";
const PLUGIN_ID: &str = "taide-ini-plugin";
const PLUGIN_LANGUAGE_ID: &str = "taide-ini";
const PLUGIN_MANIFEST_FILE: &str = "taide-plugin.json";
const PLUGIN_GRAMMAR_FILE: &str = "grammar.json";
const PLUGIN_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-ini",
    "patterns": [
        { "match": ";.*$", "name": "comment.line.semicolon.taide-ini" },
        { "begin": "\"", "end": "\"", "name": "string.quoted.double.taide-ini" }
    ]
}"##;
const PLUGIN_PATH: &str = "/synthetic/save-syntax.taide-ini";
const PLUGIN_SOURCE: &str = "key = 1  \n; note \t\nname = \"open  \nclosed\" \nlast = 2 \t";
const PLUGIN_TRIMMED: &str = "key = 1\n; note\nname = \"open  \nclosed\"\nlast = 2";
const UNTOKENIZED_LINE: &str = "a \n";
const UNTOKENIZED_LAST_LINE: &str = "\"open  ";
const TRIM: CleanupFlags = CleanupFlags {
    trim_trailing_whitespace: true,
    insert_final_newline: false,
};
const KEEP: CleanupFlags = CleanupFlags {
    trim_trailing_whitespace: false,
    insert_final_newline: false,
};

fn theme() -> ResolvedTheme {
    serde_json::from_value(json!({
        "id": "synthetic-dark",
        "name": "synthetic-dark",
        "type": "dark",
        "colors": { "editor.foreground": "#d4d4d4", "editor.background": "#1e1e1e" },
        "syntax": {},
        "terminal": {},
        "tokenColors": [
            { "scope": ["comment"], "settings": { "foreground": "#6a9955" } },
            { "scope": ["string"], "settings": { "foreground": "#ce9178" } },
            { "scope": ["keyword", "storage"], "settings": { "foreground": "#569cd6" } },
        ],
    }))
    .unwrap()
}

fn open(content: &str) -> (EditorStore, DocumentId, ViewId) {
    open_language(PATH, "rust", content)
}

fn open_language(
    path: &str,
    language_id: &str,
    content: &str,
) -> (EditorStore, DocumentId, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            path.into(),
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
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    (store, document, view)
}

fn insert(store: &mut EditorStore, document: DocumentId, at: usize, text: &str) {
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply(
            document,
            Transaction {
                revision,
                edits: vec![Edit {
                    bytes: at..at,
                    text: text.into(),
                }],
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

fn open_dirty(content: &str) -> (EditorStore, DocumentId, ViewId) {
    open_dirty_language(PATH, "rust", content)
}

fn open_dirty_language(
    path: &str,
    language_id: &str,
    content: &str,
) -> (EditorStore, DocumentId, ViewId) {
    let (first, rest) = content.split_at(1);
    let (mut store, document, view) = open_language(path, language_id, rest);
    insert(&mut store, document, 0, first);
    (store, document, view)
}

struct PluginHost {
    directory: PathBuf,
    state: AppState,
    plugins: PluginStore,
}

impl PluginHost {
    fn with_ini_plugin() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-save-syntax-{}", TabId::new()));
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let root = state.paths.plugins_dir().join(PLUGIN_ID);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(PLUGIN_GRAMMAR_FILE), PLUGIN_GRAMMAR).unwrap();
        let manifest = json!({
            "manifestVersion": 1,
            "id": PLUGIN_ID,
            "name": PLUGIN_ID,
            "version": "1.0.0",
            "contributes": {
                "languages": [{
                    "id": PLUGIN_LANGUAGE_ID,
                    "extensions": [".taide-ini"],
                    "grammar": PLUGIN_GRAMMAR_FILE,
                }],
            },
        });
        std::fs::write(root.join(PLUGIN_MANIFEST_FILE), manifest.to_string()).unwrap();
        Self {
            directory,
            state,
            plugins: PluginStore::new(),
        }
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn prepare(
    store: &mut EditorStore,
    document: DocumentId,
    view: ViewId,
    flags: CleanupFlags,
) -> (bool, String) {
    let requested = store.save_snapshot(document).unwrap();
    let prepared = save::prepare(store, requested, Some(view), flags, false)
        .unwrap()
        .unwrap();
    (prepared.changed, prepared.snapshot.rope().to_string())
}

struct Coordinator {
    syntax: EditorSyntax,
    theme: ResolvedTheme,
    ready: Arc<Notify>,
    tasks: TaskSupervisor,
}

impl Coordinator {
    fn connect() -> Self {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        Self {
            syntax: EditorSyntax::connect(&tasks, Arc::new(move || signal.notify_one())).unwrap(),
            theme: theme(),
            ready,
            tasks,
        }
    }

    async fn tokenize(&mut self, store: &mut EditorStore, document: DocumentId) {
        self.tokenize_following(None, store, document).await;
    }

    async fn tokenize_following(
        &mut self,
        host: Option<&PluginHost>,
        store: &mut EditorStore,
        document: DocumentId,
    ) {
        let line_count = store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .len_lines();
        tokio::time::timeout(TIMEOUT, async {
            loop {
                if let Some(host) = host {
                    self.syntax
                        .follow_plugins(&host.state, &host.plugins, &self.tasks);
                }
                self.syntax.tick(store, &self.theme, Instant::now());
                self.syntax.supply_save_cleanup(store, document, TRIM);
                let is_tokenized = store
                    .syntax(document)
                    .unwrap()
                    .is_some_and(|syntax| syntax.lines.len() == line_count);
                if is_tokenized {
                    return;
                }
                self.ready.notified().await;
            }
        })
        .await
        .unwrap();
    }

    async fn disconnect(mut self) {
        let worker = self.syntax.disconnect().unwrap();
        tokio::time::timeout(TIMEOUT, worker)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(self.tasks.tracked_count(), 0);
    }
}

fn block_on(scenario: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(scenario);
}

#[test]
fn 저장_정리는_문자열_안_후행_공백을_남기고_주석_뒤는_지우며_설정이_꺼지면_그대로_둔다() {
    block_on(async {
        let mut coordinator = Coordinator::connect();
        let (mut store, document, view) = open_dirty(SOURCE);
        coordinator.tokenize(&mut store, document).await;
        coordinator
            .syntax
            .supply_save_cleanup(&mut store, document, KEEP);
        assert_eq!(
            prepare(&mut store, document, view, KEEP),
            (false, SOURCE.into())
        );
        coordinator
            .syntax
            .supply_save_cleanup(&mut store, document, TRIM);
        assert_eq!(
            prepare(&mut store, document, view, TRIM),
            (true, TRIMMED.into())
        );
        coordinator
            .syntax
            .supply_save_cleanup(&mut store, document, KEEP);
        assert!(store.syntax(document).unwrap().is_none());
        coordinator.disconnect().await;
    });
}

#[test]
fn 저장_정리는_아직_다시_토큰화되지_않은_줄을_건너뛰고_토큰화가_끝난_뒤에_지운다() {
    block_on(async {
        let mut coordinator = Coordinator::connect();
        let (mut store, document, view) = open_dirty(SOURCE);
        coordinator.tokenize(&mut store, document).await;
        insert(
            &mut store,
            document,
            SOURCE.find(EDITED_LINE).unwrap(),
            EDITED_PREFIX,
        );
        coordinator
            .syntax
            .supply_save_cleanup(&mut store, document, TRIM);
        assert_eq!(
            store
                .syntax(document)
                .unwrap()
                .map(|syntax| syntax.lines.len()),
            Some(1)
        );
        assert_eq!(
            prepare(&mut store, document, view, TRIM),
            (true, FIRST_LINE_TRIMMED.into())
        );
        coordinator.tokenize(&mut store, document).await;
        assert_eq!(
            prepare(&mut store, document, view, TRIM),
            (true, EDITED_TRIMMED.into())
        );
        coordinator.disconnect().await;
    });
}

#[test]
fn 플러그인_문법_언어의_저장_정리도_문자열_안_후행_공백을_남기고_주석_뒤는_지운다() {
    block_on(async {
        let host = PluginHost::with_ini_plugin();
        let mut coordinator = Coordinator::connect();
        let (mut store, document, view) =
            open_dirty_language(PLUGIN_PATH, PLUGIN_LANGUAGE_ID, PLUGIN_SOURCE);
        coordinator
            .syntax
            .tick(&store, &coordinator.theme, Instant::now());
        coordinator
            .syntax
            .supply_save_cleanup(&mut store, document, TRIM);
        assert!(store.syntax(document).unwrap().is_none());
        assert_eq!(
            prepare(&mut store, document, view, TRIM),
            (false, PLUGIN_SOURCE.into())
        );

        coordinator
            .tokenize_following(Some(&host), &mut store, document)
            .await;
        assert_eq!(
            prepare(&mut store, document, view, TRIM),
            (true, PLUGIN_TRIMMED.into())
        );
        coordinator.disconnect().await;
    });
}

#[test]
fn 토큰화_한도를_넘는_문서의_저장_정리는_모든_줄을_일반_토큰으로_본다() {
    block_on(async {
        let mut coordinator = Coordinator::connect();
        let content = format!(
            "{}{UNTOKENIZED_LAST_LINE}",
            UNTOKENIZED_LINE.repeat(MAX_TOKENIZED_DOCUMENT_LINES)
        );
        let (mut store, document, _) = open(&content);
        coordinator
            .syntax
            .tick(&store, &coordinator.theme, Instant::now());
        coordinator
            .syntax
            .supply_save_cleanup(&mut store, document, TRIM);
        let syntax = store.syntax(document).unwrap().unwrap();
        assert_eq!(syntax.lines.len(), MAX_TOKENIZED_DOCUMENT_LINES + 1);
        assert_eq!(
            syntax.token_at(MAX_TOKENIZED_DOCUMENT_LINES, UNTOKENIZED_LAST_LINE.len()),
            Some(TokenKind::Other)
        );
        coordinator.disconnect().await;
    });
}

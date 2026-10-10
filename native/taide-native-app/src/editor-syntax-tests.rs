use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::paths::AppPaths;
use taide_model::theme::ResolvedTheme;
use taide_native_editor::document::{DocumentId, DocumentMetadata, Edit, UndoGroup};
use taide_native_editor::language_configuration::LineSyntax;
use taide_native_editor::line_tokens::{LineTokens, TokenStyleTable};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::syntax::{Token, TokenKind};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_syntax::{
    PluginGrammar, TextmateTokenizer, TokenTheme, TokenizerLimits, UNSTYLED_STYLE_ID,
    bundled_grammar_set, token_worker,
};
use taide_plugin::service::PluginStore;
use taide_runtime::{AppState, TaskSupervisor, plugin_actions};

use super::{EditorSyntax, SyntaxLease, language_rules};

const TIMEOUT: Duration = Duration::from_secs(60);
const DOCUMENT_COUNT: usize = 8;
const VIEW_COUNT: usize = 8;
const HISTORY_COUNT: usize = 8;
const DOCUMENT_BYTE_LIMIT: usize = 1024 * 1024;
const CORE_AND_RUST: [&str; 4] = ["json", "jsonc", "markdown", "rust"];
const RUST_SOURCE: &str =
    "fn main() {\n    let value = 1; // one\n    let text = \"a\";\n}\n\nfn other() {}";
const RELOADED_SOURCE: &str = "// reloaded\nfn reloaded() -> &'static str {\n    \"text\"\n}";
const COMMENT_FOREGROUND_KEY: &str = "comment";
const RUST_COMMENT_LINE: usize = 1;
const RUST_STRING_LINE: usize = 2;
const RUST_EMPTY_LINE: usize = 4;
const REINDENT_WIDTH: u32 = 4;

struct Harness {
    syntax: EditorSyntax,
    wake: Receiver<()>,
    finished: Receiver<()>,
}

impl Harness {
    fn new() -> Self {
        let (wake_sender, wake) = mpsc::channel();
        let (finished_sender, finished) = mpsc::channel();
        let (client, task) = token_worker(Arc::new(move || {
            wake_sender.send(()).ok();
        }));
        std::thread::spawn(move || {
            task.run();
            finished_sender.send(()).ok();
        });
        Self {
            syntax: EditorSyntax::new(client, None),
            wake,
            finished,
        }
    }

    fn settle(&mut self, store: &EditorStore, theme: &ResolvedTheme, now: Instant) {
        loop {
            self.syntax.tick(store, theme, now);
            if self.syntax.pipeline.is_settled() {
                return;
            }
            self.wake.recv_timeout(TIMEOUT).unwrap();
        }
    }

    fn invalid_lines(&self, document: DocumentId) -> Vec<(usize, usize)> {
        self.syntax
            .pipeline
            .tokens(document)
            .unwrap()
            .invalid_ranges()
            .iter()
            .map(|range| (range.start, range.end))
            .collect()
    }

    fn spans(&self, document: DocumentId) -> Vec<Vec<u32>> {
        let tokens = self.syntax.pipeline.tokens(document).unwrap();
        assert_eq!(tokens.first_invalid_line(), None);
        (0..tokens.line_count())
            .map(|line| tokens.spans(line).to_vec())
            .collect()
    }
}

fn theme(id: &str, comment_foreground: &str) -> ResolvedTheme {
    theme_with_rules(
        id,
        json!([
            { "scope": [COMMENT_FOREGROUND_KEY], "settings": { "foreground": comment_foreground } },
            { "scope": ["string"], "settings": { "foreground": "#ce9178" } },
            { "scope": ["keyword", "storage"], "settings": { "foreground": "#569cd6" } },
        ]),
    )
}

fn theme_with_rules(id: &str, token_colors: Value) -> ResolvedTheme {
    serde_json::from_value(json!({
        "id": id,
        "name": id,
        "type": "dark",
        "colors": { "editor.foreground": "#d4d4d4", "editor.background": "#1e1e1e" },
        "syntax": {},
        "terminal": {},
        "tokenColors": token_colors,
    }))
    .unwrap()
}

fn tokenizer(theme: &ResolvedTheme) -> (TokenTheme, TextmateTokenizer) {
    let token_theme = TokenTheme::from_resolved(theme).unwrap();
    let tokenizer = TextmateTokenizer::new(
        &bundled_grammar_set(&CORE_AND_RUST).unwrap(),
        token_theme.settings(),
        TokenizerLimits::default(),
    )
    .unwrap();
    (token_theme, tokenizer)
}

fn expected_table(theme: &ResolvedTheme) -> TokenStyleTable {
    let (token_theme, tokenizer) = tokenizer(theme);
    token_theme.style_table(&tokenizer).unwrap()
}

fn expected_spans(
    theme: &ResolvedTheme,
    store: &EditorStore,
    document: DocumentId,
) -> Vec<Vec<u32>> {
    let snapshot = store.documents().snapshot(document).unwrap();
    let (_, mut tokenizer) = tokenizer(theme);
    let mut state = None;
    snapshot
        .rope
        .to_string()
        .split('\n')
        .map(|line| {
            let tokenized = tokenizer
                .try_tokenize_line(&snapshot.metadata.language_id, line, state.as_ref())
                .unwrap();
            state = Some(tokenized.end_state);
            tokenized.spans
        })
        .collect()
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_COUNT,
        max_views: VIEW_COUNT,
        max_undo_groups: HISTORY_COUNT,
        max_document_bytes: DOCUMENT_BYTE_LIMIT,
    })
    .unwrap()
}

#[test]
fn 재들여쓰기는_본문과_peek의_미준비_구문에서도_준비된_문자열_내용을_보존한다() {
    use taide_native_editor::indent::{
        Command, IndentConfiguration, IndentOptions, run_command_with_language,
    };
    use taide_native_editor::language_configuration::Language;

    const SOURCE: &str = "if value\ntext = <<~TEXT\nif {\nTEXT\nwork\nend";
    let configuration = IndentConfiguration {
        defaults: IndentOptions {
            tab_size: REINDENT_WIDTH,
            insert_spaces: true,
        },
        detect_indentation: false,
    };
    let theme = theme("first", "#6a9955");
    let now = Instant::now();
    let mut harness = Harness::new();
    let mut store = store();
    let warm = open(&mut store, "/synthetic/warm.rb", "ruby", SOURCE);
    let warm_view = attach(&mut store, warm);
    harness.settle(&store, &theme, now);
    let rules = language_rules("ruby").unwrap();
    let lease = SyntaxLease::new(&mut harness.syntax, warm);
    assert!(
        run_command_with_language(
            &mut store,
            warm_view,
            Command::ReindentLines,
            configuration,
            Some(Language {
                rules,
                syntax: &lease
            })
        )
        .unwrap()
    );
    drop(lease);
    let expected = store.documents().snapshot(warm).unwrap().rope.to_string();
    assert!(expected.contains("\nif {\nTEXT\n"), "{expected}");
    for peek in [false, true] {
        let path = if peek {
            "/synthetic/peek.rb"
        } else {
            "/synthetic/cold.rb"
        };
        let document = open(&mut store, path, "ruby", SOURCE);
        let view = attach(&mut store, document);
        harness.syntax.follow_documents(&store);
        assert_eq!(
            harness
                .syntax
                .pipeline
                .tokens(document)
                .unwrap()
                .first_invalid_line(),
            Some(0)
        );
        let changed = if peek {
            let tokens = harness.syntax.peek_tokens(&store, document).unwrap();
            run_command_with_language(
                &mut store,
                view,
                Command::ReindentLines,
                configuration,
                Some(Language {
                    rules,
                    syntax: tokens.as_ref(),
                }),
            )
        } else {
            let lease = SyntaxLease::new(&mut harness.syntax, document);
            run_command_with_language(
                &mut store,
                view,
                Command::ReindentLines,
                configuration,
                Some(Language {
                    rules,
                    syntax: &lease,
                }),
            )
        };
        assert_eq!(changed, Ok(true));
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            expected,
            "peek={peek}"
        );
        harness.settle(&store, &theme, now);
        let retained = harness.syntax.peek_tokens(&store, document).unwrap();
        assert!(store.undo(document).unwrap());
        harness.syntax.catch_up(&store, document);
        assert!(
            harness
                .syntax
                .pipeline
                .tokens(document)
                .unwrap()
                .first_invalid_line()
                .is_some()
        );
        let changed = if peek {
            run_command_with_language(
                &mut store,
                view,
                Command::ReindentLines,
                configuration,
                Some(Language {
                    rules,
                    syntax: retained.as_ref(),
                }),
            )
        } else {
            let lease = SyntaxLease::new(&mut harness.syntax, document);
            run_command_with_language(
                &mut store,
                view,
                Command::ReindentLines,
                configuration,
                Some(Language {
                    rules,
                    syntax: &lease,
                }),
            )
        };
        assert_eq!(changed, Ok(true));
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            expected,
            "partial peek={peek}"
        );
    }
    harness.syntax.disconnect();
    harness.finished.recv_timeout(TIMEOUT).unwrap();
}

#[test]
fn 재들여쓰기는_최초_peek의_구문을_준비하고_보관한_port가_워커_종료를_막지_않는다() {
    use taide_native_editor::document::EditorError;
    use taide_native_editor::indent::{
        Command, IndentConfiguration, IndentOptions, run_command_with_language,
    };
    use taide_native_editor::language_configuration::Language;

    const SOURCE: &str = "if value\ntext = <<~TEXT\nif {\nTEXT\nwork\nend";
    const EXPECTED: &str = "if value\n    text = <<~TEXT\nif {\nTEXT\n    work\nend";
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/first.rb", "ruby", SOURCE);
    let view = attach(&mut store, document);
    harness
        .syntax
        .follow_theme(&theme("first", "#6a9955"), Instant::now());
    harness.syntax.follow_documents(&store);
    let tokens = harness.syntax.peek_tokens(&store, document).unwrap();
    assert!(tokens.frame().is_none());
    assert!(
        run_command_with_language(
            &mut store,
            view,
            Command::ReindentLines,
            IndentConfiguration {
                defaults: IndentOptions {
                    tab_size: REINDENT_WIDTH,
                    insert_spaces: true
                },
                detect_indentation: false,
            },
            Some(Language {
                rules: language_rules("ruby").unwrap(),
                syntax: tokens.as_ref()
            })
        )
        .unwrap()
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        EXPECTED
    );
    assert!(store.undo(document).unwrap());
    let current = store.views().get(view).unwrap().clone();
    let selection = taide_native_editor::view::SelectionSet {
        primary: 0,
        selections: vec![taide_native_editor::view::Selection {
            anchor: SOURCE.find("if {").unwrap(),
            head: SOURCE.find("work").unwrap(),
        }],
    };
    store
        .set_view_state(view, selection.clone(), current.scroll, current.folds)
        .unwrap();
    assert!(
        !run_command_with_language(
            &mut store,
            view,
            Command::ReindentSelectedLines,
            IndentConfiguration {
                defaults: IndentOptions {
                    tab_size: REINDENT_WIDTH,
                    insert_spaces: true
                },
                detect_indentation: false,
            },
            Some(Language {
                rules: language_rules("ruby").unwrap(),
                syntax: tokens.as_ref()
            })
        )
        .unwrap()
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        SOURCE
    );
    assert_eq!(store.views().get(view).unwrap().selection, selection);
    assert!(!store.undo(document).unwrap());
    harness.syntax.disconnect();
    harness.finished.recv_timeout(TIMEOUT).unwrap();
    let snapshot = store.documents().snapshot(document).unwrap();
    assert!(matches!(
        tokens.prepare_tokens(&snapshot, 0..snapshot.rope.len_lines()),
        Err(EditorError::Refused)
    ));
    let mut readonly = file("/synthetic/first.rb", "ruby", SOURCE);
    readonly.read_only = true;
    store
        .observe_file(
            document,
            std::path::Path::new("/synthetic/first.rb"),
            readonly,
        )
        .unwrap();
    let before = store.documents().snapshot(document).unwrap();
    let selected = store.views().get(view).unwrap().selection.clone();
    assert_eq!(
        run_command_with_language(
            &mut store,
            view,
            Command::ReindentLines,
            IndentConfiguration {
                defaults: IndentOptions {
                    tab_size: REINDENT_WIDTH,
                    insert_spaces: true
                },
                detect_indentation: false,
            },
            Some(Language {
                rules: language_rules("ruby").unwrap(),
                syntax: tokens.as_ref()
            })
        ),
        Err(EditorError::ReadOnly)
    );
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.rope, before.rope);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.indent_options, before.indent_options);
    assert_eq!(store.views().get(view).unwrap().selection, selected);
    let mut target = file("/synthetic/first.txt", "plaintext", SOURCE);
    target.editor_config = EditorConfigOptions::default();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from("/synthetic/first.txt"),
            DocumentMetadata::from_opened(&target),
        )
        .unwrap();
    let changed_language = store.documents().snapshot(document).unwrap();
    assert!(matches!(
        tokens.prepare_tokens(&changed_language, 0..changed_language.rope.len_lines()),
        Err(EditorError::StaleRevision)
    ));
}

#[test]
fn 문서_도움말_코드는_본문_textmate와_테마를_쓰고_닫힌_소스만_회수한다() {
    use eframe::egui::{Color32, FontId};
    use taide_native_editor::documentation::{Block, ListItem, RichDocument};
    use taide_native_ui::editor_surface::EditorAppearance;

    const FONT_SIZE: f32 = 14.0;
    const LINE_HEIGHT: f32 = 20.0;
    const THEME_WAIT: Duration = Duration::from_secs(1);
    let mut harness = Harness::new();
    let mut store = store();
    let source = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    let view = attach(&mut store, source);
    apply(&mut store, source, 0..0, "// dirty\n");
    let source_snapshot = store.documents().snapshot(source).unwrap();
    let mut cache = crate::editor_documentation_code::Cache::default();
    let document = RichDocument {
        blocks: vec![
            Block::Quote(vec![Block::Code {
                language: "Rust".into(),
                text: RUST_SOURCE.into(),
            }]),
            Block::List {
                start: None,
                items: vec![ListItem {
                    checked: None,
                    blocks: vec![Block::Code {
                        language: String::new(),
                        text: RUST_SOURCE.into(),
                    }],
                }],
            },
            Block::Code {
                language: "js".into(),
                text: "const value = 1;".into(),
            },
            Block::Code {
                language: "unknown language".into(),
                text: "plain text".into(),
            },
        ],
    };
    let prepare = |cache: &mut crate::editor_documentation_code::Cache,
                   store: &mut EditorStore,
                   syntax: &mut EditorSyntax| {
        cache
            .prepare(store, syntax, std::iter::once((&document, "rust")), &[])
            .unwrap()
    };
    prepare(&mut cache, &mut store, &mut harness.syntax);
    let ids = store
        .documents()
        .versions()
        .map(|version| version.id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 4);
    assert!(
        store
            .documents()
            .versions()
            .any(|version| version.language_id == "javascript")
    );
    assert!(
        store
            .documents()
            .versions()
            .any(|version| version.language_id == "plaintext")
    );
    let now = Instant::now();
    let first = theme("first", "#6a9955");
    harness.settle(&store, &first, now);
    prepare(&mut cache, &mut store, &mut harness.syntax);
    assert_eq!(
        ids,
        store
            .documents()
            .versions()
            .map(|version| version.id)
            .collect()
    );
    let appearance = EditorAppearance {
        font: FontId::monospace(FONT_SIZE),
        line_height: LINE_HEIGHT,
        horizontal_padding: 0.0,
        background: Color32::BLACK,
        foreground: Color32::WHITE,
        muted: Color32::GRAY,
        selection: Color32::BLUE,
        cursor: Color32::WHITE,
        current_line: Color32::TRANSPARENT,
        line_numbers: false,
        indent: "    ".into(),
    };
    let job = cache.job("Rust", RUST_SOURCE, "rust", &appearance).unwrap();
    assert_eq!(job.text, RUST_SOURCE);
    assert!(
        job.sections
            .iter()
            .any(|section| section.format.color == Color32::from_rgb(0x6a, 0x99, 0x55))
    );
    assert!(
        job.sections
            .iter()
            .any(|section| section.format.color == Color32::from_rgb(0x56, 0x9c, 0xd6))
    );
    assert_eq!(
        cache
            .job("", RUST_SOURCE, "rust", &appearance)
            .unwrap()
            .sections,
        job.sections
    );
    assert_eq!(
        cache
            .job("unknown language", "plain text", "rust", &appearance)
            .unwrap()
            .text,
        "plain text"
    );
    let second = theme("second", "#222222");
    harness.syntax.tick(&store, &second, now + THEME_WAIT);
    harness.settle(&store, &second, now + THEME_WAIT * 2);
    prepare(&mut cache, &mut store, &mut harness.syntax);
    let updated = cache.job("Rust", RUST_SOURCE, "rust", &appearance).unwrap();
    assert!(
        updated
            .sections
            .iter()
            .any(|section| section.format.color == Color32::from_rgb(0x22, 0x22, 0x22))
    );
    cache
        .prepare(&mut store, &mut harness.syntax, std::iter::empty(), &[])
        .unwrap();
    harness.settle(&store, &second, now + THEME_WAIT * 2);
    assert_eq!(
        store
            .documents()
            .versions()
            .map(|version| version.id)
            .collect::<Vec<_>>(),
        vec![source]
    );
    assert_eq!(store.views().get(view).unwrap().document, source);
    assert_eq!(
        store.documents().snapshot(source).unwrap().rope,
        source_snapshot.rope
    );
    assert!(store.documents().snapshot(source).unwrap().dirty);
    assert!(
        ids.iter()
            .filter(|document| **document != source)
            .all(|document| !harness.syntax.pipeline.contains(*document))
    );
    harness.syntax.disconnect();
    harness.finished.recv_timeout(TIMEOUT).unwrap();
}

fn file(path: &str, language_id: &str, content: &str) -> OpenedFile {
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
    }
}

fn open(store: &mut EditorStore, path: &str, language_id: &str, content: &str) -> DocumentId {
    store
        .open_file(PathBuf::from(path), file(path, language_id, content))
        .unwrap()
}

fn attach(store: &mut EditorStore, document: DocumentId) -> ViewId {
    store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap()
}

fn apply(store: &mut EditorStore, document: DocumentId, bytes: Range<usize>, text: &str) {
    let revision = store.documents().snapshot(document).unwrap().revision;
    store
        .apply_separate(
            document,
            Transaction {
                revision,
                edits: vec![Edit {
                    bytes,
                    text: text.into(),
                }],
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

#[test]
fn 뷰가_붙은_번들_언어_문서만_추적해_토큰화한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let shown = open(&mut store, "/synthetic/shown.rs", "rust", RUST_SOURCE);
    let hidden = open(&mut store, "/synthetic/hidden.rs", "rust", RUST_SOURCE);
    let plain = open(&mut store, "/synthetic/notes.txt", "plaintext", "plain");
    attach(&mut store, shown);
    attach(&mut store, plain);
    let now = Instant::now();
    harness.settle(&store, &theme, now);
    assert!(harness.syntax.pipeline.contains(shown));
    assert!(!harness.syntax.pipeline.contains(hidden));
    assert!(!harness.syntax.pipeline.contains(plain));
    assert_eq!(harness.spans(shown), expected_spans(&theme, &store, shown));
    assert_eq!(
        harness.syntax.pipeline.style_table(),
        Some(&expected_table(&theme))
    );

    attach(&mut store, hidden);
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(hidden),
        expected_spans(&theme, &store, hidden)
    );
}

#[test]
fn peek_토큰은_arc를_재사용하고_숨은_미리보기_편집_테마_언어와_폐기를_추적한다() {
    let first_theme = theme("peek-first", "#6a9955");
    let second_theme = theme("peek-second", "#222222");
    let mut harness = Harness::new();
    let mut store = store();
    let shown = open(&mut store, "/synthetic/source.rs", "rust", RUST_SOURCE);
    attach(&mut store, shown);
    let hidden = open(&mut store, "/synthetic/peek.rs", "rust", RUST_SOURCE);
    let now = Instant::now();
    harness.settle(&store, &first_theme, now);
    harness.syntax.peek_tokens(&store, hidden);
    harness.settle(&store, &first_theme, now);
    let before = harness.syntax.peek_tokens(&store, hidden).unwrap();
    let repeated = harness.syntax.peek_tokens(&store, hidden).unwrap();
    assert!(Arc::ptr_eq(&before, &repeated));
    assert_eq!(
        line_spans(before.frame().unwrap().lines),
        expected_spans(&first_theme, &store, hidden)
    );
    assert!(
        before
            .tokens(
                &store.documents().snapshot(hidden).unwrap(),
                RUST_COMMENT_LINE
            )
            .unwrap()
            .iter()
            .any(|token| token.kind == taide_native_editor::syntax::TokenKind::Comment)
    );
    apply(&mut store, hidden, 0..0, "// inserted\n");
    let edited = harness.syntax.peek_tokens(&store, hidden).unwrap();
    assert_eq!(edited.revision, revision(&store, hidden));
    assert!(!Arc::ptr_eq(&before, &edited));
    assert!(
        before
            .tokens(&store.documents().snapshot(hidden).unwrap(), 0)
            .is_none()
    );
    harness.settle(&store, &first_theme, now);
    let edited = harness.syntax.peek_tokens(&store, hidden).unwrap();
    assert_eq!(
        line_spans(edited.frame().unwrap().lines),
        expected_spans(&first_theme, &store, hidden)
    );
    harness.settle(&store, &second_theme, now + Duration::from_secs(1));
    let recolored = harness.syntax.peek_tokens(&store, hidden).unwrap();
    assert!(!Arc::ptr_eq(&edited, &recolored));
    assert_eq!(
        *recolored.frame().unwrap().styles,
        expected_table(&second_theme)
    );
    let path = "/synthetic/peek.json";
    let snapshot = store.documents().snapshot(hidden).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(path),
            DocumentMetadata::from_opened(&file(path, "json", RUST_SOURCE)),
        )
        .unwrap();
    let changed_language = store.documents().snapshot(hidden).unwrap();
    assert!(recolored.tokens(&changed_language, 0).is_none());
    let changed = harness.syntax.peek_tokens(&store, hidden).unwrap();
    assert_eq!(changed.language_id, "json");
    assert!(!Arc::ptr_eq(&recolored, &changed));
    store
        .discard_document(hidden, revision(&store, hidden))
        .unwrap();
    harness
        .syntax
        .tick(&store, &second_theme, now + Duration::from_secs(1));
    assert!(!harness.syntax.peek_tokens.contains_key(&hidden));
    assert!(harness.syntax.peek_tokens(&store, hidden).is_none());
}

#[test]
fn 편집은_저널로_이어가고_저널이_끊긴_변경은_문서_전체를_다시_토큰화한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let path = "/synthetic/main.rs";
    let document = open(&mut store, path, "rust", RUST_SOURCE);
    attach(&mut store, document);
    let now = Instant::now();
    harness.settle(&store, &theme, now);

    let value = RUST_SOURCE.find("value").unwrap();
    apply(
        &mut store,
        document,
        value..value + "value".len(),
        "renamed",
    );
    apply(&mut store, document, 0..0, "// top\n");
    harness.syntax.tick(&store, &theme, now);
    let tokens = harness.syntax.pipeline.tokens(document).unwrap();
    assert_eq!(tokens.line_count(), 7);
    assert_eq!(harness.invalid_lines(document), [(0, 3)]);
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(document),
        expected_spans(&theme, &store, document)
    );

    assert!(store.undo(document).unwrap());
    harness.syntax.tick(&store, &theme, now);
    assert_eq!(harness.invalid_lines(document), [(0, 1)]);
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(document),
        expected_spans(&theme, &store, document)
    );

    let saved = store.save_snapshot(document).unwrap();
    store.mark_saved(saved, None).unwrap();
    store
        .refresh_clean_file(
            document,
            &PathBuf::from(path),
            file(path, "rust", RELOADED_SOURCE),
        )
        .unwrap();
    harness.syntax.tick(&store, &theme, now);
    assert_eq!(harness.invalid_lines(document), [(0, 4)]);
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(document),
        expected_spans(&theme, &store, document)
    );
}

#[test]
fn 닫힌_문서와_번들_밖_언어로_바뀐_문서는_추적에서_뺀다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let closed = open(&mut store, "/synthetic/closed.rs", "rust", RUST_SOURCE);
    let renamed = open(&mut store, "/synthetic/renamed.rs", "rust", RUST_SOURCE);
    let closed_view = attach(&mut store, closed);
    attach(&mut store, renamed);
    let now = Instant::now();
    harness.settle(&store, &theme, now);
    assert!(harness.syntax.pipeline.contains(closed));
    assert!(harness.syntax.pipeline.contains(renamed));

    store.detach_view(closed_view).unwrap();
    harness.syntax.tick(&store, &theme, now);
    assert!(harness.syntax.pipeline.contains(closed));
    store.discard_document(closed, 0).unwrap();
    harness.syntax.tick(&store, &theme, now);
    assert!(!harness.syntax.pipeline.contains(closed));
    assert!(harness.syntax.pipeline.contains(renamed));

    let plain_path = "/synthetic/renamed.txt";
    let snapshot = store.documents().snapshot(renamed).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(plain_path),
            DocumentMetadata::from_opened(&file(plain_path, "plaintext", RUST_SOURCE)),
        )
        .unwrap();
    harness.syntax.tick(&store, &theme, now);
    assert!(!harness.syntax.pipeline.contains(renamed));

    let json_path = "/synthetic/renamed.json";
    let snapshot = store.documents().snapshot(renamed).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(json_path),
            DocumentMetadata::from_opened(&file(json_path, "json", RUST_SOURCE)),
        )
        .unwrap();
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(renamed),
        expected_spans(&theme, &store, renamed)
    );

    let rust_path = "/synthetic/renamed-again.rs";
    let snapshot = store.documents().snapshot(renamed).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(rust_path),
            DocumentMetadata::from_opened(&file(rust_path, "rust", RUST_SOURCE)),
        )
        .unwrap();
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(renamed),
        expected_spans(&theme, &store, renamed)
    );
}

#[test]
fn 한_tick_안에_닫힌_문서와_언어가_바뀐_문서와_새_문서를_함께_정리한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let closed = open(&mut store, "/synthetic/closed.rs", "rust", RUST_SOURCE);
    let renamed = open(&mut store, "/synthetic/renamed.rs", "rust", RUST_SOURCE);
    let kept = open(&mut store, "/synthetic/kept.rs", "rust", RUST_SOURCE);
    let closed_view = attach(&mut store, closed);
    attach(&mut store, renamed);
    let kept_view = attach(&mut store, kept);
    let now = Instant::now();
    harness.settle(&store, &theme, now);

    store.detach_view(closed_view).unwrap();
    store.discard_document(closed, 0).unwrap();
    let plain_path = "/synthetic/renamed.txt";
    let snapshot = store.documents().snapshot(renamed).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(plain_path),
            DocumentMetadata::from_opened(&file(plain_path, "plaintext", RUST_SOURCE)),
        )
        .unwrap();
    harness.syntax.tick(&store, &theme, now);
    assert!(!harness.syntax.pipeline.contains(closed));
    assert!(!harness.syntax.pipeline.contains(renamed));
    assert!(harness.syntax.pipeline.contains(kept));

    let added = open(&mut store, "/synthetic/added.rs", "rust", RUST_SOURCE);
    attach(&mut store, added);
    store.detach_view(kept_view).unwrap();
    store.discard_document(kept, 0).unwrap();
    harness.settle(&store, &theme, now);
    assert!(!harness.syntax.pipeline.contains(kept));
    assert_eq!(harness.spans(added), expected_spans(&theme, &store, added));
}

#[test]
fn 연속된_테마_변경은_150ms_뒤에_마지막_테마로_한_번만_다시_적용한다() {
    let first = theme("first", "#6a9955");
    let skipped = theme("skipped", "#111111");
    let last = theme("last", "#222222");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    attach(&mut store, document);
    let start = Instant::now();
    harness.settle(&store, &first, start);
    assert_eq!(
        harness.syntax.pipeline.style_table(),
        Some(&expected_table(&first))
    );

    let step = Duration::from_millis(50);
    assert_eq!(
        harness.syntax.tick(&store, &skipped, start + step),
        Some(Duration::from_millis(150))
    );
    assert_eq!(
        harness.syntax.tick(&store, &last, start + step * 2),
        Some(Duration::from_millis(150))
    );
    assert_eq!(
        harness.syntax.tick(&store, &last, start + step * 4),
        Some(step)
    );
    assert!(harness.syntax.pipeline.is_settled());
    assert_eq!(
        harness.syntax.pipeline.style_table(),
        Some(&expected_table(&first))
    );

    assert_eq!(harness.syntax.tick(&store, &last, start + step * 5), None);
    assert!(!harness.syntax.pipeline.is_settled());
    harness.settle(&store, &last, start + step * 5);
    assert_eq!(
        harness.syntax.pipeline.style_table(),
        Some(&expected_table(&last))
    );
    assert_ne!(expected_table(&last), expected_table(&skipped));
    assert_eq!(
        harness.spans(document),
        expected_spans(&last, &store, document)
    );
}

#[test]
fn 거절된_테마는_이전_스타일_표를_남기고_다음_테마는_다시_적용한다() {
    let valid = theme("valid", "#6a9955");
    let rejected = theme("rejected", "#0083080");
    let next = theme("next", "#333333");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    attach(&mut store, document);
    let start = Instant::now();
    harness.settle(&store, &valid, start);

    let later = start + Duration::from_secs(1);
    assert_eq!(harness.syntax.tick(&store, &rejected, later), None);
    assert!(harness.syntax.pipeline.is_settled());
    assert_eq!(
        harness.syntax.pipeline.style_table(),
        Some(&expected_table(&valid))
    );
    assert_eq!(
        harness.spans(document),
        expected_spans(&valid, &store, document)
    );

    let latest = later + Duration::from_secs(1);
    harness.settle(&store, &next, latest);
    assert_eq!(
        harness.syntax.pipeline.style_table(),
        Some(&expected_table(&next))
    );
}

#[test]
fn 연결을_끊으면_worker가_끝나고_이후_tick은_토큰을_바꾸지_않는다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    attach(&mut store, document);
    let now = Instant::now();
    harness.settle(&store, &theme, now);
    assert!(harness.syntax.disconnect().is_none());
    harness.finished.recv_timeout(TIMEOUT).unwrap();
    apply(&mut store, document, 0..0, "/* open\n");
    harness.syntax.tick(&store, &theme, now);
    assert!(!harness.syntax.pipeline.is_worker_running());
    assert_eq!(
        harness
            .syntax
            .pipeline
            .tokens(document)
            .unwrap()
            .first_invalid_line(),
        Some(0)
    );
}

#[test]
fn worker는_task_supervisor에_등록되고_연결을_끊거나_버리면_끝난다() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let mut syntax = EditorSyntax::connect(&tasks, Arc::new(|| {})).unwrap();
    assert_eq!(tasks.tracked_count(), 1);
    let worker = syntax.disconnect().unwrap();
    runtime
        .block_on(async { tokio::time::timeout(TIMEOUT, worker).await })
        .unwrap()
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(syntax.disconnect().is_none());

    let dropped = EditorSyntax::connect(&tasks, Arc::new(|| {})).unwrap();
    assert_eq!(tasks.tracked_count(), 1);
    drop(dropped);
    runtime
        .block_on(async { tokio::time::timeout(TIMEOUT, tasks.shutdown()).await })
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(EditorSyntax::connect(&tasks, Arc::new(|| {})).is_err());
}

fn revision(store: &EditorStore, document: DocumentId) -> u64 {
    store.documents().snapshot(document).unwrap().revision
}

fn line_spans(tokens: &LineTokens) -> Vec<Vec<u32>> {
    (0..tokens.line_count())
        .map(|line| tokens.spans(line).to_vec())
        .collect()
}

#[test]
fn 화면에_빌려주는_토큰은_편집_직후에도_현재_revision의_줄에_맞고_재토큰화를_바로_시작한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    attach(&mut store, document);
    let now = Instant::now();
    harness.settle(&store, &theme, now);
    let settled = harness.spans(document);

    apply(&mut store, document, 0..0, "// top\n");
    let tokens = harness.syntax.tokens(&store, document).unwrap();
    assert_eq!(tokens.revision, revision(&store, document));
    assert_eq!(*tokens.styles, expected_table(&theme));
    assert_eq!(tokens.lines.first_invalid_line(), Some(0));
    let shifted = line_spans(tokens.lines);
    assert_eq!(shifted.len(), settled.len() + 1);
    assert_eq!(shifted[2..], settled[1..]);
    assert!(!harness.syntax.pipeline.is_settled());
    harness.settle(&store, &theme, now);
    assert_eq!(
        harness.spans(document),
        expected_spans(&theme, &store, document)
    );
}

#[test]
fn 입력_경로에_빌려주는_줄_토큰은_표준_종류를_알리고_따라잡기_전의_revision은_모른다고_답한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    let plain = open(&mut store, "/synthetic/note.txt", "plaintext", "a {\n");
    attach(&mut store, document);
    attach(&mut store, plain);
    harness.settle(&store, &theme, Instant::now());
    let snapshot = store.documents().snapshot(document).unwrap();
    let plain_snapshot = store.documents().snapshot(plain).unwrap();
    let comment_line = RUST_SOURCE.lines().nth(RUST_COMMENT_LINE).unwrap();
    let inside_comment = comment_line.find("//").unwrap() + 1;
    {
        let lease = SyntaxLease::new(&mut harness.syntax, plain);
        let syntax: &dyn LineSyntax = &lease;
        assert_eq!(
            syntax.tokens(&plain_snapshot, 0),
            Some(vec![Token {
                start_byte: 0,
                kind: TokenKind::Other
            }])
        );
        assert_eq!(syntax.tokens(&plain_snapshot, 1), Some(Vec::new()));
        assert_eq!(
            syntax.accurate_tokens(&plain_snapshot, 0),
            syntax.tokens(&plain_snapshot, 0)
        );
    }
    let lease = SyntaxLease::new(&mut harness.syntax, document);
    let syntax: &dyn LineSyntax = &lease;
    let kinds = |line: usize| -> Vec<TokenKind> {
        let tokens = syntax.tokens(&snapshot, line).unwrap();
        tokens.iter().map(|token| token.kind).collect()
    };
    assert_eq!(kinds(RUST_COMMENT_LINE).last(), Some(&TokenKind::Comment));
    assert!(kinds(RUST_STRING_LINE).contains(&TokenKind::String));
    assert_eq!(kinds(RUST_EMPTY_LINE), []);
    assert_eq!(
        syntax.accurate_tokens(&snapshot, RUST_STRING_LINE),
        syntax.tokens(&snapshot, RUST_STRING_LINE)
    );
    assert_eq!(
        syntax.kind_if_inserting(&snapshot, RUST_COMMENT_LINE, inside_comment, '0'),
        TokenKind::Comment
    );
    assert_eq!(
        syntax.kind_if_inserting(&snapshot, RUST_COMMENT_LINE, 0, '0'),
        TokenKind::Other
    );

    apply(&mut store, document, 0..0, "x");
    let edited = store.documents().snapshot(document).unwrap();
    assert_eq!(syntax.tokens(&edited, 0), None);
    syntax.follow_edits(&store);
    assert!(syntax.tokens(&edited, 0).is_some());
    assert_eq!(syntax.accurate_tokens(&edited, 0), None);
    assert_eq!(
        lease.frame_tokens(&store).unwrap().revision,
        edited.revision
    );
    assert_eq!(syntax.tokens(&edited, 0), None);
    assert_eq!(
        syntax.kind_if_inserting(&edited, RUST_COMMENT_LINE, inside_comment, '0'),
        TokenKind::Other
    );

    assert!(language_rules("rust").is_some());
    assert!(language_rules("plaintext").is_some());
    assert!(language_rules("toml").is_none());
    assert!(language_rules("taide-unknown").is_none());
}

#[test]
fn 테마가_다시_적용되거나_언어가_바뀐_문서는_이전_토큰을_새_스타일_표로_빌려주지_않는다() {
    let first = theme("first", "#6a9955");
    let second = theme("second", "#222222");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    attach(&mut store, document);
    let start = Instant::now();
    harness.settle(&store, &first, start);
    let settled = harness.spans(document);

    let later = start + Duration::from_secs(1);
    let untokenized: Vec<Vec<u32>> = vec![Vec::new(); settled.len()];
    let second_table = expected_table(&second);
    harness.syntax.tick(&store, &second, later);
    let tokens = harness.syntax.tokens(&store, document).unwrap();
    assert_eq!(*tokens.styles, expected_table(&first));
    assert_eq!(line_spans(tokens.lines), settled);
    while harness.syntax.pipeline.style_table() != Some(&second_table) {
        harness.wake.recv_timeout(TIMEOUT).unwrap();
        harness.syntax.tick(&store, &second, later);
    }
    let tokens = harness.syntax.tokens(&store, document).unwrap();
    assert_eq!(*tokens.styles, second_table);
    assert_eq!(tokens.lines.first_invalid_line(), Some(0));
    assert_eq!(line_spans(tokens.lines), untokenized);
    harness.settle(&store, &second, later);
    assert_eq!(
        harness.spans(document),
        expected_spans(&second, &store, document)
    );

    let json_path = "/synthetic/main.json";
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(json_path),
            DocumentMetadata::from_opened(&file(json_path, "json", RUST_SOURCE)),
        )
        .unwrap();
    let tokens = harness.syntax.tokens(&store, document).unwrap();
    assert_eq!(tokens.revision, revision(&store, document));
    assert_eq!(line_spans(tokens.lines), untokenized);

    let plain_path = "/synthetic/main.txt";
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .retarget_file(
            &snapshot,
            PathBuf::from(plain_path),
            DocumentMetadata::from_opened(&file(plain_path, "plaintext", RUST_SOURCE)),
        )
        .unwrap();
    assert!(harness.syntax.tokens(&store, document).is_none());
    assert!(!harness.syntax.pipeline.contains(document));
}

#[test]
fn 토큰은_번들_밖_언어와_닫힌_문서에는_없고_처음_보인_문서는_tick_없이_추적을_시작한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let closed = open(&mut store, "/synthetic/closed.rs", "rust", RUST_SOURCE);
    let plain = open(&mut store, "/synthetic/notes.txt", "plaintext", "plain");
    let closed_view = attach(&mut store, closed);
    attach(&mut store, plain);
    assert!(harness.syntax.tokens(&store, closed).is_none());
    let now = Instant::now();
    harness.settle(&store, &theme, now);
    assert!(harness.syntax.tokens(&store, closed).is_some());
    assert!(harness.syntax.tokens(&store, plain).is_none());

    store.detach_view(closed_view).unwrap();
    store.discard_document(closed, 0).unwrap();
    assert!(harness.syntax.tokens(&store, closed).is_none());
    assert!(!harness.syntax.pipeline.contains(closed));

    let added = open(&mut store, "/synthetic/added.rs", "rust", RUST_SOURCE);
    attach(&mut store, added);
    let tokens = harness.syntax.tokens(&store, added).unwrap();
    assert_eq!(tokens.revision, revision(&store, added));
    assert_eq!(tokens.lines.first_invalid_line(), Some(0));
    assert!(!harness.syntax.pipeline.is_settled());
    harness.settle(&store, &theme, now);
    assert_eq!(harness.spans(added), expected_spans(&theme, &store, added));
}

#[test]
fn 화면이_알린_보이는_줄은_다음_tick까지_합쳐_두었다가_한_번_전달한다() {
    let theme = theme("dark", "#6a9955");
    let mut harness = Harness::new();
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    let plain = open(&mut store, "/synthetic/notes.txt", "plaintext", "plain");
    attach(&mut store, document);
    attach(&mut store, plain);
    let now = Instant::now();
    harness.settle(&store, &theme, now);

    let [lower, upper] = [4..6, 1..3];
    harness.syntax.show_lines(document, lower.clone());
    harness.syntax.show_lines(document, upper.clone());
    harness.syntax.show_lines(plain, upper.clone());
    assert_eq!(
        harness.syntax.documents[&document].shown_lines,
        Some(upper.start..lower.end)
    );
    assert!(!harness.syntax.documents.contains_key(&plain));
    harness.syntax.tick(&store, &theme, now);
    assert_eq!(harness.syntax.documents[&document].shown_lines, None);
}

const PLUGIN_MANIFEST_FILE: &str = "taide-plugin.json";
const FIRST_PLUGIN_VERSION: &str = "1.0.0";
const SECOND_PLUGIN_VERSION: &str = "1.0.1";
const INI_PLUGIN_ID: &str = "taide-ini-plugin";
const INI_LANGUAGE_ID: &str = "taide-ini";
const INI_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-ini",
    "patterns": [
        { "match": ";.*$", "name": "comment.line.semicolon.taide-ini" },
        { "begin": "\"", "end": "\"", "name": "string.quoted.double.taide-ini" }
    ]
}"##;
const INI_HASH_COMMENT_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-ini",
    "patterns": [{ "match": "#.*$", "name": "comment.line.number-sign.taide-ini" }]
}"##;
const INI_SOURCE: &str = "; note\nkey = \"open\nclosed\" # hash";
const UNPARSABLE_GRAMMAR: &str = "{";
const UNREAD_GRAMMAR: &str = r##"{ "scopeName": "source.taide-unread", "patterns": [] }"##;
const CYCLE_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-cycle",
    "patterns": [{ "include": "#a" }],
    "repository": {
        "a": { "patterns": [{ "include": "#b" }] },
        "b": { "patterns": [{ "include": "#a" }] }
    }
}"##;

struct PluginHost {
    runtime: tokio::runtime::Runtime,
    directory: PathBuf,
    state: AppState,
    plugins: PluginStore,
    tasks: TaskSupervisor,
}

impl PluginHost {
    fn new() -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let directory =
            std::env::temp_dir().join(format!("taide-native-editor-syntax-{}", TabId::new()));
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let tasks = TaskSupervisor::new(runtime.handle().clone());
        Self {
            runtime,
            directory,
            state,
            plugins: PluginStore::new(),
            tasks,
        }
    }

    fn plugin_root(&self, plugin_id: &str) -> PathBuf {
        self.state.paths.plugins_dir().join(plugin_id)
    }

    fn install(
        &self,
        plugin_id: &str,
        version: &str,
        language_id: &str,
        grammar: &str,
        language_ids_without_grammar: &[&str],
    ) {
        let root = self.plugin_root(plugin_id);
        let grammar_file = root.join(imported_grammar_file(language_id));
        std::fs::create_dir_all(grammar_file.parent().unwrap()).unwrap();
        std::fs::write(grammar_file, grammar).unwrap();
        let languages: Vec<Value> = std::iter::once(language(
            language_id,
            Some(imported_grammar_file(language_id)),
        ))
        .chain(
            language_ids_without_grammar
                .iter()
                .map(|language_id| language(language_id, None)),
        )
        .collect();
        let manifest = json!({
            "manifestVersion": 1,
            "id": plugin_id,
            "name": plugin_id,
            "version": version,
            "contributes": { "languages": languages, "lsp": [], "themes": [] },
        });
        std::fs::write(root.join(PLUGIN_MANIFEST_FILE), manifest.to_string()).unwrap();
    }

    fn reload(&self) {
        self.runtime
            .block_on(plugin_actions::plugin_reload(&self.state, &self.plugins))
            .unwrap();
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        self.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn imported_grammar_file(language_id: &str) -> String {
    format!("grammars/{language_id}.tmLanguage.json")
}

fn language(language_id: &str, grammar: Option<String>) -> Value {
    json!({
        "id": language_id,
        "extensions": [format!(".{language_id}")],
        "aliases": [],
        "grammar": grammar,
        "embeddedLanguages": null,
    })
}

fn plugin_harness() -> Harness {
    let (wake_sender, wake) = mpsc::channel();
    let (finished_sender, finished) = mpsc::channel();
    let signal: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        wake_sender.send(()).ok();
    });
    let (client, task) = token_worker(signal.clone());
    std::thread::spawn(move || {
        task.run();
        finished_sender.send(()).ok();
    });
    Harness {
        syntax: EditorSyntax::with_wake(client, None, signal),
        wake,
        finished,
    }
}

fn settle_with_plugins(
    harness: &mut Harness,
    host: &PluginHost,
    store: &EditorStore,
    theme: &ResolvedTheme,
    now: Instant,
) {
    loop {
        harness
            .syntax
            .follow_plugins(&host.state, &host.plugins, &host.tasks);
        harness.syntax.tick(store, theme, now);
        if harness.syntax.plugin_grammar_read.is_none() && harness.syntax.pipeline.is_settled() {
            return;
        }
        harness.wake.recv_timeout(TIMEOUT).unwrap();
    }
}

fn plugin_spans(
    theme: &ResolvedTheme,
    plugins: &[PluginGrammar],
    store: &EditorStore,
    document: DocumentId,
) -> Vec<Vec<u32>> {
    let snapshot = store.documents().snapshot(document).unwrap();
    let token_theme = TokenTheme::from_resolved(theme).unwrap();
    let mut tokenizer = TextmateTokenizer::with_plugin_grammars(
        &CORE_AND_RUST,
        plugins,
        token_theme.settings(),
        TokenizerLimits::default(),
    )
    .unwrap();
    let mut state = None;
    snapshot
        .rope
        .to_string()
        .split('\n')
        .map(|line| {
            let tokenized = tokenizer
                .try_tokenize_line(&snapshot.metadata.language_id, line, state.as_ref())
                .unwrap();
            state = Some(tokenized.end_state);
            tokenized.spans
        })
        .collect()
}

#[test]
fn 플러그인_목록의_문법을_읽어_그_언어의_문서를_토큰화하고_목록이_바뀌면_다시_읽는다() {
    let host = PluginHost::new();
    host.install(
        INI_PLUGIN_ID,
        FIRST_PLUGIN_VERSION,
        INI_LANGUAGE_ID,
        INI_GRAMMAR,
        &[],
    );
    let theme = theme("dark", "#6a9955");
    let mut harness = plugin_harness();
    let mut store = store();
    let ini = open(
        &mut store,
        "/synthetic/app.taide-ini",
        INI_LANGUAGE_ID,
        INI_SOURCE,
    );
    let rust = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    attach(&mut store, ini);
    attach(&mut store, rust);
    let now = Instant::now();
    harness.settle(&store, &theme, now);
    assert!(!harness.syntax.pipeline.contains(ini));
    assert!(harness.syntax.tokens(&store, ini).is_none());
    assert!(host.plugins.0.read().is_none());

    settle_with_plugins(&mut harness, &host, &store, &theme, now);
    assert!(host.plugins.0.read().is_some());
    let first = [PluginGrammar::from_contribution(INI_LANGUAGE_ID, &[], INI_GRAMMAR).unwrap()];
    let first_spans = plugin_spans(&theme, &first, &store, ini);
    assert_eq!(harness.spans(ini), first_spans);
    assert_eq!(harness.spans(rust), expected_spans(&theme, &store, rust));
    assert_eq!(
        harness.syntax.tokens(&store, ini).unwrap().revision,
        revision(&store, ini)
    );

    host.install(
        INI_PLUGIN_ID,
        SECOND_PLUGIN_VERSION,
        INI_LANGUAGE_ID,
        INI_HASH_COMMENT_GRAMMAR,
        &[],
    );
    settle_with_plugins(&mut harness, &host, &store, &theme, now);
    assert_eq!(harness.spans(ini), first_spans);

    host.reload();
    settle_with_plugins(&mut harness, &host, &store, &theme, now);
    let second = [
        PluginGrammar::from_contribution(INI_LANGUAGE_ID, &[], INI_HASH_COMMENT_GRAMMAR).unwrap(),
    ];
    let second_spans = plugin_spans(&theme, &second, &store, ini);
    assert_ne!(second_spans, first_spans);
    assert_eq!(harness.spans(ini), second_spans);

    std::fs::remove_dir_all(host.plugin_root(INI_PLUGIN_ID)).unwrap();
    host.reload();
    settle_with_plugins(&mut harness, &host, &store, &theme, now);
    assert!(!harness.syntax.pipeline.contains(ini));
    assert!(harness.syntax.tokens(&store, ini).is_none());
    assert_eq!(harness.spans(rust), expected_spans(&theme, &store, rust));
}

#[test]
fn 읽지_못하거나_잘못된_플러그인_문법은_그_언어만_평문으로_두고_다른_언어의_강조는_그대로다() {
    let host = PluginHost::new();
    let version = FIRST_PLUGIN_VERSION;
    host.install(
        "taide-unparsable-plugin",
        version,
        "taide-unparsable",
        UNPARSABLE_GRAMMAR,
        &[],
    );
    host.install(
        "taide-unread-plugin",
        version,
        "taide-unread",
        UNREAD_GRAMMAR,
        &[],
    );
    host.install(
        "taide-cycle-plugin",
        version,
        "taide-cycle",
        CYCLE_GRAMMAR,
        &["taide-plain"],
    );
    host.install(INI_PLUGIN_ID, version, INI_LANGUAGE_ID, INI_GRAMMAR, &[]);
    host.reload();
    std::fs::remove_file(
        host.plugin_root("taide-unread-plugin")
            .join(imported_grammar_file("taide-unread")),
    )
    .unwrap();

    let theme = theme("dark", "#6a9955");
    let mut harness = plugin_harness();
    let mut store = store();
    let unlisted = ["taide-unparsable", "taide-unread", "taide-plain"].map(|language_id| {
        let document = open(
            &mut store,
            &format!("/synthetic/unlisted.{language_id}"),
            language_id,
            INI_SOURCE,
        );
        attach(&mut store, document);
        document
    });
    let cycle = open(
        &mut store,
        "/synthetic/loop.taide-cycle",
        "taide-cycle",
        INI_SOURCE,
    );
    let ini = open(
        &mut store,
        "/synthetic/app.taide-ini",
        INI_LANGUAGE_ID,
        INI_SOURCE,
    );
    let rust = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    for document in [cycle, ini, rust] {
        attach(&mut store, document);
    }
    let now = Instant::now();
    settle_with_plugins(&mut harness, &host, &store, &theme, now);
    assert!(harness.syntax.pipeline.is_worker_running());
    for document in unlisted {
        assert!(!harness.syntax.pipeline.contains(document));
        assert!(harness.syntax.tokens(&store, document).is_none());
    }
    assert_eq!(
        harness.spans(cycle),
        vec![vec![0, UNSTYLED_STYLE_ID]; INI_SOURCE.split('\n').count()]
    );
    let ini_grammar =
        [PluginGrammar::from_contribution(INI_LANGUAGE_ID, &[], INI_GRAMMAR).unwrap()];
    assert_eq!(
        harness.spans(ini),
        plugin_spans(&theme, &ini_grammar, &store, ini)
    );
    assert_eq!(harness.spans(rust), expected_spans(&theme, &store, rust));
}

use std::sync::Arc;

use taide_native_editor::change_journal::ChangesSince;
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::EditorStore;
use taide_native_syntax::{
    MAX_TOKENIZED_DOCUMENT_LINES, MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH, TokenPipeline, TokenTheme,
    TokenizerLimits, token_worker,
};

use crate::support::{
    CORE_LANGUAGE_IDS, TIMEOUT, Worker, apply, dark_theme, direct_spans, light_theme, open,
    repeated_lines, snapshot, spawn_worker, store, text, token_theme,
};

const RUST_SOURCE: &str =
    "fn main() {\n    let value = 1; // one\n    let text = \"a\";\n}\n\nfn other() {}";
const MARKDOWN_SOURCE: &str =
    "# Title\n\n```rust\nfn main() { let value = \"a\"; }\n```\n\n```json\n{ \"key\": true }\n```";
const RUST_LINE: &str = "let value = \"text\"; // note";
const LONG_DOCUMENT_LINE_COUNT: usize = 20_000;
const CORE_AND_RUST: [&str; 4] = ["json", "jsonc", "markdown", "rust"];

fn preview(
    pipeline: &mut TokenPipeline,
    worker: &Worker,
    document: DocumentId,
    line: usize,
    lines: &Arc<[String]>,
) -> Arc<taide_native_editor::line_tokens::PreviewTokens> {
    let started = std::time::Instant::now();
    loop {
        pipeline.poll();
        if let Some(tokens) = pipeline.preview(document, line, lines) {
            return tokens;
        }
        let remaining = TIMEOUT
            .checked_sub(started.elapsed())
            .expect("미리보기 토큰 제한 시간 초과");
        worker.wake.recv_timeout(remaining).unwrap();
    }
}

#[test]
fn 미리보기는_앞줄_주석_문맥과_추가줄을_원문_토큰_변경_없이_강조하고_테마를_갱신한다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let mut store = store();
    let source = "/* comment\npending\n*/\nlet value = 1;";
    let document = open(&mut store, "/preview.rs", "rust", source);
    let dark = token_theme(&dark_theme());
    pipeline.set_theme(dark.clone());
    pipeline.open(snapshot(&store, document));
    settle(&mut pipeline, &worker);
    let original = pipeline_spans(&pipeline, document);
    let lines: Arc<[String]> =
        Arc::from(["let value = 2;".into(), "*/ let text = \"hello\";".into()]);
    let tokens = preview(&mut pipeline, &worker, document, 1, &lines);
    let expected = direct_spans(
        &CORE_AND_RUST,
        &dark,
        TokenizerLimits::default(),
        "rust",
        "/* comment\nlet value = 2;\n*/ let text = \"hello\";",
    );
    assert_eq!(tokens.lines.spans(0), expected[1]);
    assert_eq!(tokens.lines.spans(1), expected[2]);
    assert_eq!(pipeline_spans(&pipeline, document), original);
    assert_eq!(text(&store, document), source);
    assert!(Arc::ptr_eq(
        &tokens,
        &pipeline.preview(document, 1, &lines).unwrap()
    ));
    pipeline.set_theme(token_theme(&light_theme()));
    assert!(pipeline.preview(document, 1, &lines).is_none());
    settle(&mut pipeline, &worker);
    let light = preview(&mut pipeline, &worker, document, 1, &lines);
    assert_ne!(tokens.styles, light.styles);
    assert!(!Arc::ptr_eq(&tokens, &light));
    apply(&mut store, document, vec![(0..2, "  ")]);
    pipeline.replace(snapshot(&store, document));
    assert!(pipeline.preview(document, 1, &lines).is_none());
    settle(&mut pipeline, &worker);
    let changed = preview(&mut pipeline, &worker, document, 1, &lines);
    assert_ne!(changed.lines.spans(0), tokens.lines.spans(0));
}

#[test]
fn 미리보기는_후보_교체의_늦은_응답과_문서_닫힘을_폐기하고_사용하지_않는_캐시를_회수한다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let mut store = store();
    let document = open(&mut store, "/preview.rs", "rust", "let value = 1;");
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    pipeline.open(snapshot(&store, document));
    settle(&mut pipeline, &worker);
    let old: Arc<[String]> = Arc::from(["let text = \"old\";".into()]);
    let current: Arc<[String]> = Arc::from(["fn current() {}".into()]);
    assert!(pipeline.preview(document, 0, &old).is_none());
    assert!(pipeline.preview(document, 0, &current).is_none());
    let tokens = preview(&mut pipeline, &worker, document, 0, &current);
    assert_eq!(
        tokens.lines.spans(0),
        direct_spans(
            &CORE_AND_RUST,
            &theme,
            TokenizerLimits::default(),
            "rust",
            &current[0]
        )[0]
    );
    pipeline.retain_previews(&[]);
    assert!(pipeline.preview(document, 0, &current).is_none());
    pipeline.close(document);
    pipeline.poll();
    assert!(pipeline.preview(document, 0, &current).is_none());
}

fn settle(pipeline: &mut TokenPipeline, worker: &Worker) {
    loop {
        pipeline.poll();
        if pipeline.is_settled() {
            return;
        }
        assert!(pipeline.is_worker_running());
        worker.wake.recv_timeout(TIMEOUT).unwrap();
    }
}

fn wait_for_first_lines(pipeline: &mut TokenPipeline, worker: &Worker, document: DocumentId) {
    loop {
        pipeline.poll();
        if pipeline.tokens(document).unwrap().first_invalid_line() != Some(0) {
            return;
        }
        worker.wake.recv_timeout(TIMEOUT).unwrap();
    }
}

fn invalid_lines(pipeline: &TokenPipeline, document: DocumentId) -> Vec<(usize, usize)> {
    pipeline
        .tokens(document)
        .unwrap()
        .invalid_ranges()
        .iter()
        .map(|range| (range.start, range.end))
        .collect()
}

fn pipeline_spans(pipeline: &TokenPipeline, document: DocumentId) -> Vec<Vec<u32>> {
    let tokens = pipeline.tokens(document).unwrap();
    assert_eq!(tokens.first_invalid_line(), None);
    (0..tokens.line_count())
        .map(|line| tokens.spans(line).to_vec())
        .collect()
}

fn expected_spans(
    languages: &[&str],
    theme: &TokenTheme,
    store: &EditorStore,
    document: DocumentId,
) -> Vec<Vec<u32>> {
    let snapshot = snapshot(store, document);
    direct_spans(
        languages,
        theme,
        TokenizerLimits::default(),
        &snapshot.metadata.language_id,
        &snapshot.rope.to_string(),
    )
}

fn follow(pipeline: &mut TokenPipeline, store: &EditorStore, document: DocumentId, revision: u64) {
    match store.changes_since(document, revision).unwrap() {
        ChangesSince::Tracked(changes) => pipeline.edit(snapshot(store, document), changes),
        ChangesSince::Lagged => pipeline.replace(snapshot(store, document)),
    }
}

#[test]
fn 테마가_정해지기_전에는_아무것도_하지_않고_정해지면_연_문서를_전부_토큰화한다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    pipeline.open(snapshot(&store, document));
    assert!(!pipeline.poll());
    assert!(pipeline.style_table().is_none());
    assert_eq!(pipeline.requested_language_ids(), CORE_AND_RUST);
    assert_eq!(
        pipeline.tokens(document).unwrap().first_invalid_line(),
        Some(0)
    );

    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );
    let table = pipeline.style_table().unwrap();
    assert_eq!(
        Some(table),
        theme
            .style_table(&crate::support::tokenizer(
                &CORE_AND_RUST,
                &theme,
                TokenizerLimits::default()
            ))
            .ok()
            .as_ref()
    );
    assert!(pipeline.configuration_error().is_none());
}

#[test]
fn 편집과_undo와_redo는_저널을_따라_증분으로_반영된다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let mut store = store();
    let document = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    pipeline.open(snapshot(&store, document));
    settle(&mut pipeline, &worker);

    let value = RUST_SOURCE.find("value").unwrap();
    apply(&mut store, document, vec![(value..value + 5, "renamed")]);
    follow(&mut pipeline, &store, document, 0);
    assert_eq!(invalid_lines(&pipeline, document), [(1, 2)]);
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );

    let second_line = RUST_SOURCE.find("    let value").unwrap();
    apply(
        &mut store,
        document,
        vec![(second_line..second_line, "    /* open\n")],
    );
    follow(&mut pipeline, &store, document, 1);
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );

    assert!(store.undo(document).unwrap());
    follow(&mut pipeline, &store, document, 2);
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );

    assert!(store.redo(document).unwrap());
    follow(&mut pipeline, &store, document, 3);
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );
    assert!(text(&store, document).contains("/* open"));
}

#[test]
fn 진행_중인_작업과_겹친_편집과_전체_교체는_지난_응답을_버리고_새_문서로_수렴한다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let mut store = store();
    let source = repeated_lines(RUST_LINE, LONG_DOCUMENT_LINE_COUNT);
    let document = open(&mut store, "/synthetic/long.rs", "rust", &source);
    pipeline.open(snapshot(&store, document));
    wait_for_first_lines(&mut pipeline, &worker, document);

    apply(&mut store, document, vec![(0..0, "/* open\n")]);
    follow(&mut pipeline, &store, document, 0);
    assert_eq!(
        pipeline.tokens(document).unwrap().line_count(),
        LONG_DOCUMENT_LINE_COUNT + 1
    );
    apply(&mut store, document, vec![(0..2, "//")]);
    follow(&mut pipeline, &store, document, 1);
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );

    pipeline.replace(snapshot(&store, document));
    assert_eq!(
        invalid_lines(&pipeline, document),
        [(0, LONG_DOCUMENT_LINE_COUNT + 1)]
    );
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, document),
        expected_spans(&CORE_AND_RUST, &theme, &store, document)
    );
}

#[test]
fn 테마가_바뀌면_스타일_표와_모든_문서의_토큰이_새_테마의_결과가_된다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let dark = token_theme(&dark_theme());
    let light = token_theme(&light_theme());
    pipeline.set_theme(dark.clone());
    let mut store = store();
    let rust = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    let json = open(
        &mut store,
        "/synthetic/data.json",
        "json",
        "{ \"key\": [1, \"a\"] }",
    );
    pipeline.open(snapshot(&store, rust));
    pipeline.open(snapshot(&store, json));
    settle(&mut pipeline, &worker);
    let dark_table = pipeline.style_table().unwrap().clone();
    assert_eq!(
        pipeline_spans(&pipeline, json),
        expected_spans(&CORE_AND_RUST, &dark, &store, json)
    );

    pipeline.set_theme(dark.clone());
    assert!(pipeline.is_settled());
    pipeline.set_theme(light.clone());
    assert!(!pipeline.is_settled());
    settle(&mut pipeline, &worker);
    assert_ne!(pipeline.style_table().unwrap(), &dark_table);
    for document in [rust, json] {
        assert_eq!(
            pipeline_spans(&pipeline, document),
            expected_spans(&CORE_AND_RUST, &light, &store, document)
        );
    }
    assert_ne!(
        expected_spans(&CORE_AND_RUST, &light, &store, rust),
        expected_spans(&CORE_AND_RUST, &dark, &store, rust)
    );
}

#[test]
fn 언어_집합이_커지면_그_언어의_markdown_코드_펜스도_강조된다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let mut store = store();
    let markdown = open(
        &mut store,
        "/synthetic/readme.md",
        "markdown",
        MARKDOWN_SOURCE,
    );
    pipeline.open(snapshot(&store, markdown));
    settle(&mut pipeline, &worker);
    assert_eq!(pipeline.requested_language_ids(), CORE_LANGUAGE_IDS);
    let core_only = expected_spans(&CORE_LANGUAGE_IDS, &theme, &store, markdown);
    assert_eq!(pipeline_spans(&pipeline, markdown), core_only);

    let rust = open(&mut store, "/synthetic/main.rs", "rust", RUST_SOURCE);
    pipeline.open(snapshot(&store, rust));
    assert!(!pipeline.is_settled());
    settle(&mut pipeline, &worker);
    let with_rust = expected_spans(&CORE_AND_RUST, &theme, &store, markdown);
    assert_ne!(with_rust, core_only);
    assert_eq!(pipeline_spans(&pipeline, markdown), with_rust);
    assert_eq!(
        pipeline_spans(&pipeline, rust),
        expected_spans(&CORE_AND_RUST, &theme, &store, rust)
    );

    pipeline.close(rust);
    assert_eq!(pipeline.requested_language_ids(), CORE_AND_RUST);
    settle(&mut pipeline, &worker);
    assert_eq!(pipeline_spans(&pipeline, markdown), with_rust);
}

#[test]
fn 닫은_문서의_응답은_버리고_남은_문서는_끝까지_토큰화한다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let mut store = store();
    let source = repeated_lines(RUST_LINE, LONG_DOCUMENT_LINE_COUNT);
    let closed = open(&mut store, "/synthetic/closed.rs", "rust", &source);
    let kept = open(&mut store, "/synthetic/kept.rs", "rust", RUST_SOURCE);
    pipeline.open(snapshot(&store, closed));
    pipeline.open(snapshot(&store, kept));
    wait_for_first_lines(&mut pipeline, &worker, closed);
    pipeline.close(closed);
    assert!(!pipeline.contains(closed));
    assert!(pipeline.tokens(closed).is_none());
    settle(&mut pipeline, &worker);
    assert_eq!(
        pipeline_spans(&pipeline, kept),
        expected_spans(&CORE_AND_RUST, &theme, &store, kept)
    );
}

#[test]
fn 번들에_없는_언어와_한도를_넘는_문서는_토큰화하지_않는다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    pipeline.set_theme(token_theme(&dark_theme()));
    let mut store = store();
    let plain = open(
        &mut store,
        "/synthetic/notes.txt",
        "plaintext",
        "plain text",
    );
    let many_lines = "\n".repeat(MAX_TOKENIZED_DOCUMENT_LINES);
    let too_many_lines = open(&mut store, "/synthetic/lines.json", "json", &many_lines);
    let wide = "1".repeat(MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH + 1);
    let too_wide = open(&mut store, "/synthetic/wide.json", "json", &wide);
    let small = open(&mut store, "/synthetic/small.json", "json", "[1]");
    for document in [plain, too_many_lines, too_wide, small] {
        pipeline.open(snapshot(&store, document));
    }
    settle(&mut pipeline, &worker);
    assert_eq!(pipeline.requested_language_ids(), CORE_LANGUAGE_IDS);
    assert_eq!(pipeline.tokens(small).unwrap().first_invalid_line(), None);
    assert!(!pipeline.tokens(small).unwrap().spans(0).is_empty());
    for document in [plain, too_many_lines, too_wide] {
        let tokens = pipeline.tokens(document).unwrap();
        assert!(!tokens.has_accurate_tokens(0));
        assert!(tokens.spans(0).is_empty());
    }
    assert_eq!(pipeline.tokens(too_many_lines).unwrap().line_count(), 0);
    assert_eq!(pipeline.tokens(too_wide).unwrap().line_count(), 0);

    apply(&mut store, too_wide, vec![(0..wide.len(), "[2]")]);
    follow(&mut pipeline, &store, too_wide, 0);
    settle(&mut pipeline, &worker);
    assert_eq!(text(&store, too_wide), "[2]");
    let tokens = pipeline.tokens(too_wide).unwrap();
    assert!(!tokens.has_accurate_tokens(0));
    assert!(tokens.spans(0).is_empty());

    pipeline.replace(snapshot(&store, too_wide));
    settle(&mut pipeline, &worker);
    assert!(!pipeline.tokens(too_wide).unwrap().has_accurate_tokens(0));
}

#[test]
fn worker가_없어지면_연결이_끊긴_것으로_보고_더_요청하지_않는다() {
    let (client, task) = token_worker(Arc::new(|| {}));
    drop(task);
    let mut pipeline = TokenPipeline::new(client);
    assert!(pipeline.is_worker_running());
    pipeline.set_theme(token_theme(&dark_theme()));
    assert!(!pipeline.poll());
    assert!(!pipeline.is_worker_running());
    let mut store = store();
    let document = open(&mut store, "/synthetic/a.json", "json", "{}");
    pipeline.open(snapshot(&store, document));
    assert!(!pipeline.poll());
    assert!(pipeline.style_table().is_none());
}

#[test]
fn 연결을_끊으면_작업_중인_worker도_끝난다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    pipeline.set_theme(token_theme(&dark_theme()));
    let mut store = store();
    let source = repeated_lines(RUST_LINE, LONG_DOCUMENT_LINE_COUNT * 10);
    let document = open(&mut store, "/synthetic/long.rs", "rust", &source);
    pipeline.open(snapshot(&store, document));
    wait_for_first_lines(&mut pipeline, &worker, document);
    pipeline.disconnect();
    assert!(!pipeline.is_worker_running());
    worker.finished.recv_timeout(TIMEOUT).unwrap();
    assert!(!pipeline.poll());
}

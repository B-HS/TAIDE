use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;

use taide_native_editor::change_journal::ChangesSince;
use taide_native_editor::document::{DocumentId, DocumentSnapshot};
use taide_native_editor::line_tokens::TokenStyleTable;
use taide_native_syntax::{
    DocumentTokens, SyntaxError, TokenTheme, TokenizationJob, TokenizedLine, TokenizerLimits,
    UNSTYLED_STYLE_ID, WorkerClient, WorkerConfiguration, WorkerRequest, WorkerResponse,
    WorkerStopped, WorkerTask, token_worker,
};

use crate::support::{
    CORE_LANGUAGE_IDS, QUIET_PERIOD, TIMEOUT, Worker, apply, dark_theme, direct_spans, open,
    repeated_lines, snapshot, spawn_worker, spawn_worker_with, store, text, token_theme, tokenizer,
};

const FIRST_GENERATION: u64 = 1;
const RUST_LINE: &str = "let value = \"text\"; // note";
const BACKGROUND_LINE_COUNT: usize = 6000;
const VISIBLE_LINES_END: usize = 2050;
const VISIBLE_LINES_START: usize = 2000;
const LONG_DOCUMENT_LINE_COUNT: usize = 200_000;
const SHORT_LINE_LIMIT: usize = 10;
const JSONC_SOURCE: &str = "{\n  // comment\n  \"key\": [1, true, null],\n  /* open\n  still open */ \"last\": \"value\"\n}";

fn configuration(
    generation: u64,
    language_ids: &[&str],
    theme: &TokenTheme,
    limits: TokenizerLimits,
) -> WorkerRequest {
    WorkerRequest::Configure(Box::new(WorkerConfiguration {
        generation,
        language_ids: language_ids.iter().map(|id| (*id).to_owned()).collect(),
        theme: theme.clone(),
        limits,
    }))
}

fn job(
    id: u64,
    generation: u64,
    snapshot: DocumentSnapshot,
    tokens: &mut DocumentTokens,
) -> TokenizationJob {
    TokenizationJob {
        id,
        generation,
        snapshot,
        plan: tokens.plan().unwrap(),
        visible_lines: 0..0,
    }
}

fn receive(client: &WorkerClient, worker: &Worker) -> WorkerResponse {
    loop {
        if let Some(response) = client.try_receive().unwrap() {
            return response;
        }
        worker.wake.recv_timeout(TIMEOUT).unwrap();
    }
}

fn configured(
    client: &WorkerClient,
    worker: &Worker,
) -> (u64, Result<TokenStyleTable, SyntaxError>) {
    match receive(client, worker) {
        WorkerResponse::Configured { generation, result } => (generation, result),
        other => panic!("expected a configuration answer, got {other:?}"),
    }
}

struct Batch {
    job: u64,
    document: DocumentId,
    first_line: usize,
    lines: Vec<TokenizedLine>,
    is_finished: bool,
}

fn batch(client: &WorkerClient, worker: &Worker) -> Batch {
    match receive(client, worker) {
        WorkerResponse::Tokenized {
            job,
            document,
            first_line,
            lines,
            is_finished,
        } => Batch {
            job,
            document,
            first_line,
            lines,
            is_finished,
        },
        other => panic!("expected tokenized lines, got {other:?}"),
    }
}

fn finish(client: &WorkerClient, worker: &Worker, job: u64, tokens: &mut DocumentTokens) -> usize {
    let mut line_count = 0;
    loop {
        let batch = batch(client, worker);
        assert_eq!(batch.job, job);
        line_count += batch.lines.len();
        tokens.accept(batch.first_line, batch.lines);
        if batch.is_finished {
            return line_count;
        }
    }
}

fn stored_spans(tokens: &DocumentTokens) -> Vec<Vec<u32>> {
    (0..tokens.tokens().line_count())
        .map(|line| tokens.tokens().spans(line).to_vec())
        .collect()
}

#[test]
fn worker_작업은_다른_스레드로_넘길_수_있다() {
    fn assert_send<T: Send>() {}
    assert_send::<WorkerTask>();
    assert_send::<WorkerRequest>();
    assert_send::<WorkerResponse>();
}

#[test]
fn 준비_요청은_같은_세대의_다른_설정을_구분하고_표시_작업의_엔진을_보존한다() {
    use taide_native_editor::syntax::TokenKind;
    let theme = token_theme(&dark_theme());
    let limits = TokenizerLimits::default();
    let (client, worker) = spawn_worker();
    client
        .send(configuration(FIRST_GENERATION, &["json"], &theme, limits))
        .unwrap();
    assert!(configured(&client, &worker).1.is_ok());
    let preparer = client.token_preparer(
        WorkerConfiguration {
            generation: FIRST_GENERATION,
            language_ids: vec!["rust".into()],
            theme: theme.clone(),
            limits,
        },
        Vec::new(),
    );
    let mut store = store();
    let rust = open(&mut store, "/synthetic/prepare.rs", "rust", RUST_LINE);
    let snapshot = snapshot(&store, rust);
    let prepared = preparer
        .prepare(&snapshot, 0..snapshot.rope.len_lines())
        .unwrap();
    assert!(
        prepared.lines[0]
            .iter()
            .any(|token| token.kind == TokenKind::String)
    );
    assert!(
        preparer
            .prepare(&snapshot, 0..snapshot.rope.len_lines() + 1)
            .is_err()
    );
    let json = open(
        &mut store,
        "/synthetic/preserved.json",
        "json",
        "{\"value\":\"text\"}",
    );
    let current = crate::support::snapshot(&store, json);
    let mut tokens = DocumentTokens::new(current.rope.len_lines());
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            current,
            &mut tokens,
        ))))
        .unwrap();
    let response = batch(&client, &worker);
    assert!(
        response.lines[0]
            .kinds
            .iter()
            .any(|token| token.kind == TokenKind::String)
    );
    assert!(response.is_finished);
    drop(client);
    worker.finished.recv_timeout(TIMEOUT).unwrap();
    assert!(
        preparer
            .prepare(&snapshot, 0..snapshot.rope.len_lines())
            .is_err()
    );
}

#[test]
fn 설정에_성공하면_스타일_표를_돌려주고_실패한_설정은_이전_엔진을_남긴다() {
    let theme = token_theme(&dark_theme());
    let limits = TokenizerLimits::default();
    let (client, worker) = spawn_worker();
    client
        .send(configuration(
            FIRST_GENERATION,
            &CORE_LANGUAGE_IDS,
            &theme,
            limits,
        ))
        .unwrap();
    let (generation, result) = configured(&client, &worker);
    assert_eq!(generation, FIRST_GENERATION);
    let expected = theme
        .style_table(&tokenizer(&CORE_LANGUAGE_IDS, &theme, limits))
        .unwrap();
    assert_eq!(result, Ok(expected));

    client
        .send(configuration(
            FIRST_GENERATION + 1,
            &["no-such-language"],
            &theme,
            limits,
        ))
        .unwrap();
    let (generation, result) = configured(&client, &worker);
    assert_eq!(generation, FIRST_GENERATION + 1);
    assert_eq!(
        result,
        Err(SyntaxError::UnknownLanguage("no-such-language".to_owned()))
    );

    let mut store = store();
    let document = open(&mut store, "/synthetic/a.jsonc", "jsonc", JSONC_SOURCE);
    let mut tokens = DocumentTokens::new(JSONC_SOURCE.split('\n').count());
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    assert_eq!(
        finish(&client, &worker, 1, &mut tokens),
        JSONC_SOURCE.split('\n').count()
    );
    assert_eq!(
        stored_spans(&tokens),
        direct_spans(&CORE_LANGUAGE_IDS, &theme, limits, "jsonc", JSONC_SOURCE)
    );
    assert_eq!(tokens.tokens().first_invalid_line(), None);
}

#[test]
fn 세대가_다른_작업은_줄_없이_끝난_것으로_돌려준다() {
    let theme = token_theme(&dark_theme());
    let (client, worker) = spawn_worker();
    let mut store = store();
    let document = open(&mut store, "/synthetic/a.json", "json", "{}");
    let mut tokens = DocumentTokens::new(1);
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            7,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    let unconfigured = batch(&client, &worker);
    assert_eq!((unconfigured.job, unconfigured.document), (7, document));
    assert!(unconfigured.lines.is_empty() && unconfigured.is_finished);

    client
        .send(configuration(
            FIRST_GENERATION,
            &CORE_LANGUAGE_IDS,
            &theme,
            TokenizerLimits::default(),
        ))
        .unwrap();
    assert!(configured(&client, &worker).1.is_ok());
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            8,
            FIRST_GENERATION + 1,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    let stale = batch(&client, &worker);
    assert_eq!(stale.job, 8);
    assert!(stale.lines.is_empty() && stale.is_finished);
}

#[test]
fn 편집_뒤의_작업은_끝_상태가_저장된_상태와_같아지는_줄에서_끝난다() {
    let theme = token_theme(&dark_theme());
    let limits = TokenizerLimits::default();
    let languages = ["json", "jsonc", "markdown", "rust"];
    let source = repeated_lines(RUST_LINE, 400);
    let (client, worker) = spawn_worker();
    client
        .send(configuration(FIRST_GENERATION, &languages, &theme, limits))
        .unwrap();
    assert!(configured(&client, &worker).1.is_ok());
    let mut store = store();
    let document = open(&mut store, "/synthetic/a.rs", "rust", &source);
    let mut tokens = DocumentTokens::new(400);
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    assert_eq!(finish(&client, &worker, 1, &mut tokens), 400);

    let edited_line_start = (RUST_LINE.len() + 1) * 150;
    apply(
        &mut store,
        document,
        vec![(edited_line_start + 4..edited_line_start + 9, "renamed")],
    );
    let ChangesSince::Tracked(changes) = store.changes_since(document, 0).unwrap() else {
        panic!("journal lagged");
    };
    for change in changes {
        assert!(tokens.apply(change));
    }
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            2,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    assert_eq!(finish(&client, &worker, 2, &mut tokens), 1);
    assert_eq!(tokens.tokens().first_invalid_line(), None);
    assert_eq!(
        stored_spans(&tokens),
        direct_spans(&languages, &theme, limits, "rust", &text(&store, document))
    );

    apply(
        &mut store,
        document,
        vec![(edited_line_start..edited_line_start, "/*")],
    );
    let ChangesSince::Tracked(changes) = store.changes_since(document, 1).unwrap() else {
        panic!("journal lagged");
    };
    for change in changes {
        assert!(tokens.apply(change));
    }
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            3,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    assert_eq!(finish(&client, &worker, 3, &mut tokens), 250);
    assert_eq!(
        stored_spans(&tokens),
        direct_spans(&languages, &theme, limits, "rust", &text(&store, document))
    );
}

#[test]
fn 보이는_줄이_남은_문서를_다른_문서보다_먼저_처리한다() {
    let theme = token_theme(&dark_theme());
    let languages = ["json", "jsonc", "markdown", "rust"];
    let source = repeated_lines(RUST_LINE, BACKGROUND_LINE_COUNT);
    let mut store = store();
    let hidden = open(&mut store, "/synthetic/hidden.rs", "rust", &source);
    let visible = open(&mut store, "/synthetic/visible.rs", "rust", &source);
    let mut hidden_tokens = DocumentTokens::new(BACKGROUND_LINE_COUNT);
    let mut visible_tokens = DocumentTokens::new(BACKGROUND_LINE_COUNT);
    let (client, worker) = spawn_worker_with(vec![
        configuration(
            FIRST_GENERATION,
            &languages,
            &theme,
            TokenizerLimits::default(),
        ),
        WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            snapshot(&store, hidden),
            &mut hidden_tokens,
        ))),
        WorkerRequest::Tokenize(Box::new(TokenizationJob {
            visible_lines: VISIBLE_LINES_START..VISIBLE_LINES_END,
            ..job(
                2,
                FIRST_GENERATION,
                snapshot(&store, visible),
                &mut visible_tokens,
            )
        })),
    ]);
    assert!(configured(&client, &worker).1.is_ok());
    let mut visible_line_count = 0;
    let mut hidden_line_count = 0;
    let mut visible_lines_before_hidden = None;
    while visible_line_count < BACKGROUND_LINE_COUNT || hidden_line_count < BACKGROUND_LINE_COUNT {
        let batch = batch(&client, &worker);
        if batch.document == visible {
            visible_line_count += batch.lines.len();
        } else {
            visible_lines_before_hidden.get_or_insert(visible_line_count);
            hidden_line_count += batch.lines.len();
        }
    }
    assert_eq!(visible_lines_before_hidden, Some(VISIBLE_LINES_END));
}

#[test]
fn 취소한_문서의_작업은_멈추고_다른_문서의_작업은_끝까지_간다() {
    let theme = token_theme(&dark_theme());
    let languages = ["json", "jsonc", "markdown", "rust"];
    let long_source = repeated_lines(RUST_LINE, LONG_DOCUMENT_LINE_COUNT);
    let mut store = store();
    let cancelled = open(&mut store, "/synthetic/cancelled.rs", "rust", &long_source);
    let kept = open(&mut store, "/synthetic/kept.jsonc", "jsonc", JSONC_SOURCE);
    let mut cancelled_tokens = DocumentTokens::new(LONG_DOCUMENT_LINE_COUNT);
    let mut kept_tokens = DocumentTokens::new(JSONC_SOURCE.split('\n').count());
    let (client, worker) = spawn_worker();
    client
        .send(configuration(
            FIRST_GENERATION,
            &languages,
            &theme,
            TokenizerLimits::default(),
        ))
        .unwrap();
    assert!(configured(&client, &worker).1.is_ok());
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            snapshot(&store, cancelled),
            &mut cancelled_tokens,
        ))))
        .unwrap();
    assert_eq!(batch(&client, &worker).document, cancelled);
    client
        .send(WorkerRequest::Cancel {
            document: cancelled,
        })
        .unwrap();
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            2,
            FIRST_GENERATION,
            snapshot(&store, kept),
            &mut kept_tokens,
        ))))
        .unwrap();
    let mut cancelled_line_count = 0;
    loop {
        let batch = batch(&client, &worker);
        if batch.document == cancelled {
            cancelled_line_count += batch.lines.len();
            continue;
        }
        kept_tokens.accept(batch.first_line, batch.lines);
        if batch.is_finished {
            break;
        }
    }
    assert!(cancelled_line_count < LONG_DOCUMENT_LINE_COUNT);
    assert_eq!(kept_tokens.tokens().first_invalid_line(), None);
    while worker.wake.try_recv().is_ok() {}
    assert_eq!(
        worker.wake.recv_timeout(QUIET_PERIOD),
        Err(RecvTimeoutError::Timeout)
    );
    assert!(client.try_receive().unwrap().is_none());
}

#[test]
fn 설정의_줄_한도를_넘는_줄은_토큰화하지_않고_상태를_다음_줄로_넘긴다() {
    let theme = token_theme(&dark_theme());
    let limits = TokenizerLimits {
        max_line_utf16_length: SHORT_LINE_LIMIT,
        line_time_limit_millis: None,
    };
    let source = "/* open\nthis line is longer than the limit\nx */ 1";
    let mut store = store();
    let document = open(&mut store, "/synthetic/a.jsonc", "jsonc", source);
    let mut tokens = DocumentTokens::new(3);
    let (client, worker) = spawn_worker();
    client
        .send(configuration(
            FIRST_GENERATION,
            &CORE_LANGUAGE_IDS,
            &theme,
            limits,
        ))
        .unwrap();
    assert!(configured(&client, &worker).1.is_ok());
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    assert_eq!(finish(&client, &worker, 1, &mut tokens), 3);
    assert_eq!(tokens.tokens().spans(1), &[0, UNSTYLED_STYLE_ID]);
    let unlimited = direct_spans(
        &CORE_LANGUAGE_IDS,
        &theme,
        TokenizerLimits::default(),
        "jsonc",
        source,
    );
    assert_eq!(tokens.tokens().spans(0), unlimited[0]);
    assert_eq!(tokens.tokens().spans(2), unlimited[2]);
    assert_ne!(tokens.tokens().spans(1), unlimited[1]);
}

#[test]
fn client를_버리면_쉬는_worker도_일하는_worker도_끝난다() {
    let (idle_client, idle_worker) = spawn_worker();
    drop(idle_client);
    idle_worker.finished.recv_timeout(TIMEOUT).unwrap();

    let theme = token_theme(&dark_theme());
    let languages = ["json", "jsonc", "markdown", "rust"];
    let source = repeated_lines(RUST_LINE, LONG_DOCUMENT_LINE_COUNT);
    let mut store = store();
    let document = open(&mut store, "/synthetic/busy.rs", "rust", &source);
    let mut tokens = DocumentTokens::new(LONG_DOCUMENT_LINE_COUNT);
    let (client, worker) = spawn_worker();
    client
        .send(configuration(
            FIRST_GENERATION,
            &languages,
            &theme,
            TokenizerLimits::default(),
        ))
        .unwrap();
    assert!(configured(&client, &worker).1.is_ok());
    client
        .send(WorkerRequest::Tokenize(Box::new(job(
            1,
            FIRST_GENERATION,
            snapshot(&store, document),
            &mut tokens,
        ))))
        .unwrap();
    let first = batch(&client, &worker);
    assert!(!first.is_finished && first.lines.len() < LONG_DOCUMENT_LINE_COUNT);
    drop(client);
    worker.finished.recv_timeout(TIMEOUT).unwrap();
}

#[test]
fn 실행되지_않은_worker의_client는_요청을_보내지_못한다() {
    let mut store = store();
    let document = open(&mut store, "/synthetic/a.json", "json", "{}");
    let (client, task) = token_worker(Arc::new(|| {}));
    drop(task);
    assert_eq!(
        client.send(WorkerRequest::Cancel { document }),
        Err(WorkerStopped)
    );
    assert!(matches!(client.try_receive(), Err(WorkerStopped)));
}

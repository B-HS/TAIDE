use taide_native_editor::document::DocumentId;
use taide_native_syntax::{
    PluginGrammar, TextmateTokenizer, TokenPipeline, TokenTheme, TokenizerLimits, UNSTYLED_STYLE_ID,
};

use crate::support::{
    CORE_LANGUAGE_IDS, TIMEOUT, Worker, dark_theme, direct_spans, open, snapshot, spawn_worker,
    store, token_theme,
};

const CORE_AND_RUST: [&str; 4] = ["json", "jsonc", "markdown", "rust"];
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
const INI_SOURCE: &str = "; note\nkey = \"open\nclosed\" # hash\nlast = 1";
const HOST_LANGUAGE_ID: &str = "taide-host";
const HOST_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-host",
    "patterns": [{
        "begin": "^<rust>$",
        "end": "^</rust>$",
        "patterns": [{ "include": "source.rust" }]
    }]
}"##;
const RUST_LINE: &str = "let s = \"a\"; // n";
const HOST_SOURCE: &str = "<rust>\nlet s = \"a\"; // n\n</rust>";
const EMBEDDED_LINE: usize = 1;
const BROKEN_REGEX_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-broken-regex",
    "patterns": [{ "match": "(", "name": "keyword.taide-broken-regex" }]
}"##;
const CYCLE_GRAMMAR: &str = r##"{
    "scopeName": "source.taide-cycle",
    "patterns": [{ "include": "#a" }],
    "repository": {
        "a": { "patterns": [{ "include": "#b" }] },
        "b": { "patterns": [{ "include": "#a" }] }
    }
}"##;
const MISSHAPEN_GRAMMAR: &str = r##"{ "scopeName": "source.taide-misshapen", "patterns": [5] }"##;
const REJECTED_SOURCE: &str = "(x\n\"y\"";
const JSON_SOURCE: &str = "{ \"key\": [1, \"a\", true] }";

fn plugin(language_id: &str, grammar_json: &str) -> PluginGrammar {
    PluginGrammar::from_contribution(language_id, &[], grammar_json).unwrap()
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

fn spans(pipeline: &TokenPipeline, document: DocumentId) -> Vec<Vec<u32>> {
    let tokens = pipeline.tokens(document).unwrap();
    assert_eq!(tokens.first_invalid_line(), None);
    (0..tokens.line_count())
        .map(|line| tokens.spans(line).to_vec())
        .collect()
}

fn plugin_spans(
    requested_language_ids: &[&str],
    plugins: &[PluginGrammar],
    theme: &TokenTheme,
    language_id: &str,
    content: &str,
) -> Vec<Vec<u32>> {
    let mut tokenizer = TextmateTokenizer::with_plugin_grammars(
        requested_language_ids,
        plugins,
        theme.settings(),
        TokenizerLimits::default(),
    )
    .unwrap();
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

#[test]
fn 플러그인_문법을_알리면_그_언어의_문서를_토큰화하고_집합이_바뀌면_다시_토큰화한다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let mut store = store();
    let ini = open(
        &mut store,
        "/synthetic/app.taide-ini",
        INI_LANGUAGE_ID,
        INI_SOURCE,
    );
    let json = open(&mut store, "/synthetic/data.json", "json", JSON_SOURCE);
    pipeline.open(snapshot(&store, ini));
    pipeline.open(snapshot(&store, json));
    settle(&mut pipeline, &worker);
    assert!(!pipeline.accepts_language(INI_LANGUAGE_ID));
    assert!(pipeline.accepts_language("json"));
    assert!(!pipeline.accepts_language("plaintext"));
    assert_eq!(pipeline.tokens(ini).unwrap().first_invalid_line(), Some(0));
    let json_spans = direct_spans(
        &CORE_LANGUAGE_IDS,
        &theme,
        TokenizerLimits::default(),
        "json",
        JSON_SOURCE,
    );
    assert_eq!(spans(&pipeline, json), json_spans);

    let first = vec![plugin(INI_LANGUAGE_ID, INI_GRAMMAR)];
    pipeline.set_plugin_grammars(first.clone());
    assert!(pipeline.accepts_language(INI_LANGUAGE_ID));
    assert!(!pipeline.is_settled());
    settle(&mut pipeline, &worker);
    let first_spans = plugin_spans(
        &CORE_LANGUAGE_IDS,
        &first,
        &theme,
        INI_LANGUAGE_ID,
        INI_SOURCE,
    );
    assert_eq!(spans(&pipeline, ini), first_spans);
    assert_eq!(spans(&pipeline, json), json_spans);
    assert_eq!(pipeline.requested_language_ids(), CORE_LANGUAGE_IDS);
    assert!(pipeline.configuration_error().is_none());

    pipeline.set_plugin_grammars(first);
    assert!(pipeline.is_settled());

    let second = vec![plugin(INI_LANGUAGE_ID, INI_HASH_COMMENT_GRAMMAR)];
    pipeline.set_plugin_grammars(second.clone());
    assert!(!pipeline.is_settled());
    settle(&mut pipeline, &worker);
    let second_spans = plugin_spans(
        &CORE_LANGUAGE_IDS,
        &second,
        &theme,
        INI_LANGUAGE_ID,
        INI_SOURCE,
    );
    assert_ne!(second_spans, first_spans);
    assert_eq!(spans(&pipeline, ini), second_spans);

    pipeline.set_plugin_grammars(Vec::new());
    assert!(!pipeline.accepts_language(INI_LANGUAGE_ID));
    settle(&mut pipeline, &worker);
    let untokenized = pipeline.tokens(ini).unwrap();
    assert_eq!(untokenized.first_invalid_line(), Some(0));
    assert!(untokenized.spans(0).is_empty());
    assert_eq!(spans(&pipeline, json), json_spans);
}

#[test]
fn 플러그인_문법이_끌어오는_번들_언어는_요청_집합에_더해지고_플러그인이_사라져도_남는다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let embedded = [
        "rust".to_owned(),
        INI_LANGUAGE_ID.to_owned(),
        "no-such".to_owned(),
        "json".to_owned(),
    ];
    let plugins =
        vec![PluginGrammar::from_contribution(HOST_LANGUAGE_ID, &embedded, HOST_GRAMMAR).unwrap()];
    pipeline.set_plugin_grammars(plugins.clone());
    assert_eq!(pipeline.requested_language_ids(), CORE_AND_RUST);

    let mut store = store();
    let host = open(
        &mut store,
        "/synthetic/page.taide-host",
        HOST_LANGUAGE_ID,
        HOST_SOURCE,
    );
    pipeline.open(snapshot(&store, host));
    settle(&mut pipeline, &worker);
    let tokenized = spans(&pipeline, host);
    assert_eq!(
        tokenized,
        plugin_spans(
            &CORE_AND_RUST,
            &plugins,
            &theme,
            HOST_LANGUAGE_ID,
            HOST_SOURCE
        )
    );
    assert_eq!(
        tokenized[EMBEDDED_LINE],
        direct_spans(
            &CORE_AND_RUST,
            &theme,
            TokenizerLimits::default(),
            "rust",
            RUST_LINE
        )[0]
    );

    pipeline.set_plugin_grammars(Vec::new());
    assert_eq!(pipeline.requested_language_ids(), CORE_AND_RUST);
    settle(&mut pipeline, &worker);
    assert!(!pipeline.accepts_language(HOST_LANGUAGE_ID));
}

#[test]
fn 잘못된_플러그인_문법은_worker를_멈추지_않고_다른_언어의_토큰도_바꾸지_않는다() {
    let (client, worker) = spawn_worker();
    let mut pipeline = TokenPipeline::new(client);
    let theme = token_theme(&dark_theme());
    pipeline.set_theme(theme.clone());
    let rejected_language_ids = ["taide-broken-regex", "taide-cycle", "taide-misshapen"];
    let plugins = vec![
        plugin(rejected_language_ids[0], BROKEN_REGEX_GRAMMAR),
        plugin(rejected_language_ids[1], CYCLE_GRAMMAR),
        plugin(rejected_language_ids[2], MISSHAPEN_GRAMMAR),
        plugin(INI_LANGUAGE_ID, INI_GRAMMAR),
    ];
    pipeline.set_plugin_grammars(plugins.clone());
    let mut store = store();
    let rejected = rejected_language_ids.map(|language_id| {
        open(
            &mut store,
            &format!("/synthetic/rejected.{language_id}"),
            language_id,
            REJECTED_SOURCE,
        )
    });
    let ini = open(
        &mut store,
        "/synthetic/app.taide-ini",
        INI_LANGUAGE_ID,
        INI_SOURCE,
    );
    let json = open(&mut store, "/synthetic/data.json", "json", JSON_SOURCE);
    for document in rejected.into_iter().chain([ini, json]) {
        pipeline.open(snapshot(&store, document));
    }
    settle(&mut pipeline, &worker);
    assert!(pipeline.is_worker_running());
    assert!(pipeline.configuration_error().is_none());
    assert!(pipeline.style_table().is_some());
    for document in rejected {
        assert_eq!(
            spans(&pipeline, document),
            [[0, UNSTYLED_STYLE_ID], [0, UNSTYLED_STYLE_ID]]
        );
    }
    assert_eq!(
        spans(&pipeline, ini),
        plugin_spans(
            &CORE_LANGUAGE_IDS,
            &plugins[3..],
            &theme,
            INI_LANGUAGE_ID,
            INI_SOURCE
        )
    );
    assert_eq!(
        spans(&pipeline, json),
        direct_spans(
            &CORE_LANGUAGE_IDS,
            &theme,
            TokenizerLimits::default(),
            "json",
            JSON_SOURCE
        )
    );
}

use std::time::Instant;

use regress::Regex;
use serde_json::json;
use taide_model::ids::TabId;
use taide_native_editor::find::{FIND_SEARCH_TIMEOUT, FindOptions, FindQuery};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_syntax::MonacoFindPatternCompiler;

const LINE_COUNT: usize = 300_000;
const DOCUMENT_BYTES: usize = 20 * 1024 * 1024;
const MATCH_LIMIT: usize = 19_999;
const BACKTRACKING_LENGTH: usize = 32;
const PERFORMANCE_LIMIT_MS: f64 = 100.0;

fn large_document() -> String {
    let mut document = String::with_capacity(DOCUMENT_BYTES);
    let line_bytes = DOCUMENT_BYTES / LINE_COUNT;
    let longer_lines = DOCUMENT_BYTES % LINE_COUNT;
    for line in 0..LINE_COUNT {
        let content = format!("entry_{line:06} sample alpha beta gamma delta ");
        let length = line_bytes + usize::from(line < longer_lines);
        document.push_str(&content);
        document.extend(std::iter::repeat_n('a', length - content.len() - 1));
        document.push('\n');
    }
    assert_eq!(document.len(), DOCUMENT_BYTES);
    document
}

fn large() {
    let document = large_document();
    let no_match = Regex::with_flags(r"missing_\d+", "u").unwrap();
    let matching = Regex::with_flags(r"entry_\d+", "u").unwrap();
    let multiline = Regex::with_flags(r"\nmissing_\d+", "mu").unwrap();

    let started = Instant::now();
    let no_match_count = document
        .lines()
        .filter(|line| no_match.find(line).is_some())
        .count();
    let no_match_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(no_match_count, 0);

    let started = Instant::now();
    let limit_count = document
        .lines()
        .flat_map(|line| matching.find_iter(line))
        .take(MATCH_LIMIT)
        .count();
    let limit_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(limit_count, MATCH_LIMIT);

    let started = Instant::now();
    assert!(multiline.find(&document).is_none());
    let multiline_no_match_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        json!({
            "bytes": document.len(),
            "lines": document.lines().count(),
            "noMatchCount": no_match_count,
            "noMatchMs": no_match_ms,
            "limitCount": limit_count,
            "limitMs": limit_ms,
            "multilineNoMatchMs": multiline_no_match_ms,
        })
    );
}

fn backtracking() {
    let expression = Regex::with_flags("(a+)+b", "u").unwrap();
    let text = "a".repeat(BACKTRACKING_LENGTH);
    let started = Instant::now();
    println!("backtracking-start: {} bytes", text.len());
    let matched = expression.find(&text).is_some();
    println!(
        "{}",
        json!({ "matched": matched, "milliseconds": started.elapsed().as_secs_f64() * 1000.0 })
    );
}

fn model() {
    let text = large_document();
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: DOCUMENT_BYTES,
    })
    .unwrap();
    let id = store.restore_untitled(TabId::new(), Some(&text)).unwrap();
    let document = store.documents().snapshot(id).unwrap();
    let compiler = MonacoFindPatternCompiler;
    let no_match = FindQuery::new(
        r"missing_\d+",
        FindOptions {
            is_regex: true,
            ..Default::default()
        },
        &compiler,
    )
    .unwrap();
    let matching = FindQuery::new(
        r"entry_\d+",
        FindOptions {
            is_regex: true,
            ..Default::default()
        },
        &compiler,
    )
    .unwrap();
    let started = Instant::now();
    let no_match_result = no_match
        .find_matches(&document.rope, &[], MATCH_LIMIT, Some(FIND_SEARCH_TIMEOUT))
        .unwrap();
    let no_match_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(no_match_result.matches.is_empty());
    assert!(!no_match_result.timed_out);
    let started = Instant::now();
    let limit_result = matching
        .find_matches(&document.rope, &[], MATCH_LIMIT, Some(FIND_SEARCH_TIMEOUT))
        .unwrap();
    let limit_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(limit_result.matches.len(), MATCH_LIMIT);
    assert!(limit_result.limit_reached);
    assert!(!limit_result.timed_out);
    if !cfg!(debug_assertions) {
        assert!(
            no_match_ms <= PERFORMANCE_LIMIT_MS,
            "no match: {no_match_ms}ms"
        );
        assert!(limit_ms <= PERFORMANCE_LIMIT_MS, "limited: {limit_ms}ms");
    }
    println!(
        "{}",
        json!({
            "bytes": text.len(),
            "lines": LINE_COUNT,
            "noMatchCount": no_match_result.matches.len(),
            "noMatchMs": no_match_ms,
            "limitCount": limit_result.matches.len(),
            "limitMs": limit_ms,
        })
    );
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("large") => large(),
        Some("backtracking") => backtracking(),
        Some("model") => model(),
        _ => panic!("expected large, model or backtracking"),
    }
}

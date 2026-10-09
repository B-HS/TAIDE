use serde::Deserialize;
use taide_model::ids::TabId;
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::find::{FIND_MATCH_LIMIT, FindMatch, FindOptions, FindQuery};
use taide_native_editor::find_replacement::ReplacePattern;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_syntax::MonacoFindPatternCompiler;

const MAX_REPORTED_DIFFERENCES: usize = 20;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reference {
    model_cases: Vec<ModelCase>,
    replacement_cases: Vec<ReplacementCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplacementCase {
    input: String,
    is_regex: bool,
    preserve_case: bool,
    captures: Vec<Option<String>>,
    expected: String,
    has_captures: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelCase {
    text: String,
    source: String,
    is_regex: bool,
    match_case: bool,
    whole_word: bool,
    scope: (usize, usize),
    matches: Vec<ReferenceMatch>,
    navigation: Vec<Navigation>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ReferenceMatch {
    range: (usize, usize),
    captures: Vec<Option<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Navigation {
    start: usize,
    next: Option<ReferenceMatch>,
    previous: Option<ReferenceMatch>,
    previous_with_full_context: Option<ReferenceMatch>,
}

fn reference_match(found: FindMatch) -> ReferenceMatch {
    ReferenceMatch {
        range: (found.range.start, found.range.end),
        captures: found.captures,
    }
}

fn document(text: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: text.len().max(1),
    })
    .unwrap();
    let id = store.restore_untitled(TabId::new(), Some(text)).unwrap();
    store.documents().snapshot(id).unwrap()
}

#[test]
fn 이전_탐색은_커서_앞을_자른_가짜_줄끝이나_단어_경계를_만들지_않는다() {
    for (source, text, start, whole_word) in [
        ("^$", "cat", 0, false),
        ("cat$", "catapult", 3, false),
        ("cat", "catapult", 3, true),
    ] {
        let document = document(text);
        let query = FindQuery::new(
            source,
            FindOptions {
                is_regex: true,
                match_case: true,
                whole_word,
                ..Default::default()
            },
            &MonacoFindPatternCompiler,
        )
        .unwrap();
        assert!(
            query
                .find_previous(&document.rope, start, true, None)
                .unwrap()
                .is_none(),
            "{source} in {text}"
        );
    }
}

#[test]
fn 치환_패턴과_utf16_대소문자_지시와_보존이_monaco와_일치한다() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/find-reference.json")).unwrap();
    let mut differences = Vec::new();
    for case in reference.replacement_cases {
        let pattern = ReplacePattern::new(&case.input, case.is_regex);
        let actual = pattern.build(&case.captures, case.preserve_case);
        if actual != case.expected || pattern.has_captures() != case.has_captures {
            differences.push(format!(
                "{case:?}: actual {actual:?}, captures {}",
                pattern.has_captures()
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences: {:#?}",
        differences.len(),
        differences
            .iter()
            .take(MAX_REPORTED_DIFFERENCES)
            .collect::<Vec<_>>()
    );
}

#[test]
fn 찾기_범위와_다음_이전_탐색이_monaco_기준값과_일치한다() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/find-reference.json")).unwrap();
    assert!(!reference.model_cases.is_empty());
    let mut differences = Vec::new();
    let mut context_corrections = 0;
    for case in reference.model_cases {
        let query = FindQuery::new(
            &case.source,
            FindOptions {
                is_regex: case.is_regex,
                match_case: case.match_case,
                whole_word: case.whole_word,
                ..Default::default()
            },
            &MonacoFindPatternCompiler,
        );
        let document = document(&case.text);
        let rope = &document.rope;
        let actual = query
            .as_ref()
            .map(|query| {
                query
                    .find_matches(&rope, &[case.scope.0..case.scope.1], FIND_MATCH_LIMIT, None)
                    .unwrap()
                    .matches
                    .into_iter()
                    .map(reference_match)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if actual != case.matches {
            differences.push(format!("matches {case:?}: actual {actual:?}"));
        }
        for navigation in &case.navigation {
            context_corrections +=
                usize::from(navigation.previous != navigation.previous_with_full_context);
            let next = query
                .as_ref()
                .map(|query| {
                    query
                        .find_next(&rope, navigation.start, true, None)
                        .unwrap()
                        .map(reference_match)
                })
                .unwrap_or_default();
            let previous = query
                .as_ref()
                .map(|query| {
                    query
                        .find_previous(&rope, navigation.start, true, None)
                        .unwrap()
                        .map(reference_match)
                })
                .unwrap_or_default();
            if next != navigation.next || previous != navigation.previous_with_full_context {
                differences.push(format!(
                    "navigation {case:?} at {}: next {next:?}, previous {previous:?}",
                    navigation.start
                ));
            }
        }
    }
    assert!(context_corrections > 0);
    assert!(
        differences.is_empty(),
        "{} differences: {:#?}",
        differences.len(),
        differences
            .iter()
            .take(MAX_REPORTED_DIFFERENCES)
            .collect::<Vec<_>>()
    );
}

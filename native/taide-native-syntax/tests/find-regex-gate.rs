use std::collections::HashMap;

use regress::Regex;
use serde::Deserialize;

const EXPECTED_MONACO_VERSION: &str = "0.56.0";
const EXPECTED_VSCODE_REF: &str = "f487add297079a02eb836810185b165e50cadabc";
const MAX_REPORTED_DIFFERENCES: usize = 20;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reference {
    version: String,
    vscode_ref: String,
    engine_cases: Vec<EngineCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EngineCase {
    source: String,
    flags: String,
    text: String,
    start: usize,
    is_error: bool,
    captures: Option<Vec<Option<(usize, usize)>>>,
}

fn reference() -> Reference {
    serde_json::from_str(include_str!("fixtures/find-reference.json")).unwrap()
}

#[test]
fn 정규식_게이트는_고정된_monaco와_js_기준값을_포함한다() {
    let reference = reference();
    assert_eq!(reference.version, EXPECTED_MONACO_VERSION);
    assert_eq!(reference.vscode_ref, EXPECTED_VSCODE_REF);
    assert!(!reference.engine_cases.is_empty());
    assert!(reference.engine_cases.iter().any(|case| case.is_error));
    assert!(reference.engine_cases.iter().any(|case| case.start > 0));
}

#[test]
fn regress는_문법_오류와_unicode_플래그_시작위치_캡처가_js와_일치한다() {
    let mut compiled = HashMap::new();
    let mut differences = Vec::new();
    for case in reference().engine_cases {
        let expression = compiled
            .entry((case.source.clone(), case.flags.clone()))
            .or_insert_with(|| {
                Regex::with_flags(&case.source, case.flags.as_str())
                    .map_err(|error| error.to_string())
            });
        let actual = match expression {
            Ok(_) if case.is_error => {
                differences.push(format!("expected syntax error: {case:?}"));
                continue;
            }
            Err(_) if case.is_error => continue,
            Err(error) => {
                differences.push(format!("unexpected syntax error {error}: {case:?}"));
                continue;
            }
            Ok(expression) => expression
                .find_from(&case.text, case.start)
                .next()
                .map(|found| {
                    found
                        .groups()
                        .map(|group| group.map(|range| (range.start, range.end)))
                        .collect::<Vec<_>>()
                }),
        };
        if actual != case.captures {
            differences.push(format!("{case:?}: actual {actual:?}"));
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

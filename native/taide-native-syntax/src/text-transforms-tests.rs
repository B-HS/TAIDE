use super::*;
use serde::Deserialize;

const CASES: [TextCase; 7] = [
    TextCase::Upper,
    TextCase::Lower,
    TextCase::Title,
    TextCase::Snake,
    TextCase::Camel,
    TextCase::Pascal,
    TextCase::Kebab,
];

#[derive(Deserialize)]
struct Oracle {
    input: String,
    outputs: Vec<String>,
}

#[test]
fn 대소문자_일곱_변환은_실제_monaco_메서드의_유니코드_결과와_일치한다() {
    let transforms = MonacoTextTransforms::new().unwrap();
    let oracle: Vec<Oracle> =
        serde_json::from_str(include_str!("monaco-case-oracle.json")).unwrap();
    let mut differences = Vec::new();
    for row in oracle {
        for (case, expected) in CASES.into_iter().zip(row.outputs) {
            let actual = transforms.transform(case, &row.input);
            if actual != expected {
                differences.push(format!(
                    "{case:?}: {:?} expected {expected:?} actual {actual:?}",
                    row.input
                ));
            }
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

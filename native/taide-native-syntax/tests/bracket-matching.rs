use serde::Deserialize;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_syntax::monaco_language;

const VERSION: &str = "0.56.0";
const TAB_SIZE: u32 = 4;
const BYTE_LIMIT: usize = 1024 * 1024;
const CONFIGURATIONS: usize = 23;
const REFERENCE_CASES: usize = 483;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Reference {
    version: String,
    tab_size: u32,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    language_id: String,
    text: String,
    queries: Vec<Query>,
}

#[derive(Deserialize)]
struct Query {
    byte: usize,
    near: Option<[[usize; 2]; 2]>,
    enclosing: Option<[[usize; 2]; 2]>,
}

#[test]
fn 원본_23언어_483표본의_모든_유효_커서_위치에서_near와_enclosing이_일치한다() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/bracket-matching-reference.json")).unwrap();
    assert_eq!(reference.version, VERSION);
    assert_eq!(reference.tab_size, TAB_SIZE);
    assert_eq!(reference.cases.len(), REFERENCE_CASES);
    assert_eq!(
        reference
            .cases
            .iter()
            .map(|case| &case.language_id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        CONFIGURATIONS
    );
    for case in reference.cases {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = store
            .open_file(
                "/synthetic/bracket-match.txt".into(),
                OpenedFile {
                    path: "/synthetic/bracket-match.txt".into(),
                    content: case.text.clone(),
                    language_id: case.language_id.clone(),
                    byte_size: case.text.len().try_into().unwrap(),
                    line_count: case.text.lines().count().try_into().unwrap(),
                    tier: FileSizeTier::Normal,
                    read_only: false,
                    encoding_lossy: false,
                    modified_ms: 1.0,
                    editor_config: EditorConfigOptions::default(),
                },
            )
            .unwrap();
        let rules = monaco_language(&case.language_id).unwrap().unwrap();
        let model = store
            .bracket_model(document, Some(rules), TAB_SIZE, None)
            .unwrap();
        for query in case.queries {
            let expected = query.near.or(query.enclosing);
            let actual = model.matching_brackets(query.byte);
            assert_eq!(
                actual.as_ref().map(|pair| [
                    [pair.open.start, pair.open.end],
                    [pair.close.start, pair.close.end]
                ]),
                expected,
                "{} {:?} byte {}",
                case.language_id,
                case.text,
                query.byte
            );
            if let Some(actual) = actual {
                assert_eq!(
                    actual.is_near,
                    query.near.is_some(),
                    "{} {:?} byte {}",
                    case.language_id,
                    case.text,
                    query.byte
                );
            }
        }
    }
}

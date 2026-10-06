use serde_json::json;

use super::PluginGrammar;

const LANGUAGE_ID: &str = "taide-x";
const UNREGISTERED_SOURCES: [&str; 10] = [
    "",
    "{",
    "[]",
    "null",
    "\"source.taide-x\"",
    r#"{"patterns":[]}"#,
    r#"{"scopeName":""}"#,
    r#"{"scopeName":7}"#,
    r#"{"scopeName":null}"#,
    "\u{feff}{\"scopeName\":\"source.taide-x\"}",
];
const LOOSE_SOURCE: &str = r#"{
    "scopeName": "source.taide-x",
    "name": "Other",
    "fileTypes": "x",
    "firstLineMatch": 7,
    "patterns": "oops",
    "repository": [{ "match": "a" }, { "match": "b" }],
    "injectionSelector": "L:source.js",
    "uuid": "u"
}"#;
const WELL_FORMED_SOURCE: &str = r##"{
    "scopeName": " ",
    "patterns": [{ "include": "#r" }],
    "repository": { "r": { "match": "b" } }
}"##;
const NULL_REPOSITORY_SOURCE: &str = r#"{ "scopeName": "source.taide-x", "repository": null }"#;

#[test]
fn json이_아니거나_객체가_아니거나_scope_이름이_빈_문법은_등록하지_않는다() {
    for source in UNREGISTERED_SOURCES {
        assert!(
            PluginGrammar::from_contribution(LANGUAGE_ID, &[], source).is_none(),
            "{source:?}"
        );
    }
}

#[test]
fn 등록은_언어_id를_이름으로_삼고_patterns와_repository를_ts와_같은_형으로_맞춘다() {
    let embedded = ["rust".to_owned(), "no-such".to_owned()];
    let loose = PluginGrammar::from_contribution(LANGUAGE_ID, &embedded, LOOSE_SOURCE).unwrap();
    assert_eq!(loose.language_id(), LANGUAGE_ID);
    assert_eq!(loose.scope_name(), "source.taide-x");
    assert_eq!(loose.embedded_languages(), embedded);
    assert_eq!(
        *loose.registration(),
        json!({
            "scopeName": "source.taide-x",
            "name": LANGUAGE_ID,
            "patterns": [],
            "repository": { "0": { "match": "a" }, "1": { "match": "b" } },
            "injectionSelector": "L:source.js",
            "uuid": "u",
        })
    );
    assert!(loose.raw_grammar().is_some());

    let well_formed =
        PluginGrammar::from_contribution(LANGUAGE_ID, &[], WELL_FORMED_SOURCE).unwrap();
    assert_eq!(well_formed.scope_name(), " ");
    assert!(well_formed.embedded_languages().is_empty());
    assert_eq!(
        *well_formed.registration(),
        json!({
            "scopeName": " ",
            "name": LANGUAGE_ID,
            "patterns": [{ "include": "#r" }],
            "repository": { "r": { "match": "b" } },
        })
    );

    let without_repository =
        PluginGrammar::from_contribution(LANGUAGE_ID, &[], NULL_REPOSITORY_SOURCE).unwrap();
    assert_eq!(
        *without_repository.registration(),
        json!({
            "scopeName": "source.taide-x",
            "name": LANGUAGE_ID,
            "patterns": [],
            "repository": {},
        })
    );
}

#[test]
fn 같은_내용의_등록은_같다고_보고_문법이나_끌어오는_언어가_다르면_다르다고_본다() {
    let first = PluginGrammar::from_contribution(LANGUAGE_ID, &[], WELL_FORMED_SOURCE);
    assert_eq!(
        first,
        PluginGrammar::from_contribution(LANGUAGE_ID, &[], WELL_FORMED_SOURCE)
    );
    assert_ne!(
        first,
        PluginGrammar::from_contribution(LANGUAGE_ID, &[], NULL_REPOSITORY_SOURCE)
    );
    assert_ne!(
        first,
        PluginGrammar::from_contribution(LANGUAGE_ID, &["rust".to_owned()], WELL_FORMED_SOURCE)
    );
}

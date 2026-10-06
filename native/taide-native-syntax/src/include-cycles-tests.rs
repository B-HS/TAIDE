use serde_json::{Value, json};

use super::{IncludeSource, plugins_on_include_only_cycles};

const SCOPE_NAME_KEY: &str = "scopeName";

fn rejected(bundled: &[Value], plugins: &[Value]) -> Vec<usize> {
    let scope_name =
        |grammar: &'_ Value| -> String { grammar[SCOPE_NAME_KEY].as_str().unwrap().to_owned() };
    let scope_names: Vec<String> = bundled.iter().chain(plugins).map(scope_name).collect();
    let sources: Vec<IncludeSource<'_>> = bundled
        .iter()
        .map(|grammar| (grammar, None))
        .chain(
            plugins
                .iter()
                .enumerate()
                .map(|(index, grammar)| (grammar, Some(index))),
        )
        .zip(&scope_names)
        .map(|((grammar, plugin), scope_name)| IncludeSource {
            scope_name,
            grammar,
            plugin,
        })
        .collect();
    plugins_on_include_only_cycles(&sources)
        .into_iter()
        .collect()
}

fn plugin(patterns: Value, repository: Value) -> Value {
    json!({ "scopeName": "source.taide-a", "patterns": patterns, "repository": repository })
}

#[test]
fn include만으로_자기에게_돌아오는_규칙이_있는_플러그인_문법을_찾는다() {
    let cyclic = [
        plugin(json!([{ "include": "$self" }]), json!({})),
        plugin(json!([{ "include": "$base" }]), json!({})),
        plugin(
            json!([{ "include": "#a" }]),
            json!({
                "a": { "patterns": [{ "include": "#b" }] },
                "b": { "patterns": [{ "match": "x" }, { "include": "#a" }] },
            }),
        ),
        plugin(
            json!([{ "include": "#a" }]),
            json!({ "a": [{ "include": "#b" }], "b": { "include": "#a" } }),
        ),
        plugin(
            json!([{ "patterns": [{ "patterns": [{ "include": "source.taide-a" }] }] }]),
            json!({}),
        ),
        plugin(
            json!([{ "include": "source.taide-a#a" }]),
            json!({ "a": { "include": "source.taide-a" } }),
        ),
        plugin(
            json!([{ "match": "x", "captures": { "1": { "patterns": [{ "include": "#loop" }] } } }]),
            json!({ "loop": { "patterns": [{ "include": "#loop" }] } }),
        ),
        plugin(
            json!([{ "begin": "x", "end": "y", "patterns": [{ "include": "#loop" }] }]),
            json!({ "loop": { "name": "meta.loop", "include": "#loop" } }),
        ),
        plugin(
            json!([{ "include": "#outer" }]),
            json!({
                "outer": {
                    "repository": { "inner": { "include": "#outer" } },
                    "patterns": [{ "include": "#inner" }],
                },
            }),
        ),
        json!({
            "scopeName": "source.taide-a",
            "injections": { "L:source.taide-a": { "patterns": [{ "include": "#loop" }] } },
            "repository": { "loop": { "patterns": [{ "include": "#loop" }] } },
        }),
    ];
    for grammar in cyclic {
        assert_eq!(
            rejected(&[], std::slice::from_ref(&grammar)),
            [0],
            "{grammar}"
        );
    }
}

#[test]
fn begin이나_match가_있는_규칙을_거치는_재귀와_없는_규칙을_가리키는_include는_순환이_아니다() {
    let acyclic = [
        plugin(json!([]), json!({})),
        plugin(
            json!([{ "include": "#group" }, { "include": "#missing" }]),
            json!({
                "group": {
                    "begin": "\\(",
                    "end": "\\)",
                    "patterns": [{ "include": "$self" }, { "include": "$base" }, { "include": "#group" }],
                },
            }),
        ),
        plugin(
            json!([{ "match": "x", "patterns": [{ "include": "$self" }] }]),
            json!({}),
        ),
        plugin(
            json!([{ "include": "#a" }, { "include": "#a" }]),
            json!({
                "a": { "patterns": [{ "include": "#b" }, { "include": "#c" }] },
                "b": { "patterns": [{ "include": "#c" }] },
                "c": { "patterns": [{ "match": "x" }] },
            }),
        ),
        plugin(
            json!([{ "include": "source.taide-missing" }, { "include": "source.taide-a#missing" }]),
            json!({}),
        ),
        plugin(
            json!([{ "include": "#a" }]),
            json!({ "a": { "patterns": null, "include": "#b" }, "b": { "begin": "x", "include": "#a" } }),
        ),
        json!({ "scopeName": "source.taide-a", "patterns": 7, "repository": [] }),
    ];
    for grammar in acyclic {
        assert_eq!(
            rejected(&[], std::slice::from_ref(&grammar)),
            [0usize; 0],
            "{grammar}"
        );
    }
}

#[test]
fn 여러_문법에_걸친_순환은_고리에_든_플러그인_문법만_골라낸다() {
    let first = json!({
        "scopeName": "source.taide-first",
        "patterns": [{ "include": "source.taide-second" }],
    });
    let second = json!({
        "scopeName": "source.taide-second",
        "patterns": [{ "include": "source.taide-first" }],
    });
    let bystander = json!({
        "scopeName": "source.taide-bystander",
        "patterns": [{ "include": "source.taide-first" }, { "include": "source.json" }],
    });
    let json_like = json!({
        "scopeName": "source.json",
        "patterns": [{ "include": "#value" }],
        "repository": { "value": { "patterns": [{ "match": "true" }] } },
    });
    assert_eq!(
        rejected(
            std::slice::from_ref(&json_like),
            &[bystander.clone(), first.clone(), second]
        ),
        [1, 2]
    );
    assert_eq!(
        rejected(std::slice::from_ref(&json_like), &[bystander, first]),
        [0usize; 0]
    );
}

#[test]
fn 번들_문법을_거쳐_돌아오는_순환도_플러그인_문법의_것으로_본다() {
    let erb_like = json!({
        "scopeName": "text.html.erb",
        "patterns": [{ "include": "text.html.basic" }],
    });
    let html_like = json!({
        "scopeName": "text.html.basic",
        "patterns": [{ "match": "<" }],
    });
    let html_plugin = json!({
        "scopeName": "text.html.basic",
        "patterns": [{ "include": "#content" }],
        "repository": { "content": { "patterns": [{ "match": "<" }, { "include": "$base" }] } },
    });
    let bundled = [erb_like, html_like];
    assert_eq!(rejected(&bundled, std::slice::from_ref(&html_plugin)), [0]);

    let through_bundled = json!({
        "scopeName": "source.taide-a",
        "patterns": [{ "include": "text.html.erb" }],
    });
    let back_to_plugin = json!({
        "scopeName": "text.html.basic",
        "patterns": [{ "include": "source.taide-a" }],
    });
    assert_eq!(
        rejected(&bundled, &[through_bundled.clone(), back_to_plugin]),
        [0, 1]
    );
    assert_eq!(rejected(&bundled, &[through_bundled]), [0usize; 0]);
}

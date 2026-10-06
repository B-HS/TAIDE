use std::collections::BTreeSet;

use super::{
    GRAMMAR_SOURCES, bundled_grammar_set, bundled_language_ids, bundled_registrations, manifest,
};
use crate::tokenizer::SyntaxError;

const TAIDE_LANGUAGE_IDS: [&str; 31] = [
    "rust",
    "typescript",
    "typescriptreact",
    "javascript",
    "javascriptreact",
    "json",
    "jsonc",
    "markdown",
    "toml",
    "yaml",
    "html",
    "css",
    "scss",
    "python",
    "go",
    "shellscript",
    "java",
    "ruby",
    "erb",
    "dart",
    "swift",
    "scala",
    "elixir",
    "heex",
    "haskell",
    "c",
    "cpp",
    "kotlin",
    "lua",
    "zig",
    "hcl",
];
const HTML_SCOPE_NAME: &str = "text.html.basic";
const HTML_GRAMMAR_COUNT: usize = 3;
const RUBY_GRAMMAR_COUNT: usize = 20;
const SCOPE_NAME_KEY: &str = "scopeName";

fn scope_names(sources: &[&str]) -> BTreeSet<String> {
    sources
        .iter()
        .map(|source| {
            let grammar: serde_json::Value = serde_json::from_str(source).unwrap();
            grammar[SCOPE_NAME_KEY].as_str().unwrap().to_owned()
        })
        .collect()
}

#[test]
fn 번들_언어_목록은_ts의_언어_목록과_순서까지_같다() {
    assert_eq!(bundled_language_ids().unwrap(), TAIDE_LANGUAGE_IDS);
}

#[test]
fn 포함된_문법_파일과_매니페스트의_문법_목록과_scope_이름이_일치한다() {
    let manifest = manifest().unwrap();
    let listed: BTreeSet<&str> = manifest
        .grammars
        .iter()
        .map(|grammar| grammar.id.as_str())
        .collect();
    let embedded: BTreeSet<&str> = GRAMMAR_SOURCES.iter().map(|(id, _)| *id).collect();
    assert_eq!(listed, embedded);
    assert_eq!(GRAMMAR_SOURCES.len(), embedded.len());
    for (id, source) in GRAMMAR_SOURCES {
        let listed_scope = &manifest
            .grammars
            .iter()
            .find(|grammar| grammar.id == *id)
            .unwrap()
            .scope_name;
        assert_eq!(
            scope_names(&[source]),
            BTreeSet::from([listed_scope.clone()])
        );
    }
}

#[test]
fn 언어를_요청하면_그_문법이_끌어오는_문법까지_함께_돌려준다() {
    let html = bundled_grammar_set(&["html"]).unwrap();
    assert_eq!(html.grammar_sources.len(), HTML_GRAMMAR_COUNT);
    assert_eq!(
        scope_names(&html.grammar_sources),
        BTreeSet::from([
            HTML_SCOPE_NAME.to_owned(),
            "source.js".to_owned(),
            "source.css".to_owned()
        ])
    );
    assert_eq!(html.languages.len(), 1);
    assert_eq!(html.languages[0].language_id, "html");
    assert_eq!(html.languages[0].scope_name, HTML_SCOPE_NAME);

    let ruby = bundled_grammar_set(&["ruby"]).unwrap();
    assert_eq!(ruby.grammar_sources.len(), RUBY_GRAMMAR_COUNT);
}

#[test]
fn 이름만_다른_언어는_같은_문법_하나를_가리킨다() {
    let set = bundled_grammar_set(&["heex", "html", "typescriptreact"]).unwrap();
    let scopes: Vec<&str> = set
        .languages
        .iter()
        .map(|language| language.scope_name)
        .collect();
    assert_eq!(scopes, [HTML_SCOPE_NAME, HTML_SCOPE_NAME, "source.tsx"]);
    assert_eq!(set.grammar_sources.len(), HTML_GRAMMAR_COUNT + 1);
}

#[test]
fn 번들에_없는_언어는_오류로_돌려준다() {
    assert_eq!(
        bundled_grammar_set(&["plaintext"]).unwrap_err(),
        SyntaxError::UnknownLanguage("plaintext".to_owned())
    );
    assert_eq!(
        bundled_registrations(&["json", "plaintext"]).unwrap_err(),
        SyntaxError::UnknownLanguage("plaintext".to_owned())
    );
}

#[test]
fn 등록_목록은_ts가_문법을_싣는_순서로_끌어오는_문법을_앞에_두고_이름만_다른_언어를_따로_둔다() {
    let names = |requested: &[&str]| -> Vec<(&str, &str)> {
        bundled_registrations(requested)
            .unwrap()
            .iter()
            .map(|registration| (registration.name, registration.scope_name))
            .collect()
    };
    assert_eq!(
        names(&["heex", "html", "typescriptreact", "heex"]),
        [
            ("javascript", "source.js"),
            ("css", "source.css"),
            ("heex", HTML_SCOPE_NAME),
            ("html", HTML_SCOPE_NAME),
            ("typescriptreact", "source.tsx"),
        ]
    );
    assert_eq!(
        names(&["cpp", "c"]),
        [
            ("regexp", "source.regexp.python"),
            ("c", "source.c"),
            ("glsl", "source.glsl"),
            ("cpp-macro", "source.cpp.embedded.macro"),
            ("cpp", "source.cpp"),
        ]
    );
    let ruby = bundled_registrations(&["ruby"]).unwrap();
    assert_eq!(ruby.len(), RUBY_GRAMMAR_COUNT);
    assert_eq!(ruby[RUBY_GRAMMAR_COUNT - 1].name, "ruby");
    assert_eq!(ruby[RUBY_GRAMMAR_COUNT - 1].aliases, ["rb"]);
    let shell = ruby
        .iter()
        .find(|registration| registration.name == "shellscript")
        .unwrap();
    assert_eq!(shell.aliases, ["bash", "sh", "shell", "zsh"]);
    assert_eq!(
        scope_names(&[shell.source]),
        BTreeSet::from([shell.scope_name.to_owned()])
    );
}

use std::collections::BTreeMap;

use super::{LanguageRegistration, LoadedGrammars, loaded_grammars};

const JSON_SCOPE: &str = "source.json";
const MARKDOWN_SCOPE: &str = "text.html.markdown";
const PLUGIN_SCOPE: &str = "source.taide-json";
const HTML_SCOPE: &str = "text.html.basic";
const SHELL_SCOPE: &str = "source.shell";
const PLUGIN_SHELL_SCOPE: &str = "source.taide-bash";

fn registration<'a>(name: &'a str, scope_name: &'a str) -> LanguageRegistration<'a> {
    LanguageRegistration {
        name,
        scope_name,
        aliases: &[],
    }
}

fn indexes(entries: &[(&str, usize)]) -> BTreeMap<String, usize> {
    entries
        .iter()
        .map(|(scope_name, index)| ((*scope_name).to_owned(), *index))
        .collect()
}

fn languages(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(language_id, scope_name)| ((*language_id).to_owned(), (*scope_name).to_owned()))
        .collect()
}

#[test]
fn 이름이_같으면_뒤에_등록한_문법이_그_언어를_차지하고_앞_문법의_scope는_include용으로_남는다() {
    assert_eq!(
        loaded_grammars(&[
            registration("json", JSON_SCOPE),
            registration("markdown", MARKDOWN_SCOPE),
            registration("json", PLUGIN_SCOPE),
        ]),
        LoadedGrammars {
            root_by_scope_name: indexes(&[(PLUGIN_SCOPE, 2), (MARKDOWN_SCOPE, 1)]),
            included_by_scope_name: indexes(&[
                (JSON_SCOPE, 0),
                (PLUGIN_SCOPE, 2),
                (MARKDOWN_SCOPE, 1)
            ]),
            scope_name_by_language_id: languages(&[
                ("json", PLUGIN_SCOPE),
                ("markdown", MARKDOWN_SCOPE)
            ]),
        }
    );
}

#[test]
fn scope만_같으면_먼저_실린_문법이_문서용이_되고_나중에_실린_문법이_include용이_된다() {
    assert_eq!(
        loaded_grammars(&[
            registration("json", JSON_SCOPE),
            registration("taide-data", JSON_SCOPE),
        ]),
        LoadedGrammars {
            root_by_scope_name: indexes(&[(JSON_SCOPE, 0)]),
            included_by_scope_name: indexes(&[(JSON_SCOPE, 1)]),
            scope_name_by_language_id: languages(&[
                ("json", JSON_SCOPE),
                ("taide-data", JSON_SCOPE)
            ]),
        }
    );
}

#[test]
fn 이름과_scope가_모두_같으면_뒤에_등록한_문법만_남는다() {
    assert_eq!(
        loaded_grammars(&[
            registration("json", JSON_SCOPE),
            registration("json", JSON_SCOPE),
        ]),
        LoadedGrammars {
            root_by_scope_name: indexes(&[(JSON_SCOPE, 1)]),
            included_by_scope_name: indexes(&[(JSON_SCOPE, 1)]),
            scope_name_by_language_id: languages(&[("json", JSON_SCOPE)]),
        }
    );
}

#[test]
fn 같은_이름의_문법은_처음_등록된_자리에서_실려_scope가_같은_다른_이름보다_앞설_수_있다() {
    let plugin_first = loaded_grammars(&[
        registration("html", HTML_SCOPE),
        registration("heex", HTML_SCOPE),
        registration("html", HTML_SCOPE),
    ]);
    assert_eq!(plugin_first.root_by_scope_name, indexes(&[(HTML_SCOPE, 2)]));
    assert_eq!(
        plugin_first.included_by_scope_name,
        indexes(&[(HTML_SCOPE, 1)])
    );

    let bundled_first = loaded_grammars(&[
        registration("heex", HTML_SCOPE),
        registration("html", HTML_SCOPE),
        registration("html", HTML_SCOPE),
    ]);
    assert_eq!(
        bundled_first.root_by_scope_name,
        indexes(&[(HTML_SCOPE, 0)])
    );
    assert_eq!(
        bundled_first.included_by_scope_name,
        indexes(&[(HTML_SCOPE, 2)])
    );
    assert_eq!(
        bundled_first.scope_name_by_language_id,
        languages(&[("heex", HTML_SCOPE), ("html", HTML_SCOPE)])
    );
}

#[test]
fn 이미_실린_문법의_별칭과_이름이_같은_문법은_싣지_않고_그_이름은_별칭의_문법을_가리킨다() {
    let aliases = ["bash".to_owned(), "sh".to_owned()];
    let shell = LanguageRegistration {
        name: "shellscript",
        scope_name: SHELL_SCOPE,
        aliases: &aliases,
    };
    assert_eq!(
        loaded_grammars(&[shell, registration("bash", PLUGIN_SHELL_SCOPE)]),
        LoadedGrammars {
            root_by_scope_name: indexes(&[(SHELL_SCOPE, 0)]),
            included_by_scope_name: indexes(&[(SHELL_SCOPE, 0), (PLUGIN_SHELL_SCOPE, 1)]),
            scope_name_by_language_id: languages(&[
                ("shellscript", SHELL_SCOPE),
                ("bash", SHELL_SCOPE),
                ("sh", SHELL_SCOPE),
            ]),
        }
    );
    assert_eq!(
        loaded_grammars(&[registration("bash", PLUGIN_SHELL_SCOPE)]).scope_name_by_language_id,
        languages(&[("bash", PLUGIN_SHELL_SCOPE)])
    );
}

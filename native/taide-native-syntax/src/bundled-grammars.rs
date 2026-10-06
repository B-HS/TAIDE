use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::textmate_tokenizer::{GrammarSet, LanguageGrammar};
use crate::tokenizer::SyntaxError;

macro_rules! bundled_grammar_source {
    ($id:literal) => {
        (
            $id,
            include_str!(concat!("../grammars/", $id, ".tmLanguage.json")),
        )
    };
}

const MANIFEST_JSON: &str = include_str!("../grammars/manifest.json");
const GRAMMAR_SOURCES: &[(&str, &str)] = &[
    bundled_grammar_source!("c"),
    bundled_grammar_source!("cpp"),
    bundled_grammar_source!("cpp-macro"),
    bundled_grammar_source!("css"),
    bundled_grammar_source!("dart"),
    bundled_grammar_source!("elixir"),
    bundled_grammar_source!("erb"),
    bundled_grammar_source!("glsl"),
    bundled_grammar_source!("go"),
    bundled_grammar_source!("graphql"),
    bundled_grammar_source!("haml"),
    bundled_grammar_source!("haskell"),
    bundled_grammar_source!("hcl"),
    bundled_grammar_source!("html"),
    bundled_grammar_source!("java"),
    bundled_grammar_source!("javascript"),
    bundled_grammar_source!("json"),
    bundled_grammar_source!("jsonc"),
    bundled_grammar_source!("jsx"),
    bundled_grammar_source!("kotlin"),
    bundled_grammar_source!("lua"),
    bundled_grammar_source!("markdown"),
    bundled_grammar_source!("python"),
    bundled_grammar_source!("regexp"),
    bundled_grammar_source!("ruby"),
    bundled_grammar_source!("rust"),
    bundled_grammar_source!("scala"),
    bundled_grammar_source!("scss"),
    bundled_grammar_source!("shellscript"),
    bundled_grammar_source!("sql"),
    bundled_grammar_source!("swift"),
    bundled_grammar_source!("toml"),
    bundled_grammar_source!("tsx"),
    bundled_grammar_source!("typescript"),
    bundled_grammar_source!("xml"),
    bundled_grammar_source!("yaml"),
    bundled_grammar_source!("zig"),
];

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    languages: Vec<ManifestLanguage>,
    grammars: Vec<ManifestGrammar>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestLanguage {
    language_id: String,
    grammar_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestGrammar {
    id: String,
    scope_name: String,
    aliases: Vec<String>,
    imports: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BundledRegistration {
    pub name: &'static str,
    pub scope_name: &'static str,
    pub aliases: &'static [String],
    pub source: &'static str,
}

fn manifest() -> Result<&'static Manifest, SyntaxError> {
    static MANIFEST: OnceLock<Result<Manifest, SyntaxError>> = OnceLock::new();
    MANIFEST
        .get_or_init(|| {
            serde_json::from_str(MANIFEST_JSON)
                .map_err(|error| SyntaxError::InvalidGrammar(error.to_string()))
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn grammar(
    manifest: &'static Manifest,
    grammar_id: &str,
) -> Result<&'static ManifestGrammar, SyntaxError> {
    manifest
        .grammars
        .iter()
        .find(|grammar| grammar.id == grammar_id)
        .ok_or_else(|| SyntaxError::InvalidGrammar(grammar_id.to_owned()))
}

fn grammar_source(grammar_id: &str) -> Result<&'static str, SyntaxError> {
    GRAMMAR_SOURCES
        .iter()
        .find(|(id, _)| *id == grammar_id)
        .map(|(_, source)| *source)
        .ok_or_else(|| SyntaxError::InvalidGrammar(grammar_id.to_owned()))
}

pub fn bundled_language_ids() -> Result<Vec<&'static str>, SyntaxError> {
    Ok(manifest()?
        .languages
        .iter()
        .map(|language| language.language_id.as_str())
        .collect())
}

pub fn bundled_grammar_set(
    requested_language_ids: &[&str],
) -> Result<GrammarSet<'static>, SyntaxError> {
    let manifest = manifest()?;
    let mut languages = Vec::with_capacity(requested_language_ids.len());
    let mut grammar_ids = BTreeSet::new();
    let mut pending = Vec::new();
    for requested in requested_language_ids {
        let language = manifest
            .languages
            .iter()
            .find(|language| language.language_id == *requested)
            .ok_or_else(|| SyntaxError::UnknownLanguage((*requested).to_owned()))?;
        languages.push(LanguageGrammar {
            language_id: language.language_id.as_str(),
            scope_name: grammar(manifest, &language.grammar_id)?.scope_name.as_str(),
        });
        pending.push(language.grammar_id.as_str());
    }
    while let Some(grammar_id) = pending.pop() {
        if !grammar_ids.insert(grammar_id) {
            continue;
        }
        pending.extend(
            grammar(manifest, grammar_id)?
                .imports
                .iter()
                .map(String::as_str),
        );
    }
    let grammar_sources = grammar_ids
        .into_iter()
        .map(grammar_source)
        .collect::<Result<_, _>>()?;
    Ok(GrammarSet {
        grammar_sources,
        languages,
    })
}

fn register(
    manifest: &'static Manifest,
    grammar_id: &'static str,
    name: &'static str,
    registered: &mut BTreeSet<(&'static str, &'static str)>,
    registrations: &mut Vec<BundledRegistration>,
) -> Result<(), SyntaxError> {
    if !registered.insert((grammar_id, name)) {
        return Ok(());
    }
    let listed = grammar(manifest, grammar_id)?;
    for import in &listed.imports {
        register(manifest, import, import, registered, registrations)?;
    }
    registrations.push(BundledRegistration {
        name,
        scope_name: &listed.scope_name,
        aliases: &listed.aliases,
        source: grammar_source(grammar_id)?,
    });
    Ok(())
}

pub(crate) fn bundled_registrations(
    requested_language_ids: &[&str],
) -> Result<Vec<BundledRegistration>, SyntaxError> {
    let manifest = manifest()?;
    let mut registered = BTreeSet::new();
    let mut registrations = Vec::new();
    for requested in requested_language_ids {
        let language = manifest
            .languages
            .iter()
            .find(|language| language.language_id == *requested)
            .ok_or_else(|| SyntaxError::UnknownLanguage((*requested).to_owned()))?;
        register(
            manifest,
            &language.grammar_id,
            &language.language_id,
            &mut registered,
            &mut registrations,
        )?;
    }
    Ok(registrations)
}

#[cfg(test)]
#[path = "bundled-grammars-tests.rs"]
mod tests;

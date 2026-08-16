use std::path::Path;

use crate::domain::plugin::types::LoadedPlugin;

/// Canonical extension → language id table, the single source every extension-based language
/// detector (`file::service::open_file`, `git::service::diff_staged_text`,
/// `ide::service::guess_language_id`) now calls through instead of keeping its own copy (contract
/// §3.4/D1 — 4 previously duplicated tables: 3 in Rust, 1 hand-ported in
/// `shared/lib/language-from-path.ts`, which stays a separate hand-kept port since the frontend has
/// no IPC round-trip for a git blob diff's language — see that file's own doc comment).
///
/// Every value here is deliberately a member of the frontend's static `TAIDE_LANGUAGE_IDS`
/// (`shared/lib/shiki/lang-map.ts`) — the finite set of languages a bundled grammar actually
/// exists for. Before this unification, `ide::service`'s own copy additionally mapped `cs`→`csharp`,
/// `php`→`php`, and `sql`→`sql`, none of which have a grammar loader; consolidating onto this table
/// intentionally drops those three (they now resolve to `plaintext`, consistent with what
/// `file::service`/`git::service` already returned for them) rather than propagating three
/// dead-end language ids into every consumer. A *plugin*-contributed language id has no such
/// constraint — `plugin_overlay` below returns whatever id a plugin declares verbatim, since
/// Wave I's monaco/shiki dynamic-registration path (contract §3.4/C1) loads those at runtime rather
/// than from this static table.
const LANGUAGE_ID_BY_EXTENSION: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("ts", "typescript"),
    ("tsx", "typescriptreact"),
    ("js", "javascript"),
    ("jsx", "javascriptreact"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("json", "json"),
    ("jsonc", "jsonc"),
    ("md", "markdown"),
    ("mdx", "markdown"),
    ("toml", "toml"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("html", "html"),
    ("css", "css"),
    ("scss", "scss"),
    ("py", "python"),
    ("go", "go"),
    ("sh", "shellscript"),
    ("bash", "shellscript"),
    ("zsh", "shellscript"),
    ("java", "java"),
    ("rb", "ruby"),
    ("erb", "erb"),
    ("dart", "dart"),
    ("swift", "swift"),
    ("scala", "scala"),
    ("sbt", "scala"),
    ("ex", "elixir"),
    ("exs", "elixir"),
    ("heex", "heex"),
    ("hs", "haskell"),
    ("lhs", "haskell"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("hpp", "cpp"),
    ("cc", "cpp"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("lua", "lua"),
    ("zig", "zig"),
    ("tf", "hcl"),
];

const DEFAULT_LANGUAGE_ID: &str = "plaintext";

fn extension_of(path: &Path) -> Option<String> {
    path.extension().and_then(|value| value.to_str()).map(str::to_lowercase)
}

fn strip_leading_dot(value: &str) -> &str {
    value.strip_prefix('.').unwrap_or(value)
}

/// Checks every enabled plugin's `contributes.languages` for an extension match, returning the
/// first plugin-declared language id found (plugins are scanned in the same order
/// `plugin::service::load_plugins` returns them — directory-sorted, so the match is deterministic).
fn plugin_overlay(extension: &str, plugins: &[LoadedPlugin]) -> Option<String> {
    plugins
        .iter()
        .filter(|plugin| plugin.enabled)
        .flat_map(|plugin| &plugin.manifest.contributes.languages)
        .find(|language| {
            language
                .extensions
                .iter()
                .any(|candidate| strip_leading_dot(candidate) == extension)
        })
        .map(|language| language.id.clone())
}

/// Resolves a file path's language id — an enabled plugin's `contributes.languages` overlay first,
/// then the builtin [`LANGUAGE_ID_BY_EXTENSION`] table, falling back to `"plaintext"`. `plugins` is
/// typically the current `PluginStore` snapshot; pass `&[]` where no plugin context is available
/// (equivalent to builtin-only detection).
pub fn language_id_for_path(path: &Path, plugins: &[LoadedPlugin]) -> String {
    let Some(extension) = extension_of(path) else {
        return DEFAULT_LANGUAGE_ID.to_string();
    };

    if let Some(overlaid) = plugin_overlay(&extension, plugins) {
        return overlaid;
    }

    LANGUAGE_ID_BY_EXTENSION
        .iter()
        .find(|(key, _)| *key == extension)
        .map(|(_, id)| (*id).to_string())
        .unwrap_or_else(|| DEFAULT_LANGUAGE_ID.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plugin::types::{PluginContributions, PluginLanguageContribution, PluginManifest};

    fn plugin_with_language(extension_with_dot: &str, language_id: &str) -> LoadedPlugin {
        LoadedPlugin {
            manifest: PluginManifest {
                manifest_version: 1,
                id: "test-plugin".to_string(),
                name: "Test Plugin".to_string(),
                version: "1.0.0".to_string(),
                contributes: PluginContributions {
                    languages: vec![PluginLanguageContribution {
                        id: language_id.to_string(),
                        extensions: vec![extension_with_dot.to_string()],
                        aliases: Vec::new(),
                        grammar: None,
                        embedded_languages: None,
                    }],
                    lsp: Vec::new(),
                    themes: Vec::new(),
                },
            },
            root: "/tmp/test-plugin".to_string(),
            enabled: true,
            error: None,
        }
    }

    #[test]
    fn 확장자에_따라_빌트인_테이블에서_언어를_찾는다() {
        assert_eq!(language_id_for_path(Path::new("a.rs"), &[]), "rust");
        assert_eq!(language_id_for_path(Path::new("a.TSX"), &[]), "typescriptreact");
        assert_eq!(language_id_for_path(Path::new("a.unknown"), &[]), "plaintext");
        assert_eq!(language_id_for_path(Path::new("Makefile"), &[]), "plaintext");
    }

    #[test]
    fn 그래머_지원이_없는_확장자는_plaintext로_수렴한다() {
        assert_eq!(language_id_for_path(Path::new("a.cs"), &[]), "plaintext");
        assert_eq!(language_id_for_path(Path::new("a.php"), &[]), "plaintext");
        assert_eq!(language_id_for_path(Path::new("a.sql"), &[]), "plaintext");
    }

    #[test]
    fn 활성화된_플러그인의_확장자_기여가_빌트인보다_우선한다() {
        let plugin = plugin_with_language(".rs", "custom-rust");
        assert_eq!(language_id_for_path(Path::new("a.rs"), &[plugin]), "custom-rust");
    }

    #[test]
    fn 비활성화된_플러그인의_기여는_무시된다() {
        let mut plugin = plugin_with_language(".proto", "protobuf");
        plugin.enabled = false;
        assert_eq!(language_id_for_path(Path::new("a.proto"), &[plugin]), "plaintext");
    }

    #[test]
    fn 플러그인이_새_확장자를_추가할_수_있다() {
        let plugin = plugin_with_language(".proto", "protobuf");
        assert_eq!(language_id_for_path(Path::new("a.proto"), &[plugin]), "protobuf");
    }
}

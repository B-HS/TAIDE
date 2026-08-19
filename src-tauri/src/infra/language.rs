use std::path::Path;

/// One enabled plugin's single `contributes.languages` entry, reduced to the two fields
/// [`language_id_for_path`] actually consumes. Owned by `infra` so this module never references
/// `domain::plugin`'s manifest types (the layer direction is domain → infra, never the reverse —
/// audit R4#6, T1-I §1.3); `domain::plugin::service::language_overlays` converts the live
/// `LoadedPlugin` snapshot into this shape, applying the enabled-plugin filter in the process.
pub struct LanguageOverlay {
    pub language_id: String,
    pub extensions: Vec<String>,
}

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

/// Checks every overlay entry for an extension match, returning the first declared language id
/// found (`language_overlays` preserves the order `plugin::service::load_plugins` returns —
/// directory-sorted, so the match is deterministic).
fn plugin_overlay(extension: &str, overlays: &[LanguageOverlay]) -> Option<String> {
    overlays
        .iter()
        .find(|overlay| overlay.extensions.iter().any(|candidate| strip_leading_dot(candidate) == extension))
        .map(|overlay| overlay.language_id.clone())
}

/// Resolves a file path's language id — an enabled plugin's `contributes.languages` overlay first,
/// then the builtin [`LANGUAGE_ID_BY_EXTENSION`] table, falling back to `"plaintext"`. `overlays`
/// is typically `plugin::service::language_overlays` over the current `PluginStore` snapshot; pass
/// `&[]` where no plugin context is available (equivalent to builtin-only detection).
pub fn language_id_for_path(path: &Path, overlays: &[LanguageOverlay]) -> String {
    let Some(extension) = extension_of(path) else {
        return DEFAULT_LANGUAGE_ID.to_string();
    };

    if let Some(overlaid) = plugin_overlay(&extension, overlays) {
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

    fn overlay_with_language(extension_with_dot: &str, language_id: &str) -> LanguageOverlay {
        LanguageOverlay {
            language_id: language_id.to_string(),
            extensions: vec![extension_with_dot.to_string()],
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
    fn 플러그인_확장자_기여가_빌트인보다_우선한다() {
        let overlay = overlay_with_language(".rs", "custom-rust");
        assert_eq!(language_id_for_path(Path::new("a.rs"), &[overlay]), "custom-rust");
    }

    #[test]
    fn 플러그인이_새_확장자를_추가할_수_있다() {
        let overlay = overlay_with_language(".proto", "protobuf");
        assert_eq!(language_id_for_path(Path::new("a.proto"), &[overlay]), "protobuf");
    }
}

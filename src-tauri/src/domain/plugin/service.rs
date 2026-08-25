use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::language::LanguageOverlay;
use crate::infra::lsp_install;
use crate::infra::root_guard;

use super::types::{
    LoadedPlugin, PluginContributions, PluginErrorCode, PluginManifest, PLUGIN_GRAMMAR_MAX_BYTES, PLUGIN_MANIFEST_FILE,
    PLUGIN_MANIFEST_VERSION,
};

pub struct PluginStore(pub parking_lot::RwLock<Option<Vec<LoadedPlugin>>>);

impl PluginStore {
    pub fn new() -> Self {
        Self(parking_lot::RwLock::new(None))
    }
}

impl Default for PluginStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the cached plugin list, loading (and caching) it first if nothing has been loaded yet —
/// the shared "read-through cache" `plugin_list` already used before this was extracted, now also
/// used by every `infra::language::language_id_for_path` caller that needs the current plugin
/// overlay (contract §3.4/D1) without forcing every one of them to reimplement the same
/// lock-check-load-cache sequence. Lives in `service` (not `commands`) so cross-domain callers
/// (`file_open`, `git_diff`, `ide::server`'s MCP tools) reach the cache without re-entering the
/// command surface (R6#3).
pub fn ensure_loaded(store: &PluginStore, plugins_dir: &Path) -> Vec<LoadedPlugin> {
    if let Some(cached) = store.0.read().clone() {
        return cached;
    }
    let loaded = load_plugins(plugins_dir);
    *store.0.write() = Some(loaded.clone());
    loaded
}

/// Converts a `LoadedPlugin` snapshot into the infra-owned [`LanguageOverlay`] shape
/// `infra::language::language_id_for_path` consumes — the domain-side half of the dependency
/// inversion that keeps `infra::language` free of `domain::plugin` types (audit R4#6, T1-I §1.3).
/// Applies the enabled-plugin filter here and preserves both the plugin order (`load_plugins`'s
/// directory-sorted order) and each plugin's contribution order, so overlay precedence is
/// identical to scanning the plugins directly.
pub fn language_overlays(plugins: &[LoadedPlugin]) -> Vec<LanguageOverlay> {
    plugins
        .iter()
        .filter(|plugin| plugin.enabled)
        .flat_map(|plugin| &plugin.manifest.contributes.languages)
        .map(|language| LanguageOverlay {
            language_id: language.id.clone(),
            extensions: language.extensions.clone(),
        })
        .collect()
}

pub fn load_plugins(plugins_dir: &Path) -> Vec<LoadedPlugin> {
    let Ok(entries) = fs::read_dir(plugins_dir) else {
        return Vec::new();
    };

    // Every install path (`stage_from_directory`/`stage_from_archive` here,
    // `vsix::service::stage_vsix_import`) stages under `plugins_dir/.tmp/<uuid>` before an
    // atomic rename into its final `plugins_dir/<id>/` home, and only ever removes that `<uuid>`
    // child — the parent `.tmp` directory itself is left behind (empty, after a successful
    // install; possibly non-empty, after an install that failed mid-extraction). Without this
    // filter, `.tmp` looks like any other plugin directory here, has no `taide-plugin.json`, and
    // surfaces permanently in the PLUGINS list as a disabled `ParseFailed` entry named literally
    // `.tmp` the moment a user installs anything.
    let mut plugin_dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
        })
        .collect();
    plugin_dirs.sort();

    let mut plugins: Vec<LoadedPlugin> = plugin_dirs.iter().map(|dir| load_plugin_dir(dir)).collect();
    apply_grammar_conflicts(&mut plugins);
    plugins
}

fn apply_grammar_conflicts(plugins: &mut [LoadedPlugin]) {
    let mut claimed_language_ids: HashSet<String> = HashSet::new();
    let mut claimed_scope_names: HashSet<String> = HashSet::new();

    for plugin in plugins.iter_mut() {
        if !plugin.enabled {
            continue;
        }

        let mut has_conflict = false;

        for language in plugin.manifest.contributes.languages.iter_mut() {
            let Some(grammar) = language.grammar.as_deref() else {
                continue;
            };
            let Ok(scope_name) = validate_grammar(Path::new(&plugin.root), grammar) else {
                continue;
            };

            let conflicts = claimed_language_ids.contains(&language.id) || claimed_scope_names.contains(&scope_name);

            if conflicts {
                language.grammar = None;
                has_conflict = true;
                continue;
            }

            claimed_language_ids.insert(language.id.clone());
            claimed_scope_names.insert(scope_name);
        }

        if has_conflict {
            plugin.error = Some(PluginErrorCode::GrammarConflict);
        }
    }
}

fn load_plugin_dir(dir: &Path) -> LoadedPlugin {
    let dir_name = dir.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
    let manifest_path = dir.join(PLUGIN_MANIFEST_FILE);

    let manifest_content = match fs::read_to_string(&manifest_path) {
        Ok(content) => content,
        Err(_) => return disabled_plugin(&dir_name, dir, PluginErrorCode::ParseFailed),
    };

    let manifest: PluginManifest = match serde_json::from_str(&manifest_content) {
        Ok(manifest) => manifest,
        Err(_) => return disabled_plugin(&dir_name, dir, PluginErrorCode::ParseFailed),
    };

    match validate_manifest(dir, &dir_name, &manifest) {
        Ok(()) => LoadedPlugin {
            manifest,
            root: dir.display().to_string(),
            enabled: true,
            error: None,
        },
        Err(code) => LoadedPlugin {
            manifest,
            root: dir.display().to_string(),
            enabled: false,
            error: Some(code),
        },
    }
}

fn disabled_plugin(dir_name: &str, dir: &Path, code: PluginErrorCode) -> LoadedPlugin {
    LoadedPlugin {
        manifest: PluginManifest {
            manifest_version: PLUGIN_MANIFEST_VERSION,
            id: dir_name.to_string(),
            name: dir_name.to_string(),
            version: "0.0.0".to_string(),
            contributes: PluginContributions::default(),
        },
        root: dir.display().to_string(),
        enabled: false,
        error: Some(code),
    }
}

fn validate_manifest(dir: &Path, dir_name: &str, manifest: &PluginManifest) -> Result<(), PluginErrorCode> {
    if manifest.manifest_version != PLUGIN_MANIFEST_VERSION {
        return Err(PluginErrorCode::VersionMismatch);
    }

    if manifest.id != dir_name {
        return Err(PluginErrorCode::IdMismatch);
    }

    for language in &manifest.contributes.languages {
        if let Some(grammar) = &language.grammar {
            validate_grammar(dir, grammar)?;
        }
    }

    for theme in &manifest.contributes.themes {
        resolve_contribution_path(dir, &theme.path).map_err(|_| PluginErrorCode::PathEscape)?;
    }

    Ok(())
}

fn validate_grammar(dir: &Path, relative: &str) -> Result<String, PluginErrorCode> {
    let canonical = resolve_contribution_path(dir, relative).map_err(|_| PluginErrorCode::PathEscape)?;

    let metadata = fs::metadata(&canonical).map_err(|_| PluginErrorCode::GrammarMissing)?;
    if metadata.len() > PLUGIN_GRAMMAR_MAX_BYTES {
        return Err(PluginErrorCode::GrammarInvalid);
    }

    let content = fs::read_to_string(&canonical).map_err(|_| PluginErrorCode::GrammarInvalid)?;
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|_| PluginErrorCode::GrammarInvalid)?;

    value
        .get("scopeName")
        .and_then(serde_json::Value::as_str)
        .filter(|scope_name| !scope_name.trim().is_empty())
        .map(str::to_string)
        .ok_or(PluginErrorCode::GrammarInvalid)
}

pub fn read_grammar(plugins: &[LoadedPlugin], plugin_id: &str, language_id: &str) -> AppResult<String> {
    let plugin = plugins
        .iter()
        .find(|candidate| candidate.manifest.id == plugin_id)
        .ok_or_else(|| AppError::NotFound(format!("plugin not found: {plugin_id}")))?;

    if !plugin.enabled {
        return Err(AppError::InvalidArgument(format!("plugin is not enabled: {plugin_id}")));
    }

    let language = plugin
        .manifest
        .contributes
        .languages
        .iter()
        .find(|language| language.id == language_id)
        .ok_or_else(|| AppError::NotFound(format!("language contribution not found: {language_id}")))?;

    let grammar = language
        .grammar
        .as_deref()
        .ok_or_else(|| AppError::NotFound(format!("language has no grammar: {language_id}")))?;

    let canonical = resolve_contribution_path(Path::new(&plugin.root), grammar)?;
    if !canonical.exists() {
        return Err(AppError::NotFound(format!("grammar file not found: {grammar}")));
    }

    Ok(fs::read_to_string(&canonical)?)
}

pub fn resolve_contribution_path(plugin_root: &Path, relative: &str) -> AppResult<PathBuf> {
    let candidate = plugin_root.join(relative);
    let canonical_root = root_guard::canonicalize_lenient(plugin_root)?;
    let canonical_candidate = root_guard::canonicalize_lenient(&candidate)?;

    if canonical_candidate.starts_with(&canonical_root) {
        Ok(canonical_candidate)
    } else {
        Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.plugin.pathOutsideRoot",
            format!("path is outside the plugin root: {relative}"),
        )
        .with_arg("path", relative))
    }
}

/// Copies `source` into `dest` recursively — plain files and directories only. A symlink inside
/// `source` is deliberately skipped rather than followed, so an odd/malicious source directory
/// can't smuggle a link that resolves outside itself into `plugins_dir` (`std::fs::copy` on a
/// symlink path would otherwise copy whatever it points at, silently reading arbitrary files the
/// caller may not have intended to expose).
fn copy_dir_recursive(source: &Path, dest: &Path) -> AppResult<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

fn read_and_validate_manifest(dir: &Path) -> AppResult<PluginManifest> {
    let manifest_path = dir.join(PLUGIN_MANIFEST_FILE);
    let manifest_content = fs::read_to_string(&manifest_path).map_err(|_| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.plugin.manifestNotFound",
            format!("could not find {PLUGIN_MANIFEST_FILE}"),
        )
        .with_arg("file", PLUGIN_MANIFEST_FILE)
    })?;
    let manifest: PluginManifest = serde_json::from_str(&manifest_content).map_err(|error| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.plugin.manifestParseFailed",
            format!("failed to parse {PLUGIN_MANIFEST_FILE}: {error}"),
        )
        .with_arg("file", PLUGIN_MANIFEST_FILE)
        .with_arg("detail", &error)
    })?;
    if manifest.manifest_version != PLUGIN_MANIFEST_VERSION {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.plugin.manifestVersionUnsupported",
            format!("unsupported manifestVersion: {}", manifest.manifest_version),
        )
        .with_arg("version", manifest.manifest_version));
    }
    root_guard::ensure_safe_component(&manifest.id)?;
    Ok(manifest)
}

/// Stages a plugin already laid out as a directory (containing a top-level `taide-plugin.json`)
/// by copying it into a unique `plugins_dir/.tmp/` staging directory — the local-import
/// counterpart to [`stage_from_archive`]. The recursive copy (the expensive I/O) runs here so the
/// caller can keep it **outside** `AppState::begin_mutation` (audit R7#10, C11 axis A); the
/// already-installed check here is only a fast-fail courtesy — [`commit_staged_install`] re-checks
/// it authoritatively under the caller's guard. Returns the staging dir and the plugin id.
/// Every stage function takes `(plugins_dir, source)` in that order — `vsix::service::
/// stage_vsix_import` included — because two bare `&Path`s compile fine swapped. The caller owns
/// the returned staging dir until it hands it to [`commit_staged_install`], which removes it on
/// every failure path; nothing else ever will (no `.tmp` reaper exists), so staging without
/// committing leaks the staged bytes until `plugins_dir/.tmp` is cleared manually.
pub fn stage_from_directory(plugins_dir: &Path, source: &Path) -> AppResult<(PathBuf, String)> {
    let manifest = read_and_validate_manifest(source)?;

    if plugins_dir.join(&manifest.id).exists() {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.plugin.alreadyInstalled",
            format!("plugin is already installed: {}", manifest.id),
        )
        .with_arg("pluginId", &manifest.id));
    }

    let temp_dir = plugins_dir.join(".tmp").join(format!("{}-{}", manifest.id, uuid::Uuid::new_v4()));
    if let Err(error) = copy_dir_recursive(source, &temp_dir) {
        fs::remove_dir_all(&temp_dir).ok();
        return Err(error);
    }
    Ok((temp_dir, manifest.id))
}

/// Stages a plugin distributed as a zip archive (a TAIDE-native `.vsix`/zip bundle containing a
/// top-level `taide-plugin.json`, distinct from a real VS Code extension `.vsix` — see
/// `vsix::service::stage_vsix_import` for that flow) by extracting it with
/// `infra::archive::extract_hardened_zip` (budget/entry-cap/permission-masking hardened, unlike
/// `infra::lsp_install::extract_zip` — contract §3.4) into a unique `plugins_dir/.tmp/` staging
/// directory. The up-to-128MB extraction runs here so the caller can keep it **outside**
/// `AppState::begin_mutation` (audit R7#10); the already-installed check is a fast-fail courtesy,
/// re-checked authoritatively by [`commit_staged_install`] under the caller's guard. Argument
/// order and staging-dir ownership follow [`stage_from_directory`]'s contract: `(plugins_dir,
/// source)`, and the caller must hand the staging dir to commit or it leaks in `plugins_dir/.tmp`.
pub fn stage_from_archive(plugins_dir: &Path, source: &Path) -> AppResult<(PathBuf, String)> {
    let temp_extract_dir = plugins_dir.join(".tmp").join(format!("extract-{}", uuid::Uuid::new_v4()));

    let result = (|| -> AppResult<String> {
        crate::infra::archive::extract_hardened_zip(source, &temp_extract_dir)?;
        let manifest = read_and_validate_manifest(&temp_extract_dir)?;

        if plugins_dir.join(&manifest.id).exists() {
            return Err(AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.plugin.alreadyInstalled",
                format!("plugin is already installed: {}", manifest.id),
            )
            .with_arg("pluginId", &manifest.id));
        }
        Ok(manifest.id)
    })();

    match result {
        Ok(plugin_id) => Ok((temp_extract_dir, plugin_id)),
        Err(error) => {
            fs::remove_dir_all(&temp_extract_dir).ok();
            Err(error)
        }
    }
}

/// Finalizes a staged install ([`stage_from_directory`] / [`stage_from_archive`] /
/// `vsix::service::stage_vsix_import`) by renaming the staging directory into
/// `plugins_dir/{plugin_id}`. This is the cheap half the caller runs **under**
/// `AppState::begin_mutation`, which preserves the invariant the old single-function install held
/// its full-span guard for: the already-installed check and the placement are atomic with respect
/// to every other guarded plugin mutation (`plugin_uninstall`, `plugin_reload`, another install of
/// the same id), so a duplicate id still deterministically fails with the same error instead of
/// silently replacing a concurrent install. Cleans the staging directory up on every error path —
/// this is the **only** place staged bytes are ever reclaimed (no `.tmp` reaper exists), so every
/// stage call must reach this function exactly once with the `(plugins_dir, temp_dir, plugin_id)`
/// triple its stage function returned.
pub fn commit_staged_install(plugins_dir: &Path, temp_dir: &Path, plugin_id: &str) -> AppResult<String> {
    let final_dir = plugins_dir.join(plugin_id);
    let result = if final_dir.exists() {
        Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.plugin.alreadyInstalled",
            format!("plugin is already installed: {plugin_id}"),
        )
        .with_arg("pluginId", plugin_id))
    } else {
        lsp_install::atomic_install(temp_dir, &final_dir)
    };
    if result.is_err() {
        fs::remove_dir_all(temp_dir).ok();
    }
    result.map(|()| plugin_id.to_string())
}

/// Removes a plugin's directory outright — no built-in-plugin protection, since every entry in
/// `plugins_dir` is a user-installed directory (contract §3.4).
pub fn uninstall(plugins_dir: &Path, plugin_id: &str) -> AppResult<()> {
    root_guard::ensure_safe_component(plugin_id)?;
    let dir = plugins_dir.join(plugin_id);
    if !dir.exists() {
        return Err(AppError::NotFound(format!("plugin not found: {plugin_id}")));
    }
    fs::remove_dir_all(&dir).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use uuid::Uuid;

    use super::*;
    use crate::domain::plugin::types::PluginLanguageContribution;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("taide-plugin-test-{name}-{}", Uuid::new_v4()))
    }

    fn write_manifest(dir: &Path, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(PLUGIN_MANIFEST_FILE), content).unwrap();
    }

    fn plugin_with_language(extension_with_dot: &str, language_id: &str, enabled: bool) -> LoadedPlugin {
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
            enabled,
            error: None,
        }
    }

    #[test]
    fn 활성화된_플러그인의_언어_기여만_overlay로_변환된다() {
        let enabled = plugin_with_language(".proto", "protobuf", true);
        let disabled = plugin_with_language(".pb", "protobuf-disabled", false);

        let overlays = language_overlays(&[disabled, enabled]);

        assert_eq!(overlays.len(), 1);
        assert_eq!(overlays[0].language_id, "protobuf");
        assert_eq!(overlays[0].extensions, vec![".proto".to_string()]);
    }

    #[test]
    fn 상대경로_탈출_시도를_차단한다() {
        let root = temp_dir("escape");
        fs::create_dir_all(&root).unwrap();
        let outside = root.parent().unwrap().join(format!("taide-plugin-outside-{}", Uuid::new_v4()));
        fs::write(&outside, "secret").unwrap();

        let escaped_relative = format!("../{}", outside.file_name().unwrap().to_str().unwrap());
        let result = resolve_contribution_path(&root, &escaped_relative);

        assert!(result.is_err());

        fs::remove_dir_all(&root).ok();
        fs::remove_file(&outside).ok();
    }

    #[test]
    fn 루트_하위_경로는_허용한다() {
        let root = temp_dir("inside");
        let grammars_dir = root.join("grammars");
        fs::create_dir_all(&grammars_dir).unwrap();
        fs::write(grammars_dir.join("go.monarch.json"), "{}").unwrap();

        let result = resolve_contribution_path(&root, "grammars/go.monarch.json");

        assert!(result.is_ok());
        assert!(result.unwrap().starts_with(fs::canonicalize(&root).unwrap()));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 깊은_dotdot_탈출도_차단한다() {
        let root = temp_dir("deep-escape");
        fs::create_dir_all(root.join("grammars")).unwrap();

        let result = resolve_contribution_path(&root, "grammars/../../../../etc/passwd");

        assert!(result.is_err());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn id가_디렉토리명과_다르면_비활성화된다() {
        let dir = temp_dir("id-mismatch");
        write_manifest(
            &dir,
            r#"{"manifestVersion":1,"id":"other-id","name":"X","version":"1.0.0","contributes":{}}"#,
        );

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::IdMismatch));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 지원하지_않는_매니페스트_버전은_비활성화된다() {
        let dir_name = format!("taide-plugin-version-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        write_manifest(
            &dir,
            &format!(r#"{{"manifestVersion":99,"id":"{dir_name}","name":"X","version":"1.0.0","contributes":{{}}}}"#),
        );

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::VersionMismatch));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 매니페스트가_없으면_비활성화된다() {
        let dir = temp_dir("missing-manifest");
        fs::create_dir_all(&dir).unwrap();

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::ParseFailed));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 설치_스테이징_디렉토리_tmp_는_플러그인_목록에서_제외된다() {
        let plugins_dir = temp_dir("staging-parent");
        fs::create_dir_all(plugins_dir.join(".tmp")).unwrap();
        let dir_name = format!("real-plugin-{}", Uuid::new_v4());
        write_manifest(
            &plugins_dir.join(&dir_name),
            &format!(r#"{{"manifestVersion":1,"id":"{dir_name}","name":"X","version":"1.0.0","contributes":{{}}}}"#),
        );

        let plugins = load_plugins(&plugins_dir);

        assert_eq!(plugins.len(), 1, "숨김 디렉토리(.tmp)는 유령 플러그인으로 잡히면 안 된다");
        assert_eq!(plugins[0].manifest.id, dir_name);

        fs::remove_dir_all(&plugins_dir).ok();
    }

    #[test]
    fn 잘못된_json은_비활성화된다() {
        let dir = temp_dir("bad-json");
        write_manifest(&dir, "{ not json");

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::ParseFailed));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 유효한_매니페스트는_활성화된다() {
        let dir_name = format!("taide-plugin-valid-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(dir.join("grammars")).unwrap();
        fs::write(dir.join("grammars").join("go.monarch.json"), r#"{"scopeName":"source.go"}"#).unwrap();
        write_manifest(
            &dir,
            &format!(
                r#"{{"manifestVersion":1,"id":"{dir_name}","name":"Go","version":"1.0.0","contributes":{{"languages":[{{"id":"go","extensions":[".go"],"grammar":"grammars/go.monarch.json"}}]}}}}"#
            ),
        );

        let loaded = load_plugin_dir(&dir);

        assert!(loaded.enabled);
        assert!(loaded.error.is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 기여_경로가_플러그인_루트를_벗어나면_비활성화된다() {
        let dir_name = format!("taide-plugin-path-escape-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(&dir).unwrap();
        write_manifest(
            &dir,
            &format!(
                r#"{{"manifestVersion":1,"id":"{dir_name}","name":"Escape","version":"1.0.0","contributes":{{"languages":[{{"id":"go","extensions":[".go"],"grammar":"../../../../etc/passwd"}}]}}}}"#
            ),
        );

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::PathEscape));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 하나가_깨져도_나머지_플러그인은_계속_로드된다() {
        let root = temp_dir("scan-root");
        fs::create_dir_all(&root).unwrap();

        let good_name = "taide-plugin-good";
        write_manifest(
            &root.join(good_name),
            &format!(r#"{{"manifestVersion":1,"id":"{good_name}","name":"Good","version":"1.0.0","contributes":{{}}}}"#),
        );

        let bad_name = "taide-plugin-bad";
        write_manifest(&root.join(bad_name), "{ not json");

        let loaded = load_plugins(&root);

        assert_eq!(loaded.len(), 2);
        assert!(loaded.iter().any(|p| p.manifest.id == good_name && p.enabled));
        assert!(loaded.iter().any(|p| p.manifest.id == bad_name && !p.enabled));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn grammar_파일이_없으면_grammarmissing으로_비활성화된다() {
        let dir_name = format!("taide-plugin-grammar-missing-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(&dir).unwrap();
        write_manifest(
            &dir,
            &format!(
                r#"{{"manifestVersion":1,"id":"{dir_name}","name":"Missing","version":"1.0.0","contributes":{{"languages":[{{"id":"go","extensions":[".go"],"grammar":"grammars/go.tmLanguage.json"}}]}}}}"#
            ),
        );

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::GrammarMissing));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn grammar_json에_scopename이_없으면_grammarinvalid로_비활성화된다() {
        let dir_name = format!("taide-plugin-grammar-invalid-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(dir.join("grammars")).unwrap();
        fs::write(dir.join("grammars").join("go.tmLanguage.json"), "{}").unwrap();
        write_manifest(
            &dir,
            &format!(
                r#"{{"manifestVersion":1,"id":"{dir_name}","name":"Invalid","version":"1.0.0","contributes":{{"languages":[{{"id":"go","extensions":[".go"],"grammar":"grammars/go.tmLanguage.json"}}]}}}}"#
            ),
        );

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert_eq!(loaded.error, Some(PluginErrorCode::GrammarInvalid));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 서로_다른_플러그인이_같은_scopename을_주장하면_나중_플러그인의_grammar만_무시된다() {
        let root = temp_dir("grammar-conflict-scope");
        fs::create_dir_all(&root).unwrap();

        let first_name = "taide-plugin-aa-first";
        fs::create_dir_all(root.join(first_name).join("grammars")).unwrap();
        fs::write(
            root.join(first_name).join("grammars").join("go.tmLanguage.json"),
            r#"{"scopeName":"source.go"}"#,
        )
        .unwrap();
        write_manifest(
            &root.join(first_name),
            &format!(
                r#"{{"manifestVersion":1,"id":"{first_name}","name":"First","version":"1.0.0","contributes":{{"languages":[{{"id":"go-first","extensions":[".go"],"grammar":"grammars/go.tmLanguage.json"}}]}}}}"#
            ),
        );

        let second_name = "taide-plugin-bb-second";
        fs::create_dir_all(root.join(second_name).join("grammars")).unwrap();
        fs::write(
            root.join(second_name).join("grammars").join("go.tmLanguage.json"),
            r#"{"scopeName":"source.go"}"#,
        )
        .unwrap();
        write_manifest(
            &root.join(second_name),
            &format!(
                r#"{{"manifestVersion":1,"id":"{second_name}","name":"Second","version":"1.0.0","contributes":{{"languages":[{{"id":"go-second","extensions":[".go"],"grammar":"grammars/go.tmLanguage.json"}}]}}}}"#
            ),
        );

        let loaded = load_plugins(&root);

        let first = loaded.iter().find(|p| p.manifest.id == first_name).expect("first plugin loaded");
        let second = loaded.iter().find(|p| p.manifest.id == second_name).expect("second plugin loaded");

        assert!(first.enabled);
        assert!(first.error.is_none());
        assert!(second.enabled);
        assert_eq!(second.error, Some(PluginErrorCode::GrammarConflict));
        let second_language = second
            .manifest
            .contributes
            .languages
            .iter()
            .find(|l| l.id == "go-second")
            .expect("go-second contributed");
        assert!(second_language.grammar.is_none());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 서로_다른_플러그인이_같은_language_id로_grammar를_주장하면_나중_플러그인의_grammar만_무시된다() {
        let root = temp_dir("grammar-conflict-language-id");
        fs::create_dir_all(&root).unwrap();

        let first_name = "taide-plugin-aa-first-lang";
        fs::create_dir_all(root.join(first_name).join("grammars")).unwrap();
        fs::write(
            root.join(first_name).join("grammars").join("toml.tmLanguage.json"),
            r#"{"scopeName":"source.toml.first"}"#,
        )
        .unwrap();
        write_manifest(
            &root.join(first_name),
            &format!(
                r#"{{"manifestVersion":1,"id":"{first_name}","name":"First","version":"1.0.0","contributes":{{"languages":[{{"id":"toml","extensions":[".toml"],"grammar":"grammars/toml.tmLanguage.json"}}]}}}}"#
            ),
        );

        let second_name = "taide-plugin-bb-second-lang";
        fs::create_dir_all(root.join(second_name).join("grammars")).unwrap();
        fs::write(
            root.join(second_name).join("grammars").join("toml.tmLanguage.json"),
            r#"{"scopeName":"source.toml.second"}"#,
        )
        .unwrap();
        write_manifest(
            &root.join(second_name),
            &format!(
                r#"{{"manifestVersion":1,"id":"{second_name}","name":"Second","version":"1.0.0","contributes":{{"languages":[{{"id":"toml","extensions":[".toml"],"grammar":"grammars/toml.tmLanguage.json"}}]}}}}"#
            ),
        );

        let loaded = load_plugins(&root);

        let first = loaded.iter().find(|p| p.manifest.id == first_name).expect("first plugin loaded");
        let second = loaded.iter().find(|p| p.manifest.id == second_name).expect("second plugin loaded");

        assert!(first.enabled);
        assert!(second.enabled);
        assert_eq!(second.error, Some(PluginErrorCode::GrammarConflict));
        let second_language = second
            .manifest
            .contributes
            .languages
            .iter()
            .find(|l| l.id == "toml")
            .expect("toml contributed");
        assert!(second_language.grammar.is_none());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn grammar가_없는_language_기여끼리_id가_겹쳐도_비활성화되지_않는다() {
        let root = temp_dir("grammar-conflict-language-id-no-grammar");
        fs::create_dir_all(&root).unwrap();

        let first_name = "taide-plugin-aa-first-nolang";
        write_manifest(
            &root.join(first_name),
            &format!(
                r#"{{"manifestVersion":1,"id":"{first_name}","name":"First","version":"1.0.0","contributes":{{"languages":[{{"id":"toml","extensions":[".toml"]}}]}}}}"#
            ),
        );

        let second_name = "taide-plugin-bb-second-nolang";
        write_manifest(
            &root.join(second_name),
            &format!(
                r#"{{"manifestVersion":1,"id":"{second_name}","name":"Second","version":"1.0.0","contributes":{{"languages":[{{"id":"toml","extensions":[".toml"]}}]}}}}"#
            ),
        );

        let loaded = load_plugins(&root);

        let first = loaded.iter().find(|p| p.manifest.id == first_name).expect("first plugin loaded");
        let second = loaded.iter().find(|p| p.manifest.id == second_name).expect("second plugin loaded");

        assert!(first.enabled);
        assert!(first.error.is_none());
        assert!(second.enabled);
        assert!(second.error.is_none());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn plugin_read_grammar은_등록된_grammar_본문을_반환한다() {
        let dir_name = format!("taide-plugin-read-grammar-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(dir.join("grammars")).unwrap();
        let grammar_content = r#"{"scopeName":"source.go"}"#;
        fs::write(dir.join("grammars").join("go.tmLanguage.json"), grammar_content).unwrap();
        write_manifest(
            &dir,
            &format!(
                r#"{{"manifestVersion":1,"id":"{dir_name}","name":"Go","version":"1.0.0","contributes":{{"languages":[{{"id":"go","extensions":[".go"],"grammar":"grammars/go.tmLanguage.json"}}]}}}}"#
            ),
        );

        let loaded = vec![load_plugin_dir(&dir)];
        let content = read_grammar(&loaded, &dir_name, "go").expect("grammar read succeeds");

        assert_eq!(content, grammar_content);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn plugin_read_grammar은_루트를_벗어난_경로는_읽기_시점에도_거부한다() {
        let dir_name = format!("taide-plugin-read-grammar-escape-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(&dir).unwrap();

        let mut plugin = load_plugin_dir(&dir);
        plugin.enabled = true;
        plugin.error = None;
        plugin.manifest.contributes.languages = vec![PluginLanguageContribution {
            id: "go".to_string(),
            extensions: vec![".go".to_string()],
            aliases: Vec::new(),
            grammar: Some("../../../../etc/passwd".to_string()),
            embedded_languages: None,
        }];

        let result = read_grammar(&[plugin], &dir_name, "go");

        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stage_후_commit은_디렉토리를_plugins_dir로_복사한다() {
        let plugins_dir = temp_dir("install-dir-plugins");
        let source_name = format!("taide-plugin-install-src-{}", Uuid::new_v4());
        let source = std::env::temp_dir().join(&source_name);
        write_manifest(
            &source,
            &format!(r#"{{"manifestVersion":1,"id":"{source_name}","name":"Src","version":"1.0.0","contributes":{{}}}}"#),
        );

        let (temp_dir, plugin_id) = stage_from_directory(&plugins_dir, &source).expect("stage");
        let installed_id = commit_staged_install(&plugins_dir, &temp_dir, &plugin_id).expect("commit");

        assert_eq!(installed_id, source_name);
        assert!(plugins_dir.join(&source_name).join(PLUGIN_MANIFEST_FILE).exists());
        assert!(!temp_dir.exists(), "커밋 후 스테이징 디렉토리는 최종 위치로 이동되어야 한다");

        fs::remove_dir_all(&plugins_dir).ok();
        fs::remove_dir_all(&source).ok();
    }

    #[test]
    fn stage_from_directory는_이미_설치된_id를_거부한다() {
        let plugins_dir = temp_dir("install-dir-dup-plugins");
        let source_name = format!("taide-plugin-dup-{}", Uuid::new_v4());
        let source = std::env::temp_dir().join(&source_name);
        write_manifest(
            &source,
            &format!(r#"{{"manifestVersion":1,"id":"{source_name}","name":"Src","version":"1.0.0","contributes":{{}}}}"#),
        );
        fs::create_dir_all(plugins_dir.join(&source_name)).unwrap();

        let result = stage_from_directory(&plugins_dir, &source);

        assert!(result.is_err());

        fs::remove_dir_all(&plugins_dir).ok();
        fs::remove_dir_all(&source).ok();
    }

    #[test]
    fn commit_staged_install은_락_구간의_최종_검사에서_중복_id를_거부하고_스테이징을_정리한다() {
        let plugins_dir = temp_dir("install-dir-race-plugins");
        let source_name = format!("taide-plugin-race-{}", Uuid::new_v4());
        let source = std::env::temp_dir().join(&source_name);
        write_manifest(
            &source,
            &format!(r#"{{"manifestVersion":1,"id":"{source_name}","name":"Src","version":"1.0.0","contributes":{{}}}}"#),
        );

        let (temp_dir, plugin_id) = stage_from_directory(&plugins_dir, &source).expect("stage");
        fs::create_dir_all(plugins_dir.join(&source_name)).unwrap();

        let result = commit_staged_install(&plugins_dir, &temp_dir, &plugin_id);

        assert!(result.is_err(), "스테이징과 커밋 사이에 같은 id 가 설치되면 커밋이 거부해야 한다");
        assert!(!temp_dir.exists(), "거부된 커밋은 스테이징 디렉토리를 정리해야 한다");

        fs::remove_dir_all(&plugins_dir).ok();
        fs::remove_dir_all(&source).ok();
    }

    #[test]
    fn stage_from_directory는_manifest가_없으면_거부한다() {
        let plugins_dir = temp_dir("install-dir-nomanifest-plugins");
        let source = temp_dir("install-dir-nomanifest-src");
        fs::create_dir_all(&source).unwrap();

        let result = stage_from_directory(&plugins_dir, &source);

        assert!(result.is_err());

        fs::remove_dir_all(&plugins_dir).ok();
        fs::remove_dir_all(&source).ok();
    }

    #[test]
    fn uninstall은_플러그인_디렉토리를_삭제한다() {
        let plugins_dir = temp_dir("uninstall-plugins");
        let plugin_id = "taide-plugin-to-remove";
        fs::create_dir_all(plugins_dir.join(plugin_id)).unwrap();

        uninstall(&plugins_dir, plugin_id).expect("uninstall");

        assert!(!plugins_dir.join(plugin_id).exists());

        fs::remove_dir_all(&plugins_dir).ok();
    }

    #[test]
    fn uninstall은_존재하지_않는_id를_거부한다() {
        let plugins_dir = temp_dir("uninstall-missing-plugins");

        let result = uninstall(&plugins_dir, "does-not-exist");

        assert!(result.is_err());

        fs::remove_dir_all(&plugins_dir).ok();
    }

    #[test]
    fn uninstall은_경로_탈출_시도를_거부한다() {
        let plugins_dir = temp_dir("uninstall-escape-plugins");

        let result = uninstall(&plugins_dir, "../../etc");

        assert!(result.is_err());
    }
}

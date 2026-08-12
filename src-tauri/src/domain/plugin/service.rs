use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

use super::types::{
    LoadedPlugin, PluginContributions, PluginErrorCode, PluginManifest, PLUGIN_GRAMMAR_MAX_BYTES, PLUGIN_MANIFEST_FILE,
    PLUGIN_MANIFEST_VERSION,
};

pub fn load_plugins(plugins_dir: &Path) -> Vec<LoadedPlugin> {
    let Ok(entries) = fs::read_dir(plugins_dir) else {
        return Vec::new();
    };

    let mut plugin_dirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
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
    let canonical_root = canonicalize_lenient(plugin_root)?;
    let canonical_candidate = canonicalize_lenient(&candidate)?;

    if canonical_candidate.starts_with(&canonical_root) {
        Ok(canonical_candidate)
    } else {
        Err(AppError::InvalidArgument(format!("경로가 플러그인 루트 밖에 있습니다: {relative}")))
    }
}

fn canonicalize_lenient(path: &Path) -> AppResult<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::InvalidArgument(format!("유효하지 않은 경로입니다: {}", path.display())))?;
    let parent = path
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("유효하지 않은 경로입니다: {}", path.display())))?;

    Ok(canonicalize_lenient(parent)?.join(file_name))
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
}

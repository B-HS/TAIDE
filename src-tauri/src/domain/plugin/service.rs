use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

use super::types::{LoadedPlugin, PluginContributions, PluginManifest, PLUGIN_MANIFEST_FILE, PLUGIN_MANIFEST_VERSION};

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

    plugin_dirs.iter().map(|dir| load_plugin_dir(dir)).collect()
}

fn load_plugin_dir(dir: &Path) -> LoadedPlugin {
    let dir_name = dir.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
    let manifest_path = dir.join(PLUGIN_MANIFEST_FILE);

    let manifest_content = match fs::read_to_string(&manifest_path) {
        Ok(content) => content,
        Err(err) => return disabled_plugin(&dir_name, dir, format!("매니페스트를 읽을 수 없습니다: {err}")),
    };

    let manifest: PluginManifest = match serde_json::from_str(&manifest_content) {
        Ok(manifest) => manifest,
        Err(err) => return disabled_plugin(&dir_name, dir, format!("매니페스트 형식이 올바르지 않습니다: {err}")),
    };

    match validate_manifest(dir, &dir_name, &manifest) {
        Ok(()) => LoadedPlugin {
            manifest,
            root: dir.display().to_string(),
            enabled: true,
            error: None,
        },
        Err(message) => LoadedPlugin {
            manifest,
            root: dir.display().to_string(),
            enabled: false,
            error: Some(message),
        },
    }
}

fn disabled_plugin(dir_name: &str, dir: &Path, message: String) -> LoadedPlugin {
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
        error: Some(message),
    }
}

fn validate_manifest(dir: &Path, dir_name: &str, manifest: &PluginManifest) -> Result<(), String> {
    if manifest.manifest_version != PLUGIN_MANIFEST_VERSION {
        return Err(format!("지원하지 않는 매니페스트 버전입니다: {}", manifest.manifest_version));
    }

    if manifest.id != dir_name {
        return Err(format!(
            "매니페스트 id({})가 디렉토리명({dir_name})과 일치하지 않습니다",
            manifest.id
        ));
    }

    for language in &manifest.contributes.languages {
        if let Some(grammar) = &language.grammar {
            resolve_contribution_path(dir, grammar).map_err(|err| err.to_string())?;
        }
    }

    for theme in &manifest.contributes.themes {
        resolve_contribution_path(dir, &theme.path).map_err(|err| err.to_string())?;
    }

    Ok(())
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
        assert!(loaded.error.is_some());

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

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 매니페스트가_없으면_비활성화된다() {
        let dir = temp_dir("missing-manifest");
        fs::create_dir_all(&dir).unwrap();

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);
        assert!(loaded.error.is_some());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 잘못된_json은_비활성화된다() {
        let dir = temp_dir("bad-json");
        write_manifest(&dir, "{ not json");

        let loaded = load_plugin_dir(&dir);

        assert!(!loaded.enabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn 유효한_매니페스트는_활성화된다() {
        let dir_name = format!("taide-plugin-valid-{}", Uuid::new_v4());
        let dir = std::env::temp_dir().join(&dir_name);
        fs::create_dir_all(dir.join("grammars")).unwrap();
        fs::write(dir.join("grammars").join("go.monarch.json"), "{}").unwrap();
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
}

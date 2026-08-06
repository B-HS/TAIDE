use std::path::{Path, PathBuf};

use super::types::{LanguageServerSpec, LspServerDetection, LspServerId};

const JS_TS_MARKERS: &[&str] = &["package.json", "tsconfig.json"];
const DENO_MARKERS: &[&str] = &["deno.json", "deno.jsonc", "deno.lock"];
const CARGO_MANIFEST: &str = "Cargo.toml";
const PYTHON_MARKERS: &[&str] = &["pyproject.toml"];
const PYTHON_VENV_MARKER: &str = ".venv";
const MARKDOWN_MARKERS: &[&str] = &[".git", ".marksman.toml"];

pub fn builtin_specs() -> Vec<LanguageServerSpec> {
    vec![
        LanguageServerSpec {
            id: LspServerId::Vtsls,
            name: "vtsls".to_string(),
            command: "vtsls".to_string(),
            args: vec!["--stdio".to_string()],
            language_ids: vec![
                "typescript".to_string(),
                "typescriptreact".to_string(),
                "javascript".to_string(),
                "javascriptreact".to_string(),
            ],
            shares_sessions: true,
        },
        LanguageServerSpec {
            id: LspServerId::RustAnalyzer,
            name: "rust-analyzer".to_string(),
            command: "rust-analyzer".to_string(),
            args: Vec::new(),
            language_ids: vec!["rust".to_string()],
            shares_sessions: false,
        },
        LanguageServerSpec {
            id: LspServerId::BasedPyright,
            name: "basedpyright".to_string(),
            command: "basedpyright-langserver".to_string(),
            args: vec!["--stdio".to_string()],
            language_ids: vec!["python".to_string()],
            shares_sessions: true,
        },
        LanguageServerSpec {
            id: LspServerId::Ruff,
            name: "ruff".to_string(),
            command: "ruff".to_string(),
            args: vec!["server".to_string()],
            language_ids: vec!["python".to_string()],
            shares_sessions: true,
        },
        LanguageServerSpec {
            id: LspServerId::Marksman,
            name: "marksman".to_string(),
            command: "marksman".to_string(),
            args: vec!["server".to_string()],
            language_ids: vec!["markdown".to_string()],
            shares_sessions: true,
        },
    ]
}

fn install_hint_for(id: LspServerId) -> String {
    match id {
        LspServerId::Vtsls => "npm install -g @vtsls/language-server".to_string(),
        LspServerId::RustAnalyzer => "rustup component add rust-analyzer".to_string(),
        LspServerId::BasedPyright => "uv tool install basedpyright".to_string(),
        LspServerId::Ruff => "uv tool install ruff".to_string(),
        LspServerId::Marksman => "brew install marksman".to_string(),
    }
}

#[cfg(target_os = "windows")]
fn executable_candidate(dir: &Path, command: &str) -> PathBuf {
    dir.join(format!("{command}.cmd"))
}

#[cfg(not(target_os = "windows"))]
fn executable_candidate(dir: &Path, command: &str) -> PathBuf {
    dir.join(command)
}

fn project_local_dirs(root: &Path) -> Vec<PathBuf> {
    vec![root.join("node_modules").join(".bin"), root.join(".venv").join("bin")]
}

pub fn find_in_path(command: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var).find_map(|dir| {
        let candidate = executable_candidate(&dir, command);
        candidate.is_file().then_some(candidate)
    })
}

pub fn resolve_command(command: &str, root: Option<&Path>) -> Option<PathBuf> {
    if let Some(root) = root {
        for dir in project_local_dirs(root) {
            let candidate = executable_candidate(&dir, command);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    find_in_path(command)
}

pub fn detect_servers() -> Vec<LspServerDetection> {
    builtin_specs()
        .into_iter()
        .map(|spec| match find_in_path(&spec.command) {
            Some(path) => LspServerDetection {
                id: spec.id,
                name: spec.name,
                resolved_path: Some(path.to_string_lossy().to_string()),
                available: true,
                install_hint: None,
            },
            None => LspServerDetection {
                id: spec.id,
                install_hint: Some(install_hint_for(spec.id)),
                name: spec.name,
                resolved_path: None,
                available: false,
            },
        })
        .collect()
}

pub fn is_external_rust_source(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.contains("/.cargo/registry/src/") || normalized.contains("/.rustup/toolchains/")
}

fn find_first_ancestor_with_markers(start: &Path, markers: &[&str]) -> Option<(usize, PathBuf)> {
    for (depth, dir) in start.ancestors().enumerate() {
        if markers.iter().any(|marker| dir.join(marker).exists()) {
            return Some((depth, dir.to_path_buf()));
        }
    }
    None
}

fn find_root_js_ts(file_path: &Path) -> Option<PathBuf> {
    let start = file_path.parent()?;
    let deno = find_first_ancestor_with_markers(start, DENO_MARKERS);
    let ts = find_first_ancestor_with_markers(start, JS_TS_MARKERS);

    match (deno, ts) {
        (Some((deno_depth, _)), Some((ts_depth, ts_dir))) => {
            if deno_depth <= ts_depth {
                None
            } else {
                Some(ts_dir)
            }
        }
        (Some(_), None) => None,
        (None, Some((_, ts_dir))) => Some(ts_dir),
        (None, None) => None,
    }
}

fn cargo_toml_has_workspace(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .map(|content| content.lines().any(|line| line.trim() == "[workspace]"))
        .unwrap_or(false)
}

fn find_root_rust(file_path: &Path) -> Option<PathBuf> {
    let start = file_path.parent()?;
    let (_, nearest_dir) = find_first_ancestor_with_markers(start, &[CARGO_MANIFEST])?;

    for dir in nearest_dir.ancestors() {
        let cargo_toml = dir.join(CARGO_MANIFEST);
        if cargo_toml.exists() && cargo_toml_has_workspace(&cargo_toml) {
            return Some(dir.to_path_buf());
        }
    }

    Some(nearest_dir)
}

fn find_root_python(file_path: &Path) -> Option<PathBuf> {
    let start = file_path.parent()?;
    find_first_ancestor_with_markers(start, PYTHON_MARKERS)
        .or_else(|| find_first_ancestor_with_markers(start, &[PYTHON_VENV_MARKER]))
        .map(|(_, dir)| dir)
}

fn find_root_markdown(file_path: &Path) -> Option<PathBuf> {
    let start = file_path.parent()?;
    find_first_ancestor_with_markers(start, MARKDOWN_MARKERS).map(|(_, dir)| dir)
}

pub fn should_reuse_session(spec: &LanguageServerSpec, existing_roots: &[String], new_root: &str) -> bool {
    if !spec.shares_sessions || new_root.is_empty() {
        return false;
    }
    !existing_roots.is_empty()
}

pub fn find_root(server_id: LspServerId, file_path: &Path) -> Option<PathBuf> {
    match server_id {
        LspServerId::Vtsls => find_root_js_ts(file_path),
        LspServerId::RustAnalyzer => find_root_rust(file_path),
        LspServerId::BasedPyright | LspServerId::Ruff => find_root_python(file_path),
        LspServerId::Marksman => find_root_markdown(file_path),
    }
}

fn path_to_file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn initialization_options(id: LspServerId) -> serde_json::Value {
    match id {
        LspServerId::RustAnalyzer => serde_json::json!({
            "cargo": { "features": "all", "buildScripts": { "enable": true } },
            "procMacro": { "enable": true },
            "check": { "command": "clippy" },
            "checkOnSave": true,
        }),
        _ => serde_json::Value::Null,
    }
}

pub fn initialize_params(spec: &LanguageServerSpec, roots: &[PathBuf]) -> serde_json::Value {
    let workspace_folders: Vec<serde_json::Value> = roots
        .iter()
        .map(|root| {
            serde_json::json!({
                "uri": path_to_file_uri(root),
                "name": root.file_name().and_then(|name| name.to_str()).unwrap_or("workspace"),
            })
        })
        .collect();

    serde_json::json!({
        "processId": std::process::id(),
        "clientInfo": { "name": "TAIDE" },
        "rootUri": workspace_folders.first().and_then(|folder| folder.get("uri").cloned()),
        "rootPath": roots.first().map(|root| root.to_string_lossy().to_string()),
        "workspaceFolders": workspace_folders,
        "capabilities": {
            "general": { "positionEncoding": ["utf-16"] },
            "workspace": {
                "workspaceFolders": true,
                "configuration": true,
                "didChangeConfiguration": { "dynamicRegistration": true },
            },
            "textDocument": {
                "synchronization": { "didSave": true },
                "completion": { "completionItem": { "snippetSupport": true } },
                "rename": { "prepareSupport": true },
                "publishDiagnostics": { "relatedInformation": true },
            },
        },
        "initializationOptions": initialization_options(spec.id),
        "trace": "off",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("taide-lsp-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir 생성 실패");
        dir
    }

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("상위 디렉토리 생성 실패");
        }
        std::fs::write(path, "").expect("파일 생성 실패");
    }

    #[test]
    fn 공유_정책이_꺼진_서버는_기존_세션이_있어도_재사용하지_않는다() {
        let spec = builtin_specs()
            .into_iter()
            .find(|spec| spec.id == LspServerId::RustAnalyzer)
            .unwrap();

        assert!(!should_reuse_session(&spec, &["/root/a".to_string()], "/root/b"));
    }

    #[test]
    fn 공유_세션이지만_기존_루트가_없으면_재사용하지_않는다() {
        let spec = builtin_specs().into_iter().find(|spec| spec.id == LspServerId::Vtsls).unwrap();

        assert!(!should_reuse_session(&spec, &[], "/root/a"));
    }

    #[test]
    fn 공유_세션이고_기존_루트가_있으면_새_루트를_재사용한다() {
        let spec = builtin_specs().into_iter().find(|spec| spec.id == LspServerId::Vtsls).unwrap();

        assert!(should_reuse_session(&spec, &["/root/a".to_string()], "/root/b"));
    }

    #[test]
    fn 새_루트가_비어있으면_재사용하지_않는다() {
        let spec = builtin_specs().into_iter().find(|spec| spec.id == LspServerId::Vtsls).unwrap();

        assert!(!should_reuse_session(&spec, &["/root/a".to_string()], ""));
    }

    #[test]
    fn 빌트인_스펙은_공유_정책이_adr_대로_설정된다() {
        let specs = builtin_specs();

        let shares = |id: LspServerId| specs.iter().find(|spec| spec.id == id).unwrap().shares_sessions;

        assert!(shares(LspServerId::Vtsls));
        assert!(shares(LspServerId::BasedPyright));
        assert!(shares(LspServerId::Marksman));
        assert!(!shares(LspServerId::RustAnalyzer));
    }

    #[test]
    fn 가장_가까운_타입스크립트_설정파일_상위를_루트로_찾는다() {
        let root = make_temp_dir();
        touch(&root.join("package.json"));
        let file = root.join("src/index.ts");
        touch(&file);

        assert_eq!(find_root_js_ts(&file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 데노_마커가_같은_깊이면_루트를_찾지_않는다() {
        let root = make_temp_dir();
        touch(&root.join("package.json"));
        touch(&root.join("deno.json"));
        let file = root.join("src/index.ts");
        touch(&file);

        assert_eq!(find_root_js_ts(&file), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 데노_마커가_더_멀면_타입스크립트_루트를_사용한다() {
        let root = make_temp_dir();
        touch(&root.join("deno.json"));
        let sub = root.join("app");
        touch(&sub.join("package.json"));
        let file = sub.join("src/index.ts");
        touch(&file);

        assert_eq!(find_root_js_ts(&file), Some(sub.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 카고_워크스페이스_루트를_멤버_크레이트보다_우선한다() {
        let root = make_temp_dir();
        std::fs::write(root.join(CARGO_MANIFEST), "[workspace]\nmembers = [\"crates/foo\"]\n").unwrap();
        let member = root.join("crates/foo");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(member.join(CARGO_MANIFEST), "[package]\nname = \"foo\"\n").unwrap();
        let file = member.join("src/lib.rs");
        touch(&file);

        assert_eq!(find_root_rust(&file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 단일_크레이트는_자신의_카고_토ml_을_루트로_사용한다() {
        let root = make_temp_dir();
        std::fs::write(root.join(CARGO_MANIFEST), "[package]\nname = \"solo\"\n").unwrap();
        let file = root.join("src/main.rs");
        touch(&file);

        assert_eq!(find_root_rust(&file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 파이썬_프로젝트는_pyproject_toml_상위를_루트로_찾는다() {
        let root = make_temp_dir();
        touch(&root.join("pyproject.toml"));
        let file = root.join("pkg/module.py");
        touch(&file);

        assert_eq!(find_root_python(&file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 마크다운은_git_또는_marksman_설정_상위를_루트로_찾는다() {
        let root = make_temp_dir();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let file = root.join("docs/readme.md");
        touch(&file);

        assert_eq!(find_root_markdown(&file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 프로젝트_로컬_실행파일이_path_보다_우선한다() {
        let root = make_temp_dir();
        let bin_dir = root.join("node_modules").join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let local_bin = bin_dir.join("taide-fake-vtsls");
        touch(&local_bin);

        let resolved = resolve_command("taide-fake-vtsls", Some(&root));
        assert_eq!(resolved, Some(local_bin));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 로컬에_없으면_path_에서_찾는다() {
        let root = make_temp_dir();
        let path_dir = make_temp_dir();
        let path_bin = path_dir.join("taide-fake-only-in-path");
        touch(&path_bin);

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", &path_dir);

        let resolved = resolve_command("taide-fake-only-in-path", Some(&root));

        if let Some(original) = original_path {
            std::env::set_var("PATH", original);
        }

        assert_eq!(resolved, Some(path_bin));
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&path_dir).ok();
    }

    #[test]
    fn 외부_러스트_소스_경로를_식별한다() {
        assert!(is_external_rust_source(Path::new(
            "/Users/me/.cargo/registry/src/index/crate-1.0.0/lib.rs"
        )));
        assert!(is_external_rust_source(Path::new("/Users/me/.rustup/toolchains/stable/lib/std.rs")));
        assert!(!is_external_rust_source(Path::new("/Users/me/project/src/main.rs")));
    }

    #[test]
    fn initialize_파라미터에_position_encoding_이_포함된다() {
        let spec = builtin_specs().into_iter().find(|spec| spec.id == LspServerId::Vtsls).unwrap();
        let root = make_temp_dir();
        let params = initialize_params(&spec, std::slice::from_ref(&root));

        assert_eq!(params["capabilities"]["general"]["positionEncoding"], serde_json::json!(["utf-16"]));
        assert_eq!(params["clientInfo"]["name"], serde_json::json!("TAIDE"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rust_analyzer_는_initialization_options_에_prefix_없는_설정을_담는다() {
        let spec = builtin_specs()
            .into_iter()
            .find(|spec| spec.id == LspServerId::RustAnalyzer)
            .unwrap();
        let root = make_temp_dir();
        let params = initialize_params(&spec, std::slice::from_ref(&root));

        assert_eq!(params["initializationOptions"]["checkOnSave"], serde_json::json!(true));
        std::fs::remove_dir_all(&root).ok();
    }
}

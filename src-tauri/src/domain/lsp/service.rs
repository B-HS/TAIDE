use std::path::{Path, PathBuf};

use super::manifest;
use super::types::{LanguageServerSpec, LspRootStrategy, LspSdkProbe, LspServerDetection, LspToolchainTool};
use crate::infra::lsp_install;

const SERVER_DIR_TEMPLATE: &str = "{serverDir}";

const JS_TS_MARKERS: &[&str] = &["package.json", "tsconfig.json"];
const DENO_MARKERS: &[&str] = &["deno.json", "deno.jsonc", "deno.lock"];
const CARGO_MANIFEST: &str = "Cargo.toml";
const PYTHON_MARKERS: &[&str] = &["pyproject.toml"];
const PYTHON_VENV_MARKER: &str = ".venv";

pub fn builtin_specs() -> Vec<LanguageServerSpec> {
    manifest::servers()
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

pub fn resolve_command(
    command: &str,
    root: Option<&Path>,
    managed_dir: Option<&Path>,
    managed_relative_path: Option<&str>,
    is_managed: bool,
) -> Option<PathBuf> {
    if !is_managed {
        if let Some(root) = root {
            for dir in project_local_dirs(root) {
                let candidate = executable_candidate(&dir, command);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    if let Some(managed_dir) = managed_dir {
        let managed_command = managed_relative_path.unwrap_or(command);
        let candidate = executable_candidate(managed_dir, managed_command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    find_in_path(command)
}

fn requires_server_dir(spec: &LanguageServerSpec) -> bool {
    spec.command.args().iter().any(|arg| arg.contains(SERVER_DIR_TEMPLATE))
}

fn toolchain_extra_search_dirs(tool: LspToolchainTool) -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);

    match tool {
        LspToolchainTool::Go => {
            let mut dirs = Vec::new();
            if let Some(gobin) = std::env::var_os("GOBIN") {
                dirs.push(PathBuf::from(gobin));
            }
            if let Some(gopath) = std::env::var_os("GOPATH") {
                dirs.push(PathBuf::from(gopath).join("bin"));
            }
            if let Some(home) = &home {
                dirs.push(home.join("go").join("bin"));
            }
            dirs
        }
        LspToolchainTool::Gem => Vec::new(),
        LspToolchainTool::Coursier => home
            .map(|home| {
                vec![
                    home.join(".local/share/coursier/bin"),
                    home.join("Library/Application Support/Coursier/bin"),
                ]
            })
            .unwrap_or_default(),
        LspToolchainTool::Ghcup => home.map(|home| vec![home.join(".ghcup/bin")]).unwrap_or_default(),
    }
}

/// Resolves the executable for a language server spec, honoring the managed/path
/// command kind: managed commands never fall back to project-local `node_modules/.bin`
/// (a compromised repo must not be able to shadow an app-verified install), and managed
/// commands whose args template a `{serverDir}` placeholder are only considered available
/// once that directory actually exists.
pub fn resolve_spec_command(
    spec: &LanguageServerSpec,
    root: Option<&Path>,
    managed_dir: Option<&Path>,
    managed_relative_path: Option<&str>,
) -> Option<PathBuf> {
    let is_managed = spec.command.is_managed();

    if is_managed && requires_server_dir(spec) {
        // The command's bin is a generic runtime (e.g. `java`) that only makes sense once the
        // app-managed server directory it templates via `{serverDir}` actually exists — an
        // unmanaged runtime on PATH with no installed archive can never be spawned correctly.
        managed_dir?;
        return find_in_path(spec.command.bin());
    }

    if let Some(resolved) = resolve_command(spec.command.bin(), root, managed_dir, managed_relative_path, is_managed) {
        return Some(resolved);
    }

    let toolchain = spec.install.toolchain.as_ref()?;
    toolchain_extra_search_dirs(toolchain.tool).into_iter().find_map(|dir| {
        let candidate = executable_candidate(&dir, spec.command.bin());
        candidate.is_file().then_some(candidate)
    })
}

pub(super) fn toolchain_binary(tool: LspToolchainTool) -> &'static str {
    match tool {
        LspToolchainTool::Go => "go",
        LspToolchainTool::Gem => "gem",
        LspToolchainTool::Coursier => "cs",
        LspToolchainTool::Ghcup => "ghcup",
    }
}

fn evaluate_sdk_probes(probes: &[LspSdkProbe]) -> bool {
    probes.iter().any(|probe| match probe {
        LspSdkProbe::PathCommand { command } => find_in_path(command).is_some(),
        LspSdkProbe::FixedPath { path } => Path::new(path).exists(),
        LspSdkProbe::Xcrun { tool } => find_in_path("xcrun").is_some() && find_in_path(tool).is_some(),
    })
}

pub fn detect_servers(lsp_dir: &Path) -> Vec<LspServerDetection> {
    manifest::servers()
        .into_iter()
        .map(|spec| {
            let managed_dir = lsp_install::latest_installed_version(lsp_dir, spec.id.as_str())
                .map(|version| lsp_dir.join(spec.id.as_str()).join(version));
            let managed_relative_path = spec
                .install
                .download
                .as_ref()
                .and_then(|download| download.bin_path_in_archive.as_deref());
            let resolved = resolve_spec_command(&spec, None, managed_dir.as_deref(), managed_relative_path);

            let toolchain_available = spec
                .install
                .toolchain
                .as_ref()
                .map(|toolchain| find_in_path(toolchain_binary(toolchain.tool)).is_some());
            let sdk_available = spec
                .install
                .sdk_detect
                .as_ref()
                .map(|sdk_detect| evaluate_sdk_probes(&sdk_detect.probes));
            let toolchain_tool = spec
                .install
                .toolchain
                .as_ref()
                .map(|toolchain| toolchain_binary(toolchain.tool).to_string());
            let download_available = spec.install.download.as_ref().map(|download| {
                let platform = lsp_install::platform_key();
                download.urls.contains_key(&platform) && download.sha256.get(&platform).cloned().flatten().is_some()
            });

            LspServerDetection {
                id: spec.id.clone(),
                name: spec.name,
                language_ids: spec.language_ids.clone(),
                available: resolved.is_some(),
                resolved_path: resolved.map(|path| path.to_string_lossy().to_string()),
                install_hint: spec.install.hint.clone(),
                install_strategy: spec.install.strategy,
                installed_version: lsp_install::latest_installed_version(lsp_dir, spec.id.as_str()),
                toolchain_available,
                sdk_available,
                toolchain_tool,
                download_available,
            }
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

pub fn should_reuse_session(spec: &LanguageServerSpec, existing_roots: &[String], new_root: &str) -> bool {
    if !spec.shares_sessions || new_root.is_empty() {
        return false;
    }
    !existing_roots.is_empty()
}

pub fn find_root(spec: &LanguageServerSpec, file_path: &Path) -> Option<PathBuf> {
    match spec.root_strategy {
        LspRootStrategy::NearestMarker => {
            let start = file_path.parent()?;
            let markers: Vec<&str> = spec.root_markers.iter().map(String::as_str).collect();
            find_first_ancestor_with_markers(start, &markers).map(|(_, dir)| dir)
        }
        LspRootStrategy::JsTsDenoAware => find_root_js_ts(file_path),
        LspRootStrategy::CargoWorkspace => find_root_rust(file_path),
        LspRootStrategy::VenvFallback => find_root_python(file_path),
    }
}

fn path_to_file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
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
                "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                "hover": { "contentFormat": ["markdown", "plaintext"] },
                "signatureHelp": {},
                "inlayHint": {},
                "references": {},
                "definition": {},
                "formatting": {},
                "documentHighlight": {},
                "selectionRange": {},
            },
        },
        "initializationOptions": spec.initialization_options.clone().unwrap_or(serde_json::Value::Null),
        "trace": "off",
    })
}

#[cfg(test)]
mod tests {
    use super::super::types::LspInstallStrategy;
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

    fn spec_by_id(id: &str) -> LanguageServerSpec {
        builtin_specs().into_iter().find(|spec| spec.id == id).unwrap()
    }

    #[test]
    fn 공유_정책이_꺼진_서버는_기존_세션이_있어도_재사용하지_않는다() {
        let spec = spec_by_id("rustAnalyzer");

        assert!(!should_reuse_session(&spec, &["/root/a".to_string()], "/root/b"));
    }

    #[test]
    fn 공유_세션이지만_기존_루트가_없으면_재사용하지_않는다() {
        let spec = spec_by_id("vtsls");

        assert!(!should_reuse_session(&spec, &[], "/root/a"));
    }

    #[test]
    fn 공유_세션이고_기존_루트가_있으면_새_루트를_재사용한다() {
        let spec = spec_by_id("vtsls");

        assert!(should_reuse_session(&spec, &["/root/a".to_string()], "/root/b"));
    }

    #[test]
    fn 새_루트가_비어있으면_재사용하지_않는다() {
        let spec = spec_by_id("vtsls");

        assert!(!should_reuse_session(&spec, &["/root/a".to_string()], ""));
    }

    #[test]
    fn 빌트인_스펙은_공유_정책이_adr_대로_설정된다() {
        let specs = builtin_specs();

        let shares = |id: &str| specs.iter().find(|spec| spec.id == id).unwrap().shares_sessions;

        assert!(shares("vtsls"));
        assert!(shares("basedPyright"));
        assert!(shares("marksman"));
        assert!(!shares("rustAnalyzer"));
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

        let spec = spec_by_id("marksman");
        assert_eq!(find_root(&spec, &file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_root은_전략에_따라_알맞은_탐색을_사용한다() {
        let root = make_temp_dir();
        std::fs::write(root.join(CARGO_MANIFEST), "[package]\nname = \"solo\"\n").unwrap();
        let file = root.join("src/main.rs");
        touch(&file);

        let spec = spec_by_id("rustAnalyzer");
        assert_eq!(find_root(&spec, &file), Some(root.clone()));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 프로젝트_로컬_실행파일이_path_보다_우선한다() {
        let root = make_temp_dir();
        let bin_dir = root.join("node_modules").join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let local_bin = bin_dir.join("taide-fake-vtsls");
        touch(&local_bin);

        let resolved = resolve_command("taide-fake-vtsls", Some(&root), None, None, false);
        assert_eq!(resolved, Some(local_bin));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 관리_디렉토리는_프로젝트_로컬_다음_path_보다_우선한다() {
        let root = make_temp_dir();
        let managed_dir = make_temp_dir();
        let managed_bin = managed_dir.join("taide-fake-managed");
        touch(&managed_bin);

        let path_dir = make_temp_dir();
        let path_bin = path_dir.join("taide-fake-managed");
        touch(&path_bin);

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", &path_dir);

        let resolved = resolve_command("taide-fake-managed", Some(&root), Some(&managed_dir), None, false);

        if let Some(original) = original_path {
            std::env::set_var("PATH", original);
        }

        assert_eq!(resolved, Some(managed_bin));
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&managed_dir).ok();
        std::fs::remove_dir_all(&path_dir).ok();
    }

    #[test]
    fn 로컬에_없으면_path_에서_찾는다() {
        let root = make_temp_dir();
        let path_dir = make_temp_dir();
        let path_bin = path_dir.join("taide-fake-only-in-path");
        touch(&path_bin);

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", &path_dir);

        let resolved = resolve_command("taide-fake-only-in-path", Some(&root), None, None, false);

        if let Some(original) = original_path {
            std::env::set_var("PATH", original);
        }

        assert_eq!(resolved, Some(path_bin));
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&path_dir).ok();
    }

    #[test]
    fn managed_커맨드는_프로젝트_로컬_실행파일을_무시하고_path_에서_찾는다() {
        let root = make_temp_dir();
        let bin_dir = root.join("node_modules").join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let untrusted_local_bin = bin_dir.join("taide-fake-managed-only");
        touch(&untrusted_local_bin);

        let path_dir = make_temp_dir();
        let trusted_path_bin = path_dir.join("taide-fake-managed-only");
        touch(&trusted_path_bin);

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", &path_dir);

        let resolved = resolve_command("taide-fake-managed-only", Some(&root), None, None, true);

        if let Some(original) = original_path {
            std::env::set_var("PATH", original);
        }

        assert_eq!(resolved, Some(trusted_path_bin));
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
        let spec = spec_by_id("vtsls");
        let root = make_temp_dir();
        let params = initialize_params(&spec, std::slice::from_ref(&root));

        assert_eq!(params["capabilities"]["general"]["positionEncoding"], serde_json::json!(["utf-16"]));
        assert_eq!(params["clientInfo"]["name"], serde_json::json!("TAIDE"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn initialize_파라미터에_구현된_어댑터에_대응하는_capability가_선언된다() {
        let spec = spec_by_id("vtsls");
        let root = make_temp_dir();
        let params = initialize_params(&spec, std::slice::from_ref(&root));
        let text_document = &params["capabilities"]["textDocument"];

        assert_eq!(
            text_document["documentSymbol"]["hierarchicalDocumentSymbolSupport"],
            serde_json::json!(true)
        );
        assert_eq!(
            text_document["hover"]["contentFormat"],
            serde_json::json!(["markdown", "plaintext"])
        );
        assert!(text_document["signatureHelp"].is_object());
        assert!(text_document["inlayHint"].is_object());
        assert!(text_document["references"].is_object());
        assert!(text_document["definition"].is_object());
        assert!(text_document["formatting"].is_object());
        assert!(text_document["documentHighlight"].is_object());
        assert!(text_document["selectionRange"].is_object());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rust_analyzer_는_initialization_options_에_prefix_없는_설정을_담는다() {
        let spec = spec_by_id("rustAnalyzer");
        let root = make_temp_dir();
        let params = initialize_params(&spec, std::slice::from_ref(&root));

        assert_eq!(params["initializationOptions"]["checkOnSave"], serde_json::json!(true));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn detect_servers는_매니페스트의_모든_서버_결과를_반환한다() {
        let lsp_dir = make_temp_dir();
        let detections = detect_servers(&lsp_dir);

        assert_eq!(detections.len(), builtin_specs().len());
        let original_five = ["vtsls", "rustAnalyzer", "basedPyright", "ruff", "marksman"];
        assert!(original_five.iter().all(|id| detections
            .iter()
            .any(|detection| detection.id == *id && detection.install_strategy == LspInstallStrategy::SdkDetect)));
        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn detect_servers는_toolchain과_sdk_detect_전략도_보고한다() {
        let lsp_dir = make_temp_dir();
        let detections = detect_servers(&lsp_dir);

        let gopls = detections.iter().find(|detection| detection.id == "gopls").unwrap();
        assert_eq!(gopls.install_strategy, LspInstallStrategy::Toolchain);
        assert!(gopls.toolchain_available.is_some());
        assert!(gopls.sdk_available.is_none());

        let sourcekit = detections.iter().find(|detection| detection.id == "sourcekitLsp").unwrap();
        assert_eq!(sourcekit.install_strategy, LspInstallStrategy::SdkDetect);
        assert!(sourcekit.sdk_available.is_some());
        assert!(sourcekit.toolchain_available.is_none());

        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn detect_servers는_download_전략에_딸린_sdk_전제조건도_평가한다() {
        let lsp_dir = make_temp_dir();
        let detections = detect_servers(&lsp_dir);

        let jdtls = detections.iter().find(|detection| detection.id == "jdtls").unwrap();
        assert_eq!(jdtls.install_strategy, LspInstallStrategy::Download);
        assert!(jdtls.sdk_available.is_some());

        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn jdtls는_관리_디렉토리가_없으면_path_에_java_가_있어도_available_이_아니다() {
        let lsp_dir = make_temp_dir();

        let detections = detect_servers(&lsp_dir);
        let jdtls = detections.iter().find(|detection| detection.id == "jdtls").unwrap();

        assert!(!jdtls.available, "managed 디렉토리 없이는 java 런처만으로 available 이 되면 안된다");
        assert!(jdtls.resolved_path.is_none());

        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn jdtls는_관리_디렉토리와_path_의_java가_모두_있으면_available_이다() {
        let lsp_dir = make_temp_dir();
        let version_dir = lsp_dir.join("jdtls").join("1.60.0-202606262232");
        std::fs::create_dir_all(&version_dir).unwrap();

        let path_dir = make_temp_dir();
        touch(&path_dir.join("java"));
        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", &path_dir);

        let detections = detect_servers(&lsp_dir);

        if let Some(original) = original_path {
            std::env::set_var("PATH", original);
        }

        let jdtls = detections.iter().find(|detection| detection.id == "jdtls").unwrap();
        assert!(jdtls.available);

        std::fs::remove_dir_all(&lsp_dir).ok();
        std::fs::remove_dir_all(&path_dir).ok();
    }

    #[test]
    fn detect_servers는_관리_디렉토리에_설치된_버전을_보고한다() {
        let lsp_dir = make_temp_dir();
        let version_dir = lsp_dir.join("marksman").join("2026.1.1");
        std::fs::create_dir_all(&version_dir).unwrap();
        touch(&version_dir.join("marksman"));

        let detections = detect_servers(&lsp_dir);
        let marksman = detections.iter().find(|detection| detection.id == "marksman").unwrap();

        assert_eq!(marksman.installed_version, Some("2026.1.1".to_string()));
        assert!(marksman.available);
        std::fs::remove_dir_all(&lsp_dir).ok();
    }

    #[test]
    fn 관리_디렉토리_해석은_bin_path_in_archive_를_우선한다() {
        let managed_dir = make_temp_dir();
        let nested_bin = managed_dir.join("clangd_22.1.6").join("bin").join("clangd");
        touch(&nested_bin);

        let resolved = resolve_command("clangd", None, Some(&managed_dir), Some("clangd_22.1.6/bin/clangd"), true);

        assert_eq!(resolved, Some(nested_bin));
        std::fs::remove_dir_all(&managed_dir).ok();
    }
}

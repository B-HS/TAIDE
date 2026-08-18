use std::sync::OnceLock;

use super::types::{LanguageServerSpec, LspInstallStrategy, LspManifest, LSP_MANIFEST_SOURCE};

const REQUIRED_PLATFORM_KEY: &str = "darwin-arm64";

static BUNDLED_SERVERS: OnceLock<Vec<LanguageServerSpec>> = OnceLock::new();

pub fn parse_manifest(source: &str) -> Result<Vec<LanguageServerSpec>, String> {
    let manifest: LspManifest = serde_json::from_str(source).map_err(|error| format!("lsp manifest parse 실패: {error}"))?;
    validate_manifest(&manifest.servers)?;
    Ok(manifest.servers)
}

/// Falls back to an empty server list (logged) instead of panicking when `source` fails to parse
/// or validate. Split out from [`servers`] so the fallback itself is directly unit-testable
/// against a deliberately invalid source, independent of the [`OnceLock`] cache.
fn resolve_servers(source: &str) -> Vec<LanguageServerSpec> {
    match parse_manifest(source) {
        Ok(specs) => specs,
        Err(error) => {
            log::error!("번들된 lsp manifest 파싱에 실패했습니다. 빈 서버 목록으로 대체합니다: {error}");
            Vec::new()
        }
    }
}

/// Parses [`LSP_MANIFEST_SOURCE`] once and caches the result — every earlier call re-parsed and
/// re-validated the whole bundled JSON manifest from scratch, and this is called repeatedly from
/// IPC-handler paths (`lsp_spawn`, `lsp_resolve_root`, `lsp_install`, `detect_servers`) rather than
/// once at startup. Also removes the `expect("bundled lsp manifest must be valid")` this used to
/// panic with on a parse/validation failure: `LSP_MANIFEST_SOURCE` is a compile-time constant (so
/// that should never actually happen), but an IPC handler is the wrong place to bet a hard
/// process-wide panic on that always holding — a future manifest edit that slips past the
/// `parse_manifest` tests would otherwise crash every LSP-touching command instead of just leaving
/// language servers unavailable (see [`resolve_servers`]).
pub fn servers() -> Vec<LanguageServerSpec> {
    BUNDLED_SERVERS.get_or_init(|| resolve_servers(LSP_MANIFEST_SOURCE)).clone()
}

pub fn find_spec(server_id: &str) -> Option<LanguageServerSpec> {
    servers().into_iter().find(|spec| spec.id == server_id)
}

fn validate_manifest(specs: &[LanguageServerSpec]) -> Result<(), String> {
    let mut seen_ids = std::collections::HashSet::new();

    for spec in specs {
        if spec.id.as_str().is_empty() {
            return Err("lsp manifest: server id는 비어있을 수 없습니다".to_string());
        }
        if !seen_ids.insert(spec.id.clone()) {
            return Err(format!("lsp manifest: server id가 중복되었습니다: {}", spec.id));
        }
        if spec.language_ids.is_empty() {
            return Err(format!("lsp manifest: {} 의 languageIds가 비어있습니다", spec.id));
        }

        validate_install(spec)?;
    }

    Ok(())
}

fn validate_install(spec: &LanguageServerSpec) -> Result<(), String> {
    match spec.install.strategy {
        LspInstallStrategy::Download => {
            let Some(download) = &spec.install.download else {
                return Err(format!(
                    "lsp manifest: {} 는 download 전략인데 install.download 가 없습니다",
                    spec.id
                ));
            };
            if !download.urls.contains_key(REQUIRED_PLATFORM_KEY) {
                return Err(format!(
                    "lsp manifest: {} 의 download.urls 에 {REQUIRED_PLATFORM_KEY} 항목이 필요합니다",
                    spec.id
                ));
            }
        }
        LspInstallStrategy::Toolchain => {
            if let Some(toolchain) = &spec.install.toolchain {
                if toolchain.tool_detect.command.is_empty() {
                    return Err(format!("lsp manifest: {} 의 toolchain.toolDetect.command 가 비어있습니다", spec.id));
                }
            }
        }
        LspInstallStrategy::SdkDetect => {}
    }

    match &spec.install.sdk_detect {
        Some(sdk_detect) if sdk_detect.probes.is_empty() => {
            return Err(format!("lsp manifest: {} 의 sdkDetect.probes 가 비어있습니다", spec.id));
        }
        None if spec.install.strategy == LspInstallStrategy::SdkDetect => {
            return Err(format!(
                "lsp manifest: {} 는 sdk-detect 전략인데 install.sdkDetect 가 없습니다",
                spec.id
            ));
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_servers는_잘못된_소스에서_패닉_대신_빈_목록을_반환한다() {
        let specs = resolve_servers("not valid json");
        assert!(specs.is_empty());
    }

    #[test]
    fn resolve_servers는_유효한_소스를_정상_파싱한다() {
        let specs = resolve_servers(LSP_MANIFEST_SOURCE);
        assert!(!specs.is_empty());
    }

    #[test]
    fn servers는_반복_호출해도_동일한_서버_아이디_목록을_캐시에서_반환한다() {
        let first: Vec<_> = servers().into_iter().map(|spec| spec.id).collect();
        let second: Vec<_> = servers().into_iter().map(|spec| spec.id).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn 번들된_매니페스트는_유효하다() {
        let specs = servers();
        assert_eq!(specs.len(), 19);
    }

    #[test]
    fn 기존_5종_서버_아이디가_그대로_보존된다() {
        let specs = servers();
        let ids: Vec<_> = specs.iter().map(|spec| spec.id.as_str()).collect();

        assert!(ids.contains(&"vtsls"));
        assert!(ids.contains(&"rustAnalyzer"));
        assert!(ids.contains(&"basedPyright"));
        assert!(ids.contains(&"ruff"));
        assert!(ids.contains(&"marksman"));
    }

    #[test]
    fn 신규_13종_서버_아이디가_모두_반영된다() {
        let specs = servers();
        let ids: Vec<_> = specs.iter().map(|spec| spec.id.as_str()).collect();

        for id in [
            "jdtls",
            "clangd",
            "expert",
            "luaLanguageServer",
            "taplo",
            "terraformLs",
            "kotlinLsp",
            "gopls",
            "rubyLsp",
            "metals",
            "haskellLanguageServer",
            "dartls",
            "sourcekitLsp",
        ] {
            assert!(ids.contains(&id), "{id} 가 매니페스트에 없습니다");
        }
    }

    #[test]
    fn gopls는_semantic_tokens_initialization_option을_선언한다() {
        let specs = servers();
        let gopls = specs.iter().find(|spec| spec.id == "gopls").expect("gopls spec exists");

        assert_eq!(gopls.initialization_options, Some(serde_json::json!({ "semanticTokens": true })));
    }

    #[test]
    fn zls_서버_아이디가_반영된다() {
        let specs = servers();
        let ids: Vec<_> = specs.iter().map(|spec| spec.id.as_str()).collect();

        assert!(ids.contains(&"zls"));
    }

    #[test]
    fn download_전략에_sdk_detect_전제조건이_함께_있어도_유효하다() {
        let source = r#"{
            "servers": [
                {
                    "id": "with-prereq",
                    "name": "WithPrereq",
                    "languageIds": ["plaintext"],
                    "command": { "kind": "path", "bin": "with-prereq" },
                    "install": {
                        "strategy": "download",
                        "download": {
                            "version": "1.0.0",
                            "urls": { "darwin-arm64": "https://example.com/with-prereq" },
                            "archive": "gz"
                        },
                        "sdkDetect": {
                            "probes": [{ "kind": "path-command", "command": "java" }]
                        }
                    }
                }
            ]
        }"#;

        let result = parse_manifest(source);
        assert!(result.is_ok());
    }

    #[test]
    fn sdk_detect_전략인데_sdk_detect_필드가_없으면_검증에_실패한다() {
        let source = r#"{
            "servers": [
                {
                    "id": "no-probes",
                    "name": "NoProbes",
                    "languageIds": ["plaintext"],
                    "command": { "kind": "path", "bin": "no-probes" },
                    "install": { "strategy": "sdk-detect" }
                }
            ]
        }"#;

        let result = parse_manifest(source);
        assert!(result.is_err());
    }

    #[test]
    fn 아이디가_중복되면_검증에_실패한다() {
        let source = r#"{
            "servers": [
                {
                    "id": "dup",
                    "name": "Dup",
                    "languageIds": ["plaintext"],
                    "command": { "kind": "path", "bin": "dup" },
                    "install": { "strategy": "download" }
                },
                {
                    "id": "dup",
                    "name": "Dup2",
                    "languageIds": ["plaintext"],
                    "command": { "kind": "path", "bin": "dup2" },
                    "install": { "strategy": "download" }
                }
            ]
        }"#;

        let result = parse_manifest(source);
        assert!(result.is_err());
    }

    #[test]
    fn 다운로드_전략에_urls가_있으면_darwin_arm64가_필수다() {
        let source = r#"{
            "servers": [
                {
                    "id": "solo",
                    "name": "Solo",
                    "languageIds": ["plaintext"],
                    "command": { "kind": "path", "bin": "solo" },
                    "install": {
                        "strategy": "download",
                        "download": {
                            "version": "1.0.0",
                            "urls": { "linux-x64": "https://example.com/solo-linux" },
                            "archive": "tar-gz"
                        }
                    }
                }
            ]
        }"#;

        let result = parse_manifest(source);
        assert!(result.is_err());
    }

    #[test]
    fn 언어_아이디가_비어있으면_검증에_실패한다() {
        let source = r#"{
            "servers": [
                {
                    "id": "empty-lang",
                    "name": "Empty",
                    "languageIds": [],
                    "command": { "kind": "path", "bin": "empty" },
                    "install": { "strategy": "download" }
                }
            ]
        }"#;

        let result = parse_manifest(source);
        assert!(result.is_err());
    }

    #[test]
    fn 알려지지_않은_필드는_기본값으로_채워진다() {
        let specs = servers();
        let vtsls = specs.iter().find(|spec| spec.id == "vtsls").unwrap();

        assert!(vtsls.shares_sessions);
        assert_eq!(vtsls.command.bin(), "vtsls");
        assert_eq!(vtsls.command.args(), &["--stdio".to_string()]);
    }
}

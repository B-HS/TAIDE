use super::types::{LanguageServerSpec, LspInstallStrategy, LspManifest, LSP_MANIFEST_SOURCE};

const REQUIRED_PLATFORM_KEY: &str = "darwin-arm64";

pub fn parse_manifest(source: &str) -> Result<Vec<LanguageServerSpec>, String> {
    let manifest: LspManifest = serde_json::from_str(source).map_err(|error| format!("lsp manifest parse 실패: {error}"))?;
    validate_manifest(&manifest.servers)?;
    Ok(manifest.servers)
}

pub fn servers() -> Vec<LanguageServerSpec> {
    parse_manifest(LSP_MANIFEST_SOURCE).expect("bundled lsp manifest must be valid")
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

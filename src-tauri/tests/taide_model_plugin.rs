use taide_lib::domain::plugin::types::{LoadedPlugin, PluginErrorCode, PluginManifest, PLUGIN_MANIFEST_VERSION};
use taide_model::plugin::{LoadedPlugin as ModelLoadedPlugin, PluginManifest as ModelPluginManifest};

#[test]
fn 플러그인_구버전_manifest_기본값과_공개_타입을_유지한다() {
    let legacy = serde_json::json!({
        "manifestVersion": 1,
        "id": "sample",
        "name": "Sample",
        "version": "1.0.0"
    });
    let model: ModelPluginManifest = serde_json::from_value(legacy).expect("기존 플러그인 manifest");
    let facade: PluginManifest = model.clone();
    assert!(facade.contributes.languages.is_empty());
    assert!(facade.contributes.lsp.is_empty());
    assert!(facade.contributes.themes.is_empty());
    assert_eq!(facade.manifest_version, PLUGIN_MANIFEST_VERSION);

    let contribution: taide_model::plugin::PluginLanguageContribution = serde_json::from_value(serde_json::json!({
        "id": "rust", "extensions": [".rs"]
    }))
    .expect("기존 언어 기여");
    assert!(contribution.aliases.is_empty());
    assert!(contribution.grammar.is_none());
    assert!(contribution.embedded_languages.is_none());

    let lsp: taide_model::plugin::PluginLspContribution = serde_json::from_value(serde_json::json!({
        "languageId": "rust", "id": "analyzer", "cmd": "rust-analyzer"
    }))
    .expect("기존 LSP 기여");
    assert!(lsp.args.is_empty());
    assert!(lsp.detect.is_empty());
    assert!(!lsp.shareable);
    assert!(lsp.install_instructions.is_none());
}

#[test]
fn 로드된_플러그인_오류와_선택_필드_wire를_유지한다() {
    let wire = serde_json::json!({
        "manifest": {
            "manifestVersion": 1, "id": "sample", "name": "Sample", "version": "1.0.0",
            "contributes": { "languages": [], "lsp": [], "themes": [] }
        },
        "root": "/plugins/sample", "enabled": false, "error": "path-escape"
    });
    let model: ModelLoadedPlugin = serde_json::from_value(wire.clone()).expect("기존 플러그인 상태");
    let facade: LoadedPlugin = model.clone();
    assert_eq!(facade.error, Some(PluginErrorCode::PathEscape));
    assert_eq!(serde_json::to_value(model).expect("플러그인 상태 직렬화"), wire);

    let mut no_error = wire;
    no_error.as_object_mut().expect("플러그인 상태 객체").remove("error");
    let loaded: ModelLoadedPlugin = serde_json::from_value(no_error).expect("기존 무오류 상태");
    assert!(loaded.error.is_none());
}

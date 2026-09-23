use taide_lib::domain::vsix::types::{VsixExtensionInfo, VsixThemeExtractionResult, VSIX_ENTRY_MAX_BYTES};
use taide_lib::domain::window::types::{AuxiliaryWindowInfo, AUXILIARY_WINDOW_LABEL_PREFIX};
use taide_model::vsix::VsixThemeExtractionResult as ModelVsixThemeExtractionResult;
use taide_model::window::AuxiliaryWindowInfo as ModelAuxiliaryWindowInfo;

#[test]
fn vsix_테마_결과의_기존_wire와_공개_타입을_유지한다() {
    let wire = serde_json::json!({
        "extension": {
            "name": "demo", "displayName": "Demo Theme", "publisher": "example", "version": "1.0"
        },
        "themes": [{
            "label": "Sample", "uiTheme": "vs-dark", "rawJson": "{}",
            "includeChain": [{ "path": "base.json", "rawJson": "{}" }]
        }]
    });
    let model: ModelVsixThemeExtractionResult = serde_json::from_value(wire.clone()).expect("기존 VSIX 테마 결과");
    let facade: VsixThemeExtractionResult = model.clone();
    let _: VsixExtensionInfo = facade.extension.clone();
    assert_eq!(facade.themes[0].include_chain[0].path, "base.json");
    assert_eq!(serde_json::to_value(model).expect("VSIX 결과 직렬화"), wire);
    assert_eq!(VSIX_ENTRY_MAX_BYTES, 2 * 1024 * 1024);
}

#[test]
fn 보조_창_정보의_기존_wire와_공개_타입을_유지한다() {
    let wire = serde_json::json!({
        "label": "editor-7", "projectId": "prj-fixed", "windowSlot": 3
    });
    let model: ModelAuxiliaryWindowInfo = serde_json::from_value(wire.clone()).expect("기존 보조 창 응답");
    let facade: AuxiliaryWindowInfo = model.clone();
    assert_eq!(facade.project_id.as_str(), "prj-fixed");
    assert_eq!(serde_json::to_value(model).expect("보조 창 직렬화"), wire);
    assert_eq!(AUXILIARY_WINDOW_LABEL_PREFIX, "editor-");
}

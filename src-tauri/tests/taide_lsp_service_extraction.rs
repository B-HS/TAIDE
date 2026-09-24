use taide_lib::domain::lsp::{manifest as legacy_manifest, service as legacy_service, types as legacy_types};

#[test]
fn lsp_서비스와_매니페스트는_독립_crate와_기존_경로에서_같다() {
    let extracted_servers: fn() -> Vec<taide_model::lsp::LanguageServerSpec> = taide_lsp::manifest::servers;
    let legacy_servers: fn() -> Vec<taide_model::lsp::LanguageServerSpec> = legacy_manifest::servers;
    assert!(std::ptr::fn_addr_eq(extracted_servers, legacy_servers));

    let extracted_specs: fn() -> Vec<taide_model::lsp::LanguageServerSpec> = taide_lsp::service::builtin_specs;
    let legacy_specs: fn() -> Vec<taide_model::lsp::LanguageServerSpec> = legacy_service::builtin_specs;
    assert!(std::ptr::fn_addr_eq(extracted_specs, legacy_specs));

    assert_eq!(taide_lsp::manifest::LSP_MANIFEST_SOURCE, legacy_types::LSP_MANIFEST_SOURCE);
    assert_eq!(taide_lsp::manifest::servers(), legacy_manifest::servers());
}

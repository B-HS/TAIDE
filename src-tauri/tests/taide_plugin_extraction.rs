use std::any::TypeId;

use taide_infra::language::LanguageOverlay;
use taide_lib::domain::plugin::{service as legacy_service, types as legacy_types};
use taide_model::plugin::LoadedPlugin;

#[test]
fn 플러그인_서비스는_독립_crate와_기존_경로에서_같다() {
    assert_eq!(
        TypeId::of::<taide_plugin::service::PluginStore>(),
        TypeId::of::<legacy_service::PluginStore>()
    );

    let extracted_overlays: fn(&[LoadedPlugin]) -> Vec<LanguageOverlay> = taide_plugin::service::language_overlays;
    let legacy_overlays: fn(&[LoadedPlugin]) -> Vec<LanguageOverlay> = legacy_service::language_overlays;
    assert!(std::ptr::fn_addr_eq(extracted_overlays, legacy_overlays));
    assert!(extracted_overlays(&[]).is_empty());

    assert_eq!(taide_model::plugin::PLUGIN_MANIFEST_FILE, legacy_types::PLUGIN_MANIFEST_FILE);
}

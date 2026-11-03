use std::time::Instant;

use taide_native_ui::toast::Kind;

use super::*;

const AUXILIARY: &str = "auxiliary";
const OTHER: &str = "other";

#[test]
fn 상호작용_층은_포커스된_보조_창이고_주_창_층과_닫힌_창은_분리된다() {
    let auxiliary = ViewportId::from_hash_of(AUXILIARY);
    let other = ViewportId::from_hash_of(OTHER);
    let now = Instant::now();
    let mut layers = ToastLayers::new().unwrap();

    layers.focus(auxiliary);
    layers.error("before mount".into(), now);
    assert!(layers.main().is_showing(Kind::Error, "before mount"));

    layers.mount(auxiliary).unwrap();
    layers.mount(ViewportId::ROOT).unwrap();
    layers.focus(auxiliary);
    layers.error("interaction".into(), now);
    layers.main().info("provider".into(), now);
    let auxiliary_layer = layers.auxiliary(auxiliary).unwrap();
    assert!(auxiliary_layer.is_showing(Kind::Error, "interaction"));
    assert!(!auxiliary_layer.is_showing(Kind::Info, "provider"));
    assert!(layers.main().is_showing(Kind::Info, "provider"));
    assert!(!layers.main().is_showing(Kind::Error, "interaction"));
    assert!(layers.auxiliary(ViewportId::ROOT).is_none());

    layers.focus(other);
    assert!(layers.is_showing(Kind::Info, "provider"));

    layers.focus(auxiliary);
    layers.retain(|viewport| viewport == other);
    assert!(layers.auxiliary(auxiliary).is_none());
    assert!(layers.is_showing(Kind::Info, "provider"));
}

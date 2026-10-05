use rhwp::DocumentCore;
use taide_native_app::preview_hwp::Document;

const BLANK: &[u8] = include_bytes!("../vendor/rhwp/saved/blank2010.hwp");
const MAX_SIDE: usize = 4096;
const CHANNELS: usize = 4;
const BLACK_THRESHOLD: u8 = 96;
const RETAINED_BYTES: usize = 64 * 1024 * 1024;
const RETAINED_VISITS: usize = 262_144;
const SYNTHETIC_PAYLOAD_BYTES: usize = 32 * 1024;
const SYNTHETIC_BINDATA: u16 = 1;
const EMBEDDED_BINDATA_ATTR: u16 = 1;
const MINIMUM_LAZY_ALLOCATIONS: usize = 2;

#[derive(Debug)]
struct UnmeasuredResolver;

impl rhwp::model::bin_data::BinDataResolver for UnmeasuredResolver {
    fn resolve(&self, _: &str) -> Vec<u8> {
        panic!("retained measurement must not materialize resources")
    }
}

#[test]
fn hwp_retained_cfb는_실제_lazy_owner와_내부_컨테이너를_한번만_계산한다() {
    use rhwp::{
        model::bin_data::{BinDataBytes, BinDataContent},
        retained::{Error, Limits, measure},
    };

    let limits = Limits {
        bytes: RETAINED_BYTES,
        visits: RETAINED_VISITS,
    };
    let payload = vec![0; SYNTHETIC_PAYLOAD_BYTES];
    let encoded = synthetic_hwp_source();
    let mut parsed = rhwp::parse_document(&encoded).unwrap();
    assert_eq!(parsed.bin_data_content.len(), 1);
    let before = measure(&parsed, limits).unwrap();
    assert!(before.bytes > encoded.len());
    assert_eq!(before.shared_allocations, MINIMUM_LAZY_ALLOCATIONS);
    let entry = parsed.bin_data_content[0].clone();
    let BinDataBytes::Lazy { resolver, key } = &entry.data else {
        panic!("synthetic lazy CFB resource missing")
    };
    let key_bytes = key.capacity();
    let entry_bytes = entry.extension.capacity();
    let previous_capacity = parsed.bin_data_content.capacity();
    parsed.bin_data_content.push(entry.clone());
    let after = measure(&parsed, limits).unwrap();
    let capacity_bytes =
        (parsed.bin_data_content.capacity() - previous_capacity) * size_of::<BinDataContent>();
    assert_eq!(
        after.bytes - before.bytes,
        capacity_bytes + key_bytes + entry_bytes
    );
    assert_eq!(after.shared_allocations, before.shared_allocations);
    assert_eq!(
        resolver.resolve_limited(key, SYNTHETIC_PAYLOAD_BYTES),
        Some(payload)
    );
    assert_eq!(measure(&parsed, limits).unwrap(), after);
    assert_eq!(
        measure(
            &parsed,
            Limits {
                bytes: after.bytes - 1,
                ..limits
            }
        ),
        Err(Error::ByteBudget)
    );

    let core = DocumentCore::from_bytes(&encoded).unwrap();
    let core_cost = measure(&core, limits).unwrap();
    assert!(core_cost.bytes > before.bytes);
    assert!(core_cost.shared_allocations > before.shared_allocations);
    let unmeasured = BinDataBytes::Lazy {
        resolver: std::sync::Arc::new(UnmeasuredResolver),
        key: "synthetic".to_owned(),
    };
    assert_eq!(measure(&unmeasured, limits), Err(Error::Opaque));
}

fn synthetic_hwp_source() -> Vec<u8> {
    use rhwp::model::bin_data::{BinData, BinDataBytes, BinDataContent, BinDataType};

    let mut document = rhwp::parse_document(BLANK).unwrap();
    document.doc_info.raw_stream_dirty = true;
    document.doc_info.bin_data_list.push(BinData {
        attr: EMBEDDED_BINDATA_ATTR,
        data_type: BinDataType::Embedding,
        storage_id: SYNTHETIC_BINDATA,
        extension: Some("dat".to_owned()),
        ..Default::default()
    });
    document.bin_data_content.push(BinDataContent {
        id: SYNTHETIC_BINDATA,
        data: BinDataBytes::Loaded(vec![0; SYNTHETIC_PAYLOAD_BYTES]),
        extension: "dat".to_owned(),
    });
    rhwp::serialize_document(&document).unwrap()
}

#[test]
fn hwp_retained_zip는_실제_hwpx_lazy_owner_metadata와_payload를_한번만_계산한다() {
    use rhwp::{
        model::bin_data::{BinDataBytes, BinDataContent},
        retained::{Error, Limits, measure},
    };

    let limits = Limits {
        bytes: RETAINED_BYTES,
        visits: RETAINED_VISITS,
    };
    let original = DocumentCore::from_bytes(&synthetic_hwp_source()).unwrap();
    let encoded = original.export_hwpx_native().unwrap();
    let mut parsed = rhwp::parse_document(&encoded).unwrap();
    assert_eq!(parsed.bin_data_content.len(), 1);
    let before = measure(&parsed, limits).unwrap();
    assert!(before.bytes > encoded.len());
    assert!(before.shared_allocations >= MINIMUM_LAZY_ALLOCATIONS);
    let entry = parsed.bin_data_content[0].clone();
    let BinDataBytes::Lazy { resolver, key } = &entry.data else {
        panic!("synthetic lazy ZIP resource missing")
    };
    let child_bytes = key.capacity() + entry.extension.capacity();
    let previous_capacity = parsed.bin_data_content.capacity();
    parsed.bin_data_content.push(entry.clone());
    let after = measure(&parsed, limits).unwrap();
    let capacity_bytes =
        (parsed.bin_data_content.capacity() - previous_capacity) * size_of::<BinDataContent>();
    assert_eq!(after.bytes - before.bytes, capacity_bytes + child_bytes);
    assert_eq!(after.shared_allocations, before.shared_allocations);
    assert_eq!(
        resolver.resolve_limited(key, SYNTHETIC_PAYLOAD_BYTES),
        Some(vec![0; SYNTHETIC_PAYLOAD_BYTES])
    );
    assert_eq!(measure(&parsed, limits).unwrap(), after);
    assert_eq!(
        measure(
            &parsed,
            Limits {
                bytes: after.bytes - 1,
                ..limits
            }
        ),
        Err(Error::ByteBudget)
    );
    let core = DocumentCore::from_bytes(&encoded).unwrap();
    let core_cost = measure(&core, limits).unwrap();
    assert!(core_cost.bytes > before.bytes);
    assert!(core_cost.shared_allocations > before.shared_allocations);
}

#[test]
fn hwp_native는_원본_engine의_hwp_hwpx_페이지_svg와_픽셀을_연결한다() {
    let mut core = DocumentCore::from_bytes(BLANK).unwrap();
    core.insert_text_native(0, 0, 0, "Synthetic HWP 한글 日本語")
        .unwrap();
    let mut core_check = core.export_hwp_native().unwrap();
    assert!(!core_check.is_empty());
    let hwpx = core.export_hwpx_native().unwrap();
    let mut size = None;
    for bytes in [&core_check, &hwpx] {
        let document = Document::load(bytes).unwrap();
        assert!(document.page_count() >= 1);
        let page = document.render(0, MAX_SIDE).unwrap().unwrap();
        assert!(!page.size.contains(&0));
        assert_eq!(page.rgba.len(), page.size[0] * page.size[1] * CHANNELS);
        assert!(
            page.rgba
                .as_chunks::<CHANNELS>()
                .0
                .iter()
                .any(|pixel| pixel[0] < BLACK_THRESHOLD
                    && pixel[1] < BLACK_THRESHOLD
                    && pixel[2] < BLACK_THRESHOLD
                    && pixel[3] > BLACK_THRESHOLD)
        );
        if let Some(size) = size {
            assert_eq!(page.size, size);
        }
        size = Some(page.size);
        assert!(document.render(document.page_count(), MAX_SIDE).is_err());
        assert!(document.render(0, 1).unwrap().is_none());
    }
    core_check.truncate(1);
    assert!(Document::load(&core_check).is_err());
    assert!(Document::load(b"not a document").is_err());
}

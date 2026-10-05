use std::{
    borrow::Cow,
    io::{self, Cursor, Read, Write},
    panic::catch_unwind,
    sync::Arc,
    time::Duration,
};

use flate2::{Compression, write::DeflateEncoder};
use rhwp::{
    DocumentCore,
    model::control::Control,
    parser::hwp3::{
        Hwp3Error, Hwp3Limits, decode_body, drawing::Hwp3DrawingTextBox, ole::Hwp3OleInfo,
        parse_hwp3, parse_hwp3_with_limits, records::Hwp3AdditionalInfoBlock,
    },
};

const SIGNATURE_BYTES: usize = 30;
const DOC_INFO_BYTES: usize = 128;
const SUMMARY_BYTES: usize = 1008;
const INFO_LENGTH_OFFSET: usize = 126;
const PARAGRAPH_END_BYTES: usize = 43;
const LANGUAGES: usize = 7;
const IMAGE_MIN_BYTES: usize = 24;
const IMAGE_PAYLOAD_OFFSET: usize = 32;
const READ_CHUNK_BYTES: usize = 64 * 1024;
const DECLARED_BYTES: usize = READ_CHUNK_BYTES * 4;
const DECODED_BYTES: usize = 1024;
const SMALL_BUDGET: usize = 16;
const PAPER_LENGTH: u16 = 21046;
const PAPER_WIDTH: u16 = 14882;
const FONT_NAME_BYTES: usize = 40;
const PARAGRAPH_BYTES: usize = 43;
const LINE_BYTES: usize = 14;
const FONT_SIZE: u16 = 250;
const FONT_RATIO: u8 = 100;
const COMPRESSED_OFFSET: usize = 124;
const BODY_PREFIX_BYTES: usize =
    LANGUAGES * (size_of::<u16>() + FONT_NAME_BYTES) + size_of::<u16>();
const CONTEXT_DEPTH: usize = 4;
const BASIC_NODES: usize = LANGUAGES + 2;
const TABLE_GRID_SLOTS: usize = 16;
const OBJECT_INFO_BYTES: usize = 84;
const CELL_INFO_BYTES: usize = 27;
const CELL_SIZE: u16 = 100;
const SECOND_CELL_POSITION: u16 = 200;
const PICTURE_INFO_BYTES: usize = 348;
const DRAWING_FRAME_BYTES: usize = 28;
const DRAWING_HEADER_BYTES: usize = 92;
const HIDDEN_INFO_BYTES: usize = 8;
const CONTROL_UNITS: u16 = 4;
const WORKER_STACK_BYTES: usize = 2 * 1024 * 1024;
const CONTROL_HEADER_BYTES: usize = size_of::<u16>() * 2 + size_of::<u32>();
const INNER_PADDING_OFFSET: usize = 34;
const TABLE_PADDING_OFFSET: usize = 26;
const BORDER_MARGIN_OFFSET: usize = 112;
const BORDER_TYPE_OFFSET: usize = 120;
const MAX_BORDER_MARGIN: u16 = i16::MAX as u16 / 4;
const MAX_SIDE: usize = 4096;
const CHANNELS: usize = 4;
const INK_THRESHOLD: u8 = 96;
const OLE_SIGNATURE: u32 = 0xf8995568;
const OLE_STORAGE: &str = "00000000.OOO";
const OLE_STREAM: &str = "Synthetic";
const HEADER_DIFAT_OFFSET: usize = 76;
const OLE_BYTES: usize = 16 * 1024;
const AGGREGATE_REMAINING: usize = OLE_BYTES / 2;
const ENCRYPTED_OFFSET: usize = 96;
const HOST_TIMEOUT: Duration = Duration::from_secs(20);
const RETAINED_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const RETAINED_LIMIT_VISITS: usize = 262_144;

#[test]
fn hwp_retained_core는_실제_worker_stack과_캐시_증가_공유_소유권을_계산한다() {
    use rhwp::retained::{Error, Limits, measure};

    let result = std::thread::Builder::new()
        .stack_size(WORKER_STACK_BYTES)
        .spawn(|| {
            let limits = Limits {
                bytes: RETAINED_LIMIT_BYTES,
                visits: RETAINED_LIMIT_VISITS,
            };
            let mut body = text_body()[..BODY_PREFIX_BYTES].to_vec();
            body.extend_from_slice(&hidden_list(Hwp3Limits::default().max_depth - 1));
            let bytes = hwp3_source(&body, true);
            let parsed = parse_hwp3(&bytes).unwrap();
            let ir = measure(&parsed, limits).unwrap();
            let core = DocumentCore::from_bytes(&bytes).unwrap();
            let owned = measure(&core, limits).unwrap();
            assert!(owned.bytes > ir.bytes);
            assert!(owned.shared_allocations > 0);
            assert_eq!(
                measure(&core, Limits { bytes: owned.bytes - 1, ..limits }),
                Err(Error::ByteBudget)
            );
            let owner = Arc::new(std::sync::Mutex::new(core));
            let same_owner = owner.clone();
            let shared = measure(&(owner.clone(), same_owner.clone()), limits).unwrap();
            let single = measure(&owner, limits).unwrap();
            assert_eq!(
                shared.bytes - single.bytes,
                size_of::<Arc<std::sync::Mutex<DocumentCore>>>()
            );
            assert_eq!(shared.shared_allocations, single.shared_allocations);
            assert!(Arc::ptr_eq(&owner, &same_owner));
            let guard = owner.lock().unwrap();
            assert_eq!(measure(&owner, limits), Err(Error::Locked));
            drop(guard);

            let hml = b"<HWPML Version='2.91'><HEAD/><BODY><SECTION><P><TEXT><CHAR>Synthetic cache</CHAR></TEXT></P></SECTION></BODY><TAIL><UNKNOWN><DATA>Preserved metadata</DATA></UNKNOWN></TAIL></HWPML>";
            let core = DocumentCore::from_bytes(hml).unwrap();
            let metadata = core.hml_metadata().unwrap();
            assert!(!metadata.preserved_fragments.is_empty());
            let metadata_cost = measure(metadata, limits).unwrap();
            let before = measure(&core, limits).unwrap();
            assert!(before.bytes > metadata_cost.bytes);
            let json = core.get_page_layer_tree_native(0).unwrap();
            assert!(!json.is_empty());
            let after = measure(&core, limits).unwrap();
            assert!(after.bytes > before.bytes + json.len());
            assert_eq!(core.get_page_layer_tree_native(0).unwrap(), json);
            assert_eq!(measure(&core, limits).unwrap(), after);
        })
        .unwrap()
        .join();
    assert!(result.is_ok());
}

#[derive(Debug)]
struct UnmeasuredResolver;

impl rhwp::model::bin_data::BinDataResolver for UnmeasuredResolver {
    fn resolve(&self, _: &str) -> Vec<u8> {
        panic!("retained measurement must not load lazy payloads")
    }

    fn resolve_limited(&self, _: &str, _: usize) -> Option<Vec<u8>> {
        panic!("retained measurement must not load lazy payloads")
    }
}

#[test]
fn hwp_retained는_실제_ir_capacity와_중첩_payload를_계산하고_opaque를_거부한다() {
    use rhwp::{
        model::{
            bin_data::{BinDataBytes, BinDataContent},
            control::HiddenComment,
            paragraph::Paragraph,
        },
        retained::{Error, Limits, measure},
    };

    let limits = Limits {
        bytes: RETAINED_LIMIT_BYTES,
        visits: RETAINED_LIMIT_VISITS,
    };
    let encoded = hwp3_source(&text_body(), true);
    let mut document = parse_hwp3(&encoded).unwrap();
    document.sections[0].paragraphs[0]
        .controls
        .push(Control::HiddenComment(Box::new(HiddenComment {
            paragraphs: vec![Paragraph::default()],
        })));
    document.bin_data_content.push(BinDataContent {
        id: 1,
        data: BinDataBytes::Loaded(Vec::new()),
        extension: "synthetic".to_owned(),
    });
    let before = measure(&document, limits).unwrap();
    assert!(before.bytes > encoded.len());
    let mut capacity_delta = 0;
    let paragraph = &mut document.sections[0].paragraphs[0];
    let previous = paragraph.text.capacity();
    paragraph.text.reserve_exact(READ_CHUNK_BYTES);
    capacity_delta += paragraph.text.capacity() - previous;
    let previous = paragraph.char_offsets.capacity();
    paragraph.char_offsets.reserve_exact(READ_CHUNK_BYTES);
    capacity_delta += (paragraph.char_offsets.capacity() - previous) * size_of::<u32>();
    let previous = paragraph.raw_header_extra.capacity();
    paragraph.raw_header_extra.reserve_exact(READ_CHUNK_BYTES);
    capacity_delta += paragraph.raw_header_extra.capacity() - previous;
    let Control::HiddenComment(comment) = paragraph.controls.last_mut().unwrap() else {
        panic!("synthetic comment missing")
    };
    let text = &mut comment.paragraphs[0].text;
    let previous = text.capacity();
    text.reserve_exact(READ_CHUNK_BYTES);
    capacity_delta += text.capacity() - previous;
    let font = &mut document.doc_info.font_faces[0][0].name;
    let previous = font.capacity();
    font.reserve_exact(READ_CHUNK_BYTES);
    capacity_delta += font.capacity() - previous;
    let BinDataBytes::Loaded(bytes) = &mut document.bin_data_content[0].data else {
        panic!("synthetic loaded payload missing")
    };
    bytes.reserve_exact(READ_CHUNK_BYTES);
    capacity_delta += bytes.capacity();

    let after = measure(&document, limits).unwrap();
    assert_eq!(after.bytes - before.bytes, capacity_delta);
    assert_eq!(after.visits, before.visits);
    assert_eq!(
        measure(
            &document,
            Limits {
                bytes: after.bytes,
                ..limits
            }
        )
        .unwrap(),
        after
    );
    assert_eq!(
        measure(
            &document,
            Limits {
                bytes: after.bytes - 1,
                ..limits
            }
        ),
        Err(Error::ByteBudget)
    );
    document.bin_data_content[0].data = BinDataBytes::Lazy {
        resolver: Arc::new(UnmeasuredResolver),
        key: "synthetic".to_owned(),
    };
    assert_eq!(measure(&document, limits), Err(Error::Opaque));
}

#[test]
fn hwp_initializer는_bounded_core의_worker_stack과_hml_metadata를_보존한다() {
    let mut body = text_body()[..BODY_PREFIX_BYTES].to_vec();
    body.extend_from_slice(&hidden_list(Hwp3Limits::default().max_depth - 1));
    let bytes = hwp3_source(&body, true);
    let result = std::thread::Builder::new()
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || {
            let loaded = taide_native_app::preview_hwp::Document::load(&bytes).unwrap();
            assert!(loaded.page_count() > 0);
            assert!(loaded.render(0, MAX_SIDE).unwrap().is_some());
        })
        .unwrap()
        .join();
    assert!(result.is_ok());

    let hml = b"<HWPML Version='2.91'><HEAD/><BODY><SECTION><P><TEXT><CHAR>Synthetic metadata</CHAR></TEXT></P></SECTION></BODY></HWPML>";
    let original = DocumentCore::from_bytes(hml).unwrap();
    let parsed = rhwp::parser::parse_document_with_metadata(hml).unwrap();
    let loaded = DocumentCore::from_parsed(parsed, rhwp::parser::FileFormat::Hml).unwrap();
    let expected = original.hml_metadata().unwrap();
    let actual = loaded.hml_metadata().unwrap();
    assert_eq!(actual.hwpml_version, expected.hwpml_version);
    assert_eq!(actual.sub_version, expected.sub_version);
    assert_eq!(actual.style, expected.style);
    assert_eq!(actual.resource_count, expected.resource_count);
    assert_eq!(actual.warnings.len(), expected.warnings.len());
    assert_eq!(
        actual.preserved_fragments.len(),
        expected.preserved_fragments.len()
    );
    assert_eq!(loaded.page_count(), original.page_count());
    assert_eq!(
        loaded.render_page_svg_legacy_native(0).unwrap(),
        original.render_page_svg_legacy_native(0).unwrap()
    );
}

struct Hwp3Fixture(std::path::PathBuf);

impl Drop for Hwp3Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

struct Hwp3Sink;

impl taide_runtime::EventSink for Hwp3Sink {
    fn publish(&self, _: taide_model::app_event::AppEvent) {}
}

async fn hwp3_host_reply(
    host: &mut taide_native_app::host::HostBridge,
    ready: &tokio::sync::Notify,
    request: taide_native_app::preview_hwp::Request,
) -> Result<taide_native_app::preview_hwp::Page, taide_native_app::preview_hwp::Failure> {
    use taide_native_app::{
        host::{HostCommand, HostReply},
        preview_hwp::Failure,
    };

    host.submit(HostCommand::ReadHwpPreview(request.clone()))
        .unwrap();
    tokio::time::timeout(HOST_TIMEOUT, async {
        let mut saw_source = false;
        loop {
            match host.poll() {
                Some(HostReply::HwpSourceReady(returned)) => {
                    assert_eq!(returned, request);
                    assert!(!saw_source);
                    saw_source = true;
                }
                Some(HostReply::HwpPreview {
                    request: returned,
                    result,
                }) => {
                    assert_eq!(returned, request);
                    if !matches!(result, Err(Failure::Read(_))) {
                        assert!(saw_source);
                    }
                    return result;
                }
                Some(_) => panic!("wrong HWP3 reply"),
                None => ready.notified().await,
            }
        }
    })
    .await
    .unwrap()
}

#[test]
fn hwp3_host는_승인된_worker와_snapshot_재승인_종료를_연결한다() {
    use taide_model::{
        error::AppErrorKind,
        ids::{ProjectId, TabId},
        paths::AppPaths,
        project::Project,
    };
    use taide_native_app::{
        bootstrap::services,
        host::HostBridge,
        preview_hwp::{Cache, Failure},
    };
    use taide_runtime::{AppState, TaskSupervisor};
    use tokio::sync::Notify;

    let project = ProjectId::new();
    let fixture = Hwp3Fixture(std::env::temp_dir().join(format!("taide-hwp3-{project}")));
    let root = fixture.0.join("root");
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("synthetic.hwp").to_str().unwrap().to_owned();
    let body = text_body();
    std::fs::write(&path, hwp3_source(&body, true)).unwrap();
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.to_str().unwrap().to_owned(),
            name: "Synthetic HWP3".to_owned(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(
        services(state.clone(), tasks.clone(), Arc::new(Hwp3Sink)),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    runtime.block_on(async {
        let context = eframe::egui::Context::default();
        let mut cache = Cache::default();
        let request = cache.begin(&TabId::new(), &path, MAX_SIDE).unwrap();
        let page = hwp3_host_reply(&mut host, &ready, request.clone())
            .await
            .unwrap();
        assert!(page.total_pages >= 1);
        let expected = page.raster.as_ref().unwrap().rgba.clone();
        cache.accept(&context, request, Ok(page), 0);
        let request = cache.begin(&TabId::new(), &path, MAX_SIDE).unwrap();
        assert!(request.snapshot.is_some());
        std::fs::write(&path, b"replaced synthetic source").unwrap();
        let page = hwp3_host_reply(&mut host, &ready, request.clone())
            .await
            .unwrap();
        assert_eq!(page.raster.unwrap().rgba, expected);
        state.projects.write().remove(&project);
        let Err(Failure::Read(error)) = hwp3_host_reply(&mut host, &ready, request).await else {
            panic!("closed project must reject the cached source")
        };
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        tokio::time::timeout(HOST_TIMEOUT, host.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(HOST_TIMEOUT, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

fn hwp3_source(body: &[u8], is_compressed: bool) -> Vec<u8> {
    let mut bytes = header();
    bytes[SIGNATURE_BYTES + 6..SIGNATURE_BYTES + 8].copy_from_slice(&PAPER_LENGTH.to_le_bytes());
    bytes[SIGNATURE_BYTES + 8..SIGNATURE_BYTES + 10].copy_from_slice(&PAPER_WIDTH.to_le_bytes());
    bytes[SIGNATURE_BYTES + COMPRESSED_OFFSET] = u8::from(is_compressed);
    if is_compressed {
        bytes.extend_from_slice(&compressed(body));
        return bytes;
    }
    bytes.extend_from_slice(body);
    bytes
}

fn ole_body(payload: &[u8], signature: u32) -> Vec<u8> {
    let mut body = text_body();
    body.extend_from_slice(&2u32.to_le_bytes());
    body.extend_from_slice(&((payload.len() + size_of::<u32>()) as u32).to_le_bytes());
    body.extend_from_slice(&signature.to_le_bytes());
    body.extend_from_slice(payload);
    body.extend_from_slice(&[0; size_of::<u32>() * 2]);
    body
}

#[test]
fn hwp3_native는_plain_deflate의_원본_page와_실제_rgba를_보존한다() {
    let body = text_body();
    let mut reference = None;
    for is_compressed in [false, true] {
        let bytes = hwp3_source(&body, is_compressed);
        let original = DocumentCore::from_bytes(&bytes).unwrap();
        let loaded = taide_native_app::preview_hwp::Document::load(&bytes).unwrap();
        assert_eq!(loaded.page_count(), original.page_count() as usize);
        let raster = loaded.render(0, MAX_SIDE).unwrap().unwrap();
        assert_eq!(
            raster.rgba.len(),
            raster.size[0] * raster.size[1] * CHANNELS
        );
        assert!(
            raster
                .rgba
                .as_chunks::<CHANNELS>()
                .0
                .iter()
                .any(|pixel| pixel[0] < INK_THRESHOLD)
        );
        if let Some((size, rgba)) = &reference {
            assert_eq!(*size, raster.size);
            assert_eq!(rgba, &raster.rgba);
        }
        reference = Some((raster.size, raster.rgba));
    }
}

#[test]
fn hwp3_native는_정확한_ole를_검사하고_재포장_예산을_할당전에_지킨다() {
    use rhwp::parser::hwp3::{
        Hwp3Payload, ole::extract_ole_payloads_with_limits, parse_hwp3_with_validation,
    };
    use rhwp::serializer::mini_cfb::{build_cfb, build_cfb_with_limit};

    let payload =
        build_cfb(&[(&format!("/{OLE_STORAGE}/{OLE_STREAM}"), b"Synthetic OLE")]).unwrap();
    let body = ole_body(&payload, OLE_SIGNATURE);
    let bytes = hwp3_source(&body, false);
    let mut saw_body = false;
    let mut saw_ole = false;
    let mut saw_repacked = false;
    let parsed = parse_hwp3_with_validation(&bytes, &Hwp3Limits::default(), |part| {
        match part {
            Hwp3Payload::DecodedBody {
                metadata_bytes,
                body: returned,
            } => {
                assert!(!saw_body);
                assert_eq!(
                    metadata_bytes,
                    SIGNATURE_BYTES + DOC_INFO_BYTES + SUMMARY_BYTES
                );
                assert_eq!(returned, body);
                saw_body = true;
            }
            Hwp3Payload::Ole(returned) => {
                assert!(saw_body && !saw_ole);
                assert_eq!(returned, payload);
                saw_ole = true;
            }
            Hwp3Payload::RepackedOle(returned) => {
                assert!(saw_ole && !saw_repacked);
                assert_eq!(
                    returned,
                    build_cfb(&[(OLE_STREAM, b"Synthetic OLE")]).unwrap()
                );
                saw_repacked = true;
            }
        }
        Ok(Hwp3Limits::default().max_decoded_bytes)
    })
    .unwrap();
    assert!(saw_body && saw_ole && saw_repacked);
    assert_eq!(parsed.bin_data_content.len(), 1);
    let mut cfb = cfb::CompoundFile::open(Cursor::new(
        parsed.bin_data_content[0]
            .data
            .load_limited(payload.len())
            .unwrap(),
    ))
    .unwrap();
    let mut actual = Vec::new();
    cfb.open_stream(OLE_STREAM)
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, b"Synthetic OLE");
    assert!(
        taide_native_app::preview_hwp::Document::load(&bytes)
            .unwrap()
            .render(0, MAX_SIDE)
            .unwrap()
            .is_some()
    );
    assert!(build_cfb_with_limit(&[(OLE_STREAM, b"Synthetic OLE")], SMALL_BUDGET).is_err());
    assert!(extract_ole_payloads_with_limits(&payload, SMALL_BUDGET, 3).is_err());
    assert!(extract_ole_payloads_with_limits(&payload, payload.len(), 1).is_err());

    let mut forged = payload.clone();
    let pointer = payload[HEADER_DIFAT_OFFSET..HEADER_DIFAT_OFFSET + size_of::<u32>()].to_vec();
    forged[HEADER_DIFAT_OFFSET + size_of::<u32>()..HEADER_DIFAT_OFFSET + size_of::<u32>() * 2]
        .copy_from_slice(&pointer);
    assert!(
        taide_native_app::preview_hwp::Document::load(&hwp3_source(
            &ole_body(&forged, OLE_SIGNATURE),
            false
        ))
        .is_err()
    );
    assert!(
        taide_native_app::preview_hwp::Document::load(&hwp3_source(&ole_body(&forged, 0), false))
            .is_ok()
    );
}

#[test]
fn hwp3_native는_encoded_decoded와_nested_stream의_합산을_core전에_제한한다() {
    use taide_native_app::preview_hwp_preflight::MAX_DECODED_BYTES;

    let mut oversized = hwp3_source(&text_body(), false);
    oversized.resize(taide_model::file::READ_ONLY_FILE_BYTES as usize + 1, 0);
    assert!(taide_native_app::preview_hwp::Document::load(&oversized).is_err());
    drop(oversized);
    let mut body = text_body();
    body.resize(MAX_DECODED_BYTES, 0);
    assert!(taide_native_app::preview_hwp::Document::load(&hwp3_source(&body, true)).is_err());
    drop(body);
    let payload = rhwp::serializer::mini_cfb::build_cfb(&[(
        &format!("/{OLE_STORAGE}/{OLE_STREAM}"),
        &vec![0; OLE_BYTES],
    )])
    .unwrap();
    let mut body = ole_body(&payload, OLE_SIGNATURE);
    body.resize(
        MAX_DECODED_BYTES - SIGNATURE_BYTES - DOC_INFO_BYTES - SUMMARY_BYTES - AGGREGATE_REMAINING,
        0,
    );
    assert!(taide_native_app::preview_hwp::Document::load(&hwp3_source(&body, true)).is_err());
    drop(body);
    let mut encrypted = hwp3_source(&text_body(), false);
    encrypted
        [SIGNATURE_BYTES + ENCRYPTED_OFFSET..SIGNATURE_BYTES + ENCRYPTED_OFFSET + size_of::<u16>()]
        .copy_from_slice(&1u16.to_le_bytes());
    assert!(taide_native_app::preview_hwp::Document::load(&encrypted).is_err());
    let rejected = hwp3_source(&hidden_list(Hwp3Limits::default().max_depth), false);
    assert!(taide_native_app::preview_hwp::Document::load(&rejected).is_err());
}

#[test]
fn hwp3_margin_page_border는_원본_release의_16비트_변환을_보존한다() {
    for value in [MAX_BORDER_MARGIN, MAX_BORDER_MARGIN + 1, u16::MAX] {
        let mut bytes = with_list(&text_body()[BODY_PREFIX_BYTES..]);
        let offset = SIGNATURE_BYTES + BORDER_MARGIN_OFFSET;
        bytes[offset..offset + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
        let offset = SIGNATURE_BYTES + BORDER_TYPE_OFFSET;
        bytes[offset..offset + size_of::<u16>()].copy_from_slice(&1u16.to_le_bytes());
        let parsed = parse_hwp3(&bytes).unwrap();
        assert_eq!(
            parsed.sections[0].section_def.page_border_fill.spacing_left,
            (i32::from(value) * 4) as i16
        );
    }
}

#[test]
fn hwp3_margin_table은_원본_release의_음수와_16비트_변환을_보존한다() {
    for (value, offset, expected) in [
        (-1i16, INNER_PADDING_OFFSET, -4),
        (i16::MAX, TABLE_PADDING_OFFSET, 0),
        (i16::MAX, INNER_PADDING_OFFSET, -4),
    ] {
        let mut list = table_list();
        let offset = PARAGRAPH_BYTES + LINE_BYTES + CONTROL_HEADER_BYTES + offset;
        list[offset..offset + size_of::<i16>()].copy_from_slice(&value.to_le_bytes());
        let bytes = with_list(&list);
        let parsed = catch_unwind(|| parse_hwp3(&bytes)).unwrap().unwrap();
        let Control::Table(table) = &parsed.sections[0].paragraphs[0].controls[0] else {
            panic!("actual table required")
        };
        assert_eq!(table.padding.left, expected);
    }
}

#[test]
fn hwp3_context_default는_허용_끝의_문서를_작은_stack에서_core까지_읽는다() {
    let limits = Hwp3Limits::default();
    let mut input = with_list(&hidden_list(limits.max_depth - 1));
    input[SIGNATURE_BYTES + 6..SIGNATURE_BYTES + 8].copy_from_slice(&PAPER_LENGTH.to_le_bytes());
    input[SIGNATURE_BYTES + 8..SIGNATURE_BYTES + 10].copy_from_slice(&PAPER_WIDTH.to_le_bytes());
    let result = std::thread::Builder::new()
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || {
            let core = DocumentCore::from_bytes(&input).unwrap();
            core.page_count() > 0 && core.render_page_svg_legacy_native(0).is_ok()
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(result);
}

#[test]
fn hwp3_context_default는_작은_worker_stack에서_중첩을_안전하게_거절한다() {
    let limits = Hwp3Limits::default();
    let input = with_list(&hidden_list(limits.max_depth));
    let result = std::thread::Builder::new()
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || {
            matches!(
                parse_hwp3_with_limits(&input, &limits),
                Err(Hwp3Error::LimitExceeded { .. })
            )
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(result);
}

fn with_list(list: &[u8]) -> Vec<u8> {
    let mut bytes = header();
    bytes.extend_from_slice(&text_body()[..BODY_PREFIX_BYTES]);
    bytes.extend_from_slice(list);
    bytes
}

fn control_list(code: u16, payload: &[u8]) -> Vec<u8> {
    let mut paragraph = [0; PARAGRAPH_BYTES];
    paragraph[0] = 1;
    paragraph[1..3].copy_from_slice(&(CONTROL_UNITS + 1).to_le_bytes());
    paragraph[3..5].copy_from_slice(&1u16.to_le_bytes());
    paragraph[12..14].copy_from_slice(&FONT_SIZE.to_le_bytes());
    paragraph[21..28].fill(FONT_RATIO);
    let mut bytes = paragraph.to_vec();
    let mut line = [0; LINE_BYTES];
    line[4..6].copy_from_slice(&FONT_SIZE.to_le_bytes());
    bytes.extend_from_slice(&line);
    bytes.extend_from_slice(&code.to_le_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&code.to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes.extend_from_slice(&13u16.to_le_bytes());
    bytes.extend_from_slice(&[0; PARAGRAPH_END_BYTES]);
    bytes
}

fn hidden_list(levels: usize) -> Vec<u8> {
    let mut list = text_body()[BODY_PREFIX_BYTES..].to_vec();
    for _ in 0..levels {
        let mut payload = vec![0; HIDDEN_INFO_BYTES];
        payload.extend_from_slice(&list);
        list = control_list(15, &payload);
    }
    list
}

fn table_list() -> Vec<u8> {
    let mut payload = vec![0; OBJECT_INFO_BYTES];
    payload[80..82].copy_from_slice(&2u16.to_le_bytes());
    for position in [0u16, SECOND_CELL_POSITION] {
        let mut cell = [0; CELL_INFO_BYTES];
        cell[4..6].copy_from_slice(&position.to_le_bytes());
        cell[6..8].copy_from_slice(&position.to_le_bytes());
        cell[8..10].copy_from_slice(&CELL_SIZE.to_le_bytes());
        cell[10..12].copy_from_slice(&CELL_SIZE.to_le_bytes());
        payload.extend_from_slice(&cell);
    }
    for _ in 0..3 {
        payload.extend_from_slice(&[0; PARAGRAPH_END_BYTES]);
    }
    control_list(10, &payload)
}

fn drawing_list() -> Vec<u8> {
    let mut drawing = vec![0; DRAWING_FRAME_BYTES];
    drawing[..4].copy_from_slice(&24u32.to_le_bytes());
    drawing[8..12].copy_from_slice(&2u32.to_le_bytes());
    let mut group = [0; DRAWING_HEADER_BYTES];
    group[..4].copy_from_slice(&(DRAWING_HEADER_BYTES as u32).to_le_bytes());
    group[6..8].copy_from_slice(&2u16.to_le_bytes());
    drawing.extend_from_slice(&group);
    let mut textbox = group;
    textbox[4..6].copy_from_slice(&6u16.to_le_bytes());
    textbox[6..8].copy_from_slice(&0u16.to_le_bytes());
    drawing.extend_from_slice(&textbox);
    let text = &text_body()[BODY_PREFIX_BYTES..];
    drawing.extend_from_slice(&0u32.to_le_bytes());
    drawing.extend_from_slice(&(text.len() as u32).to_le_bytes());
    drawing.extend_from_slice(text);
    let mut picture = vec![0; PICTURE_INFO_BYTES];
    picture[..4].copy_from_slice(&(drawing.len() as u32).to_le_bytes());
    picture[74] = 3;
    picture.extend_from_slice(&drawing);
    picture.extend_from_slice(&[0; PARAGRAPH_END_BYTES]);
    control_list(11, &picture)
}

#[test]
fn hwp3_context는_paragraph_drawing_textbox의_공유_depth와_node를_지킨다() {
    let mut limits = Hwp3Limits {
        max_depth: CONTEXT_DEPTH,
        ..Hwp3Limits::default()
    };
    let accepted = with_list(&hidden_list(CONTEXT_DEPTH - 1));
    assert!(parse_hwp3_with_limits(&accepted, &limits).is_ok());
    let rejected = with_list(&hidden_list(CONTEXT_DEPTH));
    assert!(matches!(
        parse_hwp3_with_limits(&rejected, &limits),
        Err(Hwp3Error::LimitExceeded { .. })
    ));
    let drawing = with_list(&drawing_list());
    let parsed = parse_hwp3_with_limits(&drawing, &limits).unwrap();
    assert!(
        parsed.sections[0].paragraphs[0]
            .controls
            .iter()
            .any(|control| matches!(control, Control::Shape(_)))
    );
    limits.max_depth = CONTEXT_DEPTH - 1;
    assert!(matches!(
        parse_hwp3_with_limits(&drawing, &limits),
        Err(Hwp3Error::LimitExceeded { .. })
    ));
    limits = Hwp3Limits {
        max_nodes: BASIC_NODES,
        ..Hwp3Limits::default()
    };
    let normal = with_list(&text_body()[BODY_PREFIX_BYTES..]);
    let parsed = parse_hwp3_with_limits(&normal, &limits).unwrap();
    assert_eq!(parsed.sections[0].paragraphs[0].text, "Synthetic HWP3");
    limits.max_nodes -= 1;
    assert!(matches!(
        parse_hwp3_with_limits(&normal, &limits),
        Err(Hwp3Error::LimitExceeded { .. })
    ));
}

#[test]
fn hwp3_context는_개별_표와_합산_grid를_셀_ir_생성전에_제한한다() {
    let mut limits = Hwp3Limits {
        max_table_grid_slots: TABLE_GRID_SLOTS,
        ..Hwp3Limits::default()
    };
    let table = table_list();
    let accepted = parse_hwp3_with_limits(&with_list(&table), &limits).unwrap();
    let Control::Table(parsed) = &accepted.sections[0].paragraphs[0].controls[0] else {
        panic!("table fixture must produce an actual table")
    };
    assert_eq!([parsed.row_count, parsed.col_count], [3, 3]);
    assert_eq!(parsed.cells.len(), 2);
    limits.max_table_grid_slots -= 1;
    assert!(matches!(
        parse_hwp3_with_limits(&with_list(&table), &limits),
        Err(Hwp3Error::LimitExceeded { .. })
    ));
    limits.max_table_grid_slots = TABLE_GRID_SLOTS;
    let mut pair = table[..table.len() - PARAGRAPH_END_BYTES].to_vec();
    pair.extend_from_slice(&table);
    assert!(matches!(
        parse_hwp3_with_limits(&with_list(&pair), &limits),
        Err(Hwp3Error::LimitExceeded { .. })
    ));
}

struct ChunkProbe(Cursor<Vec<u8>>);

impl Read for ChunkProbe {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        assert!(
            bytes.len() <= READ_CHUNK_BYTES,
            "record allocation must not follow untrusted length"
        );
        self.0.read(bytes)
    }
}

fn compressed(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn text_body() -> Vec<u8> {
    let mut body = Vec::new();
    for _ in 0..LANGUAGES {
        body.extend_from_slice(&1u16.to_le_bytes());
        let mut font = [0; FONT_NAME_BYTES];
        font[..b"Synthetic".len()].copy_from_slice(b"Synthetic");
        body.extend_from_slice(&font);
    }
    body.extend_from_slice(&0u16.to_le_bytes());
    let text = "Synthetic HWP3";
    let mut paragraph = [0; PARAGRAPH_BYTES];
    paragraph[0] = 1;
    paragraph[1..3].copy_from_slice(&((text.len() + 1) as u16).to_le_bytes());
    paragraph[3..5].copy_from_slice(&1u16.to_le_bytes());
    paragraph[12..14].copy_from_slice(&FONT_SIZE.to_le_bytes());
    paragraph[21..28].fill(FONT_RATIO);
    body.extend_from_slice(&paragraph);
    let mut line = [0; LINE_BYTES];
    line[4..6].copy_from_slice(&FONT_SIZE.to_le_bytes());
    body.extend_from_slice(&line);
    for character in text.bytes() {
        body.extend_from_slice(&u16::from(character).to_le_bytes());
    }
    body.extend_from_slice(&13u16.to_le_bytes());
    body.extend_from_slice(&[0; PARAGRAPH_END_BYTES]);
    body
}

fn header() -> Vec<u8> {
    let mut bytes = vec![0; SIGNATURE_BYTES + DOC_INFO_BYTES + SUMMARY_BYTES];
    bytes[..b"HWP Document File V3.00".len()].copy_from_slice(b"HWP Document File V3.00");
    bytes
}

#[test]
fn hwp3_variable_record는_거짓_길이와_실제_body를_작은_chunk로_읽는다() {
    let mut info = 1u32.to_le_bytes().to_vec();
    info.extend_from_slice(&(DECLARED_BYTES as u32).to_le_bytes());
    assert!(Hwp3AdditionalInfoBlock::read(ChunkProbe(Cursor::new(info))).is_err());
    let mut drawing = 0u32.to_le_bytes().to_vec();
    drawing.extend_from_slice(&(DECLARED_BYTES as u32).to_le_bytes());
    assert!(Hwp3DrawingTextBox::read(ChunkProbe(Cursor::new(drawing))).is_err());
    let ole = 0xf8995568u32.to_le_bytes().to_vec();
    assert!(Hwp3OleInfo::read(ChunkProbe(Cursor::new(ole)), DECLARED_BYTES as u32).is_err());
    let payload = vec![FONT_RATIO; READ_CHUNK_BYTES + 1];
    let mut complete = 1u32.to_le_bytes().to_vec();
    complete.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    complete.extend_from_slice(&payload);
    let record = Hwp3AdditionalInfoBlock::read(ChunkProbe(Cursor::new(complete))).unwrap();
    assert_eq!(record.data, payload);
}

#[test]
fn hwp3_decoded_body는_raw_deflate_예산과_원본_본문을_보존한다() {
    let decoded = vec![FONT_RATIO; DECODED_BYTES];
    assert!(matches!(
        decode_body(&decoded, false, DECODED_BYTES).unwrap(),
        Cow::Borrowed(_)
    ));
    let encoded = compressed(&decoded);
    assert_eq!(
        decode_body(&encoded, true, DECODED_BYTES).unwrap().as_ref(),
        decoded
    );
    assert!(decode_body(&encoded, true, SMALL_BUDGET).is_err());
    assert!(decode_body(&decoded, false, SMALL_BUDGET).is_err());
    let mut source = header();
    let offset = SIGNATURE_BYTES;
    source[offset + 6..offset + 8].copy_from_slice(&PAPER_LENGTH.to_le_bytes());
    source[offset + 8..offset + 10].copy_from_slice(&PAPER_WIDTH.to_le_bytes());
    let body = text_body();
    let mut raw = source.clone();
    raw.extend_from_slice(&body);
    source[SIGNATURE_BYTES + COMPRESSED_OFFSET] = 1;
    source.extend_from_slice(&compressed(&body));
    let mut pages = None;
    for bytes in [raw, source] {
        let core = DocumentCore::from_bytes(&bytes).unwrap();
        assert!(core.page_count() >= 1);
        let svg = core.render_page_svg_legacy_native(0).unwrap();
        assert!(svg.contains("Synthetic"));
        if let Some(pages) = pages {
            assert_eq!(core.page_count(), pages);
        }
        pages = Some(core.page_count());
    }
}

fn empty_body() -> Vec<u8> {
    vec![0; LANGUAGES * size_of::<u16>() + size_of::<u16>() + PARAGRAPH_END_BYTES]
}

#[test]
fn hwp3_engine는_잘린_info_block_끝을_slice_panic없이_거절한다() {
    let mut bytes = header();
    let offset = SIGNATURE_BYTES + INFO_LENGTH_OFFSET;
    bytes[offset..offset + size_of::<u16>()].copy_from_slice(&u16::MAX.to_le_bytes());
    let parsed = catch_unwind(|| parse_hwp3(&bytes));
    assert!(parsed.is_ok(), "truncated metadata must not panic");
    assert!(parsed.unwrap().is_err());
}

#[test]
fn hwp3_engine는_짧은_추가_image_metadata를_slice_panic없이_처리한다() {
    for length in IMAGE_MIN_BYTES..IMAGE_PAYLOAD_OFFSET {
        let mut bytes = header();
        bytes.extend_from_slice(&empty_body());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(length as u32).to_le_bytes());
        bytes.extend_from_slice(&vec![0; length]);
        let parsed = catch_unwind(|| parse_hwp3(&bytes));
        assert!(parsed.is_ok(), "short image metadata must not panic");
        assert!(parsed.unwrap().is_ok());
    }
}

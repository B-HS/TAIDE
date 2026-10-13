use super::*;
use taide_model::ids::TabId;
use taide_native_editor::store::EditorLimits;

const BYTE_LIMIT: usize = 64 * 1024;
const OWNER_LIMIT: usize = 4;

fn document(text: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: OWNER_LIMIT,
        max_views: OWNER_LIMIT,
        max_undo_groups: OWNER_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_untitled(TabId::new(), text, "rust".into())
        .unwrap();
    store.documents().snapshot(id).unwrap()
}

#[test]
fn ai_inline의_문맥은_utf16_상한과_유효한_문자경계를_지킨다() {
    let text = format!(
        "{}한{}",
        "\u{1f600}".repeat(PREFIX_UNITS),
        "\u{1f600}".repeat(SUFFIX_UNITS)
    );
    let snapshot = document(&text);
    let byte = text.find('한').unwrap() + '한'.len_utf8();
    let (prefix, suffix) = context_window(&snapshot, byte).unwrap();
    assert!(prefix.ends_with('한'));
    assert_eq!(prefix.encode_utf16().count(), PREFIX_UNITS - 1);
    assert_eq!(suffix.encode_utf16().count(), SUFFIX_UNITS);
    assert!(context_window(&snapshot, byte - 1).is_err());
}

#[test]
fn ai_inline의_빈문맥과_파일시작끝은_원문을_그대로_보존한다() {
    let snapshot = document("한\r\n尾");
    assert_eq!(
        context_window(&snapshot, 0).unwrap(),
        (String::new(), "한\r\n尾".into())
    );
    assert_eq!(
        context_window(&snapshot, snapshot.rope.len_bytes()).unwrap(),
        ("한\r\n尾".into(), String::new())
    );
    assert_eq!(
        context_window(&document(""), 0).unwrap(),
        (String::new(), String::new())
    );
}

#[test]
fn ai_inline의_설정은_활성화_provider_model을_모두_요구한다() {
    let mut settings = Settings::default();
    assert!(Config::new(&settings).is_none());
    settings.ai_auto_tab_enabled = true;
    assert!(Config::new(&settings).is_none());
    settings.ai_provider = Some(AiProviderId::Omlx);
    assert!(Config::new(&settings).is_none());
    settings.ai_model = Some("fixture".into());
    let before = Config::new(&settings).unwrap();
    settings.ai_omlx_base_url = Some("http://127.0.0.1".into());
    assert!(Config::new(&settings).unwrap() != before);
    settings.ai_model = Some(String::new());
    assert!(Config::new(&settings).is_none());
}

use serde_json::{Value, json};
use taide_model::settings::Settings;
use taide_model::theme::ResolvedTheme;
use taide_remote_web::presentation::{PresentationState, Read, SystemInputs};
use taide_remote_web::shell::Failure;
use taide_remote_web::{InvokeError, ResponsePayload};

const THEME_SEQ: u32 = 1;
const LOCALE_SEQ: u32 = 2;
const STALE_SEQ: u32 = 3;
const FRESH_SEQ: u32 = 4;
const RETRY_SEQ: u32 = 5;
const RECOVERED_SEQ: u32 = 6;
const COLOR_TOKEN_COUNT: usize = 15;

fn inputs(theme: &str) -> SystemInputs {
    SystemInputs {
        theme: theme.into(),
        language: "ko-KR".into(),
    }
}

fn theme(id: &str) -> Value {
    let keys = [
        "app.background",
        "app.foreground",
        "appSidebar.background",
        "appSidebar.iconDefault",
        "app.border",
        "app.focusBorder",
        "tabBar.tabActiveBackground",
        "tabBar.tabInactiveBackground",
        "tabBar.tabActiveIndicator",
        "editor.background",
        "editor.foreground",
        "editor.lineNumber",
        "editor.selection",
        "editor.cursor",
        "editor.lineHighlight",
    ];
    assert_eq!(keys.len(), COLOR_TOKEN_COUNT);
    let colors: serde_json::Map<String, Value> = keys
        .into_iter()
        .map(|key| (key.into(), json!("#102030")))
        .collect();
    json!({"id": id, "name": id, "type": "dark", "colors": colors, "syntax": {}, "terminal": {}})
}

fn complete(state: &mut PresentationState, read: Read, seq: u32, value: Value) {
    state.sent(read, seq);
    assert!(state.response(seq, &Ok(ResponsePayload::Json(value))));
    assert!(state.failures().is_empty(), "{:?}", state.failures());
}

#[test]
fn 원격_presentation은_필수_시스템_인자와_같은_색상_편집기_번역을_사용한다() {
    let mut state = PresentationState::new(inputs("dark"));
    assert!(state.theme().is_none());
    assert!(state.locale().is_none());
    assert!(state.shell_colors().is_none());
    assert!(state.message("hello", &[]).is_none());
    let theme_call = Read::Theme.call(state.inputs());
    assert_eq!(theme_call.command, "theme_get_current");
    assert_eq!(theme_call.args, json!({"systemTheme": "dark"}));
    let locale_call = Read::Locale.call(state.inputs());
    assert_eq!(locale_call.command, "locale_get_current");
    assert_eq!(locale_call.args, json!({"systemLanguage": "ko-KR"}));
    let settings = Settings::default();
    state.settings(&settings);
    assert_eq!(state.next_reads(), [Read::Theme, Read::Locale]);
    complete(&mut state, Read::Theme, THEME_SEQ, theme("synthetic"));
    complete(
        &mut state,
        Read::Locale,
        LOCALE_SEQ,
        json!({"id": "ko", "name": "Korean", "messages": {"hello": "안녕 {{name}}"}}),
    );
    let resolved: ResolvedTheme = serde_json::from_value(theme("synthetic")).unwrap();
    let colors = state.shell_colors().unwrap().unwrap();
    assert_eq!(
        colors.background,
        taide_native_ui::presentation::color(&resolved, "app.background").unwrap()
    );
    let editor = state.editor_appearance(&settings).unwrap().unwrap();
    assert_eq!(editor.font.size, settings.editor_font_size as f32);
    assert_eq!(
        editor.line_height,
        (editor.font.size * taide_native_ui::presentation::EDITOR_LINE_HEIGHT_FACTOR).round()
    );
    assert_eq!(
        state.message("hello", &[("name", "Rust")]).unwrap(),
        "안녕 Rust"
    );
    assert_eq!(state.message("missing", &[]).unwrap(), "missing");
    state.settings(&settings);
    assert!(state.next_reads().is_empty());
}

#[test]
fn 원격_presentation은_설정과_시스템_변경의_지연_응답을_버리고_한번만_재조회한다() {
    let mut state = PresentationState::new(inputs("dark"));
    let mut settings = Settings::default();
    state.settings(&settings);
    complete(&mut state, Read::Theme, THEME_SEQ, theme("initial"));
    complete(
        &mut state,
        Read::Locale,
        LOCALE_SEQ,
        json!({"id": "ko", "name": "Korean", "messages": {}}),
    );
    state.retry(Read::Theme);
    state.sent(Read::Theme, STALE_SEQ);
    state.set_inputs(inputs("light"));
    state.event("theme:changed", "{}");
    state.event("theme:changed", "{}");
    assert!(state.next_reads().is_empty());
    assert!(state.response(STALE_SEQ, &Ok(ResponsePayload::Json(theme("stale")))));
    assert_eq!(state.theme().unwrap().id, "initial");
    assert_eq!(state.next_reads(), [Read::Theme]);
    assert_eq!(
        Read::Theme.call(state.inputs()).args,
        json!({"systemTheme": "light"})
    );
    complete(&mut state, Read::Theme, FRESH_SEQ, theme("fresh"));
    assert_eq!(state.theme().unwrap().id, "fresh");
    settings.editor_font_size += 1;
    state.settings(&settings);
    assert!(state.next_reads().is_empty());
    settings.follow_system_theme = !settings.follow_system_theme;
    state.settings(&settings);
    assert_eq!(state.next_reads(), [Read::Theme]);
    complete(&mut state, Read::Theme, RETRY_SEQ, theme("system"));
    settings.language = "ja".into();
    state.event(
        "settings:changed",
        &json!({"settings": settings}).to_string(),
    );
    assert_eq!(state.next_reads(), [Read::Locale]);
    state.sent(Read::Locale, STALE_SEQ);
    settings.language = "zh".into();
    state.settings(&settings);
    assert!(state.response(
        STALE_SEQ,
        &Ok(ResponsePayload::Json(
            json!({"id": "ja", "name": "Japanese", "messages": {}})
        ))
    ));
    assert_eq!(state.locale().unwrap().id, "ko");
    assert_eq!(state.next_reads(), [Read::Locale]);
}

#[test]
fn 원격_presentation은_오류를_숨기지않고_명시_재시도와_재연결_소유권을_보존한다() {
    let mut state = PresentationState::new(inputs("dark"));
    state.refresh();
    state.sent(Read::Theme, THEME_SEQ);
    assert!(state.response(THEME_SEQ, &Ok(ResponsePayload::Binary(vec![]))));
    assert_eq!(
        state.failures().get(&Read::Theme),
        Some(&Failure::MalformedResponse)
    );
    assert!(state.theme().is_none());
    state.invocation_failed(Read::Locale, InvokeError::Closed);
    assert!(state.next_reads().is_empty());
    state.retry(Read::Theme);
    state.sent(Read::Theme, RETRY_SEQ);
    let remote_error = json!({"code": "INTERNAL", "message": "synthetic failure"});
    assert!(state.response(RETRY_SEQ, &Err(remote_error.clone())));
    assert_eq!(
        state.failures().get(&Read::Theme),
        Some(&Failure::Remote(remote_error))
    );
    state.retry(Read::Theme);
    state.sent(Read::Theme, STALE_SEQ);
    state.disconnected();
    assert!(!state.response(
        STALE_SEQ,
        &Ok(ResponsePayload::Json(theme("old-connection")))
    ));
    assert!(state.next_reads().is_empty());
    state.refresh();
    state.sent(Read::Theme, RECOVERED_SEQ);
    assert!(state.response(
        RECOVERED_SEQ,
        &Ok(ResponsePayload::Json(theme("recovered")))
    ));
    assert_eq!(state.theme().unwrap().id, "recovered");
    assert!(!state.failures().contains_key(&Read::Theme));
    assert_eq!(state.next_reads(), [Read::Locale]);
}

use taide_lib::domain::settings::types::{
    EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, Settings, SettingsPatch, TerminalCursorStyle,
};
use taide_model::settings::{
    EditorCursorBlinking as ModelEditorCursorBlinking, EditorCursorStyle as ModelEditorCursorStyle,
    EditorRenderWhitespace as ModelEditorRenderWhitespace, TerminalCursorStyle as ModelTerminalCursorStyle,
};

#[test]
fn 설정_선택지의_기본값과_기존_wire를_유지한다() {
    let whitespace: ModelEditorRenderWhitespace = serde_json::from_str("\"boundary\"").expect("공백 표시 선택지");
    let facade: EditorRenderWhitespace = whitespace;
    assert_eq!(facade, EditorRenderWhitespace::Boundary);
    assert_eq!(
        serde_json::to_value(EditorRenderWhitespace::default()).expect("기본 공백 표시"),
        "selection"
    );

    let cursor: ModelEditorCursorStyle = serde_json::from_str("\"underline\"").expect("편집기 커서 선택지");
    let facade: EditorCursorStyle = cursor;
    assert_eq!(facade, EditorCursorStyle::Underline);
    assert_eq!(
        serde_json::to_value(EditorCursorStyle::default()).expect("기본 편집기 커서"),
        "line"
    );

    let blinking: ModelEditorCursorBlinking = serde_json::from_str("\"phase\"").expect("커서 깜빡임 선택지");
    let facade: EditorCursorBlinking = blinking;
    assert_eq!(facade, EditorCursorBlinking::Phase);
    assert_eq!(serde_json::to_value(EditorCursorBlinking::default()).expect("기본 깜빡임"), "blink");

    let terminal: ModelTerminalCursorStyle = serde_json::from_str("\"block\"").expect("터미널 커서 선택지");
    let facade: TerminalCursorStyle = terminal;
    assert_eq!(facade, TerminalCursorStyle::Block);
    assert_eq!(
        serde_json::to_value(TerminalCursorStyle::default()).expect("기본 터미널 커서"),
        "bar"
    );
}

#[test]
fn 설정과_패치의_선택지_필드가_기존_wire를_유지한다() {
    let legacy = serde_json::json!({
        "version": 1,
        "editorRenderWhitespace": "all",
        "editorCursorStyle": "block",
        "editorCursorBlinking": "smooth",
        "terminalCursorStyle": "underline"
    });
    let settings: Settings = serde_json::from_value(legacy.clone()).expect("기존 설정");
    assert_eq!(settings.editor_render_whitespace, EditorRenderWhitespace::All);
    assert_eq!(settings.editor_cursor_style, EditorCursorStyle::Block);
    assert_eq!(settings.editor_cursor_blinking, EditorCursorBlinking::Smooth);
    assert_eq!(settings.terminal_cursor_style, TerminalCursorStyle::Underline);
    let saved = serde_json::to_value(settings).expect("설정 직렬화");
    for key in [
        "editorRenderWhitespace",
        "editorCursorStyle",
        "editorCursorBlinking",
        "terminalCursorStyle",
    ] {
        assert_eq!(saved[key], legacy[key], "기존 설정 필드: {key}");
    }

    let patch: SettingsPatch = serde_json::from_value(serde_json::json!({
        "editorRenderWhitespace": "none",
        "editorCursorStyle": "underline",
        "editorCursorBlinking": "solid",
        "terminalCursorStyle": "block"
    }))
    .expect("기존 설정 패치");
    assert_eq!(patch.editor_render_whitespace, Some(EditorRenderWhitespace::None));
    assert_eq!(patch.editor_cursor_style, Some(EditorCursorStyle::Underline));
    assert_eq!(patch.editor_cursor_blinking, Some(EditorCursorBlinking::Solid));
    assert_eq!(patch.terminal_cursor_style, Some(TerminalCursorStyle::Block));
    let saved_patch = serde_json::to_value(patch).expect("설정 패치 직렬화");
    assert_eq!(saved_patch["editorCursorBlinking"], "solid");
    assert_eq!(saved_patch["terminalCursorStyle"], "block");
}

use taide_lib::domain::file::types::{EditorConfigOptions, FileSizeTier, FsChange, OpenedFile};
use taide_lib::domain::font::types::FontFamily;
use taide_lib::domain::tree::types::{TreeEntryKind, TreeRowPage};
use taide_model::file::{FsChange as ModelFsChange, OpenedFile as ModelOpenedFile};

#[test]
fn 파일_변경과_편집기_옵션의_기존_wire와_타입을_유지한다() {
    let wire = serde_json::json!({ "kind": "renamed", "paths": ["old.rs", "new.rs"], "fromApp": false });
    let change: ModelFsChange = serde_json::from_value(wire.clone()).expect("파일 변경 이벤트");
    let _: FsChange = change.clone();
    assert_eq!(serde_json::to_value(change).expect("파일 변경 직렬화"), wire);

    let defaults: taide_model::file::EditorConfigOptions = EditorConfigOptions::default();
    assert!(defaults.indent_style.is_none());
    assert!(defaults.indent_size.is_none());

    let opened: ModelOpenedFile = serde_json::from_value(serde_json::json!({
        "path": "main.rs", "content": "fn main() {}", "languageId": "rust", "byteSize": 12,
        "lineCount": 1, "tier": "readOnly", "readOnly": true, "encodingLossy": false,
        "modifiedMs": 0.0, "editorConfig": {
            "indentStyle": "space", "indentSize": 2, "tabWidth": null,
            "insertFinalNewline": null, "trimTrailingWhitespace": null
        }
    }))
    .expect("기존 파일 열기 응답");
    let facade: OpenedFile = opened;
    assert_eq!(facade.tier, FileSizeTier::ReadOnly);
    assert_eq!(facade.editor_config.indent_size, Some(2));
}

#[test]
fn 트리_페이지의_기존_wire와_공개_타입을_유지한다() {
    let wire = serde_json::json!({
        "rows": [{
            "path": "src", "name": "src", "kind": "directory", "depth": 0,
            "expanded": true, "hasChildren": true
        }],
        "total": 1
    });
    let page: taide_model::tree::TreeRowPage = serde_json::from_value(wire.clone()).expect("기존 트리 페이지");
    let facade: TreeRowPage = page.clone();
    assert_eq!(facade.rows[0].kind, TreeEntryKind::Directory);
    assert_eq!(serde_json::to_value(page).expect("트리 페이지 직렬화"), wire);
}

#[test]
fn 글꼴_목록의_기존_wire와_공개_타입을_유지한다() {
    let wire = serde_json::json!({ "name": "Mono", "monospaced": true });
    let model: taide_model::font::FontFamily = serde_json::from_value(wire.clone()).expect("글꼴 항목");
    let facade: FontFamily = model.clone();
    assert!(facade.monospaced);
    assert_eq!(serde_json::to_value(model).expect("글꼴 직렬화"), wire);
}

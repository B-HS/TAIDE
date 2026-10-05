use serde_json::{Value, json};
use taide_editor_core_spike::{DocumentProbe, Edit, EditError};
use taide_lsp::native::feature::TypedReply;
use taide_lsp::native::protocol::lsp_types::request::Formatting;
use taide_lsp::native::protocol::lsp_types::{Position, Range, TextEdit};
use taide_lsp::native::{DocumentMirror, Failure, LspCoordinator};

const MAX_UINT: u32 = 2147483647;
const TIMEOUT_MS: u64 = 500;
const URI: &str = "file:///synthetic/editor.rs";

fn position(line: u32, character: u32) -> Position {
    Position::new(line, character)
}

fn edit(start: Position, end: Position, text: &str) -> TextEdit {
    TextEdit {
        range: Range::new(start, end),
        new_text: text.into(),
    }
}

#[test]
fn lsp_좌표는_cr_lf_crlf만_줄로_세며_surrogate와_crlf_중간을_거절한다() {
    let text = "한𐐀e\u{301}\r\n日本\r中\n\u{2028}x\u{85}";
    let document = DocumentProbe::new(text);
    assert_eq!(document.rope.len_lines(), 4);
    for (line, content) in ["한𐐀e\u{301}", "日本", "中", "\u{2028}x\u{85}"]
        .into_iter()
        .enumerate()
    {
        let mut units = 0;
        for scalar in content.chars().chain(std::iter::once('\0')) {
            let point = position(u32::try_from(line).unwrap(), units);
            let byte = document.position_to_byte(0, point).unwrap();
            assert_eq!(document.byte_to_position(0, byte), Ok(point));
            units += u32::try_from(scalar.len_utf16()).unwrap();
        }
    }
    assert_eq!(
        document.position_to_byte(0, position(0, 2)),
        Err(EditError::InvalidBoundary)
    );
    assert_eq!(
        document.position_to_byte(0, position(0, MAX_UINT)),
        Ok(text.find('\r').unwrap())
    );
    assert_eq!(
        document.byte_to_position(0, text.find('\n').unwrap()),
        Err(EditError::InvalidBoundary)
    );
    assert_eq!(
        document.byte_to_position(0, 1),
        Err(EditError::InvalidBoundary)
    );
    assert_eq!(
        document.position_to_byte(0, position(4, 0)),
        Err(EditError::InvalidBoundary)
    );
    assert_eq!(
        document.position_to_byte(0, position(0, u32::MAX)),
        Err(EditError::InvalidBoundary)
    );
    assert_eq!(
        document.position_to_byte(1, position(0, 0)),
        Err(EditError::StaleRevision)
    );
    for empty in ["", "\n", "\r", "\r\n"] {
        let document = DocumentProbe::new(empty);
        let end = document.byte_to_position(0, empty.len()).unwrap();
        assert_eq!(document.position_to_byte(0, end), Ok(empty.len()));
    }
}

#[test]
fn lsp_edit는_동일_위치_삽입_순서를_보존하고_원자적_거절과_undo_revision을_공유한다() {
    let mut document = DocumentProbe::new("한𐐀e\u{301}\r\n日本");
    document
        .apply_lsp_edits(
            0,
            vec![
                edit(position(1, 0), position(1, 2), "中文"),
                edit(position(0, 1), position(0, 1), "A"),
                edit(position(0, 1), position(0, 1), "B"),
                edit(position(0, 1), position(0, 3), "C"),
            ],
        )
        .unwrap();
    assert_eq!(document.rope.to_string(), "한ABCe\u{301}\r\n中文");
    assert_eq!(document.revision, 1);
    let before = document.rope.to_string();
    for (revision, edits, expected) in [
        (
            0,
            vec![edit(position(0, 0), position(0, 0), "stale")],
            EditError::StaleRevision,
        ),
        (
            1,
            vec![
                edit(position(0, 0), position(0, 2), "first"),
                edit(position(0, 1), position(0, 3), "overlap"),
            ],
            EditError::Overlap,
        ),
        (
            1,
            vec![
                edit(position(0, 1), position(0, 2), "replace"),
                edit(position(0, 1), position(0, 1), "insert"),
            ],
            EditError::Overlap,
        ),
        (
            1,
            vec![edit(position(0, 2), position(0, 1), "reversed")],
            EditError::InvalidBoundary,
        ),
        (
            1,
            vec![
                edit(position(0, 0), position(0, 0), "valid"),
                edit(position(2, 0), position(2, 1), "invalid"),
            ],
            EditError::InvalidBoundary,
        ),
    ] {
        assert_eq!(document.apply_lsp_edits(revision, edits), Err(expected));
        assert_eq!(document.rope.to_string(), before);
        assert_eq!(document.revision, 1);
    }
    assert!(document.undo().unwrap());
    assert_eq!(document.rope.to_string(), "한𐐀e\u{301}\r\n日本");
    assert_eq!(document.revision, 2);
    assert!(document.redo().unwrap());
    assert_eq!(document.rope.to_string(), before);
    assert_eq!(document.revision, 3);
}

fn coordinator(text: &str) -> LspCoordinator {
    let mut coordinator = LspCoordinator::new(TIMEOUT_MS);
    coordinator
        .open(DocumentMirror {
            uri: URI.into(),
            language_id: "rust".into(),
            revision: 0,
            version: 0,
            text: text.into(),
        })
        .unwrap();
    let init = coordinator.begin(0, json!({})).unwrap();
    coordinator.receive(0,1,json!({"jsonrpc":"2.0","id":init["id"],"result":{"capabilities":{"positionEncoding":"utf-16","textDocumentSync":2,"documentFormattingProvider":true}}})).unwrap();
    assert!(coordinator.finish_replay(0).unwrap().is_empty());
    coordinator
}

fn formatting(coordinator: &mut LspCoordinator, revision: u64) -> Value {
    coordinator
        .request(
            2,
            "textDocument/formatting",
            json!({"textDocument":{"uri":URI},"options":{"tabSize":4,"insertSpaces":true}}),
            Some((URI, revision)),
        )
        .unwrap()
}

#[test]
fn native_formatting_reply는_편집기_revision과_undo_mirror를_공유하고_stale_결과를_반영하지_않는다()
{
    let mut document = DocumentProbe::new("한𐐀\r\n日本");
    let mut coordinator = coordinator(&document.rope.to_string());
    let request = formatting(&mut coordinator, 0);
    let result = json!([{"range":{"start":{"line":0,"character":1},"end":{"line":0,"character":3}},"newText":"CJK"}]);
    let completed = coordinator
        .receive(
            0,
            3,
            json!({"jsonrpc":"2.0","id":request["id"],"result":result}),
        )
        .unwrap();
    let reply =
        TypedReply::<Formatting>::decode(completed.completed[0].result.clone().unwrap()).unwrap();
    document.apply_lsp_edits(0, reply.value.unwrap()).unwrap();
    assert_eq!(document.rope.to_string(), "한CJK\r\n日本");
    let changed = coordinator
        .change(URI, 0, document.revision, document.rope.to_string())
        .unwrap()
        .unwrap();
    assert_eq!(
        changed["params"]["contentChanges"][0]["range"]["end"],
        json!({"line":1,"character":2})
    );
    let pending = formatting(&mut coordinator, 1);
    document
        .apply(
            1,
            vec![Edit {
                bytes: 0..0,
                text: "typing".into(),
            }],
        )
        .unwrap();
    coordinator
        .change(URI, 1, document.revision, document.rope.to_string())
        .unwrap();
    let completed = coordinator
        .receive(
            0,
            4,
            json!({"jsonrpc":"2.0","id":pending["id"],"result":result}),
        )
        .unwrap();
    assert_eq!(completed.completed[0].result, Err(Failure::StaleRevision));
    let before = document.rope.to_string();
    assert_eq!(
        document.apply_lsp_edits(1, vec![edit(position(0, 0), position(0, 1), "stale")]),
        Err(EditError::StaleRevision)
    );
    assert_eq!(document.rope.to_string(), before);
    assert!(document.undo().unwrap());
    coordinator
        .change(URI, 2, document.revision, document.rope.to_string())
        .unwrap();
    assert_eq!(document.rope.to_string(), "한CJK\r\n日本");
}

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::{LineSyntax, UntokenizedLines};
use taide_native_editor::line_commands::{LineCommand, LineCommandContext, run_line_command};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::{Token, TokenKind};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 64 * 1024;
const TAB_SIZE: u32 = 4;
const SPACES: IndentOptions = IndentOptions {
    tab_size: TAB_SIZE,
    insert_spaces: true,
};
const CONTEXT: LineCommandContext<'static> = LineCommandContext {
    indent: SPACES,
    language: None,
    syntax: &UntokenizedLines,
    compare: Some(&str::cmp),
    transforms: None,
    word_rules: None,
};

fn fixture(text: &str, ranges: &[(usize, usize)], read_only: bool) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/line-commands.txt".into(),
            OpenedFile {
                path: "/synthetic/line-commands.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: ranges
                    .iter()
                    .map(|(anchor, head)| Selection {
                        anchor: *anchor,
                        head: *head,
                    })
                    .collect(),
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    (store, view)
}

fn content(store: &EditorStore, view: ViewId) -> String {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
        .rope
        .to_string()
}

fn ranges(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections
        .iter()
        .map(|selection| (selection.anchor, selection.head))
        .collect()
}

fn assert_command(
    command: LineCommand,
    text: &str,
    before: &[(usize, usize)],
    expected: &str,
    after: &[(usize, usize)],
) {
    let (mut store, view) = fixture(text, before, false);
    assert!(run_line_command(&mut store, view, command, CONTEXT).unwrap());
    assert_eq!(content(&store, view), expected);
    assert_eq!(ranges(&store, view), after);
    let document = store.views().get(view).unwrap().document;
    assert!(store.undo(document).unwrap());
    assert_eq!(content(&store, view), text);
    assert_eq!(ranges(&store, view), before);
    assert!(store.redo(document).unwrap());
    assert_eq!(content(&store, view), expected);
    assert_eq!(ranges(&store, view), after);
}

#[test]
fn transpose는_utf16_단위와_crlf를_원본대로_바꾸고_하나의_undo로_복구한다() {
    assert_command(
        LineCommand::Transpose,
        "a\u{1f600}b",
        &[(1, 1)],
        "\u{fffd}\u{fffd}ab",
        &[(6, 6)],
    );
    assert_command(
        LineCommand::Transpose,
        "a\r\nb",
        &[(1, 1)],
        "\r\n\r\nab",
        &[(5, 5)],
    );
    assert_command(
        LineCommand::Transpose,
        "ab\ncd\nef",
        &[(1, 1), (4, 4), (6, 8)],
        "ba\ndc\nef",
        &[(2, 2), (5, 5), (6, 8)],
    );
}

#[test]
fn transpose는_마지막_줄끝과_선택을_보존하고_내용이_같아도_커서를_이동한다() {
    let (mut store, view) = fixture("ab", &[(2, 2)], false);
    assert!(!run_line_command(&mut store, view, LineCommand::Transpose, CONTEXT).unwrap());
    assert_eq!(ranges(&store, view), [(2, 2)]);
    let (mut store, view) = fixture("a", &[(0, 0)], false);
    assert!(!run_line_command(&mut store, view, LineCommand::Transpose, CONTEXT).unwrap());
    assert_eq!(content(&store, view), "a");
    assert_eq!(ranges(&store, view), [(1, 1)]);
    let (mut store, view) = fixture("ab", &[(1, 0)], false);
    assert!(!run_line_command(&mut store, view, LineCommand::Transpose, CONTEXT).unwrap());
    assert_eq!(ranges(&store, view), [(1, 0)]);
    let (mut store, view) = fixture("ab", &[(1, 1)], true);
    assert_eq!(
        run_line_command(&mut store, view, LineCommand::Transpose, CONTEXT),
        Err(EditorError::ReadOnly)
    );
}

#[test]
fn 줄_이동은_유니코드와_다음_줄_시작에서_끝나는_선택을_보존한다() {
    assert_command(
        LineCommand::MoveLinesUp,
        "one\nλ\nlast",
        &[(4, 4)],
        "λ\none\nlast",
        &[(0, 0)],
    );
    assert_command(
        LineCommand::MoveLinesDown,
        "a\nb\nc\nd",
        &[(0, 4)],
        "c\na\nb\nd",
        &[(2, 6)],
    );
}

#[test]
fn 줄_복사는_위에서는_원래_위치_아래에서는_복사된_줄을_선택한다() {
    assert_command(
        LineCommand::CopyLinesUp,
        "one\nlast",
        &[(1, 1)],
        "one\none\nlast",
        &[(1, 1)],
    );
    assert_command(
        LineCommand::CopyLinesDown,
        "one\nlast",
        &[(1, 1)],
        "one\none\nlast",
        &[(5, 5)],
    );
    assert_command(LineCommand::CopyLinesUp, "", &[(0, 0)], "\n", &[(0, 0)]);
    assert_command(LineCommand::CopyLinesDown, "", &[(0, 0)], "\n", &[(1, 1)]);
}

#[test]
fn 선택_복제는_새로_복제된_텍스트를_선택한다() {
    assert_command(
        LineCommand::DuplicateSelection,
        "abcd",
        &[(1, 3)],
        "abcbcd",
        &[(3, 5)],
    );
    assert_command(
        LineCommand::DuplicateSelection,
        "one\nlast",
        &[(1, 1)],
        "one\none\nlast",
        &[(5, 5)],
    );
}

#[test]
fn 줄_삭제는_마지막_줄과_인접_커서의_범위를_처리한다() {
    assert_command(
        LineCommand::DeleteLines,
        "a\nβ\nlong",
        &[(2, 2)],
        "a\nlong",
        &[(2, 2)],
    );
    assert_command(
        LineCommand::DeleteLines,
        "one\nlast",
        &[(6, 6)],
        "one",
        &[(2, 2)],
    );
    assert_command(
        LineCommand::DeleteLines,
        "a\nb\nc",
        &[(0, 0), (2, 2)],
        "c",
        &[(0, 0)],
    );
}

#[test]
fn 줄_삽입은_선택된_문자를_지우지_않고_새_줄로_이동한다() {
    assert_command(
        LineCommand::InsertLineBefore,
        "one\nlast",
        &[(5, 7)],
        "one\n\nlast",
        &[(4, 4)],
    );
    assert_command(
        LineCommand::InsertLineBefore,
        "one",
        &[(2, 2)],
        "\none",
        &[(0, 0)],
    );
    assert_command(
        LineCommand::InsertLineAfter,
        "one\nlast",
        &[(1, 1)],
        "one\n\nlast",
        &[(4, 4)],
    );
    assert_command(
        LineCommand::InsertLineAfter,
        "  one",
        &[(1, 1)],
        "  one\n  ",
        &[(8, 8)],
    );
}

#[test]
fn 줄_합치기는_들여쓰기를_지우고_접합_위치에_커서를_둔다() {
    assert_command(
        LineCommand::JoinLines,
        "a  \n  b\nlast",
        &[(1, 1)],
        "a b\nlast",
        &[(2, 2)],
    );
    assert_command(LineCommand::JoinLines, "\n  b", &[(0, 0)], "b", &[(0, 0)]);
    assert_command(
        LineCommand::JoinLines,
        "a\r\n  b\r\nlast",
        &[(1, 1)],
        "a b\r\nlast",
        &[(1, 1)],
    );
}

#[test]
fn 줄_명령은_읽기_전용_문서를_바꾸지_않는다() {
    let (mut store, view) = fixture("one\nlast", &[(1, 1)], true);
    assert_eq!(
        run_line_command(&mut store, view, LineCommand::CopyLinesDown, CONTEXT),
        Err(EditorError::ReadOnly),
    );
    assert_eq!(content(&store, view), "one\nlast");
}

#[test]
fn 문서_경계의_줄_이동은_내용과_선택을_유지한다() {
    for (command, caret) in [
        (LineCommand::MoveLinesUp, 0),
        (LineCommand::MoveLinesDown, 2),
    ] {
        let (mut store, view) = fixture("a\nb", &[(caret, caret)], false);
        assert!(!run_line_command(&mut store, view, command, CONTEXT).unwrap());
        assert_eq!(content(&store, view), "a\nb");
        assert_eq!(ranges(&store, view), [(caret, caret)]);
    }
}

#[test]
fn 오른쪽_전부_삭제는_선택과_줄_끝의_개행과_인접_범위를_처리한다() {
    assert_command(
        LineCommand::DeleteAllRight,
        "abc\ndef",
        &[(1, 1)],
        "a\ndef",
        &[(1, 1)],
    );
    assert_command(
        LineCommand::DeleteAllRight,
        "abc\ndef",
        &[(3, 3)],
        "abcdef",
        &[(3, 3)],
    );
    assert_command(
        LineCommand::DeleteAllRight,
        "abc\ndef",
        &[(1, 2)],
        "ac\ndef",
        &[(1, 1)],
    );
    assert_command(
        LineCommand::DeleteAllRight,
        "abc\ndef",
        &[(3, 3), (4, 4)],
        "abc",
        &[(3, 3)],
    );
}

#[test]
fn 왼쪽_전부_삭제는_선택이_시작하는_줄의_앞을_지운다() {
    assert_command(
        LineCommand::DeleteAllLeft,
        "abc\ndef",
        &[(5, 6)],
        "abc\nf",
        &[(4, 4)],
    );
}

#[test]
fn 단어_삭제는_커서가_포함된_단어를_지운다() {
    assert_command(
        LineCommand::DeleteInsideWord,
        "a hello z",
        &[(4, 4)],
        "a z",
        &[(2, 2)],
    );
}

struct CleanupTokens;

impl LineSyntax for CleanupTokens {
    fn tokens(&self, _document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        Some(vec![Token {
            start_byte: 0,
            kind: match line {
                0 => TokenKind::String,
                1 => TokenKind::Regex,
                2 => TokenKind::Comment,
                _ => TokenKind::Other,
            },
        }])
    }

    fn accurate_tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        if line == 3 {
            return None;
        }
        self.tokens(document, line)
    }

    fn kind_if_inserting(
        &self,
        _document: &DocumentSnapshot,
        _line: usize,
        _byte_in_line: usize,
        _character: char,
    ) -> TokenKind {
        TokenKind::Other
    }
}

#[test]
fn 공백_정리는_문자열과_정규식과_미확정_토큰을_보존한다() {
    let (mut store, view) = fixture(
        "string \nregex \ncomment \npending \ncode ",
        &[(0, 0)],
        false,
    );
    assert!(
        run_line_command(
            &mut store,
            view,
            LineCommand::TrimTrailingWhitespace,
            LineCommandContext {
                syntax: &CleanupTokens,
                ..CONTEXT
            },
        )
        .unwrap()
    );
    assert_eq!(
        content(&store, view),
        "string \nregex \ncomment\npending \ncode"
    );
}

#[test]
fn 명시적_줄_들여쓰기는_선택한_줄과_선택_방향을_보존한다() {
    assert_command(
        LineCommand::IndentLines,
        "a\nb\nc",
        &[(0, 4)],
        "    a\n    b\nc",
        &[(0, 12)],
    );
}

#[test]
fn 명시적_줄_내어쓰기는_역방향_선택을_보존한다() {
    assert_command(
        LineCommand::OutdentLines,
        "    a\n    b\nc",
        &[(12, 0)],
        "a\nb\nc",
        &[(4, 0)],
    );
}

#[test]
fn 공백_정리는_공백과_탭만_지우고_주_선택을_추적한다() {
    assert_command(
        LineCommand::TrimTrailingWhitespace,
        "a \t\nb\u{a0}",
        &[(0, 0), (5, 5)],
        "a\nb\u{a0}",
        &[(0, 0)],
    );
}

#[test]
fn 마지막_개행은_기존_줄_끝_형식과_주_선택을_보존한다() {
    assert_command(
        LineCommand::InsertFinalNewLine,
        "abc",
        &[(3, 3)],
        "abc\n",
        &[(3, 3)],
    );
    assert_command(
        LineCommand::InsertFinalNewLine,
        "a\r\nb",
        &[(4, 4)],
        "a\r\nb\r\n",
        &[(4, 4)],
    );
}

#[test]
fn 공백뿐인_마지막_줄과_기존_마지막_개행에는_개행을_더하지_않는다() {
    for text in ["", " \t", "a\n", "a\n \t"] {
        let (mut store, view) = fixture(text, &[(0, 0)], false);
        assert!(
            !run_line_command(&mut store, view, LineCommand::InsertFinalNewLine, CONTEXT).unwrap()
        );
        assert_eq!(content(&store, view), text);
    }
}

#[test]
fn 한_줄_선택의_정렬은_전체_문서를_정렬하고_끝_빈_줄을_유지한다() {
    assert_command(
        LineCommand::SortLinesAscending,
        "b\na\n",
        &[(0, 0)],
        "a\nb\n",
        &[(0, 4)],
    );
    assert_command(
        LineCommand::SortLinesDescending,
        "a\nb",
        &[(0, 0)],
        "b\na",
        &[(0, 3)],
    );
}

#[test]
fn 정렬은_단일_행_선택이_섞이거나_이미_정렬된_범위가_있으면_전체를_중단한다() {
    let text = "a\nb\nz\nc";
    let (mut store, view) = fixture(text, &[(0, 3), (4, 7)], false);
    assert!(!run_line_command(&mut store, view, LineCommand::SortLinesAscending, CONTEXT).unwrap());
    assert_eq!(content(&store, view), text);
}

#[test]
fn 중복_제거는_선택_끝의_줄까지_포함하고_단일_커서를_추적한다() {
    assert_command(
        LineCommand::RemoveDuplicateLines,
        "a\na\nb",
        &[(2, 2)],
        "a\nb",
        &[(2, 2)],
    );
    assert_command(
        LineCommand::RemoveDuplicateLines,
        "a\na\nb\nb",
        &[(0, 6)],
        "a\nb",
        &[(0, 3)],
    );
}

#[test]
fn 줄_역순은_양_끝의_줄과_열과_선택_방향을_역으로_옮긴다() {
    assert_command(
        LineCommand::ReverseLines,
        "one\ntwo\nlast\n",
        &[(1, 1)],
        "last\ntwo\none\n",
        &[(10, 10)],
    );
    assert_command(
        LineCommand::ReverseLines,
        "one\ntwo\nlast",
        &[(2, 9)],
        "last\ntwo\none",
        &[(11, 1)],
    );
}

#[test]
fn 글자_맞바꾸기는_문서_끝과_줄_경계와_문자소를_처리한다() {
    assert_command(
        LineCommand::TransposeLetters,
        "abcd",
        &[(2, 2)],
        "acbd",
        &[(3, 3)],
    );
    assert_command(
        LineCommand::TransposeLetters,
        "abcd",
        &[(4, 4)],
        "abdc",
        &[(4, 4)],
    );
    assert_command(
        LineCommand::TransposeLetters,
        "a\nb",
        &[(2, 2)],
        "ab\n",
        &[(3, 3)],
    );
    assert_command(
        LineCommand::TransposeLetters,
        "a\u{1f600}b",
        &[(1, 1)],
        "\u{1f600}ab",
        &[(5, 5)],
    );
}

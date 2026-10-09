use serde::Deserialize;
use std::ops::Range;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::auto_closing::AutoClosedPairs;
use taide_native_editor::bracket_navigation::{
    bracket_pairs, enclosing_brackets, matching_brackets,
};
use taide_native_editor::cursor_commands::{
    CursorCommand, cursor_selection, literal_match_ranges, selection_ranges,
};
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::editing::line_content_range;
use taide_native_editor::folding::{
    FoldCommand, FoldRegion, MAX_FOLDING_REGIONS, indent_regions, language_regions,
    run_language_fold_command,
};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::{
    Language, LanguageRules, LineSyntax, UntokenizedLines, token_kind_at,
};
use taide_native_editor::language_typing::{
    Typing, commit_composition, delete_backward, insert_line_break, type_text,
};
use taide_native_editor::line_commands::{
    LineCommand, LineCommandContext, TextCase, run_line_command,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::{Token, TokenKind};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_syntax::{MonacoTextTransforms, monaco_language};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 16;
const BYTE_LIMIT: usize = 64 * 1024;
const TAB_SIZE: u32 = 4;
const SPACES: IndentOptions = IndentOptions {
    tab_size: TAB_SIZE,
    insert_spaces: true,
};
const TABS: IndentOptions = IndentOptions {
    tab_size: TAB_SIZE,
    insert_spaces: false,
};
const STRING_DELIMITER: char = '"';
const LINE_COMMENT: &str = "//";
const BLOCK_COMMENT_START: &str = "/*";
const BLOCK_COMMENT_END: &str = "*/";

#[derive(Deserialize)]
struct CursorOracle {
    language: String,
    text: String,
    byte: usize,
    matched: Option<Vec<(usize, usize)>>,
    enclosing: Option<Vec<(usize, usize)>>,
    jump: usize,
    select: Vec<(usize, usize)>,
    ranges: Vec<(usize, usize)>,
}

#[derive(Deserialize)]
struct MatchOracle {
    text: String,
    needle: String,
    match_case: bool,
    ranges: Vec<(usize, usize)>,
}

#[derive(Deserialize)]
struct TransposeOracle {
    text: String,
    byte: usize,
    expected: String,
    head: usize,
}

#[test]
fn 원본_monaco_transpose는_잘못된_utf16을_대체문자로_저장한다() {
    let cases: Vec<TransposeOracle> =
        serde_json::from_str(include_str!("fixtures/transpose-reference.json")).unwrap();
    let mut differences = Vec::new();
    for case in cases {
        let mut editor = Editor::new("javascript", &case.text);
        editor.select(&[(case.byte, case.byte)]);
        editor.command(LineCommand::Transpose);
        if editor.text() != case.expected || editor.selections() != vec![(case.head, case.head)] {
            differences.push((
                case.text,
                case.byte,
                case.expected,
                case.head,
                editor.text(),
                editor.selections(),
            ));
        }
    }
    assert!(differences.is_empty(), "{differences:#?}");
}

#[test]
fn 원본_monaco_괄호_파서와_스마트_선택_범위가_일치한다() {
    let cases: Vec<CursorOracle> =
        serde_json::from_str(include_str!("fixtures/cursor-ranges-reference.json")).unwrap();
    let mut differences = Vec::new();
    for case in cases {
        let mut editor = Editor::new(&case.language, &case.text);
        editor.syntax = &UntokenizedLines;
        let document = editor
            .store
            .documents()
            .snapshot(editor.store.views().get(editor.view).unwrap().document)
            .unwrap();
        let language = Language {
            rules: editor.rules,
            syntax: editor.syntax,
        };
        let context = LineCommandContext {
            indent: SPACES,
            language: Some(language),
            syntax: editor.syntax,
            compare: None,
            transforms: None,
            word_rules: Some(editor.rules),
        };
        let selection = SelectionSet {
            primary: 0,
            selections: vec![Selection {
                anchor: case.byte,
                head: case.byte,
            }],
        };
        let jump = cursor_selection(&document, &selection, CursorCommand::JumpToBracket, context)
            .selections[0]
            .head;
        let select: Vec<_> = cursor_selection(
            &document,
            &selection,
            CursorCommand::SelectToBracket,
            context,
        )
        .selections
        .into_iter()
        .map(|s| (s.anchor, s.head))
        .collect();
        let pairs = bracket_pairs(&document, language);
        let actual = |pair: Option<&taide_native_editor::bracket_navigation::MatchedBrackets>| {
            pair.map(|pair| {
                vec![
                    (pair.open.start, pair.open.end),
                    (pair.close.start, pair.close.end),
                ]
            })
        };
        let ranges = selection_ranges(
            &document,
            Selection {
                anchor: case.byte,
                head: case.byte,
            },
            LineCommandContext {
                indent: SPACES,
                language: Some(language),
                syntax: editor.syntax,
                compare: None,
                transforms: None,
                word_rules: Some(editor.rules),
            },
        );
        let mut ranges: Vec<_> = ranges
            .into_iter()
            .skip(1)
            .map(|range| (range.start, range.end))
            .collect();
        ranges.dedup();
        if actual(matching_brackets(&pairs, case.byte)) != case.matched
            || actual(enclosing_brackets(&pairs, case.byte)) != case.enclosing
            || ranges != case.ranges
            || jump != case.jump
            || select != case.select
        {
            differences.push(format!("{} {:?} byte {} matched {:?}/{:?} enclosing {:?}/{:?} ranges {:?}/{:?} jump {:?}/{:?} select {:?}/{:?}", case.language, case.text, case.byte, actual(matching_brackets(&pairs, case.byte)), case.matched, actual(enclosing_brackets(&pairs, case.byte)), case.enclosing, ranges, case.ranges, jump, case.jump, select, case.select));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences: {:?}",
        differences.len(),
        &differences[..differences.len().min(8)]
    );
}

#[test]
fn 원본_monaco_문자열_일치는_줄바꿈과_unicode_대소문자를_보존한다() {
    let cases: Vec<MatchOracle> =
        serde_json::from_str(include_str!("fixtures/cursor-matches-reference.json")).unwrap();
    for case in cases {
        let editor = Editor::new("plaintext", &case.text);
        let document = editor
            .store
            .documents()
            .snapshot(editor.store.views().get(editor.view).unwrap().document)
            .unwrap();
        let actual: Vec<_> = literal_match_ranges(&document, &case.needle, case.match_case, false)
            .map(|range| (range.start, range.end))
            .collect();
        assert_eq!(
            actual, case.ranges,
            "{} {:?} case {}",
            case.text, case.needle, case.match_case
        );
    }
}

struct CLikeSyntax;

fn c_like_tokens(text: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut push = |start_byte: usize, kind: TokenKind| {
        if tokens.last().is_none_or(|last| last.kind != kind) {
            tokens.push(Token { start_byte, kind });
        }
    };
    let mut byte = 0;
    while byte < text.len() {
        let rest = &text[byte..];
        let (kind, length) = if rest.starts_with(LINE_COMMENT) {
            (TokenKind::Comment, rest.len())
        } else if rest.starts_with(BLOCK_COMMENT_START) {
            let end = rest[BLOCK_COMMENT_START.len()..]
                .find(BLOCK_COMMENT_END)
                .map_or(rest.len(), |end| {
                    BLOCK_COMMENT_START.len() + end + BLOCK_COMMENT_END.len()
                });
            (TokenKind::Comment, end)
        } else if rest.starts_with(STRING_DELIMITER) {
            let end = rest[STRING_DELIMITER.len_utf8()..]
                .find(STRING_DELIMITER)
                .map_or(rest.len(), |end| end + 2 * STRING_DELIMITER.len_utf8());
            (TokenKind::String, end)
        } else {
            (
                TokenKind::Other,
                rest.chars().next().map_or(1, char::len_utf8),
            )
        };
        push(byte, kind);
        byte += length;
    }
    tokens
}

fn line(document: &DocumentSnapshot, line: usize) -> String {
    document
        .rope
        .byte_slice(line_content_range(document, line))
        .to_string()
}

impl LineSyntax for CLikeSyntax {
    fn tokens(&self, document: &DocumentSnapshot, index: usize) -> Option<Vec<Token>> {
        Some(c_like_tokens(&line(document, index)))
    }

    fn kind_if_inserting(
        &self,
        document: &DocumentSnapshot,
        index: usize,
        byte_in_line: usize,
        character: char,
    ) -> TokenKind {
        let mut text = line(document, index);
        text.insert(byte_in_line, character);
        token_kind_at(&c_like_tokens(&text), byte_in_line)
    }
}

struct Expensive;

impl LineSyntax for Expensive {
    fn tokens(&self, _: &DocumentSnapshot, _: usize) -> Option<Vec<Token>> {
        None
    }

    fn kind_if_inserting(&self, _: &DocumentSnapshot, _: usize, _: usize, _: char) -> TokenKind {
        TokenKind::Other
    }
}

struct Editor {
    store: EditorStore,
    view: ViewId,
    rules: &'static dyn LanguageRules,
    syntax: &'static dyn LineSyntax,
    indent: IndentOptions,
    auto_closed: AutoClosedPairs,
}

impl Editor {
    fn command(&mut self, command: LineCommand) -> bool {
        let transforms = MonacoTextTransforms::new().unwrap();
        run_line_command(
            &mut self.store,
            self.view,
            command,
            LineCommandContext {
                indent: self.indent,
                language: Some(Language {
                    rules: self.rules,
                    syntax: self.syntax,
                }),
                syntax: self.syntax,
                compare: Some(&str::cmp),
                transforms: Some(&transforms),
                word_rules: Some(self.rules),
            },
        )
        .unwrap()
    }
    fn new(language_id: &str, text: &str) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = store
            .open_file(
                "/synthetic/language-editing".into(),
                OpenedFile {
                    path: "/synthetic/language-editing".into(),
                    content: text.into(),
                    language_id: language_id.into(),
                    byte_size: text.len().try_into().unwrap(),
                    line_count: text.lines().count().try_into().unwrap(),
                    tier: FileSizeTier::Normal,
                    read_only: false,
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
        let mut editor = Self {
            store,
            view,
            rules: monaco_language(language_id).unwrap().unwrap(),
            syntax: &CLikeSyntax,
            indent: SPACES,
            auto_closed: AutoClosedPairs::default(),
        };
        editor.select(&[(text.len(), text.len())]);
        editor
    }

    fn typing(&mut self) -> (&mut EditorStore, ViewId, Typing<'_>) {
        (
            &mut self.store,
            self.view,
            Typing {
                language: Some(Language {
                    rules: self.rules,
                    syntax: self.syntax,
                }),
                indent: self.indent,
                auto_closed: &mut self.auto_closed,
            },
        )
    }

    fn select(&mut self, ranges: &[(usize, usize)]) {
        let current = self.store.views().get(self.view).unwrap().clone();
        self.store
            .set_view_state(
                self.view,
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
    }

    fn moves_caret(&mut self, offset: usize) {
        self.select(&[(offset, offset)]);
        self.auto_closed.follow(&self.store, self.view).unwrap();
    }

    fn caret_at(mut self, offset: usize) -> Self {
        self.select(&[(offset, offset)]);
        self
    }

    fn caret_after(self, prefix: &str) -> Self {
        let offset = self.text().find(prefix).unwrap() + prefix.len();
        self.caret_at(offset)
    }

    fn types(&mut self, text: &str) -> &mut Self {
        let (store, view, mut typing) = self.typing();
        type_text(store, view, text, &mut typing).unwrap();
        self
    }

    fn enters(&mut self) -> &mut Self {
        let (store, view, mut typing) = self.typing();
        insert_line_break(store, view, &mut typing).unwrap();
        self
    }

    fn backspaces(&mut self) -> &mut Self {
        let (store, view, mut typing) = self.typing();
        delete_backward(store, view, &mut typing).unwrap();
        self
    }

    fn commits(&mut self, replaced: Range<usize>, text: &str) -> &mut Self {
        let (store, view, mut typing) = self.typing();
        commit_composition(store, view, replaced, text, &mut typing).unwrap();
        self
    }

    fn undoes(&mut self) -> &mut Self {
        let document = self.store.views().get(self.view).unwrap().document;
        assert!(self.store.undo(document).unwrap());
        self
    }

    fn snapshot(&self) -> DocumentSnapshot {
        let document = self.store.views().get(self.view).unwrap().document;
        self.store.documents().snapshot(document).unwrap()
    }

    fn text(&self) -> String {
        self.snapshot().rope.to_string()
    }

    fn selections(&self) -> Vec<(usize, usize)> {
        self.store
            .views()
            .get(self.view)
            .unwrap()
            .selection
            .selections
            .iter()
            .map(|selection| (selection.anchor, selection.head))
            .collect()
    }

    fn marked(&self) -> String {
        let mut text = self.text();
        let mut heads: Vec<usize> = self.selections().iter().map(|(_, head)| *head).collect();
        heads.sort_unstable();
        for head in heads.into_iter().rev() {
            text.insert(head, '|');
        }
        text
    }
}

fn typescript(text: &str) -> Editor {
    Editor::new("typescript", text)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LineOracle {
    language_id: String,
    command: String,
    down: bool,
    text: String,
    first: usize,
    count: usize,
    expected: String,
}

#[test]
fn 줄_명령은_실제_monaco_명령의_언어별_결과와_일치한다() {
    let oracle: Vec<LineOracle> =
        serde_json::from_str(include_str!("fixtures/line-commands-reference.json")).unwrap();
    let mut differences = Vec::new();
    for row in oracle {
        let mut editor = Editor::new(&row.language_id, &row.text);
        editor.syntax = &UntokenizedLines;
        let document = editor.snapshot();
        let start = line_content_range(&document, row.first).start;
        let end = line_content_range(&document, row.first + row.count - 1).end;
        editor.select(&[(start, end)]);
        let command = match (row.command.as_str(), row.down) {
            ("move", false) => LineCommand::MoveLinesUp,
            ("move", true) => LineCommand::MoveLinesDown,
            ("shift", false) => LineCommand::IndentLines,
            ("shift", true) => LineCommand::OutdentLines,
            ("comment", _) => LineCommand::ToggleLineComment,
            ("addComment", _) => LineCommand::AddLineComment,
            ("removeComment", _) => LineCommand::RemoveLineComment,
            ("blockComment", _) => LineCommand::ToggleBlockComment,
            ("copy", false) => LineCommand::CopyLinesUp,
            ("copy", true) => LineCommand::CopyLinesDown,
            _ => unreachable!(),
        };
        editor.command(command);
        if editor.text() != row.expected {
            differences.push(format!(
                "{} {command:?} {}+{} {:?}: expected {:?}, actual {:?}",
                row.language_id,
                row.first,
                row.count,
                row.text,
                row.expected,
                editor.text()
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences\n{}",
        differences.len(),
        differences
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WordOracle {
    language_id: String,
    text: String,
    offset: usize,
    range: Option<(usize, usize)>,
}

fn utf16_byte(text: &str, target: usize) -> usize {
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        if units >= target {
            return byte;
        }
        units += character.len_utf16();
    }
    text.len()
}

#[test]
fn 구성된_단어_범위는_monaco의_단어_정의와_긴_줄_창에_일치한다() {
    let oracle: Vec<WordOracle> =
        serde_json::from_str(include_str!("fixtures/word-ranges-reference.json")).unwrap();
    let mut differences = Vec::new();
    for row in oracle {
        let rules = monaco_language(&row.language_id).unwrap().unwrap();
        let offset = utf16_byte(&row.text, row.offset);
        let expected = row
            .range
            .map(|(start, end)| utf16_byte(&row.text, start)..utf16_byte(&row.text, end));
        let actual = rules.word_range(&row.text, offset);
        if actual != expected {
            differences.push(format!(
                "{} {} {:?}: expected {expected:?}, actual {actual:?}",
                row.language_id, row.offset, row.text
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "{} differences\n{}",
        differences.len(),
        differences
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn 주석_명령은_공통_들여쓰기에_맞추고_빈_줄과_역방향_선택을_처리한다() {
    let mut editor = typescript("    a\n        b\n\n");
    editor.select(&[(15, 0)]);
    assert!(editor.command(LineCommand::ToggleLineComment));
    assert_eq!(editor.text(), "    // a\n    //     b\n\n");
    assert!(editor.command(LineCommand::ToggleLineComment));
    assert_eq!(editor.text(), "    a\n        b\n\n");
    let mut blank = typescript("    \n").caret_at(2);
    assert!(blank.command(LineCommand::ToggleLineComment));
    assert_eq!(blank.text(), "    // \n");
    let mut force = typescript("// a").caret_at(4);
    assert!(force.command(LineCommand::AddLineComment));
    assert_eq!(force.text(), "// // a");
    assert!(force.command(LineCommand::RemoveLineComment));
    assert_eq!(force.text(), "// a");
}

#[test]
fn 블록_주석은_선택을_앞방향으로_놓고_빈_커서는_가운데에_놓는다() {
    let mut editor = typescript("abcd");
    editor.select(&[(3, 1)]);
    assert!(editor.command(LineCommand::ToggleBlockComment));
    assert_eq!(editor.text(), "a/* bc */d");
    assert_eq!(editor.selections(), [(4, 6)]);
    assert!(editor.command(LineCommand::ToggleBlockComment));
    assert_eq!(editor.text(), "abcd");
    assert_eq!(editor.selections(), [(1, 3)]);
    let mut empty = typescript("");
    assert!(empty.command(LineCommand::ToggleBlockComment));
    assert_eq!(empty.marked(), "/* | */");
    assert!(empty.command(LineCommand::ToggleBlockComment));
    assert_eq!(empty.marked(), "|");
}

#[test]
fn 줄_주석이_없는_언어는_블록_주석으로_전환하고_구성이_없으면_무변경이다() {
    let mut css = Editor::new("css", "    color: red;").caret_at(8);
    assert!(css.command(LineCommand::ToggleLineComment));
    assert_eq!(css.text(), "    /* color: red; */");
    assert!(css.command(LineCommand::ToggleLineComment));
    assert_eq!(css.text(), "    color: red;");
    let mut plain = Editor::new("plaintext", "text");
    assert!(!plain.command(LineCommand::ToggleLineComment));
    assert_eq!(plain.text(), "text");
}

#[test]
fn 대소문자_명령은_구성된_단어와_여러_선택을_사용하고_undo로_복구한다() {
    let mut editor = typescript("fooBar next_word").caret_at(3);
    assert!(editor.command(LineCommand::Transform(TextCase::Snake)));
    assert_eq!(editor.text(), "foo_bar next_word");
    assert_eq!(editor.undoes().text(), "fooBar next_word");
    editor.select(&[(0, 6), (7, 16)]);
    assert!(editor.command(LineCommand::Transform(TextCase::Upper)));
    assert_eq!(editor.text(), "FOOBAR NEXT_WORD");
    assert_eq!(editor.selections(), [(0, 6), (7, 16)]);
    let mut unicode = typescript("İx").caret_at(2);
    assert!(unicode.command(LineCommand::Transform(TextCase::Lower)));
    assert_eq!(unicode.text(), "i\u{0307}x");
    assert_eq!(unicode.selections(), [(1, 1)]);
}

#[test]
fn 괄호_제거는_코드의_가장_가까운_짝을_지우고_문자열과_주석을_제외한다() {
    let mut editor = typescript("({text})").caret_at(3);
    assert!(editor.command(LineCommand::RemoveBrackets));
    assert_eq!(editor.marked(), "(t|ext)");
    let mut quoted = typescript("(\"}\" /* ) */ text)").caret_at(14);
    assert!(quoted.command(LineCommand::RemoveBrackets));
    assert_eq!(quoted.text(), "\"}\" /* ) */ text");
    assert_eq!(quoted.undoes().text(), "(\"}\" /* ) */ text)");
}

#[test]
fn 괄호_사이에서_enter_는_두_줄을_벌리고_안쪽을_들여쓴다() {
    let mut editor = typescript("if (ready) {}").caret_after("{");
    assert_eq!(editor.enters().marked(), "if (ready) {\n    |\n}");
    assert_eq!(editor.undoes().text(), "if (ready) {}");

    let mut nested = typescript("    call([])").caret_after("[");
    assert_eq!(nested.enters().marked(), "    call([\n        |\n    ])");

    let mut tabbed = typescript("\tif (ready) {}").caret_after("{");
    tabbed.indent = TABS;
    assert_eq!(tabbed.enters().marked(), "\tif (ready) {\n\t\t|\n\t}");
}

#[test]
fn 여는_괄호_뒤에서_enter_는_한_단계_들여쓴다() {
    let mut editor = typescript("  run(");
    assert_eq!(editor.enters().marked(), "  run(\n    |");
    let mut aligned = typescript("    run(");
    assert_eq!(aligned.enters().marked(), "    run(\n        |");
    let mut spaced = typescript("const items = [  ");
    assert_eq!(spaced.enters().marked(), "const items = [  \n    |");
    let mut closed = typescript("run()");
    assert_eq!(closed.enters().marked(), "run()\n|");
}

#[test]
fn 문자열과_주석_안의_괄호는_enter_들여쓰기를_만들지_않는다() {
    let mut string = typescript("  const text = \"{\"").caret_after("\"{");
    assert_eq!(string.enters().marked(), "  const text = \"{\n  |\"");
    let mut comment = typescript("  // open {");
    assert_eq!(comment.enters().marked(), "  // open {\n  |");
}

#[test]
fn 문서_주석은_enter_로_이어_쓴다() {
    let mut opened = typescript("/** */").caret_after("/**");
    assert_eq!(opened.enters().marked(), "/**\n * |\n */");

    let mut started = typescript("  /** summary");
    assert_eq!(started.enters().marked(), "  /** summary\n   * |");

    let mut continued = typescript("   * detail");
    assert_eq!(continued.enters().marked(), "   * detail\n   * |");

    let mut finished = typescript("   */");
    assert_eq!(finished.enters().marked(), "   */\n  |");
}

#[test]
fn 토큰을_싸게_얻지_못하는_줄은_선행_공백만_유지한다() {
    let mut editor = typescript("  if (ready) {}").caret_after("{");
    editor.syntax = &Expensive;
    assert_eq!(editor.enters().marked(), "  if (ready) {\n  |}");
}

#[test]
fn 문서의_줄_끝_문자를_그대로_쓴다() {
    let mut editor = typescript("a\r\nif (b) {}").caret_after("{");
    assert_eq!(editor.enters().text(), "a\r\nif (b) {\r\n    \r\n}");
}

#[test]
fn 일반_텍스트도_괄호_규칙을_쓴다() {
    let mut editor = Editor::new("plaintext", "list {}").caret_after("{");
    editor.syntax = &UntokenizedLines;
    assert_eq!(editor.enters().marked(), "list {\n    |\n}");
}

#[test]
fn 콜론으로_끝난_python_블록_머리는_다음_줄을_들여쓴다() {
    let mut editor = Editor::new("python", "def run(self):");
    assert_eq!(editor.enters().marked(), "def run(self):\n    |");
    let mut plain = Editor::new("python", "value = 1");
    assert_eq!(plain.enters().marked(), "value = 1\n|");
}

#[test]
fn ruby_는_들여쓰기_규칙으로_다음_줄과_end_를_맞춘다() {
    let mut opened = Editor::new("ruby", "def run");
    assert_eq!(opened.enters().marked(), "def run\n    |");

    let mut body = Editor::new("ruby", "def run\n    work");
    assert_eq!(body.enters().marked(), "def run\n    work\n    |");

    let mut closing = Editor::new("ruby", "def run\n    work\n    en");
    assert_eq!(closing.types("d").marked(), "def run\n    work\nend|");
    assert_eq!(closing.undoes().text(), "def run\n    work\n    en");

    let mut kept = Editor::new("ruby", "def run\n    work\nen");
    assert_eq!(kept.types("d").marked(), "def run\n    work\nend|");
}

#[test]
fn 공백뿐인_줄에_닫는_괄호를_치면_여는_줄의_들여쓰기로_맞춘다() {
    let mut editor = typescript("  if (ready) {\n      run();\n      ");
    assert_eq!(
        editor.types("}").marked(),
        "  if (ready) {\n      run();\n  }|"
    );

    let mut nested = typescript("call(\n    [\n        1,\n        ");
    assert_eq!(
        nested.types("]").marked(),
        "call(\n    [\n        1,\n    ]|"
    );

    let mut same_line = typescript("    run(1");
    assert_eq!(same_line.types(")").marked(), "    run(1)|");

    let mut unmatched = typescript("    ");
    assert_eq!(unmatched.types("}").marked(), "    }|");
}

#[test]
fn 여는_괄호와_따옴표는_닫는_짝을_함께_넣는다() {
    let mut editor = typescript("run");
    assert_eq!(editor.types("(").marked(), "run(|)");
    assert_eq!(editor.types("[").marked(), "run([|])");
    assert_eq!(editor.types("\"").marked(), "run([\"|\"])");

    let mut quoted = typescript("const text = ");
    assert_eq!(quoted.types("'").marked(), "const text = '|'");
    let mut template = typescript("const text = ");
    assert_eq!(template.types("`").marked(), "const text = `|`");
}

#[test]
fn 뒤_문자가_허용_목록에_없으면_짝을_넣지_않는다() {
    let mut before_word = typescript("value").caret_at(0);
    assert_eq!(before_word.types("(").marked(), "(|value");
    let mut before_closer = typescript("run()").caret_after("(");
    assert_eq!(before_closer.types("[").marked(), "run([|])");
    let mut before_semicolon = typescript("run;").caret_after("run");
    assert_eq!(before_semicolon.types("(").marked(), "run(|);");
    let mut before_opener = typescript("run(").caret_at(3);
    assert_eq!(before_opener.types("[").marked(), "run[|(");
}

#[test]
fn 단어_문자_바로_뒤의_따옴표는_짝을_넣지_않는다() {
    let mut editor = typescript("don");
    assert_eq!(editor.types("'").marked(), "don'|");
    let mut after_separator = typescript("run(");
    assert_eq!(after_separator.types("'").marked(), "run('|'");
}

#[test]
fn 문자열과_주석_안에서는_제외된_짝을_넣지_않는다() {
    let mut comment = typescript("// it ");
    assert_eq!(comment.types("'").marked(), "// it '|");
    let mut allowed = typescript("// say ");
    assert_eq!(allowed.types("\"").marked(), "// say \"|\"");
    let mut bracket = typescript("// call ");
    assert_eq!(bracket.types("(").marked(), "// call (|)");
    let mut unterminated = typescript("const text = \"a ");
    assert_eq!(unterminated.types("\"").marked(), "const text = \"a \"|");
    let mut json = Editor::new("json", "{ \"key\": \"a ").caret_after("\"a ");
    assert_eq!(json.types("[").marked(), "{ \"key\": \"a [|");
}

#[test]
fn 여러_글자_짝은_가장_긴_여는_문자열을_고른다() {
    let mut editor = typescript("");
    assert_eq!(editor.types("/**").marked(), "/**| */");
}

#[test]
fn 자동으로_넣은_닫는_문자는_같은_문자를_치면_덮어쓴다() {
    let mut editor = typescript("run");
    assert_eq!(editor.types("(").types(")").marked(), "run()|");
    assert_eq!(editor.types(")").marked(), "run())|");

    let mut quoted = typescript("x = ");
    assert_eq!(
        quoted.types("\"").types("a").types("\"").marked(),
        "x = \"a\"|"
    );

    let mut existing = typescript("run()").caret_after("(");
    assert_eq!(existing.types(")").marked(), "run()|)");
}

#[test]
fn 캐럿이_짝_밖으로_나가면_덮어쓰기와_짝_삭제를_그만둔다() {
    let mut editor = typescript("run");
    editor.types("(");
    editor.select(&[(0, 0)]);
    editor.types("a");
    editor.select(&[(5, 5)]);
    assert_eq!(editor.marked(), "arun(|)");
    assert_eq!(editor.types(")").marked(), "arun()|)");

    let mut deleted = typescript("run");
    deleted.types("(");
    deleted.moves_caret(0);
    assert!(deleted.auto_closed.is_empty());
    deleted.moves_caret(4);
    deleted.types("x").backspaces();
    assert_eq!(deleted.backspaces().marked(), "run|)");

    let mut kept = typescript("run");
    kept.types("(");
    kept.moves_caret(4);
    assert!(!kept.auto_closed.is_empty());
    kept.types("x").backspaces();
    assert_eq!(kept.backspaces().marked(), "run|");

    let mut broken = typescript("run");
    broken.types("{").enters();
    assert!(broken.auto_closed.is_empty());
}

#[test]
fn 자동으로_넣은_짝은_backspace_로_함께_지운다() {
    let mut editor = typescript("run");
    assert_eq!(editor.types("(").backspaces().marked(), "run|");
    assert_eq!(editor.undoes().text(), "run()");

    let mut existing = typescript("run()").caret_after("(");
    assert_eq!(existing.backspaces().marked(), "run|)");

    let mut quoted = typescript("x = ");
    assert_eq!(quoted.types("'").backspaces().marked(), "x = |");
}

#[test]
fn 선택한_글자는_여는_문자를_치면_짝으로_감싼다() {
    let mut editor = typescript("call value");
    editor.select(&[(5, 10)]);
    editor.types("(");
    assert_eq!(editor.text(), "call (value)");
    assert_eq!(editor.selections(), [(6, 11)]);
    editor.types("\"");
    assert_eq!(editor.text(), "call (\"value\")");
    assert_eq!(editor.selections(), [(7, 12)]);
    assert_eq!(editor.undoes().text(), "call (value)");
    assert_eq!(editor.undoes().text(), "call value");

    let mut backward = typescript("call value");
    backward.select(&[(10, 5)]);
    backward.types("[");
    assert_eq!(backward.text(), "call [value]");
    assert_eq!(backward.selections(), [(6, 11)]);
}

#[test]
fn 공백만_고르거나_따옴표_하나를_따옴표로_바꿀_때는_감싸지_않는다() {
    let mut blank = typescript("a  b");
    blank.select(&[(1, 3)]);
    assert_eq!(blank.types("(").marked(), "a(|b");

    let mut quote = typescript("x = 'a'");
    quote.select(&[(4, 5)]);
    assert_eq!(quote.types("\"").marked(), "x = \"|a'");

    let mut letter = typescript("value");
    letter.select(&[(0, 5)]);
    assert_eq!(letter.types("x").marked(), "x|");
}

#[test]
fn 여러_캐럿은_각자_짝을_넣고_덮어쓰고_지운다() {
    let mut editor = typescript("a\nb");
    editor.select(&[(1, 1), (3, 3)]);
    assert_eq!(editor.types("(").marked(), "a(|)\nb(|)");
    assert_eq!(editor.types("x").marked(), "a(x|)\nb(x|)");
    assert_eq!(editor.types(")").marked(), "a(x)|\nb(x)|");
    assert_eq!(editor.text(), "a(x)\nb(x)");

    let mut deleted = typescript("a\nb");
    deleted.select(&[(1, 1), (3, 3)]);
    assert_eq!(deleted.types("[").backspaces().marked(), "a|\nb|");

    let mut mixed = typescript("a\nbc");
    mixed.select(&[(1, 1), (3, 3)]);
    assert_eq!(mixed.types("(").marked(), "a(|\nb(|c");
}

#[test]
fn 짝을_넣는_입력은_앞선_입력과_다른_undo_단위다() {
    let mut editor = typescript("");
    editor
        .types("r")
        .types("u")
        .types("n")
        .types("(")
        .types("1");
    assert_eq!(editor.text(), "run(1)");
    assert_eq!(editor.undoes().text(), "run");
    assert_eq!(editor.undoes().text(), "");
}

#[test]
fn 조합이_끝나_들어온_한_글자도_짝_규칙을_따른다() {
    let mut editor = typescript("x = ");
    assert_eq!(editor.commits(4..4, "\"").marked(), "x = \"|\"");
    assert_eq!(editor.commits(5..5, "\"").marked(), "x = \"\"|");

    let mut selected = typescript("x = value");
    selected.select(&[(4, 9)]);
    assert_eq!(selected.commits(4..9, "'").text(), "x = 'value'");

    let mut syllable = typescript("x = ");
    assert_eq!(syllable.commits(4..4, "한").marked(), "x = 한|");
    let mut phrase = typescript("x = ");
    assert_eq!(phrase.commits(4..4, "(\"").marked(), "x = (\"|");
}

#[test]
fn 언어_구성이_없으면_입력을_그대로_넣는다() {
    let mut editor = typescript("run {}").caret_after("{");
    let mut auto_closed = AutoClosedPairs::default();
    let mut typing = Typing {
        language: None,
        indent: SPACES,
        auto_closed: &mut auto_closed,
    };
    insert_line_break(&mut editor.store, editor.view, &mut typing).unwrap();
    type_text(&mut editor.store, editor.view, "(", &mut typing).unwrap();
    assert_eq!(editor.marked(), "run {\n(|}");
    delete_backward(&mut editor.store, editor.view, &mut typing).unwrap();
    assert_eq!(editor.marked(), "run {\n|}");
}

fn regions(language_id: &str, text: &str) -> Vec<(usize, usize)> {
    let rules = monaco_language(language_id).unwrap().unwrap();
    language_regions(&text.into(), TAB_SIZE, MAX_FOLDING_REGIONS, rules)
        .iter()
        .map(|region| (region.start_line, region.end_line))
        .collect()
}

#[test]
fn 접기_표식은_들여쓰기와_무관하게_시작부터_끝_표식_줄까지_묶는다() {
    let text =
        "// #region setup\nconst a = 1\nfunction run() {\n    work()\n}\n// #endregion\nrest()";
    assert_eq!(regions("typescript", text), [(0, 5), (2, 3)]);
    assert_eq!(
        indent_regions(&text.into(), TAB_SIZE, MAX_FOLDING_REGIONS),
        [FoldRegion {
            start_line: 2,
            end_line: 3
        }]
    );
    let nested = "//#region a\n//#region b\nx\n//#endregion\n//#endregion";
    assert_eq!(regions("typescript", nested), [(0, 4), (1, 3)]);
    let unopened = "x\n// #endregion\ny";
    assert_eq!(regions("typescript", unopened), []);
    let unclosed = "// #region a\n    x\ny";
    assert_eq!(regions("typescript", unclosed), [(0, 1)]);
}

#[test]
fn off_side_언어는_빈_줄을_앞_블록에_붙인다() {
    let text = "def a():\n    x\n\n\ndef b():\n    y";
    assert_eq!(regions("python", text), [(0, 1), (4, 5)]);
    assert_eq!(regions("typescript", text), [(0, 3), (4, 5)]);
    assert_eq!(regions("yaml", "a:\n  b: 1\n\nc: 2"), [(0, 1)]);
    assert_eq!(regions("go", "a:\n  b: 1\n\nc: 2"), [(0, 2)]);
}

#[test]
fn 표식_접기_명령은_시작_표식_줄의_영역만_접고_편다() {
    let text =
        "// #region a\nx\n// #endregion\nfunction run() {\n    work()\n}\n/* note\n   more\n*/";
    let mut editor = typescript(text).caret_at(0);
    let rules = editor.rules;
    let found = language_regions(&text.into(), TAB_SIZE, MAX_FOLDING_REGIONS, rules);
    let folded_lines = |editor: &Editor| -> Vec<usize> {
        let snapshot = editor.snapshot();
        editor
            .store
            .views()
            .get(editor.view)
            .unwrap()
            .folds
            .iter()
            .map(|fold| snapshot.rope.byte_to_line(fold.start) - 1)
            .collect()
    };
    let run = |editor: &mut Editor, command: FoldCommand| {
        run_language_fold_command(&mut editor.store, editor.view, &found, command, rules).unwrap();
        folded_lines(editor)
    };
    assert_eq!(run(&mut editor, FoldCommand::FoldAllMarkerRegions), [0]);
    assert_eq!(run(&mut editor, FoldCommand::FoldAllBlockComments), [0, 6]);
    assert_eq!(run(&mut editor, FoldCommand::UnfoldAllMarkerRegions), [6]);
    assert_eq!(run(&mut editor, FoldCommand::FoldAll), [0, 3, 6]);
}

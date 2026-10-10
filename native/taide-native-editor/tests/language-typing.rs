use std::ops::Range;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::auto_closing::AutoClosedPairs;
use taide_native_editor::document::DocumentSnapshot;
use taide_native_editor::folding::{MAX_FOLDING_REGIONS, language_regions};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::{
    AutoClosingPair, BracketPair, CharacterPairs, EnterAction, FoldMarker, IndentAction,
    IndentMetadata, Language, LanguageRules, LineSyntax, UntokenizedLines, is_js_whitespace,
    token_kind_at, without_brackets_outside_code,
};
use taide_native_editor::language_typing::{Typing, insert_line_break, type_text};
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
const TABS: IndentOptions = IndentOptions {
    tab_size: TAB_SIZE,
    insert_spaces: false,
};
const AUTO_CLOSE_BEFORE: &str = ";:.,=}])> \n\t";
const BRACKETS: [char; 2] = ['{', '}'];
const UNINDENTED_PREFIX: char = '#';
const OUTDENT_KEYWORD: &str = "return";
const LIST_MARKER: &str = "- ";
const TRAILER: &str = "*/";
const REGION_START: &str = "#region";
const REGION_END: &str = "#endregion";

#[derive(Default)]
struct ScriptedRules {
    pairs: CharacterPairs,
    has_indentation_rules: bool,
    is_off_side: bool,
}

impl LanguageRules for ScriptedRules {
    fn pairs(&self) -> &CharacterPairs {
        &self.pairs
    }

    fn enter_action(&self, _: &str, before_enter: &str, _: &str) -> Option<EnterAction> {
        let action = |indent_action, append_text: Option<&str>, remove_text| EnterAction {
            indent_action,
            append_text: append_text.map(str::to_owned),
            remove_text,
        };
        let content = before_enter.trim_start();
        if content.starts_with(OUTDENT_KEYWORD) {
            Some(action(IndentAction::Outdent, None, None))
        } else if content.starts_with(LIST_MARKER) && content.ends_with(':') {
            Some(action(IndentAction::Indent, Some(LIST_MARKER), None))
        } else if content.ends_with(TRAILER) {
            Some(action(
                IndentAction::None,
                None,
                Some(TAB_SIZE as usize + 1),
            ))
        } else {
            None
        }
    }

    fn indent_metadata(&self, line: &str) -> Option<IndentMetadata> {
        let content = line.trim();
        self.has_indentation_rules.then(|| IndentMetadata {
            increases: content.ends_with('{'),
            decreases: content.starts_with('}'),
            indents_next_line: content.ends_with(')'),
            is_unindented: content.starts_with(UNINDENTED_PREFIX),
        })
    }

    fn without_brackets(&self, text: &str) -> String {
        text.replace(BRACKETS, "")
    }

    fn bracket_ranges(&self, line: &str) -> Vec<Range<usize>> {
        line.match_indices(BRACKETS)
            .map(|(start, bracket)| start..start + bracket.len())
            .collect()
    }

    fn last_bracket(&self, text: &str) -> Option<Range<usize>> {
        text.rmatch_indices(BRACKETS)
            .next()
            .map(|(start, bracket)| start..start + bracket.len())
    }

    fn is_off_side(&self) -> bool {
        self.is_off_side
    }

    fn fold_marker(&self, line: &str) -> Option<FoldMarker> {
        if line.starts_with(REGION_START) {
            Some(FoldMarker::Start)
        } else {
            line.starts_with(REGION_END).then_some(FoldMarker::End)
        }
    }

    fn starts_marker_region(&self, line: &str) -> bool {
        line.starts_with(REGION_START)
    }
}

fn pair(open: &str, close: &str) -> AutoClosingPair {
    AutoClosingPair {
        open: open.into(),
        close: close.into(),
        excluded_tokens: Vec::new(),
    }
}

fn rules_with_pairs(auto_closing_pairs: Vec<AutoClosingPair>) -> ScriptedRules {
    ScriptedRules {
        pairs: CharacterPairs {
            brackets: vec![BracketPair {
                open: "{".into(),
                close: "}".into(),
            }],
            auto_closing_pairs,
            auto_close_before_quotes: AUTO_CLOSE_BEFORE.into(),
            auto_close_before_brackets: AUTO_CLOSE_BEFORE.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn indentation_rules() -> ScriptedRules {
    ScriptedRules {
        has_indentation_rules: true,
        ..rules_with_pairs(Vec::new())
    }
}

struct ClassifiedLine {
    kind: TokenKind,
    bytes: Range<usize>,
}

struct AccurateLine(ClassifiedLine);

impl LineSyntax for AccurateLine {
    fn tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        UntokenizedLines.tokens(document, line)
    }

    fn accurate_tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        self.0.tokens(document, line)
    }

    fn kind_if_inserting(
        &self,
        document: &DocumentSnapshot,
        line: usize,
        byte: usize,
        character: char,
    ) -> TokenKind {
        self.0.kind_if_inserting(document, line, byte, character)
    }
}

impl LineSyntax for ClassifiedLine {
    fn tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        if line != 1 {
            return UntokenizedLines.tokens(document, line);
        }
        Some(vec![
            Token {
                start_byte: 0,
                kind: TokenKind::Other,
            },
            Token {
                start_byte: self.bytes.start,
                kind: self.kind,
            },
            Token {
                start_byte: self.bytes.end,
                kind: TokenKind::Other,
            },
        ])
    }

    fn kind_if_inserting(
        &self,
        document: &DocumentSnapshot,
        line: usize,
        byte_in_line: usize,
        _: char,
    ) -> TokenKind {
        self.tokens(document, line)
            .map_or(TokenKind::Other, |tokens| {
                token_kind_at(&tokens, byte_in_line)
            })
    }
}

fn fixture(text: &str, caret: usize) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/language-typing.txt".into(),
            OpenedFile {
                path: "/synthetic/language-typing.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
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
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: caret,
                    head: caret,
                }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    (store, view)
}

fn marked(store: &EditorStore, view: ViewId) -> String {
    let current = store.views().get(view).unwrap();
    let mut text = store
        .documents()
        .snapshot(current.document)
        .unwrap()
        .rope
        .to_string();
    text.insert(current.selection.selections[0].head, '|');
    text
}

#[test]
fn 재들여쓰기는_다중_선택과_mirror를_추적하고_한_번의_undo로_복원한다() {
    use taide_native_editor::indent::{Command, IndentConfiguration, run_command_with_language};
    let source = "if {\nbody\n}\n끝";
    let expected = "if {\n    body\n}\n끝";
    let (mut store, view) = fixture(source, source.len());
    let document = store.views().get(view).unwrap().document;
    let selected = SelectionSet {
        primary: 1,
        selections: vec![
            Selection {
                anchor: source.find("body").unwrap(),
                head: source.find("body").unwrap(),
            },
            Selection {
                anchor: source.len(),
                head: source.find('}').unwrap(),
            },
        ],
    };
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(view, selected.clone(), current.scroll, current.folds)
        .unwrap();
    let mirror = store
        .attach_view(
            ViewKey {
                window: "mirror".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let mirrored = SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: source.len(),
            head: source.len(),
        }],
    };
    let current = store.views().get(mirror).unwrap().clone();
    store
        .set_view_state(mirror, mirrored.clone(), current.scroll, current.folds)
        .unwrap();
    let rules = indentation_rules();
    let configuration = IndentConfiguration {
        defaults: SPACES,
        detect_indentation: false,
    };
    let language = Some(Language {
        rules: &rules,
        syntax: &UntokenizedLines,
    });
    assert!(
        run_command_with_language(
            &mut store,
            view,
            Command::ReindentLines,
            configuration,
            language
        )
        .unwrap()
    );
    let snapshot = store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.rope.to_string(), expected);
    assert_eq!(snapshot.indent_options, Some(SPACES));
    let converted = store.views().get(view).unwrap().selection.clone();
    assert_eq!(converted.primary, selected.primary);
    assert_eq!(converted.selections.len(), selected.selections.len());
    assert_eq!(converted.selections[0].head, expected.find("body").unwrap());
    assert_eq!(converted.selections[1].anchor, expected.len());
    assert_eq!(
        store.views().get(mirror).unwrap().selection.selections[0].head,
        expected.len()
    );
    assert!(
        !run_command_with_language(
            &mut store,
            view,
            Command::ReindentLines,
            configuration,
            language
        )
        .unwrap()
    );
    assert_eq!(
        store.documents().snapshot(document).unwrap().revision,
        snapshot.revision
    );
    assert!(store.undo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        source
    );
    assert_eq!(store.views().get(view).unwrap().selection, selected);
    assert_eq!(store.views().get(mirror).unwrap().selection, mirrored);
    assert!(!store.undo(document).unwrap());
    assert!(store.redo(document).unwrap());
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    assert_eq!(store.views().get(view).unwrap().selection, converted);
}

#[test]
fn 재들여쓰기의_readonly와_용량_거절은_내용과_선택과_문서_옵션을_보존한다() {
    use taide_native_editor::document::EditorError;
    use taide_native_editor::indent::{Command, IndentConfiguration, run_command_with_language};
    let source = "if {\nbody\n}";
    for readonly in [false, true] {
        let (mut store, view) = fixture(source, source.len());
        let document = store.views().get(view).unwrap().document;
        let configuration = IndentConfiguration {
            defaults: IndentOptions {
                tab_size: u32::MAX,
                insert_spaces: true,
            },
            detect_indentation: false,
        };
        store
            .configure_indentation(document, configuration)
            .unwrap();
        if readonly {
            store
                .observe_file(
                    document,
                    std::path::Path::new("/synthetic/language-typing.txt"),
                    OpenedFile {
                        path: "/synthetic/language-typing.txt".into(),
                        content: source.into(),
                        language_id: "plaintext".into(),
                        byte_size: source.len().try_into().unwrap(),
                        line_count: source.lines().count().try_into().unwrap(),
                        tier: FileSizeTier::Normal,
                        read_only: true,
                        encoding_lossy: false,
                        modified_ms: 1.0,
                        editor_config: EditorConfigOptions::default(),
                    },
                )
                .unwrap();
        }
        let before = store.documents().snapshot(document).unwrap();
        let selected = store.views().get(view).unwrap().selection.clone();
        let rules = indentation_rules();
        for command in [Command::ReindentLines, Command::ReindentSelectedLines] {
            if command == Command::ReindentSelectedLines {
                let current = store.views().get(view).unwrap().clone();
                store
                    .set_view_state(
                        view,
                        SelectionSet {
                            primary: 0,
                            selections: vec![Selection {
                                anchor: 0,
                                head: source.len(),
                            }],
                        },
                        current.scroll,
                        current.folds,
                    )
                    .unwrap();
            }
            let expected = if readonly {
                EditorError::ReadOnly
            } else {
                EditorError::Capacity
            };
            let requested = store.views().get(view).unwrap().selection.clone();
            assert_eq!(
                run_command_with_language(
                    &mut store,
                    view,
                    command,
                    configuration,
                    Some(Language {
                        rules: &rules,
                        syntax: &UntokenizedLines
                    })
                ),
                Err(expected)
            );
            let after = store.documents().snapshot(document).unwrap();
            assert_eq!(after.rope, before.rope);
            assert_eq!(after.revision, before.revision);
            assert_eq!(after.dirty, before.dirty);
            assert_eq!(after.indent_options, before.indent_options);
            assert_eq!(store.views().get(view).unwrap().selection, requested);
        }
        let current = store.views().get(view).unwrap().clone();
        store
            .set_view_state(view, selected.clone(), current.scroll, current.folds)
            .unwrap();
        if !readonly {
            assert!(!store.undo(document).unwrap());
        }
        assert_eq!(store.views().get(view).unwrap().selection, selected);
    }
}

#[test]
fn 재들여쓰기는_정확한_토큰의_문자열_줄과_주석_정규식_안의_괄호를_보존한다() {
    use taide_native_editor::indent::{Command, IndentConfiguration, run_command_with_language};
    let source = "if {\n value {\nbody\n}";
    let rules = indentation_rules();
    for kind in [TokenKind::String, TokenKind::Comment, TokenKind::Regex] {
        let (mut store, view) = fixture(source, source.len());
        let syntax = AccurateLine(ClassifiedLine {
            kind,
            bytes: 0.." value {".len(),
        });
        assert!(
            run_command_with_language(
                &mut store,
                view,
                Command::ReindentLines,
                IndentConfiguration {
                    defaults: SPACES,
                    detect_indentation: false
                },
                Some(Language {
                    rules: &rules,
                    syntax: &syntax
                }),
            )
            .unwrap()
        );
        let document = store.views().get(view).unwrap().document;
        let expected = if kind == TokenKind::String {
            "if {\n value {\n    body\n}"
        } else {
            "if {\n    value {\n    body\n}"
        };
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            expected,
            "{kind:?}"
        );
    }
}

#[test]
fn 재들여쓰기는_무시하는_줄과_다음_줄의_임시_들여쓰기를_구분한다() {
    use taide_native_editor::indent::{Command, IndentConfiguration, run_command_with_language};
    let rules = indentation_rules();
    for (source, expected) in [
        (
            "#header\nif {\n#inside\nbody\n}\nend",
            "#header\nif {\n    #inside\n    body\n}\nend",
        ),
        ("if (x)\nbody\nend", "if (x)\n    body\nend"),
        (
            "if (x)\nif (y)\nbody\nend",
            "if (x)\n    if (y)\n        body\nend",
        ),
        ("#first\n#second", "#first\n#second"),
    ] {
        let (mut store, view) = fixture(source, source.len());
        let changed = run_command_with_language(
            &mut store,
            view,
            Command::ReindentLines,
            IndentConfiguration {
                defaults: SPACES,
                detect_indentation: false,
            },
            Some(Language {
                rules: &rules,
                syntax: &UntokenizedLines,
            }),
        )
        .unwrap();
        let document = store.views().get(view).unwrap().document;
        assert_eq!(changed, source != expected);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            expected
        );
    }
}

#[test]
fn 재들여쓰기_용량은_표시_폭이_아닌_저장할_탭의_바이트로_검사한다() {
    use taide_native_editor::indent::{Command, IndentConfiguration, run_command_with_language};
    let source = "if {\nbody\n}";
    let (mut store, view) = fixture(source, source.len());
    let rules = indentation_rules();
    let result = run_command_with_language(
        &mut store,
        view,
        Command::ReindentLines,
        IndentConfiguration {
            defaults: IndentOptions {
                tab_size: u32::MAX,
                insert_spaces: false,
            },
            detect_indentation: false,
        },
        Some(Language {
            rules: &rules,
            syntax: &UntokenizedLines,
        }),
    );
    assert_eq!(result, Ok(true));
    let document = store.views().get(view).unwrap().document;
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "if {\n\tbody\n}"
    );
}

fn entered(rules: &ScriptedRules, indent: IndentOptions, text: &str, caret: usize) -> String {
    let (mut store, view) = fixture(text, caret);
    let mut auto_closed = AutoClosedPairs::default();
    let mut typing = Typing {
        language: Some(Language {
            rules,
            syntax: &UntokenizedLines,
        }),
        indent,
        auto_closed: &mut auto_closed,
    };
    insert_line_break(&mut store, view, &mut typing).unwrap();
    marked(&store, view)
}

fn typed(rules: &ScriptedRules, text: &str, caret: usize, input: &[&str]) -> String {
    typed_with_syntax(rules, &UntokenizedLines, text, caret, input)
}

fn typed_with_syntax(
    rules: &ScriptedRules,
    syntax: &dyn LineSyntax,
    text: &str,
    caret: usize,
    input: &[&str],
) -> String {
    let (mut store, view) = fixture(text, caret);
    let mut auto_closed = AutoClosedPairs::default();
    let mut typing = Typing {
        language: Some(Language { rules, syntax }),
        indent: SPACES,
        auto_closed: &mut auto_closed,
    };
    for character in input {
        type_text(&mut store, view, character, &mut typing).unwrap();
    }
    marked(&store, view)
}

fn assert_ignored_brackets(kind: TokenKind, lines: &[&str]) {
    let rules = rules_with_pairs(Vec::new());
    for line in lines {
        let text = format!("  {{\n{line}\n        ");
        let syntax = ClassifiedLine {
            kind,
            bytes: line.len() - line.trim_start().len()..line.len(),
        };
        assert_eq!(
            typed_with_syntax(&rules, &syntax, &text, text.len(), &["}"]),
            format!("  {{\n{line}\n  }}|"),
            "{kind:?}: {line:?}",
        );
    }
}

#[test]
fn 닫는_괄호_내어쓰기는_이전_줄의_문자열_괄호를_무시한다() {
    assert_ignored_brackets(TokenKind::String, &["      \"{\"", "      \"}\""]);
}

#[test]
fn 닫는_괄호_내어쓰기는_이전_줄의_주석_괄호를_무시한다() {
    assert_ignored_brackets(
        TokenKind::Comment,
        &["      // {", "      // }", "      /* { */", "      /* } */"],
    );
}

#[test]
fn 닫는_괄호_내어쓰기는_이전_줄의_정규식_괄호를_무시한다() {
    assert_ignored_brackets(TokenKind::Regex, &["      /\\{/", "      /\\}/"]);
}

#[test]
fn 닫는_괄호_내어쓰기는_other_토큰의_괄호를_센다() {
    let rules = rules_with_pairs(Vec::new());
    for (line, indent) in [("      {", "      "), ("      }", "        ")] {
        let text = format!("  {{\n{line}\n        ");
        let syntax = ClassifiedLine {
            kind: TokenKind::Other,
            bytes: line.len() - line.trim_start().len()..line.len(),
        };
        assert_eq!(
            typed_with_syntax(&rules, &syntax, &text, text.len(), &["}"]),
            format!("  {{\n{line}\n{indent}}}|"),
        );
    }
}

fn at_end(rules: &ScriptedRules, text: &str) -> String {
    entered(rules, SPACES, text, text.len())
}

#[test]
fn enter_동작은_내어쓰기와_덧붙일_글자와_지울_들여쓰기를_적용한다() {
    let rules = rules_with_pairs(Vec::new());
    assert_eq!(
        at_end(&rules, "        return x"),
        "        return x\n    |"
    );
    assert_eq!(at_end(&rules, "return x"), "return x\n|");
    assert_eq!(at_end(&rules, "  - key:"), "  - key:\n    - |");
    assert_eq!(entered(&rules, TABS, "\t- key:", 7), "\t- key:\n\t\t- |");
    assert_eq!(at_end(&rules, "      */"), "      */\n |");
    assert_eq!(at_end(&rules, "  */"), "  */\n|");
    assert_eq!(at_end(&rules, "  plain"), "  plain\n  |");
}

#[test]
fn 들여쓰기_규칙은_위_줄에서_다음_줄의_들여쓰기를_물려받는다() {
    let rules = indentation_rules();
    assert_eq!(at_end(&rules, "  a {"), "  a {\n    |");
    assert_eq!(at_end(&rules, "if (a)"), "if (a)\n    |");
    assert_eq!(at_end(&rules, "if (a)\n    work;"), "if (a)\n    work;\n|");
    assert_eq!(at_end(&rules, "  a {\n#x"), "  a {\n#x\n    |");
    assert_eq!(at_end(&rules, "a {\n    b;\n  }"), "a {\n    b;\n  }\n  |");
    assert_eq!(at_end(&rules, "\n\n"), "\n\n\n|");
    assert_eq!(at_end(&rules, "    \n  x"), "    \n  x\n  |");
}

#[test]
fn 들여쓰기_규칙은_캐럿_뒤_글자가_내어쓰기_줄이면_한_단계_줄인다() {
    let rules = indentation_rules();
    assert_eq!(entered(&rules, SPACES, "a {}", 3), "a {\n|}");
    assert_eq!(entered(&rules, SPACES, "  a {  }", 5), "  a {\n|  }");
    assert_eq!(entered(&rules, SPACES, "    a {}", 7), "    a {\n    |}");
}

#[test]
fn 선행_공백_안에서_enter_하면_캐럿의_열을_유지한다() {
    let rules = indentation_rules();
    assert_eq!(entered(&rules, SPACES, "a {\n    b", 6), "a {\n  \n  |  b");
    assert_eq!(
        entered(&rules, SPACES, "a {\n    b", 8),
        "a {\n    \n    |b"
    );
}

#[test]
fn 내어쓰기_줄이_되는_글자를_치면_물려받은_들여쓰기로_맞춘다() {
    let rules = indentation_rules();
    assert_eq!(
        typed(&rules, "a {\n    b;\n    ", 15, &["}"]),
        "a {\n    b;\n}|"
    );
    assert_eq!(
        typed(&rules, "a {\n    b;\n", 11, &["}"]),
        "a {\n    b;\n}|"
    );
    assert_eq!(
        typed(&rules, "a {\n    b;\n    }", 16, &["x"]),
        "a {\n    b;\n    }x|"
    );
}

#[test]
fn 포함된_짝이_이미_닫혀_있으면_남은_닫는_글자만_넣는다() {
    let rules = rules_with_pairs(vec![pair("(", ")"), pair("(*", "*)")]);
    assert_eq!(typed(&rules, "", 0, &["("]), "(|)");
    assert_eq!(typed(&rules, "", 0, &["(", "*"]), "(*|*)");
    assert_eq!(typed(&rules, "(", 1, &["*"]), "(*|*)");
}

#[test]
fn 짝의_중립_문자는_여는_글자와_닫는_글자에_없는_첫_숫자나_영문자다() {
    assert_eq!(pair("(", ")").neutral_character(), Some('0'));
    assert_eq!(pair("0a", "1").neutral_character(), Some('2'));
    assert_eq!(
        pair("0123456789", "abcdefghijklmnopqrstuvwxyz").neutral_character(),
        Some('A')
    );
}

#[test]
fn 문자열과_주석_토큰의_괄호만_지우고_토큰_종류를_위치로_찾는다() {
    let rules = rules_with_pairs(Vec::new());
    let text = "a{ \"{\" }// }";
    let tokens = [
        Token {
            start_byte: 0,
            kind: TokenKind::Other,
        },
        Token {
            start_byte: 3,
            kind: TokenKind::String,
        },
        Token {
            start_byte: 6,
            kind: TokenKind::Other,
        },
        Token {
            start_byte: 8,
            kind: TokenKind::Comment,
        },
    ];
    assert_eq!(
        without_brackets_outside_code(&rules, text, &tokens, 0..text.len()),
        "a{ \"\" }// "
    );
    assert_eq!(
        without_brackets_outside_code(&rules, text, &tokens, 4..7),
        "\" "
    );
    assert_eq!(without_brackets_outside_code(&rules, text, &[], 0..2), "a{");
    assert_eq!(token_kind_at(&tokens, 0), TokenKind::Other);
    assert_eq!(token_kind_at(&tokens, 5), TokenKind::String);
    assert_eq!(token_kind_at(&tokens, 6), TokenKind::Other);
    assert_eq!(token_kind_at(&tokens, text.len()), TokenKind::Comment);
    assert_eq!(token_kind_at(&[], 0), TokenKind::Other);
}

#[test]
fn js_공백은_정규식의_공백_목록과_같다() {
    for whitespace in [
        ' ', '\t', '\u{B}', '\u{A0}', '\u{2028}', '\u{3000}', '\u{FEFF}',
    ] {
        assert!(is_js_whitespace(whitespace), "{whitespace:?}");
    }
    for other in ['a', '\u{85}', '\u{200B}', '\u{180E}'] {
        assert!(!is_js_whitespace(other), "{other:?}");
    }
}

#[test]
fn 접기_영역은_표식과_off_side_규칙을_따른다() {
    let regions = |rules: &ScriptedRules, text: &str| -> Vec<(usize, usize)> {
        language_regions(&text.into(), TAB_SIZE, MAX_FOLDING_REGIONS, rules)
            .iter()
            .map(|region| (region.start_line, region.end_line))
            .collect()
    };
    let rules = rules_with_pairs(Vec::new());
    assert_eq!(
        regions(&rules, "#region\na\n    b\n#endregion\nc"),
        [(0, 3), (1, 2)]
    );
    assert_eq!(regions(&rules, "a\n    b\n\nc"), [(0, 2)]);
    let off_side = ScriptedRules {
        is_off_side: true,
        ..rules_with_pairs(Vec::new())
    };
    assert_eq!(regions(&off_side, "a\n    b\n\nc"), [(0, 1)]);
}

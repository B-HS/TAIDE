use taide_model::{
    error::AppErrorKind,
    settings::{
        EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, Settings,
        TerminalCursorStyle,
    },
};
use taide_native_ui::{
    settings_code_controls::{
        Change as CodeChange, Font, Selection, Switch, Text, TextDraft, parse_rulers,
    },
    settings_controls::{Change, Numeric, NumericDraft},
};

const RULER_CAP: usize = 16;

#[test]
fn 편집기와_터미널_변경은_원본_범위_타입_단일_patch와_ack_초안을_보존한다() {
    let settings = Settings::default();
    for field in Switch::ALL {
        for value in [false, true] {
            let patch = Change::Code(CodeChange::Switch(field, value))
                .patch()
                .unwrap();
            let values = [
                patch.format_on_save,
                patch.organize_imports_on_save,
                patch.fix_all_on_save,
                patch.trim_trailing_whitespace_on_save,
                patch.insert_final_newline_on_save,
                patch.editor_config_enabled,
                patch.editor_code_lens_enabled,
                patch.editor_word_wrap,
                patch.editor_line_numbers,
                patch.editor_insert_spaces,
                patch.editor_detect_indentation,
                patch.editor_bracket_pair_colorization,
                patch.editor_bracket_pair_guides,
                patch.editor_font_ligatures,
                patch.editor_cursor_smooth_caret_animation,
                patch.editor_scroll_beyond_last_line,
                patch.editor_smooth_scrolling,
                patch.editor_sticky_scroll_enabled,
                patch.editor_semantic_highlighting,
                patch.editor_format_on_type,
                patch.editor_format_on_paste,
                patch.editor_suggest_preview,
                patch.emmet_enabled,
                patch.editor_diff_hide_unchanged_regions,
                patch.editor_diff_show_moves,
                patch.terminal_cursor_blink,
            ];
            assert_eq!(values.iter().filter(|value| value.is_some()).count(), 1);
            assert_eq!(
                values[Switch::ALL
                    .iter()
                    .position(|candidate| *candidate == field)
                    .unwrap()],
                Some(value)
            );
            let stored = match field {
                Switch::FormatOnSave => settings.format_on_save,
                Switch::OrganizeImportsOnSave => settings.organize_imports_on_save,
                Switch::FixAllOnSave => settings.fix_all_on_save,
                Switch::TrimTrailingWhitespaceOnSave => settings.trim_trailing_whitespace_on_save,
                Switch::InsertFinalNewlineOnSave => settings.insert_final_newline_on_save,
                Switch::EditorConfigEnabled => settings.editor_config_enabled,
                Switch::EditorCodeLens => settings.editor_code_lens_enabled,
                Switch::EditorWordWrap => settings.editor_word_wrap,
                Switch::EditorLineNumbers => settings.editor_line_numbers,
                Switch::EditorInsertSpaces => settings.editor_insert_spaces,
                Switch::EditorDetectIndentation => settings.editor_detect_indentation,
                Switch::EditorBracketPairColorization => settings.editor_bracket_pair_colorization,
                Switch::EditorBracketPairGuides => settings.editor_bracket_pair_guides,
                Switch::EditorFontLigatures => settings.editor_font_ligatures,
                Switch::EditorCursorSmoothCaretAnimation => {
                    settings.editor_cursor_smooth_caret_animation
                }
                Switch::EditorScrollBeyondLastLine => settings.editor_scroll_beyond_last_line,
                Switch::EditorSmoothScrolling => settings.editor_smooth_scrolling,
                Switch::EditorStickyScroll => settings.editor_sticky_scroll_enabled,
                Switch::EditorSemanticHighlighting => settings.editor_semantic_highlighting,
                Switch::EditorFormatOnType => settings.editor_format_on_type,
                Switch::EditorFormatOnPaste => settings.editor_format_on_paste,
                Switch::EditorSuggestPreview => settings.editor_suggest_preview,
                Switch::EmmetEnabled => settings.emmet_enabled,
                Switch::EditorDiffHideUnchangedRegions => {
                    settings.editor_diff_hide_unchanged_regions
                }
                Switch::EditorDiffShowMoves => settings.editor_diff_show_moves,
                Switch::TerminalCursorBlink => settings.terminal_cursor_blink,
            };
            assert_eq!(field.value(&settings), stored);
        }
    }
    let fields = [
        (Numeric::EditorFontSize, 6, 48, settings.editor_font_size),
        (
            Numeric::TerminalFontSize,
            6,
            48,
            settings.terminal_font_size,
        ),
        (
            Numeric::AutoSaveDelay,
            0,
            60_000,
            settings.auto_save_delay_ms,
        ),
        (Numeric::EditorTabSize, 1, 8, settings.editor_tab_size),
        (
            Numeric::TerminalScrollback,
            100,
            100_000,
            settings.terminal_scrollback,
        ),
    ];
    for (field, min, max, stored) in fields {
        assert_eq!(field.bounds(), (min, max));
        assert_eq!(field.value(&settings), stored);
        let mut draft = NumericDraft::new(stored);
        draft.text = (max + 1).to_string();
        let change = draft.commit(field, stored).unwrap().unwrap();
        assert_eq!(change, Change::Numeric(field, max));
        assert_eq!(draft.text, stored.to_string());
        draft.sync(stored);
        assert_eq!(draft.text, stored.to_string());
        draft.sync(max);
        assert_eq!(draft.text, max.to_string());
        let patch = change.patch().unwrap();
        let values = [
            patch.editor_font_size,
            patch.terminal_font_size,
            patch.auto_save_delay_ms,
            patch.editor_tab_size,
            patch.terminal_scrollback,
        ];
        assert_eq!(values.iter().filter(|value| value.is_some()).count(), 1);
        assert_eq!(values.into_iter().flatten().next(), Some(max));
        let lower = field.commit(-1.0, max).unwrap().unwrap();
        assert_eq!(lower, Change::Numeric(field, min));
        assert!(field.commit(f64::NAN, stored).unwrap().is_none());
        assert!(field.commit(f64::from(stored), stored).unwrap().is_none());
        let fraction = f64::from(min) + 0.5;
        assert_eq!(
            field.commit(fraction, stored).unwrap_err().kind(),
            AppErrorKind::InvalidArgument
        );
        let mut invalid = NumericDraft::new(stored);
        invalid.text = "0x10".into();
        assert!(invalid.commit(field, stored).unwrap().is_none());
        assert_eq!(invalid.text, stored.to_string());
    }

    let whitespace = Selection::Whitespace(EditorRenderWhitespace::All);
    assert_eq!(whitespace.choices(), &Selection::WHITESPACE);
    assert_eq!(whitespace.option_label(), "settings.renderWhitespaceAll");
    assert_eq!(
        CodeChange::Selection(whitespace)
            .patch()
            .editor_render_whitespace,
        Some(EditorRenderWhitespace::All)
    );
    let cursor = Selection::EditorCursor(EditorCursorStyle::Underline);
    assert_eq!(cursor.choices(), &Selection::EDITOR_CURSOR);
    assert_eq!(
        CodeChange::Selection(cursor).patch().editor_cursor_style,
        Some(EditorCursorStyle::Underline)
    );
    let blinking = Selection::EditorBlinking(EditorCursorBlinking::Solid);
    assert_eq!(blinking.choices(), &Selection::EDITOR_BLINKING);
    assert_eq!(
        CodeChange::Selection(blinking)
            .patch()
            .editor_cursor_blinking,
        Some(EditorCursorBlinking::Solid)
    );
    let terminal = Selection::TerminalCursor(TerminalCursorStyle::Bar);
    assert_eq!(terminal.choices(), &Selection::TERMINAL_CURSOR);
    assert_eq!(terminal.option_label(), "settings.cursorStyleBar");
    assert_eq!(
        CodeChange::Selection(terminal)
            .patch()
            .terminal_cursor_style,
        Some(TerminalCursorStyle::Bar)
    );

    assert_eq!(
        parse_rulers(
            "80, 0x50, 0b1010000, 0o120, 8e1, +80., 1000, 1001, 0, -1, 1.5, x, Infinity, NaN, 1_0"
        ),
        [80, 1000]
    );
    assert_eq!(
        parse_rulers("\u{feff}80\u{00a0}, \u{0085}90\u{0085}, +0x50, 0x, , "),
        [80]
    );
    let many = (1..=1000)
        .rev()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        parse_rulers(&many),
        (1..=RULER_CAP as u32).collect::<Vec<_>>()
    );
    let mut rulers = TextDraft::new("80".into());
    rulers.text = "80, 80, 2000, no".into();
    assert_eq!(rulers.commit(Text::Rulers), CodeChange::Rulers(vec![80]));
    assert_eq!(rulers.text, "80");
    rulers.text = "120, 40".into();
    let changed = rulers.commit(Text::Rulers);
    assert_eq!(changed.patch().editor_rulers, Some(vec![40, 120]));
    rulers.sync("80".into());
    assert_eq!(rulers.text, "40, 120");
    rulers.sync("40, 120".into());
    assert_eq!(rulers.text, "40, 120");
    let mut shell = TextDraft::new("/bin/zsh".into());
    shell.text = "  /synthetic/shell\u{feff}".into();
    assert_eq!(
        shell.commit(Text::Shell).patch().shell_override.as_deref(),
        Some("/synthetic/shell")
    );
    assert_eq!(shell.text, "  /synthetic/shell\u{feff}");
    shell.sync("/bin/zsh".into());
    assert_eq!(shell.text, "  /synthetic/shell\u{feff}");
    shell.sync("/synthetic/shell".into());
    assert_eq!(shell.text, "/synthetic/shell");
    assert_eq!(Text::Rulers.value(&settings), "");
    assert_eq!(Text::Shell.value(&settings), "");
    assert_eq!(
        CodeChange::Font(Font::Editor, None)
            .patch()
            .editor_font_family
            .as_deref(),
        Some("")
    );
    assert_eq!(
        CodeChange::Font(Font::Terminal, Some("Synthetic mono".into()))
            .patch()
            .terminal_font_family
            .as_deref(),
        Some("Synthetic mono")
    );
    assert_eq!(Font::Editor.value(&settings), None);
    assert_eq!(Font::Terminal.value(&settings), None);
}

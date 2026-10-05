use std::collections::BTreeSet;

use taide_model::settings::{
    EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, Settings, SettingsPatch,
    TerminalCursorStyle,
};

const RULER_MIN: u32 = 1;
const RULER_MAX: u32 = 1000;
const RULER_COUNT: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    FormatOnSave,
    OrganizeImportsOnSave,
    FixAllOnSave,
    TrimTrailingWhitespaceOnSave,
    InsertFinalNewlineOnSave,
    EditorConfigEnabled,
    EditorCodeLens,
    EditorWordWrap,
    EditorLineNumbers,
    EditorInsertSpaces,
    EditorDetectIndentation,
    EditorBracketPairColorization,
    EditorBracketPairGuides,
    EditorFontLigatures,
    EditorCursorSmoothCaretAnimation,
    EditorScrollBeyondLastLine,
    EditorSmoothScrolling,
    EditorStickyScroll,
    EditorSemanticHighlighting,
    EditorFormatOnType,
    EditorFormatOnPaste,
    EditorSuggestPreview,
    EmmetEnabled,
    EditorDiffHideUnchangedRegions,
    EditorDiffShowMoves,
    TerminalCursorBlink,
}

impl Switch {
    pub const ALL: [Self; 26] = [
        Self::FormatOnSave,
        Self::OrganizeImportsOnSave,
        Self::FixAllOnSave,
        Self::TrimTrailingWhitespaceOnSave,
        Self::InsertFinalNewlineOnSave,
        Self::EditorConfigEnabled,
        Self::EditorCodeLens,
        Self::EditorWordWrap,
        Self::EditorLineNumbers,
        Self::EditorInsertSpaces,
        Self::EditorDetectIndentation,
        Self::EditorBracketPairColorization,
        Self::EditorBracketPairGuides,
        Self::EditorFontLigatures,
        Self::EditorCursorSmoothCaretAnimation,
        Self::EditorScrollBeyondLastLine,
        Self::EditorSmoothScrolling,
        Self::EditorStickyScroll,
        Self::EditorSemanticHighlighting,
        Self::EditorFormatOnType,
        Self::EditorFormatOnPaste,
        Self::EditorSuggestPreview,
        Self::EmmetEnabled,
        Self::EditorDiffHideUnchangedRegions,
        Self::EditorDiffShowMoves,
        Self::TerminalCursorBlink,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::FormatOnSave => "settings.formatOnSave",
            Self::OrganizeImportsOnSave => "settings.organizeImportsOnSave",
            Self::FixAllOnSave => "settings.fixAllOnSave",
            Self::TrimTrailingWhitespaceOnSave => "settings.trimTrailingWhitespaceOnSave",
            Self::InsertFinalNewlineOnSave => "settings.insertFinalNewlineOnSave",
            Self::EditorConfigEnabled => "settings.editorConfigEnabled",
            Self::EditorCodeLens => "settings.editorCodeLens",
            Self::EditorWordWrap => "settings.editorWordWrap",
            Self::EditorLineNumbers => "settings.editorLineNumbers",
            Self::EditorInsertSpaces => "settings.editorInsertSpaces",
            Self::EditorDetectIndentation => "settings.editorDetectIndentation",
            Self::EditorBracketPairColorization => "settings.editorBracketPairColorization",
            Self::EditorBracketPairGuides => "settings.editorBracketPairGuides",
            Self::EditorFontLigatures => "settings.editorFontLigatures",
            Self::EditorCursorSmoothCaretAnimation => "settings.editorCursorSmoothCaretAnimation",
            Self::EditorScrollBeyondLastLine => "settings.editorScrollBeyondLastLine",
            Self::EditorSmoothScrolling => "settings.editorSmoothScrolling",
            Self::EditorStickyScroll => "settings.editorStickyScroll",
            Self::EditorSemanticHighlighting => "settings.editorSemanticHighlighting",
            Self::EditorFormatOnType => "settings.editorFormatOnType",
            Self::EditorFormatOnPaste => "settings.editorFormatOnPaste",
            Self::EditorSuggestPreview => "settings.editorSuggestPreview",
            Self::EmmetEnabled => "settings.emmetEnabled",
            Self::EditorDiffHideUnchangedRegions => "settings.editorDiffHideUnchangedRegions",
            Self::EditorDiffShowMoves => "settings.editorDiffShowMoves",
            Self::TerminalCursorBlink => "settings.terminalCursorBlink",
        }
    }

    pub fn description(self) -> Option<&'static str> {
        match self {
            Self::OrganizeImportsOnSave => Some("settings.organizeImportsOnSaveDescription"),
            Self::FixAllOnSave => Some("settings.fixAllOnSaveDescription"),
            Self::TrimTrailingWhitespaceOnSave => {
                Some("settings.trimTrailingWhitespaceOnSaveDescription")
            }
            Self::InsertFinalNewlineOnSave => Some("settings.insertFinalNewlineOnSaveDescription"),
            Self::EditorConfigEnabled => Some("settings.editorConfigEnabledDescription"),
            Self::EditorCodeLens => Some("settings.editorCodeLensDescription"),
            Self::EditorDetectIndentation => Some("settings.editorDetectIndentationHint"),
            Self::EditorBracketPairGuides => Some("settings.editorBracketPairGuidesDescription"),
            Self::EditorCursorSmoothCaretAnimation => {
                Some("settings.editorCursorSmoothCaretAnimationDescription")
            }
            Self::EditorSmoothScrolling => Some("settings.editorSmoothScrollingDescription"),
            Self::EditorStickyScroll => Some("settings.editorStickyScrollDescription"),
            Self::EditorSemanticHighlighting => {
                Some("settings.editorSemanticHighlightingDescription")
            }
            Self::EditorFormatOnType => Some("settings.editorFormatOnTypeDescription"),
            Self::EditorFormatOnPaste => Some("settings.editorFormatOnPasteDescription"),
            Self::EditorSuggestPreview => Some("settings.editorSuggestPreviewDescription"),
            Self::EmmetEnabled => Some("settings.emmetEnabledDescription"),
            Self::EditorDiffHideUnchangedRegions => {
                Some("settings.editorDiffHideUnchangedRegionsDescription")
            }
            Self::EditorDiffShowMoves => Some("settings.editorDiffShowMovesDescription"),
            _ => None,
        }
    }

    pub fn value(self, settings: &Settings) -> bool {
        match self {
            Self::FormatOnSave => settings.format_on_save,
            Self::OrganizeImportsOnSave => settings.organize_imports_on_save,
            Self::FixAllOnSave => settings.fix_all_on_save,
            Self::TrimTrailingWhitespaceOnSave => settings.trim_trailing_whitespace_on_save,
            Self::InsertFinalNewlineOnSave => settings.insert_final_newline_on_save,
            Self::EditorConfigEnabled => settings.editor_config_enabled,
            Self::EditorCodeLens => settings.editor_code_lens_enabled,
            Self::EditorWordWrap => settings.editor_word_wrap,
            Self::EditorLineNumbers => settings.editor_line_numbers,
            Self::EditorInsertSpaces => settings.editor_insert_spaces,
            Self::EditorDetectIndentation => settings.editor_detect_indentation,
            Self::EditorBracketPairColorization => settings.editor_bracket_pair_colorization,
            Self::EditorBracketPairGuides => settings.editor_bracket_pair_guides,
            Self::EditorFontLigatures => settings.editor_font_ligatures,
            Self::EditorCursorSmoothCaretAnimation => settings.editor_cursor_smooth_caret_animation,
            Self::EditorScrollBeyondLastLine => settings.editor_scroll_beyond_last_line,
            Self::EditorSmoothScrolling => settings.editor_smooth_scrolling,
            Self::EditorStickyScroll => settings.editor_sticky_scroll_enabled,
            Self::EditorSemanticHighlighting => settings.editor_semantic_highlighting,
            Self::EditorFormatOnType => settings.editor_format_on_type,
            Self::EditorFormatOnPaste => settings.editor_format_on_paste,
            Self::EditorSuggestPreview => settings.editor_suggest_preview,
            Self::EmmetEnabled => settings.emmet_enabled,
            Self::EditorDiffHideUnchangedRegions => settings.editor_diff_hide_unchanged_regions,
            Self::EditorDiffShowMoves => settings.editor_diff_show_moves,
            Self::TerminalCursorBlink => settings.terminal_cursor_blink,
        }
    }

    fn patch(self, value: bool) -> SettingsPatch {
        match self {
            Self::FormatOnSave => SettingsPatch {
                format_on_save: Some(value),
                ..Default::default()
            },
            Self::OrganizeImportsOnSave => SettingsPatch {
                organize_imports_on_save: Some(value),
                ..Default::default()
            },
            Self::FixAllOnSave => SettingsPatch {
                fix_all_on_save: Some(value),
                ..Default::default()
            },
            Self::TrimTrailingWhitespaceOnSave => SettingsPatch {
                trim_trailing_whitespace_on_save: Some(value),
                ..Default::default()
            },
            Self::InsertFinalNewlineOnSave => SettingsPatch {
                insert_final_newline_on_save: Some(value),
                ..Default::default()
            },
            Self::EditorConfigEnabled => SettingsPatch {
                editor_config_enabled: Some(value),
                ..Default::default()
            },
            Self::EditorCodeLens => SettingsPatch {
                editor_code_lens_enabled: Some(value),
                ..Default::default()
            },
            Self::EditorWordWrap => SettingsPatch {
                editor_word_wrap: Some(value),
                ..Default::default()
            },
            Self::EditorLineNumbers => SettingsPatch {
                editor_line_numbers: Some(value),
                ..Default::default()
            },
            Self::EditorInsertSpaces => SettingsPatch {
                editor_insert_spaces: Some(value),
                ..Default::default()
            },
            Self::EditorDetectIndentation => SettingsPatch {
                editor_detect_indentation: Some(value),
                ..Default::default()
            },
            Self::EditorBracketPairColorization => SettingsPatch {
                editor_bracket_pair_colorization: Some(value),
                ..Default::default()
            },
            Self::EditorBracketPairGuides => SettingsPatch {
                editor_bracket_pair_guides: Some(value),
                ..Default::default()
            },
            Self::EditorFontLigatures => SettingsPatch {
                editor_font_ligatures: Some(value),
                ..Default::default()
            },
            Self::EditorCursorSmoothCaretAnimation => SettingsPatch {
                editor_cursor_smooth_caret_animation: Some(value),
                ..Default::default()
            },
            Self::EditorScrollBeyondLastLine => SettingsPatch {
                editor_scroll_beyond_last_line: Some(value),
                ..Default::default()
            },
            Self::EditorSmoothScrolling => SettingsPatch {
                editor_smooth_scrolling: Some(value),
                ..Default::default()
            },
            Self::EditorStickyScroll => SettingsPatch {
                editor_sticky_scroll_enabled: Some(value),
                ..Default::default()
            },
            Self::EditorSemanticHighlighting => SettingsPatch {
                editor_semantic_highlighting: Some(value),
                ..Default::default()
            },
            Self::EditorFormatOnType => SettingsPatch {
                editor_format_on_type: Some(value),
                ..Default::default()
            },
            Self::EditorFormatOnPaste => SettingsPatch {
                editor_format_on_paste: Some(value),
                ..Default::default()
            },
            Self::EditorSuggestPreview => SettingsPatch {
                editor_suggest_preview: Some(value),
                ..Default::default()
            },
            Self::EmmetEnabled => SettingsPatch {
                emmet_enabled: Some(value),
                ..Default::default()
            },
            Self::EditorDiffHideUnchangedRegions => SettingsPatch {
                editor_diff_hide_unchanged_regions: Some(value),
                ..Default::default()
            },
            Self::EditorDiffShowMoves => SettingsPatch {
                editor_diff_show_moves: Some(value),
                ..Default::default()
            },
            Self::TerminalCursorBlink => SettingsPatch {
                terminal_cursor_blink: Some(value),
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    Editor,
    Terminal,
}

impl Font {
    pub fn label(self) -> &'static str {
        match self {
            Self::Editor => "settings.editorFontFamily",
            Self::Terminal => "settings.terminalFontFamily",
        }
    }

    pub fn value(self, settings: &Settings) -> Option<&str> {
        match self {
            Self::Editor => settings.editor_font_family.as_deref(),
            Self::Terminal => settings.terminal_font_family.as_deref(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Whitespace(EditorRenderWhitespace),
    EditorCursor(EditorCursorStyle),
    EditorBlinking(EditorCursorBlinking),
    TerminalCursor(TerminalCursorStyle),
}

impl Selection {
    pub const WHITESPACE: [Self; 4] = [
        Self::Whitespace(EditorRenderWhitespace::None),
        Self::Whitespace(EditorRenderWhitespace::Boundary),
        Self::Whitespace(EditorRenderWhitespace::Selection),
        Self::Whitespace(EditorRenderWhitespace::All),
    ];
    pub const EDITOR_CURSOR: [Self; 3] = [
        Self::EditorCursor(EditorCursorStyle::Line),
        Self::EditorCursor(EditorCursorStyle::Block),
        Self::EditorCursor(EditorCursorStyle::Underline),
    ];
    pub const EDITOR_BLINKING: [Self; 5] = [
        Self::EditorBlinking(EditorCursorBlinking::Blink),
        Self::EditorBlinking(EditorCursorBlinking::Smooth),
        Self::EditorBlinking(EditorCursorBlinking::Phase),
        Self::EditorBlinking(EditorCursorBlinking::Expand),
        Self::EditorBlinking(EditorCursorBlinking::Solid),
    ];
    pub const TERMINAL_CURSOR: [Self; 3] = [
        Self::TerminalCursor(TerminalCursorStyle::Bar),
        Self::TerminalCursor(TerminalCursorStyle::Block),
        Self::TerminalCursor(TerminalCursorStyle::Underline),
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Whitespace(_) => "settings.editorRenderWhitespace",
            Self::EditorCursor(_) => "settings.editorCursorStyle",
            Self::EditorBlinking(_) => "settings.editorCursorBlinking",
            Self::TerminalCursor(_) => "settings.terminalCursorStyle",
        }
    }

    pub fn choices(self) -> &'static [Self] {
        match self {
            Self::Whitespace(_) => &Self::WHITESPACE,
            Self::EditorCursor(_) => &Self::EDITOR_CURSOR,
            Self::EditorBlinking(_) => &Self::EDITOR_BLINKING,
            Self::TerminalCursor(_) => &Self::TERMINAL_CURSOR,
        }
    }

    pub fn option_label(self) -> &'static str {
        match self {
            Self::Whitespace(EditorRenderWhitespace::None) => "settings.renderWhitespaceNone",
            Self::Whitespace(EditorRenderWhitespace::Boundary) => {
                "settings.renderWhitespaceBoundary"
            }
            Self::Whitespace(EditorRenderWhitespace::Selection) => {
                "settings.renderWhitespaceSelection"
            }
            Self::Whitespace(EditorRenderWhitespace::All) => "settings.renderWhitespaceAll",
            Self::EditorCursor(EditorCursorStyle::Line) => "settings.cursorStyleLine",
            Self::EditorCursor(EditorCursorStyle::Block)
            | Self::TerminalCursor(TerminalCursorStyle::Block) => "settings.cursorStyleBlock",
            Self::EditorCursor(EditorCursorStyle::Underline)
            | Self::TerminalCursor(TerminalCursorStyle::Underline) => {
                "settings.cursorStyleUnderline"
            }
            Self::TerminalCursor(TerminalCursorStyle::Bar) => "settings.cursorStyleBar",
            Self::EditorBlinking(EditorCursorBlinking::Blink) => "settings.cursorBlinkingBlink",
            Self::EditorBlinking(EditorCursorBlinking::Smooth) => "settings.cursorBlinkingSmooth",
            Self::EditorBlinking(EditorCursorBlinking::Phase) => "settings.cursorBlinkingPhase",
            Self::EditorBlinking(EditorCursorBlinking::Expand) => "settings.cursorBlinkingExpand",
            Self::EditorBlinking(EditorCursorBlinking::Solid) => "settings.cursorBlinkingSolid",
        }
    }

    fn patch(self) -> SettingsPatch {
        match self {
            Self::Whitespace(value) => SettingsPatch {
                editor_render_whitespace: Some(value),
                ..Default::default()
            },
            Self::EditorCursor(value) => SettingsPatch {
                editor_cursor_style: Some(value),
                ..Default::default()
            },
            Self::EditorBlinking(value) => SettingsPatch {
                editor_cursor_blinking: Some(value),
                ..Default::default()
            },
            Self::TerminalCursor(value) => SettingsPatch {
                terminal_cursor_style: Some(value),
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    Rulers,
    Shell,
}

impl Text {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rulers => "settings.editorRulers",
            Self::Shell => "settings.shell",
        }
    }

    pub fn value(self, settings: &Settings) -> String {
        match self {
            Self::Rulers => format_rulers(&settings.editor_rulers),
            Self::Shell => settings.shell_override.clone().unwrap_or_default(),
        }
    }
}

pub struct TextDraft {
    committed: String,
    pub text: String,
}

impl TextDraft {
    pub fn new(value: String) -> Self {
        Self {
            text: value.clone(),
            committed: value,
        }
    }

    pub fn sync(&mut self, value: String) {
        if self.committed != value {
            self.text.clone_from(&value);
            self.committed = value;
        }
    }

    pub fn commit(&mut self, field: Text) -> Change {
        match field {
            Text::Rulers => {
                let rulers = parse_rulers(&self.text);
                self.text = format_rulers(&rulers);
                Change::Rulers(rulers)
            }
            Text::Shell => Change::Shell(trim_text(&self.text).into()),
        }
    }
}

pub fn parse_rulers(text: &str) -> Vec<u32> {
    text.split(',')
        .map(trim_text)
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let prefixed = [
                ("0x", 16),
                ("0X", 16),
                ("0b", 2),
                ("0B", 2),
                ("0o", 8),
                ("0O", 8),
            ]
            .into_iter()
            .find_map(|(prefix, radix)| part.strip_prefix(prefix).map(|digits| (digits, radix)));
            let value = match prefixed {
                Some((digits, radix)) => u32::from_str_radix(digits, radix).ok().map(f64::from),
                None => part.parse::<f64>().ok(),
            }?;
            (value.is_finite()
                && value.fract() == 0.0
                && (f64::from(RULER_MIN)..=f64::from(RULER_MAX)).contains(&value))
            .then_some(value as u32)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(RULER_COUNT)
        .collect()
}

pub fn format_rulers(rulers: &[u32]) -> String {
    rulers
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn trim_text(text: &str) -> &str {
    text.trim_matches(|character: char| {
        matches!(
            character,
            '\u{0009}'..='\u{000d}'
                | '\u{0020}'
                | '\u{00a0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
        )
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Switch(Switch, bool),
    Font(Font, Option<String>),
    Selection(Selection),
    Rulers(Vec<u32>),
    Shell(String),
}

impl Change {
    pub fn patch(&self) -> SettingsPatch {
        match self {
            Self::Switch(field, value) => field.patch(*value),
            Self::Selection(value) => value.patch(),
            Self::Font(field, value) => {
                let value = Some(value.clone().unwrap_or_default());
                match field {
                    Font::Editor => SettingsPatch {
                        editor_font_family: value,
                        ..Default::default()
                    },
                    Font::Terminal => SettingsPatch {
                        terminal_font_family: value,
                        ..Default::default()
                    },
                }
            }
            Self::Rulers(value) => SettingsPatch {
                editor_rulers: Some(value.clone()),
                ..Default::default()
            },
            Self::Shell(value) => SettingsPatch {
                shell_override: Some(value.clone()),
                ..Default::default()
            },
        }
    }
}

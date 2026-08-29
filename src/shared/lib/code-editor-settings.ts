import type { Settings } from '@shared/api/bindings'
import {
    DEFAULT_EDITOR_CURSOR_BLINKING,
    DEFAULT_EDITOR_CURSOR_STYLE,
    DEFAULT_EDITOR_RENDER_WHITESPACE,
    DEFAULT_EDITOR_TAB_SIZE,
} from '@shared/constants/code-editor'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { NO_EDITOR_RULERS } from '@shared/lib/editor-rulers'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'

type CodeEditorSettingsSource = Pick<
    Settings,
    | 'editorFontFamily'
    | 'editorFontSize'
    | 'editorMinimap'
    | 'editorWordWrap'
    | 'editorLineNumbers'
    | 'editorTabSize'
    | 'editorInsertSpaces'
    | 'editorDetectIndentation'
    | 'editorRenderWhitespace'
    | 'editorBracketPairColorization'
    | 'editorFontLigatures'
    | 'editorCursorStyle'
    | 'editorCursorBlinking'
    | 'editorScrollBeyondLastLine'
    | 'editorStickyScrollEnabled'
    | 'editorBracketPairGuides'
    | 'editorSmoothScrolling'
    | 'editorCursorSmoothCaretAnimation'
    | 'editorSuggestPreview'
    | 'editorRulers'
    | 'aiAutoTabEnabled'
>

/**
 * Derives the `CodeEditor` props that come straight from `Settings` (with their defaults) — shared
 * across every `CodeEditor` host (`editor-pane`, `untitled-pane`, `app-file-pane`). Each host still
 * wires `formatOnType`/`formatOnPaste` itself: `editor-pane` reads them from settings, the other two
 * hosts hard-code `false` (no LSP formatting provider ever attaches to an untitled/app-file model).
 */
export const resolveCodeEditorSettingsProps = (settings: CodeEditorSettingsSource | undefined) => ({
    fontFamily: buildMonospaceFontStack(settings?.editorFontFamily ?? null),
    fontSize: settings?.editorFontSize ?? DEFAULT_CODE_FONT_SIZE,
    minimap: settings?.editorMinimap ?? true,
    wordWrap: settings?.editorWordWrap ?? false,
    lineNumbers: settings?.editorLineNumbers ?? true,
    tabSize: settings?.editorTabSize ?? DEFAULT_EDITOR_TAB_SIZE,
    insertSpaces: settings?.editorInsertSpaces ?? true,
    detectIndentation: settings?.editorDetectIndentation ?? true,
    renderWhitespace: settings?.editorRenderWhitespace ?? DEFAULT_EDITOR_RENDER_WHITESPACE,
    bracketPairColorization: settings?.editorBracketPairColorization ?? true,
    fontLigatures: settings?.editorFontLigatures ?? false,
    cursorStyle: settings?.editorCursorStyle ?? DEFAULT_EDITOR_CURSOR_STYLE,
    cursorBlinking: settings?.editorCursorBlinking ?? DEFAULT_EDITOR_CURSOR_BLINKING,
    scrollBeyondLastLine: settings?.editorScrollBeyondLastLine ?? true,
    stickyScroll: settings?.editorStickyScrollEnabled ?? true,
    bracketPairGuides: settings?.editorBracketPairGuides ?? false,
    smoothScrolling: settings?.editorSmoothScrolling ?? false,
    cursorSmoothCaretAnimation: settings?.editorCursorSmoothCaretAnimation ?? false,
    suggestPreview: settings?.editorSuggestPreview ?? false,
    rulers: settings?.editorRulers ?? NO_EDITOR_RULERS,
    aiAutoTabEnabled: settings?.aiAutoTabEnabled ?? false,
})

import type { EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, Settings } from '@shared/api/bindings'
import { DEFAULT_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'

export const DEFAULT_EDITOR_TAB_SIZE = 4
export const DEFAULT_EDITOR_RENDER_WHITESPACE: EditorRenderWhitespace = 'selection'
export const DEFAULT_EDITOR_CURSOR_STYLE: EditorCursorStyle = 'line'
export const DEFAULT_EDITOR_CURSOR_BLINKING: EditorCursorBlinking = 'blink'

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
    aiAutoTabEnabled: settings?.aiAutoTabEnabled ?? false,
})

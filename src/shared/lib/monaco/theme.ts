import type { editor as MonacoEditorAPI } from 'monaco-editor'
import type { ResolvedTheme, SyntaxStyle } from '@shared/api/bindings'

export const TAIDE_MONACO_THEME_NAME = 'taide'

const HEX6_PATTERN = /^#[0-9a-fA-F]{6}$/
const HEX_COLOR_PATTERN = /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/

const EDITOR_COLOR_ID_MAP: Record<string, string> = {
    'editor.background': 'editor.background',
    'editor.foreground': 'editor.foreground',
    'editor.lineHighlight': 'editor.lineHighlightBackground',
    'editor.cursor': 'editorCursor.foreground',
    'editor.selection': 'editor.selectionBackground',
    'editor.inactiveSelection': 'editor.inactiveSelectionBackground',
    'editor.lineNumber': 'editorLineNumber.foreground',
    'editor.lineNumberActive': 'editorLineNumber.activeForeground',
    'editor.indentGuide': 'editorIndentGuide.background',
    'editor.whitespace': 'editorWhitespace.foreground',
    'editor.bracketMatch': 'editorBracketMatch.border',
    'editor.findMatch': 'editor.findMatchBackground',
    'editor.findMatchHighlight': 'editor.findMatchHighlightBackground',
    'editor.hoverBackground': 'editorHoverWidget.background',
    'editor.widgetBackground': 'editorWidget.background',
    'editor.widgetBorder': 'editorWidget.border',
    'editorGutter.addedBackground': 'editorGutter.addedBackground',
    'editorGutter.modifiedBackground': 'editorGutter.modifiedBackground',
    'editorGutter.deletedBackground': 'editorGutter.deletedBackground',
    'diff.insertedBackground': 'diffEditor.insertedTextBackground',
    'diff.insertedLineBackground': 'diffEditor.insertedLineBackground',
    'diff.removedBackground': 'diffEditor.removedTextBackground',
    'diff.removedLineBackground': 'diffEditor.removedLineBackground',
    'diff.border': 'diffEditor.border',
}

export class InvalidHexColorError extends Error {
    constructor(value: string) {
        super(`invalid hex color: ${value}`)
        this.name = 'InvalidHexColorError'
    }
}

export const toRuleForeground = (hex: string) => {
    if (!HEX6_PATTERN.test(hex)) throw new InvalidHexColorError(hex)
    return hex.slice(1)
}

export const toThemeColor = (hex: string) => {
    if (!HEX_COLOR_PATTERN.test(hex)) throw new InvalidHexColorError(hex)
    return hex
}

export const toMonacoFontStyle = (style: SyntaxStyle) => {
    const modifiers: string[] = []
    if (style.bold) modifiers.push('bold')
    if (style.italic) modifiers.push('italic')
    return modifiers.length > 0 ? modifiers.join(' ') : undefined
}

export type MonacoTokenRule = {
    token: string
    foreground: string
    fontStyle?: string
}

export const buildTokenRules = (syntax: ResolvedTheme['syntax']): MonacoTokenRule[] =>
    Object.entries(syntax).map(([token, style]) => {
        const fontStyle = toMonacoFontStyle(style)
        const foreground = toRuleForeground(style.fg)
        return fontStyle ? { token, foreground, fontStyle } : { token, foreground }
    })

export const buildThemeColors = (colors: ResolvedTheme['colors']): Record<string, string> =>
    Object.fromEntries(
        Object.entries(EDITOR_COLOR_ID_MAP)
            .filter(([taideToken]) => taideToken in colors)
            .map(([taideToken, monacoColorId]) => [monacoColorId, toThemeColor(colors[taideToken])]),
    )

export type MonacoThemeData = {
    base: 'vs' | 'vs-dark'
    inherit: boolean
    rules: MonacoTokenRule[]
    colors: Record<string, string>
}

export const buildMonacoThemeData = (theme: ResolvedTheme): MonacoThemeData => ({
    base: theme.type === 'dark' ? 'vs-dark' : 'vs',
    inherit: true,
    rules: buildTokenRules(theme.syntax),
    colors: buildThemeColors(theme.colors),
})

type MonacoEditorNamespace = Pick<typeof MonacoEditorAPI, 'defineTheme' | 'setTheme'>

export const applyMonacoTheme = (theme: ResolvedTheme, editorNamespace: MonacoEditorNamespace) => {
    editorNamespace.defineTheme(TAIDE_MONACO_THEME_NAME, buildMonacoThemeData(theme))
    editorNamespace.setTheme(TAIDE_MONACO_THEME_NAME)
}

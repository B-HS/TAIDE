import type { ResolvedTheme, SyntaxStyle } from '@shared/api/bindings'

export const TAIDE_MONACO_THEME_NAME = 'taide'

const HEX6_PATTERN = /^#[0-9a-fA-F]{6}$/
const HEX_COLOR_PATTERN = /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/

const MONACO_COLOR_SOURCE_EDITOR_CORE: Record<string, string> = {
    'editor.background': 'editor.background',
    'editor.foreground': 'editor.foreground',
    'editor.lineHighlightBackground': 'editor.lineHighlight',
    'editorCursor.foreground': 'editor.cursor',
    'editor.selectionBackground': 'editor.selection',
    'editor.inactiveSelectionBackground': 'editor.inactiveSelection',
    'editorLineNumber.foreground': 'editor.lineNumber',
    'editorLineNumber.activeForeground': 'editor.lineNumberActive',
    'editorIndentGuide.background': 'editor.indentGuide',
    'editorWhitespace.foreground': 'editor.whitespace',
    'editorBracketMatch.border': 'editor.bracketMatch',
    'editor.findMatchBackground': 'editor.findMatch',
    'editor.findMatchHighlightBackground': 'editor.findMatchHighlight',
    'editorHoverWidget.background': 'editor.hoverBackground',
    'editorWidget.background': 'editor.widgetBackground',
    'editorWidget.border': 'editor.widgetBorder',
    'editorGutter.addedBackground': 'editorGutter.addedBackground',
    'editorGutter.modifiedBackground': 'editorGutter.modifiedBackground',
    'editorGutter.deletedBackground': 'editorGutter.deletedBackground',
    'diffEditor.insertedTextBackground': 'diff.insertedBackground',
    'diffEditor.insertedLineBackground': 'diff.insertedLineBackground',
    'diffEditor.removedTextBackground': 'diff.removedBackground',
    'diffEditor.removedLineBackground': 'diff.removedLineBackground',
    'diffEditor.border': 'diff.border',
}

const MONACO_COLOR_SOURCE_WIDGET_SHELL: Record<string, string> = {
    'editorWidget.foreground': 'editor.foreground',
    'editorWidget.resizeBorder': 'app.focusBorder',
    'widget.border': 'editor.widgetBorder',
    'widget.shadow': 'app.shadow',
    focusBorder: 'app.focusBorder',
    foreground: 'app.foreground',
    descriptionForeground: 'panel.sectionHeader',
    errorForeground: 'statusIndicator.error',
    'icon.foreground': 'appSidebar.iconDefault',
    'toolbar.hoverBackground': 'list.hoverBackground',
    'toolbar.activeBackground': 'list.activeBackground',
    'selection.background': 'editor.selection',
    'progressBar.background': 'app.accent',
    'badge.background': 'appSidebar.badge',
    'badge.foreground': 'button.primaryForeground',
    'textLink.foreground': 'terminal.linkForeground',
    'textLink.activeForeground': 'app.accent',
    'textCodeBlock.background': 'panel.background',
    'textPreformat.foreground': 'editor.foreground',
    'textPreformat.background': 'panel.background',
    'textSeparator.foreground': 'app.border',
}

const MONACO_COLOR_SOURCE_INPUT: Record<string, string> = {
    'input.background': 'input.background',
    'input.foreground': 'input.foreground',
    'input.border': 'input.border',
    'input.placeholderForeground': 'input.placeholder',
    'inputOption.activeBackground': 'button.primaryBackground',
    'inputOption.activeForeground': 'button.primaryForeground',
    'inputOption.activeBorder': 'app.focusBorder',
    'inputOption.hoverBackground': 'list.hoverBackground',
    'inputValidation.errorBackground': 'panel.background',
    'inputValidation.errorBorder': 'statusIndicator.error',
    'inputValidation.errorForeground': 'app.foreground',
    'inputValidation.warningBackground': 'panel.background',
    'inputValidation.warningBorder': 'statusIndicator.warning',
    'inputValidation.warningForeground': 'app.foreground',
    'inputValidation.infoBackground': 'panel.background',
    'inputValidation.infoBorder': 'statusIndicator.info',
    'inputValidation.infoForeground': 'app.foreground',
}

const MONACO_COLOR_SOURCE_PEEK_VIEW: Record<string, string> = {
    'peekView.border': 'app.accent',
    'peekViewTitle.background': 'editor.widgetBackground',
    'peekViewTitleLabel.foreground': 'app.foreground',
    'peekViewTitleDescription.foreground': 'panel.sectionHeader',
    'peekViewEditor.background': 'editor.background',
    'peekViewEditorGutter.background': 'editor.background',
    'peekViewEditorStickyScroll.background': 'editor.widgetBackground',
    'peekViewEditorStickyScrollGutter.background': 'editor.widgetBackground',
    'peekViewEditor.matchHighlightBackground': 'editor.findMatchHighlight',
    'peekViewEditor.matchHighlightBorder': 'editor.findMatch',
    'peekViewResult.background': 'list.background',
    'peekViewResult.fileForeground': 'app.foreground',
    'peekViewResult.lineForeground': 'panel.sectionHeader',
    'peekViewResult.matchHighlightBackground': 'editor.findMatchHighlight',
    'peekViewResult.selectionBackground': 'list.activeBackground',
    'peekViewResult.selectionForeground': 'list.foreground',
}

const MONACO_COLOR_SOURCE_LIST_TREE: Record<string, string> = {
    'list.hoverBackground': 'list.hoverBackground',
    'list.hoverForeground': 'list.foreground',
    'list.focusBackground': 'list.activeBackground',
    'list.focusForeground': 'list.foreground',
    'list.focusOutline': 'app.focusBorder',
    'list.activeSelectionBackground': 'list.activeBackground',
    'list.activeSelectionForeground': 'list.foreground',
    'list.activeSelectionIconForeground': 'list.foreground',
    'list.inactiveSelectionBackground': 'list.hoverBackground',
    'list.inactiveSelectionForeground': 'list.foreground',
    'list.inactiveFocusBackground': 'list.hoverBackground',
    'list.highlightForeground': 'app.accent',
    'list.focusHighlightForeground': 'app.accent',
    'list.dropBackground': 'list.hoverBackground',
    'list.deemphasizedForeground': 'panel.sectionHeader',
    'list.errorForeground': 'statusIndicator.error',
    'list.warningForeground': 'statusIndicator.warning',
    'tree.indentGuidesStroke': 'explorer.indentGuide',
    'tree.inactiveIndentGuidesStroke': 'explorer.indentGuide',
    'tree.tableColumnsBorder': 'app.border',
    'tree.tableOddRowsBackground': 'list.hoverBackground',
}

const MONACO_COLOR_SOURCE_SUGGEST_HOVER: Record<string, string> = {
    'editorSuggestWidget.background': 'editor.widgetBackground',
    'editorSuggestWidget.border': 'editor.widgetBorder',
    'editorSuggestWidget.foreground': 'editor.foreground',
    'editorSuggestWidget.selectedBackground': 'list.activeBackground',
    'editorSuggestWidget.selectedForeground': 'list.foreground',
    'editorSuggestWidget.selectedIconForeground': 'list.foreground',
    'editorSuggestWidget.highlightForeground': 'app.accent',
    'editorSuggestWidget.focusHighlightForeground': 'app.accent',
    'editorSuggestWidgetStatus.foreground': 'panel.sectionHeader',
    'editorHoverWidget.foreground': 'editor.foreground',
    'editorHoverWidget.border': 'editor.widgetBorder',
    'editorHoverWidget.statusBarBackground': 'editor.widgetBackground',
    'editorHoverWidget.highlightForeground': 'app.accent',
}

const MONACO_COLOR_SOURCE_QUICK_INPUT_MENU: Record<string, string> = {
    'quickInput.background': 'modal.background',
    'quickInput.foreground': 'app.foreground',
    'quickInputTitle.background': 'editor.widgetBackground',
    'quickInputList.focusBackground': 'list.activeBackground',
    'quickInputList.focusForeground': 'list.foreground',
    'quickInputList.focusIconForeground': 'list.foreground',
    'quickInputList.focusHighlightForeground': 'app.accent',
    'pickerGroup.foreground': 'panel.sectionHeader',
    'pickerGroup.border': 'app.border',
    'menu.background': 'menu.background',
    'menu.foreground': 'app.foreground',
    'menu.border': 'menu.border',
    'menu.selectionBackground': 'menu.itemHover',
    'menu.selectionForeground': 'app.foreground',
    'menu.separatorBackground': 'menu.separator',
    'dropdown.background': 'popover.background',
    'dropdown.listBackground': 'list.background',
    'dropdown.foreground': 'app.foreground',
    'dropdown.border': 'popover.border',
    'keybindingLabel.background': 'button.background',
    'keybindingLabel.foreground': 'button.foreground',
    'keybindingLabel.border': 'app.border',
    'keybindingLabel.bottomBorder': 'app.border',
    'checkbox.background': 'input.background',
    'checkbox.foreground': 'input.foreground',
    'checkbox.border': 'input.border',
    'button.background': 'button.primaryBackground',
    'button.foreground': 'button.primaryForeground',
    'button.hoverBackground': 'button.hoverBackground',
    'button.border': 'app.border',
    'button.secondaryBackground': 'button.background',
    'button.secondaryForeground': 'button.foreground',
    'button.secondaryHoverBackground': 'button.hoverBackground',
}

const MONACO_COLOR_SOURCE_SCROLLBAR_MARKER: Record<string, string> = {
    'scrollbarSlider.background': 'scrollbar.thumb',
    'scrollbarSlider.hoverBackground': 'scrollbar.thumbHover',
    'scrollbarSlider.activeBackground': 'scrollbar.thumbHover',
    'scrollbar.shadow': 'app.shadow',
    'minimapSlider.background': 'scrollbar.thumb',
    'minimapSlider.hoverBackground': 'scrollbar.thumbHover',
    'minimapSlider.activeBackground': 'scrollbar.thumbHover',
    'editorMarkerNavigation.background': 'editor.widgetBackground',
    'editorMarkerNavigationError.background': 'statusIndicator.error',
    'editorMarkerNavigationWarning.background': 'statusIndicator.warning',
    'editorMarkerNavigationInfo.background': 'statusIndicator.info',
    'editorError.foreground': 'statusIndicator.error',
    'editorWarning.foreground': 'statusIndicator.warning',
    'editorInfo.foreground': 'statusIndicator.info',
    'problemsErrorIcon.foreground': 'statusIndicator.error',
    'problemsWarningIcon.foreground': 'statusIndicator.warning',
    'problemsInfoIcon.foreground': 'statusIndicator.info',
    'editorLink.activeForeground': 'terminal.linkForeground',
    'editorGutter.background': 'editor.background',
    'editorStickyScroll.background': 'editor.widgetBackground',
    'editorStickyScrollGutter.background': 'editor.widgetBackground',
    'editorStickyScroll.border': 'editor.widgetBorder',
    'editorCodeLens.foreground': 'editorBlame.foreground',
}

export const MONACO_COLOR_SOURCE: Record<string, string> = {
    ...MONACO_COLOR_SOURCE_EDITOR_CORE,
    ...MONACO_COLOR_SOURCE_WIDGET_SHELL,
    ...MONACO_COLOR_SOURCE_INPUT,
    ...MONACO_COLOR_SOURCE_PEEK_VIEW,
    ...MONACO_COLOR_SOURCE_LIST_TREE,
    ...MONACO_COLOR_SOURCE_SUGGEST_HOVER,
    ...MONACO_COLOR_SOURCE_QUICK_INPUT_MENU,
    ...MONACO_COLOR_SOURCE_SCROLLBAR_MARKER,
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
        Object.entries(MONACO_COLOR_SOURCE)
            .filter(([, taideToken]) => taideToken in colors && HEX_COLOR_PATTERN.test(colors[taideToken]))
            .map(([monacoColorId, taideToken]) => [monacoColorId, toThemeColor(colors[taideToken])]),
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

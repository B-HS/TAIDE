import { hexToRgb } from '@shared/lib/color'
import type { AnsiLookup, ColorCategory, ColorMappingEntry, ResolveContext, ThemeTypeArg } from '@shared/lib/theme-convert/types'

const SELF_REF_PREFIX = '@'

export const FAMILY_FALLBACK_SOURCE_KEYS: Record<ColorCategory, string[]> = {
    foreground: ['editor.foreground', 'foreground'],
    background: ['editor.background'],
    border: ['panel.border', 'editorGroup.border', 'contrastBorder'],
    status: [],
    shadow: [],
}

export const SAFE_DEFAULT_COLORS: Record<ThemeTypeArg, Record<ColorCategory, string>> = {
    dark: {
        foreground: '#D4D4D4',
        background: '#1E1E1E',
        border: '#3C3C3C',
        status: '#569CD6',
        shadow: '#00000080',
    },
    light: {
        foreground: '#1E1E1E',
        background: '#FFFFFF',
        border: '#D4D4D4',
        status: '#0066BF',
        shadow: '#00000026',
    },
}

/**
 * VS Code's official `list.hoverBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.hoverBackground', { dark: '#2A2D2E', light: '#F0F0F0', ... })). Themes that either
 * omit `list.hoverBackground` or set it to a value indistinguishable from the row background (same RGB,
 * or alpha 00) fall back to this instead of the generic same-family background fallback — otherwise the
 * hover state silently disappears (docs/theme-system.md §8.2.2).
 */
export const VSCODE_LIST_HOVER_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#2A2D2E',
    light: '#F0F0F0',
}

/**
 * VS Code's official `list.activeSelectionBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.activeSelectionBackground', { dark: '#04395E', light: '#0060C0', ... })). Themes that either
 * omit `list.activeSelectionBackground` or set it to a value indistinguishable from the row background fall back
 * to this instead of the generic same-family background fallback — otherwise the focused selection silently
 * disappears (docs/theme-system.md §8.2.2).
 *
 * The `light` value diverges from VS Code's own default (`#0060C0`). VS Code pairs that dark blue with a
 * dedicated `list.activeSelectionForeground` (white) that TAIDE has no equivalent token for, so rows inherit
 * `app.foreground` (black) and land at ~3.4:1 contrast — below the 4.5:1 AA floor for small text. `#ADD6FF`
 * is VS Code's own light-theme `editor.selectionBackground` default, which clears 13.8:1 against black
 * (docs/theme-system.md §8.2.2).
 */
export const VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#04395E',
    light: '#ADD6FF',
}

/**
 * VS Code's official `list.inactiveSelectionBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.inactiveSelectionBackground', { dark: '#37373D', light: '#E4E6F1', ... })). Same fallback
 * reasoning as `VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT` above, applied to the unfocused selection state.
 */
export const VSCODE_LIST_INACTIVE_SELECTION_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#37373D',
    light: '#E4E6F1',
}

export const COLOR_NAMESPACES: readonly { id: string; tokens: readonly string[] }[] = [
    { id: 'app', tokens: ['background', 'foreground', 'border', 'focusBorder', 'shadow', 'accent'] },
    {
        id: 'appSidebar',
        tokens: [
            'background',
            'itemHover',
            'itemActive',
            'iconDefault',
            'iconAgentRunning',
            'iconAgentWorking',
            'iconAgentAwaiting',
            'iconAgentIdle',
            'iconAgentUnknown',
            'badge',
        ],
    },
    {
        id: 'tabBar',
        tokens: [
            'background',
            'tabActiveBackground',
            'tabInactiveBackground',
            'tabActiveForeground',
            'tabInactiveForeground',
            'tabBorder',
            'tabActiveIndicator',
            'dirtyDot',
            'previewForeground',
            'dropTarget',
        ],
    },
    {
        id: 'explorer',
        tokens: [
            'background',
            'itemHover',
            'itemSelected',
            'itemFocused',
            'indentGuide',
            'folderIcon',
            'gitModified',
            'gitAdded',
            'gitDeleted',
            'gitUntracked',
            'gitIgnored',
        ],
    },
    { id: 'panel', tokens: ['background', 'sectionHeader', 'inputBackground', 'inputBorder', 'matchHighlight'] },
    {
        id: 'editor',
        tokens: [
            'background',
            'foreground',
            'lineHighlight',
            'cursor',
            'selection',
            'inactiveSelection',
            'lineNumber',
            'lineNumberActive',
            'indentGuide',
            'whitespace',
            'bracketMatch',
            'findMatch',
            'findMatchHighlight',
            'hoverBackground',
            'widgetBackground',
            'widgetBorder',
        ],
    },
    { id: 'editorGutter', tokens: ['addedBackground', 'modifiedBackground', 'deletedBackground'] },
    { id: 'editorBlame', tokens: ['foreground', 'background'] },
    { id: 'diff', tokens: ['insertedBackground', 'insertedLineBackground', 'removedBackground', 'removedLineBackground', 'border'] },
    { id: 'terminal', tokens: ['background', 'foreground', 'cursor', 'selection', 'commandBlockBorder', 'linkForeground'] },
    { id: 'git', tokens: ['added', 'modified', 'deleted', 'renamed', 'untracked', 'conflicted', 'staged'] },
    {
        id: 'graph',
        tokens: [
            'lane1',
            'lane2',
            'lane3',
            'lane4',
            'lane5',
            'lane6',
            'lane7',
            'lane8',
            'lane9',
            'lane10',
            'lane11',
            'lane12',
            'refBranch',
            'refTag',
            'refHead',
        ],
    },
    { id: 'statusIndicator', tokens: ['info', 'warning', 'error', 'success'] },
    { id: 'menu', tokens: ['background', 'border', 'itemHover', 'separator'] },
    { id: 'popover', tokens: ['background', 'border', 'itemHover', 'separator'] },
    { id: 'tooltip', tokens: ['background', 'border', 'itemHover', 'separator'] },
    { id: 'modal', tokens: ['background', 'border', 'itemHover', 'separator'] },
    { id: 'scrollbar', tokens: ['thumb', 'thumbHover', 'track'] },
    { id: 'input', tokens: ['background', 'foreground', 'border', 'placeholder', 'focusBorder'] },
    { id: 'button', tokens: ['background', 'foreground', 'hoverBackground', 'primaryBackground', 'primaryForeground'] },
    { id: 'list', tokens: ['background', 'hoverBackground', 'activeBackground', 'foreground'] },
]

export const SYNTAX_TOKENS = [
    'keyword',
    'storage',
    'operator',
    'string',
    'number',
    'regexp',
    'comment',
    'docComment',
    'function',
    'method',
    'variable',
    'parameter',
    'property',
    'type',
    'class',
    'interface',
    'enum',
    'constant',
    'namespace',
    'decorator',
    'tag',
    'attribute',
    'punctuation',
    'invalid',
    'link',
    'markdownHeading',
    'markdownEmphasis',
    'markdownStrong',
    'markdownCode',
    'markdownQuote',
    'markdownListMarker',
] as const

export const SYNTAX_SCOPE_CANDIDATES: Record<(typeof SYNTAX_TOKENS)[number], string[]> = {
    keyword: ['keyword.control', 'keyword'],
    storage: ['storage.type', 'storage.modifier', 'storage'],
    operator: ['keyword.operator', 'punctuation.separator'],
    string: ['string.quoted', 'string'],
    number: ['constant.numeric'],
    regexp: ['string.regexp'],
    comment: ['comment.line', 'comment'],
    docComment: ['comment.block.documentation', 'comment'],
    function: ['entity.name.function', 'support.function'],
    method: ['entity.name.function.member', 'meta.function-call', 'entity.name.function'],
    variable: ['variable.other.readwrite', 'variable.other', 'variable'],
    parameter: ['variable.parameter'],
    property: ['variable.other.property', 'support.type.property-name', 'meta.object-literal.key'],
    type: ['support.type', 'entity.name.type'],
    class: ['entity.name.type.class', 'entity.name.class', 'support.class', 'entity.name.type'],
    interface: ['entity.name.type.interface', 'entity.name.type'],
    enum: ['entity.name.type.enum', 'entity.name.type'],
    constant: ['constant.language', 'variable.other.constant', 'constant'],
    namespace: ['entity.name.namespace', 'entity.name.type.module'],
    decorator: ['meta.decorator', 'entity.name.function.decorator', 'punctuation.decorator'],
    tag: ['entity.name.tag'],
    attribute: ['entity.other.attribute-name'],
    punctuation: ['punctuation.definition', 'punctuation'],
    invalid: ['invalid.illegal', 'invalid'],
    link: ['markup.underline.link', 'string.other.link'],
    markdownHeading: ['markup.heading'],
    markdownEmphasis: ['markup.italic'],
    markdownStrong: ['markup.bold'],
    markdownCode: ['markup.inline.raw', 'markup.raw'],
    markdownQuote: ['markup.quote'],
    markdownListMarker: ['punctuation.definition.list', 'markup.list'],
}

export const TERMINAL_MIRRORED_TOKENS = {
    background: 'terminal.background',
    foreground: 'terminal.foreground',
    cursor: 'terminal.cursor',
    selection: 'terminal.selection',
} as const

export const VSCODE_DEFAULT_ANSI_PALETTE: Record<ThemeTypeArg, AnsiLookup> = {
    dark: {
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
        brightBlack: '#666666',
        brightRed: '#f14c4c',
        brightGreen: '#23d18b',
        brightYellow: '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: '#e5e5e5',
    },
    light: {
        black: '#000000',
        red: '#cd3131',
        green: '#107c10',
        yellow: '#949800',
        blue: '#0451a5',
        magenta: '#bc05bc',
        cyan: '#0598bc',
        white: '#555555',
        brightBlack: '#666666',
        brightRed: '#cd3131',
        brightGreen: '#14ce14',
        brightYellow: '#b5ba00',
        brightBlue: '#0451a5',
        brightMagenta: '#bc05bc',
        brightCyan: '#0598bc',
        brightWhite: '#a5a5a5',
    },
}

export const GRAPH_LANE_ANSI_ORDER = [
    'blue',
    'green',
    'yellow',
    'magenta',
    'cyan',
    'red',
    'brightBlue',
    'brightGreen',
    'brightYellow',
    'brightMagenta',
    'brightCyan',
    'brightRed',
] as const

const chain = (taideKey: string, category: ColorCategory, candidates: string[]): ColorMappingEntry => ({ taideKey, category, candidates })
const derived = (taideKey: string, category: ColorCategory, derive: (ctx: ResolveContext) => string | undefined): ColorMappingEntry => ({
    taideKey,
    category,
    derive,
})

export const COLOR_MAPPING: ColorMappingEntry[] = [
    chain('app.background', 'background', ['editor.background']),
    chain('app.foreground', 'foreground', ['foreground']),
    chain('app.border', 'border', ['panel.border', 'editorGroup.border', 'contrastBorder']),
    chain('app.focusBorder', 'border', ['focusBorder']),
    chain('app.shadow', 'shadow', ['widget.shadow', 'scrollbar.shadow']),
    chain('app.accent', 'status', ['textLink.foreground', 'button.background', 'focusBorder']),

    chain('appSidebar.background', 'background', ['activityBar.background', 'sideBar.background']),
    chain('appSidebar.itemHover', 'background', ['list.hoverBackground']),
    chain('appSidebar.itemActive', 'background', ['list.activeSelectionBackground']),
    chain('appSidebar.iconDefault', 'foreground', ['activityBar.inactiveForeground', 'icon.foreground']),
    derived('appSidebar.iconAgentRunning', 'status', (ctx) => ctx.ansi.green),
    derived('appSidebar.iconAgentWorking', 'status', (ctx) => ctx.ansi.blue),
    derived('appSidebar.iconAgentAwaiting', 'status', (ctx) => ctx.ansi.yellow),
    derived('appSidebar.iconAgentIdle', 'status', (ctx) => ctx.ansi.brightGreen ?? ctx.ansi.green),
    chain('appSidebar.iconAgentUnknown', 'foreground', ['disabledForeground', 'descriptionForeground']),
    chain('appSidebar.badge', 'status', ['activityBarBadge.background', 'badge.background']),

    chain('tabBar.background', 'background', ['editorGroupHeader.tabsBackground']),
    chain('tabBar.tabActiveBackground', 'background', ['tab.activeBackground']),
    chain('tabBar.tabInactiveBackground', 'background', ['tab.inactiveBackground']),
    chain('tabBar.tabActiveForeground', 'foreground', ['tab.activeForeground']),
    chain('tabBar.tabInactiveForeground', 'foreground', ['tab.inactiveForeground']),
    chain('tabBar.tabBorder', 'border', ['tab.border']),
    chain('tabBar.tabActiveIndicator', 'status', ['tab.activeBorderTop', 'tab.activeBorder', 'focusBorder']),
    chain('tabBar.dirtyDot', 'status', ['tab.activeModifiedBorder', 'gitDecoration.modifiedResourceForeground', 'terminal.ansiYellow']),
    chain('tabBar.previewForeground', 'foreground', ['tab.unfocusedActiveForeground', 'tab.inactiveForeground']),
    chain('tabBar.dropTarget', 'status', ['list.dropBackground', 'editorGroup.dropBackground', 'focusBorder']),

    chain('explorer.background', 'background', ['sideBar.background']),
    derived('explorer.itemHover', 'background', (ctx) => {
        const candidate = ctx.vscodeColors['list.hoverBackground']
        const background = ctx.resolved['explorer.background']
        return isUsableListBackground(candidate, background) ? candidate : VSCODE_LIST_HOVER_BACKGROUND_DEFAULT[ctx.type]
    }),
    derived('explorer.itemSelected', 'background', (ctx) => {
        const candidate = ctx.vscodeColors['list.activeSelectionBackground']
        const background = ctx.resolved['explorer.background']
        return isUsableListBackground(candidate, background) ? candidate : VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT[ctx.type]
    }),
    derived('explorer.itemFocused', 'background', (ctx) => {
        const background = ctx.resolved['explorer.background']
        const candidate = [ctx.vscodeColors['list.focusBackground'], ctx.vscodeColors['list.inactiveSelectionBackground']].find((value) =>
            isUsableListBackground(value, background),
        )
        return candidate ?? VSCODE_LIST_INACTIVE_SELECTION_BACKGROUND_DEFAULT[ctx.type]
    }),
    chain('explorer.indentGuide', 'border', ['tree.indentGuidesStroke']),
    chain('explorer.folderIcon', 'foreground', ['icon.foreground', 'sideBar.foreground']),
    chain('explorer.gitModified', 'status', ['gitDecoration.modifiedResourceForeground', 'terminal.ansiYellow']),
    chain('explorer.gitAdded', 'status', ['gitDecoration.addedResourceForeground', 'terminal.ansiGreen']),
    chain('explorer.gitDeleted', 'status', ['gitDecoration.deletedResourceForeground', 'terminal.ansiRed']),
    chain('explorer.gitUntracked', 'status', ['gitDecoration.untrackedResourceForeground', 'terminal.ansiGreen']),
    chain('explorer.gitIgnored', 'status', ['gitDecoration.ignoredResourceForeground', 'terminal.ansiBrightBlack']),

    chain('panel.background', 'background', ['sideBar.background']),
    chain('panel.sectionHeader', 'foreground', ['sideBarSectionHeader.foreground', 'sideBarTitle.foreground']),
    chain('panel.inputBackground', 'background', ['input.background']),
    chain('panel.inputBorder', 'border', ['input.border', 'panelInput.border', 'dropdown.border']),
    chain('panel.matchHighlight', 'status', ['list.highlightForeground', 'editor.findMatchHighlightBackground']),

    chain('editor.background', 'background', ['editor.background']),
    chain('editor.foreground', 'foreground', ['editor.foreground']),
    chain('editor.lineHighlight', 'background', ['editor.lineHighlightBackground']),
    chain('editor.cursor', 'status', ['editorCursor.foreground', 'terminal.ansiWhite']),
    chain('editor.selection', 'background', ['editor.selectionBackground']),
    chain('editor.inactiveSelection', 'background', ['editor.inactiveSelectionBackground']),
    chain('editor.lineNumber', 'foreground', ['editorLineNumber.foreground']),
    chain('editor.lineNumberActive', 'foreground', ['editorLineNumber.activeForeground']),
    chain('editor.indentGuide', 'border', ['editorIndentGuide.background1', 'editorIndentGuide.background']),
    chain('editor.whitespace', 'foreground', ['editorWhitespace.foreground']),
    chain('editor.bracketMatch', 'border', ['editorBracketMatch.border', 'editorBracketMatch.background']),
    chain('editor.findMatch', 'status', ['editor.findMatchBackground', 'terminal.ansiYellow']),
    chain('editor.findMatchHighlight', 'status', ['editor.findMatchHighlightBackground', `${SELF_REF_PREFIX}editor.findMatch`]),
    chain('editor.hoverBackground', 'background', ['editorHoverWidget.background']),
    chain('editor.widgetBackground', 'background', ['editorWidget.background', 'editorSuggestWidget.background']),
    chain('editor.widgetBorder', 'border', ['editorWidget.border', 'editorHoverWidget.border']),

    chain('editorGutter.addedBackground', 'status', ['editorGutter.addedBackground', 'terminal.ansiGreen']),
    chain('editorGutter.modifiedBackground', 'status', ['editorGutter.modifiedBackground', 'terminal.ansiBlue']),
    chain('editorGutter.deletedBackground', 'status', ['editorGutter.deletedBackground', 'terminal.ansiRed']),

    chain('editorBlame.foreground', 'foreground', ['editorCodeLens.foreground', 'editorInlayHint.foreground', 'descriptionForeground']),
    derived('editorBlame.background', 'background', () => 'transparent'),

    chain('diff.insertedBackground', 'status', ['diffEditor.insertedTextBackground', 'terminal.ansiGreen']),
    chain('diff.insertedLineBackground', 'status', ['diffEditor.insertedLineBackground', `${SELF_REF_PREFIX}diff.insertedBackground`]),
    chain('diff.removedBackground', 'status', ['diffEditor.removedTextBackground', 'terminal.ansiRed']),
    chain('diff.removedLineBackground', 'status', ['diffEditor.removedLineBackground', `${SELF_REF_PREFIX}diff.removedBackground`]),
    chain('diff.border', 'border', ['diffEditor.border', 'editorGroup.border']),

    chain('terminal.background', 'background', ['terminal.background', 'panel.background', 'editor.background']),
    chain('terminal.foreground', 'foreground', ['terminal.foreground', 'foreground']),
    chain('terminal.cursor', 'status', ['terminalCursor.foreground', 'editorCursor.foreground', 'terminal.ansiWhite']),
    chain('terminal.selection', 'background', ['terminal.selectionBackground']),
    chain('terminal.commandBlockBorder', 'border', ['panel.border', `${SELF_REF_PREFIX}app.border`]),
    chain('terminal.linkForeground', 'foreground', ['textLink.foreground', 'editorLink.activeForeground']),

    chain('git.added', 'status', ['gitDecoration.addedResourceForeground', 'terminal.ansiGreen']),
    chain('git.modified', 'status', ['gitDecoration.modifiedResourceForeground', 'terminal.ansiYellow']),
    chain('git.deleted', 'status', ['gitDecoration.deletedResourceForeground', 'terminal.ansiRed']),
    chain('git.renamed', 'status', ['gitDecoration.renamedResourceForeground', 'terminal.ansiBlue']),
    chain('git.untracked', 'status', ['gitDecoration.untrackedResourceForeground', 'terminal.ansiGreen']),
    chain('git.conflicted', 'status', ['gitDecoration.conflictingResourceForeground', 'terminal.ansiMagenta']),
    chain('git.staged', 'status', ['gitDecoration.stageModifiedResourceForeground', `${SELF_REF_PREFIX}git.modified`]),

    ...GRAPH_LANE_ANSI_ORDER.map((ansiName, index) => derived(`graph.lane${index + 1}`, 'status', (ctx) => ctx.ansi[ansiName])),
    derived('graph.refBranch', 'status', (ctx) => ctx.vscodeColors['gitDecoration.modifiedResourceForeground'] ?? ctx.ansi.blue),
    derived('graph.refTag', 'status', (ctx) => ctx.ansi.yellow),
    derived('graph.refHead', 'status', (ctx) => ctx.ansi.green),

    chain('statusIndicator.info', 'status', ['editorInfo.foreground', 'notificationsInfoIcon.foreground', 'terminal.ansiBlue']),
    chain('statusIndicator.warning', 'status', ['editorWarning.foreground', 'notificationsWarningIcon.foreground', 'terminal.ansiYellow']),
    chain('statusIndicator.error', 'status', ['editorError.foreground', 'errorForeground', 'terminal.ansiRed']),
    derived('statusIndicator.success', 'status', (ctx) => ctx.ansi.green ?? ctx.vscodeColors['gitDecoration.addedResourceForeground']),

    chain('menu.background', 'background', ['menu.background', 'dropdown.background']),
    chain('menu.border', 'border', ['menu.border', 'dropdown.border']),
    chain('menu.itemHover', 'background', ['menu.selectionBackground', 'list.hoverBackground']),
    chain('menu.separator', 'border', ['menu.separatorBackground']),

    chain('popover.background', 'background', ['editorWidget.background', 'menu.background']),
    chain('popover.border', 'border', ['editorWidget.border']),
    chain('popover.itemHover', 'background', ['list.hoverBackground']),
    chain('popover.separator', 'border', ['menu.separatorBackground']),

    chain('tooltip.background', 'background', ['editorHoverWidget.background', 'editorWidget.background']),
    chain('tooltip.border', 'border', ['editorHoverWidget.border']),
    chain('tooltip.itemHover', 'background', ['list.hoverBackground']),
    chain('tooltip.separator', 'border', ['menu.separatorBackground']),

    chain('modal.background', 'background', ['editorWidget.background', 'notifications.background']),
    chain('modal.border', 'border', ['editorWidget.border', 'notificationCenter.border']),
    chain('modal.itemHover', 'background', ['list.hoverBackground']),
    chain('modal.separator', 'border', ['menu.separatorBackground']),

    chain('scrollbar.thumb', 'background', ['scrollbarSlider.background']),
    chain('scrollbar.thumbHover', 'background', ['scrollbarSlider.hoverBackground']),
    derived('scrollbar.track', 'background', () => 'transparent'),

    chain('input.background', 'background', ['input.background']),
    chain('input.foreground', 'foreground', ['input.foreground']),
    chain('input.border', 'border', ['input.border', 'dropdown.border']),
    chain('input.placeholder', 'foreground', ['input.placeholderForeground']),
    chain('input.focusBorder', 'border', ['focusBorder']),

    chain('button.background', 'background', ['button.secondaryBackground', 'button.background']),
    chain('button.foreground', 'foreground', ['button.secondaryForeground', 'button.foreground']),
    chain('button.hoverBackground', 'background', ['button.secondaryHoverBackground', 'button.hoverBackground']),
    chain('button.primaryBackground', 'background', ['button.background']),
    chain('button.primaryForeground', 'foreground', ['button.foreground']),

    chain('list.background', 'background', ['sideBar.background']),
    chain('list.hoverBackground', 'background', ['list.hoverBackground']),
    chain('list.activeBackground', 'background', ['list.activeSelectionBackground']),
    chain('list.foreground', 'foreground', ['sideBar.foreground', 'foreground']),
]

const HEX_ALPHA_LENGTH = 9
const ALPHA_CHANNEL_MAX = 255

/**
 * A `list.*Background` candidate (hover/activeSelection/inactiveSelection) is unusable when it is fully
 * transparent (alpha 00) or shares the exact same RGB as the row background — both render as "no state at
 * all" (docs/theme-system.md §8.2.2). Themes that hit either case fall back to VS Code's official default
 * for that token instead.
 */
export const isUsableListBackground = (candidateHex: string | undefined, backgroundHex: string | undefined): candidateHex is string => {
    if (!candidateHex) return false
    const alpha = candidateHex.length === HEX_ALPHA_LENGTH ? Number.parseInt(candidateHex.slice(7, HEX_ALPHA_LENGTH), 16) : ALPHA_CHANNEL_MAX
    if (alpha === 0) return false
    if (!backgroundHex) return true
    const candidateRgb = hexToRgb(candidateHex)
    const backgroundRgb = hexToRgb(backgroundHex)
    if (!candidateRgb || !backgroundRgb) return true
    return candidateRgb.r !== backgroundRgb.r || candidateRgb.g !== backgroundRgb.g || candidateRgb.b !== backgroundRgb.b
}

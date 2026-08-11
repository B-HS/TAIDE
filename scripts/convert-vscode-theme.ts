import { hexToRgb, isHexColor, rgbToHex } from '@shared/lib/color'

const HEX_ALPHA_LENGTH = 9
const ALPHA_CHANNEL_MAX = 255
const KEBAB_ID_PATTERN = /^[a-z0-9]+(-[a-z0-9]+)*$/
const SELF_REF_PREFIX = '@'
const MAX_INCLUDE_CHAIN_DEPTH = 5

type ThemeTypeArg = 'dark' | 'light'

type CliArgs = {
    input: string
    id: string
    name: string
    type: ThemeTypeArg
    sourceUrl: string
    author: string
    license: string
    out: string
    includeDir?: string
}

type VscodeTokenColorRule = {
    scopes: string[]
    fg?: string
    bold: boolean
    italic: boolean
}

type VscodeTheme = {
    colors: Record<string, string>
    tokenColors: VscodeTokenColorRule[]
}

type SyntaxStyle = { fg: string; bold: boolean; italic: boolean }

type ColorCategory = 'foreground' | 'background' | 'border' | 'status' | 'shadow'

type ColorMappingEntry = {
    taideKey: string
    category: ColorCategory
    candidates?: string[]
    derive?: (ctx: ResolveContext) => string | undefined
}

type ResolveContext = {
    vscodeColors: Record<string, string>
    resolved: Record<string, string>
    ansi: AnsiLookup
    type: ThemeTypeArg
}

const FAMILY_FALLBACK_SOURCE_KEYS: Record<ColorCategory, string[]> = {
    foreground: ['editor.foreground', 'foreground'],
    background: ['editor.background'],
    border: ['panel.border', 'editorGroup.border', 'contrastBorder'],
    status: [],
    shadow: [],
}

const SAFE_DEFAULT_COLORS: Record<ThemeTypeArg, Record<ColorCategory, string>> = {
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
const VSCODE_LIST_HOVER_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
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
const VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#04395E',
    light: '#ADD6FF',
}

/**
 * VS Code's official `list.inactiveSelectionBackground` default (src/vs/platform/theme/common/colors/listColors.ts,
 * registerColor('list.inactiveSelectionBackground', { dark: '#37373D', light: '#E4E6F1', ... })). Same fallback
 * reasoning as `VSCODE_LIST_ACTIVE_SELECTION_BACKGROUND_DEFAULT` above, applied to the unfocused selection state.
 */
const VSCODE_LIST_INACTIVE_SELECTION_BACKGROUND_DEFAULT: Record<ThemeTypeArg, string> = {
    dark: '#37373D',
    light: '#E4E6F1',
}

const COLOR_NAMESPACES: readonly { id: string; tokens: readonly string[] }[] = [
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

const SYNTAX_TOKENS = [
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

const SYNTAX_SCOPE_CANDIDATES: Record<(typeof SYNTAX_TOKENS)[number], string[]> = {
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

const TERMINAL_ANSI_TOKENS = [
    'black',
    'red',
    'green',
    'yellow',
    'blue',
    'magenta',
    'cyan',
    'white',
    'brightBlack',
    'brightRed',
    'brightGreen',
    'brightYellow',
    'brightBlue',
    'brightMagenta',
    'brightCyan',
    'brightWhite',
] as const

type AnsiLookup = Record<(typeof TERMINAL_ANSI_TOKENS)[number], string>

const TERMINAL_MIRRORED_TOKENS = {
    background: 'terminal.background',
    foreground: 'terminal.foreground',
    cursor: 'terminal.cursor',
    selection: 'terminal.selection',
} as const

const VSCODE_DEFAULT_ANSI_PALETTE: Record<ThemeTypeArg, AnsiLookup> = {
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

const GRAPH_LANE_ANSI_ORDER = [
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

const COLOR_MAPPING: ColorMappingEntry[] = [
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

const parseArgs = (argv: string[]): CliArgs => {
    const flags = new Map<string, string>()
    for (let index = 0; index < argv.length; index += 1) {
        const token = argv[index]
        if (!token.startsWith('--')) continue
        flags.set(token.slice(2), argv[index + 1])
        index += 1
    }

    const required = ['input', 'id', 'name', 'type', 'source-url', 'author', 'license']
    const missing = required.filter((key) => !flags.get(key))
    if (missing.length > 0) {
        console.error(`Missing required args: ${missing.map((key) => `--${key}`).join(', ')}`)
        process.exit(1)
    }

    const type = flags.get('type')
    if (type !== 'dark' && type !== 'light') {
        console.error(`--type must be 'dark' or 'light', got: ${type}`)
        process.exit(1)
    }

    const id = flags.get('id') ?? ''
    if (!KEBAB_ID_PATTERN.test(id)) {
        console.error(`--id must be kebab-case (lowercase letters, digits, hyphens), got: ${id}`)
        process.exit(1)
    }

    return {
        input: flags.get('input') ?? '',
        id,
        name: flags.get('name') ?? '',
        type,
        sourceUrl: flags.get('source-url') ?? '',
        author: flags.get('author') ?? '',
        license: flags.get('license') ?? '',
        out: flags.get('out') ?? 'src-tauri/resources/themes/',
        includeDir: flags.get('include-dir'),
    }
}

const stripJsonComments = (source: string) => {
    let result = ''
    let inString = false
    let inLineComment = false
    let inBlockComment = false
    let escapeNext = false

    for (let index = 0; index < source.length; index += 1) {
        const char = source[index]
        const nextChar = source[index + 1]

        if (inLineComment) {
            if (char === '\n') {
                inLineComment = false
                result += char
            }
            continue
        }
        if (inBlockComment) {
            if (char === '*' && nextChar === '/') {
                inBlockComment = false
                index += 1
            }
            continue
        }
        if (inString) {
            result += char
            if (escapeNext) {
                escapeNext = false
            } else if (char === '\\') {
                escapeNext = true
            } else if (char === '"') {
                inString = false
            }
            continue
        }
        if (char === '"') {
            inString = true
            result += char
            continue
        }
        if (char === '/' && nextChar === '/') {
            inLineComment = true
            index += 1
            continue
        }
        if (char === '/' && nextChar === '*') {
            inBlockComment = true
            index += 1
            continue
        }
        result += char
    }

    return result.replace(/,(\s*[}\]])/g, '$1')
}

const parseJsonc = (source: string): Record<string, unknown> => JSON.parse(stripJsonComments(source))

const SHORT_HEX_NO_ALPHA_LENGTH = 3
const SHORT_HEX_WITH_ALPHA_LENGTH = 4

const expandVscodeHex = (value: string) => {
    if (!value.startsWith('#')) return value
    const hex = value.slice(1)
    if (hex.length === SHORT_HEX_NO_ALPHA_LENGTH || hex.length === SHORT_HEX_WITH_ALPHA_LENGTH) {
        return `#${[...hex].map((digit) => `${digit}${digit}`).join('')}`
    }
    return value
}

const readVscodeTheme = (raw: Record<string, unknown>): VscodeTheme => {
    const rawColors = typeof raw.colors === 'object' && raw.colors !== null ? (raw.colors as Record<string, unknown>) : {}
    const colors = Object.fromEntries(
        Object.entries(rawColors)
            .filter((entry): entry is [string, string] => typeof entry[1] === 'string')
            .map(([key, value]) => [key, expandVscodeHex(value)]),
    )
    const rawTokenColors = Array.isArray(raw.tokenColors) ? raw.tokenColors : []

    const tokenColors: VscodeTokenColorRule[] = rawTokenColors.flatMap((entry) => {
        if (typeof entry !== 'object' || entry === null) return []
        const { scope, settings } = entry as { scope?: string | string[]; settings?: { foreground?: string; fontStyle?: string } }
        if (!settings) return []
        const scopeList = scope === undefined ? [] : Array.isArray(scope) ? scope : scope.split(',')
        const scopes = scopeList.map((value) => value.trim()).filter((value) => value.length > 0)
        const fontStyle = settings.fontStyle ?? ''
        const fg = settings.foreground === undefined ? undefined : expandVscodeHex(settings.foreground)
        return [{ scopes, fg, bold: fontStyle.includes('bold'), italic: fontStyle.includes('italic') }]
    })

    return { colors, tokenColors }
}

/**
 * VS Code's built-in theme-defaults extension ships include chains that
 * reference sibling files by their original repo filename (e.g.
 * "./dark_vs.json"). This repo's local mirror of those sources uses
 * different filenames (base-dark-vs.json, vscode-dark-plus.json, ...), so
 * an include path must be mapped explicitly to the file that actually
 * holds that role rather than resolved literally against --include-dir.
 */
const INCLUDE_ROLE_FILENAME_MAP: Record<string, string> = {
    'dark_vs.json': 'base-dark-vs.json',
    'light_vs.json': 'base-light-vs.json',
    'dark_plus.json': 'vscode-dark-plus.json',
    'light_plus.json': 'vscode-light-plus.json',
}

const resolveIncludeFilename = (includePath: string): string => {
    const normalized = includePath.replace(/^\.\//, '')
    return INCLUDE_ROLE_FILENAME_MAP[normalized] ?? normalized
}

const loadRawThemeChain = async (inputPath: string, includeDir: string | undefined, depth = 0): Promise<Record<string, unknown>[]> => {
    if (depth > MAX_INCLUDE_CHAIN_DEPTH) {
        console.error(`convert-vscode-theme: include chain deeper than ${MAX_INCLUDE_CHAIN_DEPTH} levels while resolving '${inputPath}'`)
        process.exit(1)
    }

    const source = await Bun.file(inputPath).text()
    const raw = parseJsonc(source)
    const includePath = typeof raw.include === 'string' ? raw.include : undefined
    if (!includePath) return [raw]

    if (!includeDir) {
        console.error(`convert-vscode-theme: '${inputPath}' declares "include": "${includePath}" but --include-dir was not provided`)
        process.exit(1)
    }

    const basePath = `${includeDir.replace(/\/$/, '')}/${resolveIncludeFilename(includePath)}`
    const baseChain = await loadRawThemeChain(basePath, includeDir, depth + 1)
    return [...baseChain, raw]
}

const mergeVscodeThemeChain = (rawChain: Record<string, unknown>[]): VscodeTheme =>
    rawChain
        .map(readVscodeTheme)
        .reduce((merged, theme) => ({ colors: { ...merged.colors, ...theme.colors }, tokenColors: [...merged.tokenColors, ...theme.tokenColors] }))

const TERMINAL_ANSI_CANDIDATE_PREFIX = 'terminal.ansi'

const ansiNameFromCandidate = (candidate: string): (typeof TERMINAL_ANSI_TOKENS)[number] | undefined => {
    if (!candidate.startsWith(TERMINAL_ANSI_CANDIDATE_PREFIX)) return undefined
    const rest = candidate.slice(TERMINAL_ANSI_CANDIDATE_PREFIX.length)
    const name = `${rest.charAt(0).toLowerCase()}${rest.slice(1)}` as (typeof TERMINAL_ANSI_TOKENS)[number]
    return TERMINAL_ANSI_TOKENS.includes(name) ? name : undefined
}

const resolveCandidate = (candidate: string, ctx: ResolveContext): string | undefined => {
    if (candidate.startsWith(SELF_REF_PREFIX)) return ctx.resolved[candidate.slice(SELF_REF_PREFIX.length)]
    const ansiName = ansiNameFromCandidate(candidate)
    return ansiName ? ctx.ansi[ansiName] : ctx.vscodeColors[candidate]
}

const resolveFamilyFallback = (category: ColorCategory, ctx: ResolveContext): string | undefined =>
    FAMILY_FALLBACK_SOURCE_KEYS[category].map((key) => ctx.vscodeColors[key]).find((value) => value !== undefined)

const resolveColorEntry = (entry: ColorMappingEntry, ctx: ResolveContext): string | undefined => {
    if (entry.derive) return entry.derive(ctx) ?? resolveFamilyFallback(entry.category, ctx)
    for (const candidate of entry.candidates ?? []) {
        const value = resolveCandidate(candidate, ctx)
        if (value) return value
    }
    return resolveFamilyFallback(entry.category, ctx)
}

/**
 * A `list.*Background` candidate (hover/activeSelection/inactiveSelection) is unusable when it is fully
 * transparent (alpha 00) or shares the exact same RGB as the row background — both render as "no state at
 * all" (docs/theme-system.md §8.2.2). Themes that hit either case fall back to VS Code's official default
 * for that token instead.
 */
const isUsableListBackground = (candidateHex: string | undefined, backgroundHex: string | undefined): candidateHex is string => {
    if (!candidateHex) return false
    const alpha = candidateHex.length === HEX_ALPHA_LENGTH ? Number.parseInt(candidateHex.slice(7, HEX_ALPHA_LENGTH), 16) : ALPHA_CHANNEL_MAX
    if (alpha === 0) return false
    if (!backgroundHex) return true
    const candidateRgb = hexToRgb(candidateHex)
    const backgroundRgb = hexToRgb(backgroundHex)
    if (!candidateRgb || !backgroundRgb) return true
    return candidateRgb.r !== backgroundRgb.r || candidateRgb.g !== backgroundRgb.g || candidateRgb.b !== backgroundRgb.b
}

const readAnsiFromVscodeColors = (vscodeColors: Record<string, string>): Partial<AnsiLookup> => {
    const names: (typeof TERMINAL_ANSI_TOKENS)[number][] = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white']
    const ansi: Partial<AnsiLookup> = {}
    for (const name of names) {
        const capitalized = name[0].toUpperCase() + name.slice(1)
        const base = vscodeColors[`terminal.ansi${capitalized}`]
        const bright = vscodeColors[`terminal.ansiBright${capitalized}`]
        if (base) ansi[name] = base
        if (bright) ansi[`bright${capitalized}` as (typeof TERMINAL_ANSI_TOKENS)[number]] = bright
    }
    return ansi
}

/**
 * Mirrors VS Code's own terminal ANSI resolution: when a theme declares no
 * `terminal.ansi*` colors, the editor falls back to its built-in default ANSI
 * palette (see docs/theme-system.md §8.2) rather than leaving the terminal
 * uncolored. Every consumer of ANSI colors in this script (terminal output
 * and ANSI-derived semantic tokens like `graph.lane*`) shares this resolved,
 * always-complete lookup so a theme with no ANSI colors still gets VS Code's
 * real defaults instead of a generic single-color fallback.
 */
const resolveAnsiLookup = (vscodeColors: Record<string, string>, type: ThemeTypeArg): { ansi: AnsiLookup; fallbackTokens: string[] } => {
    const sourceAnsi = readAnsiFromVscodeColors(vscodeColors)
    const fallbackTokens: string[] = []
    const ansi = {} as AnsiLookup
    for (const name of TERMINAL_ANSI_TOKENS) {
        const value = sourceAnsi[name]
        if (value) {
            ansi[name] = value
            continue
        }
        ansi[name] = VSCODE_DEFAULT_ANSI_PALETTE[type][name]
        fallbackTokens.push(name)
    }
    return { ansi, fallbackTokens }
}

const resolveColors = (vscodeColors: Record<string, string>, type: ThemeTypeArg, ansi: AnsiLookup) => {
    const ctx: ResolveContext = { vscodeColors, resolved: {}, ansi, type }
    const safeDefaultNotices: string[] = []

    for (const entry of COLOR_MAPPING) {
        const resolved = resolveColorEntry(entry, ctx)
        if (resolved === undefined) safeDefaultNotices.push(`${entry.taideKey} (category: ${entry.category})`)
        ctx.resolved[entry.taideKey] = resolved ?? SAFE_DEFAULT_COLORS[type][entry.category]
    }

    return { colors: ctx.resolved, safeDefaultNotices }
}

const compositeAlphaOverBackground = (fgHex8: string, backgroundHex: string) => {
    const fgRgb = hexToRgb(fgHex8.slice(0, 7))
    const bgRgb = hexToRgb(backgroundHex)
    if (!fgRgb || !bgRgb) return fgHex8.slice(0, 7)
    const alpha = Number.parseInt(fgHex8.slice(7, HEX_ALPHA_LENGTH), 16) / ALPHA_CHANNEL_MAX
    return rgbToHex({
        r: fgRgb.r * alpha + bgRgb.r * (1 - alpha),
        g: fgRgb.g * alpha + bgRgb.g * (1 - alpha),
        b: fgRgb.b * alpha + bgRgb.b * (1 - alpha),
    })
}

const normalizeSyntaxForeground = (fg: string, editorBackground: string) =>
    isHexColor(fg) && fg.length === HEX_ALPHA_LENGTH ? compositeAlphaOverBackground(fg, editorBackground) : fg

const findBestRule = (candidateScope: string, rules: VscodeTokenColorRule[]) => {
    let best: { rule: VscodeTokenColorRule; length: number } | null = null
    for (const rule of rules) {
        for (const scope of rule.scopes) {
            const matches = candidateScope === scope || candidateScope.startsWith(`${scope}.`)
            if (matches && rule.fg && (!best || scope.length > best.length)) best = { rule, length: scope.length }
        }
    }
    return best?.rule
}

const resolveSyntax = (theme: VscodeTheme, editorForeground: string, editorBackground: string) => {
    const syntax: Record<string, SyntaxStyle> = {}
    for (const token of SYNTAX_TOKENS) {
        const candidates = SYNTAX_SCOPE_CANDIDATES[token]
        const matchedRule = candidates.map((candidate) => findBestRule(candidate, theme.tokenColors)).find((rule) => rule !== undefined)
        const fg = matchedRule?.fg ?? editorForeground
        syntax[token] = {
            fg: normalizeSyntaxForeground(fg, editorBackground),
            bold: matchedRule?.bold ?? false,
            italic: matchedRule?.italic ?? false,
        }
    }
    return syntax
}

const resolveTerminal = (resolvedColors: Record<string, string>, ansi: AnsiLookup) => {
    const terminal: Record<string, string> = { ...ansi }

    for (const [terminalKey, colorKey] of Object.entries(TERMINAL_MIRRORED_TOKENS)) {
        terminal[terminalKey] = resolvedColors[colorKey]
    }

    return terminal
}

const validateCompleteness = (colors: Record<string, string>, syntax: Record<string, SyntaxStyle>, terminal: Record<string, string>) => {
    const requiredColorKeys = COLOR_NAMESPACES.flatMap((namespace) => namespace.tokens.map((token) => `${namespace.id}.${token}`))
    const missingColors = requiredColorKeys.filter((key) => !colors[key])
    const missingSyntax = SYNTAX_TOKENS.filter((key) => !syntax[key]?.fg)
    const missingTerminal = [...TERMINAL_ANSI_TOKENS, ...Object.keys(TERMINAL_MIRRORED_TOKENS)].filter((key) => !terminal[key])

    return { missingColors, missingSyntax, missingTerminal }
}

const RGB_CHANNEL_MAX = 255
const SRGB_LINEAR_THRESHOLD = 0.03928
const SRGB_LINEAR_DIVISOR = 12.92
const SRGB_GAMMA_OFFSET = 0.055
const SRGB_GAMMA_DIVISOR = 1.055
const SRGB_GAMMA_EXPONENT = 2.4
const LUMINANCE_WEIGHT_R = 0.2126
const LUMINANCE_WEIGHT_G = 0.7152
const LUMINANCE_WEIGHT_B = 0.0722
const CONTRAST_RATIO_OFFSET = 0.05
const MIN_CONTRAST_RATIO = 3

const srgbChannelToLinear = (channel: number) => {
    const normalized = channel / RGB_CHANNEL_MAX
    return normalized <= SRGB_LINEAR_THRESHOLD
        ? normalized / SRGB_LINEAR_DIVISOR
        : ((normalized + SRGB_GAMMA_OFFSET) / SRGB_GAMMA_DIVISOR) ** SRGB_GAMMA_EXPONENT
}

const relativeLuminance = (hex: string): number | null => {
    const rgb = hexToRgb(hex)
    if (!rgb) return null
    return (
        LUMINANCE_WEIGHT_R * srgbChannelToLinear(rgb.r) +
        LUMINANCE_WEIGHT_G * srgbChannelToLinear(rgb.g) +
        LUMINANCE_WEIGHT_B * srgbChannelToLinear(rgb.b)
    )
}

const contrastRatio = (hexA: string, hexB: string): number | null => {
    const luminanceA = relativeLuminance(hexA)
    const luminanceB = relativeLuminance(hexB)
    if (luminanceA === null || luminanceB === null) return null
    const lighter = Math.max(luminanceA, luminanceB)
    const darker = Math.min(luminanceA, luminanceB)
    return (lighter + CONTRAST_RATIO_OFFSET) / (darker + CONTRAST_RATIO_OFFSET)
}

const CONTRAST_PAIRS: readonly { label: string; foregroundKey: string; backgroundKey: string }[] = [
    { label: 'app', foregroundKey: 'app.foreground', backgroundKey: 'app.background' },
    { label: 'editor', foregroundKey: 'editor.foreground', backgroundKey: 'editor.background' },
    { label: 'panel', foregroundKey: 'panel.sectionHeader', backgroundKey: 'panel.background' },
    { label: 'tooltip', foregroundKey: 'app.foreground', backgroundKey: 'tooltip.background' },
]

const CONTRAST_REPAIR_BACKGROUND_CANDIDATES: Record<string, string[]> = {
    'tooltip.background': ['editorWidget.background', 'menu.background', 'dropdown.background'],
}

const CONTRAST_REPAIR_FOREGROUND_CANDIDATES: Record<string, string[]> = {
    'app.foreground': ['editor.foreground'],
    'panel.sectionHeader': ['editor.foreground'],
}

const repairContrastPairs = (colors: Record<string, string>, vscodeColors: Record<string, string>) => {
    const repairs: string[] = []
    let repairedColors = colors

    for (const pair of CONTRAST_PAIRS) {
        const foreground = repairedColors[pair.foregroundKey]
        const background = repairedColors[pair.backgroundKey]
        const ratio = contrastRatio(foreground, background)
        if (ratio !== null && ratio >= MIN_CONTRAST_RATIO) continue

        const backgroundCandidates = CONTRAST_REPAIR_BACKGROUND_CANDIDATES[pair.backgroundKey] ?? []
        const repairedBackground = backgroundCandidates
            .map((key) => vscodeColors[key])
            .find((value) => value && (contrastRatio(foreground, value) ?? 0) >= MIN_CONTRAST_RATIO)
        if (repairedBackground) {
            repairedColors = { ...repairedColors, [pair.backgroundKey]: repairedBackground }
            repairs.push(`${pair.backgroundKey}: ${background} -> ${repairedBackground} (${pair.label} 대비 확보)`)
            continue
        }

        const foregroundCandidates = CONTRAST_REPAIR_FOREGROUND_CANDIDATES[pair.foregroundKey] ?? []
        const repairedForeground = foregroundCandidates
            .map((key) => vscodeColors[key])
            .find((value) => value && (contrastRatio(value, background) ?? 0) >= MIN_CONTRAST_RATIO)
        if (!repairedForeground) continue

        repairedColors = { ...repairedColors, [pair.foregroundKey]: repairedForeground }
        repairs.push(`${pair.foregroundKey}: ${foreground} -> ${repairedForeground} (${pair.label} 대비 확보)`)
    }

    return { colors: repairedColors, repairs }
}

const validateOutputColors = (colors: Record<string, string>) => {
    const errors: string[] = []

    if (colors['app.foreground'] === colors['app.background']) {
        errors.push(`app.foreground와 app.background가 동일한 색(${colors['app.foreground']})입니다`)
    }

    for (const pair of CONTRAST_PAIRS) {
        const foreground = colors[pair.foregroundKey]
        const background = colors[pair.backgroundKey]
        const ratio = contrastRatio(foreground, background)
        if (ratio === null || ratio < MIN_CONTRAST_RATIO) {
            errors.push(
                `${pair.label} 대비 부족: ${pair.foregroundKey}(${foreground}) vs ${pair.backgroundKey}(${background}) = ${ratio?.toFixed(2) ?? 'N/A'} (최소 ${MIN_CONTRAST_RATIO})`,
            )
        }
    }

    return errors
}

const main = async () => {
    const args = parseArgs(process.argv.slice(2))

    const rawChain = await loadRawThemeChain(args.input, args.includeDir)
    const theme = mergeVscodeThemeChain(rawChain)

    const { ansi, fallbackTokens: ansiFallbackTokens } = resolveAnsiLookup(theme.colors, args.type)
    const { colors: resolvedColors, safeDefaultNotices } = resolveColors(theme.colors, args.type, ansi)
    const { colors, repairs } = repairContrastPairs(resolvedColors, theme.colors)
    const editorBackground = colors['editor.background'] ?? theme.colors['editor.background'] ?? '#000000'
    const editorForeground = colors['editor.foreground'] ?? theme.colors['editor.foreground'] ?? theme.colors.foreground ?? '#000000'
    const syntax = resolveSyntax(theme, editorForeground, editorBackground)
    const terminal = resolveTerminal(colors, ansi)

    const { missingColors, missingSyntax, missingTerminal } = validateCompleteness(colors, syntax, terminal)
    const allMissing = [...new Set([...missingColors, ...missingSyntax, ...missingTerminal])]

    if (allMissing.length > 0) {
        console.error(`convert-vscode-theme: incomplete output for '${args.id}', missing tokens:`)
        for (const key of allMissing) console.error(`  - ${key}`)
        process.exit(1)
    }

    if (safeDefaultNotices.length > 0) {
        console.warn(`convert-vscode-theme: '${args.id}' used ${args.type} safe-default fallback for tokens with no matching source color:`)
        for (const notice of safeDefaultNotices) console.warn(`  - ${notice}`)
    }

    if (ansiFallbackTokens.length > 0) {
        console.warn(
            `convert-vscode-theme: '${args.id}' has no terminal.ansi* colors in source — used VS Code's official default ${args.type} ANSI palette (terminalColorRegistry) for: ${ansiFallbackTokens.join(', ')}`,
        )
    }

    if (repairs.length > 0) {
        console.warn(`convert-vscode-theme: '${args.id}' substituted a low-contrast token with a same-family alternative:`)
        for (const repair of repairs) console.warn(`  - ${repair}`)
    }

    const outputColorErrors = validateOutputColors(colors)
    if (outputColorErrors.length > 0) {
        console.error(`convert-vscode-theme: output color validation failed for '${args.id}':`)
        for (const error of outputColorErrors) console.error(`  - ${error}`)
        process.exit(1)
    }

    const output = {
        version: 1,
        id: args.id,
        name: args.name,
        type: args.type,
        palette: {},
        colors,
        syntax,
        terminal,
        author: args.author,
        license: args.license,
        source: args.sourceUrl,
    }

    await Bun.write(`${args.out.replace(/\/$/, '')}/${args.id}.json`, `${JSON.stringify(output, null, 4)}\n`)
    console.log(`convert-vscode-theme: wrote ${args.out.replace(/\/$/, '')}/${args.id}.json (133 colors, 31 syntax, 20 terminal)`)
}

await main()

import { hexToRgb, isHexColor, rgbToHex } from '@shared/lib/color'

const HEX_ALPHA_LENGTH = 9
const ALPHA_CHANNEL_MAX = 255
const KEBAB_ID_PATTERN = /^[a-z0-9]+(-[a-z0-9]+)*$/
const SELF_REF_PREFIX = '@'

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

type ColorMappingEntry = {
    taideKey: string
    candidates?: string[]
    derive?: (ctx: ResolveContext) => string | undefined
}

type ResolveContext = {
    vscodeColors: Record<string, string>
    resolved: Record<string, string>
    ansi: Record<string, string | undefined>
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

const TERMINAL_MIRRORED_TOKENS = {
    background: 'terminal.background',
    foreground: 'terminal.foreground',
    cursor: 'terminal.cursor',
    selection: 'terminal.selection',
} as const

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

const chain = (taideKey: string, candidates: string[]): ColorMappingEntry => ({ taideKey, candidates })
const derived = (taideKey: string, derive: (ctx: ResolveContext) => string | undefined): ColorMappingEntry => ({ taideKey, derive })

const COLOR_MAPPING: ColorMappingEntry[] = [
    chain('app.background', ['editor.background']),
    chain('app.foreground', ['foreground']),
    chain('app.border', ['panel.border', 'editorGroup.border', 'contrastBorder']),
    chain('app.focusBorder', ['focusBorder']),
    chain('app.shadow', ['widget.shadow', 'scrollbar.shadow']),
    chain('app.accent', ['textLink.foreground', 'button.background', 'focusBorder']),

    chain('appSidebar.background', ['activityBar.background', 'sideBar.background']),
    chain('appSidebar.itemHover', ['list.hoverBackground']),
    chain('appSidebar.itemActive', ['list.activeSelectionBackground']),
    chain('appSidebar.iconDefault', ['activityBar.inactiveForeground', 'icon.foreground']),
    derived('appSidebar.iconAgentRunning', (ctx) => ctx.ansi.green),
    derived('appSidebar.iconAgentWorking', (ctx) => ctx.ansi.blue),
    derived('appSidebar.iconAgentAwaiting', (ctx) => ctx.ansi.yellow),
    derived('appSidebar.iconAgentIdle', (ctx) => ctx.ansi.brightGreen ?? ctx.ansi.green),
    chain('appSidebar.iconAgentUnknown', ['disabledForeground', 'descriptionForeground']),
    chain('appSidebar.badge', ['activityBarBadge.background', 'badge.background']),

    chain('tabBar.background', ['editorGroupHeader.tabsBackground']),
    chain('tabBar.tabActiveBackground', ['tab.activeBackground']),
    chain('tabBar.tabInactiveBackground', ['tab.inactiveBackground']),
    chain('tabBar.tabActiveForeground', ['tab.activeForeground']),
    chain('tabBar.tabInactiveForeground', ['tab.inactiveForeground']),
    chain('tabBar.tabBorder', ['tab.border']),
    chain('tabBar.tabActiveIndicator', ['tab.activeBorderTop', 'tab.activeBorder', 'focusBorder']),
    chain('tabBar.dirtyDot', ['tab.activeModifiedBorder', 'gitDecoration.modifiedResourceForeground']),
    chain('tabBar.previewForeground', ['tab.unfocusedActiveForeground', 'tab.inactiveForeground']),
    chain('tabBar.dropTarget', ['list.dropBackground', 'editorGroup.dropBackground', 'focusBorder']),

    chain('explorer.background', ['sideBar.background']),
    chain('explorer.itemHover', ['list.hoverBackground']),
    chain('explorer.itemSelected', ['list.activeSelectionBackground']),
    chain('explorer.itemFocused', ['list.focusBackground', 'list.inactiveSelectionBackground']),
    chain('explorer.indentGuide', ['tree.indentGuidesStroke']),
    chain('explorer.folderIcon', ['icon.foreground', 'sideBar.foreground']),
    chain('explorer.gitModified', ['gitDecoration.modifiedResourceForeground']),
    chain('explorer.gitAdded', ['gitDecoration.addedResourceForeground']),
    chain('explorer.gitDeleted', ['gitDecoration.deletedResourceForeground']),
    chain('explorer.gitUntracked', ['gitDecoration.untrackedResourceForeground']),
    chain('explorer.gitIgnored', ['gitDecoration.ignoredResourceForeground']),

    chain('panel.background', ['sideBar.background']),
    chain('panel.sectionHeader', ['sideBarSectionHeader.foreground', 'sideBarTitle.foreground']),
    chain('panel.inputBackground', ['input.background']),
    chain('panel.inputBorder', ['input.border', 'panelInput.border', 'dropdown.border']),
    chain('panel.matchHighlight', ['list.highlightForeground', 'editor.findMatchHighlightBackground']),

    chain('editor.background', ['editor.background']),
    chain('editor.foreground', ['editor.foreground']),
    chain('editor.lineHighlight', ['editor.lineHighlightBackground']),
    chain('editor.cursor', ['editorCursor.foreground']),
    chain('editor.selection', ['editor.selectionBackground']),
    chain('editor.inactiveSelection', ['editor.inactiveSelectionBackground']),
    chain('editor.lineNumber', ['editorLineNumber.foreground']),
    chain('editor.lineNumberActive', ['editorLineNumber.activeForeground']),
    chain('editor.indentGuide', ['editorIndentGuide.background1', 'editorIndentGuide.background']),
    chain('editor.whitespace', ['editorWhitespace.foreground']),
    chain('editor.bracketMatch', ['editorBracketMatch.border', 'editorBracketMatch.background']),
    chain('editor.findMatch', ['editor.findMatchBackground']),
    chain('editor.findMatchHighlight', ['editor.findMatchHighlightBackground']),
    chain('editor.hoverBackground', ['editorHoverWidget.background']),
    chain('editor.widgetBackground', ['editorWidget.background', 'editorSuggestWidget.background']),
    chain('editor.widgetBorder', ['editorWidget.border', 'editorHoverWidget.border']),

    chain('editorGutter.addedBackground', ['editorGutter.addedBackground']),
    chain('editorGutter.modifiedBackground', ['editorGutter.modifiedBackground']),
    chain('editorGutter.deletedBackground', ['editorGutter.deletedBackground']),

    chain('editorBlame.foreground', ['editorCodeLens.foreground', 'editorInlayHint.foreground', 'descriptionForeground']),
    derived('editorBlame.background', () => 'transparent'),

    chain('diff.insertedBackground', ['diffEditor.insertedTextBackground']),
    chain('diff.insertedLineBackground', ['diffEditor.insertedLineBackground']),
    chain('diff.removedBackground', ['diffEditor.removedTextBackground']),
    chain('diff.removedLineBackground', ['diffEditor.removedLineBackground']),
    chain('diff.border', ['diffEditor.border', 'editorGroup.border']),

    chain('terminal.background', ['terminal.background', 'panel.background', 'editor.background']),
    chain('terminal.foreground', ['terminal.foreground', 'foreground']),
    chain('terminal.cursor', ['terminalCursor.foreground', 'editorCursor.foreground']),
    chain('terminal.selection', ['terminal.selectionBackground']),
    chain('terminal.commandBlockBorder', ['panel.border', `${SELF_REF_PREFIX}app.border`]),
    chain('terminal.linkForeground', ['textLink.foreground', 'editorLink.activeForeground']),

    chain('git.added', ['gitDecoration.addedResourceForeground']),
    chain('git.modified', ['gitDecoration.modifiedResourceForeground']),
    chain('git.deleted', ['gitDecoration.deletedResourceForeground']),
    chain('git.renamed', ['gitDecoration.renamedResourceForeground']),
    chain('git.untracked', ['gitDecoration.untrackedResourceForeground']),
    chain('git.conflicted', ['gitDecoration.conflictingResourceForeground']),
    chain('git.staged', ['gitDecoration.stageModifiedResourceForeground', `${SELF_REF_PREFIX}git.modified`]),

    ...GRAPH_LANE_ANSI_ORDER.map((ansiName, index) => derived(`graph.lane${index + 1}`, (ctx) => ctx.ansi[ansiName])),
    derived('graph.refBranch', (ctx) => ctx.vscodeColors['gitDecoration.modifiedResourceForeground'] ?? ctx.ansi.blue),
    derived('graph.refTag', (ctx) => ctx.ansi.yellow),
    derived('graph.refHead', (ctx) => ctx.ansi.green),

    chain('statusIndicator.info', ['editorInfo.foreground', 'notificationsInfoIcon.foreground']),
    chain('statusIndicator.warning', ['editorWarning.foreground', 'notificationsWarningIcon.foreground']),
    chain('statusIndicator.error', ['editorError.foreground', 'errorForeground']),
    derived('statusIndicator.success', (ctx) => ctx.ansi.green ?? ctx.vscodeColors['gitDecoration.addedResourceForeground']),

    chain('menu.background', ['menu.background', 'dropdown.background']),
    chain('menu.border', ['menu.border', 'dropdown.border']),
    chain('menu.itemHover', ['menu.selectionBackground', 'list.hoverBackground']),
    chain('menu.separator', ['menu.separatorBackground']),

    chain('popover.background', ['editorWidget.background', 'menu.background']),
    chain('popover.border', ['editorWidget.border']),
    chain('popover.itemHover', ['list.hoverBackground']),
    chain('popover.separator', ['menu.separatorBackground']),

    chain('tooltip.background', ['editorHoverWidget.background']),
    chain('tooltip.border', ['editorHoverWidget.border']),
    chain('tooltip.itemHover', ['list.hoverBackground']),
    chain('tooltip.separator', ['menu.separatorBackground']),

    chain('modal.background', ['editorWidget.background', 'notifications.background']),
    chain('modal.border', ['editorWidget.border', 'notificationCenter.border']),
    chain('modal.itemHover', ['list.hoverBackground']),
    chain('modal.separator', ['menu.separatorBackground']),

    chain('scrollbar.thumb', ['scrollbarSlider.background']),
    chain('scrollbar.thumbHover', ['scrollbarSlider.hoverBackground']),
    derived('scrollbar.track', () => 'transparent'),

    chain('input.background', ['input.background']),
    chain('input.foreground', ['input.foreground']),
    chain('input.border', ['input.border', 'dropdown.border']),
    chain('input.placeholder', ['input.placeholderForeground']),
    chain('input.focusBorder', ['focusBorder']),

    chain('button.background', ['button.secondaryBackground', 'button.background']),
    chain('button.foreground', ['button.secondaryForeground', 'button.foreground']),
    chain('button.hoverBackground', ['button.secondaryHoverBackground', 'button.hoverBackground']),
    chain('button.primaryBackground', ['button.background']),
    chain('button.primaryForeground', ['button.foreground']),

    chain('list.background', ['sideBar.background']),
    chain('list.hoverBackground', ['list.hoverBackground']),
    chain('list.activeBackground', ['list.activeSelectionBackground']),
    chain('list.foreground', ['sideBar.foreground', 'foreground']),
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
    const rawColors = typeof raw.colors === 'object' && raw.colors !== null ? (raw.colors as Record<string, string>) : {}
    const colors = Object.fromEntries(Object.entries(rawColors).map(([key, value]) => [key, expandVscodeHex(value)]))
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

const resolveCandidate = (candidate: string, ctx: ResolveContext): string | undefined =>
    candidate.startsWith(SELF_REF_PREFIX) ? ctx.resolved[candidate.slice(SELF_REF_PREFIX.length)] : ctx.vscodeColors[candidate]

const resolveColorEntry = (entry: ColorMappingEntry, ctx: ResolveContext): string | undefined => {
    if (entry.derive) return entry.derive(ctx)
    for (const candidate of entry.candidates ?? []) {
        const value = resolveCandidate(candidate, ctx)
        if (value) return value
    }
    return undefined
}

const buildAnsiLookup = (vscodeColors: Record<string, string>) => {
    const names = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white']
    const ansi: Record<string, string | undefined> = {}
    for (const name of names) {
        const capitalized = name[0].toUpperCase() + name.slice(1)
        ansi[name] = vscodeColors[`terminal.ansi${capitalized}`]
        ansi[`bright${capitalized}`] = vscodeColors[`terminal.ansiBright${capitalized}`]
    }
    return ansi
}

const resolveColors = (vscodeColors: Record<string, string>) => {
    const ctx: ResolveContext = { vscodeColors, resolved: {}, ansi: buildAnsiLookup(vscodeColors) }
    const missing: string[] = []

    for (const entry of COLOR_MAPPING) {
        const value = resolveColorEntry(entry, ctx) ?? ctx.vscodeColors['foreground'] ?? ctx.vscodeColors['editor.background']
        if (!value) {
            missing.push(entry.taideKey)
            continue
        }
        ctx.resolved[entry.taideKey] = value
    }

    return { colors: ctx.resolved, missing }
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

const resolveSyntax = (theme: VscodeTheme, editorBackground: string) => {
    const syntax: Record<string, SyntaxStyle> = {}
    for (const token of SYNTAX_TOKENS) {
        const candidates = SYNTAX_SCOPE_CANDIDATES[token]
        const matchedRule = candidates.map((candidate) => findBestRule(candidate, theme.tokenColors)).find((rule) => rule !== undefined)
        const fg = matchedRule?.fg ?? theme.colors['editor.foreground']
        syntax[token] = {
            fg: normalizeSyntaxForeground(fg, editorBackground),
            bold: matchedRule?.bold ?? false,
            italic: matchedRule?.italic ?? false,
        }
    }
    return syntax
}

const resolveTerminal = (vscodeColors: Record<string, string>, resolvedColors: Record<string, string>) => {
    const terminal: Record<string, string> = {}
    const missing: string[] = []
    const ansi = buildAnsiLookup(vscodeColors)

    for (const name of TERMINAL_ANSI_TOKENS) {
        const value = ansi[name]
        if (!value) {
            missing.push(name)
            continue
        }
        terminal[name] = value
    }

    for (const [terminalKey, colorKey] of Object.entries(TERMINAL_MIRRORED_TOKENS)) {
        terminal[terminalKey] = resolvedColors[colorKey]
    }

    return { terminal, missing }
}

const validateCompleteness = (colors: Record<string, string>, syntax: Record<string, SyntaxStyle>, terminal: Record<string, string>) => {
    const requiredColorKeys = COLOR_NAMESPACES.flatMap((namespace) => namespace.tokens.map((token) => `${namespace.id}.${token}`))
    const missingColors = requiredColorKeys.filter((key) => !colors[key])
    const missingSyntax = SYNTAX_TOKENS.filter((key) => !syntax[key]?.fg)
    const missingTerminal = [...TERMINAL_ANSI_TOKENS, ...Object.keys(TERMINAL_MIRRORED_TOKENS)].filter((key) => !terminal[key])

    return { missingColors, missingSyntax, missingTerminal }
}

const main = async () => {
    const args = parseArgs(process.argv.slice(2))

    const source = await Bun.file(args.input).text()
    const raw = parseJsonc(source)
    const theme = readVscodeTheme(raw)

    const { colors, missing: missingColorSources } = resolveColors(theme.colors)
    const editorBackground = colors['editor.background'] ?? theme.colors['editor.background'] ?? '#000000'
    const syntax = resolveSyntax(theme, editorBackground)
    const { terminal, missing: missingTerminalSources } = resolveTerminal(theme.colors, colors)

    const { missingColors, missingSyntax, missingTerminal } = validateCompleteness(colors, syntax, terminal)
    const allMissing = [...new Set([...missingColorSources, ...missingColors, ...missingSyntax, ...missingTerminalSources, ...missingTerminal])]

    if (allMissing.length > 0) {
        console.error(`convert-vscode-theme: incomplete output for '${args.id}', missing tokens:`)
        for (const key of allMissing) console.error(`  - ${key}`)
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

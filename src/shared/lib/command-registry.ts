import type { TFunction } from 'i18next'
import type { IRange, languages } from 'monaco-editor'
import { toast } from 'sonner'
import type { KeymapActionId } from '@shared/lib/keymap'
import { requestEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'
import { requestShowExplorerView, requestToggleExplorerSidebar } from '@shared/lib/explorer-panel-bridge'
import { buildImeDebugReport } from '@shared/lib/ime-debug'
import { i18next } from '@shared/i18n/i18n'
import { requestOpenKeybindingsEditor } from '@shared/lib/keybindings-bridge'
import { requestOpenSearchPanel } from '@shared/lib/search-panel-bridge'

export const KEYMAP_CATEGORY = {
    APP: 'keymap.category.app',
    VIEW: 'keymap.category.view',
    EDITOR: 'keymap.category.editor',
    TAB: 'keymap.category.tab',
    TERMINAL: 'keymap.category.terminal',
    SEARCH: 'keymap.category.search',
    FILE: 'keymap.category.file',
    SYNC: 'keymap.category.sync',
    WINDOW: 'keymap.category.window',
    GIT: 'keymap.category.git',
    EDITOR_SUGGEST: 'keymap.category.editorSuggest',
    EDITOR_NAVIGATION: 'keymap.category.editorNavigation',
    EDITOR_SELECTION: 'keymap.category.editorSelection',
    EDITOR_LINES: 'keymap.category.editorLines',
    EDITOR_FOLDING: 'keymap.category.editorFolding',
    EDITOR_FORMAT: 'keymap.category.editorFormat',
    EDITOR_REFACTOR: 'keymap.category.editorRefactor',
    EDITOR_DISPLAY: 'keymap.category.editorDisplay',
    SHELL_COMMAND: 'keymap.category.shellCommand',
} as const

export type CommandContext = {
    activeProjectId: string | null
    activeEditorActionIds: Set<string> | null
    openSettingsTab: () => void
    openTerminalTab: () => void
    reopenClosedTab: () => void
    switchToFileSearchMode: () => void
}

export type AppCommand = {
    id: string
    titleKey: string
    categoryKey?: string
    keymapId?: KeymapActionId
    titleDefaultValue?: string
    run: (context: CommandContext) => void | Promise<void>
    isEnabled?: (context: CommandContext) => boolean
}

export type PaletteMode = 'commands' | 'files' | 'symbol' | 'line' | 'workspaceSymbol'

export const COMMAND_MODE_PREFIX = '>'
export const SYMBOL_MODE_PREFIX = '@'
export const LINE_MODE_PREFIX = ':'
export const WORKSPACE_SYMBOL_MODE_PREFIX = '#'

export type ParsedPaletteQuery = {
    mode: PaletteMode
    searchTerm: string
}

const PALETTE_MODE_PREFIXES: { prefix: string; mode: PaletteMode }[] = [
    { prefix: COMMAND_MODE_PREFIX, mode: 'commands' },
    { prefix: SYMBOL_MODE_PREFIX, mode: 'symbol' },
    { prefix: LINE_MODE_PREFIX, mode: 'line' },
    { prefix: WORKSPACE_SYMBOL_MODE_PREFIX, mode: 'workspaceSymbol' },
]

export const parsePaletteQuery = (rawQuery: string): ParsedPaletteQuery => {
    const matchedPrefix = PALETTE_MODE_PREFIXES.find(({ prefix }) => rawQuery.startsWith(prefix))
    if (!matchedPrefix) return { mode: 'files', searchTerm: rawQuery }
    return { mode: matchedPrefix.mode, searchTerm: rawQuery.slice(matchedPrefix.prefix.length).trimStart() }
}

export const buildCommandModeQuery = (searchTerm: string = '') => `${COMMAND_MODE_PREFIX}${searchTerm}`

export type PaletteLineTarget = { line: number; column: number }

const LINE_TARGET_PATTERN = /^(\d+)(?::(\d+))?$/

/** Parses a `line` mode search term (`"123"` or `"123:45"`, already stripped of `:` by {@link parsePaletteQuery}) into a 1-based line/column target, or `null` when the input isn't a valid line reference. */
export const parseLineModeTarget = (searchTerm: string): PaletteLineTarget | null => {
    const match = LINE_TARGET_PATTERN.exec(searchTerm.trim())
    if (!match) return null
    const line = Number(match[1])
    const column = match[2] ? Number(match[2]) : 1
    if (line < 1 || column < 1) return null
    return { line, column }
}

export type FlatPaletteSymbol = {
    name: string
    detail: string
    kind: number
    containerLabel: string
    selectionRange: IRange
}

/** Flattens a monaco `DocumentSymbol` hierarchy (from `requestDocumentSymbols`) into a fuzzy-filterable list for `symbol` mode, each entry carrying an ancestor breadcrumb (`"Class > method"`) as `containerLabel`. */
export const flattenDocumentSymbols = (symbols: languages.DocumentSymbol[], containerLabel: string = ''): FlatPaletteSymbol[] =>
    symbols.flatMap((symbol) => [
        { name: symbol.name, detail: symbol.detail, kind: symbol.kind, containerLabel, selectionRange: symbol.selectionRange },
        ...flattenDocumentSymbols(symbol.children ?? [], containerLabel ? `${containerLabel} > ${symbol.name}` : symbol.name),
    ])

const commandRegistry = new Map<string, AppCommand>()

export const registerCommand = (command: AppCommand) => {
    commandRegistry.set(command.id, command)
}

export const registerCommands = (commands: AppCommand[]) => {
    for (const command of commands) registerCommand(command)
}

export const unregisterCommand = (id: string) => {
    commandRegistry.delete(id)
}

export const clearCommandRegistry = () => {
    commandRegistry.clear()
}

export const getRegisteredCommand = (id: string) => commandRegistry.get(id) ?? null

export const listRegisteredCommands = () => Array.from(commandRegistry.values())

export const isCommandRunnable = (command: AppCommand, context: CommandContext) => (command.isEnabled ? command.isEnabled(context) : true)

export const formatCategorizedLabel = (t: TFunction, categoryKey: string | null | undefined, titleKey: string, titleDefaultValue?: string) => {
    const title = titleDefaultValue ? t(titleKey, { defaultValue: titleDefaultValue }) : t(titleKey)
    return categoryKey ? `${t(categoryKey)}: ${title}` : title
}

const notImplementedRun = () => {}

const alwaysDisabled = () => false

export const DEFAULT_COMMANDS: AppCommand[] = [
    { id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: KEYMAP_CATEGORY.WINDOW, run: () => window.location.reload() },
    { id: 'settings.open', titleKey: 'settings.title', categoryKey: KEYMAP_CATEGORY.APP, run: (context) => context.openSettingsTab() },
    {
        id: 'keybindings.open',
        titleKey: 'settings.keymapOpenEditor',
        categoryKey: KEYMAP_CATEGORY.APP,
        keymapId: 'open-keybindings-editor',
        run: () => requestOpenKeybindingsEditor(),
    },
    {
        id: 'terminal.new',
        titleKey: 'keymap.newTerminal',
        categoryKey: KEYMAP_CATEGORY.TERMINAL,
        keymapId: 'new-terminal',
        run: (context) => context.openTerminalTab(),
    },
    {
        id: 'tab.reopenClosed',
        titleKey: 'keymap.reopenClosedTab',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'reopen-closed-tab',
        run: (context) => context.reopenClosedTab(),
    },
    {
        id: 'file.quickOpen',
        titleKey: 'keymap.quickOpen',
        categoryKey: KEYMAP_CATEGORY.FILE,
        keymapId: 'quick-open',
        run: (context) => context.switchToFileSearchMode(),
    },
    {
        id: 'tab.close',
        titleKey: 'keymap.closeTab',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'close-tab',
        run: notImplementedRun,
        isEnabled: alwaysDisabled,
    },
    {
        id: 'view.toggleSidebar',
        titleKey: 'keymap.toggleSidebar',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'toggle-sidebar',
        run: () => requestToggleExplorerSidebar(),
    },
    {
        id: 'editor.find',
        titleKey: 'keymap.find',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'find',
        run: notImplementedRun,
        isEnabled: alwaysDisabled,
    },
    { id: 'search.find', titleKey: 'keymap.search', categoryKey: KEYMAP_CATEGORY.SEARCH, keymapId: 'search', run: () => requestOpenSearchPanel() },
    {
        id: 'search.replace',
        titleKey: 'keymap.searchReplace',
        categoryKey: KEYMAP_CATEGORY.SEARCH,
        keymapId: 'search-replace',
        run: () => requestOpenSearchPanel({ openReplace: true }),
    },
    {
        id: 'view.explorer',
        titleKey: 'keymap.explorer',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'explorer',
        run: () => requestShowExplorerView('files'),
    },
    { id: 'view.git', titleKey: 'git.title', categoryKey: KEYMAP_CATEGORY.VIEW, keymapId: 'git', run: () => requestShowExplorerView('git') },
    {
        id: 'editor.split',
        titleKey: 'keymap.split',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'split',
        run: () => requestEditorPaneCommand({ type: 'split' }),
    },
    {
        id: 'tab.cycleNext',
        titleKey: 'keymap.tabCycleNext',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'tab-cycle-next',
        run: () => requestEditorPaneCommand({ type: 'cycle-tab', direction: 'next' }),
    },
    {
        id: 'tab.cyclePrev',
        titleKey: 'keymap.tabCyclePrev',
        categoryKey: KEYMAP_CATEGORY.TAB,
        keymapId: 'tab-cycle-prev',
        run: () => requestEditorPaneCommand({ type: 'cycle-tab', direction: 'prev' }),
    },
    {
        id: 'editor.save',
        titleKey: 'keymap.save',
        categoryKey: KEYMAP_CATEGORY.EDITOR,
        keymapId: 'save',
        run: () => requestEditorPaneCommand({ type: 'save-active-tab' }),
    },
    {
        id: 'view.toggleTerminal',
        titleKey: 'keymap.toggleTerminal',
        categoryKey: KEYMAP_CATEGORY.VIEW,
        keymapId: 'toggle-terminal',
        run: () => requestEditorPaneCommand({ type: 'toggle-terminal' }),
    },
    {
        id: 'terminal.copyImeDebug',
        titleKey: 'terminal.copyImeDebugLog',
        categoryKey: KEYMAP_CATEGORY.TERMINAL,
        run: async () => {
            await navigator.clipboard.writeText(buildImeDebugReport())
            toast.success(i18next.t('terminal.imeDebugCopied'))
        },
    },
]
